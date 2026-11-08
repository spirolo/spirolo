//! Mocks on a `TestCase`: `vi.mock`/`jest.mock`, `vi.spyOn`, `vi.fn`, fake timers and
//! MSW `server.use`, each with its kind, target and span. A module mock is written at the
//! top of the file, where the runner hoists it, and lands on every case in the file; a
//! spy or timer installed in a `describe` lands on the cases of that suite. Reads only
//! the rows returned.

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use spirolo_frameworks::VitestJest;
use spirolo_model::{FileContext, Language, Mock, MockKind, Recogniser, TestCase, TestKind};

/// Run the recogniser over one TypeScript snippet, as if it were `src/example.test.ts`.
fn recognise(source: &str) -> Vec<TestCase> {
    let allocator = Allocator::default();
    let path = "src/example.test.ts";
    let parsed = Parser::new(&allocator, source, SourceType::from_path(path).unwrap()).parse();
    let file = FileContext {
        path,
        language: Language::TypeScript,
        source,
        program: &parsed.program,
        handler_globs: None,
    };
    VitestJest.recognise(&file)
}

/// The mocks of the first test in a snippet.
fn mocks_of(source: &str) -> Vec<Mock> {
    recognise(source)
        .into_iter()
        .find(|row| row.kind == TestKind::Test)
        .expect("a test")
        .mocks
}

// ----------------------------------------------------------------- one snapshot per idiom

#[test]
fn vi_mock_with_a_factory() {
    insta::assert_json_snapshot!(recognise(
        "import { it, vi } from 'vitest'\nvi.mock('./db', () => ({ query: vi.fn() }))\nit('t', () => { run() })\n"
    ));
}

#[test]
fn vi_mock_bare() {
    insta::assert_json_snapshot!(recognise(
        "import { it, vi } from 'vitest'\nvi.mock('./db')\nit('t', () => { run() })\n"
    ));
}

#[test]
fn jest_mock() {
    insta::assert_json_snapshot!(recognise(
        "import { it, jest } from '@jest/globals'\njest.mock('axios')\nit('t', () => { run() })\n"
    ));
}

#[test]
fn vi_do_mock_inside_the_body() {
    insta::assert_json_snapshot!(recognise(
        "import { it, vi } from 'vitest'\nit('t', async () => { vi.doMock('./db'); await run() })\n"
    ));
}

#[test]
fn vi_spy_on() {
    insta::assert_json_snapshot!(recognise(
        "import { it, vi } from 'vitest'\nit('t', () => { vi.spyOn(console, 'log') })\n"
    ));
}

#[test]
fn vi_fn() {
    insta::assert_json_snapshot!(recognise(
        "import { it, vi } from 'vitest'\nit('t', () => { const fn = vi.fn() })\n"
    ));
}

#[test]
fn fake_timers() {
    insta::assert_json_snapshot!(recognise(
        "import { it, vi } from 'vitest'\nit('t', () => { vi.useFakeTimers() })\n"
    ));
}

#[test]
fn msw_server_use() {
    insta::assert_json_snapshot!(recognise(
        "import { it } from 'vitest'\nit('t', () => { server.use(http.get('/api', resolver)) })\n"
    ));
}

// ------------------------------------------------------------ what the snapshots must hold

#[test]
fn a_module_mock_records_its_specifier_and_factory() {
    let mocks = mocks_of(
        "import { it, vi } from 'vitest'\nvi.mock('./db', () => ({}))\nit('t', () => { run() })\n",
    );
    assert_eq!(mocks.len(), 1);
    assert_eq!(mocks[0].kind, MockKind::Module);
    assert_eq!(mocks[0].target.as_deref(), Some("./db"));
    assert!(mocks[0].factory, "a factory was supplied");
}

#[test]
fn a_bare_module_mock_has_no_factory() {
    let mocks =
        mocks_of("import { it, vi } from 'vitest'\nvi.mock('./db')\nit('t', () => { run() })\n");
    assert_eq!(mocks[0].kind, MockKind::Module);
    assert!(!mocks[0].factory, "no factory means an auto-mock");
}

#[test]
fn a_hoisted_module_mock_lands_on_every_test_in_the_file() {
    let rows = recognise(
        "import { describe, it, vi } from 'vitest'\n\
         vi.mock('./db')\n\
         it('a', () => { run() })\n\
         describe('s', () => {\n\
         \x20 vi.mock('./mail', () => ({}))\n\
         \x20 it('b', () => { run() })\n\
         })\n\
         it('c', () => { vi.doMock('./late') })\n",
    );
    let targets = |title: &str| -> Vec<String> {
        rows.iter()
            .find(|row| row.title.as_deref() == Some(title))
            .expect(title)
            .mocks
            .iter()
            .map(|mock| mock.target.clone().unwrap_or_default())
            .collect()
    };
    // `vi.mock` is hoisted to the top of the file wherever it is written, so the one
    // inside the describe reaches `a` and `c` too; the in-body doMock is `c`'s alone.
    assert_eq!(targets("a"), ["./db", "./mail"]);
    assert_eq!(targets("b"), ["./db", "./mail"]);
    assert_eq!(targets("c"), ["./db", "./mail", "./late"]);
    let suite = rows
        .iter()
        .find(|row| row.kind == TestKind::Suite)
        .expect("the suite");
    assert!(
        suite.mocks.is_empty(),
        "a suite row carries no mocks; its cases do"
    );
}

#[test]
fn a_spy_installed_in_a_describe_lands_on_that_suites_cases_only() {
    let rows = recognise(
        "import { describe, it, vi } from 'vitest'\n\
         const clock = vi.useFakeTimers()\n\
         describe('one', () => {\n\
         \x20 const spy = vi.spyOn(console, 'log')\n\
         \x20 it('a', () => { run() })\n\
         })\n\
         describe('two', () => {\n\
         \x20 it('b', () => { run() })\n\
         })\n\
         beforeEach(() => { vi.spyOn(console, 'warn') })\n",
    );
    let kinds = |title: &str| -> Vec<(MockKind, Option<String>)> {
        rows.iter()
            .find(|row| row.title.as_deref() == Some(title))
            .expect(title)
            .mocks
            .iter()
            .map(|mock| (mock.kind, mock.target.clone()))
            .collect()
    };
    assert_eq!(
        kinds("a"),
        [
            (MockKind::Timer, None),
            (MockKind::Function, Some("console.log".to_owned()))
        ],
        "module-scope timer plus the suite's own spy, in source order"
    );
    assert_eq!(
        kinds("b"),
        [(MockKind::Timer, None)],
        "the sibling suite's spy does not reach `two`"
    );
    // A hook's body is the hook's own; its spy is on the hook row, not on the cases.
    let hook = rows
        .iter()
        .find(|row| row.kind == TestKind::Hook)
        .expect("the hook");
    assert_eq!(hook.mocks.len(), 1);
}

#[test]
fn a_spy_records_its_object_and_method() {
    let mocks =
        mocks_of("import { it, vi } from 'vitest'\nit('t', () => { vi.spyOn(console, 'log') })\n");
    assert_eq!(mocks[0].kind, MockKind::Function);
    assert_eq!(mocks[0].target.as_deref(), Some("console.log"));
}

#[test]
fn every_mock_row_carries_a_span() {
    let mocks = mocks_of("import { it, vi } from 'vitest'\nit('t', () => { vi.fn() })\n");
    assert_eq!(mocks[0].kind, MockKind::Function);
    assert!(mocks[0].span.start > 0, "a real span");
}

#[test]
fn a_test_with_no_mocks_is_representable() {
    let mocks = mocks_of("import { it } from 'vitest'\nit('t', () => { doSomething() })\n");
    assert!(mocks.is_empty(), "zero mocks, not an error");
}
