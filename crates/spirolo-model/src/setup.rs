//! Reading a runner's setup files for the four fail-closed invariants.
//!
//! `fail-closed-setup` asks whether a suite fails closed on the four ways a test passes for
//! the wrong reason: an unhandled request slips through, a logged error scrolls past, a mock
//! leaks into the next test, or the mock server is started per test instead of once. Three of
//! the four live in a setup file's body, not in the config object the gate already read, so
//! this step re-reads each Vitest/Jest gate's `setup_files` as source — the same way
//! [`crate::presets`] re-reads a config to follow its `mergeConfig` chain — and records what
//! it found onto [`crate::Gate::setup`]. The fourth, mock reset, also reads the gate's own
//! `restoreMocks`.
//!
//! The AST patterns, all found without executing anything:
//!
//! - **unhandled requests error**: a call anywhere in a setup file passing
//!   `{ onUnhandledRequest: "error" }` (the option MSW's `server.listen` takes), so a request
//!   no handler matches fails the test rather than warning.
//! - **console guard**: a `console.error`/`console.warn` spy whose implementation throws
//!   (`spyOn(console, 'error').mockImplementation(() => { throw … })`), a console method
//!   assigned a function that throws at its top level (`global.console.error = msg => { throw msg }`), or an
//!   import of a fail-on-console package (`vitest-fail-on-console`, `jest-fail-on-console`),
//!   so a logged error fails the test. A replacement that filters and forwards to the original,
//!   or only silences, is not a guard. A replacement that records the calls is a guard when a
//!   hook reads that recorder and can fail through a throw or assertion; `recorder` holds
//!   that scope-aware rule.
//! - **mock reset**: `restoreMocks: true` in the gate, or a `restoreAllMocks()` call inside
//!   an `afterEach` in a setup file, so a mock set in one test cannot leak into the next.
//! - **global server**: a `.listen(` call inside a `beforeAll` in a setup file, so the mock
//!   server runs for every test rather than being started case by case.
//!
//! **MSW presence.** The two request invariants apply only when the runner uses MSW, and
//! that is decided per gate: a `server.listen()` in one of the gate's own setup files (its
//! own or a preset's) or an MSW handler file under the config's directory counts as present;
//! a handler in another package of the monorepo does not. A gate with neither has both
//! request invariants [`crate::Invariant::NotApplicable`], so it is never told to configure a
//! mock server it does not use. When the server is started by a preset's setup file the
//! invariants record that ([`crate::SetupInvariants::msw_inherited`]), so a rule can lift the
//! gap to the preset.
//!
//! **Per test, not per file.** The mock-reset invariant is a `restoreAllMocks()` in an
//! `afterEach`; one in an `afterAll` runs once per file and lets a mock set in one test leak
//! into the next, so it does not count.
//!
//! Nothing here is executed: the setup files are parsed as source and read the way every other
//! model step reads a file.

use std::path::{Path, PathBuf};

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Argument, ArrowFunctionBody, AssignmentExpression, AssignmentTarget, CallExpression,
    Expression, Function, ObjectPropertyKind, PropertyKey, Statement, VariableDeclarator,
};
use oxc_ast_visit::{walk, Visit};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};

use crate::{GateTool, HandlerLibrary, Invariant, Model, SetupInvariants};

mod recorder;

/// Read the four fail-closed invariants of every Vitest/Jest gate from its setup files and
/// its `restoreMocks`, filling [`crate::Gate::setup`]. Mutates `model` in place; runs after
/// the gates and handlers are built so MSW presence can be read from the handler table.
pub fn analyze(model: &mut Model, module_resolver: &crate::resolve::WorkspaceResolver) {
    let root = model.root.clone();
    // The MSW handler files, so a gate can ask whether one lives under its own directory.
    let msw_handler_files: Vec<&str> = model
        .handlers
        .iter()
        .filter(|handler| handler.library == HandlerLibrary::Msw)
        .map(|handler| handler.file.as_str())
        .collect();

    for gate in &mut model.gates {
        if !matches!(gate.tool, GateTool::Vitest | GateTool::Jest) {
            continue;
        }
        let mut invariants = SetupInvariants::unread();
        // The mock-reset invariant is half in the config: `restoreMocks: true` on the gate is
        // as good as an `afterEach(restoreAllMocks)`.
        let restore_in_config = gate.restore_mocks == Some(true);

        // Read every setup file the gate loads, unioning their signals, and keep the
        // inherited ones' apart so a server a preset starts is known to be the preset's.
        let mut signals = FileSignals::default();
        let mut inherited_signals = FileSignals::default();
        let mut own_file: Option<String> = None;
        let mut inherited_file: Option<String> = None;
        for spelling in &gate.setup_files {
            let Some(resolved) = resolve_setup(&root, &gate.source, spelling) else {
                continue;
            };
            let Some(relative) = crate::relative_slash(&root, &resolved) else {
                continue;
            };
            let Ok(source) = std::fs::read_to_string(&resolved) else {
                continue;
            };
            let file_signals =
                scan_with_imports(&source, &resolved, Some((&root, module_resolver)));
            let inherited = gate.inherited_setup_files.contains(spelling);
            let first = if inherited {
                &mut inherited_file
            } else {
                &mut own_file
            };
            if first.is_none() {
                *first = Some(relative);
            }
            if inherited {
                inherited_signals.union(file_signals);
            }
            signals.union(file_signals);
        }

        let handler_in_package = msw_handler_files
            .iter()
            .any(|file| under_config_dir(&gate.source, file));
        let msw_present = handler_in_package || signals.server_listen;
        invariants.msw_present = msw_present;
        invariants.msw_inherited = inherited_signals.server_listen;
        // A finding on this gate points at the config's own setup file when it has one, else
        // at the first inherited one; the rule falls back to the config when there is none.
        invariants.setup_file = own_file.or(inherited_file);
        invariants.console_guard = present(signals.console_guard);
        invariants.restore_mocks = present(restore_in_config || signals.restore_all_mocks);
        // The two MSW invariants are judged only when a mock server is present.
        invariants.unhandled_request_error = if msw_present {
            present(signals.unhandled_request_error)
        } else {
            Invariant::NotApplicable
        };
        invariants.global_server = if msw_present {
            present(signals.global_server)
        } else {
            Invariant::NotApplicable
        };
        gate.setup = invariants;
    }
}

/// Whether `file` sits under the directory of the config at `config_source`: a root config
/// (no directory) owns the whole tree, `packages/a/vitest.config.ts` owns `packages/a/…`.
fn under_config_dir(config_source: &str, file: &str) -> bool {
    match config_source.rsplit_once('/') {
        Some((dir, _)) => file
            .strip_prefix(dir)
            .is_some_and(|rest| rest.starts_with('/')),
        None => true,
    }
}

/// [`Invariant::Present`] when the signal was seen, [`Invariant::Absent`] otherwise.
const fn present(seen: bool) -> Invariant {
    if seen {
        Invariant::Present
    } else {
        Invariant::Absent
    }
}

/// Resolve a setup-file spelling from the config against the config's directory: a relative
/// path joins the config's directory; an absolute one is taken as is. A bare package specifier
/// (no `.`/`/` lead) is not a file the model reads, so it is skipped.
fn resolve_setup(root: &Path, config_source: &str, spelling: &str) -> Option<PathBuf> {
    if spelling.starts_with('.') {
        let config_dir = Path::new(config_source).parent();
        let joined = match config_dir {
            Some(dir) if !dir.as_os_str().is_empty() => root.join(dir).join(spelling),
            _ => root.join(spelling),
        };
        return Some(normalise(&joined));
    }
    if spelling.starts_with('/') {
        return Some(PathBuf::from(spelling));
    }
    // `<rootDir>/…` (Jest) and a plain project-relative path both resolve from the root.
    let trimmed = spelling.trim_start_matches("<rootDir>/");
    if trimmed.contains('/') {
        return Some(normalise(&root.join(trimmed)));
    }
    None
}

/// Collapse `.` and `..` segments so a joined path names one file the reader can open.
fn normalise(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        use std::path::Component;
        match component {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// What one setup file's body carries — the raw signals before MSW presence gates them.
///
/// Five independent booleans, one per thing the scanner looks for; they are not a state that a
/// flag enum would model better, so the `struct_excessive_bools` lint is allowed here.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct FileSignals {
    /// A call passing `{ onUnhandledRequest: "error" }`.
    unhandled_request_error: bool,
    /// A throwing console spy, or a fail-on-console package import — the two idioms decidable
    /// from a single call. The deferred-assertion idiom (a `spyOn(console, …)` collected and
    /// inspected in an `afterEach` that throws) is derived from `console_spy` and
    /// `after_hook_throws` after the walk.
    console_guard: bool,
    /// A `spyOn(console, "error"|"warn")` anywhere in the file, whatever it does with the
    /// calls. A console method replaced by assignment is read separately, as a recorder.
    console_spy: bool,
    /// A `throw` inside a per-test hook (`beforeEach`, `afterEach`) or an `afterAll`, directly
    /// or through a function the file declares — the deferred half of a console guard that
    /// records calls and fails on them around each test.
    after_hook_throws: bool,
    /// A `restoreAllMocks()` call inside an `afterEach`.
    restore_all_mocks: bool,
    /// A `.listen(` call inside a `beforeAll`.
    global_server: bool,
    /// Any `.listen(` call — evidence the file starts a mock server, so MSW is present.
    server_listen: bool,
}

impl FileSignals {
    /// Fold `other`'s signals into this one, so several setup files union.
    const fn union(&mut self, other: Self) {
        self.unhandled_request_error |= other.unhandled_request_error;
        self.console_guard |= other.console_guard;
        self.console_spy |= other.console_spy;
        self.after_hook_throws |= other.after_hook_throws;
        self.restore_all_mocks |= other.restore_all_mocks;
        self.global_server |= other.global_server;
        self.server_listen |= other.server_listen;
    }
}

/// Scan the supplied setup source without following imported helpers, for unit fixtures.
#[cfg(test)]
fn scan(source: &str, path: &Path) -> FileSignals {
    scan_with_imports(source, path, None)
}

fn scan_with_imports(
    source: &str,
    path: &Path,
    imports: Option<(&Path, &crate::resolve::WorkspaceResolver)>,
) -> FileSignals {
    let mut signals = FileSignals {
        console_guard: imports_fail_on_console(source),
        ..FileSignals::default()
    };
    let Ok(source_type) = SourceType::from_path(path) else {
        return signals;
    };
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();
    let mut visitor = Scanner {
        source,
        signals: &mut signals,
        in_before_all: 0,
        in_after_each: 0,
        hooks: Vec::new(),
        hook_stack: Vec::new(),
        throwing_functions: Vec::new(),
    };
    visitor.visit_program(&parsed.program);
    // The spy half pairs only with an after hook that throws, inline or through a helper
    // the file declares (carbon's `formatConsoleCallStack`).
    let hook_throws = visitor.hooks.iter().any(|hook| {
        hook.after
            && (visitor.span_throws(hook.span)
                || hook
                    .calls
                    .iter()
                    .any(|name| visitor.throwing_functions.contains(name)))
    });
    // A recorder guard: a hook (a per-test `beforeEach` too) reads what the replaced console
    // recorded on the way to a failure, inline or through a helper it calls. A hook that fails
    // for another reason, or only clears the recorder, does not make the replacement a guard.
    let recorder_guard = recorder::recorder_guard(&parsed.program, path, imports);
    drop(visitor);
    signals.after_hook_throws |= hook_throws;
    signals.console_guard |= recorder_guard;
    // The deferred-assertion console guard: a `spyOn(console, …)` collected in a `beforeAll`
    // and inspected in a per-test hook that throws on an unexpected call. Neither half alone
    // is a guard — a bare spy that swallows is not, and a hook that throws for another reason
    // is not — but a file that both spies on the console and throws in such a hook is the
    // pattern a common console-guard fix uses.
    if signals.console_spy && signals.after_hook_throws {
        signals.console_guard = true;
    }
    signals
}

/// One per-test hook (`beforeEach`, `afterEach`) or `afterAll`: its callback's span and the
/// plain functions called inside it, in case one is a throwing helper.
struct Hook {
    span: oxc_span::Span,
    calls: Vec<String>,
    /// An `afterEach`/`afterAll`, the only hooks a `spyOn(console, …)` pairs with.
    after: bool,
}

/// Whether the source imports a fail-on-console package (`import 'vitest-fail-on-console'`,
/// `require('jest-fail-on-console')`), read from the raw text so both module systems and a
/// bare side-effect import count.
fn imports_fail_on_console(source: &str) -> bool {
    source.contains("fail-on-console")
}

/// The AST walk: it tracks whether it is inside a `beforeAll` or an `afterEach` so a
/// `.listen()` or `restoreAllMocks()` is credited to the right hook, and reads every call for
/// the `onUnhandledRequest` option and a throwing console spy.
struct Scanner<'a> {
    source: &'a str,
    signals: &'a mut FileSignals,
    in_before_all: usize,
    in_after_each: usize,
    /// Every `beforeEach`/`afterEach`/`afterAll` hook seen.
    hooks: Vec<Hook>,
    /// Indices into `hooks` of the hooks the walk is currently inside.
    hook_stack: Vec<usize>,
    /// The functions the file declares by name whose body throws.
    throwing_functions: Vec<String>,
}

impl<'a> Visit<'a> for Scanner<'a> {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        self.read_call(call);

        // Track the hook a nested call sits inside, so `.listen()` in a `beforeAll` and
        // `restoreAllMocks()` in an `afterEach` are attributed to their hook.
        let hook = hook_name(call);
        let before_all = hook == Some("beforeAll") || hook == Some("beforeEach");
        // Only `afterEach` runs per test: an `afterAll` restore leaks mocks across tests.
        let after_each = hook == Some("afterEach");
        let checking_hook = is_checking_hook(call);
        if checking_hook {
            if let Some(callback) = call.arguments.first().and_then(Argument::as_expression) {
                self.hooks.push(Hook {
                    span: callback.span(),
                    calls: Vec::new(),
                    after: is_after_hook(call),
                });
                self.hook_stack.push(self.hooks.len() - 1);
            }
        }
        let pushed = checking_hook && call.arguments.first().is_some_and(Argument::is_expression);
        if before_all {
            self.in_before_all += 1;
        }
        if after_each {
            self.in_after_each += 1;
        }

        walk::walk_call_expression(self, call);

        if before_all {
            self.in_before_all -= 1;
        }
        if after_each {
            self.in_after_each -= 1;
        }
        if pushed {
            self.hook_stack.pop();
        }
    }

    fn visit_function(&mut self, function: &Function<'a>, flags: oxc_syntax::scope::ScopeFlags) {
        if let Some(id) = &function.id {
            if self.span_throws(function.span) {
                self.throwing_functions.push(id.name.as_str().to_owned());
            }
        }
        walk::walk_function(self, function, flags);
    }

    fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
        // `const report = () => { throw … }` names a throwing helper as a declaration does.
        if let (Some(id), Some(init)) = (declarator.id.get_identifier_name(), &declarator.init) {
            if matches!(
                init.get_inner_expression(),
                Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)
            ) && self.body_throws(init)
            {
                self.throwing_functions.push(id.as_str().to_owned());
            }
        }
        walk::walk_variable_declarator(self, declarator);
    }

    fn visit_assignment_expression(&mut self, assignment: &AssignmentExpression<'a>) {
        // `console.error = fn` (or `global.console.error`): a replaced console method whose
        // body throws is a guard by assignment rather than by spy.
        if assigns_console_method(&assignment.left) {
            // A replacement that records the calls is read by `recorder`.
            if throws_on_every_call(&assignment.right) {
                self.signals.console_guard = true;
            }
        }
        walk::walk_assignment_expression(self, assignment);
    }
}

impl Scanner<'_> {
    /// Read one call for every signal it may carry.
    fn read_call(&mut self, call: &CallExpression<'_>) {
        // `onUnhandledRequest: "error"` anywhere in the file — the option is passed to
        // `server.listen`, but reading it from any call is enough and robust to how the server
        // is named.
        if call_sets_unhandled_error(call) {
            self.signals.unhandled_request_error = true;
        }

        // A `.listen(` call: evidence of a mock server (MSW presence), and the global-server
        // invariant when it sits inside a `beforeAll`.
        if is_listen_call(call) {
            self.signals.server_listen = true;
            if self.in_before_all > 0 {
                self.signals.global_server = true;
            }
        }

        // `restoreAllMocks()` inside an `afterEach` resets mocks between tests; inside an
        // `afterAll` it runs once per file and does not.
        if is_restore_all_mocks(call) && self.in_after_each > 0 {
            self.signals.restore_all_mocks = true;
        }

        // A throwing console spy: `spyOn(console, 'error'|'warn').mockImplementation(fn)` whose
        // body throws.
        if self.is_throwing_console_spy(call) {
            self.signals.console_guard = true;
        }

        // A bare `spyOn(console, 'error'|'warn')` — one half of the deferred-assertion guard,
        // paired with a throwing after hook below.
        if call_is_console_spy(call) {
            self.signals.console_spy = true;
        }

        // A plain function called inside a hook, in case it is a throwing helper.
        if let Expression::Identifier(id) = call.callee.get_inner_expression() {
            for &index in &self.hook_stack {
                self.hooks[index].calls.push(id.name.as_str().to_owned());
            }
        }
    }

    /// Whether `call` is a `…mockImplementation(fn)` (or `mockImplementationOnce`) whose object
    /// chain roots in a `spyOn(console, "error"|"warn")` and whose function body throws.
    fn is_throwing_console_spy(&self, call: &CallExpression<'_>) -> bool {
        let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
            return false;
        };
        let property = member.property.name.as_str();
        if property != "mockImplementation" && property != "mockImplementationOnce" {
            return false;
        }
        if !chain_spies_on_console(&member.object) {
            return false;
        }
        // The implementation must throw: read its source text for a `throw`.
        call.arguments
            .first()
            .and_then(Argument::as_expression)
            .is_some_and(|argument| self.body_throws(argument))
    }

    /// Whether an implementation argument's source text contains a `throw`, so a spy that
    /// re-logs or swallows the call is not mistaken for a guard.
    fn body_throws(&self, argument: &Expression<'_>) -> bool {
        self.span_throws(argument.span())
    }

    /// Whether the source text of a span contains a `throw` keyword.
    fn span_throws(&self, span: oxc_span::Span) -> bool {
        let start = span.start as usize;
        let end = span.end as usize;
        self.source
            .get(start..end)
            .is_some_and(|text| text.contains("throw"))
    }
}

/// Whether a console replacement is a function whose body throws at its top level, so every
/// call it does not return from early fails the test. A `throw` nested in a branch rethrows
/// one message and forwards the rest (react-devtools' wrapper), and the word in a filter
/// string (`'expected to be thrown'`) is no throw at all: neither is a guard.
fn throws_on_every_call(replacement: &Expression<'_>) -> bool {
    let body = match replacement.get_inner_expression() {
        Expression::ArrowFunctionExpression(arrow) => match &arrow.body {
            ArrowFunctionBody::FunctionBody(body) => &**body,
            _ => return false,
        },
        Expression::FunctionExpression(function) => match &function.body {
            Some(body) => &**body,
            None => return false,
        },
        _ => return false,
    };
    body.statements
        .iter()
        .any(|statement| matches!(statement, Statement::ThrowStatement(_)))
}

/// Whether an assignment target is a console `error`/`warn` method: `console.error`,
/// `global.console.warn`, `globalThis.console.error` or `window.console.error`, or a computed
/// `console[methodName]` whose key is not a literal naming another method (carbon's loop
/// over `['error', 'warn', …]`).
fn assigns_console_method(target: &AssignmentTarget<'_>) -> bool {
    match target {
        AssignmentTarget::StaticMemberExpression(member) => {
            matches!(member.property.name.as_str(), "error" | "warn") && is_console(&member.object)
        }
        AssignmentTarget::ComputedMemberExpression(member) => {
            let key_is_error_or_warn = match member.expression.get_inner_expression() {
                Expression::StringLiteral(literal) => {
                    literal.value == "error" || literal.value == "warn"
                }
                _ => true,
            };
            key_is_error_or_warn && is_console(&member.object)
        }
        _ => false,
    }
}

/// Whether an expression names the console: `console`, or `console` on a global object.
fn is_console(object: &Expression<'_>) -> bool {
    match object.get_inner_expression() {
        Expression::Identifier(id) => id.name == "console",
        Expression::StaticMemberExpression(member) => {
            member.property.name == "console"
                && matches!(
                    member.object.get_inner_expression(),
                    Expression::Identifier(id)
                        if matches!(id.name.as_str(), "global" | "globalThis" | "window")
                )
        }
        _ => false,
    }
}

/// Whether a call is a hook that can fail a test on what the previous one logged: a
/// per-test `beforeEach`/`afterEach`, or an `afterAll`.
fn is_checking_hook(call: &CallExpression<'_>) -> bool {
    matches!(
        hook_name(call),
        Some("beforeEach" | "afterEach" | "afterAll")
    )
}

/// Whether a call is an `afterEach` or `afterAll` hook.
fn is_after_hook(call: &CallExpression<'_>) -> bool {
    matches!(hook_name(call), Some("afterEach" | "afterAll"))
}

/// Whether a call is `jest.fn()` or `vi.fn()`, a mock that records its calls on itself.
fn is_mock_fn(call: &CallExpression<'_>) -> bool {
    matches!(
        call.callee.get_inner_expression(),
        Expression::StaticMemberExpression(member)
            if member.property.name == "fn"
                && matches!(
                    member.object.get_inner_expression(),
                    Expression::Identifier(id) if matches!(id.name.as_str(), "jest" | "vi")
                )
    )
}

/// Whether a call is a hook (`beforeAll`, `afterEach`, …, or `global.afterEach`), returning
/// its name.
fn hook_name<'a>(call: &CallExpression<'a>) -> Option<&'a str> {
    let name = match call.callee.get_inner_expression() {
        Expression::Identifier(id) => id.name.as_str(),
        Expression::StaticMemberExpression(member)
            if matches!(
                member.object.get_inner_expression(),
                Expression::Identifier(id) if matches!(id.name.as_str(), "global" | "globalThis")
            ) =>
        {
            member.property.name.as_str()
        }
        _ => return None,
    };
    matches!(name, "beforeAll" | "beforeEach" | "afterEach" | "afterAll").then_some(name)
}

/// Whether a call passes `{ onUnhandledRequest: "error" }` as one of its arguments.
fn call_sets_unhandled_error(call: &CallExpression<'_>) -> bool {
    call.arguments.iter().any(|argument| {
        argument
            .as_expression()
            .is_some_and(object_sets_unhandled_error)
    })
}

/// Whether an expression is an object literal with `onUnhandledRequest: "error"`.
fn object_sets_unhandled_error(expression: &Expression<'_>) -> bool {
    let Expression::ObjectExpression(object) = expression.get_inner_expression() else {
        return false;
    };
    object.properties.iter().any(|property| {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return false;
        };
        if key_name(&property.key).as_deref() != Some("onUnhandledRequest") {
            return false;
        }
        matches!(
            property.value.get_inner_expression(),
            Expression::StringLiteral(literal) if literal.value == "error"
        )
    })
}

/// Whether a call's callee is a `.listen` member access (`server.listen(…)`).
fn is_listen_call(call: &CallExpression<'_>) -> bool {
    matches!(
        call.callee.get_inner_expression(),
        Expression::StaticMemberExpression(member) if member.property.name == "listen"
    )
}

/// Whether a call is `restoreAllMocks()` on `vi` or `jest` (`vi.restoreAllMocks()`).
fn is_restore_all_mocks(call: &CallExpression<'_>) -> bool {
    let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
        return false;
    };
    member.property.name == "restoreAllMocks"
}

/// Whether a member-expression object chain roots in `spyOn(console, "error"|"warn")` — either
/// `vi.spyOn(console, …)`, `jest.spyOn(console, …)` or a bare `spyOn(console, …)`.
fn chain_spies_on_console(object: &Expression<'_>) -> bool {
    match object.get_inner_expression() {
        Expression::CallExpression(call) => call_is_console_spy(call),
        Expression::StaticMemberExpression(member) => chain_spies_on_console(&member.object),
        _ => false,
    }
}

/// Whether a call is `spyOn(console, "error"|"warn")` (or `vi`/`jest`-qualified).
fn call_is_console_spy(call: &CallExpression<'_>) -> bool {
    let is_spy_on = match call.callee.get_inner_expression() {
        Expression::Identifier(id) => id.name == "spyOn",
        Expression::StaticMemberExpression(member) => member.property.name == "spyOn",
        _ => false,
    };
    if !is_spy_on {
        return false;
    }
    let mut arguments = call.arguments.iter().filter_map(Argument::as_expression);
    let target = arguments.next();
    let method = arguments.next();
    let on_console = matches!(
        target.map(Expression::get_inner_expression),
        Some(Expression::Identifier(id)) if id.name == "console"
    );
    let on_error_or_warn = matches!(
        method.map(Expression::get_inner_expression),
        Some(Expression::StringLiteral(literal)) if literal.value == "error" || literal.value == "warn"
    );
    on_console && on_error_or_warn
}

/// A property key's plain name: a static identifier or a string literal.
fn key_name(key: &PropertyKey<'_>) -> Option<String> {
    match key {
        PropertyKey::StaticIdentifier(id) => Some(id.name.as_str().to_owned()),
        PropertyKey::StringLiteral(literal) => literal.value.as_str().map(str::to_owned),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::scan;
    use std::path::Path;

    fn signals(source: &str) -> super::FileSignals {
        scan(source, Path::new("setup.ts"))
    }

    #[test]
    fn onunhandledrequest_error_is_read_from_a_listen_call() {
        let s = signals("server.listen({ onUnhandledRequest: 'error' })\n");
        assert!(s.unhandled_request_error);
        assert!(s.server_listen);
    }

    #[test]
    fn onunhandledrequest_warn_is_not_the_error_invariant() {
        let s = signals("server.listen({ onUnhandledRequest: 'warn' })\n");
        assert!(!s.unhandled_request_error);
        assert!(s.server_listen);
    }

    #[test]
    fn a_bare_listen_is_a_server_but_not_the_error_invariant() {
        let s = signals("beforeAll(() => server.listen())\n");
        assert!(!s.unhandled_request_error);
        assert!(s.server_listen);
        assert!(s.global_server);
    }

    #[test]
    fn a_listen_outside_a_hook_is_not_the_global_server_invariant() {
        let s = signals("server.listen()\n");
        assert!(s.server_listen);
        assert!(
            !s.global_server,
            "a top-level listen is not the beforeAll idiom"
        );
    }

    #[test]
    fn a_throwing_console_spy_is_a_guard() {
        let s = signals(
            "vi.spyOn(console, 'error').mockImplementation(() => { throw new Error('no logs') })\n",
        );
        assert!(s.console_guard);
    }

    #[test]
    fn a_non_throwing_console_spy_is_not_a_guard() {
        let s = signals("vi.spyOn(console, 'error').mockImplementation(() => {})\n");
        assert!(!s.console_guard, "a swallowing spy does not fail the test");
    }

    #[test]
    fn a_jest_qualified_throwing_spy_is_a_guard() {
        let s = signals(
            "jest.spyOn(console, 'warn').mockImplementation(() => { throw new Error('x') })\n",
        );
        assert!(s.console_guard);
    }

    #[test]
    fn a_fail_on_console_import_is_a_guard() {
        let s = signals("import 'vitest-fail-on-console'\n");
        assert!(s.console_guard);
    }

    #[test]
    fn a_deferred_spy_asserted_in_a_throwing_after_each_is_a_guard() {
        // A common console-guard fix idiom: spy on the console in a beforeAll, collect the spies,
        // and throw in an afterEach when an unexpected call was recorded.
        let s = signals(
            "const spies = []\n\
             beforeAll(() => { spies.push(vi.spyOn(console, 'error')) })\n\
             afterEach(() => { for (const spy of spies) { if (spy.mock.calls.length) { throw new Error('unexpected console') } } })\n",
        );
        assert!(
            s.console_guard,
            "a spy asserted in a throwing afterEach is a console guard"
        );
    }

    #[test]
    fn a_console_spy_with_no_throwing_after_hook_is_not_a_guard() {
        // A spy that is never asserted on (no throwing after hook) is not a guard.
        let s = signals("beforeAll(() => { vi.spyOn(console, 'error') })\n");
        assert!(!s.console_guard);
        assert!(s.console_spy);
        assert!(!s.after_hook_throws);
    }

    #[test]
    fn a_throwing_after_each_without_a_console_spy_is_not_a_guard() {
        // An afterEach that throws for an unrelated reason, with no console spy, is not a guard.
        let s = signals("afterEach(() => { if (leaked) throw new Error('cleanup') })\n");
        assert!(!s.console_guard);
        assert!(!s.console_spy);
        assert!(s.after_hook_throws);
    }

    #[test]
    fn restore_all_mocks_in_after_each_is_the_reset_invariant() {
        let s = signals("afterEach(() => { vi.restoreAllMocks() })\n");
        assert!(s.restore_all_mocks);
    }

    #[test]
    fn restore_all_mocks_in_after_all_is_not_the_per_test_reset() {
        // `afterAll` runs once per file: a mock set in test 1 still leaks into test 2.
        let s = signals("afterAll(() => { vi.restoreAllMocks() })\n");
        assert!(
            !s.restore_all_mocks,
            "the reset invariant is per test — afterEach, not afterAll"
        );
    }

    #[test]
    fn restore_all_mocks_outside_a_hook_is_not_the_reset_invariant() {
        let s = signals("vi.restoreAllMocks()\n");
        assert!(
            !s.restore_all_mocks,
            "the reset invariant is a per-test afterEach, not a one-off call"
        );
    }

    #[test]
    fn an_empty_setup_carries_no_signal() {
        let s = signals("export const x = 1\n");
        assert_eq!(s, super::FileSignals::default());
    }
}
