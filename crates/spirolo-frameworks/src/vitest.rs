//! The Vitest/Jest recogniser.
//!
//! Vitest and Jest share one call vocabulary — `describe`, `it`/`test`, the `beforeEach`
//! family, `bench` — reached through the same chained modifiers (`.skip`, `.each`,
//! `.skipIf`, `.only`, …). One recogniser reads both; the framework a row is attributed
//! to is decided by which package the file imported the vocabulary from.
//!
//! The recogniser claims the call names themselves, not the import that introduced them.
//! Vitest and Jest inject their globals by default, so a file that never imports `test`
//! is the common case, not the exception; a recogniser that only fired on a seen import
//! would silently see nothing in most suites (the oxlint bug the PRD names). Recognition
//! is therefore by name; the import only labels the framework and picks up local aliases.
//! A name is not the global where a function parameter or a local `var` / `let` / `const`
//! shadows it: a script's `const test = new Workflow(); test.run()` or a validator's
//! `(test) => test(value)` calls its own binding and is no test. Module-scope bindings
//! are left to the imports, so an `export const test = base.extend(…)` stays a test.
//!
//! Mocks are read where they take effect, not only where they are written. A case's own
//! body is read by [`body_facts`]. A mock written outside any case body is attached by
//! scope: `vi.mock` / `jest.mock` / `doMock` are module mocks, which the runner hoists to
//! the top of the file wherever they sit, so they land on every test and benchmark in the
//! file; a spy, `vi.fn`, fake timer or `server.use` at module scope lands on every case
//! too, and one inside a `describe` on that suite's cases only. A hook's body is the
//! hook's own. Suite rows carry no mocks.
//!
//! Known limitation: a namespace import (`import * as vi from 'vitest'; vi.test(…)`) is
//! not recognised — the call roots in the namespace, not a claimed name. Named imports
//! and injected globals, which cover the corpus, are.

use std::collections::HashMap;

use crate::analysis::{
    body_facts, body_hash, callback_of, comments_beside, is_empty_callback, mock_of,
};
use crate::chain::{resolve_chain, title_of, Chain, Modifier, EMPTY_EACH};
use crate::context::{parameters_frame, statements_frame, Scopes};
use crate::countable::{for_each_receiver, Tables};
use oxc_ast::ast::{
    ArrowFunctionExpression, BindingIdentifier, BindingPattern, BlockStatement, CallExpression,
    Expression, ForInStatement, ForOfStatement, ForStatementLeft, FormalParameters, Function,
    FunctionBody, ImportDeclarationSpecifier, ModuleExportName, Program, Statement,
    VariableDeclaration, VariableDeclarator,
};
use oxc_ast_visit::{walk, Visit};
use oxc_syntax::scope::ScopeFlags;
use spirolo_model::{
    ContextRef, FileContext, LineIndex, Mock, MockKind, Recogniser, Skip, SkipKind, Span, TestCase,
    TestKind,
};

/// Recognises Vitest and Jest test cases. See the [module docs](self).
#[derive(Debug, Default, Clone, Copy)]
pub struct VitestJest;

impl Recogniser for VitestJest {
    fn recognise(&self, file: &FileContext<'_>) -> Vec<TestCase> {
        let mut collector = Collector::new(file);
        collector
            .local_bindings
            .push(own_functions(&file.program.body));
        let module = statements_frame(&collector.lines, &file.program.body);
        collector.scopes.enter(module);
        collector.visit_program(file.program);
        collector.attach_outer_mocks();
        collector.rows
    }
}

/// What one name in the call vocabulary means: the kind of row it opens, and whether it
/// is a `test.extend` binding (a test carrying fixtures).
#[derive(Debug, Clone, Copy)]
pub struct NameInfo {
    pub kind: TestKind,
    fixtures: bool,
}

/// The vocabulary a file has in scope, and which framework its imports name.
struct Vocabulary {
    /// Every name that opens a case — the base globals, import aliases, and
    /// `test.extend` bindings — to what it means.
    names: HashMap<String, NameInfo>,
    /// The framework a row is attributed to, decided by the file's imports.
    framework: &'static str,
}

/// The visitor that walks a parsed file and collects its rows.
struct Collector<'a> {
    path: &'a str,
    /// The file's line starts, so every span is located without rescanning the source.
    lines: LineIndex,
    source: &'a str,
    vocabulary: Vocabulary,
    /// Enclosing `describe` titles, outermost first — the current row's `suite_path`.
    suite_path: Vec<String>,
    /// The `const` tables walked past and the loops currently open, so a `.each` over a
    /// local binding is counted here and a definition in a loop knows its multiplicity.
    tables: Tables,
    rows: Vec<TestCase>,
    /// Module mocks written outside any case body: hoisted, so file-wide.
    file_mocks: Vec<Mock>,
    /// The other mocks written outside any case body, per scope: index 0 is module scope,
    /// each `describe` opens the next.
    scope_mocks: Vec<Vec<Mock>>,
    /// The `beforeEach` / `beforeAll` hooks per scope, indexed like `scope_mocks`.
    scope_hooks: Vec<Vec<Span>>,
    /// The scopes currently open, module scope first.
    open_scopes: Vec<usize>,
    /// Each leaf row (its index in `rows`) with the scopes it was defined in, so the outer
    /// mocks can be attached once the whole file has been walked — a mock written after
    /// the case still runs before it.
    leaf_scopes: Vec<(usize, Vec<usize>)>,
    /// The names each enclosing function's parameters, function body and block bind with
    /// `var` / `let` / `const`, innermost last. A call rooted in one of them
    /// (`const test = new Workflow(); test.run()`) calls that binding, not the injected
    /// global, so it opens no case.
    local_bindings: Vec<Vec<String>>,
    /// The declarations in scope, so a test body's free names resolve to what they read.
    scopes: Scopes,
}

impl<'a> Collector<'a> {
    fn new(file: &FileContext<'a>) -> Self {
        Self {
            path: file.path,
            lines: LineIndex::new(file.source),
            source: file.source,
            vocabulary: Vocabulary::read(file.program),
            suite_path: Vec::new(),
            tables: Tables::default(),
            rows: Vec::new(),
            file_mocks: Vec::new(),
            scope_mocks: vec![Vec::new()],
            scope_hooks: vec![Vec::new()],
            open_scopes: vec![0],
            leaf_scopes: Vec::new(),
            local_bindings: Vec::new(),
            scopes: Scopes::default(),
        }
    }

    /// Record a mock written outside any case body, in the scope it sits in.
    fn note_outer_mock(&mut self, mock: Mock) {
        if mock.kind == MockKind::Module {
            self.file_mocks.push(mock);
        } else if let Some(scope) = self.open_scopes.last() {
            self.scope_mocks[*scope].push(mock);
        }
    }

    /// Give every test and benchmark the mocks in effect for it: the file's module mocks
    /// and the mocks of each enclosing scope, merged with its own in source order; and the
    /// before-hooks of each enclosing scope, in its context.
    fn attach_outer_mocks(&mut self) {
        for (index, scopes) in std::mem::take(&mut self.leaf_scopes) {
            let row = &mut self.rows[index];
            row.mocks.extend(self.file_mocks.iter().cloned());
            for scope in scopes {
                row.mocks.extend(self.scope_mocks[scope].iter().cloned());
                row.context.extend(
                    self.scope_hooks[scope]
                        .iter()
                        .map(|span| ContextRef::Hook { span: *span }),
                );
            }
            row.context.sort();
            row.context.dedup();
            row.mocks
                .sort_by_key(|mock| (mock.span.start, mock.span.end));
            row.mocks.dedup();
        }
    }

    /// Build the row for a matched definition call, reading its title, skip, each-table
    /// and residual modifiers off the resolved chain and the call's own arguments.
    fn build_row(&self, call: &CallExpression<'_>, chain: &Chain<'_>, info: NameInfo) -> TestCase {
        let mut skip = None;
        let mut each = None;
        let mut modifiers = Vec::new();
        for modifier in &chain.modifiers {
            match modifier.name {
                "skip" => skip = skip.or_else(|| Some(self.bare_skip(SkipKind::Skip, modifier))),
                "todo" => skip = skip.or_else(|| Some(self.bare_skip(SkipKind::Todo, modifier))),
                "fixme" => skip = skip.or_else(|| Some(self.bare_skip(SkipKind::Fixme, modifier))),
                "only" => skip = skip.or_else(|| Some(self.bare_skip(SkipKind::Only, modifier))),
                "fails" => skip = skip.or_else(|| Some(self.bare_skip(SkipKind::Fails, modifier))),
                "skipIf" => {
                    skip = skip.or_else(|| Some(self.conditional_skip(SkipKind::SkipIf, modifier)));
                }
                "runIf" => {
                    skip = skip.or_else(|| Some(self.conditional_skip(SkipKind::RunIf, modifier)));
                }
                "each" | "for" => {
                    each = each.or_else(|| modifier.table.clone()).or(Some(EMPTY_EACH));
                }
                "concurrent" | "sequential" => modifiers.push(modifier.name.to_owned()),
                _ => {}
            }
        }
        if let Some(skip) = skip.as_mut() {
            skip.comments = comments_beside(self.source, call);
        }
        if info.fixtures {
            modifiers.push("fixtures".to_owned());
        }
        modifiers.sort();
        modifiers.dedup();
        // A table read from a `const` this walk has already passed is counted on the spot.
        let each = each.map(|table| self.tables.localise(table));

        let title = if info.kind == TestKind::Hook {
            None
        } else {
            call.arguments.first().and_then(title_of)
        };

        // A suite's callback holds nested cases, not its own body, so only a leaf case
        // reads assertions and mocks; a body hash is kept for what actually runs.
        let callback = if info.kind == TestKind::Suite {
            None
        } else {
            callback_of(call)
        };
        let (assertions, mocks, throwing_queries) = callback.map_or_else(
            || (Vec::new(), Vec::new(), Vec::new()),
            |callback| {
                let facts = body_facts(&self.lines, self.source, callback);
                (facts.assertions, facts.mocks, facts.throwing_queries)
            },
        );
        let body_hash = match info.kind {
            TestKind::Test | TestKind::Benchmark => callback.map(body_hash),
            // The Vitest/Jest recogniser never opens a `Step`; it belongs to Playwright.
            TestKind::Suite | TestKind::Hook | TestKind::Step => None,
        };
        let context = match (info.kind, callback) {
            (TestKind::Test | TestKind::Benchmark, Some(callback)) => {
                self.scopes.context_of(callback)
            }
            _ => Vec::new(),
        };

        TestCase {
            file: self.path.to_owned(),
            span: self.lines.locate(call.span.start, call.span.end),
            framework: Some(self.vocabulary.framework.to_owned()),
            title,
            kind: info.kind,
            suite_path: self.suite_path.clone(),
            assertions,
            throwing_queries,
            mocks,
            skip,
            each,
            loops: self.tables.loops(),
            modifiers,
            tags: Vec::new(),
            body_hash,
            empty: matches!(info.kind, TestKind::Test | TestKind::Benchmark)
                && callback.is_some_and(is_empty_callback),
            context,
        }
    }

    /// The chain and meaning of `call` when it is a test/suite/hook definition, or
    /// `None` when it is an ordinary call (or a `test.extend` factory, which defines no
    /// case itself).
    fn classify<'e>(&self, call: &'e CallExpression<'_>) -> Option<(Chain<'e>, NameInfo)> {
        let chain = resolve_chain(&call.callee)?;
        // `test.extend({...})` builds a fixture-carrying test function; it is not itself
        // a case. Its binding is picked up as a name in the vocabulary instead.
        if chain.modifiers.last().is_some_and(|m| m.name == "extend") {
            return None;
        }
        if self
            .local_bindings
            .iter()
            .flatten()
            .any(|name| name == chain.root)
        {
            return None;
        }
        let info = *self.vocabulary.names.get(chain.root)?;
        Some((chain, info))
    }

    fn bare_skip(&self, kind: SkipKind, modifier: &Modifier<'_>) -> Skip {
        Skip {
            kind,
            condition: None,
            reason: None,
            comments: Vec::new(),
            span: self.lines.locate(modifier.span.start, modifier.span.end),
        }
    }

    fn conditional_skip(&self, kind: SkipKind, modifier: &Modifier<'_>) -> Skip {
        let condition = modifier
            .condition_span
            .and_then(|span| self.source.get(span.start as usize..span.end as usize))
            .map(str::to_owned);
        Skip {
            kind,
            condition,
            reason: None,
            comments: Vec::new(),
            span: self.lines.locate(modifier.span.start, modifier.span.end),
        }
    }
}

impl<'a> Visit<'a> for Collector<'a> {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        let Some((chain, info)) = self.classify(call) else {
            // A mock outside any case body is in effect for the cases around it.
            if let Some(mock) = mock_of(&self.lines, self.source, call) {
                self.note_outer_mock(mock);
            }
            // A `vi.mock` / `vi.hoisted` factory runs before the file's test vocabulary is
            // bound, so an `it(…)` written inside one never registers with the runner.
            if is_hoisted_factory(call) {
                return;
            }
            // `xs.forEach((x) => it(…))` defines its cases once per element, like a loop.
            if let Some(receiver) = for_each_receiver(call) {
                self.tables.enter_loop(receiver);
                walk::walk_call_expression(self, call);
                self.tables.leave_loop();
            } else {
                walk::walk_call_expression(self, call);
            }
            return;
        };

        let row = self.build_row(call, &chain, info);
        if info.kind == TestKind::Suite {
            // Only a suite nests cases. Walk into its callback (the call's arguments,
            // never its callee — the `.each`/`.skipIf` sub-calls were consumed as
            // modifiers) with this title on the suite path.
            self.suite_path
                .push(row.title.clone().unwrap_or_else(|| "<dynamic>".to_owned()));
            self.rows.push(row);
            self.tables.enter_suite();
            self.scope_mocks.push(Vec::new());
            self.scope_hooks.push(Vec::new());
            self.open_scopes.push(self.scope_mocks.len() - 1);
            self.visit_arguments(&call.arguments);
            self.open_scopes.pop();
            self.tables.leave_suite();
            self.suite_path.pop();
        } else {
            // A test/hook/bench body defines no further case — a test called inside
            // another test is a collection error in both frameworks — so it is not
            // descended into, and a nested call is not mistaken for a second case.
            if matches!(info.kind, TestKind::Test | TestKind::Benchmark) {
                self.leaf_scopes
                    .push((self.rows.len(), self.open_scopes.clone()));
            }
            if matches!(chain.root, "beforeEach" | "beforeAll") {
                if let Some(scope) = self.open_scopes.last() {
                    self.scope_hooks[*scope].push(row.span);
                }
            }
            self.rows.push(row);
        }
    }

    fn visit_variable_declaration(&mut self, declaration: &VariableDeclaration<'a>) {
        // `const myTest = test.extend({…})` names a fixture-carrying test for the rest of
        // its block; register it before the walk reaches its uses. Done here, not only at
        // the top level, so a binding declared inside a `describe` is recognised too.
        for declarator in &declaration.declarations {
            register_extend_binding(&mut self.vocabulary.names, declarator);
            self.tables.register(declarator);
        }
        walk::walk_variable_declaration(self, declaration);
    }

    fn visit_block_statement(&mut self, block: &BlockStatement<'a>) {
        self.local_bindings.push(declared_names(&block.body));
        self.scopes
            .enter(statements_frame(&self.lines, &block.body));
        walk::walk_block_statement(self, block);
        self.scopes.leave();
        self.local_bindings.pop();
    }

    fn visit_function(&mut self, function: &Function<'a>, flags: ScopeFlags) {
        self.local_bindings.push(parameter_names(&function.params));
        self.scopes
            .enter(parameters_frame(&self.lines, &function.params));
        walk::walk_function(self, function, flags);
        self.scopes.leave();
        self.local_bindings.pop();
    }

    fn visit_arrow_function_expression(&mut self, arrow: &ArrowFunctionExpression<'a>) {
        self.local_bindings.push(parameter_names(&arrow.params));
        self.scopes
            .enter(parameters_frame(&self.lines, &arrow.params));
        walk::walk_arrow_function_expression(self, arrow);
        self.scopes.leave();
        self.local_bindings.pop();
    }

    fn visit_function_body(&mut self, body: &FunctionBody<'a>) {
        self.local_bindings.push(declared_names(&body.statements));
        self.scopes
            .enter(statements_frame(&self.lines, &body.statements));
        walk::walk_function_body(self, body);
        self.scopes.leave();
        self.local_bindings.pop();
    }

    fn visit_for_of_statement(&mut self, statement: &ForOfStatement<'a>) {
        // `for (const row of rows) it(…)` defines its cases once per element of `rows`.
        // The loop's own binding shadows the vocabulary inside it: a reporter's
        // `for (const test of tests) test.meta()` reads an object, it defines no case.
        self.tables.enter_loop(&statement.right);
        self.local_bindings.push(loop_names(&statement.left));
        walk::walk_for_of_statement(self, statement);
        self.local_bindings.pop();
        self.tables.leave_loop();
    }

    fn visit_for_in_statement(&mut self, statement: &ForInStatement<'a>) {
        self.local_bindings.push(loop_names(&statement.left));
        walk::walk_for_in_statement(self, statement);
        self.local_bindings.pop();
    }
}

// --------------------------------------------------------------- the vocabulary

impl Vocabulary {
    /// Read a file's imports into the vocabulary: the aliases they add, and which
    /// framework they name. The `test.extend` bindings are added later, during the walk,
    /// so one inside a `describe` is caught as readily as one at the top level.
    fn read(program: &Program<'_>) -> Self {
        let mut names = base_globals();
        let mut saw_jest = false;
        // A base name imported from a package the recogniser does not claim (Playwright's
        // `test`, a project's own `test` helper) is no longer the injected global — it is
        // that import. Recording the shadow and dropping it after the pass keeps a
        // Playwright file's `test` out of the Vitest table, so the two frameworks are
        // counted apart rather than double-counted.
        let mut claimed_locals = Vec::new();
        let mut shadowed_locals = Vec::new();

        for statement in &program.body {
            let Statement::ImportDeclaration(import) = statement else {
                continue;
            };
            let Some(specifiers) = &import.specifiers else {
                continue;
            };
            match import.source.value.as_str().and_then(claimed_framework) {
                Some(framework) => {
                    saw_jest |= framework == "jest";
                    for specifier in specifiers {
                        if let ImportDeclarationSpecifier::ImportSpecifier(imported) = specifier {
                            if let Some(info) = base_info(module_export_name(&imported.imported)) {
                                let local = imported.local.name.as_str().to_owned();
                                names.insert(local.clone(), info);
                                claimed_locals.push(local);
                            }
                        }
                    }
                }
                None => {
                    for specifier in specifiers {
                        let local = local_name(specifier);
                        if names.contains_key(local) {
                            shadowed_locals.push(local.to_owned());
                        }
                    }
                }
            }
        }

        for local in shadowed_locals {
            if !claimed_locals.contains(&local) {
                names.remove(&local);
            }
        }

        Self {
            names,
            framework: if saw_jest { "jest" } else { "vitest" },
        }
    }
}

/// The names `statements` declare with `var` / `let` / `const` directly, not in a nested
/// block. A `test.extend(…)` binding is left out: it builds a test function rather than
/// shadowing one, and [`register_extend_binding`] adds it to the vocabulary.
fn declared_names(statements: &[Statement<'_>]) -> Vec<String> {
    let mut names = BindingNames::default();
    for statement in statements {
        match statement {
            Statement::VariableDeclaration(declaration) => {
                for declarator in &declaration.declarations {
                    if !is_extend(declarator) {
                        names.visit_binding_pattern(&declarator.id);
                    }
                }
            }
            Statement::FunctionDeclaration(function) => {
                names
                    .0
                    .extend(function.id.as_ref().map(|id| id.name.as_str().to_owned()));
            }
            _ => {}
        }
    }
    names.0
}

/// The names a module declares at its top level as functions of its own: a
/// `function it(…) {…}` or a `var test = function (…) {…}` — a minified bundle's locals,
/// never the runner's vocabulary. Any other top-level binding is left to the imports, so
/// an `export const test = base.extend(…)` or a destructured `require` stays a test.
fn own_functions(statements: &[Statement<'_>]) -> Vec<String> {
    let mut names = Vec::new();
    for statement in statements {
        match statement {
            Statement::FunctionDeclaration(function) => {
                names.extend(function.id.as_ref().map(|id| id.name.as_str().to_owned()));
            }
            Statement::VariableDeclaration(declaration) => {
                for declarator in &declaration.declarations {
                    let is_function = declarator.init.as_ref().is_some_and(|init| {
                        matches!(
                            init.get_inner_expression(),
                            Expression::FunctionExpression(_)
                                | Expression::ArrowFunctionExpression(_)
                        )
                    });
                    if let (true, BindingPattern::BindingIdentifier(id)) =
                        (is_function, &declarator.id)
                    {
                        names.push(id.name.as_str().to_owned());
                    }
                }
            }
            _ => {}
        }
    }
    names
}

/// The names a `for … of` / `for … in` head declares (`for (const test of tests)`).
fn loop_names(left: &ForStatementLeft<'_>) -> Vec<String> {
    let mut names = BindingNames::default();
    if let ForStatementLeft::VariableDeclaration(declaration) = left {
        for declarator in &declaration.declarations {
            names.visit_binding_pattern(&declarator.id);
        }
    }
    names.0
}

/// Whether `declarator` binds the result of a `….extend(…)` call.
fn is_extend(declarator: &VariableDeclarator<'_>) -> bool {
    declarator
        .init
        .as_ref()
        .and_then(resolve_chain)
        .is_some_and(|chain| chain.modifiers.iter().any(|m| m.name == "extend"))
}

/// The names a function's parameters bind.
fn parameter_names(params: &FormalParameters<'_>) -> Vec<String> {
    let mut names = BindingNames::default();
    names.visit_formal_parameters(params);
    names.0
}

/// Collects the identifiers a binding pattern binds.
#[derive(Default)]
struct BindingNames(Vec<String>);

impl<'a> Visit<'a> for BindingNames {
    fn visit_binding_identifier(&mut self, identifier: &BindingIdentifier<'a>) {
        self.0.push(identifier.name.as_str().to_owned());
    }
}

/// Whether a call is a `vi.mock` / `vi.doMock` / `vi.hoisted` / `jest.mock` / `jest.doMock`
/// factory, whose body the runner evaluates outside any suite.
fn is_hoisted_factory(call: &CallExpression<'_>) -> bool {
    let Some(chain) = resolve_chain(&call.callee) else {
        return false;
    };
    matches!(chain.root, "vi" | "jest")
        && chain
            .modifiers
            .last()
            .is_some_and(|m| matches!(m.name, "mock" | "doMock" | "hoisted"))
}

/// Record `const myTest = test.extend({…})` as a fixture-carrying test binding.
fn register_extend_binding(
    names: &mut HashMap<String, NameInfo>,
    declarator: &VariableDeclarator<'_>,
) {
    let (Some(init), BindingPattern::BindingIdentifier(binding)) =
        (&declarator.init, &declarator.id)
    else {
        return;
    };
    let Some(chain) = resolve_chain(init) else {
        return;
    };
    if names.contains_key(chain.root) && chain.modifiers.iter().any(|m| m.name == "extend") {
        names.insert(
            binding.name.as_str().to_owned(),
            NameInfo {
                kind: TestKind::Test,
                fixtures: true,
            },
        );
    }
}

/// Every base name Vitest and Jest inject, so recognition never depends on an import.
const BASE_NAMES: [&str; 9] = [
    "it",
    "test",
    "describe",
    "suite",
    "bench",
    "beforeAll",
    "beforeEach",
    "afterAll",
    "afterEach",
];

/// The injected globals as a vocabulary, derived from [`base_info`] so the two never
/// disagree about what a name means.
fn base_globals() -> HashMap<String, NameInfo> {
    BASE_NAMES
        .into_iter()
        .filter_map(|name| Some((name.to_owned(), base_info(name)?)))
        .collect()
}

/// The meaning of a base name (`it`, `describe`, `bench`, a hook), when it is one.
pub fn base_info(name: &str) -> Option<NameInfo> {
    let kind = match name {
        "it" | "test" => TestKind::Test,
        "describe" | "suite" => TestKind::Suite,
        "bench" => TestKind::Benchmark,
        "beforeAll" | "beforeEach" | "afterAll" | "afterEach" => TestKind::Hook,
        _ => return None,
    };
    Some(NameInfo {
        kind,
        fixtures: false,
    })
}

/// The framework an import source names, when it is one the recogniser claims.
pub fn claimed_framework(source: &str) -> Option<&'static str> {
    match source {
        "@jest/globals" => Some("jest"),
        "vitest" | "vite-plus/test" => Some("vitest"),
        _ => None,
    }
}

/// The local binding an import specifier introduces, whatever its shape — a named,
/// default, or namespace import all bind one name into the module's scope.
fn local_name<'a>(specifier: &'a ImportDeclarationSpecifier<'a>) -> &'a str {
    match specifier {
        ImportDeclarationSpecifier::ImportSpecifier(imported) => imported.local.name.as_str(),
        ImportDeclarationSpecifier::ImportDefaultSpecifier(default) => default.local.name.as_str(),
        ImportDeclarationSpecifier::ImportNamespaceSpecifier(namespace) => {
            namespace.local.name.as_str()
        }
    }
}

fn module_export_name<'a>(name: &'a ModuleExportName<'a>) -> &'a str {
    match name {
        ModuleExportName::IdentifierName(id) => id.name.as_str(),
        ModuleExportName::IdentifierReference(id) => id.name.as_str(),
        // oxc's parser rejects a lone surrogate in an export name (a fatal error), so this holds.
        ModuleExportName::StringLiteral(_) => name.name().as_str(),
    }
}
