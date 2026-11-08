//! The Playwright recogniser.
//!
//! Playwright's vocabulary hangs off one imported root, `test`: `test(…)` is a case,
//! `test.describe(…)` a suite, `test.step(…)` a sub-unit inside a case, and
//! `test.skip`/`fixme`/`fail`/`slow`/`only` are modifiers that reach the same root
//! through a member chain. Unlike Vitest, nothing is injected as a global — a Playwright
//! file always imports `test` — so recognition starts from the import (and its aliases,
//! including a `base.extend({…})` fixture root) rather than from a fixed name list.
//!
//! A conditional modifier is written as a statement inside the body it governs
//! (`test('t', async ({ browserName }) => { test.skip(browserName === 'firefox') })`),
//! so the recogniser descends a case's callback to attach that condition to the case and
//! to collect its `test.step` children. Written directly in a `describe` body it governs
//! the whole group, and attaches to the suite row the same way. A leaf case reads its own assertions and mocks;
//! a step is a structural marker whose expects are counted once, under the case.

use std::collections::HashMap;

use crate::analysis::{body_facts, body_hash, callback_of, comments_beside, is_empty_callback};
use crate::chain::{resolve_chain, title_of};
use crate::context::{parameters_frame, statements_frame, Scopes};
use crate::countable::{for_each_receiver, Tables};
use oxc_ast::ast::{
    Argument, ArrowFunctionExpression, BlockStatement, CallExpression, Expression, ForOfStatement,
    Function, FunctionBody, ImportDeclarationSpecifier, ModuleExportName, ObjectPropertyKind,
    Program, PropertyKey, Statement, VariableDeclaration, VariableDeclarator,
};
use oxc_ast_visit::{walk, Visit};
use oxc_span::GetSpan;
use oxc_syntax::scope::ScopeFlags;
use spirolo_model::{
    ContextRef, FileContext, LineIndex, Recogniser, Skip, SkipKind, Span, TestCase, TestKind,
};

/// Recognises Playwright test cases. See the [module docs](self).
#[derive(Debug, Default, Clone, Copy)]
pub struct Playwright;

impl Recogniser for Playwright {
    fn recognise(&self, file: &FileContext<'_>) -> Vec<TestCase> {
        let roots = read_roots(file.program);
        // No Playwright import means no root, and Playwright injects nothing globally, so
        // there is nothing here to recognise — leave the file to the other recognisers.
        if roots.is_empty() {
            return Vec::new();
        }
        let mut collector = Collector {
            path: file.path,
            lines: LineIndex::new(file.source),
            source: file.source,
            roots,
            suite_path: Vec::new(),
            current_case: None,
            open_suites: Vec::new(),
            tables: Tables::default(),
            rows: Vec::new(),
            scopes: Scopes::default(),
            scope_hooks: vec![Vec::new()],
            open_scopes: vec![0],
            leaf_scopes: Vec::new(),
        };
        let module = statements_frame(&collector.lines, &file.program.body);
        collector.scopes.enter(module);
        collector.visit_program(file.program);
        collector.attach_hooks();
        collector.rows
    }
}

/// A root name and whether it carries `test.extend` fixtures.
#[derive(Debug, Clone, Copy)]
struct RootInfo {
    fixtures: bool,
}

/// The visitor that walks a parsed file and collects its rows.
struct Collector<'a> {
    path: &'a str,
    /// The file's line starts, so every span is located without rescanning the source.
    lines: LineIndex,
    source: &'a str,
    /// Local names that mean a Playwright test function — the imported `test` and its
    /// aliases, plus any `base.extend({…})` binding found during the walk.
    roots: HashMap<String, RootInfo>,
    /// Enclosing `test.describe` titles, outermost first.
    suite_path: Vec<String>,
    /// Index in `rows` of the case currently being descended, so a conditional modifier
    /// statement in its body attaches its condition to it.
    current_case: Option<usize>,
    /// Indexes in `rows` of the suites currently being descended, innermost last, so a
    /// conditional modifier statement written directly in a `describe` body (Playwright
    /// skips the whole group that way) attaches to the suite rather than being dropped.
    open_suites: Vec<usize>,
    /// The `const` tables walked past and the loops currently open, so a definition in a
    /// `for…of` knows its multiplicity.
    tables: Tables,
    rows: Vec<TestCase>,
    /// The declarations in scope at the walk's current point, for each case's context.
    scopes: Scopes,
    /// The before-hooks of the file (index 0) and of each suite walked, by suite.
    scope_hooks: Vec<Vec<Span>>,
    /// Indexes in `scope_hooks` of the file and the suites currently being descended.
    open_scopes: Vec<usize>,
    /// Each case's index in `rows` and the `scope_hooks` indexes enclosing it.
    leaf_scopes: Vec<(usize, Vec<usize>)>,
}

/// What a matched call is.
enum Match {
    /// `test.describe(…)` — a suite that nests cases.
    Suite(TestCase),
    /// `test(…)` / `test.skip('t', fn)` — a leaf case.
    Case(TestCase),
    /// `test.step(…)` — a sub-unit inside a case.
    Step(TestCase),
    /// `test.beforeEach(…)` / `test.afterAll(…)` — a hook, not a case: the runner runs it
    /// around cases but never lists it as a test.
    Hook(TestCase),
    /// `test.skip(condition)` as a statement — a modifier on the enclosing case.
    Annotation(Skip),
}

impl<'a> Visit<'a> for Collector<'a> {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        match self.classify(call) {
            Some(Match::Suite(row)) => {
                self.suite_path
                    .push(row.title.clone().unwrap_or_else(|| "<dynamic>".to_owned()));
                self.open_suites.push(self.rows.len());
                self.rows.push(row);
                self.scope_hooks.push(Vec::new());
                self.open_scopes.push(self.scope_hooks.len() - 1);
                self.tables.enter_suite();
                self.visit_arguments(&call.arguments);
                self.tables.leave_suite();
                self.open_scopes.pop();
                self.open_suites.pop();
                self.suite_path.pop();
            }
            Some(Match::Case(row)) => {
                // A case nested in a case is a collection error in Playwright; only the
                // outer one counts, and its body is descended for steps and conditions.
                if self.current_case.is_some() {
                    self.visit_arguments(&call.arguments);
                    return;
                }
                let index = self.rows.len();
                self.rows.push(row);
                self.leaf_scopes.push((index, self.open_scopes.clone()));
                self.current_case = Some(index);
                self.visit_arguments(&call.arguments);
                self.current_case = None;
            }
            Some(Match::Step(row)) => {
                self.rows.push(row);
                // A step can nest steps; descend, but the enclosing case still owns the
                // conditions, so `current_case` is left as it is.
                self.visit_arguments(&call.arguments);
            }
            Some(Match::Hook(row)) => {
                if is_before_hook(call) {
                    if let Some(scope) = self.open_scopes.last() {
                        self.scope_hooks[*scope].push(row.span);
                    }
                }
                self.rows.push(row);
                // A hook body may nest steps or define nothing; descend it, but it is not a
                // case, so it never becomes `current_case`.
                self.visit_arguments(&call.arguments);
            }
            Some(Match::Annotation(skip)) => {
                // Inside a case the annotation governs the case; directly inside a
                // `describe` body it governs the suite. At file level it governs nothing
                // the model rows, so it is dropped.
                let owner = self
                    .current_case
                    .or_else(|| self.open_suites.last().copied());
                if let Some(index) = owner {
                    let row = &mut self.rows[index];
                    if row.skip.is_none() {
                        row.skip = Some(skip);
                    }
                }
            }
            None => {
                // `xs.forEach((x) => test(…))` defines its cases once per element.
                if let Some(receiver) = for_each_receiver(call) {
                    self.tables.enter_loop(receiver);
                    walk::walk_call_expression(self, call);
                    self.tables.leave_loop();
                } else {
                    walk::walk_call_expression(self, call);
                }
            }
        }
    }

    fn visit_block_statement(&mut self, block: &BlockStatement<'a>) {
        self.scopes
            .enter(statements_frame(&self.lines, &block.body));
        walk::walk_block_statement(self, block);
        self.scopes.leave();
    }

    fn visit_function(&mut self, function: &Function<'a>, flags: ScopeFlags) {
        self.scopes
            .enter(parameters_frame(&self.lines, &function.params));
        walk::walk_function(self, function, flags);
        self.scopes.leave();
    }

    fn visit_arrow_function_expression(&mut self, arrow: &ArrowFunctionExpression<'a>) {
        self.scopes
            .enter(parameters_frame(&self.lines, &arrow.params));
        walk::walk_arrow_function_expression(self, arrow);
        self.scopes.leave();
    }

    fn visit_function_body(&mut self, body: &FunctionBody<'a>) {
        self.scopes
            .enter(statements_frame(&self.lines, &body.statements));
        walk::walk_function_body(self, body);
        self.scopes.leave();
    }

    fn visit_for_of_statement(&mut self, statement: &ForOfStatement<'a>) {
        // `for (const route of routes) test(…)` defines a case per element of `routes`.
        self.tables.enter_loop(&statement.right);
        walk::walk_for_of_statement(self, statement);
        self.tables.leave_loop();
    }

    fn visit_variable_declaration(&mut self, declaration: &VariableDeclaration<'a>) {
        // `const test = base.extend({…})` names a fixture-carrying root for the rest of
        // the block; register it before the walk reaches its uses.
        for declarator in &declaration.declarations {
            self.register_extend_binding(declarator);
            self.tables.register(declarator);
        }
        walk::walk_variable_declaration(self, declaration);
    }
}

impl Collector<'_> {
    /// Give every case the before-hooks of the file and of each suite enclosing it, in
    /// its context.
    fn attach_hooks(&mut self) {
        for (index, scopes) in std::mem::take(&mut self.leaf_scopes) {
            let row = &mut self.rows[index];
            for scope in scopes {
                row.context.extend(
                    self.scope_hooks[scope]
                        .iter()
                        .map(|span| ContextRef::Hook { span: *span }),
                );
            }
            row.context.sort();
            row.context.dedup();
        }
    }

    /// Decide what `call` is, or `None` when it is not a Playwright definition.
    fn classify(&self, call: &CallExpression<'_>) -> Option<Match> {
        let chain = resolve_chain(&call.callee)?;
        let root = *self.roots.get(chain.root)?;
        let names: Vec<&str> = chain
            .modifiers
            .iter()
            .map(|modifier| modifier.name)
            .collect();

        // `test.describe.configure({…})` sets options for a suite; it defines no case.
        if names.contains(&"configure") {
            return None;
        }

        if names.contains(&"describe") {
            return Some(Match::Suite(self.suite_row(call, &names)));
        }
        if names.contains(&"step") {
            return Some(Match::Step(self.step_row(call)));
        }
        // `test.beforeEach` / `afterEach` / `beforeAll` / `afterAll` are hooks: they carry a
        // callback like a case but the runner runs them around cases and never lists them.
        if names.iter().any(|name| is_hook(name)) {
            return Some(Match::Hook(self.hook_row(call)));
        }

        // A modifier call with no callback is a conditional annotation on the enclosing
        // case (`test.skip(browserName === 'firefox')`), not a case of its own. So is the
        // callback form, `test.skip(({ browserName }) => browserName === 'webkit', 'why')`:
        // its first argument is the condition, not a title, so nothing is defined.
        let condition_form = matches!(
            call.arguments.first(),
            Some(Argument::ArrowFunctionExpression(_) | Argument::FunctionExpression(_))
        );
        if callback_of(call).is_none() || condition_form {
            let &modifier = names.last()?;
            let kind = skip_kind(modifier)?;
            return Some(Match::Annotation(self.annotation(call, kind)));
        }

        Some(Match::Case(self.case_row(call, root, &names)))
    }

    /// A `test.describe(…)` suite row, carrying its skip/only and serial/parallel mode.
    fn suite_row(&self, call: &CallExpression<'_>, names: &[&str]) -> TestCase {
        let skip = names
            .iter()
            .find_map(|name| skip_kind(name))
            .map(|kind| self.bare_skip(kind, call));
        let modifiers = mode_modifiers(names);
        TestCase {
            file: self.path.to_owned(),
            span: self.lines.locate(call.span.start, call.span.end),
            framework: Some("playwright".to_owned()),
            title: call.arguments.first().and_then(title_of),
            kind: TestKind::Suite,
            suite_path: self.suite_path.clone(),
            assertions: Vec::new(),
            throwing_queries: Vec::new(),
            mocks: Vec::new(),
            skip,
            each: None,
            loops: self.tables.loops(),
            modifiers,
            tags: Vec::new(),
            body_hash: None,
            empty: false,
            context: Vec::new(),
        }
    }

    /// A `test(…)` / `test.skip('t', fn)` leaf-case row, with its assertions, mocks,
    /// tags and body hash.
    fn case_row(&self, call: &CallExpression<'_>, root: RootInfo, names: &[&str]) -> TestCase {
        let skip = names
            .iter()
            .find_map(|name| skip_kind(name))
            .map(|kind| self.bare_skip(kind, call));
        let mut modifiers = mode_modifiers(names);
        if root.fixtures {
            modifiers.push("fixtures".to_owned());
        }
        modifiers.sort();
        modifiers.dedup();

        let title = call.arguments.first().and_then(title_of);
        let (assertions, mocks, throwing_queries) = callback_of(call).map_or_else(
            || (Vec::new(), Vec::new(), Vec::new()),
            |callback| {
                let facts = body_facts(&self.lines, self.source, callback);
                (facts.assertions, facts.mocks, facts.throwing_queries)
            },
        );
        let tags = tags_of(call, title.as_deref());

        TestCase {
            file: self.path.to_owned(),
            span: self.lines.locate(call.span.start, call.span.end),
            framework: Some("playwright".to_owned()),
            title,
            kind: TestKind::Test,
            suite_path: self.suite_path.clone(),
            assertions,
            throwing_queries,
            mocks,
            skip,
            each: None,
            loops: self.tables.loops(),
            modifiers,
            tags,
            body_hash: callback_of(call).map(body_hash),
            empty: callback_of(call).is_some_and(is_empty_callback),
            context: callback_of(call)
                .map_or_else(Vec::new, |callback| self.scopes.context_of(callback)),
        }
    }

    /// A `test.step(…)` row — a structural marker whose title and place are kept, but
    /// whose expects stay counted once, under the enclosing case.
    fn step_row(&self, call: &CallExpression<'_>) -> TestCase {
        TestCase {
            file: self.path.to_owned(),
            span: self.lines.locate(call.span.start, call.span.end),
            framework: Some("playwright".to_owned()),
            title: call.arguments.first().and_then(title_of),
            kind: TestKind::Step,
            suite_path: self.suite_path.clone(),
            assertions: Vec::new(),
            throwing_queries: Vec::new(),
            mocks: Vec::new(),
            skip: None,
            each: None,
            loops: Vec::new(),
            modifiers: Vec::new(),
            tags: Vec::new(),
            body_hash: None,
            empty: false,
            context: Vec::new(),
        }
    }

    /// A `test.beforeEach(…)` / `afterAll(…)` hook row — a structural marker with no
    /// title, so a runner's test list never counts it.
    fn hook_row(&self, call: &CallExpression<'_>) -> TestCase {
        TestCase {
            file: self.path.to_owned(),
            span: self.lines.locate(call.span.start, call.span.end),
            framework: Some("playwright".to_owned()),
            title: None,
            kind: TestKind::Hook,
            suite_path: self.suite_path.clone(),
            assertions: Vec::new(),
            throwing_queries: Vec::new(),
            mocks: Vec::new(),
            skip: None,
            each: None,
            loops: Vec::new(),
            modifiers: Vec::new(),
            tags: Vec::new(),
            body_hash: None,
            empty: false,
            context: Vec::new(),
        }
    }

    /// A conditional modifier statement's skip: its kind, its condition lifted verbatim
    /// from the first argument, and its reason read from the second when that is a string
    /// (`test.skip(cond, 'why')`). A bare `test.skip()` has neither.
    fn annotation(&self, call: &CallExpression<'_>, kind: SkipKind) -> Skip {
        let condition = call
            .arguments
            .first()
            .and_then(Argument::as_expression)
            .map(GetSpan::span)
            .and_then(|span| self.source.get(span.start as usize..span.end as usize))
            .map(str::to_owned);
        let reason = call
            .arguments
            .get(1)
            .and_then(Argument::as_expression)
            .and_then(string_text);
        Skip {
            kind,
            condition,
            reason,
            comments: Vec::new(),
            span: self.lines.locate(call.span.start, call.span.end),
        }
    }

    /// A modifier with no condition (the definition forms `test.skip('t', fn)` and
    /// `test.describe.only('s', fn)`), located at the whole call.
    fn bare_skip(&self, kind: SkipKind, call: &CallExpression<'_>) -> Skip {
        Skip {
            kind,
            condition: None,
            reason: None,
            comments: comments_beside(self.source, call),
            span: self.lines.locate(call.span.start, call.span.end),
        }
    }

    /// Register `const test = base.extend({…})` as a fixture-carrying root.
    fn register_extend_binding(&mut self, declarator: &VariableDeclarator<'_>) {
        use oxc_ast::ast::BindingPattern;
        let (Some(init), BindingPattern::BindingIdentifier(binding)) =
            (&declarator.init, &declarator.id)
        else {
            return;
        };
        let Some(chain) = resolve_chain(init) else {
            return;
        };
        if self.roots.contains_key(chain.root)
            && chain
                .modifiers
                .iter()
                .any(|modifier| modifier.name == "extend")
        {
            self.roots.insert(
                binding.name.as_str().to_owned(),
                RootInfo { fixtures: true },
            );
        }
    }
}

/// The text of a string literal or a substitution-free template literal, or `None` for
/// any other expression: a reason the reader can quote, never one it would have to compute.
fn string_text(expression: &Expression<'_>) -> Option<String> {
    match expression.get_inner_expression() {
        Expression::StringLiteral(literal) => literal.value.as_str().map(str::to_owned),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => template
            .quasis
            .first()
            .and_then(|quasi| {
                quasi
                    .value
                    .cooked
                    .unwrap_or_else(|| quasi.value.raw.into())
                    .as_str()
            })
            .map(str::to_owned),
        _ => None,
    }
}

/// Whether a Playwright chain modifier names a lifecycle hook rather than a case.
fn is_hook(name: &str) -> bool {
    matches!(name, "beforeEach" | "afterEach" | "beforeAll" | "afterAll")
}

/// Whether `call` is a `test.beforeEach(…)` / `test.beforeAll(…)`: a hook that sets up
/// what the cases after it see.
fn is_before_hook(call: &CallExpression<'_>) -> bool {
    resolve_chain(&call.callee).is_some_and(|chain| {
        chain
            .modifiers
            .iter()
            .any(|modifier| matches!(modifier.name, "beforeEach" | "beforeAll"))
    })
}

/// The `SkipKind` a Playwright modifier name maps to, when it is one.
fn skip_kind(name: &str) -> Option<SkipKind> {
    match name {
        "skip" => Some(SkipKind::Skip),
        "fixme" => Some(SkipKind::Fixme),
        "fail" => Some(SkipKind::Fails),
        "slow" => Some(SkipKind::Slow),
        "only" => Some(SkipKind::Only),
        _ => None,
    }
}

/// The run-mode modifiers (`serial`, `parallel`) a chain carries — kept as plain
/// modifier strings, since they change how a suite runs, not whether it runs.
fn mode_modifiers(names: &[&str]) -> Vec<String> {
    names
        .iter()
        .filter(|name| matches!(**name, "serial" | "parallel"))
        .map(|name| (*name).to_owned())
        .collect()
}

/// The tags a case declares: the `tag` option (`test('t', { tag: '@smoke' }, fn)`, a
/// string or an array), joined with the `@token`s Playwright reads out of the title.
/// Sorted and deduplicated so the row is stable.
fn tags_of(call: &CallExpression<'_>, title: Option<&str>) -> Vec<String> {
    let mut tags = Vec::new();
    for argument in &call.arguments {
        if let Some(Expression::ObjectExpression(object)) = argument.as_expression() {
            for property in &object.properties {
                if let ObjectPropertyKind::ObjectProperty(property) = property {
                    if property_is(&property.key, "tag") {
                        collect_tag_values(&property.value, &mut tags);
                    }
                }
            }
        }
    }
    if let Some(title) = title {
        for token in title.split_whitespace() {
            if token.starts_with('@') && token.len() > 1 {
                tags.push(token.to_owned());
            }
        }
    }
    tags.sort();
    tags.dedup();
    tags
}

/// Whether an object-property key is the plain name `expected`.
fn property_is(key: &PropertyKey<'_>, expected: &str) -> bool {
    match key {
        PropertyKey::StaticIdentifier(id) => id.name == expected,
        PropertyKey::StringLiteral(literal) => literal.value == expected,
        _ => false,
    }
}

/// Push a `tag` value — a string, or every string in an array — onto `tags`.
fn collect_tag_values(value: &Expression<'_>, tags: &mut Vec<String>) {
    match value {
        Expression::StringLiteral(literal) => {
            tags.extend(literal.value.as_str().map(str::to_owned));
        }
        Expression::ArrayExpression(array) => {
            for element in &array.elements {
                if let Some(Expression::StringLiteral(literal)) = element.as_expression() {
                    tags.extend(literal.value.as_str().map(str::to_owned));
                }
            }
        }
        _ => {}
    }
}

/// Read the file's imports into the set of names that mean a Playwright test root.
fn read_roots(program: &Program<'_>) -> HashMap<String, RootInfo> {
    let mut roots = HashMap::new();
    for statement in &program.body {
        let Statement::ImportDeclaration(import) = statement else {
            continue;
        };
        if !import.source.value.as_str().is_some_and(is_playwright) {
            continue;
        }
        let Some(specifiers) = &import.specifiers else {
            continue;
        };
        for specifier in specifiers {
            if let ImportDeclarationSpecifier::ImportSpecifier(imported) = specifier {
                if imported_name(&imported.imported) == "test" {
                    roots.insert(
                        imported.local.name.as_str().to_owned(),
                        RootInfo { fixtures: false },
                    );
                }
            }
        }
    }
    roots
}

/// Whether an import source is Playwright's — the runner, or a component-testing entry.
fn is_playwright(source: &str) -> bool {
    source == "playwright" || source.starts_with("@playwright/")
}

/// The exported name an import specifier binds from, whatever spelling it uses.
fn imported_name<'a>(name: &'a ModuleExportName<'a>) -> &'a str {
    match name {
        ModuleExportName::IdentifierName(id) => id.name.as_str(),
        ModuleExportName::IdentifierReference(id) => id.name.as_str(),
        // oxc's parser rejects a lone surrogate in an export name (a fatal error), so this holds.
        ModuleExportName::StringLiteral(_) => name.name().as_str(),
    }
}
