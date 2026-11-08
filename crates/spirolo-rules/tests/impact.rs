//! `impact`'s selection and fixed ranking on hand-built models: one case per ranking key
//! this slice owns (a changed test file, `paired`, import distance, path), the changed-path
//! diagnostics, `unreached`, and the mutation-blindness guard.

use spirolo_model::{
    Import, ImportKind, Language, Model, Module, Pairing, PairingReason, ParseStatus, Run,
    RunOutcome, Span, TestCase, TestKind,
};
use spirolo_rules::{impact, Selected};

/// A module at `path` whose imports resolve to each of `targets`.
fn module(path: &str, targets: &[&str]) -> Module {
    Module {
        path: path.to_owned(),
        language: Language::TypeScript,
        parse_status: ParseStatus::Parsed,
        imports: targets
            .iter()
            .map(|target| Import {
                specifier: format!("./{target}"),
                resolved: Some((*target).to_owned()),
                reason: None,
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

fn pairing(test_file: &str, source_module: &str) -> Pairing {
    Pairing {
        test_file: test_file.to_owned(),
        source_module: source_module.to_owned(),
        reason: PairingReason::SiblingName,
        ambiguous: false,
    }
}

fn model(modules: Vec<Module>, tests: &[&str], pairings: Vec<Pairing>) -> Model {
    Model {
        modules,
        test_cases: tests.iter().map(|file| test_case(file)).collect(),
        pairings,
        ..Model::default()
    }
}

/// One ingested row: a test in `file` that ended `outcome` in `run_id`, started at
/// `started_at_ms`, taking `duration_ms`.
fn run(
    file: &str,
    run_id: &str,
    started_at_ms: Option<u64>,
    outcome: RunOutcome,
    duration_ms: Option<u64>,
) -> Run {
    Run {
        file: file.to_owned(),
        title: "t".to_owned(),
        run_id: run_id.to_owned(),
        attempt: 0,
        title_path: vec!["t".to_owned()],
        outcome,
        duration_ms,
        retries: None,
        tree_hash: Some("tree".to_owned()),
        started_at_ms,
        condition: None,
        report: format!(".spirolo/runs/{run_id}.json"),
        runner_home: None,
        joined: None,
    }
}

/// Two tests that import `src/a.ts` directly: equal on every key before `last-failed`.
fn two_direct_importers() -> Model {
    model(
        vec![
            module("src/a.ts", &[]),
            module("src/m.test.ts", &["src/a.ts"]),
            module("src/z.test.ts", &["src/a.ts"]),
        ],
        &["src/m.test.ts", "src/z.test.ts"],
        Vec::new(),
    )
}

fn changed(paths: &[&str]) -> Vec<String> {
    paths.iter().map(|path| (*path).to_owned()).collect()
}

/// The selected files in rank order.
fn files(selected: &[Selected]) -> Vec<&str> {
    selected.iter().map(|s| s.file.as_str()).collect()
}

#[test]
fn a_test_importing_the_changed_file_is_selected_with_its_distance() {
    let model = model(
        vec![
            module("src/a.ts", &[]),
            module("src/x.test.ts", &["src/a.ts"]),
        ],
        &["src/x.test.ts"],
        Vec::new(),
    );

    let report = impact(&model, &changed(&["src/a.ts"]));

    assert_eq!(report.changed, changed(&["src/a.ts"]));
    assert_eq!(
        report.selected,
        vec![Selected {
            rank: 1,
            file: "src/x.test.ts".to_owned(),
            distance: Some(1),
            paired: false,
            last_failed: false,
            median_ms: None,
        }]
    );
    assert!(report.unreached.is_empty());
    assert!(report.diagnostics.is_empty());
}

#[test]
fn a_paired_test_ranks_ahead_of_a_closer_unpaired_importer() {
    let model = model(
        vec![
            module("src/a.ts", &[]),
            module("src/b.ts", &["src/a.ts"]),
            module("src/a.test.ts", &["src/b.ts"]),
            module("src/z.test.ts", &["src/a.ts"]),
        ],
        &["src/a.test.ts", "src/z.test.ts"],
        vec![pairing("src/a.test.ts", "src/a.ts")],
    );

    let report = impact(&model, &changed(&["src/a.ts"]));

    assert_eq!(files(&report.selected), ["src/a.test.ts", "src/z.test.ts"]);
    assert!(report.selected[0].paired);
    assert_eq!(report.selected[0].distance, Some(2));
    assert_eq!(report.selected[1].rank, 2);
}

#[test]
fn a_test_paired_with_the_changed_file_is_selected_without_importing_it() {
    let model = model(
        vec![module("src/a.ts", &[]), module("src/a.test.ts", &[])],
        &["src/a.test.ts"],
        vec![pairing("src/a.test.ts", "src/a.ts")],
    );

    let report = impact(&model, &changed(&["src/a.ts"]));

    assert_eq!(files(&report.selected), ["src/a.test.ts"]);
    assert_eq!(report.selected[0].distance, None);
    assert!(report.selected[0].paired);
}

#[test]
fn a_changed_test_file_ranks_first_ahead_of_a_paired_test() {
    let model = model(
        vec![
            module("src/a.ts", &[]),
            module("src/a.test.ts", &["src/a.ts"]),
            module("src/z.test.ts", &["src/a.ts"]),
        ],
        &["src/a.test.ts", "src/z.test.ts"],
        vec![pairing("src/a.test.ts", "src/a.ts")],
    );

    let report = impact(&model, &changed(&["src/a.ts", "src/z.test.ts"]));

    assert_eq!(files(&report.selected), ["src/z.test.ts", "src/a.test.ts"]);
    assert_eq!(report.selected[0].distance, Some(0));
}

#[test]
fn unpaired_tests_rank_by_import_distance_then_path() {
    let model = model(
        vec![
            module("src/a.ts", &[]),
            module("src/b.ts", &["src/a.ts"]),
            module("src/far.test.ts", &["src/b.ts"]),
            module("src/near-z.test.ts", &["src/a.ts"]),
            module("src/near-m.test.ts", &["src/a.ts"]),
        ],
        &[
            "src/far.test.ts",
            "src/near-z.test.ts",
            "src/near-m.test.ts",
        ],
        Vec::new(),
    );

    let report = impact(&model, &changed(&["src/a.ts"]));

    assert_eq!(
        files(&report.selected),
        [
            "src/near-m.test.ts",
            "src/near-z.test.ts",
            "src/far.test.ts"
        ]
    );
    let ranks: Vec<usize> = report.selected.iter().map(|s| s.rank).collect();
    assert_eq!(ranks, [1, 2, 3]);
}

#[test]
fn a_changed_file_no_test_reaches_is_listed_unreached() {
    let model = model(
        vec![
            module("src/a.ts", &[]),
            module("src/lonely.ts", &[]),
            module("src/uses-lonely.ts", &["src/lonely.ts"]),
            module("src/a.test.ts", &["src/a.ts"]),
        ],
        &["src/a.test.ts"],
        Vec::new(),
    );

    let report = impact(&model, &changed(&["src/lonely.ts", "src/a.ts"]));

    assert_eq!(files(&report.selected), ["src/a.test.ts"]);
    assert_eq!(report.unreached, changed(&["src/lonely.ts"]));
    assert_eq!(report.changed, changed(&["src/a.ts", "src/lonely.ts"]));
}

#[test]
fn changed_paths_are_normalized_to_repo_relative_before_selection() {
    let mut model = model(
        vec![
            module("src/a.ts", &[]),
            module("src/x.test.ts", &["src/a.ts"]),
        ],
        &["src/x.test.ts"],
        Vec::new(),
    );
    model.root = "/repo".into();

    let report = impact(
        &model,
        &changed(&["./src/a.ts", "/repo/src/a.ts", "src/lib/../a.ts"]),
    );

    assert_eq!(report.changed, changed(&["src/a.ts"]));
    assert_eq!(files(&report.selected), ["src/x.test.ts"]);
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
}

#[test]
fn a_path_outside_the_root_or_unknown_to_the_model_is_a_diagnostic_not_a_guess() {
    let mut model = model(
        vec![
            module("src/a.ts", &[]),
            module("src/x.test.ts", &["src/a.ts"]),
        ],
        &["src/x.test.ts"],
        Vec::new(),
    );
    model.root = "/repo".into();

    let report = impact(
        &model,
        &changed(&["../other/a.ts", "/elsewhere/a.ts", "README.md", "src/a.ts"]),
    );

    assert_eq!(report.changed, changed(&["src/a.ts"]));
    assert!(report.unreached.is_empty());
    let diagnostics: Vec<(&str, Option<&str>)> = report
        .diagnostics
        .iter()
        .map(|d| (d.code.as_str(), d.file.as_deref()))
        .collect();
    assert_eq!(
        diagnostics,
        [
            ("impact_path_outside_root", None),
            ("impact_path_outside_root", None),
            ("impact_path_unknown", Some("README.md")),
        ]
    );
    assert!(report.diagnostics[0].message.contains("../other/a.ts"));
    assert!(report.diagnostics[1].message.contains("/elsewhere/a.ts"));
}

#[test]
fn impact_is_blind_to_mutation_data() {
    let without = model(
        vec![
            module("src/a.ts", &[]),
            module("src/b.ts", &["src/a.ts"]),
            module("src/a.test.ts", &["src/b.ts"]),
            module("src/z.test.ts", &["src/a.ts"]),
        ],
        &["src/a.test.ts", "src/z.test.ts"],
        vec![pairing("src/a.test.ts", "src/a.ts")],
    );
    let mut with = without.clone();
    // A mutant in the changed file that only the lower-ranked test kills: were the ranking
    // reading it, `z.test.ts` would move up.
    with.mutants = vec![spirolo_model::Mutants {
        file: "src/a.ts".to_owned(),
        span: Span::default(),
        id: "1".to_owned(),
        mutator: Some("ConditionalExpression".to_owned()),
        status: spirolo_model::MutantStatus::Killed,
        killed_by: vec!["0".to_owned()],
        covered_by: vec!["0".to_owned()],
        report: "reports/mutation/mutation.json".to_owned(),
    }];
    with.mutation_tests = vec![spirolo_model::MutationTest {
        report: "reports/mutation/mutation.json".to_owned(),
        id: "0".to_owned(),
        file: "src/z.test.ts".to_owned(),
        name: "t".to_owned(),
        joined: None,
    }];

    let changed = changed(&["src/a.ts"]);
    assert_eq!(impact(&with, &changed), impact(&without, &changed));
}

#[test]
fn a_test_that_failed_in_the_most_recent_run_ranks_ahead_of_an_equally_close_one() {
    let mut model = two_direct_importers();
    model.runs = vec![
        run("src/m.test.ts", "r1", Some(1_000), RunOutcome::Passed, None),
        run("src/z.test.ts", "r1", Some(1_000), RunOutcome::Failed, None),
    ];

    let report = impact(&model, &changed(&["src/a.ts"]));

    assert_eq!(files(&report.selected), ["src/z.test.ts", "src/m.test.ts"]);
    assert!(report.selected[0].last_failed);
    assert!(!report.selected[1].last_failed);
}

#[test]
fn a_failure_in_an_older_run_is_not_last_failed() {
    let mut model = two_direct_importers();
    // Run ids sort the other way from start times: the latest run is `a-latest`.
    model.runs = vec![
        run(
            "src/m.test.ts",
            "a-latest",
            Some(2_000),
            RunOutcome::Passed,
            None,
        ),
        run(
            "src/m.test.ts",
            "b-older",
            Some(1_000),
            RunOutcome::Failed,
            None,
        ),
        run(
            "src/z.test.ts",
            "a-latest",
            Some(2_000),
            RunOutcome::Passed,
            None,
        ),
        run(
            "src/z.test.ts",
            "b-older",
            Some(1_000),
            RunOutcome::Passed,
            None,
        ),
    ];

    let report = impact(&model, &changed(&["src/a.ts"]));

    assert_eq!(files(&report.selected), ["src/m.test.ts", "src/z.test.ts"]);
    assert!(report.selected.iter().all(|selected| !selected.last_failed));
}

#[test]
fn the_faster_of_two_otherwise_equal_tests_ranks_first_and_unknown_duration_last() {
    let model = model(
        vec![
            module("src/a.ts", &[]),
            module("src/a1.test.ts", &["src/a.ts"]),
            module("src/b2.test.ts", &["src/a.ts"]),
            module("src/c3.test.ts", &["src/a.ts"]),
        ],
        &["src/a1.test.ts", "src/b2.test.ts", "src/c3.test.ts"],
        Vec::new(),
    );
    let mut model = model;
    model.runs = vec![
        run(
            "src/b2.test.ts",
            "r1",
            Some(1_000),
            RunOutcome::Passed,
            Some(40),
        ),
        run(
            "src/c3.test.ts",
            "r1",
            Some(1_000),
            RunOutcome::Passed,
            Some(900),
        ),
    ];

    let report = impact(&model, &changed(&["src/a.ts"]));

    assert_eq!(
        files(&report.selected),
        ["src/b2.test.ts", "src/c3.test.ts", "src/a1.test.ts"]
    );
    let medians: Vec<Option<u64>> = report.selected.iter().map(|s| s.median_ms).collect();
    assert_eq!(medians, [Some(40), Some(900), None]);
}

#[test]
fn keys_rank_paired_then_last_failed_then_distance_then_duration() {
    let mut model = model(
        vec![
            module("src/a.ts", &[]),
            module("src/b.ts", &["src/a.ts"]),
            module("src/far-red.test.ts", &["src/b.ts"]),
            module("src/near-slow.test.ts", &["src/a.ts"]),
            module("src/near-fast.test.ts", &["src/a.ts"]),
            module("src/paired.test.ts", &["src/b.ts"]),
        ],
        &[
            "src/far-red.test.ts",
            "src/near-slow.test.ts",
            "src/near-fast.test.ts",
            "src/paired.test.ts",
        ],
        vec![pairing("src/paired.test.ts", "src/a.ts")],
    );
    model.runs = vec![
        run(
            "src/far-red.test.ts",
            "r1",
            Some(1),
            RunOutcome::Failed,
            Some(5_000),
        ),
        run(
            "src/near-fast.test.ts",
            "r1",
            Some(1),
            RunOutcome::Passed,
            Some(1),
        ),
        run(
            "src/near-slow.test.ts",
            "r1",
            Some(1),
            RunOutcome::Passed,
            Some(800),
        ),
        run(
            "src/paired.test.ts",
            "r1",
            Some(1),
            RunOutcome::Passed,
            Some(9_000),
        ),
    ];

    let report = impact(&model, &changed(&["src/a.ts"]));

    assert_eq!(
        files(&report.selected),
        [
            "src/paired.test.ts",
            "src/far-red.test.ts",
            "src/near-fast.test.ts",
            "src/near-slow.test.ts",
        ]
    );
}

#[test]
fn median_duration_spans_every_ingested_run_and_leaves_skipped_rows_out() {
    let mut model = two_direct_importers();
    model.runs = vec![
        run("src/m.test.ts", "r1", Some(1), RunOutcome::Passed, Some(10)),
        run(
            "src/m.test.ts",
            "r2",
            Some(2),
            RunOutcome::Failed,
            Some(1_000),
        ),
        run("src/m.test.ts", "r3", Some(3), RunOutcome::Passed, Some(30)),
        run("src/m.test.ts", "r4", Some(4), RunOutcome::Skipped, Some(0)),
        run("src/z.test.ts", "r1", Some(1), RunOutcome::Passed, Some(20)),
        run("src/z.test.ts", "r2", Some(2), RunOutcome::Passed, Some(60)),
    ];

    let report = impact(&model, &changed(&["src/a.ts"]));

    let medians: Vec<(&str, Option<u64>)> = report
        .selected
        .iter()
        .map(|s| (s.file.as_str(), s.median_ms))
        .collect();
    assert_eq!(
        medians,
        [("src/m.test.ts", Some(30)), ("src/z.test.ts", Some(40))]
    );
}

#[test]
fn with_no_runs_ingested_both_keys_are_inert_and_the_order_falls_to_the_path() {
    let report = impact(&two_direct_importers(), &changed(&["src/a.ts"]));

    assert_eq!(files(&report.selected), ["src/m.test.ts", "src/z.test.ts"]);
    assert!(report
        .selected
        .iter()
        .all(|selected| !selected.last_failed && selected.median_ms.is_none()));
}

#[test]
fn a_test_absent_from_a_later_run_keeps_the_outcome_of_its_own_most_recent_run() {
    let mut model = two_direct_importers();
    // One report per package per run, as a monorepo runner writes them: the later report
    // (another package's) does not name `z.test.ts`, whose own latest run failed.
    model.runs = vec![
        run(
            "src/m.test.ts",
            "b-later",
            Some(2_000),
            RunOutcome::Passed,
            None,
        ),
        run(
            "src/z.test.ts",
            "a-earlier",
            Some(1_000),
            RunOutcome::Failed,
            None,
        ),
    ];

    let report = impact(&model, &changed(&["src/a.ts"]));

    assert_eq!(files(&report.selected), ["src/z.test.ts", "src/m.test.ts"]);
    assert!(report.selected[0].last_failed);
}
