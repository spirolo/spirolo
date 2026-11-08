//! Pulling a file's accesses to module-scope state out of its AST, for the `shared-state`
//! and `order-dependence` kinds of `flake-candidate`.
//!
//! **State.** A module-scope `let` or `var` bound to a plain name, or a `const` bound to
//! something mutable written in place: an object or array literal, or a `new` `Map`, `Set`,
//! `WeakMap`, `WeakSet`, `Array` or `Object`.
//!
//! **Accesses.** A write is an assignment or update whose target is the binding or a member
//! under it (`count += 1`, `cache.total = 3`, `rows[0] = x`), a `delete` of such a member, a
//! mutating method called on it or on a member under it (`items.push(1)`, `seen.clear()`), or
//! an `Object.assign` into it. Writes are recorded wherever they sit, module scope included,
//! so the rule can tell a binding only tests assign from one other code assigns too. Any other
//! reference, inside a function, is a read. A name a function redeclares (a parameter, a local
//! binding) is that function's own and not the module's, in it and in the functions it holds.
//!
//! **Hooks.** An access inside the callback of a `before*`/`after*` call (bare, or a member
//! such as `test.beforeEach`) is marked with the hook, so a write in a `beforeEach` /
//! `afterEach` reads as a reset between tests, and a read in an `afterAll` as one after
//! every test is done. Which tests a hook reaches is the rule's to decide.

use std::collections::{HashMap, HashSet};

use oxc_ast::ast::{
    ArrowFunctionExpression, AssignmentExpression, AssignmentOperator, AssignmentTarget,
    BindingIdentifier, BindingPattern, CallExpression, Declaration, Expression, Function,
    IdentifierReference, Program, SimpleAssignmentTarget, Statement, UnaryExpression,
    UnaryOperator, UpdateExpression, VariableDeclaration, VariableDeclarationKind,
};
use oxc_ast_visit::{walk, Visit};
use oxc_syntax::scope::ScopeFlags;

use crate::flake::{hook_of, shorten};
use crate::{Hook, LineIndex, StateAccess};

/// The methods that change the array, map or set they are called on.
const MUTATORS: [&str; 13] = [
    "push",
    "pop",
    "shift",
    "unshift",
    "splice",
    "sort",
    "reverse",
    "fill",
    "copyWithin",
    "set",
    "add",
    "delete",
    "clear",
];

/// The constructors whose instance is a mutable collection.
const COLLECTIONS: [&str; 6] = ["Map", "Set", "WeakMap", "WeakSet", "Array", "Object"];

/// Walk `program` and collect every access to its module-scope state, in span order.
pub fn collect(path: &str, source: &str, program: &Program<'_>) -> Vec<StateAccess> {
    let state = module_state(program);
    if state.is_empty() {
        return Vec::new();
    }
    let mut collector = Collector {
        path,
        source,
        lines: LineIndex::new(source),
        state,
        frames: Vec::new(),
        hooks: Vec::new(),
        callbacks: HashMap::new(),
        registrations: Vec::new(),
        accesses: Vec::new(),
    };
    collector.visit_program(program);
    let mut accesses = collector.accesses.clone();
    for (name, hook, span) in &collector.registrations {
        let Some(body) = collector.callbacks.get(name) else {
            continue;
        };
        // A named callback executes in the hook's suite, not in the place where it was
        // declared. Keep the access text, and anchor these scoped copies at registration.
        accesses.extend(
            collector
                .accesses
                .iter()
                .filter(|access| body.start <= access.span.start && access.span.end <= body.end)
                .map(|access| {
                    let mut access = access.clone();
                    access.hook = Some(*hook);
                    access.span = collector.lines.locate(span.start, span.end);
                    access
                }),
        );
    }
    accesses.sort();
    accesses.dedup();
    accesses
}

/// The names of the module-scope bindings that hold state (see the module docs).
fn module_state(program: &Program<'_>) -> HashSet<String> {
    let mut names = HashSet::new();
    for statement in &program.body {
        let declaration = match statement {
            Statement::VariableDeclaration(declaration) => declaration,
            Statement::ExportDeclaration(export) => match &export.declaration {
                Declaration::VariableDeclaration(declaration) => declaration,
                _ => continue,
            },
            _ => continue,
        };
        add_state(declaration, &mut names);
    }
    names
}

/// Add the state bindings one module-scope declaration makes to `names`.
fn add_state(declaration: &VariableDeclaration<'_>, names: &mut HashSet<String>) {
    for declarator in &declaration.declarations {
        let BindingPattern::BindingIdentifier(id) = &declarator.id else {
            continue;
        };
        let holds_state = match declaration.kind {
            VariableDeclarationKind::Let | VariableDeclarationKind::Var => true,
            VariableDeclarationKind::Const => declarator
                .init
                .as_ref()
                .is_some_and(|init| is_mutable(init.get_inner_expression())),
            _ => false,
        };
        if holds_state {
            names.insert(id.name.to_string());
        }
    }
}

/// Whether a `const` initialiser is a value written in place: an object or array literal,
/// or a new collection.
fn is_mutable(init: &Expression<'_>) -> bool {
    match init {
        Expression::ObjectExpression(_) | Expression::ArrayExpression(_) => true,
        Expression::NewExpression(new) => matches!(
            new.callee.get_inner_expression(),
            Expression::Identifier(id) if COLLECTIONS.contains(&id.name.as_str())
        ),
        _ => false,
    }
}

/// The name a member chain or a name roots in: `cache` for `cache.a.b` and `rows[0]`.
fn root_name<'e>(expr: &'e Expression<'_>) -> Option<&'e str> {
    let mut current = expr.get_inner_expression();
    loop {
        current = match current {
            Expression::Identifier(id) => return Some(id.name.as_str()),
            Expression::StaticMemberExpression(member) => member.object.get_inner_expression(),
            Expression::ComputedMemberExpression(member) => member.object.get_inner_expression(),
            _ => return None,
        };
    }
}

/// The name a simple assignment target (a name or a member) roots in.
fn target_root<'e>(target: &'e SimpleAssignmentTarget<'_>) -> Option<&'e str> {
    match target {
        SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => Some(id.name.as_str()),
        SimpleAssignmentTarget::StaticMemberExpression(member) => root_name(&member.object),
        SimpleAssignmentTarget::ComputedMemberExpression(member) => root_name(&member.object),
        _ => None,
    }
}

struct Collector<'s> {
    path: &'s str,
    source: &'s str,
    lines: LineIndex,
    /// The module-scope state bindings.
    state: HashSet<String>,
    /// The names each enclosing function declares, innermost last; empty at module scope.
    frames: Vec<HashSet<String>>,
    /// The hooks whose callbacks enclose the walk, innermost last.
    hooks: Vec<Hook>,
    /// Locally declared callbacks, and each hook's registration of one by name.
    callbacks: HashMap<String, oxc_span::Span>,
    registrations: Vec<(String, Hook, oxc_span::Span)>,
    accesses: Vec<StateAccess>,
}

impl Collector<'_> {
    /// Whether `name`, referenced here, is module-scope state: no enclosing function
    /// redeclares it.
    fn is_state(&self, name: &str) -> bool {
        self.state.contains(name) && !self.frames.iter().any(|f| f.contains(name))
    }

    /// Record an access to `name` at `span` when `name` is module-scope state here.
    fn record(&mut self, name: Option<&str>, span: oxc_span::Span, write: bool) {
        let Some(name) = name.filter(|n| self.is_state(n)) else {
            return;
        };
        self.accesses.push(StateAccess {
            file: self.path.to_owned(),
            span: self.lines.locate(span.start, span.end),
            binding: name.to_owned(),
            write,
            hook: self.hooks.last().copied(),
            replaces: false,
            what: shorten(self.source, span.start, span.end),
        });
    }
}

impl<'a> Visit<'a> for Collector<'_> {
    fn visit_variable_declarator(&mut self, declarator: &oxc_ast::ast::VariableDeclarator<'a>) {
        if self.frames.is_empty() {
            if let (
                BindingPattern::BindingIdentifier(id),
                Some(Expression::ArrowFunctionExpression(arrow)),
            ) = (&declarator.id, &declarator.init)
            {
                self.callbacks.insert(id.name.to_string(), arrow.span);
            }
        }
        walk::walk_variable_declarator(self, declarator);
    }

    fn visit_function(&mut self, func: &Function<'a>, flags: ScopeFlags) {
        if self.frames.is_empty() && func.is_declaration() {
            if let Some(id) = &func.id {
                self.callbacks.insert(id.name.to_string(), func.span);
            }
        }
        self.frames.push(HashSet::new());
        walk::walk_function(self, func, flags);
        self.frames.pop();
    }

    fn visit_arrow_function_expression(&mut self, arrow: &ArrowFunctionExpression<'a>) {
        self.frames.push(HashSet::new());
        walk::walk_arrow_function_expression(self, arrow);
        self.frames.pop();
    }

    fn visit_binding_identifier(&mut self, id: &BindingIdentifier<'a>) {
        if let Some(frame) = self.frames.last_mut() {
            frame.insert(id.name.to_string());
        }
    }

    fn visit_identifier_reference(&mut self, id: &IdentifierReference<'a>) {
        if !self.frames.is_empty() {
            self.record(Some(id.name.as_str()), id.span, false);
        }
    }

    fn visit_assignment_expression(&mut self, assignment: &AssignmentExpression<'a>) {
        let root = match &assignment.left {
            AssignmentTarget::AssignmentTargetIdentifier(id) => Some(id.name.as_str()),
            AssignmentTarget::StaticMemberExpression(member) => root_name(&member.object),
            AssignmentTarget::ComputedMemberExpression(member) => root_name(&member.object),
            _ => None,
        };
        let recorded = self.accesses.len();
        self.record(root, assignment.span, true);
        if assignment.operator == AssignmentOperator::Assign
            && matches!(
                assignment.left,
                AssignmentTarget::AssignmentTargetIdentifier(_)
            )
        {
            if let Some(access) = self.accesses.get_mut(recorded) {
                access.replaces = true;
            }
        }
        walk::walk_assignment_expression(self, assignment);
    }

    fn visit_update_expression(&mut self, update: &UpdateExpression<'a>) {
        self.record(target_root(&update.argument), update.span, true);
        walk::walk_update_expression(self, update);
    }

    fn visit_unary_expression(&mut self, unary: &UnaryExpression<'a>) {
        if unary.operator == UnaryOperator::Delete {
            let root = match unary.argument.get_inner_expression() {
                Expression::StaticMemberExpression(member) => root_name(&member.object),
                Expression::ComputedMemberExpression(member) => root_name(&member.object),
                _ => None,
            };
            self.record(root, unary.span, true);
        }
        walk::walk_unary_expression(self, unary);
    }

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        let written = match call.callee.get_inner_expression() {
            Expression::StaticMemberExpression(member)
                if MUTATORS.contains(&member.property.name.as_str()) =>
            {
                root_name(&member.object)
            }
            Expression::StaticMemberExpression(member)
                if member.property.name == "assign"
                    && matches!(member.object.get_inner_expression(), Expression::Identifier(id) if id.name == "Object") =>
            {
                call.arguments
                    .first()
                    .and_then(|a| a.as_expression())
                    .and_then(root_name)
            }
            _ => None,
        };
        self.record(written, call.span, true);
        let hook = hook_of(&call.callee);
        if let (Some(hook), Some(Expression::Identifier(id))) = (
            hook,
            call.arguments.first().and_then(|arg| arg.as_expression()),
        ) {
            self.registrations
                .push((id.name.to_string(), hook, call.span));
        }
        self.hooks.extend(hook);
        walk::walk_call_expression(self, call);
        if hook.is_some() {
            self.hooks.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;

    /// The `(binding, write, in an each hook, what)` of every access collected from a
    /// snippet.
    fn accesses(source: &str) -> Vec<(String, bool, bool, String)> {
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
        super::collect("a.test.ts", source, &parsed.program)
            .into_iter()
            .map(|a| {
                let each = a.hook.is_some_and(crate::Hook::is_each);
                (a.binding, a.write, each, a.what)
            })
            .collect()
    }

    fn write(binding: &str, what: &str) -> (String, bool, bool, String) {
        (binding.to_owned(), true, false, what.to_owned())
    }

    fn read(binding: &str) -> (String, bool, bool, String) {
        (binding.to_owned(), false, false, binding.to_owned())
    }

    #[test]
    fn a_let_and_a_mutable_const_are_state_and_a_plain_const_is_not() {
        let source = "let n = 0\nconst cache = {}\nconst list = new Map()\nconst k = 3\n\
                      it('a', () => { n += k; cache.a = 1; list.set(1, 2) })\n";
        let found: Vec<_> = accesses(source).into_iter().filter(|a| a.1).collect();
        assert_eq!(
            found,
            [
                write("n", "n += k"),
                write("cache", "cache.a = 1"),
                write("list", "list.set(1, 2)"),
            ]
        );
    }

    #[test]
    fn a_name_a_function_redeclares_is_not_the_modules() {
        let source = "let n = 0\nit('a', () => { let n = 1; n += 1 })\n\
                      it('b', (n) => { n = 2 })\nit('c', () => { expect(n).toBe(0) })\n";
        assert_eq!(accesses(source), [read("n")]);
    }

    #[test]
    fn a_write_in_an_each_hook_is_marked() {
        let source = "let n = 0\nbeforeEach(() => { n = 0 })\nbeforeAll(() => { n = 1 })\n";
        let marks: Vec<bool> = accesses(source)
            .into_iter()
            .filter(|a| a.1)
            .map(|a| a.2)
            .collect();
        assert_eq!(marks, [true, false]);
    }

    #[test]
    fn only_a_plain_assignment_to_the_binding_itself_replaces_it() {
        let source =
            "let n = 0\nlet o = {}\nfunction f() { n = 1; n += 1; n++; o.a = 1; o = {} }\n";
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let replaces: Vec<(String, bool)> = super::collect("a.test.ts", source, &parsed.program)
            .into_iter()
            .filter(|a| a.write && a.span.line == 3)
            .map(|a| (a.what, a.replaces))
            .collect();
        let row = |what: &str, replaces: bool| (what.to_owned(), replaces);
        assert_eq!(
            replaces,
            [
                row("n = 1", true),
                row("n += 1", false),
                row("n++", false),
                row("o.a = 1", false),
                row("o = {}", true)
            ]
        );
    }
}
