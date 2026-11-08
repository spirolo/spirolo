//! The saga recogniser: yield-order step-walk sites into the `SagaWalk` table.
//!
//! Two idioms test a redux-saga generator by stepping it and asserting the yielded effect
//! in order — the brittle style the redux-saga docs steer away from towards
//! `expectSaga(...).run()`:
//!
//! - **`gen.next().value`**: `expect(gen.next().value).toEqual(call(api, 1))`, an assertion
//!   whose subject reaches `.value` off a `.next()` call and whose first matcher argument is
//!   a call to an effect creator imported from `redux-saga/effects` (`call`, `put`, `take`,
//!   …). One step per such assertion, tagged with the creator name.
//! - **`testSaga` chain**: `testSaga(saga).next().put(a).next().isDone()` from
//!   `redux-saga-test-plan`, one step per `.next(…)` in the chain, tagged with the fluent
//!   effect method that follows it (`put`, `isDone`, …).
//!
//! Recognition is by import, never by shape: the recogniser reads the file's imports for
//! `redux-saga/effects` (which effect-creator names are in scope, honouring `import { call
//! as callEffect }`) and for `testSaga` from `redux-saga-test-plan`, and stays silent in a
//! file that imports neither, so a `.next()` on some other iterator is never mistaken for a
//! saga walk.

use std::collections::HashSet;

use oxc_ast::ast::{
    CallExpression, Expression, ImportDeclarationSpecifier, ModuleExportName, Program, Statement,
};
use oxc_ast_visit::{walk, Visit};
use spirolo_model::{FileContext, Recogniser, SagaStepKind, SagaWalk, Span};

/// Recognises redux-saga generator step-walk sites. See the [module docs](self).
#[derive(Debug, Default, Clone, Copy)]
pub struct Saga;

impl Recogniser for Saga {
    fn saga_walks(&self, file: &FileContext<'_>) -> Vec<SagaWalk> {
        let imports = SagaImports::read(file.program);
        if imports.is_empty() {
            return Vec::new();
        }
        let mut collector = Collector {
            path: file.path,
            source: file.source,
            imports,
            rows: Vec::new(),
            consumed: HashSet::new(),
            next_bindings: HashSet::new(),
        };
        collector.visit_program(file.program);
        collector
            .rows
            .sort_by_key(|row| (row.span.start, row.span.end));
        collector.rows
    }
}

/// What a saga test file imported: the effect-creator names in scope from
/// `redux-saga/effects`, and whether `testSaga` is imported from `redux-saga-test-plan`.
#[derive(Default)]
struct SagaImports {
    /// Local names bound to a `redux-saga/effects` export, honouring `as` aliases — the set
    /// a `gen.next().value` matcher argument must be a call to.
    effect_creators: HashSet<String>,
    /// The local name `testSaga` is imported under, when it is, so a chain rooted in it (or
    /// its alias) is recognised.
    test_saga: Option<String>,
}

impl SagaImports {
    /// Read a file's imports into the saga vocabulary it has in scope.
    fn read(program: &Program<'_>) -> Self {
        let mut imports = Self::default();
        for statement in &program.body {
            let Statement::ImportDeclaration(import) = statement else {
                continue;
            };
            let Some(specifiers) = &import.specifiers else {
                continue;
            };
            match import.source.value.as_str() {
                Some("redux-saga/effects") => {
                    for specifier in specifiers {
                        if let ImportDeclarationSpecifier::ImportSpecifier(imported) = specifier {
                            imports
                                .effect_creators
                                .insert(imported.local.name.as_str().to_owned());
                        }
                    }
                }
                Some("redux-saga-test-plan") => {
                    for specifier in specifiers {
                        if let ImportDeclarationSpecifier::ImportSpecifier(imported) = specifier {
                            if module_export_name(&imported.imported) == "testSaga" {
                                imports.test_saga = Some(imported.local.name.as_str().to_owned());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        imports
    }

    /// Whether the file imported neither library, so the recogniser has nothing to claim.
    fn is_empty(&self) -> bool {
        self.effect_creators.is_empty() && self.test_saga.is_none()
    }
}

/// Walks a saga test file, collecting its step-walk sites.
struct Collector<'a> {
    path: &'a str,
    source: &'a str,
    imports: SagaImports,
    rows: Vec<SagaWalk>,
    /// Spans of the calls already consumed by a `testSaga` chain, so the chain is counted
    /// once at its outermost call and not re-counted at each inner call the visitor descends
    /// into. A chain is visited top-down, outermost first, so the outermost call claims the
    /// whole chain's spans before the visitor reaches its inner links.
    consumed: HashSet<(u32, u32)>,
    /// Local names bound to a `.next(…)` result — `const step = gen.next()` — so the common
    /// `expect(step.value).toEqual(call(…))` idiom, where the yield is read through a binding
    /// rather than inline, is recognised. Tracked file-flat and first-seen, in keeping with
    /// the hand-rolled bindings of ADR 0002; the visit is top-down, so a binding is seen
    /// before the assertions that read it.
    next_bindings: HashSet<String>,
}

impl<'a> Visit<'a> for Collector<'a> {
    fn visit_variable_declaration(&mut self, declaration: &oxc_ast::ast::VariableDeclaration<'a>) {
        for declarator in &declaration.declarations {
            if let (Some(name), Some(init)) =
                (declarator.id.get_identifier_name(), &declarator.init)
            {
                if is_next_result(init) {
                    self.next_bindings.insert(name.as_str().to_owned());
                }
            }
        }
        walk::walk_variable_declaration(self, declaration);
    }

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if let Some(walk) = self.gen_next_walk(call) {
            self.rows.push(walk);
        }
        self.collect_test_saga_steps(call);
        walk::walk_call_expression(self, call);
    }
}

impl Collector<'_> {
    /// A `gen.next().value` step-walk, when `call` is an `expect(subject).matcher(arg)` whose
    /// subject reaches `.value` off a `.next()` call and whose first matcher argument is a
    /// call to an imported effect creator. `None` otherwise.
    fn gen_next_walk(&self, call: &CallExpression<'_>) -> Option<SagaWalk> {
        // The outermost call is the matcher: `expect(subject).toEqual(arg)`. Its callee is a
        // static member off an `expect(subject)` call.
        let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
            return None;
        };
        let Expression::CallExpression(root) = member.object.get_inner_expression() else {
            return None;
        };
        if !is_expect_call(root) {
            return None;
        }
        // The subject `expect(<subject>)` must step a generator: `<gen>.next(…).value`
        // inline, or `<step>.value` where `<step>` is a binding of a `.next(…)` result.
        let subject = root.arguments.first()?.as_expression()?;
        if !self.subject_steps_a_generator(subject) {
            return None;
        }
        // The matcher argument must be a call to an imported effect creator.
        let effect = self.effect_creator_call(call.arguments.first()?.as_expression()?)?;
        Some(SagaWalk {
            file: self.path.to_owned(),
            span: Span::locate(self.source, call.span.start, call.span.end),
            kind: SagaStepKind::GenNext,
            effect: Some(effect),
        })
    }

    /// The effect-creator name a matcher argument is a call to, when it is one: a call whose
    /// callee is an identifier in the file's imported `redux-saga/effects` names. `None` for
    /// anything else (a plain object, a call to a non-effect function).
    fn effect_creator_call(&self, argument: &Expression<'_>) -> Option<String> {
        let Expression::CallExpression(call) = argument.get_inner_expression() else {
            return None;
        };
        let Expression::Identifier(callee) = call.callee.get_inner_expression() else {
            return None;
        };
        let name = callee.name.as_str();
        if self.imports.effect_creators.contains(name) {
            Some(name.to_owned())
        } else {
            None
        }
    }

    /// Whether a subject expression reads a generator's yielded value: `<...>.value` off a
    /// `.next(…)` call inline (`gen.next().value`, `gen.next(x).value`), or `<step>.value`
    /// where `<step>` is a local binding of a `.next(…)` result. A deeper read of the value
    /// (`step.value.type`) is not a yield-order assertion against an effect and does not
    /// count — the PRD's idiom is `expect(<yield>).toEqual(<effect>)`.
    fn subject_steps_a_generator(&self, subject: &Expression<'_>) -> bool {
        let Expression::StaticMemberExpression(member) = subject.get_inner_expression() else {
            return false;
        };
        if member.property.name != "value" {
            return false;
        }
        match member.object.get_inner_expression() {
            // Inline: `gen.next(...).value`.
            Expression::CallExpression(call) => is_next_call(call),
            // Bound: `step.value` where `const step = gen.next(...)`.
            Expression::Identifier(id) => self.next_bindings.contains(id.name.as_str()),
            _ => false,
        }
    }

    /// Collect one step per `.next(…)` in a `testSaga` chain whose outermost call is `call`.
    ///
    /// The chain is read only from its outermost call, so the whole
    /// `testSaga(s).next().put(a).next().isDone()` expression is counted once. The chain is
    /// walked twice: first to confirm it roots in `testSaga(…)` (else a `.next()` on some
    /// other fluent builder would be counted), then to record each `.next(…)` step tagged
    /// with the fluent effect method that follows it in source order.
    fn collect_test_saga_steps(&mut self, call: &CallExpression<'_>) {
        let Some(name) = self.imports.test_saga.as_deref() else {
            return;
        };
        // A chain is visited outermost-first; once its outermost call has claimed the whole
        // chain's spans, an inner link the visitor later reaches is already consumed and does
        // not re-count the chain.
        if self.consumed.contains(&(call.span.start, call.span.end)) {
            return;
        }
        if !chain_roots_in_test_saga(call, name) {
            return;
        }
        // Walk the callee chain, recording each `.next(...)` step with the effect method that
        // immediately follows it (the `.method` of the call one link out) and consuming every
        // call in the chain. The chain is built inside-out, so the steps come out in reverse
        // source order; the final sort fixes it.
        self.consumed.insert((call.span.start, call.span.end));
        let mut following = property_name(call);
        let mut node: &Expression<'_> = &call.callee;
        loop {
            match node.get_inner_expression() {
                Expression::StaticMemberExpression(member) => node = &member.object,
                Expression::CallExpression(inner) => {
                    self.consumed.insert((inner.span.start, inner.span.end));
                    if is_next_call(inner) {
                        self.rows.push(SagaWalk {
                            file: self.path.to_owned(),
                            span: Span::locate(self.source, inner.span.start, inner.span.end),
                            kind: SagaStepKind::TestSaga,
                            effect: following.clone(),
                        });
                    }
                    following = property_name(inner);
                    node = &inner.callee;
                }
                _ => break,
            }
        }
    }
}

/// Whether the callee chain of `call` bottoms out at a `testSaga(…)` call under `name`.
fn chain_roots_in_test_saga(call: &CallExpression<'_>, name: &str) -> bool {
    let mut node: &Expression<'_> = &call.callee;
    loop {
        match node.get_inner_expression() {
            Expression::StaticMemberExpression(member) => node = &member.object,
            Expression::CallExpression(inner) => {
                if let Expression::Identifier(id) = inner.callee.get_inner_expression() {
                    if id.name.as_str() == name {
                        return true;
                    }
                }
                node = &inner.callee;
            }
            _ => return false,
        }
    }
}

/// Whether an `expect(subject)` call opens an assertion: the callee is the identifier
/// `expect`.
fn is_expect_call(call: &CallExpression<'_>) -> bool {
    matches!(
        call.callee.get_inner_expression(),
        Expression::Identifier(id) if id.name == "expect"
    )
}

/// Whether a call is a `<something>.next(…)` call.
fn is_next_call(call: &CallExpression<'_>) -> bool {
    matches!(
        call.callee.get_inner_expression(),
        Expression::StaticMemberExpression(member) if member.property.name == "next"
    )
}

/// Whether an initializer is a `<gen>.next(…)` call, so `const step = gen.next()` binds
/// `step` to a stepped yield the assertions below can read as `step.value`.
fn is_next_result(init: &Expression<'_>) -> bool {
    matches!(
        init.get_inner_expression(),
        Expression::CallExpression(call) if is_next_call(call)
    )
}

/// The property name of a call whose callee is a static member (`x.put(a)` → `put`).
fn property_name(call: &CallExpression<'_>) -> Option<String> {
    match call.callee.get_inner_expression() {
        Expression::StaticMemberExpression(member) => {
            Some(member.property.name.as_str().to_owned())
        }
        _ => None,
    }
}

/// A module export name as written.
fn module_export_name<'a>(name: &'a ModuleExportName<'a>) -> &'a str {
    match name {
        ModuleExportName::IdentifierName(id) => id.name.as_str(),
        ModuleExportName::IdentifierReference(id) => id.name.as_str(),
        // oxc's parser rejects a lone surrogate in an export name (a fatal error), so this holds.
        ModuleExportName::StringLiteral(_) => name.name().as_str(),
    }
}
