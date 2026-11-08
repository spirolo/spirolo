//! Pulling a file's flake mechanisms, and the calls that neutralise them, out of its AST.
//!
//! This runs inside [`extract_file`](crate::extract::extract_file) on every module, test or
//! not: which tests a mechanism reaches is a question of scope (a test body, a hook, a setup
//! file, a test-only helper) that only the whole model can answer, so `flake-candidate`
//! decides it and this step only records where each mechanism is written: the file, the span,
//! and the named code it is written in ([`Enclosing`], from [`crate::holders`]), beside the
//! names each named code of the file references, so the rule can follow a test's calls into a
//! test-only module by name.
//!
//! The mechanisms are keyed on globals and on specifiers as written, never on what
//! `node_modules` holds, so a bare worktree at a historical commit gives the same rows as an
//! installed one.
//!
//! **Wall clock.** A read of `Date.now()`, an argument-less `new Date()`, `performance.now()`
//! or `process.hrtime*` is a site only when its value reaches the arguments of an
//! `expect`/`assert` chain together with a second clock read's, or a comparison against a
//! second clock read, in the same function: an elapsed time. One read compared to itself
//! (typed in, then expected back) or to a constant holds under any clock. An assertion that
//! holds the elapsed time at least a bound (`toBeGreaterThanOrEqual`, `.not.toBeLessThan`, or
//! `>=` inside the assertion, read from the side holding the later read) does not count
//! either: a slower machine only makes it pass. The value is followed through local
//! bindings, arithmetic, and the calls whose
//! result is plainly the value again (a method on the clock value, `Math.*`, `Number`,
//! `String`); through any other call it is still followed, but the site is recorded
//! undecided, because what the call returns is not visible here. A comparison in the
//! condition or body exit of a poll loop (one whose condition, split on `&&`, or successful
//! body exit observes awaited state beside its clock and counter comparisons) is a
//! deadline: it caps the wait, and does not count.
//!
//! **Guards.** Fake timers (`vi.useFakeTimers()`, `jest.useFakeTimers()`, a Playwright
//! `clock.install()`) cover the clock and the timers; a system-time set (`vi.setSystemTime`,
//! `clock.setFixedTime`) covers the clock only. A `faker.seed(…)` covers faker; a spy on the
//! source (`vi.spyOn(Math, 'random')`, `vi.spyOn(crypto, 'randomUUID')`), an assignment to
//! `Math.random`, `vi.stubGlobal('crypto', …)` or a module mock of `crypto` / `node:crypto` /
//! `@faker-js/faker` covers that source.
//!
//! **Fixed timeout.** An awaited wait with a literal delay (a number, or arithmetic on
//! numbers): a `new Promise` whose executor calls `setTimeout`, a `setTimeout` imported from
//! `timers/promises`, or a call to one of the file's top-level sleep helpers (a function
//! whose result is one of those, with a literal delay or its own parameter as the delay); and
//! `page.waitForTimeout(<literal>)`, awaited or not. A sleep helper imported from another
//! module is not followed. One in the body of a poll loop spaces its re-reads of the awaited
//! state, and is no fixed wait. Fake timers guard it; a system-time set does not.
//!
//! **Networkidle.** `waitForLoadState('networkidle')`, and `waitUntil: 'networkidle'` in an
//! options object passed to a call (a navigation). Nothing guards it.
//!
//! **Unseeded random.** `Math.random()`, a `crypto` draw (`randomUUID` or `getRandomValues`,
//! on the global, on `globalThis.crypto`, or imported from `crypto` / `node:crypto`), or any call on a `@faker-js/faker` import but its `seed`.
//!
//! **Unawaited promise.** An expression statement that is a dynamic `import()`, a `fetch`
//! (global, `globalThis.` or `window.`), a call to one of the file's top-level `async`
//! functions, or a `.then` chain — unless a link of the chain handles a rejection (`.catch`,
//! a two-argument `.then`). Awaited, returned or `void`ed, it is not a statement of its own.
//! Nothing guards it.
//!
//! **Real I/O.** A call through `fs` / `fs/promises` / `node:fs*` (a function imported by
//! name, or a member of the module, `fs.promises.*` included). Its first argument's path is
//! followed through bindings in scope, innermost function first, so a `beforeEach` that
//! assigns a module-scope `let dir` reaches the tests: built from `os.tmpdir()` or a `mkdtemp`
//! result, it is no site; from literals, `__dirname`, `import.meta`, `process.cwd()` and
//! `path` joins of them, a decided site; from anything else (a parameter, an import, another
//! call), an undecided one. A module mock of an `fs` specifier guards it.
//!
//! **Missing restore.** `spyOn`, `stubGlobal`, `stubEnv` or `useFakeTimers` on `vi` / `jest`.
//! `mockRestore()` / `restoreAllMocks()` / `mockReset()`, `unstubAllGlobals()`,
//! `unstubAllEnvs()` and `useRealTimers()` are its guards, and so is a `stubGlobal` of a global back to its
//! original (`globalThis.Image`, or a module-scope `const` that captured it), which guards
//! that one global and installs nothing; the runner config's `restoreMocks` /
//! `unstubGlobals` / `unstubEnvs` are read by the rule, which knows which config governs the
//! file. Each site records the hook it is installed in, for the rule to tell an install
//! ahead of every test from a leftover.

use std::collections::{BTreeMap, HashMap, HashSet};

use oxc_ast::ast::{
    Argument, ArrowFunctionExpression, AssignmentExpression, AssignmentTarget, AwaitExpression,
    BinaryExpression, BindingPattern, CallExpression, Declaration, Expression, ExpressionStatement,
    ForStatement, FormalParameters, Function, FunctionBody, ImportDeclarationSpecifier,
    NewExpression, ObjectPropertyKind, Program, SimpleAssignmentTarget, Statement,
    UpdateExpression, VariableDeclarator, WhileStatement,
};
use oxc_ast_visit::{walk, Visit};
use oxc_span::GetSpan;
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, LogicalOperator};
use oxc_syntax::scope::ScopeFlags;

use crate::{
    Enclosing, FlakeGuard, FlakeKind, FlakeNames, FlakeSite, FlakeSource, Hook, LineIndex,
    StateAccess,
};

/// The flake mechanisms and guards one file writes, in span order.
#[derive(Debug, Default)]
pub struct FlakeFacts {
    /// Every mechanism written in the file.
    pub sites: Vec<FlakeSite>,
    /// Every call that neutralises one where it is in scope.
    pub guards: Vec<FlakeGuard>,
    /// Every access to the file's module-scope state (see [`crate::state`]).
    pub state: Vec<StateAccess>,
    /// The names the file's code references, by enclosing code (see [`crate::holders`]).
    pub names: FlakeNames,
}

/// Walk `program` and collect its flake sites and guards.
pub fn collect(path: &str, source: &str, program: &Program<'_>) -> FlakeFacts {
    let mut collector = Collector {
        path,
        source,
        lines: LineIndex::new(source),
        frames: vec![HashMap::new()],
        paths: vec![HashMap::new()],
        clock: BTreeMap::new(),
        calendar: BTreeMap::new(),
        escaped: HashSet::new(),
        asserting: 0,
        bounds: HashSet::new(),
        polls: 0,
        sites: Vec::new(),
        guards: Vec::new(),
        imports: Imports::of(program),
        sleeps: HashMap::new(),
        asyncs: async_functions(program),
        delays: HashSet::new(),
        hooks: Vec::new(),
        declared: 0,
        hook_functions: hook_functions(program),
        originals: originals(program),
    };
    collector.sleeps = sleep_helpers(program, &collector.imports);
    collector.visit_program(program);
    let calendar = collector
        .calendar
        .into_iter()
        .filter(|(read, _)| !collector.clock.contains_key(read))
        .filter(|(read, _)| !collector.escaped.contains(read))
        .map(|(read, decided)| (read, decided, true));
    let mut sites: Vec<FlakeSite> = collector
        .clock
        .iter()
        .map(|(read, decided)| (*read, *decided, false))
        .chain(calendar)
        .map(|((start, end), decided, calendar)| FlakeSite {
            file: path.to_owned(),
            span: collector.lines.locate(start, end),
            kind: FlakeKind::WallClock,
            source: FlakeSource::Clock,
            what: shorten(source, start, end),
            decided,
            hook: None,
            target: None,
            writes: false,
            calendar,
            enclosing: Enclosing::Module,
        })
        .collect();
    sites.append(&mut collector.sites);
    let holders = crate::holders::collect(path, program);
    for site in &mut sites {
        site.enclosing = holders.enclosing(site.span.start);
    }
    sites.sort();
    collector.guards.sort();
    FlakeFacts {
        sites,
        guards: collector.guards,
        state: crate::state::collect(path, source, program),
        names: holders.names,
    }
}

/// The clock reads an expression's value carries, each with how the value was followed to
/// here.
type Taint = BTreeMap<(u32, u32), Carried>;

/// How a value carries one clock read.
#[derive(Debug, Clone, Copy, Default)]
struct Carried {
    /// Followed plainly (`true`), or through a call the reader cannot see into (`false`).
    plain: bool,
    /// Through a calendar field of the read (`getFullYear()`, `toDateString()`): a value
    /// that changes at a date boundary.
    calendar: bool,
}

/// Merge `other` into `into`: a read followed plainly, or through a calendar field, on
/// either side stays so.
fn union(into: &mut Taint, other: Taint) {
    for (read, carried) in other {
        let entry = into.entry(read).or_default();
        entry.plain |= carried.plain;
        entry.calendar |= carried.calendar;
    }
}

struct Collector<'s> {
    path: &'s str,
    source: &'s str,
    lines: LineIndex,
    /// One binding map per function being walked, innermost last: the clock reads each
    /// local name carries. A value is followed within its own function only.
    frames: Vec<HashMap<String, Taint>>,
    /// One binding map per function being walked, innermost last: what kind of filesystem
    /// path each local name holds. Looked up innermost first, so a hook's assignment to a
    /// module-scope `let dir` reaches the tests that read `dir`.
    paths: Vec<HashMap<String, PathClass>>,
    /// Every clock read that reached an assertion or a two-read comparison, keyed by its
    /// byte range, with whether any path to it was followed plainly.
    clock: BTreeMap<(u32, u32), bool>,
    /// Every clock read whose calendar field an assertion expects, with whether any path to
    /// it was followed plainly: a site unless the read also reaches other code.
    calendar: BTreeMap<(u32, u32), bool>,
    /// Every clock read a value handed to a call outside an assertion carries: code under
    /// test receives it, so an assertion on it compares the read with itself.
    escaped: HashSet<(u32, u32)>,
    /// How many assertion calls enclose the walk.
    asserting: usize,
    /// Where each clock comparison that only bounds a poll loop starts: a deadline capping
    /// how long the loop waits for the state its condition re-reads.
    bounds: HashSet<u32>,
    /// How many poll loops' bodies enclose the walk in the current function: a wait there
    /// spaces the loop's re-reads of the state it waits for, and is no fixed wait.
    polls: usize,
    /// Every mechanism other than a clock read, recorded where it is written.
    sites: Vec<FlakeSite>,
    guards: Vec<FlakeGuard>,
    imports: Imports,
    /// The file's sleep helpers by name: functions that resolve after a `setTimeout`.
    sleeps: HashMap<String, Delay>,
    /// The names of the file's top-level `async` functions.
    asyncs: HashSet<String>,
    /// Where each `await` a function returns an already-built value right after starts: a
    /// delay a test double adds to its result, which its caller awaits, and not a wait.
    delays: HashSet<u32>,
    /// The hooks whose callbacks enclose the walk, innermost last, each with whether it is
    /// registered inside a declared function: a helper's `describeReal` registers its hook in
    /// whichever suite calls it, which the reader cannot see.
    hooks: Vec<(Hook, bool)>,
    /// How many declared functions (a function declaration, or a function bound to a name)
    /// enclose the walk.
    declared: usize,
    /// The file's top-level functions passed to a hook by name (`beforeEach(applyMocks)`).
    hook_functions: HashMap<String, Hook>,
    /// Module-scope `const`s holding a global as the file loaded it
    /// (`const OriginalImage = global.Image`), each with the global's name.
    originals: HashMap<String, String>,
}

/// The local names a file binds to a randomness source through its imports, keyed on the
/// specifier as written: `@faker-js/faker` and `crypto` / `node:crypto`.
#[derive(Default)]
struct Imports {
    /// Local names of anything imported from `@faker-js/faker` (`faker`, `fakerDE`).
    faker: HashSet<String>,
    /// Local names of the `crypto` module itself (`import crypto from 'node:crypto'`).
    crypto_module: HashSet<String>,
    /// Local names of `randomUUID` / `getRandomValues` imported by name.
    crypto_function: HashSet<String>,
    /// Local names of `setTimeout` imported from `timers/promises` / `node:timers/promises`,
    /// which resolves after the delay it is given.
    timer_promise: HashSet<String>,
    /// Local names of an `fs` module (`import fs from 'node:fs'`, `import * as fs`,
    /// `import { promises as fs }`).
    fs_module: HashSet<String>,
    /// Local names of `fs` functions imported by name (`readFileSync`, `writeFile`).
    fs_function: HashSet<String>,
}

/// What a filesystem path is built from, ordered so the combination of two parts is the
/// greater: a part under a temporary directory makes the whole path private to the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PathClass {
    /// Literals, `__dirname`, `import.meta`, `process.cwd()`: the same path every run.
    Fixed,
    /// A value the reader cannot see the origin of (a parameter, an import, a call).
    Unknown,
    /// Built from `os.tmpdir()` or a `mkdtemp` result.
    Temp,
}

/// Whether `specifier` names Node's filesystem module.
fn is_fs(specifier: &str) -> bool {
    matches!(
        specifier,
        "fs" | "node:fs" | "fs/promises" | "node:fs/promises"
    )
}

impl Imports {
    /// Read the file's import declarations.
    fn of(program: &Program<'_>) -> Self {
        let mut imports = Self::default();
        for statement in &program.body {
            let Statement::ImportDeclaration(declaration) = statement else {
                continue;
            };
            let Some(from) = declaration.source.value.as_str() else {
                continue;
            };
            for specifier in declaration.specifiers.iter().flatten() {
                let local = specifier.local().name.to_string();
                match (from, specifier) {
                    ("@faker-js/faker", _) => {
                        imports.faker.insert(local);
                    }
                    (
                        "crypto" | "node:crypto",
                        ImportDeclarationSpecifier::ImportSpecifier(named),
                    ) => {
                        if is_crypto_draw(named.imported.name().as_str()) {
                            imports.crypto_function.insert(local);
                        }
                    }
                    ("crypto" | "node:crypto", _) => {
                        imports.crypto_module.insert(local);
                    }
                    (from, ImportDeclarationSpecifier::ImportSpecifier(named))
                        if is_fs(from) && named.imported.name() != "promises" =>
                    {
                        imports.fs_function.insert(local);
                    }
                    (from, _) if is_fs(from) => {
                        imports.fs_module.insert(local);
                    }
                    (
                        "timers/promises" | "node:timers/promises",
                        ImportDeclarationSpecifier::ImportSpecifier(named),
                    ) if named.imported.name() == "setTimeout" => {
                        imports.timer_promise.insert(local);
                    }
                    _ => {}
                }
            }
        }
        imports
    }
}

/// The `networkidle` waits a call makes: the call itself when it is
/// `waitForLoadState('networkidle')`, and each `waitUntil: 'networkidle'` property of an
/// options object it is passed (`page.goto(url, { waitUntil: 'networkidle' })`).
fn networkidle_waits(call: &CallExpression<'_>) -> Vec<oxc_span::Span> {
    let mut spans = Vec::new();
    if let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() {
        if member.property.name == "waitForLoadState"
            && string_argument(call, 0) == Some("networkidle")
        {
            spans.push(call.span);
        }
    }
    for argument in &call.arguments {
        let Some(Expression::ObjectExpression(object)) = argument
            .as_expression()
            .map(Expression::get_inner_expression)
        else {
            continue;
        };
        for property in &object.properties {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                continue;
            };
            if property
                .key
                .static_name()
                .is_some_and(|name| name == "waitUntil")
                && matches!(property.value.get_inner_expression(), Expression::StringLiteral(value) if value.value == "networkidle")
            {
                spans.push(property.span);
            }
        }
    }
    spans
}

/// What a timer's delay is written as.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Delay {
    /// A number literal, or arithmetic on number literals (`2 * 1000`).
    Literal,
    /// The helper's parameter at this index, decided at each call.
    Parameter(usize),
    /// Another plain name, resolved against the enclosing helper's parameters.
    Name(String),
    /// Anything else: computed, so not a fixed wait.
    Other,
}

/// Whether a delay is a number literal or arithmetic on number literals.
fn is_literal_delay(expr: &Expression<'_>) -> bool {
    match expr.get_inner_expression() {
        Expression::NumericLiteral(_) => true,
        Expression::BinaryExpression(binary) => {
            is_literal_delay(&binary.left) && is_literal_delay(&binary.right)
        }
        _ => false,
    }
}

/// How a delay expression is written.
fn delay_of(expr: Option<&Expression<'_>>) -> Delay {
    match expr.map(Expression::get_inner_expression) {
        Some(expr) if is_literal_delay(expr) => Delay::Literal,
        Some(Expression::Identifier(id)) => Delay::Name(id.name.to_string()),
        _ => Delay::Other,
    }
}

/// The delay of the `setTimeout` a `new Promise(executor)` waits on, when the executor calls
/// one; `None` for any other `new` expression.
fn promise_delay(new: &NewExpression<'_>, imports: &Imports) -> Option<Delay> {
    if !matches!(new.callee.get_inner_expression(), Expression::Identifier(id) if id.name == "Promise")
    {
        return None;
    }
    let executor = new.arguments.first()?.as_expression()?;
    let mut finder = TimerFinder {
        imports,
        delay: None,
    };
    finder.visit_expression(executor);
    finder.delay
}

/// Finds the first `setTimeout(callback, delay)` call in a subtree.
struct TimerFinder<'i> {
    imports: &'i Imports,
    delay: Option<Delay>,
}

impl<'a> Visit<'a> for TimerFinder<'_> {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if self.delay.is_some() {
            return;
        }
        if let Expression::Identifier(id) = call.callee.get_inner_expression() {
            if id.name == "setTimeout" {
                self.delay = Some(delay_of(
                    call.arguments.get(1).and_then(Argument::as_expression),
                ));
                return;
            }
            if self.imports.timer_promise.contains(id.name.as_str()) {
                self.delay = Some(delay_of(
                    call.arguments.first().and_then(Argument::as_expression),
                ));
                return;
            }
        }
        walk::walk_call_expression(self, call);
    }
}

/// The file's top-level sleep helpers by name: a function whose result is a `new Promise`
/// that waits on a `setTimeout`, or a `timers/promises` `setTimeout`, with a literal delay
/// or one of its own parameters as the delay.
fn sleep_helpers(program: &Program<'_>, imports: &Imports) -> HashMap<String, Delay> {
    let mut helpers = HashMap::new();
    for statement in &program.body {
        let declaration = match statement {
            Statement::ExportDeclaration(export) => Some(&export.declaration),
            other => other.as_declaration(),
        };
        match declaration {
            Some(Declaration::FunctionDeclaration(function)) => {
                if let (Some(id), Some(body)) = (&function.id, &function.body) {
                    let delay = result_delay(&body.statements, imports);
                    record_helper(&mut helpers, id.name.as_str(), &function.params, delay);
                }
            }
            Some(Declaration::VariableDeclaration(variables)) => {
                for declarator in &variables.declarations {
                    let (BindingPattern::BindingIdentifier(id), Some(init)) =
                        (&declarator.id, &declarator.init)
                    else {
                        continue;
                    };
                    let (params, delay) = match init.get_inner_expression() {
                        Expression::ArrowFunctionExpression(arrow) => {
                            (&arrow.params, arrow_result_delay(arrow, imports))
                        }
                        Expression::FunctionExpression(function) => match &function.body {
                            Some(body) => {
                                (&function.params, result_delay(&body.statements, imports))
                            }
                            None => continue,
                        },
                        _ => continue,
                    };
                    record_helper(&mut helpers, id.name.as_str(), params, delay);
                }
            }
            _ => {}
        }
    }
    helpers
}

/// The names of the top-level functions `program` declares `async`: `async function f`,
/// `const f = async () => …`, exported or not.
fn async_functions(program: &Program<'_>) -> HashSet<String> {
    let mut names = HashSet::new();
    for statement in &program.body {
        let declaration = match statement {
            Statement::ExportDeclaration(export) => Some(&export.declaration),
            other => other.as_declaration(),
        };
        match declaration {
            Some(Declaration::FunctionDeclaration(function)) if function.r#async => {
                if let Some(id) = &function.id {
                    names.insert(id.name.to_string());
                }
            }
            Some(Declaration::VariableDeclaration(variables)) => {
                for declarator in &variables.declarations {
                    let (BindingPattern::BindingIdentifier(id), Some(init)) =
                        (&declarator.id, &declarator.init)
                    else {
                        continue;
                    };
                    let is_async = match init.get_inner_expression() {
                        Expression::ArrowFunctionExpression(arrow) => arrow.r#async,
                        Expression::FunctionExpression(function) => function.r#async,
                        _ => false,
                    };
                    if is_async {
                        names.insert(id.name.to_string());
                    }
                }
            }
            _ => {}
        }
    }
    names
}

/// Record `name` as a sleep helper when its result waits on a timer with a literal delay or
/// one of its `params`.
fn record_helper(
    helpers: &mut HashMap<String, Delay>,
    name: &str,
    params: &FormalParameters<'_>,
    delay: Option<Delay>,
) {
    let delay = match delay {
        Some(Delay::Literal) => Delay::Literal,
        Some(Delay::Name(delay)) => {
            let Some(index) = params.items.iter().position(
                |p| matches!(&p.pattern, BindingPattern::BindingIdentifier(id) if id.name.as_str() == delay),
            ) else {
                return;
            };
            Delay::Parameter(index)
        }
        _ => return,
    };
    helpers.insert(name.to_owned(), delay);
}

/// The delay of the timer a function body's result waits on: its `return` (or, for an
/// expression arrow, its one expression) is a `new Promise` over a `setTimeout`, or a
/// `timers/promises` `setTimeout` call, awaited or not.
fn result_delay(statements: &[Statement<'_>], imports: &Imports) -> Option<Delay> {
    statements.iter().find_map(|statement| {
        let expr = match statement {
            Statement::ReturnStatement(ret) => ret.argument.as_ref()?,
            Statement::ExpressionStatement(expression) => &expression.expression,
            _ => return None,
        };
        expression_delay(expr, imports)
    })
}

/// [`result_delay`] for an arrow, whose concise body is the returned expression itself.
fn arrow_result_delay(arrow: &ArrowFunctionExpression<'_>, imports: &Imports) -> Option<Delay> {
    match arrow.get_expression() {
        Some(expr) => expression_delay(expr, imports),
        None => result_delay(&arrow.get_function_body()?.statements, imports),
    }
}

/// The delay one returned (or awaited) expression waits for, if it is a timer promise.
fn expression_delay(expr: &Expression<'_>, imports: &Imports) -> Option<Delay> {
    let expr = match expr.get_inner_expression() {
        Expression::AwaitExpression(inner) => inner.argument.get_inner_expression(),
        other => other,
    };
    match expr {
        Expression::NewExpression(new) => promise_delay(new, imports),
        Expression::CallExpression(call) => match call.callee.get_inner_expression() {
            Expression::Identifier(id) if imports.timer_promise.contains(id.name.as_str()) => Some(
                delay_of(call.arguments.first().and_then(Argument::as_expression)),
            ),
            _ => None,
        },
        _ => None,
    }
}

/// The string literal a call passes at `index`, if it passes one.
fn string_argument<'c>(call: &'c CallExpression<'_>, index: usize) -> Option<&'c str> {
    match call
        .arguments
        .get(index)?
        .as_expression()?
        .get_inner_expression()
    {
        Expression::StringLiteral(literal) => literal.value.as_str(),
        _ => None,
    }
}

/// Whether a `crypto` member draws random values.
fn is_crypto_draw(name: &str) -> bool {
    matches!(
        name,
        "randomUUID" | "getRandomValues" | "randomBytes" | "randomInt"
    )
}

impl Collector<'_> {
    /// The clock reads `expr`'s value carries in the current function.
    fn taint(&self, expr: &Expression<'_>) -> Taint {
        let mut taint = Taint::new();
        match expr.get_inner_expression() {
            Expression::CallExpression(call) => {
                if is_clock_read(call) {
                    taint.insert(
                        (call.span.start, call.span.end),
                        Carried {
                            plain: true,
                            calendar: false,
                        },
                    );
                } else if let Some(receiver) = plain_receiver(call) {
                    taint = self.taint(receiver);
                    if is_calendar_field(call) {
                        for carried in taint.values_mut() {
                            carried.calendar = true;
                        }
                    }
                } else {
                    let plain = is_plain_function(&call.callee);
                    for argument in &call.arguments {
                        if let Some(argument) = argument.as_expression() {
                            let mut inner = self.taint(argument);
                            if !plain {
                                for carried in inner.values_mut() {
                                    carried.plain = false;
                                }
                            }
                            union(&mut taint, inner);
                        }
                    }
                }
            }
            Expression::NewExpression(new) => {
                if matches!(new.callee.get_inner_expression(), Expression::Identifier(id) if id.name == "Date")
                {
                    if new.arguments.is_empty() {
                        taint.insert(
                            (new.span.start, new.span.end),
                            Carried {
                                plain: true,
                                calendar: false,
                            },
                        );
                    } else {
                        for argument in &new.arguments {
                            if let Some(argument) = argument.as_expression() {
                                union(&mut taint, self.taint(argument));
                            }
                        }
                    }
                }
            }
            Expression::Identifier(id) => {
                if let Some(bound) = self.frames.last().and_then(|f| f.get(id.name.as_str())) {
                    taint.clone_from(bound);
                }
            }
            Expression::BinaryExpression(binary) => {
                taint = self.taint(&binary.left);
                union(&mut taint, self.taint(&binary.right));
            }
            Expression::UnaryExpression(unary) => taint = self.taint(&unary.argument),
            Expression::AwaitExpression(inner) => taint = self.taint(&inner.argument),
            Expression::StaticMemberExpression(member) => taint = self.taint(&member.object),
            Expression::ComputedMemberExpression(member) => taint = self.taint(&member.object),
            Expression::LogicalExpression(logical) => {
                taint = self.taint(&logical.left);
                union(&mut taint, self.taint(&logical.right));
            }
            Expression::ConditionalExpression(conditional) => {
                taint = self.taint(&conditional.consequent);
                union(&mut taint, self.taint(&conditional.alternate));
            }
            Expression::TemplateLiteral(template) => {
                for expression in &template.expressions {
                    union(&mut taint, self.taint(expression));
                }
            }
            _ => {}
        }
        taint
    }

    /// Record a decided site of `kind` drawing on `source` at `span`.
    fn site(&mut self, kind: FlakeKind, source: FlakeSource, span: oxc_span::Span) {
        self.sites.push(FlakeSite {
            file: self.path.to_owned(),
            span: self.lines.locate(span.start, span.end),
            kind,
            source,
            what: shorten(self.source, span.start, span.end),
            decided: !self.hooks.last().is_some_and(|(_, nested)| *nested),
            hook: self.hooks.last().map(|(hook, _)| *hook),
            target: None,
            writes: false,
            calendar: false,
            enclosing: Enclosing::Module,
        });
    }

    /// Record the clock reads an assertion chain compares: an elapsed budget, or a calendar
    /// field of a fresh read as the expected value.
    fn assertion(&mut self, call: &CallExpression<'_>) {
        // One read compared to itself (a value typed in and expected back) holds under
        // any clock; an elapsed budget measures between two, anywhere in the chain
        // (`expect(Date.now()).toBeLessThan(deadline)`).
        let (actual, expected) = assertion_arguments(call);
        let mut measured = Taint::new();
        for argument in &actual {
            union(&mut measured, self.taint(argument));
        }
        let mut taint = measured.clone();
        for argument in &expected {
            union(&mut taint, self.taint(argument));
        }
        let calendar = taint.values().all(|carried| carried.calendar);
        let ordering = ordering_of(call);
        if taint.len() >= 2 && (ordering.is_some() || !calendar) {
            // An ordering written in the actual value is the comparison's to judge.
            let compared = actual.iter().any(|argument| {
                matches!(argument.get_inner_expression(),
                    Expression::BinaryExpression(binary) if binary.operator.is_compare())
            });
            // The later read sits on the side that measures; held at least a bound, a
            // slower machine only makes it pass.
            let actual_later = taint
                .keys()
                .next_back()
                .is_some_and(|last| measured.contains_key(last));
            let at_least = ordering.is_some_and(|actual_below| actual_below != actual_later);
            if !compared && !at_least {
                self.reach(taint);
            }
        } else {
            // A calendar field of a fresh read as the expected value: the code under test
            // read the date too, and the two disagree across a date boundary. A read the
            // actual value carries too was handed to the code under test: one read.
            let mut expected_taint = Taint::new();
            for argument in &expected {
                union(&mut expected_taint, self.taint(argument));
            }
            for (read, carried) in expected_taint
                .into_iter()
                .filter(|(read, c)| c.calendar && !measured.contains_key(read))
            {
                let entry = self.calendar.entry(read).or_insert(false);
                *entry |= carried.plain;
            }
        }
    }

    /// Record every clock read `taint` carries as having reached an assertion or comparison.
    fn reach(&mut self, taint: Taint) {
        for (read, carried) in taint {
            let entry = self.clock.entry(read).or_insert(false);
            *entry |= carried.plain;
        }
    }

    /// Bind `name` to the clock reads `taint` carries in the current function, or unbind it.
    fn bind(&mut self, name: &str, taint: Taint) {
        if let Some(frame) = self.frames.last_mut() {
            if taint.is_empty() {
                frame.remove(name);
            } else {
                frame.insert(name.to_owned(), taint);
            }
        }
    }

    /// The randomness source `call` draws from, if it is an unseeded draw: `Math.random()`,
    /// a `crypto` draw (global, imported, or reached as `globalThis.crypto`), or a call on a
    /// `@faker-js/faker` import other than its `seed`.
    fn random_source(&self, call: &CallExpression<'_>) -> Option<FlakeSource> {
        match call.callee.get_inner_expression() {
            Expression::Identifier(id)
                if self.imports.crypto_function.contains(id.name.as_str()) =>
            {
                Some(FlakeSource::Crypto)
            }
            Expression::StaticMemberExpression(member) => {
                let property = member.property.name.as_str();
                let object = member.object.get_inner_expression();
                if property == "random"
                    && matches!(object, Expression::Identifier(id) if id.name == "Math")
                {
                    return Some(FlakeSource::MathRandom);
                }
                if is_crypto_draw(property) && self.is_crypto(object) {
                    return Some(FlakeSource::Crypto);
                }
                if property != "seed" && self.roots_in_faker(object) {
                    return Some(FlakeSource::Faker);
                }
                None
            }
            _ => None,
        }
    }

    /// Whether `object` is the `crypto` module: the global, an import of it, or a
    /// `globalThis.crypto` / `window.crypto` member.
    fn is_crypto(&self, object: &Expression<'_>) -> bool {
        match object {
            Expression::Identifier(id) => {
                id.name == "crypto" || self.imports.crypto_module.contains(id.name.as_str())
            }
            Expression::StaticMemberExpression(member) => member.property.name == "crypto",
            _ => false,
        }
    }

    /// Whether a member chain (`faker.person`, `faker`) roots in a faker import. Calls are not
    /// walked through, so `faker.date.past().toISOString()` is one draw, not two.
    fn roots_in_faker(&self, object: &Expression<'_>) -> bool {
        let mut current = object;
        loop {
            current = match current {
                Expression::StaticMemberExpression(member) => member.object.get_inner_expression(),
                Expression::ComputedMemberExpression(member) => {
                    member.object.get_inner_expression()
                }
                Expression::Identifier(id) => return self.imports.faker.contains(id.name.as_str()),
                _ => return false,
            };
        }
    }

    /// The span of the fixed wait an awaited expression is, if it is one with a literal delay:
    /// a `new Promise` whose executor calls `setTimeout`, a `timers/promises` `setTimeout`, or
    /// a call to one of the file's sleep helpers.
    fn fixed_wait(&self, awaited: &Expression<'_>) -> Option<oxc_span::Span> {
        match awaited.get_inner_expression() {
            Expression::NewExpression(new) => {
                (promise_delay(new, &self.imports) == Some(Delay::Literal)).then_some(new.span)
            }
            Expression::CallExpression(call) => {
                let Expression::Identifier(id) = call.callee.get_inner_expression() else {
                    return None;
                };
                let delay = if self.imports.timer_promise.contains(id.name.as_str()) {
                    Delay::Parameter(0)
                } else {
                    self.sleeps.get(id.name.as_str())?.clone()
                };
                let literal = match delay {
                    Delay::Literal => true,
                    Delay::Parameter(index) => call
                        .arguments
                        .get(index)
                        .and_then(Argument::as_expression)
                        .is_some_and(is_literal_delay),
                    Delay::Name(_) | Delay::Other => false,
                };
                literal.then_some(call.span)
            }
            _ => None,
        }
    }

    /// Whether a loop whose condition is `test`, and whose counters (the names its update
    /// clause or body steps) are `counters`, polls: some operand of the condition, split on
    /// `&&`, re-reads the state the loop waits for, being no comparison of a clock value or a
    /// counter. The clock comparisons beside it then only cap how long the loop waits, and
    /// are marked as bounds: their result decides when the wait stops, not what the test
    /// asserts.
    fn polls(
        &mut self,
        test: &Expression<'_>,
        counters: &HashSet<String>,
        state_exit: bool,
    ) -> bool {
        let mut operands = Vec::new();
        conjuncts(test, &mut operands);
        let counted = |side: &Expression<'_>| matches!(side.get_inner_expression(), Expression::Identifier(id) if counters.contains(id.name.as_str()));
        let mut clocked = Vec::new();
        let mut budgets = 0;
        for operand in &operands {
            if matches!(
                operand.get_inner_expression(),
                Expression::BooleanLiteral(_)
            ) {
                budgets += 1;
            }
            if let Expression::BinaryExpression(binary) = operand.get_inner_expression() {
                if binary.operator.is_compare() {
                    if !(self.taint(&binary.left).is_empty()
                        && self.taint(&binary.right).is_empty())
                    {
                        clocked.push(binary.span.start);
                        budgets += 1;
                    } else if counted(&binary.left) || counted(&binary.right) {
                        budgets += 1;
                    }
                }
            }
        }
        let polls = state_exit || budgets < operands.len();
        if polls {
            self.bounds.extend(clocked);
        }
        polls
    }

    /// A condition or successful body exit observes state. The clock/counter exits
    /// beside it cap the wait, including a deadline that throws on expiry.
    fn loop_polls(&mut self, test: &Expression<'_>, body: &Statement<'_>) -> bool {
        // Exit tests may use clock aliases declared in the body. Read those bindings in
        // a temporary frame before classifying the exits; the real walk still records
        // each mechanism and updates the enclosing function's bindings only once.
        self.frames
            .push(self.frames.last().cloned().unwrap_or_default());
        ClockBindings(self).visit_statement(body);
        let counters = counters(None, body);
        let mut exits = Vec::new();
        exit_conditions(body, &mut exits);
        let state_exit = exits
            .iter()
            .any(|(test, successful)| *successful && self.polls(test, &counters, false));
        let polls = self.polls(test, &counters, state_exit);
        if polls {
            for (test, _) in exits {
                self.polls(test, &counters, true);
            }
        }
        self.frames.pop();
        polls
    }

    /// Walk a loop's `body`, counting it as a poll's when `polls` holds.
    fn visit_loop_body(&mut self, body: &Statement<'_>, polls: bool) {
        self.polls += usize::from(polls);
        self.visit_statement(body);
        self.polls -= usize::from(polls);
    }

    /// What the path `expr` names is built from, followed through the bindings in scope.
    fn path_class(&self, expr: &Expression<'_>) -> PathClass {
        let all = |parts: &mut dyn Iterator<Item = &Expression<'_>>| {
            parts
                .map(|part| self.path_class(part))
                .max()
                .unwrap_or(PathClass::Fixed)
        };
        match expr.get_inner_expression() {
            Expression::StringLiteral(_) => PathClass::Fixed,
            Expression::TemplateLiteral(template) => all(&mut template.expressions.iter()),
            Expression::BinaryExpression(binary) => {
                all(&mut [&binary.left, &binary.right].into_iter())
            }
            Expression::ConditionalExpression(conditional) => {
                all(&mut [&conditional.consequent, &conditional.alternate].into_iter())
            }
            Expression::AwaitExpression(inner) => self.path_class(&inner.argument),
            Expression::Identifier(id) => match id.name.as_str() {
                "__dirname" | "__filename" => PathClass::Fixed,
                name => self
                    .paths
                    .iter()
                    .rev()
                    .find_map(|frame| frame.get(name).copied())
                    .unwrap_or(PathClass::Unknown),
            },
            Expression::StaticMemberExpression(member)
                if matches!(
                    member.object.get_inner_expression(),
                    Expression::ImportMeta(_) | Expression::NewTarget(_)
                ) =>
            {
                PathClass::Fixed
            }
            Expression::NewExpression(new) if matches!(new.callee.get_inner_expression(), Expression::Identifier(id) if id.name == "URL") => {
                all(&mut new.arguments.iter().filter_map(Argument::as_expression))
            }
            Expression::CallExpression(call) => {
                let name = match call.callee.get_inner_expression() {
                    Expression::Identifier(id) => id.name.as_str(),
                    Expression::StaticMemberExpression(member) => member.property.name.as_str(),
                    _ => return PathClass::Unknown,
                };
                match name {
                    "tmpdir" => PathClass::Temp,
                    name if name.starts_with("mkdtemp") => PathClass::Temp,
                    "cwd" => PathClass::Fixed,
                    "join" | "resolve" | "normalize" | "dirname" | "fileURLToPath" => {
                        all(&mut call.arguments.iter().filter_map(Argument::as_expression))
                    }
                    _ => PathClass::Unknown,
                }
            }
            _ => PathClass::Unknown,
        }
    }

    /// Whether `call` goes through `fs`: a function imported by name from it, or a member of
    /// an `fs` module (`fs.writeFileSync`, `fs.promises.readFile`).
    fn is_fs_call(&self, call: &CallExpression<'_>) -> bool {
        let mut current = call.callee.get_inner_expression();
        if let Expression::Identifier(id) = current {
            return self.imports.fs_function.contains(id.name.as_str());
        }
        while let Expression::StaticMemberExpression(member) = current {
            current = member.object.get_inner_expression();
        }
        matches!(current, Expression::Identifier(id) if self.imports.fs_module.contains(id.name.as_str()))
    }

    /// Bind `name` to the path class of `value`: in the innermost frame that already holds
    /// it (an assignment to an outer `let`), or in the current one (a declaration).
    fn bind_path(&mut self, name: &str, value: Option<&Expression<'_>>, declared: bool) {
        let class = value.map_or(PathClass::Unknown, |value| self.path_class(value));
        let frame = if declared {
            self.paths.last_mut()
        } else {
            let found = self
                .paths
                .iter()
                .rposition(|frame| frame.contains_key(name));
            match found {
                Some(index) => self.paths.get_mut(index),
                None => self.paths.first_mut(),
            }
        };
        if let Some(frame) = frame {
            if declared {
                frame.insert(name.to_owned(), class);
            } else {
                let entry = frame.entry(name.to_owned()).or_insert(class);
                *entry = (*entry).max(class);
            }
        }
    }

    /// Whether an expression statement starts a promise and drops it: a dynamic `import()`, a
    /// `fetch`, a call to one of the file's `async` functions, or a `.then` chain, when no
    /// link of the chain handles a rejection (`.catch(…)`, a two-argument `.then`).
    fn drops_promise(&self, expression: &Expression<'_>) -> bool {
        let mut current = expression.get_inner_expression();
        let mut then = false;
        loop {
            match current {
                Expression::ImportExpression(_) => return true,
                Expression::CallExpression(call) => match call.callee.get_inner_expression() {
                    Expression::StaticMemberExpression(member) => {
                        match member.property.name.as_str() {
                            "catch" => return false,
                            "then" if call.arguments.len() >= 2 => return false,
                            "then" => then = true,
                            "finally" => {}
                            "fetch" => {
                                return matches!(member.object.get_inner_expression(),
                                    Expression::Identifier(id) if matches!(id.name.as_str(), "globalThis" | "window"))
                                    || then
                            }
                            _ => return then,
                        }
                        current = member.object.get_inner_expression();
                    }
                    Expression::Identifier(id) => {
                        return then
                            || id.name == "fetch"
                            || self.asyncs.contains(id.name.as_str());
                    }
                    _ => return then,
                },
                _ => return then,
            }
        }
    }

    /// The global a `stubGlobal(name, value)` call puts back to its original: `value` is
    /// the global as the file loaded it, read in place (`globalThis.Image`) or through a
    /// module-scope `const` that captured it.
    fn puts_back(&self, call: &CallExpression<'_>) -> Option<String> {
        let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
            return None;
        };
        if member.property.name != "stubGlobal" {
            return None;
        }
        let name = string_argument(call, 0)?;
        let value = call
            .arguments
            .get(1)?
            .as_expression()?
            .get_inner_expression();
        let original = match value {
            Expression::Identifier(id) => self.originals.get(id.name.as_str()).cloned(),
            other => global_read(other),
        };
        (original.as_deref() == Some(name)).then(|| name.to_owned())
    }

    /// The guard `call` is, if it neutralises a mechanism.
    fn guard_of(&self, call: &CallExpression<'_>) -> Option<FlakeGuard> {
        let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
            return None;
        };
        let property = member.property.name.as_str();
        let object = member.object.get_inner_expression();
        let runner = is_runner(object);
        let covers = match (object, property) {
            (_, "restoreAllMocks") if runner => vec![FlakeSource::Spy],
            (_, "mockRestore" | "mockReset") => vec![FlakeSource::Spy],
            (_, "unstubAllGlobals") if runner => vec![FlakeSource::GlobalStub],
            (_, "unstubAllEnvs") if runner => vec![FlakeSource::EnvStub],
            (_, "useRealTimers") if runner => vec![FlakeSource::FakeTimers],
            (Expression::Identifier(id), "useFakeTimers")
                if matches!(id.name.as_str(), "vi" | "jest" | "sinon") =>
            {
                vec![FlakeSource::Clock, FlakeSource::Timer]
            }
            (_, "setSystemTime") if runner => vec![FlakeSource::Clock],
            (Expression::StaticMemberExpression(clock), "install")
                if clock.property.name == "clock" =>
            {
                vec![FlakeSource::Clock, FlakeSource::Timer]
            }
            (Expression::StaticMemberExpression(clock), "setFixedTime" | "setSystemTime")
                if clock.property.name == "clock" =>
            {
                vec![FlakeSource::Clock]
            }
            (_, "seed") if self.roots_in_faker(object) => vec![FlakeSource::Faker],
            (_, "spyOn") if runner => {
                let target = call.arguments.first().and_then(Argument::as_expression);
                let method = string_argument(call, 1);
                match (target.map(Expression::get_inner_expression), method) {
                    (Some(Expression::Identifier(id)), Some("random")) if id.name == "Math" => {
                        vec![FlakeSource::MathRandom]
                    }
                    (Some(target), Some(method))
                        if is_crypto_draw(method) && self.is_crypto(target) =>
                    {
                        vec![FlakeSource::Crypto]
                    }
                    _ => return None,
                }
            }
            (_, "stubGlobal") if runner && string_argument(call, 0) == Some("crypto") => {
                vec![FlakeSource::Crypto]
            }
            (_, "mock" | "doMock") if runner => match string_argument(call, 0) {
                Some("crypto" | "node:crypto") => vec![FlakeSource::Crypto],
                Some("@faker-js/faker") => vec![FlakeSource::Faker],
                Some(specifier) if is_fs(specifier) => vec![FlakeSource::Fs],
                _ => return None,
            },
            _ => return None,
        };
        Some(FlakeGuard {
            file: self.path.to_owned(),
            span: self.lines.locate(call.span.start, call.span.end),
            covers,
            target: None,
            what: shorten(self.source, call.span.start, call.span.end),
        })
    }
}

impl<'a> Visit<'a> for Collector<'_> {
    fn visit_function(&mut self, func: &Function<'a>, flags: ScopeFlags) {
        let hook = func
            .id
            .as_ref()
            .filter(|_| self.frames.len() == 1)
            .and_then(|id| self.hook_functions.get(id.name.as_str()).copied());
        self.hooks.extend(hook.map(|hook| (hook, false)));
        let declared = usize::from(func.is_declaration());
        self.declared += declared;
        self.frames.push(HashMap::new());
        self.paths.push(HashMap::new());
        let polls = std::mem::take(&mut self.polls);
        walk::walk_function(self, func, flags);
        self.polls = polls;
        self.paths.pop();
        self.frames.pop();
        self.declared -= declared;
        if hook.is_some() {
            self.hooks.pop();
        }
    }

    fn visit_arrow_function_expression(&mut self, arrow: &ArrowFunctionExpression<'a>) {
        self.frames.push(HashMap::new());
        self.paths.push(HashMap::new());
        let polls = std::mem::take(&mut self.polls);
        // A concise body (`() => save()`) is one expression statement whose value the arrow
        // returns: walk the expression, never the statement, so it is not read as dropped.
        match arrow.get_expression() {
            Some(expression) => {
                self.visit_formal_parameters(&arrow.params);
                self.visit_expression(expression);
            }
            _ => walk::walk_arrow_function_expression(self, arrow),
        }
        self.polls = polls;
        self.paths.pop();
        self.frames.pop();
    }

    fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
        let declared = usize::from(matches!(
            declarator
                .init
                .as_ref()
                .map(Expression::get_inner_expression),
            Some(Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_))
        ));
        self.declared += declared;
        walk::walk_variable_declarator(self, declarator);
        self.declared -= declared;
        if let BindingPattern::BindingIdentifier(id) = &declarator.id {
            self.bind_path(id.name.as_str(), declarator.init.as_ref(), true);
        }
        if let (BindingPattern::BindingIdentifier(id), Some(init)) =
            (&declarator.id, &declarator.init)
        {
            let taint = self.taint(init);
            self.bind(id.name.as_str(), taint);
        }
    }

    fn visit_assignment_expression(&mut self, assignment: &AssignmentExpression<'a>) {
        walk::walk_assignment_expression(self, assignment);
        if let AssignmentTarget::StaticMemberExpression(member) = &assignment.left {
            if member.property.name == "random"
                && matches!(member.object.get_inner_expression(), Expression::Identifier(id) if id.name == "Math")
            {
                self.guards.push(FlakeGuard {
                    file: self.path.to_owned(),
                    span: self
                        .lines
                        .locate(assignment.span.start, assignment.span.end),
                    covers: vec![FlakeSource::MathRandom],
                    target: None,
                    what: shorten(self.source, assignment.span.start, assignment.span.end),
                });
            }
        }
        if let AssignmentTarget::AssignmentTargetIdentifier(id) = &assignment.left {
            self.bind_path(id.name.as_str(), Some(&assignment.right), false);
            let taint = self.taint(&assignment.right);
            self.bind(id.name.as_str(), taint);
        }
    }

    fn visit_expression_statement(&mut self, statement: &ExpressionStatement<'a>) {
        // Module scope is no test body or hook: only a statement inside a function counts.
        if self.frames.len() > 1 && self.drops_promise(&statement.expression) {
            self.site(
                FlakeKind::UnawaitedPromise,
                FlakeSource::Promise,
                statement.expression.span(),
            );
        }
        walk::walk_expression_statement(self, statement);
    }

    fn visit_function_body(&mut self, body: &FunctionBody<'a>) {
        for pair in body.statements.windows(2) {
            if let [Statement::ExpressionStatement(wait), Statement::ReturnStatement(ret)] = pair {
                if let (Expression::AwaitExpression(wait), Some(value)) =
                    (wait.expression.get_inner_expression(), &ret.argument)
                {
                    if is_built(value) {
                        self.delays.insert(wait.span.start);
                    }
                }
            }
        }
        walk::walk_function_body(self, body);
    }

    fn visit_await_expression(&mut self, await_expression: &AwaitExpression<'a>) {
        if self.delays.contains(&await_expression.span.start) {
            walk::walk_await_expression(self, await_expression);
            return;
        }
        if let Some(span) = self
            .fixed_wait(&await_expression.argument)
            .filter(|_| self.polls == 0)
        {
            self.site(FlakeKind::FixedTimeout, FlakeSource::Timer, span);
        }
        walk::walk_await_expression(self, await_expression);
    }

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        for span in networkidle_waits(call) {
            self.site(FlakeKind::Networkidle, FlakeSource::Network, span);
        }
        if let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() {
            if member.property.name == "waitForTimeout"
                && call
                    .arguments
                    .first()
                    .and_then(Argument::as_expression)
                    .is_some_and(is_literal_delay)
            {
                self.site(FlakeKind::FixedTimeout, FlakeSource::Timer, call.span);
            }
        }
        if is_assertion(&call.callee) {
            self.assertion(call);
        } else if self.asserting == 0 && !is_plain_function(&call.callee) {
            for argument in call.arguments.iter().filter_map(Argument::as_expression) {
                let taint = self.taint(argument);
                self.escaped.extend(taint.into_keys());
            }
        }
        if self.is_fs_call(call) {
            // A copy, move or link touches two paths; it stays in the run's own directory only
            // when both do.
            let paths = if is_two_path(&call.callee) { 2 } else { 1 };
            let classes: Vec<PathClass> = call
                .arguments
                .iter()
                .take(paths)
                .filter_map(Argument::as_expression)
                .map(|path| self.path_class(path))
                .collect();
            let class = classes.iter().copied().min().unwrap_or(PathClass::Fixed);
            // A copy writes only its destination: into a temp dir, it reads its source only.
            let writes = match fs_access(&call.callee) {
                FsAccess::Read => false,
                FsAccess::Copy => classes.get(1) != Some(&PathClass::Temp),
                FsAccess::Write => true,
            };
            if class != PathClass::Temp {
                self.site(FlakeKind::RealIo, FlakeSource::Fs, call.span);
                if let Some(site) = self.sites.last_mut() {
                    site.decided &= class == PathClass::Fixed;
                    site.writes = writes;
                }
            }
        }
        if let Some(source) = installs(call) {
            if let Some(global) = self.puts_back(call) {
                self.guards.push(FlakeGuard {
                    file: self.path.to_owned(),
                    span: self.lines.locate(call.span.start, call.span.end),
                    covers: vec![FlakeSource::GlobalStub],
                    target: Some(global),
                    what: shorten(self.source, call.span.start, call.span.end),
                });
            } else {
                self.site(FlakeKind::MissingRestore, source, call.span);
                if let Some(site) = self.sites.last_mut() {
                    site.target = (source == FlakeSource::GlobalStub)
                        .then(|| string_argument(call, 0).map(str::to_owned))
                        .flatten();
                }
            }
        }
        if let Some(source) = self.random_source(call) {
            self.site(FlakeKind::UnseededRandom, source, call.span);
        }
        if let Some(guard) = self.guard_of(call) {
            self.guards.push(guard);
        }
        let hook = hook_of(&call.callee);
        self.hooks
            .extend(hook.map(|hook| (hook, self.declared > 0)));
        let asserting = usize::from(is_assertion(&call.callee));
        self.asserting += asserting;
        walk::walk_call_expression(self, call);
        self.asserting -= asserting;
        if hook.is_some() {
            self.hooks.pop();
        }
    }

    fn visit_while_statement(&mut self, statement: &WhileStatement<'a>) {
        let polls = self.loop_polls(&statement.test, &statement.body);
        self.visit_expression(&statement.test);
        self.visit_loop_body(&statement.body, polls);
    }

    fn visit_do_while_statement(&mut self, statement: &oxc_ast::ast::DoWhileStatement<'a>) {
        let polls = self.loop_polls(&statement.test, &statement.body);
        self.visit_loop_body(&statement.body, polls);
        self.visit_expression(&statement.test);
    }

    fn visit_for_statement(&mut self, statement: &ForStatement<'a>) {
        if let Some(init) = &statement.init {
            self.visit_for_statement_init(init);
        }
        let counters = counters(statement.update.as_ref(), &statement.body);
        let polls = statement
            .test
            .as_ref()
            .is_some_and(|test| self.polls(test, &counters, false));
        if let Some(test) = &statement.test {
            self.visit_expression(test);
        }
        if let Some(update) = &statement.update {
            self.visit_expression(update);
        }
        self.visit_loop_body(&statement.body, polls);
    }

    fn visit_binary_expression(&mut self, binary: &BinaryExpression<'a>) {
        if self.bounds.contains(&binary.span.start) {
            walk::walk_binary_expression(self, binary);
            return;
        }
        if binary.operator.is_compare() || binary.operator.is_equality() {
            let left = self.taint(&binary.left);
            let mut taint = left.clone();
            union(&mut taint, self.taint(&binary.right));
            // Asserted, the side holding the later read measures; held at least a bound, a
            // slower machine only makes it pass.
            let left_later = taint
                .keys()
                .next_back()
                .is_some_and(|last| left.contains_key(last));
            let left_below = matches!(
                binary.operator,
                BinaryOperator::LessThan | BinaryOperator::LessEqualThan
            );
            let at_least =
                self.asserting > 0 && binary.operator.is_compare() && left_below != left_later;
            if taint.len() >= 2 && !at_least {
                self.reach(taint);
            }
        }
        walk::walk_binary_expression(self, binary);
    }
}

/// Read clock bindings without recording sites or following another function's locals.
struct ClockBindings<'c, 's>(&'c mut Collector<'s>);

impl<'a> Visit<'a> for ClockBindings<'_, '_> {
    fn visit_function(&mut self, _function: &Function<'a>, _flags: ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _arrow: &ArrowFunctionExpression<'a>) {}

    fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
        walk::walk_variable_declarator(self, declarator);
        if let (BindingPattern::BindingIdentifier(id), Some(init)) =
            (&declarator.id, &declarator.init)
        {
            let taint = self.0.taint(init);
            self.0.bind(id.name.as_str(), taint);
        }
    }

    fn visit_assignment_expression(&mut self, assignment: &AssignmentExpression<'a>) {
        walk::walk_assignment_expression(self, assignment);
        if let AssignmentTarget::AssignmentTargetIdentifier(id) = &assignment.left {
            let taint = self.0.taint(&assignment.right);
            self.0.bind(id.name.as_str(), taint);
        }
    }
}

/// Conditional exits of this loop, and whether they succeed rather than throw. A nested
/// loop or function's exits belong to that nested scope and are not followed.
fn exit_conditions<'e, 'a>(body: &'e Statement<'a>, into: &mut Vec<(&'e Expression<'a>, bool)>) {
    fn exits(statement: &Statement<'_>) -> Option<bool> {
        match statement {
            Statement::BreakStatement(statement) if statement.label.is_none() => Some(true),
            Statement::ReturnStatement(_) => Some(true),
            Statement::ThrowStatement(_) => Some(false),
            Statement::BlockStatement(block) => block.body.last().and_then(exits),
            _ => None,
        }
    }
    match body {
        Statement::BlockStatement(block) => {
            for statement in &block.body {
                exit_conditions(statement, into);
            }
        }
        Statement::IfStatement(statement) => {
            if let Some(successful) = exits(&statement.consequent) {
                into.push((&statement.test, successful));
            }
            exit_conditions(&statement.consequent, into);
            if let Some(alternate) = &statement.alternate {
                exit_conditions(alternate, into);
            }
        }
        // A nested loop's break and a nested callback's return do not end this wait.
        _ => {}
    }
}

/// The names a loop steps: those its update clause or its body increments, decrements or
/// updates in place (`i++`, `attempt += 1`).
fn counters(update: Option<&Expression<'_>>, body: &Statement<'_>) -> HashSet<String> {
    #[derive(Default)]
    struct Counters(HashSet<String>);
    impl<'a> Visit<'a> for Counters {
        fn visit_update_expression(&mut self, update: &UpdateExpression<'a>) {
            if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = &update.argument {
                self.0.insert(id.name.to_string());
            }
        }
        fn visit_assignment_expression(&mut self, assignment: &AssignmentExpression<'a>) {
            if let AssignmentTarget::AssignmentTargetIdentifier(id) = &assignment.left {
                if assignment.operator != AssignmentOperator::Assign {
                    self.0.insert(id.name.to_string());
                }
            }
            walk::walk_assignment_expression(self, assignment);
        }
    }
    let mut counters = Counters::default();
    if let Some(update) = update {
        counters.visit_expression(update);
    }
    counters.visit_statement(body);
    counters.0
}

/// The operands of `expr` read as a chain of `&&`, in order.
fn conjuncts<'e, 'a>(expr: &'e Expression<'a>, into: &mut Vec<&'e Expression<'a>>) {
    match expr.get_inner_expression() {
        Expression::LogicalExpression(logical) if logical.operator == LogicalOperator::And => {
            conjuncts(&logical.left, into);
            conjuncts(&logical.right, into);
        }
        _ => into.push(expr),
    }
}

/// Whether `value` was built before it is returned: a literal, a name, a member of one, or
/// an object, array or template of those. A call, or an `await`, reads state as it is at the
/// return, which work that ran during a preceding wait may have changed.
fn is_built(value: &Expression<'_>) -> bool {
    match value.get_inner_expression() {
        Expression::Identifier(_)
        | Expression::ThisExpression(_)
        | Expression::StringLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_) => true,
        Expression::StaticMemberExpression(member) => is_built(&member.object),
        Expression::TemplateLiteral(template) => template.expressions.iter().all(is_built),
        Expression::ArrayExpression(array) => array
            .elements
            .iter()
            .all(|element| element.as_expression().is_some_and(is_built)),
        Expression::ObjectExpression(object) => object.properties.iter().all(|property| {
            matches!(property, ObjectPropertyKind::ObjectProperty(p) if !p.computed && is_built(&p.value))
        }),
        _ => false,
    }
}

/// What a filesystem call does to the paths it names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FsAccess {
    /// Reads them only: `readFile`, `existsSync`, `stat`, `readdir`, a read stream.
    Read,
    /// Reads its first path and writes its second: `copyFile`, `cp`.
    Copy,
    /// Writes, creates, removes or may do either (`open`, `rename`, `mkdir`, `rm`).
    Write,
}

/// What the filesystem call `callee` names does, by its name as written.
fn fs_access(callee: &Expression<'_>) -> FsAccess {
    match fs_function(callee) {
        "readFile" | "readdir" | "readlink" | "realpath" | "exists" | "stat" | "lstat"
        | "statfs" | "access" | "createReadStream" | "opendir" | "watch" | "watchFile" => {
            FsAccess::Read
        }
        "copyFile" | "cp" => FsAccess::Copy,
        _ => FsAccess::Write,
    }
}

/// Whether an `fs` call's callee names a function that takes a source and a destination
/// path: `copyFile`, `cp`, `rename`, `link`, `symlink`, in any of their forms.
fn is_two_path(callee: &Expression<'_>) -> bool {
    matches!(
        fs_function(callee),
        "copyFile" | "cp" | "rename" | "link" | "symlink"
    )
}

/// The `fs` function `callee` names, bare or as a member, without a `Sync` suffix; empty
/// when the callee is neither.
fn fs_function<'e>(callee: &'e Expression<'_>) -> &'e str {
    let name = match callee.get_inner_expression() {
        Expression::Identifier(id) => id.name.as_str(),
        Expression::StaticMemberExpression(member) => member.property.name.as_str(),
        _ => "",
    };
    name.strip_suffix("Sync").unwrap_or(name)
}

/// The hook `callee` registers, bare or as a member (`test.beforeEach`).
pub fn hook_of(callee: &Expression<'_>) -> Option<Hook> {
    let name = match callee.get_inner_expression() {
        Expression::Identifier(id) => id.name.as_str(),
        Expression::StaticMemberExpression(member) => member.property.name.as_str(),
        _ => return None,
    };
    match name {
        "beforeAll" => Some(Hook::BeforeAll),
        "beforeEach" => Some(Hook::BeforeEach),
        "afterEach" => Some(Hook::AfterEach),
        "afterAll" => Some(Hook::AfterAll),
        _ => None,
    }
}

/// The names passed to a hook as its callback in a module-scope statement
/// (`beforeEach(applyMocks)`), each with the hook.
fn hook_functions(program: &Program<'_>) -> HashMap<String, Hook> {
    program
        .body
        .iter()
        .filter_map(|statement| match statement {
            Statement::ExpressionStatement(statement) => match &statement.expression {
                Expression::CallExpression(call) => Some(call),
                _ => None,
            },
            _ => None,
        })
        .filter_map(|call| {
            let hook = hook_of(&call.callee)?;
            match call.arguments.first().and_then(Argument::as_expression) {
                Some(Expression::Identifier(id)) => Some((id.name.to_string(), hook)),
                _ => None,
            }
        })
        .collect()
}

/// The global `expr` reads off the global object: `global.N`, `globalThis.N`, `window.N`.
fn global_read(expr: &Expression<'_>) -> Option<String> {
    let Expression::StaticMemberExpression(member) = expr.get_inner_expression() else {
        return None;
    };
    matches!(
        member.object.get_inner_expression(),
        Expression::Identifier(id) if matches!(id.name.as_str(), "global" | "globalThis" | "window")
    )
    .then(|| member.property.name.to_string())
}

/// The module-scope `const`s that capture a global as the file loads it, each with the
/// global's name: `const OriginalImage = global.Image`.
fn originals(program: &Program<'_>) -> HashMap<String, String> {
    program
        .body
        .iter()
        .filter_map(|statement| match statement {
            Statement::VariableDeclaration(declaration) if declaration.kind.is_const() => {
                Some(declaration)
            }
            _ => None,
        })
        .flat_map(|declaration| &declaration.declarations)
        .filter_map(|declarator| {
            let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                return None;
            };
            let global = global_read(declarator.init.as_ref()?)?;
            Some((id.name.to_string(), global))
        })
        .collect()
}

/// Whether `object` is the runner's mocking namespace, `vi` or `jest`.
fn is_runner(object: &Expression<'_>) -> bool {
    matches!(object, Expression::Identifier(id) if matches!(id.name.as_str(), "vi" | "jest"))
}

/// What `call` installs that needs putting back, if it is `spyOn`, `stubGlobal`, `stubEnv`
/// or `useFakeTimers` on `vi` / `jest`.
fn installs(call: &CallExpression<'_>) -> Option<FlakeSource> {
    let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
        return None;
    };
    if !is_runner(member.object.get_inner_expression()) {
        return None;
    }
    match member.property.name.as_str() {
        "spyOn" => Some(FlakeSource::Spy),
        "stubGlobal" => Some(FlakeSource::GlobalStub),
        "stubEnv" => Some(FlakeSource::EnvStub),
        "useFakeTimers" => Some(FlakeSource::FakeTimers),
        _ => None,
    }
}

/// Whether `call` reads the wall clock: `Date.now()`, `performance.now()`,
/// `process.hrtime()` or `process.hrtime.bigint()`.
fn is_clock_read(call: &CallExpression<'_>) -> bool {
    let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
        return false;
    };
    let property = member.property.name.as_str();
    match member.object.get_inner_expression() {
        Expression::Identifier(id) => matches!(
            (id.name.as_str(), property),
            ("Date" | "performance", "now") | ("process", "hrtime")
        ),
        Expression::StaticMemberExpression(inner) => {
            property == "bigint"
                && inner.property.name == "hrtime"
                && matches!(inner.object.get_inner_expression(), Expression::Identifier(id) if id.name == "process")
        }
        _ => false,
    }
}

/// The receiver of a method call on a value (`end.getTime()`, `elapsed.toFixed(2)`): the
/// result is that value again in another form, so it carries the receiver's reads plainly.
/// `None` for a call on a namespace the reader treats separately (`Math`, `Number`).
fn plain_receiver<'e>(call: &'e CallExpression<'_>) -> Option<&'e Expression<'e>> {
    let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
        return None;
    };
    match member.object.get_inner_expression() {
        Expression::Identifier(id) if matches!(id.name.as_str(), "Math" | "Number" | "String") => {
            None
        }
        _ => Some(&member.object),
    }
}

/// Whether a call to `callee` returns its argument in another form (`Math.round(ms)`,
/// `Number(t)`, `String(t)`), so the value is followed through it plainly.
fn is_plain_function(callee: &Expression<'_>) -> bool {
    match callee.get_inner_expression() {
        Expression::Identifier(id) => matches!(
            id.name.as_str(),
            "Number" | "String" | "BigInt" | "parseInt" | "parseFloat"
        ),
        Expression::StaticMemberExpression(member) => matches!(
            member.object.get_inner_expression(),
            Expression::Identifier(id) if matches!(id.name.as_str(), "Math" | "Number")
        ),
        _ => false,
    }
}

/// Whether `callee` belongs to an assertion chain: its root is `expect` or `assert`
/// (`expect(x)`, `expect(x).toBe(y)`, `expect.soft(x)`, `assert.equal(a, b)`).
fn is_assertion(callee: &Expression<'_>) -> bool {
    let mut current = callee.get_inner_expression();
    loop {
        current = match current {
            Expression::CallExpression(call) => call.callee.get_inner_expression(),
            Expression::StaticMemberExpression(member) => member.object.get_inner_expression(),
            Expression::ComputedMemberExpression(member) => member.object.get_inner_expression(),
            Expression::Identifier(id) => return matches!(id.name.as_str(), "expect" | "assert"),
            _ => return false,
        };
    }
}

/// The actual and the expected arguments of an assertion chain: `expect(actual)` and the
/// matchers' arguments, or `assert.equal(actual, expected)`.
fn assertion_arguments<'e, 'a>(
    call: &'e CallExpression<'a>,
) -> (Vec<&'e Expression<'a>>, Vec<&'e Expression<'a>>) {
    let mut calls = vec![call];
    let mut current = call.callee.get_inner_expression();
    let root = loop {
        current = match current {
            Expression::CallExpression(inner) => {
                calls.push(inner);
                inner.callee.get_inner_expression()
            }
            Expression::StaticMemberExpression(member) => member.object.get_inner_expression(),
            Expression::ComputedMemberExpression(member) => member.object.get_inner_expression(),
            Expression::Identifier(id) => break id.name.as_str(),
            _ => return (Vec::new(), Vec::new()),
        };
    };
    // `expect(actual)` alone asserts nothing: its matcher's call judges the chain.
    if root == "expect" && calls.len() == 1 {
        return (Vec::new(), Vec::new());
    }
    // The innermost call is the root's: `expect(actual)`, or `assert.equal(actual, …)`.
    let mut actual: Vec<&Expression<'a>> = calls
        .pop()
        .into_iter()
        .flat_map(|c| c.arguments.iter().filter_map(Argument::as_expression))
        .collect();
    let mut expected: Vec<&Expression<'a>> = calls
        .into_iter()
        .flat_map(|c| c.arguments.iter().filter_map(Argument::as_expression))
        .collect();
    if root == "assert" {
        expected.extend(actual.drain(1.min(actual.len())..));
    }
    (actual, expected)
}

/// Whether an ordering assertion holds its actual value below (`Some(true)`) or above
/// (`Some(false)`) the expected one, `.not` read in; `None` for any other assertion.
fn ordering_of(call: &CallExpression<'_>) -> Option<bool> {
    let Expression::StaticMemberExpression(matcher) = call.callee.get_inner_expression() else {
        return None;
    };
    let below = match matcher.property.name.as_str() {
        "toBeLessThan"
        | "toBeLessThanOrEqual"
        | "below"
        | "lessThan"
        | "lt"
        | "most"
        | "lte"
        | "isBelow"
        | "isAtMost" => true,
        "toBeGreaterThan"
        | "toBeGreaterThanOrEqual"
        | "above"
        | "greaterThan"
        | "gt"
        | "least"
        | "gte"
        | "isAbove"
        | "isAtLeast" => false,
        _ => return None,
    };
    let mut negated = false;
    let mut current = matcher.object.get_inner_expression();
    while let Expression::StaticMemberExpression(member) = current {
        negated ^= member.property.name == "not";
        current = member.object.get_inner_expression();
    }
    Some(below != negated)
}

/// Whether `call` reads a calendar field of a date: its year, month, day or hour, or its
/// date as text.
fn is_calendar_field(call: &CallExpression<'_>) -> bool {
    let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
        return false;
    };
    matches!(
        member.property.name.as_str(),
        "getFullYear"
            | "getMonth"
            | "getDate"
            | "getDay"
            | "getHours"
            | "toDateString"
            | "toLocaleDateString"
    )
}

/// The source between `start` and `end`, cut at the first line break and at 60 characters.
pub fn shorten(source: &str, start: u32, end: u32) -> String {
    let text = source.get(start as usize..end as usize).unwrap_or_default();
    let line = text.lines().next().unwrap_or_default();
    let mut short: String = line.chars().take(60).collect();
    if short.len() < text.len() {
        short.push('…');
    }
    short
}

#[cfg(test)]
mod tests {
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;

    use crate::{FlakeKind, FlakeSource};

    /// The `(kind, what, decided)` of every site collected from a snippet, in span order.
    fn sites(source: &str) -> Vec<(FlakeKind, String, bool)> {
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
        super::collect("a.test.ts", source, &parsed.program)
            .sites
            .into_iter()
            .map(|s| (s.kind, s.what, s.decided))
            .collect()
    }

    /// The `what` of every guard collected from a snippet, in span order.
    fn guards(source: &str) -> Vec<String> {
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
        super::collect("a.test.ts", source, &parsed.program)
            .guards
            .into_iter()
            .map(|g| g.what)
            .collect()
    }

    fn wall(what: &str, decided: bool) -> (FlakeKind, String, bool) {
        (FlakeKind::WallClock, what.to_owned(), decided)
    }

    #[test]
    fn a_clock_read_that_never_reaches_an_assertion_is_not_a_site() {
        assert!(
            sites("const id = `run-${Date.now()}`\nsave(id)\nexpect(total()).toBe(3)\n").is_empty()
        );
        assert!(sites(
            "vi.setSystemTime(new Date('2024-01-01'))\nexpect(new Date(0)).toBeTruthy()\n"
        )
        .is_empty());
    }

    #[test]
    fn one_clock_read_compared_to_itself_or_a_constant_is_not_a_site() {
        assert!(sites(
            "const v = new Date().toJSON()\nawait user.type(input, v)\nexpect(input).toHaveValue(v)\n"
        )
        .is_empty());
        assert!(sites("expect(Date.now()).toBeGreaterThan(0)\n").is_empty());
    }

    /// The `(what, calendar)` of every wall-clock site collected from a snippet.
    fn calendar(source: &str) -> Vec<(String, bool)> {
        collect(source)
            .sites
            .into_iter()
            .filter(|s| s.kind == FlakeKind::WallClock)
            .map(|s| (s.what, s.calendar))
            .collect()
    }

    #[test]
    fn a_calendar_field_of_a_read_the_assertion_expects_is_a_calendar_site() {
        let expected = [("new Date()".to_owned(), true)];
        assert_eq!(
            calendar("expect(label()).toBe(`${new Date().getFullYear()}`)\n"),
            expected
        );
        assert_eq!(
            calendar("const now = new Date()\nconst year = now.getFullYear()\nexpect(label()).toBe(String(year))\n"),
            expected
        );
        assert_eq!(
            calendar("assert.equal(label(), new Date().toDateString())\n"),
            expected
        );
    }

    #[test]
    fn a_calendar_field_handed_to_other_code_or_asserted_against_a_constant_is_no_site() {
        // Handed to the code under test, it is compared with itself.
        assert!(calendar(
            "const year = new Date().getFullYear()\nrender(year)\nexpect(label()).toBe(String(year))\n"
        )
        .is_empty());
        // The actual value, against a constant: no second read to disagree with.
        assert!(calendar("expect(new Date().getFullYear()).toBe(2024)\n").is_empty());
        // Handed to the code under test inside the assertion: still one read.
        assert!(calendar(
            "const year = new Date().getFullYear()\nexpect(label(year)).toBe(String(year))\n"
        )
        .is_empty());
        // A read's time of day is no calendar field.
        assert!(calendar("expect(label()).toBe(new Date().toJSON())\n").is_empty());
    }

    #[test]
    fn an_assertion_that_elapsed_time_is_at_least_a_bound_is_not_a_site() {
        // A slower machine only makes these pass.
        for assertion in [
            "expect(Date.now() - start).toBeGreaterThanOrEqual(100)",
            "expect(Date.now() - start).not.toBeLessThan(100)",
            "expect(start + 100).toBeLessThanOrEqual(Date.now())",
            "expect(Date.now() - start >= 100).toBe(true)",
            "assert(Date.now() - start >= 100)",
        ] {
            let source = format!("const start = Date.now()\nawait wait(100)\n{assertion}\n");
            assert_eq!(sites(&source), [], "{assertion}");
        }
    }

    #[test]
    fn an_assertion_that_elapsed_time_is_under_a_budget_is_a_site() {
        for assertion in [
            "expect(Date.now() - start).toBeLessThan(100)",
            "expect(Date.now() - start).not.toBeGreaterThan(100)",
            "expect(start + 100).toBeGreaterThan(Date.now())",
            "expect(Date.now()).toBeLessThan(start + 100)",
            "expect(Date.now() - start < 100).toBe(true)",
            "assert(100 > Date.now() - start)",
        ] {
            let source = format!("const start = Date.now()\nwork()\n{assertion}\n");
            assert_eq!(
                sites(&source),
                [wall("Date.now()", true), wall("Date.now()", true)],
                "{assertion}"
            );
        }
    }

    #[test]
    fn two_calendar_reads_in_one_assertion_are_a_calendar_site() {
        assert_eq!(
            calendar(
                "expect(label(new Date().getFullYear())).toBe(String(new Date().getFullYear()))\n"
            ),
            [("new Date()".to_owned(), true)]
        );
    }

    #[test]
    fn a_comparison_of_two_clock_reads_is_a_site_even_outside_an_assertion() {
        assert_eq!(
            sites("const t0 = performance.now()\nwork()\nif (performance.now() - t0 > 100) throw new Error('slow')\n"),
            [wall("performance.now()", true), wall("performance.now()", true)]
        );
    }

    #[test]
    fn a_method_on_a_clock_value_or_a_math_call_is_followed_plainly() {
        assert_eq!(
            sites("const a = new Date()\nconst b = new Date()\nexpect(Math.round(b.getTime() - a.getTime())).toBeLessThan(5)\n"),
            [wall("new Date()", true), wall("new Date()", true)]
        );
    }

    #[test]
    fn a_value_passed_through_a_call_the_reader_cannot_follow_is_undecided() {
        assert_eq!(
            sites(
                "const start = Date.now()\nexpect(elapsedLabel(Date.now() - start)).toBe('0s')\n"
            ),
            [wall("Date.now()", false), wall("Date.now()", false)]
        );
    }

    #[test]
    fn a_value_is_followed_within_its_own_function_only() {
        assert!(sites(
            "const start = Date.now()\nit('x', () => { expect(start).toBeGreaterThan(0) })\n"
        )
        .is_empty());
    }

    #[test]
    fn fake_timers_and_a_system_time_set_are_guards() {
        assert_eq!(
            guards("vi.useFakeTimers()\njest.setSystemTime(0)\nawait page.clock.install()\nfoo.useFakeTimers()\n"),
            ["vi.useFakeTimers()", "jest.setSystemTime(0)", "page.clock.install()"]
        );
    }

    fn collect(source: &str) -> super::FlakeFacts {
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
        super::collect("a.test.ts", source, &parsed.program)
    }

    /// The `(source, what)` of every unseeded-random site, in span order.
    fn random(source: &str) -> Vec<(FlakeSource, String)> {
        collect(source)
            .sites
            .into_iter()
            .filter(|s| s.kind == FlakeKind::UnseededRandom)
            .map(|s| (s.source, s.what))
            .collect()
    }

    #[test]
    fn math_random_and_crypto_draws_are_sites() {
        assert_eq!(
            random("const n = Math.random()\nconst id = crypto.randomUUID()\nglobalThis.crypto.getRandomValues(buf)\n"),
            [
                (FlakeSource::MathRandom, "Math.random()".to_owned()),
                (FlakeSource::Crypto, "crypto.randomUUID()".to_owned()),
                (FlakeSource::Crypto, "globalThis.crypto.getRandomValues(buf)".to_owned()),
            ]
        );
    }

    #[test]
    fn a_faker_call_is_a_site_and_its_seed_is_not() {
        assert_eq!(
            random("import { faker as f } from '@faker-js/faker'\nf.seed(1)\nconst when = f.date.past().toISOString()\n"),
            [(FlakeSource::Faker, "f.date.past()".to_owned())]
        );
        // A `faker` the file did not import from the package is someone else's.
        assert!(random("const faker = makeStub()\nfaker.name()\n").is_empty());
    }

    #[test]
    fn a_seed_a_spy_on_the_source_or_a_module_mock_is_a_random_guard() {
        let guards: Vec<(Vec<FlakeSource>, String)> = collect(
            "import { faker } from '@faker-js/faker'\n\
             faker.seed(42)\n\
             vi.spyOn(Math, 'random').mockReturnValue(0.5)\n\
             jest.spyOn(global.crypto, 'randomUUID')\n\
             vi.stubGlobal('crypto', fake)\n\
             vi.mock('node:crypto')\n\
             vi.mock('@faker-js/faker', () => ({}))\n\
             Math.random = () => 0.1\n\
             vi.spyOn(api, 'fetch')\n",
        )
        .guards
        .into_iter()
        .map(|g| (g.covers, g.what))
        .collect();
        assert_eq!(
            guards,
            [
                (vec![FlakeSource::Faker], "faker.seed(42)".to_owned()),
                (
                    vec![FlakeSource::MathRandom],
                    "vi.spyOn(Math, 'random')".to_owned()
                ),
                (
                    vec![FlakeSource::Crypto],
                    "jest.spyOn(global.crypto, 'randomUUID')".to_owned()
                ),
                (
                    vec![FlakeSource::Crypto],
                    "vi.stubGlobal('crypto', fake)".to_owned()
                ),
                (
                    vec![FlakeSource::Crypto],
                    "vi.mock('node:crypto')".to_owned()
                ),
                (
                    vec![FlakeSource::Faker],
                    "vi.mock('@faker-js/faker', () => ({}))".to_owned()
                ),
                (
                    vec![FlakeSource::MathRandom],
                    "Math.random = () => 0.1".to_owned()
                ),
            ]
        );
    }

    /// The `what` of every site of `kind`, in span order.
    fn of_kind(source: &str, kind: FlakeKind) -> Vec<String> {
        collect(source)
            .sites
            .into_iter()
            .filter(|s| s.kind == kind)
            .map(|s| s.what)
            .collect()
    }

    #[test]
    fn an_awaited_literal_wait_is_a_fixed_timeout_site() {
        let source = "import { setTimeout as wait } from 'node:timers/promises'\n\
                      const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms))\n\
                      async function tick() { return new Promise((r) => setTimeout(r, 10)) }\n\
                      await new Promise((resolve) => setTimeout(resolve, 100))\n\
                      await sleep(50)\n\
                      await tick()\n\
                      await wait(20)\n\
                      await page.waitForTimeout(500)\n";
        assert_eq!(
            of_kind(source, FlakeKind::FixedTimeout),
            [
                "new Promise((resolve) => setTimeout(resolve, 100))",
                "sleep(50)",
                "tick()",
                "wait(20)",
                "page.waitForTimeout(500)",
            ]
        );
    }

    #[test]
    fn a_computed_delay_or_an_unawaited_timer_is_not_a_fixed_timeout_site() {
        let source = "const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms))\n\
                      await new Promise((resolve) => setTimeout(resolve, delay))\n\
                      await sleep(ms)\n\
                      setTimeout(done, 100)\n\
                      await page.waitForTimeout(timeout)\n";
        assert!(of_kind(source, FlakeKind::FixedTimeout).is_empty());
    }

    #[test]
    fn a_sleep_in_a_loop_polling_awaited_state_is_no_site_and_one_in_a_counted_loop_is() {
        let source = "const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms))\n\
                      while (!done) await sleep(10)\n\
                      for (let i = 0; i < 40 && !done; i++) await sleep(20)\n\
                      let n = 0\n\
                      while (n < 3) { await sleep(30); n += 1 }\n\
                      for (let i = 0; i < 40 && !done; i++) { setTimeout(async () => { await sleep(40) }) }\n";
        assert_eq!(
            of_kind(source, FlakeKind::FixedTimeout),
            ["sleep(30)", "sleep(40)"]
        );
    }

    #[test]
    fn a_deadline_in_a_loop_polling_awaited_state_is_no_site_and_a_bare_busy_wait_is() {
        assert_eq!(
            sites(
                "const deadline = Date.now() + 1000\n\
                   while (Date.now() < deadline && !ready()) spin()\n\
                   const until = performance.now() + 50\n\
                   while (performance.now() < until) {}\n"
            ),
            [
                wall("performance.now()", true),
                wall("performance.now()", true)
            ]
        );
    }

    #[test]
    fn a_networkidle_wait_is_a_site_and_a_web_first_wait_is_not() {
        let source = "await page.waitForLoadState('networkidle')\n\
                      await page.goto('/', { waitUntil: 'networkidle' })\n\
                      await page.waitForLoadState('domcontentloaded')\n\
                      await expect(page.getByRole('heading')).toBeVisible()\n";
        assert_eq!(
            of_kind(source, FlakeKind::Networkidle),
            [
                "page.waitForLoadState('networkidle')",
                "waitUntil: 'networkidle'",
            ]
        );
    }

    #[test]
    fn a_dropped_promise_is_a_site_unless_a_link_handles_its_rejection() {
        let source = "async function save() {}\n\
                      test('t', () => {\n\
                      save()\n\
                      window.fetch('/a')\n\
                      load().then(use).finally(done)\n\
                      load().then(use, fail)\n\
                      load().then(use).catch(fail)\n\
                      void save()\n\
                      load()\n\
                      })\n";
        assert_eq!(
            of_kind(source, FlakeKind::UnawaitedPromise),
            [
                "save()",
                "window.fetch('/a')",
                "load().then(use).finally(done)"
            ]
        );
    }

    #[test]
    fn a_real_io_site_is_decided_by_what_its_path_is_built_from() {
        let source = "import fs from 'node:fs'\n\
                      import { readFile } from 'fs/promises'\n\
                      import os from 'node:os'\n\
                      import { join } from 'node:path'\n\
                      fs.promises.writeFile(join(process.cwd(), 'out'), '')\n\
                      readFile(`${os.tmpdir()}/x`)\n\
                      const f = (p) => fs.readFileSync(p)\n";
        let sites: Vec<(String, bool)> = collect(source)
            .sites
            .into_iter()
            .filter(|s| s.kind == FlakeKind::RealIo)
            .map(|s| (s.what, s.decided))
            .collect();
        assert_eq!(
            sites,
            [
                (
                    "fs.promises.writeFile(join(process.cwd(), 'out'), '')".to_owned(),
                    true
                ),
                ("fs.readFileSync(p)".to_owned(), false),
            ]
        );
    }

    #[test]
    fn the_body_of_a_concise_arrow_is_returned_not_dropped() {
        let source = "async function save() {}\n\
                      beforeAll(() => save())\n\
                      const chain = { then: (ok) => Promise.resolve(1).then(ok) }\n\
                      beforeEach(() => { save() })\n";
        assert_eq!(of_kind(source, FlakeKind::UnawaitedPromise), ["save()"]);
    }

    #[test]
    fn a_promise_dropped_at_module_scope_is_not_in_any_test_body_or_hook() {
        let source = "async function main() {}\n\
                      main()\n\
                      export function setup() { main() }\n";
        assert_eq!(of_kind(source, FlakeKind::UnawaitedPromise), ["main()"]);
        let site = collect(source)
            .sites
            .into_iter()
            .find(|s| s.kind == FlakeKind::UnawaitedPromise)
            .unwrap();
        assert_eq!(site.span.line, 3);
    }

    #[test]
    fn a_two_path_call_is_private_to_the_run_only_when_both_paths_are() {
        let source = "import { renameSync, copyFileSync, mkdtempSync } from 'node:fs'\n\
                      import { tmpdir } from 'node:os'\n\
                      import { join } from 'node:path'\n\
                      const dir = mkdtempSync(join(tmpdir(), 'x-'))\n\
                      renameSync(join(dir, 'a'), 'out.txt')\n\
                      copyFileSync(join(dir, 'a'), join(dir, 'b'))\n";
        assert_eq!(
            of_kind(source, FlakeKind::RealIo),
            ["renameSync(join(dir, 'a'), 'out.txt')"]
        );
    }

    #[test]
    fn a_site_records_the_hook_it_is_installed_in_including_one_passed_by_name() {
        let source = "import { afterEach, beforeAll, test, vi } from 'vitest'\n\
                      function reapply() { vi.stubGlobal('Image', class {}) }\n\
                      function helper() { vi.stubEnv('A', 'b') }\n\
                      reapply()\n\
                      afterEach(reapply)\n\
                      beforeAll(() => { vi.spyOn(console, 'log') })\n\
                      test('t', () => { vi.useFakeTimers() })\n";
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let hooks: Vec<_> = super::collect("a.test.ts", source, &parsed.program)
            .sites
            .into_iter()
            .map(|s| s.hook)
            .collect();
        assert_eq!(
            hooks,
            [
                Some(crate::Hook::AfterEach),
                None,
                Some(crate::Hook::BeforeAll),
                None
            ]
        );
    }
}
