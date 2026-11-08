//! The model's tables. Every row type is declared in full here so later slices fill
//! fields rather than reshape the schema: anything a later milestone computes is an
//! `Option` or a `Vec`, and is empty until then.

use serde::{Deserialize, Serialize};

use crate::Span;

/// An inline runner project's file selection, independent of its reporting options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionProject {
    /// Explicit collection root, relative to the analysis root.
    pub root: Option<String>,
    /// Jest's roots, interpreted relative to the collection root.
    pub roots: Vec<String>,
    /// The project's include globs, relative to the parent config.
    pub include: Vec<String>,
    /// The project's exclude globs (Jest ignore regexes).
    pub exclude: Vec<String>,
    /// Jest's alternative test-selection regexes.
    pub test_regex: Vec<String>,
}

/// A source file selected by a runner's static collection settings.
///
/// Independent of case recognition: helpers can generate tests and a parser can reject a body
/// the project's own transform accepts. This never claims how many cases execute.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TestFile {
    /// Repo-relative, slash-separated path.
    pub file: String,
    /// Runner family that collects the file.
    pub framework: String,
}

// ---------------------------------------------------------------- Module

/// Which dialect a file was read as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    /// `.ts`, `.mts`, `.cts`.
    #[serde(rename = "typescript")]
    TypeScript,
    /// `.tsx`.
    Tsx,
    /// `.js`, `.mjs`, `.cjs`.
    #[serde(rename = "javascript")]
    JavaScript,
    /// `.jsx`.
    Jsx,
    /// `.json`, `.jsonc` — configs read as data.
    Json,
    /// Read, but no dialect matched.
    Unknown,
}

/// What became of a file when the model tried to read it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParseStatus {
    /// Parsed cleanly; the row's facts are complete.
    Parsed,
    /// Parsed with recovered errors; the row's facts may be partial.
    Recovered,
    /// Could not be parsed at all; see the matching diagnostic.
    Failed,
}

/// How a specifier entered a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportKind {
    /// `import x from "y"`.
    Static,
    /// `import("y")`.
    Dynamic,
    /// `require("y")`.
    Require,
    /// `import type { X } from "y"`.
    Type,
    /// `export { x } from "y"`.
    ReExport,
}

/// One specifier a module depends on.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Import {
    /// The specifier exactly as written, for example `@acme/core-kit`.
    pub specifier: String,
    /// Repo-relative, slash-separated path the specifier resolved to, if it did.
    pub resolved: Option<String>,
    /// Why `resolved` is empty, when it is: `external` (an installed dependency under a
    /// `node_modules`, not a module the model tables), `missing` (a relative specifier
    /// nothing satisfies — also an `unresolved_import` diagnostic), `unknown` (a relative
    /// specifier the resolver failed on for another reason, such as a tsconfig that cannot
    /// be loaded — a `tsconfig_unloadable` diagnostic, once per config), `unknown_package` (a
    /// bare specifier nothing satisfies: uninstalled or misspelled, undecidable without
    /// the install), `builtin` (a Node builtin such as `node:fs` or `path`), `computed` (an
    /// `import(x)` / `require(x)` whose specifier is an expression, recorded as written), or
    /// `outside_root` (resolved to a file above the project root), or `build_output` (a
    /// relative specifier into the package's build directory, which only a build fills).
    /// `None` for a resolved import.
    pub reason: Option<String>,
    /// How the specifier was written.
    pub kind: ImportKind,
    /// Location of the specifier.
    pub span: Span,
}

/// The shape of an export binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportKind {
    /// `export const x`, `export function x`.
    Named,
    /// `export default …`.
    Default,
    /// `export * from "y"`.
    Namespace,
    /// `export type X`.
    Type,
}

/// One name a module makes available.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Export {
    /// Exported name; `default` for a default export.
    pub name: String,
    /// The shape of the binding.
    pub kind: ExportKind,
    /// For a re-export (`export { x } from "y"`, `export * from "y"`), the specifier the
    /// name comes from, as written. `None` for a name the module binds itself.
    pub origin: Option<String>,
    /// Repo-relative, slash-separated path `origin` resolved to, if it did — the same
    /// answer the matching `re_export` import row carries. `None` for a local export or an
    /// origin that did not resolve (see that import row's `reason`).
    pub resolved: Option<String>,
    /// Location of the export.
    pub span: Span,
}

/// A module-level string constant, folded where the value could be decided.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Constant {
    /// Binding name.
    pub name: String,
    /// The folded value, or `None` when folding gave up (see `opaque`).
    pub value: Option<String>,
    /// True when the value could not be decided statically, so rules say
    /// "could not decide" instead of guessing.
    pub opaque: bool,
    /// Why folding gave up, when `opaque`; `None` for a folded value. Recorded so a
    /// later rule can report *why* a constant is undecidable rather than guess one.
    pub reason: Option<String>,
    /// The constant rendered with every `process.env.NAME` read left as a `${NAME}` hole,
    /// set only when the constant is opaque *solely* because of such reads (no other opaque
    /// part). A rule that holds assumed values (`handler-drift`'s `env` config) completes the
    /// holes; the model itself never reads the environment, so `value` stays `None` and
    /// `opaque` stays `true`. `None` for a folded constant or one opaque for any other reason.
    pub env_template: Option<String>,
    /// Location of the binding.
    pub span: Span,
}

/// One source file, with the facts extracted from it. The AST is not kept.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Module {
    /// Repo-relative, slash-separated path. The table's sort key.
    pub path: String,
    /// The dialect the file was read as.
    pub language: Language,
    /// What became of the file when it was parsed.
    pub parse_status: ParseStatus,
    /// Specifiers this module depends on, sorted by span.
    pub imports: Vec<Import>,
    /// Names this module makes available, sorted by span.
    pub exports: Vec<Export>,
    /// Folded module-level string constants, sorted by span.
    pub constants: Vec<Constant>,
    /// The module mocks the file declares (`vi.mock("x")`, `jest.mock("x")`), sorted by
    /// span. These are file-scoped by design — the runner hoists them above the tests — so
    /// they live on the module, not on a test case, which is where a `data-layer-mock` rule
    /// and `init`'s data-layer detection read them.
    pub mocks: Vec<Mock>,
    /// Every identifier *reference* (a read of a name, not a binding declaration) the file
    /// makes, sorted by span. `unused-mock`'s unread-spy kind reads this to decide whether a
    /// spy binding is read anywhere else without a second parse. Never serialized — it is a
    /// working table the rule reads in-process, not part of the `list` document.
    #[serde(skip)]
    pub references: Vec<IdentRef>,
    /// Every `vi.fn()` / `jest.fn()` / `vi.spyOn(…)` / `jest.spyOn(…)` result bound to a plain
    /// name in the file (`const spy = vi.fn()`), sorted by span. `unused-mock`'s unread-spy
    /// kind reports one whose `binding` no [`IdentRef`] in [`Module::references`] reads. Never
    /// serialized, for the same reason as `references`.
    #[serde(skip)]
    pub spies: Vec<Spy>,
    /// Every module the file loads through the runner's registry rather than an import —
    /// `jest.requireActual("x")`, `jest.requireMock("x")`, `vi.importActual("x")`,
    /// `vi.importMock("x")` with a string-literal specifier — sorted by span. `unused-mock`
    /// walks them as load edges of the import closure. Never serialized, for the same reason
    /// as `references`.
    #[serde(skip)]
    pub loads: Vec<Load>,
    /// The module's cyclomatic complexity as PRD 0005 defines it for `untested-hotspot`: the
    /// sum over its functions (declarations, expressions, arrows and methods with a body) of
    /// 1 + decision points (`if`, `?:`, `&&`, `||`, `??`, `case`, `for`, `for…in/of`,
    /// `while`, `do`, `catch`), with the module top level counted as one function. 0 for a
    /// file that did not parse, where nothing was counted. Never serialized.
    #[serde(skip)]
    pub complexity: u32,
    /// Whether the file says it is generated: its first 20 lines carry `@generated`,
    /// `Do not edit`, `Generated from` or `auto-generated`. A generated module is not a
    /// place to add tests or hand edits. Never serialized.
    #[serde(skip)]
    pub generated: bool,
}

/// One identifier reference — a *read* of a name, not the binding that introduces it.
///
/// An oxc `IdentifierReference`, as opposed to a `BindingIdentifier`. `unused-mock` counts
/// these to decide whether a spy binding is ever read after it is created.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct IdentRef {
    /// The name read.
    pub name: String,
    /// Location of the reference.
    pub span: Span,
}

/// One `vi.fn()` / `jest.fn()` / `vi.spyOn(…)` / `jest.spyOn(…)` result bound to a plain
/// name — the raw material for `unused-mock`'s unread-spy kind.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Spy {
    /// The name the spy is bound to (`const spy = vi.fn()` → `spy`).
    pub binding: String,
    /// Location of the binding's initialiser call.
    pub span: Span,
}

/// One module loaded through the test runner's registry at run time (`jest.requireActual`,
/// `jest.requireMock`, `vi.importActual`, `vi.importMock`), resolved the way an import from
/// the same file is.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Load {
    /// The specifier exactly as written.
    pub specifier: String,
    /// Repo-relative, slash-separated path the specifier resolved to, if it did.
    pub resolved: Option<String>,
    /// Why `resolved` is empty, in the vocabulary of [`Import::reason`]. `None` for a
    /// resolved load, and before resolution.
    pub reason: Option<String>,
    /// Location of the whole call.
    pub span: Span,
}

// ---------------------------------------------------------------- TestCase

/// What kind of node a recogniser matched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestKind {
    /// `test(…)`, `it(…)`.
    Test,
    /// `describe(…)`.
    Suite,
    /// `beforeEach(…)` and friends.
    Hook,
    /// `bench(…)`.
    Benchmark,
    /// A Playwright `test.step(…)` inside a test body — a sub-unit of a test, not a
    /// test in its own right, so it is not counted among the suite's tests.
    Step,
}

/// The chain modifier that changes whether, or how, a case runs.
///
/// `only` and `fails` are not skips in the plain sense — the case still runs — but they
/// change which cases run or what "pass" means, so a later rule reasons about them the
/// same way it reasons about a `skip`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SkipKind {
    /// `.skip` — never runs.
    Skip,
    /// `.todo` — a placeholder with no body yet.
    Todo,
    /// `.fixme` — Playwright's "known broken, do not run".
    Fixme,
    /// `.only` — the inverse of a skip: everything without `.only` is skipped.
    Only,
    /// `.fails` — runs, but the run is expected to fail.
    Fails,
    /// `.slow` — Playwright's "this test is slow", tripling its timeout rather than
    /// stopping it; carries a condition when it is a conditional annotation.
    Slow,
    /// `.skipIf(condition)` — skipped when the condition holds.
    SkipIf,
    /// `.runIf(condition)` — the inverse: run only when the condition holds.
    RunIf,
}

/// A modifier on a case that changes whether, or how, it runs: a `skip`, `todo`,
/// `only`, a conditional, and their kin.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Skip {
    /// Which modifier it is.
    pub kind: SkipKind,
    /// Source text of the condition, for `.skipIf` / `.runIf` and for a Playwright
    /// annotation's first argument (`test.skip(cond, why)`); `None` otherwise.
    pub condition: Option<String>,
    /// The reason the skip gives, when the call carries one as a string: the `why` of a
    /// Playwright `test.skip(cond, why)` / `test.fixme(cond, why)`. A rule that looks for an
    /// issue reference reads it beside the title. `None` when the call gives no reason or
    /// gives one the reader cannot lift (a computed string).
    pub reason: Option<String>,
    /// The comment text written beside a definition-level skip: up to three comment lines
    /// (`//`, `/*` or ` *`) directly above the definition, in source order, stopping at the
    /// first blank or code line, then a trailing `//` comment on the title's line. A rule that looks for an
    /// issue reference reads them beside the title and the reason. Empty for an in-body
    /// annotation and when no comment sits there.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<String>,
    /// Location of the modifier keyword.
    pub span: Span,
}

/// A `.each` / `.for` parameterised table — or the iterable of a loop that wraps the
/// definition — kept as one row with its size rather than expanded into a row per data
/// case.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Each {
    /// Number of data cases the table declares (its rows), when it could be counted.
    pub cases: Option<u32>,
    /// Number of parameters each case supplies (its columns), when it could be counted.
    pub params: Option<u32>,
    /// The binding the table was read from when it is not a literal in the call
    /// (`it.each(ROWS)`, `for (const r of pack.phrases)`), as a dotted path. The model
    /// follows it through local constants, imports and re-exports to count `cases`; when
    /// it leads nowhere countable the source stays and `cases` stays `None`.
    pub source: Option<String>,
}

/// One `expect(…)` chain, or a framework equivalent.
///
/// Four booleans, each a fact a different reader turns on; `subject_is_query` and
/// `subject_is_call` overlap (a query is a call) but are serialized apart, so the
/// `struct_excessive_bools` lint is allowed here.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Assertion {
    /// Matcher name, for example `toBe`, when it could be read.
    pub matcher: Option<String>,
    /// Source text of the matcher's first argument: the `1` of `expect(x).toBe(1)`, the
    /// `0` of `toHaveBeenCalledTimes(0)`, the expected value of an `assert.equal(x, y)`.
    /// `None` when the matcher takes none (`toBeNull()`, `assert(x)`). A rule that turns on
    /// one literal (`toHaveLength(0)`, `toHaveBeenCalledTimes(0)`) reads it here.
    pub argument: Option<String>,
    /// Source text of the subject, for example `result.status`.
    pub subject: Option<String>,
    /// True when the subject is a non-throwing Testing Library query: a `queryBy*` /
    /// `queryAllBy*` call, after any `await`, parentheses or cast. `queryBy*` returns
    /// `null` (and `queryAllBy*` an empty array) for a missing element, so `toBeNull()`,
    /// `toBeFalsy()`, `not.toBeTruthy()` or `toHaveLength(0)` on such a subject is the
    /// Testing Library absence idiom: it holds against an empty body as readily as against
    /// the render it means to test. Read by the value-less table and by `absence-anchor`.
    pub subject_is_query: bool,
    /// True when the subject is an element read off the render, nullish when the render
    /// produced nothing: a call, after any `await`, parentheses or cast (a Testing Library
    /// query, `container.querySelector(…)`, a helper such as `gridcell(day)`), or a name a
    /// `const`/`let` in the same callback binds to such a call (`const link =
    /// container.querySelector('a')`). Optional call chains and nullable DOM child reads
    /// (`container.firstChild`, `firstElementChild`, `lastChild`, `lastElementChild`) count
    /// too, directly or through a local. The field retains its historical name for
    /// compatibility. Every jest-dom matcher but `toBeInTheDocument` checks
    /// its subject is an element before any negation and throws on `null`, so a negated one
    /// on such a subject fails against an empty body. Read by `absence-anchor`. Left out of
    /// the serialized row when false.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub subject_is_call: bool,
    /// True when the chain goes through `.not`.
    pub negated: bool,
    /// False when the assertion checks no produced value: `toBeTruthy`, `toBeDefined`,
    /// `not.toBeNull`, `not.toBeUndefined`, `not.toBeFalsy`, a bare `assert(x)` or
    /// `assert.ok(x)`, and `toBeNull` / `toBeUndefined` / `toBeFalsy` on a `queryBy*` /
    /// `queryAllBy*` subject (the Testing Library absence idiom). Every other matcher,
    /// including argument-less ones such as `toThrow()` and `toBeInTheDocument()`, and any
    /// matcher the taxonomy does not know, carries a value. The table is
    /// [`ValueLess`](crate::ValueLess); the `weak-assertion` rule counts the `false` rows.
    pub carries_value: bool,
    /// Location of the chain.
    pub span: Span,
}

/// One Testing Library query that *throws* when it finds nothing.
///
/// Its mere presence in a test body anchors the render: `getBy*`, `getAllBy*`, `findBy*`,
/// `findAllBy*`, whether called on `screen`, a bare import, or through `within(...)`.
///
/// The `queryBy*` / `queryAllBy*` family is deliberately excluded: those return `null` (or an
/// empty array) rather than throwing, so a test built only on them is *not* anchored — the
/// distinction the `absence-anchor` rule turns on. Recorded by the Vitest/Jest and Playwright
/// recognisers, which descend the whole callback subtree, so a query inside a `waitFor(…)`
/// callback counts.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ThrowingQuery {
    /// The query method as written, for example `getByRole` or `findAllByText`.
    pub name: String,
    /// True when the query is reached through `within(...)` — a scoped query on a subtree,
    /// which throws the same way its `screen`-level twin does.
    pub within: bool,
    /// Location of the query call.
    pub span: Span,
}

impl ThrowingQuery {
    /// Whether a method name is a throwing Testing Library query: `getBy*`, `getAllBy*`,
    /// `findBy*` or `findAllBy*`.
    pub fn is_named(name: &str) -> bool {
        ["getBy", "getAllBy", "findBy", "findAllBy"]
            .iter()
            .any(|prefix| name.starts_with(prefix))
    }

    /// Whether a method name is one of Testing Library's own throwing queries: a
    /// [`is_named`](Self::is_named) prefix followed by one of its query kinds (`getByRole`,
    /// `findAllByTestId`), never a utility that only starts the same way (`getByPath`).
    pub fn is_testing_library(name: &str) -> bool {
        const KINDS: &[&str] = &[
            "AltText",
            "DisplayValue",
            "LabelText",
            "PlaceholderText",
            "Role",
            "TestId",
            "Text",
            "Title",
        ];
        ["getBy", "getAllBy", "findBy", "findAllBy"]
            .iter()
            .filter_map(|prefix| name.strip_prefix(prefix))
            .any(|kind| KINDS.contains(&kind))
    }
}

/// What a mock replaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MockKind {
    /// `vi.mock("…")`, `jest.mock("…")`.
    Module,
    /// `vi.fn()`, `vi.spyOn(…)`.
    Function,
    /// `vi.useFakeTimers()`.
    Timer,
    /// An MSW or nock handler installed inside a test.
    Network,
    /// Recognised as a mock, kind undecided.
    Unknown,
}

/// One mock in effect for a test case.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Mock {
    /// What the mock replaces: a specifier, a member expression, or nothing.
    pub target: Option<String>,
    /// For a module mock, the repo-relative, slash-separated path `target` resolved to
    /// through the same resolver imports go through — so a later rule can compare the
    /// factory against the real module. `None` when the target is not a module specifier,
    /// is an external package, or did not resolve (a relative target that does not is an
    /// `unresolved_mock` diagnostic).
    pub resolved: Option<String>,
    /// Why a module mock's `resolved` is empty, in the vocabulary of [`Import::reason`]:
    /// `external` and `builtin` name a package outside the project; `unknown_package`,
    /// `missing`, `unknown` and `outside_root` say the target could not be told. `None` for a
    /// resolved target, a non-module mock, and before resolution.
    pub reason: Option<String>,
    /// What kind of thing is replaced.
    pub kind: MockKind,
    /// True when a module mock supplies a factory (`vi.mock("x", () => …)`), so a
    /// later rule can tell a factory mock from a bare auto-mock. Always false for the
    /// non-module kinds.
    pub factory: bool,
    /// Location of the call.
    pub span: Span,
}

/// One test, suite, hook or benchmark, as the recognisers saw it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TestCase {
    /// Repo-relative, slash-separated path of the file it lives in. First sort key.
    pub file: String,
    /// Location of the whole call. Second sort key.
    pub span: Span,
    /// Recogniser that claimed it, for example `vitest` or `playwright`.
    pub framework: Option<String>,
    /// The title as written; `None` when it is not a literal.
    pub title: Option<String>,
    /// What kind of node it is.
    pub kind: TestKind,
    /// Enclosing `describe` titles, outermost first.
    pub suite_path: Vec<String>,
    /// Assertions inside the case's own body, sorted by span.
    pub assertions: Vec<Assertion>,
    /// Throwing Testing Library queries (`getBy*`, `findBy*`, `within(...).getBy*`) in the
    /// case's own body, sorted by span: each anchors the render, so a test that carries one
    /// is not a candidate for `absence-anchor`. Empty on a suite row. Collected across the
    /// whole callback subtree, so a query inside a `waitFor(…)` callback is counted.
    pub throwing_queries: Vec<ThrowingQuery>,
    /// Mocks in effect for the case, sorted by span: those its own body installs, the
    /// file's module mocks (`vi.mock` / `jest.mock`, hoisted wherever written), and the
    /// spies, fakes, timers and `server.use` calls installed at module scope or in an
    /// enclosing `describe`. Empty on a suite row; a hook row carries its own body's only.
    pub mocks: Vec<Mock>,
    /// Set when a modifier changes whether or how the case runs.
    pub skip: Option<Skip>,
    /// Set when the case is a `.each` / `.for` parameterised table.
    pub each: Option<Each>,
    /// The `for…of` / `forEach` loops the definition sits inside, outermost first: the
    /// runner defines it once per element of each. Empty for a definition at the top
    /// level of its suite.
    pub loops: Vec<Each>,
    /// Chain modifiers that do not change whether the case runs — `concurrent`,
    /// `sequential`, `fixtures` (a `test.extend` binding) — sorted and deduplicated.
    pub modifiers: Vec<String>,
    /// Tags read from the title or the framework's tag API, sorted.
    pub tags: Vec<String>,
    /// Hash of the body with identifiers normalised and the title excluded, so two
    /// tests with the same behaviour and different names hash equal.
    pub body_hash: Option<String>,
    /// True when the case's callback body holds no statements (`it('…', () => {})`): a
    /// placeholder that runs nothing. Always false on a suite or hook row.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub empty: bool,
    /// What the body depends on outside its own text, sorted: what each free name it reads
    /// is bound to in its file. Two equal bodies are one test only when this is the same.
    /// Empty on a suite or hook row, and for a body that reads only globals.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context: Vec<ContextRef>,
}

/// One thing a test body depends on outside its own text: see [`TestCase::context`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ContextRef {
    /// A free name bound in the test's own file, outside its body: at module scope or in an
    /// enclosing suite. `span` is the binding's, so two tests reading the same declaration
    /// carry the same span and two reading two declarations of one name do not.
    Binding {
        /// The name as written.
        name: String,
        /// Location of the binding identifier.
        span: Span,
    },
    /// A free name bound by an `import` of the file: the module's [`Import::resolved`] tells
    /// whether two files' imports name one module.
    Import {
        /// The local name as written.
        name: String,
        /// The specifier as written, to look up in the file's [`Module::imports`].
        specifier: String,
        /// The export it binds: its name, `default`, or `*` for a namespace import.
        export: String,
    },
    /// A relative module specifier the body loads itself (`import('.')`,
    /// `require('./x')`): the text is in the hash, the module it names is not.
    Specifier {
        /// The specifier as written, to look up in the file's [`Module::imports`].
        specifier: String,
    },
    /// A `beforeEach` / `beforeAll` hook of the file's module scope or an enclosing suite:
    /// it runs before the case, wherever it is written.
    Hook {
        /// Location of the hook call.
        span: Span,
    },
}

// ---------------------------------------------------------------- SagaWalk

/// Which step-walk idiom a saga assertion site belongs to.
///
/// Both idioms test a redux-saga generator by stepping it and asserting the yielded effect
/// in order — the brittle style the redux-saga docs steer away from towards
/// `expectSaga(...).run()`. They differ only in surface: one drives the raw generator, the
/// other a fluent test harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SagaStepKind {
    /// `expect(gen.next().value).toEqual(call(api, 1))`: a `.next()` on a generator asserted
    /// against a call to an effect creator imported from `redux-saga/effects`.
    GenNext,
    /// One `.next(…)` step in a `testSaga(saga).next().put(a).next()…` chain from
    /// `redux-saga-test-plan`.
    TestSaga,
}

/// One place a saga is tested by stepping its generator: a `gen.next().value` assertion
/// against an effect creator, or one `.next()` step of a `testSaga` chain.
///
/// Recorded by the saga recogniser, which claims `redux-saga/effects` and
/// `redux-saga-test-plan` imports; the `generator-walk` rule counts these sites and reports
/// them so yield-order tests are a number a maintainer can bring down.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SagaWalk {
    /// Repo-relative, slash-separated path of the file it lives in. First sort key.
    pub file: String,
    /// Location of the site — the `expect(…)` assertion, or the `.next(…)` step. Second
    /// sort key.
    pub span: Span,
    /// Which idiom the site belongs to.
    pub kind: SagaStepKind,
    /// The effect the site pins: the creator name for a `gen.next().value` assertion
    /// (`call`, `put`, `take`, …, as imported from `redux-saga/effects`), or the fluent
    /// effect method for a `testSaga` step (`put`, `take`, `isDone`, …). `None` when the
    /// step names no effect (a bare `.next(value)` advancing the generator).
    pub effect: Option<String>,
}

// ---------------------------------------------------------------- FlakeSite

/// Which flake mechanism a [`FlakeSite`] is: the `kind` evidence of a `flake-candidate`
/// finding, one of the mechanisms `docs/categories.md` listed before the oracle existed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FlakeKind {
    /// A wall-clock read (`Date.now()`, `new Date()`, `performance.now()`,
    /// `process.hrtime*`) whose value reaches an assertion or a comparison against a second
    /// clock read.
    WallClock,
    /// `Math.random`, `crypto.randomUUID` / `getRandomValues`, or a `faker` call.
    UnseededRandom,
    /// An awaited `setTimeout`, a sleep helper that resolves to one, or
    /// `page.waitForTimeout`, with a literal delay.
    FixedTimeout,
    /// `waitForLoadState('networkidle')` or `waitUntil: 'networkidle'`.
    Networkidle,
    /// A dynamic `import()`, a `fetch`, a `.then` chain or a call to a function known to be
    /// async, as an expression statement that is not awaited, returned or chained to a
    /// rejection handler.
    UnawaitedPromise,
    /// A read or write through `fs` whose path is not built from `os.tmpdir()` or `mkdtemp`.
    RealIo,
    /// `vi.spyOn`/`jest.spyOn`, `vi.stubGlobal`/`vi.stubEnv` or `useFakeTimers` with no
    /// matching restore in scope and none in the runner config that governs the file.
    MissingRestore,
    /// A module-scope `let`, or a mutable object or array, written inside a test body with
    /// no reset in a `beforeEach`/`afterEach` in scope.
    SharedState,
    /// A test that reads a module-scope binding which only another test body assigns.
    OrderDependence,
}

impl FlakeKind {
    /// The kebab-case name a finding's `kind` evidence spells this kind with.
    pub const fn name(self) -> &'static str {
        match self {
            Self::WallClock => "wall-clock",
            Self::UnseededRandom => "unseeded-random",
            Self::FixedTimeout => "fixed-timeout",
            Self::Networkidle => "networkidle",
            Self::UnawaitedPromise => "unawaited-promise",
            Self::RealIo => "real-io",
            Self::MissingRestore => "missing-restore",
            Self::SharedState => "shared-state",
            Self::OrderDependence => "order-dependence",
        }
    }
}

/// What a flake mechanism draws on, so a [`FlakeGuard`] can say which sites it neutralises:
/// fake timers cover the clock and the timers, a `faker.seed` covers faker and nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlakeSource {
    /// The wall clock: `Date`, `performance`, `process.hrtime`.
    Clock,
    /// Real timers: `setTimeout` and the waits built on it.
    Timer,
    /// `Math.random`.
    MathRandom,
    /// Web or Node `crypto` randomness.
    Crypto,
    /// A `@faker-js/faker` generator.
    Faker,
    /// A browser page's network activity; nothing guards it.
    Network,
    /// A promise nobody waits for; nothing guards it.
    Promise,
    /// The real filesystem, through `fs`, `fs/promises` or `node:fs*`.
    Fs,
    /// A `spyOn` left installed: undone by `mockRestore`, `restoreAllMocks`, `restoreMocks`.
    Spy,
    /// A `stubGlobal` left installed: undone by `unstubAllGlobals`, `unstubGlobals`.
    GlobalStub,
    /// A `stubEnv` left installed: undone by `unstubAllEnvs`, `unstubEnvs`.
    EnvStub,
    /// Fake timers left installed: undone by `useRealTimers`.
    FakeTimers,
    /// Module-scope state; a reset is a write to the same binding, which the rule matches by
    /// name, so no guard covers it.
    ModuleState,
}

/// Which runner hook a callback is: `beforeAll`, `beforeEach`, `afterEach` or `afterAll`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Hook {
    /// `beforeAll`: runs once, before the first test of its scope.
    BeforeAll,
    /// `beforeEach`: runs before every test of its scope.
    BeforeEach,
    /// `afterEach`: runs after every test of its scope.
    AfterEach,
    /// `afterAll`: runs once, after the last test of its scope.
    AfterAll,
}

impl Hook {
    /// Whether the hook runs before the tests of its scope rather than after them.
    #[must_use]
    pub const fn is_before(self) -> bool {
        matches!(self, Self::BeforeAll | Self::BeforeEach)
    }

    /// Whether the hook runs around every test of its scope rather than once.
    #[must_use]
    pub const fn is_each(self) -> bool {
        matches!(self, Self::BeforeEach | Self::AfterEach)
    }
}

/// One flake mechanism in a file, where it sits: the raw material of `flake-candidate`.
///
/// Recorded wherever it is written, test or not; the rule decides by scope which tests it
/// reaches (their body, a hook in scope, a setup file, a test-only module whose named code
/// their own code calls into).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FlakeSite {
    /// Repo-relative, slash-separated path of the file. First sort key.
    pub file: String,
    /// Location of the mechanism (the clock read, the random call, the wait). Second sort
    /// key.
    pub span: Span,
    /// Which mechanism it is.
    pub kind: FlakeKind,
    /// What it draws on, matched against the guards in scope.
    pub source: FlakeSource,
    /// The mechanism as written, shortened to one line, for the finding's evidence.
    pub what: String,
    /// False when the reader could not follow the value to its use (a clock value passed
    /// through a call it cannot see into before it reaches an assertion).
    pub decided: bool,
    /// The hook whose callback the mechanism is written in, innermost: inside the callback
    /// itself, or inside a top-level function of the file passed to the hook by name
    /// (`beforeEach(applyMocks)`). `None` in a test body, a helper, or at module scope.
    pub hook: Option<Hook>,
    /// The global a `stubGlobal` names, when it is a literal; `None` for every other
    /// mechanism.
    pub target: Option<String>,
    /// For a filesystem call, whether it writes a path outside a temp dir: false for a
    /// read, and for a copy of a file into a temp dir, which reads its source only. False
    /// for every other mechanism.
    pub writes: bool,
    /// For a clock read, whether the assertion re-reads a calendar field of it (the year,
    /// the date) that the code under test reads too, so a date boundary fails it, rather
    /// than measuring elapsed time between two reads. False for every other mechanism.
    #[serde(default)]
    pub calendar: bool,
    /// The code the mechanism is written in, for the rule to tell which tests reach it in
    /// a test-only module (see [`Enclosing`]).
    pub enclosing: Enclosing,
}

/// The code a flake mechanism or a name reference is written in, innermost named first.
///
/// A function is named by its own identifier, or by what it is bound to: a
/// `const x = () => …` (or `function`, or `class`) declarator, an object property or method,
/// a class method, an arrow-valued class field, an assignment to a member (`exports.x = …`),
/// or `default` for an unnamed default export. A class's constructor and instance field initialisers run on `new`, so they are named by
/// the class. A function with no name of its own takes its enclosing function's; at module
/// scope, the name of the `const`/`let` whose initialiser holds it. A callback a module-scope
/// call registers as a hook or a module mock, or an immediately invoked function, runs at
/// load, like module scope; any other function at module scope with nothing to name it (one
/// assigned to `module.exports`, an argument of another call) is anonymous.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Enclosing {
    /// Module scope, and what runs at load with it: every importer runs it.
    #[default]
    Module,
    /// The innermost named function, method or class.
    Named(String),
    /// A function the reader cannot tie to a name its importers could use.
    Anonymous,
}

/// The names one file's code references, by the code that references them.
///
/// The raw material of the intra-module call graph `flake-candidate` follows into a
/// test-only module, and of the names a test's own code uses.
///
/// A name is an identifier read or called, the property of a member expression
/// (`this.search` and `provider.getById` both give their property, as does
/// `this['search']`), the name an import refers to in its module, or the property a renamed
/// destructuring takes; [`FlakeNames::DYNAMIC`] stands for a call that may reach any name
/// (see `holders`). Names are matched as written, never resolved to a binding: two methods
/// called `search` are one name, which over-approximates the code a name reaches.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FlakeNames {
    /// Repo-relative, slash-separated path of the file. Sort key.
    pub file: String,
    /// For each enclosing code (see [`Enclosing`]), the names it references.
    pub calls: std::collections::BTreeMap<Enclosing, std::collections::BTreeSet<String>>,
    /// Every reference with its byte offset, in source order; kept only for a file with
    /// recognised test cases, where the rule places each one in a test's scope.
    pub uses: Vec<(u32, String)>,
}

impl FlakeNames {
    /// The name code that may call anything references: a call through a computed member
    /// whose key is not a literal, or an iteration over an import's values. No identifier
    /// spells it.
    pub const DYNAMIC: &'static str = "*";
}

/// One call that neutralises a flake mechanism where it is in scope: fake timers, a
/// system-time set, a seed, or a mock of a randomness source.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FlakeGuard {
    /// Repo-relative, slash-separated path of the file. First sort key.
    pub file: String,
    /// Location of the call. Second sort key.
    pub span: Span,
    /// The sources it neutralises, sorted.
    pub covers: Vec<FlakeSource>,
    /// The one global it puts back, for a `stubGlobal` of a global back to its original;
    /// `None` when it neutralises every mechanism of the sources it covers.
    pub target: Option<String>,
    /// The call as written, shortened to one line.
    pub what: String,
}

/// One access to a module-scope binding that holds state: the raw material of the
/// `shared-state` and `order-dependence` kinds.
///
/// The state is a `let`/`var`, or a `const` bound to an object, an array or a `new`
/// collection. Writes are recorded wherever they sit, reads only inside a function; the rule
/// places each in a test body or a hook by span.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StateAccess {
    /// Repo-relative, slash-separated path of the file. First sort key.
    pub file: String,
    /// Location of the access (the reference, or the whole write). A scoped copy for a
    /// named hook callback is anchored at its registration. Second sort key.
    pub span: Span,
    /// The module-scope binding accessed.
    pub binding: String,
    /// True for a write: an assignment or update of the binding or of a member under it, a
    /// `delete` of such a member, a mutating method call on it (`push`, `set`, `clear`, …)
    /// or an `Object.assign` into it. False for any other reference.
    pub write: bool,
    /// The hook whose callback the access sits in, innermost: a write in a `beforeEach` /
    /// `afterEach` is a reset between tests. `None` outside every hook.
    pub hook: Option<Hook>,
    /// True for a plain assignment to the binding itself (`chain = …`), which replaces its
    /// value whatever an earlier write left in it.
    pub replaces: bool,
    /// The access as written, shortened to one line, for the finding's evidence.
    pub what: String,
}

// ---------------------------------------------------------------- Pairing

/// Which strategy paired a test with a source module. Ordered by precedence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PairingReason {
    /// The test file imports the module directly.
    #[serde(rename = "import")]
    DirectImport,
    /// `x.test.ts` next to `x.ts`.
    #[serde(rename = "sibling")]
    SiblingName,
    /// `__tests__/x.test.ts` above `x.ts`.
    #[serde(rename = "tests-dir")]
    TestsDirectory,
}

/// A test file and the source module it exercises.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Pairing {
    /// Repo-relative, slash-separated path of the test file. First sort key.
    pub test_file: String,
    /// Repo-relative, slash-separated path of the source module. Second sort key.
    pub source_module: String,
    /// Which strategy decided it.
    pub reason: PairingReason,
    /// True when the strategy had to break a tie between several candidates — an
    /// `import` pairing chosen by name from more than one imported module — so a later
    /// rule can trust an unambiguous pairing more than one that guessed by name.
    pub ambiguous: bool,
}

// ---------------------------------------------------------------- Handler

/// The library a handler belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandlerLibrary {
    /// Mock Service Worker.
    Msw,
    /// nock.
    Nock,
    /// Recognised as a handler, library undecided.
    Unknown,
}

/// How a handler's URL pattern was written, so a rule matching it knows whether the folded
/// `pattern` is a comparable path or a thing it must never judge.
///
/// A string or a substitution-free template folds to a path `handler-drift` matches with
/// MSW's rules. A regex-literal pattern (`http.get(/status-message/, …)`) is recorded and
/// never judged — MSW runs it against the URL at runtime, which the model cannot do
/// statically. A computed pattern (a variable, a call, a template with substitutions) is
/// opaque: recorded so it is not dropped, but `pattern` is `None` and the rule says "could
/// not decide".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternKind {
    /// A string literal (`'/api/user'`).
    String,
    /// A substitution-free template literal (`` `/api/user` ``), folded like a string.
    Template,
    /// A regular-expression literal (`/status-message/`): recorded, never judged.
    Regex,
    /// Anything else — a variable, a call, a template with substitutions: `pattern` is
    /// `None` and a rule reports "could not decide" rather than guess.
    Opaque,
}

/// One network request interceptor declared in the tree.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Handler {
    /// Repo-relative, slash-separated path it is declared in. First sort key.
    pub file: String,
    /// Location of the declaration. Second sort key.
    pub span: Span,
    /// Which library declares it.
    pub library: HandlerLibrary,
    /// HTTP method, uppercased; `None` for "any method".
    pub method: Option<String>,
    /// The URL pattern as folded; `None` when it could not be decided (a regex literal, or
    /// a computed pattern). See `pattern_kind` for which.
    pub pattern: Option<String>,
    /// How the pattern was written, so a matcher knows a regex from an opaque computation
    /// even though both leave `pattern` `None`.
    pub pattern_kind: PatternKind,
}

// ---------------------------------------------------------------- Gate

/// Which tool's config a gate was read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateTool {
    /// `vitest.config.*`.
    Vitest,
    /// `jest.config.*`.
    Jest,
    /// `playwright.config.*`.
    Playwright,
    /// `stryker.conf.*`.
    Stryker,
    /// A GitHub Actions workflow or the root `CircleCI` config.
    Ci,
    /// Recognised as a gate config, tool undecided.
    Unknown,
}

/// A test file its framework's runner would never collect: a test that exists and never runs.
///
/// The model recognised it as a test file — a recogniser claimed at least one `test(…)` /
/// `it(…)` in it — yet the runner's include and exclude globs omit it. Filled by
/// [`scope`](crate::scope) as it drops the file's cases from the [`TestCase`] table, so
/// `orphan-test` can report the file the runner's own listing omits and name the config and
/// globs it was judged against.
///
/// An in-memory fact for the rule, not part of the serialized model: [`crate::Model`] skips
/// it, since a never-collected file is a finding, not a table the `list` command exposes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Uncollected {
    /// Repo-relative, slash-separated path of the recognised test file.
    pub file: String,
    /// The framework the file's cases were attributed to, when one was known.
    pub framework: Option<String>,
    /// The runner config the file was judged against, or `None` when no config governs it
    /// and the runner's own defaults decided.
    pub config: Option<String>,
    /// The include globs the file was judged against — the config's own, or the runner's
    /// defaults where it declared none.
    pub include: Vec<String>,
    /// The exclude globs the file was judged against, likewise.
    pub exclude: Vec<String>,
    /// True when the file sits outside every workspace member and no config that reaches it
    /// collects it: the globs never decided, the workspace boundary did. A file the workspace
    /// covers but whose globs omit it reads `false`.
    pub outside_workspace: bool,
}

// ---------------------------------------------------------------- Churn

/// The lines a supplied `git log --numstat` changed in one path, summed over every commit
/// the file lists.
///
/// An in-memory fact for `untested-hotspot`, not part of the serialized model: the binary
/// reads the `inputs.churn` file (or `--churn`) through the `spirolo-inputs` churn reader and
/// fills [`crate::Model::churn`] after the build. A rename counts toward its new path; a
/// binary row (`-`) adds no lines. Sorted by path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Churn {
    /// Repo-relative, slash-separated path, as the numstat names it (the new path of a
    /// rename). Sort key.
    pub path: String,
    /// Lines added, summed.
    pub added: u64,
    /// Lines deleted, summed.
    pub deleted: u64,
}

// ---------------------------------------------------------------- Snapshot

/// One `exports[`key`]` entry a `.snap` file declares.
///
/// A Vitest/Jest external snapshot key is the test's full name — the `>`-joined `describe`
/// titles and the test title — then, for a snapshot taken with a hint, ` > hint` (Jest:
/// `: hint`), then a space and a one-based ordinal that counts the snapshots. A hint or a
/// title may itself hold ` > `, so the key cannot be split back into its parts: it is kept
/// whole, with only the ordinal lifted off, and matched against a test's name by prefix.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SnapshotEntry {
    /// The key as the runner names it, for example `Widget > renders 1`: the text between
    /// the backticks with the template literal's escapes undone (a backslash before a
    /// backtick, a backslash or a dollar sign).
    pub key: String,
    /// The one-based ordinal that distinguishes a test's several snapshots.
    pub ordinal: u32,
    /// The one-based line of the entry's `exports[…]` statement in the `.snap` file.
    pub line: u32,
}

/// One `__snapshots__/<name>.snap` file beside a test file, with the entries it declares
/// and the sibling test file it belongs to.
///
/// An in-memory fact for `stale-snapshot`, not part of the serialized model: the binary
/// walks the `.snap` files, reads each with the frameworks reader, and fills this table
/// after [`crate::Model::build`], the way it fills the mutation tables. `sibling` is the
/// `<name>` test file beside the `__snapshots__` directory when one exists in the model;
/// `None` names an orphan `.snap` whose test file is gone, stale as a whole.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Snapshot {
    /// Repo-relative, slash-separated path of the `.snap` file. Sort key.
    pub file: String,
    /// The sibling test file, repo-relative, when one exists in the model; `None` when the
    /// `.snap` outlived its test file.
    pub sibling: Option<String>,
    /// The entries the file declares, sorted by key.
    pub entries: Vec<SnapshotEntry>,
    /// The functions the sibling imports and calls where tests are registered
    /// (`formTests(Form)`, `describeWithPointerEvent(…)`), sorted: tests such a call
    /// registers have names the sibling does not spell. Empty without a sibling.
    pub imported_calls: Vec<String>,
}

/// A boolean gate flag as a config writes it: a literal, or a value that holds under CI.
///
/// Playwright's own scaffold writes `forbidOnly: !!process.env.CI`, which is neither `true`
/// nor `false` statically but is decidedly *on* in the build that matters. The reader folds
/// the CI-conditional spellings (`!!process.env.CI`, `Boolean(process.env.CI)`,
/// `process.env.CI === "true"`, and `||` / `&&` / `?:` compositions of them) under the
/// assumption that CI is set, and records [`GateFlag::CiConditional`] when the flag is true
/// there. A rule that judges the CI build treats it as set; a rule that needs the local
/// value can tell it apart from a literal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateFlag {
    /// The literal `true`.
    True,
    /// The literal `false`.
    False,
    /// An expression that is true when CI is set (`!!process.env.CI`).
    CiConditional,
}

impl GateFlag {
    /// Whether the flag is on in a CI build: a literal `true`, or a CI-conditional value.
    #[must_use]
    pub const fn holds_in_ci(self) -> bool {
        matches!(self, Self::True | Self::CiConditional)
    }
}

/// One numeric threshold a config enforces.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Threshold {
    /// Metric name as the tool spells it, for example `lines` or `branches`.
    pub metric: String,
    /// The configured value.
    pub value: f64,
}

/// Whether one fail-closed invariant is present in a runner's setup, read by AST.
///
/// A setup either fails closed on a class of silent pass (an unhandled request, a logged
/// error, a leaked mock, a per-test mock server) or it does not; the third state is when
/// the invariant does not apply — the MSW ones are [`Invariant::NotApplicable`] in a project
/// with no mock server, so a project without MSW is never told to configure one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Invariant {
    /// The setup fails closed on this class: the invariant holds.
    Present,
    /// The setup lets this class pass silently: the invariant is missing.
    Absent,
    /// The invariant does not apply here (an MSW invariant with no mock server present), so
    /// it is neither a finding nor an `ok` — the rule skips it entirely.
    NotApplicable,
}

/// The four fail-closed invariants read from a runner's setup files and its mock-reset config.
///
/// Records where each was read too, so `fail-closed-setup` can say which one is missing and
/// point at the file that owns it. Read by [`crate::setup`] after the gates are built: each
/// Vitest/Jest gate's `setup_files` are re-read as source (the same way the preset reader
/// re-reads a config) and scanned for the MSW `listen` options, a throwing console guard or a
/// fail-on-console package, an `afterEach(restoreAllMocks)`, and a `beforeAll(server.listen)`.
/// The mock-reset invariant also reads the gate's own `restore_mocks`. Nothing is executed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetupInvariants {
    /// `listen({ onUnhandledRequest: "error" })` on the mock server, so a request no handler
    /// matches fails the test instead of passing with a warning. Applies only with MSW.
    pub unhandled_request_error: Invariant,
    /// A `console.error`/`console.warn` spy whose implementation throws, or a fail-on-console
    /// package (`vitest-fail-on-console`, `jest-fail-on-console`), so a logged error fails
    /// the test rather than scrolling past.
    pub console_guard: Invariant,
    /// `restoreMocks: true` in the gate or `restoreAllMocks()` in an `afterEach`, so a mock
    /// set in one test cannot leak into the next.
    pub restore_mocks: Invariant,
    /// The mock server's `listen()` called in a setup-file `beforeAll` rather than per test,
    /// so every test runs behind the server. Applies only with MSW.
    pub global_server: Invariant,
    /// Whether this gate's runner uses MSW at all, so the two MSW invariants are judged only
    /// when a mock server is present. A `server.listen()` in one of the gate's setup files
    /// (own or inherited) or an MSW handler file under the config's directory counts as
    /// present; a handler in another package does not.
    pub msw_present: bool,
    /// Whether the mock server is started by a setup file a preset declares (see
    /// [`Gate::inherited_setup_files`]) rather than by the config's own, so a missing MSW
    /// invariant is the preset's decision, lifted to the preset, not the inheritor's.
    /// `false` when MSW is absent or when the config's own setup starts the server.
    pub msw_inherited: bool,
    /// The setup file the invariants were read from, when the gate has one — the file a
    /// finding points at. `None` when the gate declares no setup file.
    pub setup_file: Option<String>,
}

impl SetupInvariants {
    /// The invariants of a gate whose setup files have not been read: every invariant absent,
    /// no MSW, no setup file. [`crate::setup`] overwrites this once it reads the files.
    #[must_use]
    pub const fn unread() -> Self {
        Self {
            unhandled_request_error: Invariant::Absent,
            console_guard: Invariant::Absent,
            restore_mocks: Invariant::Absent,
            global_server: Invariant::Absent,
            msw_present: false,
            msw_inherited: false,
            setup_file: None,
        }
    }
}

/// One preset a config inherits from, as the import is written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresetImport {
    /// The specifier as written: a package export (`@acme/config-vitest/base`) or a
    /// relative path (`../config/base.mjs`).
    pub specifier: String,
    /// The named export the config takes from the preset module (`import { strict } from`,
    /// `export { strict as default } from`); `None` for the module's default export.
    pub name: Option<String>,
}

/// What actually fails the build, read from one config file by AST. Never executed.
///
/// Every field is read from the exported object literal; a value the reader could not
/// decide statically (a spread, a function, a `require`) leaves its field empty and sets
/// [`Gate::opaque`], so a later rule says "could not decide" rather than guessing.
///
/// Its booleans are independent facts about one config, not a state a flag enum would model
/// better, so the `struct_excessive_bools` lint is allowed here.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Gate {
    /// Repo-relative, slash-separated path of the config. The sort key.
    pub source: String,
    /// Which tool the config belongs to.
    pub tool: GateTool,
    /// Coverage and mutation thresholds, sorted by metric.
    pub thresholds: Vec<Threshold>,
    /// Setup files the runner loads, in the order it loads them: those the presets in
    /// [`Gate::preset_chain`] declare first (spelled from the root, see
    /// [`Gate::inherited_setup_files`]), then the config's own as written.
    pub setup_files: Vec<String>,
    /// The subset of [`Gate::setup_files`] the preset chain declared, spelled from the root
    /// (`packages/config/setup.ts`), so the setup reader can tell a mock server a preset
    /// starts — the preset's decision, lifted there — from one the config's own setup
    /// starts. Empty when no preset declares a setup file.
    pub inherited_setup_files: Vec<String>,
    /// The globs that decide which files the runner treats as tests, in source order.
    /// A test file outside every include glob is a module but not a test.
    pub include: Vec<String>,
    /// Jest's `testRegex`, an alternative to `testMatch`, matched against paths.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub test_regex: Vec<String>,
    /// File selection of inline Vitest/Jest projects. These scopes are alternatives,
    /// not one intersection of all projects' include and exclude settings.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub collection_projects: Vec<CollectionProject>,
    /// Jest's explicit rootDir, resolved relative to the analysis root. An empty
    /// string denotes that root (including the static `process.cwd()` spelling).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collection_root: Option<String>,
    /// Jest's `roots`, limiting test discovery to these directories under rootDir.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub collection_roots: Vec<String>,
    /// The globs that exclude a file from being a test, in source order.
    pub exclude: Vec<String>,
    /// Jest's `moduleNameMapper`, in source order: each regular expression with the paths
    /// it maps a matching specifier to (`<rootDir>` and `$1` as written). An entry whose
    /// key or value is not a literal is left out.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub module_name_mapper: Vec<(String, Vec<String>)>,
    /// Configured retry count, when the tool has one (`retry`, `retries`). A
    /// `process.env.CI ? 2 : 0` ternary reads as its CI branch, since the gate is judged
    /// for the build CI runs.
    pub retries: Option<u32>,
    /// Vitest/Jest `restoreMocks`, when set.
    pub restore_mocks: Option<bool>,
    /// Vitest/Jest `clearMocks`, when set.
    pub clear_mocks: Option<bool>,
    /// Vitest `mockReset` or Jest `resetMocks`, resetting mock implementations per test.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reset_mocks: Option<bool>,
    /// Vitest `unstubGlobals`, when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unstub_globals: Option<bool>,
    /// Vitest `unstubEnvs`, when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unstub_envs: Option<bool>,
    /// Playwright's `forbidOnly`, when set: a literal, or a value that holds under CI
    /// (`!!process.env.CI`, the scaffold's own spelling). `None` when absent or unread.
    pub forbid_only: Option<GateFlag>,
    /// Playwright's `testDir`, when set.
    pub test_dir: Option<String>,
    /// Playwright project names, in source order; a project whose name could not be read
    /// is omitted and sets [`Gate::opaque`].
    pub projects: Vec<String>,
    /// Stryker's `mutate` globs — the files it mutates — in source order.
    pub mutate: Vec<String>,
    /// Stryker's `testRunner`, when set.
    pub test_runner: Option<String>,
    /// Stryker's `break` score, when set. `0` means mutation never fails the build.
    pub mutation_break: Option<f64>,
    /// The presets this gate's config inherits from, as written: each import the export
    /// takes its value from — `mergeConfig(preset, …)`, `export default preset`,
    /// `export { default } from`, `defineConfig(preset)`, `preset(…)` — with the specifier
    /// and the named export it takes. Recorded by the config recogniser, which sees one file;
    /// resolved and read by [`crate::presets`] into [`Gate::preset_chain`]. Empty when the
    /// config imports no preset.
    pub preset_imports: Vec<PresetImport>,
    /// The imported presets this gate's config inherits from, resolved to repo-relative paths
    /// in inheritance order (a variant before the base it merges) — the chain the preset
    /// follower read [`Gate::preset_imports`] through. A finding that a shared preset lacks an
    /// invariant is lifted to the preset every gate in the chain names, so a monorepo whose
    /// packages compose one preset reports one finding, not one per package. Empty when the
    /// config imports no preset, or when none resolved inside the root.
    pub preset_chain: Vec<String>,
    /// The four fail-closed invariants read from the setup files and mock-reset config. Read
    /// by [`crate::setup`] after the gate is built; [`SetupInvariants::unread`] until then.
    pub setup: SetupInvariants,
    /// CI run commands, including reached package scripts and Makefile recipes, in
    /// workflow order. Cypress test actions contribute `cypress run`; install-only
    /// actions do not. Populated only for a [`GateTool::Ci`] gate by [`crate::ci`].
    pub steps: Vec<String>,
    /// True when part of the config could not be decided statically. Exactly when
    /// [`Gate::unread`] is non-empty.
    pub opaque: bool,
    /// What the reader could not decide, one entry per cause in the order met — a spread,
    /// a function export, a `require`, a computed glob, a preset it could not resolve —
    /// so a rule that says "could not decide" can say what. Empty when the config was read
    /// whole. Every entry is recorded through [`Gate::could_not_read`].
    pub unread: Vec<String>,
    /// True when the config sets a custom `resolveSnapshotPath` — a snapshot-path convention
    /// the model cannot follow, so `stale-snapshot` says "could not decide" for the whole
    /// project rather than report every `.snap` as orphaned. An in-memory fact for the rule,
    /// not part of the serialized gate.
    #[serde(skip)]
    pub snapshot_resolver: bool,
    /// Jest's own `preset: 'name'` key, as written: a package whose `jest-preset` module the
    /// config inherits, or a relative path to one. Followed by [`crate::presets`] when it
    /// resolves inside the root; an in-memory fact, not part of the serialized gate.
    #[serde(skip)]
    pub jest_preset: Option<String>,
    /// True for a Vitest/Jest gate whose own include/testMatch (or the runner's default)
    /// matches no file the walk found: a fixture app's `jest` block with only app sources
    /// beside it. Its setup protects no test. Set by [`crate::scope`]; an in-memory fact.
    #[serde(skip)]
    pub collects_no_tests: bool,
    /// True when what the gate collects hangs on a key the reader does not apply: Vitest's
    /// `root`, or the `testMatch`/`testRegex`/`roots`/`rootDir` of a Jest preset the gate
    /// inherits. [`crate::scope`] never judges such a gate as collecting no test. An
    /// in-memory fact.
    #[serde(skip)]
    pub selection_unapplied: bool,
    /// The string entries of a Vitest/Jest `projects` list, as written (`packages/*`,
    /// `<rootDir>/packages/*/jest.config.js`): the directories or configs this run loads as
    /// its projects. An in-memory fact for [`crate::scope`].
    #[serde(skip)]
    pub project_globs: Vec<String>,
}

impl Gate {
    /// Record one thing the reader could not decide about this config: `what` joins
    /// [`Gate::unread`] (once — the same cause met twice is one entry) and the gate is
    /// opaque from here on.
    pub fn could_not_read(&mut self, what: impl Into<String>) {
        let what = what.into();
        if !self.unread.contains(&what) {
            self.unread.push(what);
        }
        self.opaque = true;
    }

    /// The gate before any field is read: the source and tool decided, everything else
    /// empty and nothing opaque. Every reader starts from here.
    #[must_use]
    pub fn blank(source: &str, tool: GateTool) -> Self {
        Self {
            source: source.to_owned(),
            tool,
            thresholds: Vec::new(),
            setup_files: Vec::new(),
            inherited_setup_files: Vec::new(),
            include: Vec::new(),
            test_regex: Vec::new(),
            collection_projects: Vec::new(),
            collection_root: None,
            collection_roots: Vec::new(),
            exclude: Vec::new(),
            module_name_mapper: Vec::new(),
            retries: None,
            restore_mocks: None,
            clear_mocks: None,
            reset_mocks: None,
            unstub_globals: None,
            unstub_envs: None,
            forbid_only: None,
            test_dir: None,
            projects: Vec::new(),
            mutate: Vec::new(),
            test_runner: None,
            mutation_break: None,
            preset_imports: Vec::new(),
            preset_chain: Vec::new(),
            setup: SetupInvariants::unread(),
            steps: Vec::new(),
            opaque: false,
            unread: Vec::new(),
            snapshot_resolver: false,
            jest_preset: None,
            collects_no_tests: false,
            selection_unapplied: false,
            project_globs: Vec::new(),
        }
    }
}

// ------------------------------------------------- Run / Coverage / Mutants
// Declared here so the schema is complete; populated in M4 from files other
// tools write. Nothing in this workspace ever executes a test to fill them.

/// How one test ended in one run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunOutcome {
    /// Passed.
    Passed,
    /// Failed.
    Failed,
    /// Did not run.
    Skipped,
    /// Failed at least once, then passed when the runner retried it: both a pass and a fail.
    PassedOnRetry,
}

/// One test's outcome in one recorded run, ingested from `JUnit` or runner JSON.
///
/// Sorted by file, title, run id and attempt: the field order below.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Run {
    /// Repo-relative, slash-separated path of the test file. First sort key.
    pub file: String,
    /// Full title, suite path included, the parts joined by `" > "`. Second sort key.
    pub title: String,
    /// The run this row belongs to: one ingested report. Third sort key.
    pub run_id: String,
    /// Which attempt of the run this is, from 0. Fourth sort key.
    pub attempt: u32,
    /// The title as a path: the enclosing `describe` titles, then the test's own title, as
    /// [`TestCase::suite_path`] and [`TestCase::title`] carry them. Empty on a file-level
    /// row: the file failed outside any test (a hook threw, or the module did not load).
    pub title_path: Vec<String>,
    /// How it ended.
    pub outcome: RunOutcome,
    /// Wall-clock duration in milliseconds, when the reporter recorded one.
    pub duration_ms: Option<u64>,
    /// Retries the runner performed before settling.
    pub retries: Option<u32>,
    /// Hash of the tree the run was made on, so the ledger can be attributed.
    pub tree_hash: Option<String>,
    /// When the run started, in milliseconds since the Unix epoch, as the report records it.
    pub started_at_ms: Option<u64>,
    /// The label the run was ingested under, for example `idle` or `load`.
    pub condition: Option<String>,
    /// In-tree path of the report the row was read from.
    pub report: String,
    /// Stable runner/configuration identity. Unlabelled legacy runs share one context.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runner_home: Option<String>,
    /// The `TestCase` the row joined to by file and title path, or `None` when the model
    /// recognises no such test: the row is kept, marked unjoined, never dropped.
    pub joined: Option<JoinedCase>,
}

/// Execution count for one line, ingested from a coverage report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LineCoverage {
    /// One-based line number.
    pub line: u32,
    /// Times the line was executed.
    pub hits: u64,
}

/// One file's coverage, ingested from `coverage-final.json` or lcov.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Coverage {
    /// Repo-relative, slash-separated path. The sort key.
    pub file: String,
    /// Per-line hits, sorted by line.
    pub lines: Vec<LineCoverage>,
}

/// What a mutation run did to one mutant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutantStatus {
    /// A test caught it.
    Killed,
    /// Every test still passed.
    Survived,
    /// No test covered it.
    NoCoverage,
    /// The run timed out.
    Timeout,
    /// The mutated code threw at run time (not a test assertion).
    RuntimeError,
    /// The mutated code did not compile.
    CompileError,
    /// Excluded by configuration.
    Ignored,
    /// Recognised but not yet run.
    Pending,
}

/// One mutant, ingested from a mutation-testing-elements `mutation.json`. First of the
/// three row kinds the reader produces (see also [`MutationTest`] and [`MutationReport`]).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Mutants {
    /// Repo-relative, slash-separated path of the mutated file. First sort key.
    pub file: String,
    /// Location of the mutation. Second sort key.
    pub span: Span,
    /// The report's own mutant id.
    pub id: String,
    /// The mutator that produced it, for example `ConditionalExpression`.
    pub mutator: Option<String>,
    /// What the run did to it.
    pub status: MutantStatus,
    /// The report ids of the tests that killed it, sorted and deduplicated.
    pub killed_by: Vec<String>,
    /// The report ids of the tests that covered it, sorted and deduplicated.
    pub covered_by: Vec<String>,
    /// Path of the report this mutant came from — the join key to [`MutationReport`].
    pub report: String,
}

/// One test a mutation report names, joined to the model's `TestCase` where it can be.
/// Second of the three row kinds the reader produces.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MutationTest {
    /// Path of the report this test was named in. First sort key.
    pub report: String,
    /// The report's own test id, referenced by [`Mutants::killed_by`] and `covered_by`.
    pub id: String,
    /// The test file, repo-relative and slash-separated: the report's package-relative key
    /// under the report's package directory. Second sort key.
    pub file: String,
    /// The full test name as the runner reported it: suite titles and title joined by
    /// single spaces, as Stryker writes it.
    pub name: String,
    /// The `TestCase` this test joined to by file and full name, or `None` when the
    /// model recognised no such test — a report that names a test the model cannot see.
    pub joined: Option<JoinedCase>,
}

/// The identity of the `TestCase` a [`MutationTest`] joined to: its file and span, so a
/// consumer can find the row without re-deriving the join.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct JoinedCase {
    /// Repo-relative, slash-separated path of the joined `TestCase`.
    pub file: String,
    /// Span of the joined `TestCase`.
    pub span: Span,
}

/// Whether a mutation report's kill sets are whole or truncated by Stryker's bail.
///
/// With bail on, `killedBy` holds only the first killing test, so every test looks
/// redundant with the first; the reader flags such a report as unusable for redundancy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Bail {
    /// The report's config says bail was off (`disableBail: true`): the kill sets are whole.
    Disabled,
    /// The report's config says bail was on: `killedBy` holds only the first killer.
    Declared,
    /// No config, but every killed mutant has exactly one killer while some mutant is
    /// covered by more than one test — the signature of a bailed run.
    Inferred,
    /// Cannot tell: no config, and too little per-test data to infer either way.
    Unknown,
}

/// One mutation report the reader ingested. Third of the three row kinds; the diagnostic
/// a report the rule cannot use is carried alongside, so a bad report is data, not silence.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MutationReport {
    /// Repo-relative, slash-separated path of the report. Sort key and the join key from
    /// [`Mutants::report`] and [`MutationTest::report`].
    pub path: String,
    /// The report's `schemaVersion`, verbatim, or `None` when it names none.
    pub schema_version: Option<String>,
    /// The tool that wrote the report, for example `StrykerJS`, from `framework.name`.
    pub tool: Option<String>,
    /// The tool's version, from `framework.version`.
    pub tool_version: Option<String>,
    /// Whether the report carries per-test data: a `testFiles` block naming tests and
    /// `coveredBy` / `killedBy` referencing them. Without it `redundant-test` cannot run.
    pub per_test_data: bool,
    /// Whether the kill sets are whole or truncated by bail.
    pub bail: Bail,
}
