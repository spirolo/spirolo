//! The reverse import closure `impact` selects from: for a set of changed modules, every
//! test file whose resolved imports reach one, at the shortest hop count. Built on
//! hand-made models, so each case names exactly the import edges it is about.

use std::collections::BTreeMap;

use spirolo_model::{
    Import, ImportKind, Language, Model, Module, ParseStatus, Span, TestCase, TestKind,
};

/// A module at `path` whose imports resolve to each of `targets`; `None` is an import that
/// did not resolve (a missing relative file).
fn module(path: &str, targets: &[Option<&str>]) -> Module {
    Module {
        path: path.to_owned(),
        language: Language::TypeScript,
        parse_status: ParseStatus::Parsed,
        imports: targets
            .iter()
            .enumerate()
            .map(|(index, target)| Import {
                specifier: format!("./dep{index}"),
                resolved: target.map(str::to_owned),
                reason: target.is_none().then(|| "missing".to_owned()),
                kind: ImportKind::Static,
                span: Span::default(),
            })
            .collect(),
        exports: Vec::new(),
        constants: Vec::new(),
        mocks: Vec::new(),
        references: Vec::new(),
        spies: Vec::new(),
        loads: Vec::new(),
        complexity: 1,
        generated: false,
    }
}

/// A collected vitest test in `file`, which makes `file` a test file.
fn test_case(file: &str) -> TestCase {
    TestCase {
        file: file.to_owned(),
        span: Span::default(),
        framework: Some("vitest".to_owned()),
        title: Some("t".to_owned()),
        kind: TestKind::Test,
        suite_path: Vec::new(),
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

fn model(modules: Vec<Module>, tests: &[&str]) -> Model {
    Model {
        modules,
        test_cases: tests.iter().map(|file| test_case(file)).collect(),
        ..Model::default()
    }
}

fn distances(pairs: &[(&str, u32)]) -> BTreeMap<String, u32> {
    pairs
        .iter()
        .map(|(file, distance)| ((*file).to_owned(), *distance))
        .collect()
}

#[test]
fn a_test_importing_a_changed_module_is_one_hop_away() {
    let model = model(
        vec![
            module("src/a.ts", &[]),
            module("src/a.test.ts", &[Some("src/a.ts")]),
        ],
        &["src/a.test.ts"],
    );

    assert_eq!(
        model.test_distances(&["src/a.ts"]),
        distances(&[("src/a.test.ts", 1)])
    );
}

#[test]
fn a_test_two_hops_away_through_a_non_test_module_is_at_distance_two_and_the_module_is_not_listed()
{
    let model = model(
        vec![
            module("src/a.ts", &[]),
            module("src/b.ts", &[Some("src/a.ts")]),
            module("src/b.test.ts", &[Some("src/b.ts")]),
        ],
        &["src/b.test.ts"],
    );

    assert_eq!(
        model.test_distances(&["src/a.ts"]),
        distances(&[("src/b.test.ts", 2)])
    );
}

#[test]
fn the_shortest_path_wins_when_a_test_reaches_the_change_two_ways() {
    let model = model(
        vec![
            module("src/a.ts", &[]),
            module("src/b.ts", &[Some("src/a.ts")]),
            module("src/c.ts", &[Some("src/b.ts")]),
            module("src/x.test.ts", &[Some("src/c.ts"), Some("src/b.ts")]),
        ],
        &["src/x.test.ts"],
    );

    assert_eq!(
        model.test_distances(&["src/a.ts"]),
        distances(&[("src/x.test.ts", 2)])
    );
}

#[test]
fn with_two_changed_modules_a_test_is_as_far_as_the_nearest() {
    let model = model(
        vec![
            module("src/a.ts", &[]),
            module("src/b.ts", &[Some("src/a.ts")]),
            module("src/z.ts", &[]),
            module("src/x.test.ts", &[Some("src/b.ts"), Some("src/z.ts")]),
        ],
        &["src/x.test.ts"],
    );

    assert_eq!(
        model.test_distances(&["src/a.ts", "src/z.ts"]),
        distances(&[("src/x.test.ts", 1)])
    );
}

#[test]
fn a_changed_test_file_is_at_distance_zero() {
    let model = model(
        vec![
            module("src/a.ts", &[]),
            module("src/a.test.ts", &[Some("src/a.ts")]),
        ],
        &["src/a.test.ts"],
    );

    assert_eq!(
        model.test_distances(&["src/a.test.ts"]),
        distances(&[("src/a.test.ts", 0)])
    );
}

#[test]
fn an_unresolved_import_is_not_followed() {
    let model = model(
        vec![module("src/a.ts", &[]), module("src/a.test.ts", &[None])],
        &["src/a.test.ts"],
    );

    assert!(model.test_distances(&["src/a.ts"]).is_empty());
}
