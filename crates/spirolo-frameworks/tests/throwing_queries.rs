//! Throwing Testing Library queries on a `TestCase`: `getBy*`, `getAllBy*`, `findBy*`,
//! `findAllBy*`, whether bare, on `screen`, or through `within(...)`. Each anchors the render
//! that a test asserts against, so `absence-anchor` reads them to tell an anchored test from
//! one that only negates. The `queryBy*` family (returns `null`, never throws) is not an
//! anchor and must not be collected.

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use spirolo_frameworks::VitestJest;
use spirolo_model::{FileContext, Language, Recogniser, TestCase, TestKind, ThrowingQuery};

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

/// The throwing queries of the first test in a snippet.
fn queries_of(source: &str) -> Vec<ThrowingQuery> {
    recognise(source)
        .into_iter()
        .find(|row| row.kind == TestKind::Test)
        .expect("a test")
        .throwing_queries
}

// ----------------------------------------------------------------- one snapshot per idiom

#[test]
fn get_by_role_bare() {
    insta::assert_json_snapshot!(recognise(
        "import { it } from 'vitest'\nit('t', () => { getByRole('button') })\n"
    ));
}

#[test]
fn screen_get_by_text() {
    insta::assert_json_snapshot!(recognise(
        "import { it } from 'vitest'\nimport { screen } from '@testing-library/react'\nit('t', () => { screen.getByText('hi') })\n"
    ));
}

#[test]
fn within_get_by_test_id() {
    insta::assert_json_snapshot!(recognise(
        "import { it } from 'vitest'\nimport { within } from '@testing-library/react'\nit('t', () => { within(row).getByTestId('cell') })\n"
    ));
}

#[test]
fn find_by_in_wait_for() {
    insta::assert_json_snapshot!(recognise(
        "import { it } from 'vitest'\nimport { screen, waitFor } from '@testing-library/react'\nit('t', async () => { await waitFor(() => screen.findByText('done')) })\n"
    ));
}

// ------------------------------------------------------------ what the snapshots must hold

#[test]
fn a_bare_get_by_query_is_recorded() {
    let queries =
        queries_of("import { it } from 'vitest'\nit('t', () => { getByRole('button') })\n");
    assert_eq!(queries.len(), 1);
    assert_eq!(queries[0].name, "getByRole");
    assert!(
        !queries[0].within,
        "a bare query is not a within-scoped one"
    );
    assert!(
        queries[0].span.start > 0 && queries[0].span.end > queries[0].span.start,
        "a real span"
    );
}

#[test]
fn a_screen_get_by_query_is_recorded() {
    let queries = queries_of(
        "import { it } from 'vitest'\nimport { screen } from '@testing-library/react'\nit('t', () => { screen.getByText('hi') })\n",
    );
    assert_eq!(queries.len(), 1);
    assert_eq!(queries[0].name, "getByText");
    assert!(!queries[0].within);
}

#[test]
fn a_within_scoped_query_sets_the_within_flag() {
    let queries = queries_of(
        "import { it } from 'vitest'\nimport { within } from '@testing-library/react'\nit('t', () => { within(row).getByTestId('cell') })\n",
    );
    assert_eq!(queries.len(), 1);
    assert_eq!(queries[0].name, "getByTestId");
    assert!(queries[0].within, "within(...).getBy* is scoped");
}

#[test]
fn the_find_by_family_throws_and_is_recorded() {
    let queries = queries_of(
        "import { it } from 'vitest'\nimport { screen } from '@testing-library/react'\nit('t', async () => { await screen.findByRole('alert') })\n",
    );
    assert_eq!(queries.len(), 1);
    assert_eq!(queries[0].name, "findByRole");
}

#[test]
fn the_all_by_families_throw_and_are_recorded() {
    let get_all = queries_of(
        "import { it } from 'vitest'\nimport { screen } from '@testing-library/react'\nit('t', () => { screen.getAllByRole('row') })\n",
    );
    assert_eq!(get_all.len(), 1);
    assert_eq!(get_all[0].name, "getAllByRole");

    let find_all = queries_of(
        "import { it } from 'vitest'\nimport { screen } from '@testing-library/react'\nit('t', async () => { await screen.findAllByText('x') })\n",
    );
    assert_eq!(find_all.len(), 1);
    assert_eq!(find_all[0].name, "findAllByText");
}

#[test]
fn a_query_by_family_does_not_throw_and_is_not_recorded() {
    // `queryBy*` / `queryAllBy*` return null (or []) rather than throwing, so a test built on
    // them is NOT anchored — it must not be recorded as a throwing query.
    let query_by = queries_of(
        "import { it } from 'vitest'\nimport { screen } from '@testing-library/react'\nit('t', () => { screen.queryByText('gone') })\n",
    );
    assert!(
        query_by.is_empty(),
        "queryByText does not throw: {query_by:?}"
    );

    let query_all = queries_of(
        "import { it } from 'vitest'\nimport { screen } from '@testing-library/react'\nit('t', () => { screen.queryAllByText('gone') })\n",
    );
    assert!(
        query_all.is_empty(),
        "queryAllByText does not throw: {query_all:?}"
    );
}

#[test]
fn a_query_inside_wait_for_is_collected() {
    // The collector descends the whole callback subtree, so a query in a `waitFor(…)` closure
    // is the case's anchor.
    let queries = queries_of(
        "import { it } from 'vitest'\nimport { screen, waitFor } from '@testing-library/react'\nit('t', async () => { await waitFor(() => screen.findByText('done')) })\n",
    );
    assert_eq!(queries.len(), 1);
    assert_eq!(queries[0].name, "findByText");
}

#[test]
fn a_test_with_no_query_is_representable() {
    let queries = queries_of("import { it } from 'vitest'\nit('t', () => { doSomething() })\n");
    assert!(queries.is_empty(), "zero throwing queries, not an error");
}

#[test]
fn a_plain_getter_that_is_not_a_testing_library_query_is_not_recorded() {
    // `get*` without the `By` infix (a getter, a lodash `get`) is not a Testing Library query.
    let queries =
        queries_of("import { it } from 'vitest'\nit('t', () => { getUser(1); get(obj, 'a') })\n");
    assert!(
        queries.is_empty(),
        "getUser/get are not queries: {queries:?}"
    );
}
