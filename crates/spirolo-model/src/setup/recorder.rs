//! The recorder half of a console guard, read by binding rather than by name.
//!
//! A setup file can replace `console.error` with a function that records each call into an
//! array (`calls.push(args)`, or `store.calls.push(args)`), or with a `jest.fn()`/`vi.fn()`
//! mock that records calls on itself. The replacement alone is no guard. The file guards
//! when a per-test hook (`beforeEach`, `afterEach`) or an `afterAll`, together with every
//! function it reaches, both:
//!
//! - **reads** the recorder: a reference that resolves to the recorder's own declaration
//!   (not a same-named local or parameter that shadows it), used as a value. Clearing it
//!   (`calls.length = 0`, `calls.splice(0)`, reassigning it) or pushing to it is a write,
//!   not a read. A mock is read through `console.<method>.mock` or an assertion on the mock.
//! - **fails**: an escaping `throw`, or a completed synchronous `expect(recorder).matcher()`
//!   assertion. Local `try … catch` blocks and deferred callbacks (`setTimeout`, a promise
//!   `then`) cannot turn a swallowed or late failure into a guard.
//!
//! "Reaches" follows every mention of a function (`afterEach(check)`, `check()`,
//! `check.call(null)`, `[check].forEach(f => f())`, `calls.forEach(c => …)`), cycle-safe,
//! through local functions, static object methods, and named/default/namespace ES imports
//! from the project. The existing module resolver selects the imported file and its export
//! selects the helper. A function mentioned inside a local `try … catch` or handed to a
//! deferring call still reads; only its failures stop there. Immutable aliases preserve
//! binding and member-path identity; simple call arguments also bind parameters. An alias
//! or default export of a recorder's member (`const n = calls.length`) is the value copied
//! at load, not the recorder, so reading it is no read.
//!
//! Known imprecision, accepted: the read and the throw are not tied to each other, so
//! `if (calls) { if (leak) throw … }` counts, and any completed matcher on the recorder
//! counts as its check, even `expect(calls).toBeDefined()`. Dynamic member paths,
//! `CommonJS` helpers, and helpers outside the project remain unread.

use std::collections::{HashMap, HashSet};

use oxc_ast::ast::{
    Argument, ArrowFunctionExpression, AssignmentExpression, AssignmentTarget, BindingIdentifier,
    BlockStatement, CallExpression, CatchClause, Declaration, ExportDeclaration,
    ExportDefaultDeclaration, ExportDefaultDeclarationKind, ExportFromDeclaration,
    ExportNamedDeclaration, Expression, ForInStatement, ForOfStatement, ForStatement,
    FormalParameters, Function, FunctionBody, IdentifierReference, ImportDeclaration,
    ImportDeclarationSpecifier, ObjectExpression, ObjectPropertyKind, Program, Statement,
    StaticMemberExpression, ThrowStatement, TryStatement, VariableDeclaration,
    VariableDeclarationKind, VariableDeclarator,
};
use oxc_ast::StaticPropertyName;
use oxc_ast_visit::{walk, Visit};
use oxc_syntax::scope::ScopeFlags;

mod project;

#[derive(Clone)]
struct Imported {
    source: String,
    /// None for a namespace import, whose first member selects the export.
    name: Option<String>,
}

#[derive(Clone)]
enum Exported {
    Local(Binding, Vec<String>),
    From(String, String),
}

/// What a reference resolves to: the declaration it names (by the binding's span start), or
/// a global the file never declares.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Binding {
    Decl(u32),
    Global(String),
}

/// One reference inside a unit: its binding, the static member path after it
/// (`store.calls.length` is `store` + `["calls", "length"]`), and whether it is a read.
#[derive(Clone)]
struct Ref {
    binding: Binding,
    path: Vec<String>,
    read: bool,
    /// The subject of an assertion whose failure escapes this function.
    asserted: bool,
    /// Whether a function named here could fail this one: not inside a local `try … catch`
    /// and not handed to a deferring call.
    escapes: bool,
}

/// One function body (a hook callback, a helper, a console replacement).
#[derive(Default, Clone)]
struct Unit {
    refs: Vec<Ref>,
    /// Calls and functions passed as arguments, with what each argument names.
    calls: Vec<Call>,
    parameters: Vec<Option<u32>>,
    /// What this function pushes onto: the recorder, when it replaces a console method.
    pushes: Vec<(Binding, Vec<String>)>,
    /// A `throw` no local `try … catch` swallows.
    throws: bool,
    /// Anonymous functions nested in this one (`calls.forEach(c => …)`), and whether their
    /// failures escape it.
    children: Vec<(u32, bool)>,
}

#[derive(Clone)]
struct Call {
    binding: Binding,
    path: Vec<String>,
    arguments: Vec<Option<(Binding, Vec<String>)>>,
    /// Outside any local `try … catch`, so the callee's failure fails this unit.
    escapes: bool,
}

/// A console method's replacement that does not throw on every call.
#[derive(Clone)]
enum Replacement {
    /// An inline function, by its unit.
    Unit(u32),
    /// A function named by reference (`console[m] = patched`).
    Binding(Binding),
    /// A `jest.fn()`/`vi.fn()` mock.
    Mock,
}

/// A hook's callback: inline (its unit) or by reference.
#[derive(Clone)]
enum HookRoot {
    Unit(u32),
    Binding(Binding, Vec<String>),
}

/// Whether a console replacement in this program records calls that some hook reads on the
/// way to a throw it does not swallow. See the module doc for the rule.
pub(super) fn recorder_guard(
    program: &Program<'_>,
    path: &std::path::Path,
    imports: Option<(&std::path::Path, &crate::resolve::WorkspaceResolver)>,
) -> bool {
    let mut walk = Walk::default();
    walk.visit_program(program);
    project::guard(walk, path, imports)
}

/// Whether a reference reads a console mock's calls: `console.error.mock…` or an assertion
/// on the mock, also through `global`/`globalThis`/`window`.
fn reads_console_mock(reference: &Ref) -> bool {
    let path: &[String] = match &reference.binding {
        Binding::Global(name) if name == "console" => &reference.path,
        Binding::Global(name)
            if matches!(name.as_str(), "global" | "globalThis" | "window")
                && reference.path.first().is_some_and(|p| p == "console") =>
        {
            &reference.path[1..]
        }
        _ => return false,
    };
    path.iter().any(|segment| segment == "mock")
        || (reference.asserted && path.len() == 1 && matches!(path[0].as_str(), "error" | "warn"))
}

/// The identifier a static member chain roots in, and the property names after it.
fn member_chain<'b>(expression: &'b Expression<'_>) -> Option<(&'b str, Vec<String>)> {
    match expression.get_inner_expression() {
        Expression::Identifier(id) => Some((id.name.as_str(), Vec::new())),
        Expression::StaticMemberExpression(member) => static_chain(member),
        _ => None,
    }
}

fn static_chain<'b>(member: &'b StaticMemberExpression<'_>) -> Option<(&'b str, Vec<String>)> {
    let (root, mut path) = member_chain(&member.object)?;
    path.push(member.property.name.as_str().to_owned());
    Some((root, path))
}

/// A completed synchronous `expect(subject).matcher(...)` chain. A bare `expect`, a
/// matcher reference, and promise modifiers without a known await do not fail a hook.
fn assertion_subject<'b, 'a>(
    call: &'b CallExpression<'a>,
) -> Option<(&'b str, &'b Expression<'a>)> {
    let Expression::StaticMemberExpression(matcher) = call.callee.get_inner_expression() else {
        return None;
    };
    let mut object = matcher.object.get_inner_expression();
    while let Expression::StaticMemberExpression(modifier) = object {
        if modifier.property.name != "not" {
            return None;
        }
        object = modifier.object.get_inner_expression();
    }
    let Expression::CallExpression(expect) = object else {
        return None;
    };
    let Expression::Identifier(id) = expect.callee.get_inner_expression() else {
        return None;
    };
    Some((id.name.as_str(), expect.arguments.first()?.as_expression()?))
}

/// Whether a call schedules its function arguments to run after the test (`setTimeout`,
/// a promise `then`), so a throw inside them cannot fail it.
fn defers_callbacks(call: &CallExpression<'_>) -> bool {
    match call.callee.get_inner_expression() {
        Expression::Identifier(id) => matches!(
            id.name.as_str(),
            "setTimeout"
                | "setInterval"
                | "setImmediate"
                | "queueMicrotask"
                | "requestAnimationFrame"
        ),
        Expression::StaticMemberExpression(member) => matches!(
            member.property.name.as_str(),
            "then" | "catch" | "finally" | "nextTick"
        ),
        _ => false,
    }
}

/// The single walk: lexical scopes for resolution, and per-function units.
#[derive(Default)]
struct Walk {
    /// Lexical scopes, innermost last: each name with its binding's span start.
    scopes: Vec<Vec<(String, u32)>>,
    units: HashMap<u32, Unit>,
    unit_stack: Vec<u32>,
    /// Per open unit, how many `try` blocks with a `catch` enclose the walk.
    catch_depth: Vec<usize>,
    /// A function binding's declaration → its unit.
    fn_decls: HashMap<u32, u32>,
    /// Statically named object members, kept separate from the object's other methods.
    fn_members: HashMap<Binding, HashMap<Vec<String>, u32>>,
    /// Units passed to a deferring call.
    deferred: HashSet<u32>,
    replacements: Vec<Replacement>,
    hooks: Vec<HookRoot>,
    /// The binding a `const f = () => …` initialiser is about to be named by.
    pending_name: Option<u32>,
    pending_member: Option<(Binding, Vec<String>)>,
    pending_object: Option<(Binding, Vec<String>)>,
    /// Runner-provided `expect`, including an imported local alias.
    expects: HashSet<u32>,
    /// Immutable aliases retain the binding and member path of their initializer.
    aliases: HashMap<Binding, (Binding, Vec<String>)>,
    in_const: bool,
    imports: HashMap<Binding, Imported>,
    exports: HashMap<String, Exported>,
}

impl Walk {
    fn unalias(&self, binding: &Binding, path: &[String]) -> (Binding, Vec<String>) {
        let mut binding = binding.clone();
        let mut path = path.to_vec();
        let mut seen = HashSet::new();
        while seen.insert(binding.clone()) {
            let Some((target, prefix)) = self.aliases.get(&binding) else {
                break;
            };
            binding = target.clone();
            path.splice(..0, prefix.iter().cloned());
        }
        (binding, path)
    }

    fn declare(&mut self, name: &str, at: u32) {
        if let Some(scope) = self.scopes.last_mut() {
            if !scope.iter().any(|(n, a)| n == name && *a == at) {
                scope.push((name.to_owned(), at));
            }
        }
    }

    fn resolve(&self, name: &str) -> Binding {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| {
                scope
                    .iter()
                    .rev()
                    .find(|(n, _)| n == name)
                    .map(|(_, at)| *at)
            })
            .map_or_else(|| Binding::Global(name.to_owned()), Binding::Decl)
    }

    /// Hoist a block's own declarations, so a reference before a `function` or `const` in
    /// the same block resolves to it.
    fn predeclare(&mut self, statements: &[Statement<'_>]) {
        for statement in statements {
            match statement {
                Statement::VariableDeclaration(declaration) => {
                    for declarator in &declaration.declarations {
                        if let Some(id) = declarator.id.get_binding_identifier() {
                            self.declare(id.name.as_str(), id.span.start);
                        }
                    }
                }
                Statement::FunctionDeclaration(function) => {
                    if let Some(id) = &function.id {
                        self.declare(id.name.as_str(), id.span.start);
                    }
                }
                Statement::ExportDeclaration(export) => self.predeclare_export(&export.declaration),
                Statement::ImportDeclaration(import) => {
                    for specifier in import.specifiers.iter().flatten() {
                        let local = match specifier {
                            ImportDeclarationSpecifier::ImportSpecifier(s) => &s.local,
                            ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => &s.local,
                            ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => &s.local,
                        };
                        self.declare(local.name.as_str(), local.span.start);
                    }
                }
                _ => {}
            }
        }
    }

    fn predeclare_export(&mut self, declaration: &Declaration<'_>) {
        match declaration {
            Declaration::FunctionDeclaration(function) => {
                if let Some(id) = &function.id {
                    self.declare(id.name.as_str(), id.span.start);
                }
            }
            Declaration::VariableDeclaration(declaration) => {
                for declarator in &declaration.declarations {
                    if let Some(id) = declarator.id.get_binding_identifier() {
                        self.declare(id.name.as_str(), id.span.start);
                    }
                }
            }
            _ => {}
        }
    }

    fn current(&mut self) -> Option<&mut Unit> {
        let key = *self.unit_stack.last()?;
        self.units.get_mut(&key)
    }

    fn record(&mut self, name: &str, path: Vec<String>, read: bool) {
        let binding = self.resolve(name);
        let escapes = self.catch_depth.last() == Some(&0);
        if let Some(unit) = self.current() {
            unit.refs.push(Ref {
                binding,
                path,
                read,
                asserted: false,
                escapes,
            });
        }
    }

    /// A call of a named function, or a function passed as an argument. `check.call(t, a)`
    /// passes `a` to `check`; `check.apply(t, args)` passes an array this does not read.
    fn record_call(
        &mut self,
        expression: &Expression<'_>,
        mut arguments: Vec<Option<(Binding, Vec<String>)>>,
    ) {
        let escapes = self.catch_depth.last() == Some(&0);
        if let Some((name, path)) = member_chain(expression) {
            match path.last().map(String::as_str) {
                Some("call") if !arguments.is_empty() => {
                    arguments.remove(0);
                }
                Some("apply") => arguments.clear(),
                _ => {}
            }
            let binding = self.resolve(name);
            if let Some(unit) = self.current() {
                unit.calls.push(Call {
                    binding,
                    path,
                    arguments,
                    escapes,
                });
            }
        }
    }

    /// Open a function's unit. A named function is reached only by reference; an anonymous
    /// one is part of the function it sits in.
    fn enter_unit(&mut self, key: u32, named: bool) {
        self.units.entry(key).or_default();
        if !named {
            let escapes = self.catch_depth.last() == Some(&0);
            if let Some(parent) = self.current() {
                parent.children.push((key, escapes));
            }
        }
        self.unit_stack.push(key);
        self.catch_depth.push(0);
        self.scopes.push(Vec::new());
    }

    fn exit_unit(&mut self) {
        self.scopes.pop();
        self.catch_depth.pop();
        self.unit_stack.pop();
    }

    /// Name the function about to open: by its own id, or by the declarator it initialises.
    fn name_unit(&mut self, key: u32, own: Option<&BindingIdentifier<'_>>) -> bool {
        let decl = own
            .map(|id| id.span.start)
            .or_else(|| self.pending_name.take());
        if let Some(decl) = decl {
            self.fn_decls.insert(decl, key);
        }
        let member = self.pending_member.take();
        let named = decl.is_some() || member.is_some();
        if let Some((binding, path)) = member {
            self.fn_members
                .entry(binding)
                .or_default()
                .insert(path, key);
        }
        named
    }
    /// A deferring call's function arguments run after the test (`setTimeout(check)`): their
    /// reads still count, their failures never do.
    fn visit_deferring_call(&mut self, call: &CallExpression<'_>) {
        self.visit_expression(&call.callee);
        for argument in &call.arguments {
            match argument
                .as_expression()
                .map(Expression::get_inner_expression)
            {
                Some(Expression::ArrowFunctionExpression(arrow)) => {
                    self.deferred.insert(arrow.span.start);
                }
                Some(Expression::FunctionExpression(function)) => {
                    self.deferred.insert(function.span.start);
                }
                _ => {}
            }
            let start = self.current().map_or(0, |unit| unit.refs.len());
            self.visit_argument(argument);
            if let Some(unit) = self.current() {
                for reference in &mut unit.refs[start..] {
                    reference.escapes = false;
                }
            }
        }
    }
}

impl<'a> Visit<'a> for Walk {
    fn visit_import_declaration(&mut self, declaration: &ImportDeclaration<'a>) {
        if !declaration.import_kind.is_type() {
            for specifier in declaration.specifiers.iter().flatten() {
                let Some(source) = declaration.source.value.as_str() else {
                    continue;
                };
                let (local, name) = match specifier {
                    ImportDeclarationSpecifier::ImportSpecifier(s) if !s.import_kind.is_type() => {
                        (&s.local, Some(s.imported.name().to_string()))
                    }
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                        (&s.local, Some("default".to_owned()))
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => (&s.local, None),
                    ImportDeclarationSpecifier::ImportSpecifier(_) => continue,
                };
                self.imports.insert(
                    Binding::Decl(local.span.start),
                    Imported {
                        source: source.to_owned(),
                        name,
                    },
                );
            }
        }
        if matches!(
            declaration.source.value.as_str(),
            Some("vitest" | "@jest/globals" | "expect")
        ) {
            for specifier in declaration.specifiers.iter().flatten() {
                if let ImportDeclarationSpecifier::ImportSpecifier(specifier) = specifier {
                    if specifier.imported.name() == "expect" {
                        self.expects.insert(specifier.local.span.start);
                    }
                }
            }
        }
        walk::walk_import_declaration(self, declaration);
    }

    fn visit_export_declaration(&mut self, declaration: &ExportDeclaration<'a>) {
        let mut ids = Vec::new();
        match &declaration.declaration {
            Declaration::FunctionDeclaration(function) => ids.extend(function.id.iter()),
            Declaration::VariableDeclaration(declaration) => {
                ids.extend(
                    declaration
                        .declarations
                        .iter()
                        .filter_map(|d| d.id.get_binding_identifier()),
                );
            }
            _ => {}
        }
        for id in ids {
            self.exports.insert(
                id.name.to_string(),
                Exported::Local(Binding::Decl(id.span.start), Vec::new()),
            );
        }
        walk::walk_export_declaration(self, declaration);
    }

    fn visit_export_named_declaration(&mut self, declaration: &ExportNamedDeclaration<'a>) {
        if !declaration.export_kind.is_type() {
            for specifier in &declaration.specifiers {
                self.exports.insert(
                    specifier.exported.name().to_string(),
                    Exported::Local(self.resolve(specifier.local.name().as_str()), Vec::new()),
                );
            }
        }
        walk::walk_export_named_declaration(self, declaration);
    }

    fn visit_export_from_declaration(&mut self, declaration: &ExportFromDeclaration<'a>) {
        if !declaration.export_kind.is_type() {
            for specifier in &declaration.specifiers {
                let Some(source) = declaration.source.value.as_str() else {
                    continue;
                };
                self.exports.insert(
                    specifier.exported.name().to_string(),
                    Exported::From(source.to_owned(), specifier.local.name().to_string()),
                );
            }
        }
        walk::walk_export_from_declaration(self, declaration);
    }

    fn visit_export_default_declaration(&mut self, declaration: &ExportDefaultDeclaration<'a>) {
        let synthetic = Binding::Decl(declaration.span.start);
        let target = if let Some(expression) = declaration.declaration.as_expression() {
            member_chain(expression).map_or_else(
                || (synthetic.clone(), Vec::new()),
                |(name, path)| (self.resolve(name), path),
            )
        } else {
            (synthetic, Vec::new())
        };
        self.exports.insert(
            "default".to_owned(),
            Exported::Local(target.0.clone(), target.1),
        );
        if let ExportDefaultDeclarationKind::FunctionDeclaration(function) =
            &declaration.declaration
        {
            self.visit_function(function, ScopeFlags::Function);
            if let Binding::Decl(decl) = target.0 {
                self.fn_decls.insert(decl, function.span.start);
            }
        } else {
            if matches!(
                declaration
                    .declaration
                    .as_expression()
                    .map(Expression::get_inner_expression),
                Some(Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_))
            ) {
                self.pending_name = Some(declaration.span.start);
            }
            self.pending_object = Some((target.0, Vec::new()));
            walk::walk_export_default_declaration(self, declaration);
            self.pending_name = None;
            self.pending_object = None;
        }
    }

    fn visit_program(&mut self, program: &Program<'a>) {
        self.scopes.push(Vec::new());
        self.predeclare(&program.body);
        walk::walk_program(self, program);
        self.scopes.pop();
    }

    fn visit_function_body(&mut self, body: &FunctionBody<'a>) {
        self.scopes.push(Vec::new());
        self.predeclare(&body.statements);
        walk::walk_function_body(self, body);
        self.scopes.pop();
    }

    fn visit_block_statement(&mut self, block: &BlockStatement<'a>) {
        self.scopes.push(Vec::new());
        self.predeclare(&block.body);
        walk::walk_block_statement(self, block);
        self.scopes.pop();
    }

    fn visit_for_statement(&mut self, statement: &ForStatement<'a>) {
        self.scopes.push(Vec::new());
        walk::walk_for_statement(self, statement);
        self.scopes.pop();
    }

    fn visit_for_of_statement(&mut self, statement: &ForOfStatement<'a>) {
        self.scopes.push(Vec::new());
        walk::walk_for_of_statement(self, statement);
        self.scopes.pop();
    }

    fn visit_for_in_statement(&mut self, statement: &ForInStatement<'a>) {
        self.scopes.push(Vec::new());
        walk::walk_for_in_statement(self, statement);
        self.scopes.pop();
    }

    fn visit_catch_clause(&mut self, clause: &CatchClause<'a>) {
        self.scopes.push(Vec::new());
        walk::walk_catch_clause(self, clause);
        self.scopes.pop();
    }

    fn visit_binding_identifier(&mut self, id: &BindingIdentifier<'a>) {
        self.declare(id.name.as_str(), id.span.start);
    }

    fn visit_function(&mut self, function: &Function<'a>, flags: ScopeFlags) {
        let key = function.span.start;
        let named = self.name_unit(key, function.id.as_ref());
        self.enter_unit(key, named);
        walk::walk_function(self, function, flags);
        self.exit_unit();
    }

    fn visit_formal_parameters(&mut self, parameters: &FormalParameters<'a>) {
        if let Some(unit) = self.current() {
            unit.parameters = parameters
                .items
                .iter()
                .map(|p| p.pattern.get_binding_identifier().map(|id| id.span.start))
                .collect();
        }
        walk::walk_formal_parameters(self, parameters);
    }

    fn visit_arrow_function_expression(&mut self, arrow: &ArrowFunctionExpression<'a>) {
        let key = arrow.span.start;
        let named = self.name_unit(key, None);
        self.enter_unit(key, named);
        walk::walk_arrow_function_expression(self, arrow);
        self.exit_unit();
    }

    fn visit_variable_declaration(&mut self, declaration: &VariableDeclaration<'a>) {
        let previous = self.in_const;
        self.in_const = declaration.kind == VariableDeclarationKind::Const;
        walk::walk_variable_declaration(self, declaration);
        self.in_const = previous;
    }

    fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
        if self.in_const {
            if let Some(id) = declarator.id.get_binding_identifier() {
                if let Some((root, path)) = declarator.init.as_ref().and_then(member_chain) {
                    self.aliases
                        .insert(Binding::Decl(id.span.start), (self.resolve(root), path));
                }
            }
        }
        let names_function = declarator.init.as_ref().is_some_and(|init| {
            matches!(
                init.get_inner_expression(),
                Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)
            )
        });
        if names_function {
            self.pending_name = declarator
                .id
                .get_binding_identifier()
                .map(|id| id.span.start);
        }
        if matches!(
            declarator
                .init
                .as_ref()
                .map(Expression::get_inner_expression),
            Some(Expression::ObjectExpression(_))
        ) {
            self.pending_object = declarator
                .id
                .get_binding_identifier()
                .map(|id| (Binding::Decl(id.span.start), Vec::new()));
        }
        walk::walk_variable_declarator(self, declarator);
        self.pending_name = None;
        self.pending_object = None;
    }

    fn visit_object_expression(&mut self, object: &ObjectExpression<'a>) {
        let Some((binding, path)) = self.pending_object.take() else {
            walk::walk_object_expression(self, object);
            return;
        };
        for property in &object.properties {
            if let ObjectPropertyKind::ObjectProperty(property) = property {
                if !property.computed {
                    if let Some(name) = property
                        .key
                        .static_name()
                        .and_then(StaticPropertyName::into_cow_str)
                    {
                        let mut member_path = path.clone();
                        member_path.push(name.into_owned());
                        let target = (binding.clone(), member_path);
                        match property.value.get_inner_expression() {
                            Expression::ArrowFunctionExpression(_)
                            | Expression::FunctionExpression(_) => {
                                self.pending_member = Some(target);
                            }
                            Expression::ObjectExpression(_) => self.pending_object = Some(target),
                            _ => {}
                        }
                    }
                }
            }
            self.visit_object_property_kind(property);
            self.pending_member = None;
            self.pending_object = None;
        }
    }

    fn visit_identifier_reference(&mut self, id: &IdentifierReference<'a>) {
        self.record(id.name.as_str(), Vec::new(), true);
    }

    fn visit_static_member_expression(&mut self, member: &StaticMemberExpression<'a>) {
        if let Some((root, path)) = static_chain(member) {
            let root = root.to_owned();
            self.record(&root, path, true);
        } else {
            walk::walk_static_member_expression(self, member);
        }
    }

    fn visit_assignment_expression(&mut self, assignment: &AssignmentExpression<'a>) {
        if super::assigns_console_method(&assignment.left)
            && !super::throws_on_every_call(&assignment.right)
        {
            let replacement = match assignment.right.get_inner_expression() {
                Expression::ArrowFunctionExpression(arrow) => {
                    Some(Replacement::Unit(arrow.span.start))
                }
                Expression::FunctionExpression(function) => {
                    Some(Replacement::Unit(function.span.start))
                }
                Expression::Identifier(id) => {
                    Some(Replacement::Binding(self.resolve(id.name.as_str())))
                }
                Expression::CallExpression(call) if super::is_mock_fn(call) => {
                    Some(Replacement::Mock)
                }
                _ => None,
            };
            if let Some(replacement) = replacement {
                self.replacements.push(replacement);
            }
        }
        // Assigning a name or a member of it (`calls = []`, `calls.length = 0`) writes it.
        let target = match &assignment.left {
            AssignmentTarget::AssignmentTargetIdentifier(id) => {
                Some((id.name.as_str().to_owned(), Vec::new()))
            }
            AssignmentTarget::StaticMemberExpression(member) => {
                static_chain(member).map(|(root, path)| (root.to_owned(), path))
            }
            _ => None,
        };
        if let Some((root, path)) = target {
            self.record(&root, path, false);
            self.visit_expression(&assignment.right);
        } else {
            walk::walk_assignment_expression(self, assignment);
        }
    }

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        let arguments = call
            .arguments
            .iter()
            .map(|argument| {
                let (name, path) = member_chain(argument.as_expression()?)?;
                Some((self.resolve(name), path))
            })
            .collect();
        self.record_call(&call.callee, arguments);
        if !defers_callbacks(call) {
            for argument in &call.arguments {
                if let Some(expression) = argument.as_expression() {
                    self.record_call(expression, Vec::new());
                }
            }
        }
        if let Some((name, subject)) = assertion_subject(call) {
            let runner_expect = match self.resolve(name) {
                Binding::Global(name) => name == "expect",
                Binding::Decl(decl) => self.expects.contains(&decl),
            };
            if runner_expect && self.catch_depth.last() == Some(&0) {
                let start = self.current().map_or(0, |unit| unit.refs.len());
                self.visit_expression(subject);
                if let Some(unit) = self.current() {
                    for reference in &mut unit.refs[start..] {
                        reference.asserted = true;
                    }
                }
                for argument in &call.arguments {
                    self.visit_argument(argument);
                }
                return;
            }
        }
        if super::is_checking_hook(call) {
            match call
                .arguments
                .first()
                .and_then(Argument::as_expression)
                .map(Expression::get_inner_expression)
            {
                Some(Expression::ArrowFunctionExpression(arrow)) => {
                    self.hooks.push(HookRoot::Unit(arrow.span.start));
                }
                Some(Expression::FunctionExpression(function)) => {
                    self.hooks.push(HookRoot::Unit(function.span.start));
                }
                Some(expression) => {
                    if let Some((name, path)) = member_chain(expression) {
                        self.hooks.push(HookRoot::Binding(self.resolve(name), path));
                    }
                }
                _ => {}
            }
        }
        if defers_callbacks(call) {
            self.visit_deferring_call(call);
            return;
        }
        // `calls.push(x)` records into `calls`; `calls.splice(0)` clears it. Both write.
        if let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() {
            let method = member.property.name.as_str();
            if matches!(method, "push" | "splice") {
                if let Some((root, path)) = member_chain(&member.object) {
                    let root = root.to_owned();
                    if method == "push" {
                        let binding = self.resolve(&root);
                        if let Some(unit) = self.current() {
                            unit.pushes.push((binding, path.clone()));
                        }
                    }
                    self.record(&root, path, false);
                    for argument in &call.arguments {
                        self.visit_argument(argument);
                    }
                    return;
                }
            }
        }
        walk::walk_call_expression(self, call);
    }

    fn visit_throw_statement(&mut self, statement: &ThrowStatement<'a>) {
        if self.catch_depth.last() == Some(&0) {
            if let Some(unit) = self.current() {
                unit.throws = true;
            }
        }
        walk::walk_throw_statement(self, statement);
    }

    fn visit_try_statement(&mut self, statement: &TryStatement<'a>) {
        let catches = statement.handler.is_some();
        if catches {
            if let Some(depth) = self.catch_depth.last_mut() {
                *depth += 1;
            }
        }
        self.visit_block_statement(&statement.block);
        if catches {
            if let Some(depth) = self.catch_depth.last_mut() {
                *depth -= 1;
            }
        }
        if let Some(handler) = &statement.handler {
            self.visit_catch_clause(handler);
        }
        if let Some(finalizer) = &statement.finalizer {
            self.visit_block_statement(finalizer);
        }
    }
}
