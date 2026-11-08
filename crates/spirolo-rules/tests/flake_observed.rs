//! `flake-observed` on hand-built models: one case per outcome combination PRD 0004 names — a
//! pass and a fail on one tree, a pass on retry (both sides at once), a pass and a fail on two
//! different trees, a test red in every run, a red beside a skip — and where an unjoined row's
//! finding is anchored.
//!
//! The rule is a pure function of the `Run` table, so these tests build the rows by hand. The
//! binary fixture (Seam 2) proves the same rule end to end from real vitest reports.

use spirolo_model::{JoinedCase, Model, Run, RunOutcome, Span};
use spirolo_rules::{Config, Finding, FlakeObserved, Rule};

/// A row for `ledger > flips` in `src/a.test.ts`, joined to a test at line 5, in run `run_id`
/// on `tree`.
fn row(run_id: &str, outcome: RunOutcome, tree: &str) -> Run {
    let title_path = vec!["ledger".to_owned(), "flips".to_owned()];
    Run {
        file: "src/a.test.ts".to_owned(),
        title: title_path.join(" > "),
        run_id: run_id.to_owned(),
        attempt: 0,
        title_path,
        outcome,
        duration_ms: None,
        retries: None,
        tree_hash: Some(tree.to_owned()),
        started_at_ms: None,
        condition: Some("idle".to_owned()),
        report: format!("reports/{run_id}.json"),
        runner_home: None,
        joined: Some(JoinedCase {
            file: "src/a.test.ts".to_owned(),
            span: Span {
                line: 5,
                column: 3,
                ..Span::default()
            },
        }),
    }
}

fn findings(runs: Vec<Run>) -> Vec<Finding> {
    let model = Model {
        runs,
        ..Model::default()
    };
    FlakeObserved.run(&model, &Config::default())
}

/// A finding's evidence value under `key`.
fn evidence<'a>(finding: &'a Finding, key: &str) -> &'a str {
    &finding
        .evidence
        .iter()
        .find(|e| e.key == key)
        .unwrap()
        .value
}

#[test]
fn a_pass_and_a_fail_on_one_tree_is_a_flake_at_the_test() {
    let found = findings(vec![
        row("r1", RunOutcome::Passed, "t1"),
        row("r2", RunOutcome::Failed, "t1"),
    ]);

    assert_eq!(found.len(), 1);
    assert_eq!((found[0].location.line, found[0].location.column), (5, 3));
    assert_eq!(evidence(&found[0], "passed_runs"), "r1");
    assert_eq!(evidence(&found[0], "failed_runs"), "r2");
}

#[test]
fn a_pass_on_retry_counts_as_both_a_pass_and_a_fail() {
    let found = findings(vec![row("r1", RunOutcome::PassedOnRetry, "t1")]);

    assert_eq!(found.len(), 1);
    assert_eq!(evidence(&found[0], "passed"), "1");
    assert_eq!(evidence(&found[0], "failed"), "1");
}

#[test]
fn a_pass_and_a_fail_on_different_trees_is_not_a_flake() {
    let found = findings(vec![
        row("r1", RunOutcome::Passed, "t1"),
        row("r2", RunOutcome::Failed, "t2"),
    ]);

    assert!(found.is_empty());
}

#[test]
fn a_test_red_in_every_run_or_red_beside_a_skip_is_not_a_flake() {
    let found = findings(vec![
        row("r1", RunOutcome::Failed, "t1"),
        row("r2", RunOutcome::Failed, "t1"),
        row("r3", RunOutcome::Skipped, "t1"),
    ]);

    assert!(found.is_empty());
}

#[test]
fn an_unjoined_flake_is_anchored_at_its_file_and_keeps_its_title_path() {
    let unjoined = |run_id, outcome| Run {
        joined: None,
        ..row(run_id, outcome, "t1")
    };
    let found = findings(vec![
        unjoined("r1", RunOutcome::Passed),
        unjoined("r2", RunOutcome::Failed),
    ]);

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].location.file, "src/a.test.ts");
    assert_eq!((found[0].location.line, found[0].location.column), (1, 1));
    let test = found[0].test.as_ref().unwrap();
    assert_eq!(test.suite_path, ["ledger"]);
    assert_eq!(test.title.as_deref(), Some("flips"));
}

/// A file-level row: `src/a.test.ts` failed outside any test in run `run_id` on `tree` (a
/// hook threw or the module did not load), as the vitest reader records it.
fn file_failed(run_id: &str, tree: &str) -> Run {
    Run {
        title: String::new(),
        title_path: Vec::new(),
        joined: None,
        condition: Some("load".to_owned()),
        ..row(run_id, RunOutcome::Failed, tree)
    }
}

#[test]
fn a_run_whose_file_failed_outside_any_test_counts_a_fail_for_each_test_it_did_not_report() {
    let found = findings(vec![
        row("r1", RunOutcome::Passed, "t1"),
        file_failed("r2", "t1"),
    ]);

    assert_eq!(found.len(), 1);
    assert_eq!((found[0].location.line, found[0].location.column), (5, 3));
    assert_eq!(evidence(&found[0], "failed_runs"), "r2");
    assert_eq!(evidence(&found[0], "failed_conditions"), "load");
}

#[test]
fn a_file_failure_is_no_fail_for_a_test_its_run_reported_nor_on_another_tree_nor_a_finding_alone() {
    // An `afterAll` that threw after the test passed: the run's own row stands.
    assert_eq!(
        findings(vec![
            row("r1", RunOutcome::Passed, "t1"),
            row("r2", RunOutcome::Passed, "t1"),
            file_failed("r2", "t1"),
        ]),
        Vec::new()
    );
    assert_eq!(
        findings(vec![
            row("r1", RunOutcome::Passed, "t1"),
            file_failed("r2", "t2")
        ]),
        Vec::new()
    );
    assert_eq!(findings(vec![file_failed("r1", "t1")]), Vec::new());
}
