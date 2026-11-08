//! The harness end to end on a synthetic corpus: a canned `spirolo list` document stands in
//! for a real run, so the diff, the verdict and the printed table are exercised without
//! cloning a corpus repo. The real binary is driven against the real corpus by `just bench`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{json, Value};
use spirolo_bench::{any_failed, render, run_corpus, Run, Status};

/// A fresh, empty corpus directory named after the calling test.
fn corpus(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Write one corpus entry: its `truth.json`, and a `repo/` directory when `cloned`.
fn entry(corpus: &Path, name: &str, truth: &str, cloned: bool) {
    let dir = corpus.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("truth.json"), truth).unwrap();
    if cloned {
        std::fs::create_dir_all(dir.join("repo")).unwrap();
    }
}

/// An `spirolo list --format json` document carrying `count` vitest tests across two files.
fn vitest_document(count: u64) -> Value {
    let cases: Vec<Value> = (0..count)
        .map(|index| {
            json!({
                "kind": "test",
                "framework": "vitest",
                "file": if index % 2 == 0 { "a.test.ts" } else { "b.test.ts" },
            })
        })
        .collect();
    json!({ "test_cases": cases })
}

/// A runner that returns `json` in a fixed time — deterministic, so the table snapshots.
fn canned(json: Value, elapsed_ms: u64) -> impl Fn(&Path) -> Result<Run, String> {
    sampled(json, &[elapsed_ms], Some(0.05))
}

/// A runner that returns `json` with these wall-clock samples, after an idle-box check that
/// read the box `busy` (a fraction of all CPUs), or could not read it (`None`).
fn sampled(
    json: Value,
    samples_ms: &[u64],
    busy: Option<f64>,
) -> impl Fn(&Path) -> Result<Run, String> {
    let samples: Vec<Duration> = samples_ms
        .iter()
        .copied()
        .map(Duration::from_millis)
        .collect();
    move |_repo: &Path| {
        Ok(Run {
            json: json.clone(),
            samples: samples.clone(),
            busy,
        })
    }
}

/// A large monorepo's five warm `spirolo list` runs, re-measured on 2026-09-23:
/// a single sample read the first, 6.02 s; the median is 4.00 s.
const LAB_WARM_RUNS_MS: [u64; 5] = [6020, 4080, 3900, 4000, 4000];

const LOCAL_TWO_FILES_THREE_TESTS: &str = r#"{
  "access": "local", "budget_ms": 5000,
  "counts": { "vitest": { "files": 2, "tests": 3 } }
}"#;

#[test]
fn a_pending_entry_is_skipped_not_failed() {
    let corpus = corpus("pending");
    entry(
        &corpus,
        "example-console",
        r#"{ "access": "pending", "budget_ms": 2000,
            "counts": { "vitest": { "files": 118, "tests": 97 } } }"#,
        false,
    );

    // The runner must never see a pending entry.
    let reports = run_corpus(&corpus, &|_repo| panic!("pending entry was run")).unwrap();

    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].status, Status::Skipped);
    assert!(!any_failed(&reports));
    insta::assert_snapshot!(render(&reports));
}

#[test]
fn a_local_entry_whose_counts_match_within_budget_passes() {
    let corpus = corpus("match");
    entry(&corpus, "fixture-repo", LOCAL_TWO_FILES_THREE_TESTS, true);

    let reports = run_corpus(&corpus, &canned(vitest_document(3), 1200)).unwrap();

    assert_eq!(reports[0].status, Status::Pass);
    assert!(!any_failed(&reports));
    insta::assert_snapshot!(render(&reports));
}

#[test]
fn a_count_delta_fails_and_the_table_shows_the_mismatch() {
    let corpus = corpus("delta");
    entry(&corpus, "fixture-repo", LOCAL_TWO_FILES_THREE_TESTS, true);

    // The model found only two of the three tests.
    let reports = run_corpus(&corpus, &canned(vitest_document(2), 1200)).unwrap();

    assert_eq!(reports[0].status, Status::Fail);
    assert!(any_failed(&reports));
    insta::assert_snapshot!(render(&reports));
}

#[test]
fn a_run_over_budget_fails_even_when_counts_match() {
    let corpus = corpus("slow");
    entry(&corpus, "fixture-repo", LOCAL_TWO_FILES_THREE_TESTS, true);

    let reports = run_corpus(&corpus, &canned(vitest_document(3), 9000)).unwrap();

    assert_eq!(reports[0].status, Status::Fail);
    assert!(!reports[0].timing.unwrap().within);
    insta::assert_snapshot!(render(&reports));
}

/// The budget reads the median of the samples, not one wall-clock: the same five real runs
/// pass a 5 s budget a single first sample (6.02 s) would have failed, and fail a 3.30 s
/// budget (the recorded M3 ceiling) their fastest run (3.90 s) could not have passed either.
#[test]
fn the_budget_reads_the_median_of_the_samples() {
    let corpus = corpus("median");
    entry(&corpus, "fixture-repo", LOCAL_TWO_FILES_THREE_TESTS, true);
    let run = sampled(vitest_document(3), &LAB_WARM_RUNS_MS, Some(0.05));

    let reports = run_corpus(&corpus, &run).unwrap();
    let timing = reports[0].timing.unwrap();
    assert_eq!(timing.elapsed_ms, 4000, "the median of five");
    assert!(timing.within);
    assert_eq!(reports[0].status, Status::Pass);
    insta::assert_snapshot!(render(&reports));

    let tight = r#"{ "access": "local", "budget_ms": 5000, "m2_list_ms": 2300,
        "counts": { "vitest": { "files": 2, "tests": 3 } } }"#;
    entry(&corpus, "fixture-repo", tight, true);
    let reports = run_corpus(&corpus, &run).unwrap();
    assert!(
        !reports[0].timing.unwrap().within,
        "4.00 s median over 3.30 s"
    );
    assert_eq!(reports[0].status, Status::Fail);
}

/// A timing taken on a busy box is not a measurement: the review's re-run saw load ~6.7 on
/// this 16-CPU box (about 42 % busy), so the same five runs that pass on an idle box are
/// refused, the entry fails and the line says why, rather than passing or failing the budget
/// on another job's load.
#[test]
fn a_timing_taken_on_a_busy_box_is_refused_not_judged() {
    let corpus = corpus("busy");
    entry(&corpus, "fixture-repo", LOCAL_TWO_FILES_THREE_TESTS, true);

    let run = sampled(vitest_document(3), &LAB_WARM_RUNS_MS, Some(0.42));
    let reports = run_corpus(&corpus, &run).unwrap();

    assert!(!reports[0].timing.unwrap().within);
    assert_eq!(reports[0].status, Status::Fail);
    insta::assert_snapshot!(render(&reports));
}

#[test]
fn a_local_entry_that_is_not_cloned_is_skipped() {
    let corpus = corpus("uncloned");
    entry(&corpus, "fixture-repo", LOCAL_TWO_FILES_THREE_TESTS, false);

    let reports = run_corpus(&corpus, &|_repo| panic!("un-cloned entry was run")).unwrap();

    assert_eq!(reports[0].status, Status::Skipped);
    assert!(!any_failed(&reports));
    insta::assert_snapshot!(render(&reports));
}

#[test]
fn a_mixed_corpus_reports_each_entry_in_name_order() {
    let corpus = corpus("mixed");
    // A local entry the model under-counts (a real monorepo's shape), plus a pending one.
    entry(
        &corpus,
        "big-local",
        r#"{ "access": "local", "budget_ms": 5000,
            "counts": { "vitest": { "files": 2, "tests": 3 },
                        "playwright": { "files": 1, "tests": 4 } } }"#,
        true,
    );
    entry(
        &corpus,
        "pending-client",
        r#"{ "access": "pending", "budget_ms": 2000,
            "counts": { "vitest": { "files": 118, "tests": 97 } } }"#,
        false,
    );

    let reports = run_corpus(&corpus, &canned(vitest_document(3), 1040)).unwrap();

    assert_eq!(reports.len(), 2);
    assert!(any_failed(&reports)); // playwright missing → the local entry fails
    insta::assert_snapshot!(render(&reports));
}

/// An entry with a `runner` record prints a second line under the framework: the runner's
/// listing (net of its load-time file) against spirolo's tally with the runner's own
/// exclusions, within the stated tolerance. The runner crashed on `apps/crasher`, expanded
/// a `.each` in `a.test.ts` to 3 tests where spirolo folds it to 2, and listed 3 generated
/// tests in `e2e/gen.test.ts`: 7 tests in 3 files, compared as 4 in 2 once the generated
/// file is off. Spirolo tallies 3 in 2 on that set; the one-test gap is the tolerance.
#[test]
fn a_runner_row_prints_under_its_framework_and_matches_within_tolerance() {
    let corpus = corpus("runner");
    entry(
        &corpus,
        "fixture-repo",
        r#"{ "access": "local", "budget_ms": 5000,
            "counts": { "vitest": { "files": 4, "tests": 6 } },
            "runner": { "vitest": { "files": 3, "tests": 7, "tolerance_tests": 1,
                                    "excluded_packages": ["apps/crasher"],
                                    "excluded_files": [ { "path": "e2e/gen.test.ts", "listed_tests": 3 } ] } } }"#,
        true,
    );
    let document = json!({ "test_cases": [
        { "kind": "test", "framework": "vitest", "file": "a.test.ts" },
        { "kind": "test", "framework": "vitest", "file": "a.test.ts" },
        { "kind": "test", "framework": "vitest", "file": "b.test.ts" },
        { "kind": "test", "framework": "vitest", "file": "e2e/gen.test.ts" },
        { "kind": "test", "framework": "vitest", "file": "apps/crasher/c.test.ts" },
        { "kind": "test", "framework": "vitest", "file": "apps/crasher/c.test.ts" }
    ]});

    let reports = run_corpus(&corpus, &canned(document, 1200)).unwrap();

    assert_eq!(reports[0].status, Status::Pass);
    assert!(!any_failed(&reports));
    let row = &reports[0].runner[0];
    assert_eq!((row.listed_files, row.listed_tests), (2, Some(4)));
    assert_eq!((row.actual_files, row.actual_tests), (2, 3));
    assert!(row.matched);
    insta::assert_snapshot!(render(&reports));
}

/// `jest --listTests` names files, not tests, so a Jest runner record carries
/// `"tests": null` and only its files are compared. The document is a real `spirolo list` of
/// ChatGPTNextWeb/NextChat at defdcdb5 (fields the tally reads, sliced): Jest lists 34 test
/// files there, and spirolo stamps 33 of them `vitest` and one `jest`, so the runner row
/// disagrees and fails the entry.
#[test]
fn a_jest_runner_record_without_a_test_count_compares_files_only() {
    let corpus = corpus("runner-jest-files-only");
    entry(
        &corpus,
        "ChatGPTNextWeb__NextChat",
        r#"{ "access": "local", "budget_ms": 60000,
            "runner": { "jest": { "files": 34, "tests": null } } }"#,
        true,
    );
    let text = include_str!("fixtures/nextchat-list-slice.json");
    let document: Value = serde_json::from_str(text).unwrap();

    let reports = run_corpus(&corpus, &canned(document, 1200)).unwrap();

    let row = &reports[0].runner[0];
    assert_eq!(row.framework, "jest");
    assert_eq!((row.listed_files, row.listed_tests), (34, None));
    assert_eq!(row.actual_files, 1);
    assert!(!row.matched);
    assert_eq!(reports[0].status, Status::Fail);
}

/// The substrate an installed fixture clone probes as: a `node_modules/` beside a
/// `pnpm-lock.yaml` holding the first line of a corpus entry's lockfile at its pin, whose git
/// blob id is b0a073a9.
const INSTALLED: &str = "installed pnpm-lock.yaml git:b0a073a9c139532d0cc2b381cad8a03e56276dea";

/// Decision 3 of the M3 reopen: an entry's counts are recorded on one substrate and compared
/// only on that substrate. The same clone with its dependencies installed passes; with no
/// `node_modules` it is another substrate, and the entry errors naming both rather than
/// reporting a count delta that is only the environment (one entry's vitest counted 18177 bare
/// against 18313 installed).
#[test]
fn an_entry_whose_clone_is_not_on_the_recorded_substrate_errors_naming_both() {
    let truth = json!({
        "access": "local", "budget_ms": 5000, "substrate": INSTALLED,
        "counts": { "vitest": { "files": 2, "tests": 3 } },
    })
    .to_string();

    let installed = corpus("substrate-installed");
    entry(&installed, "sample-monorepo", &truth, true);
    let repo = installed.join("sample-monorepo").join("repo");
    std::fs::create_dir_all(repo.join("node_modules")).unwrap();
    std::fs::write(repo.join("pnpm-lock.yaml"), "lockfileVersion: '9.0'\n").unwrap();
    let reports = run_corpus(&installed, &canned(vitest_document(3), 100)).unwrap();
    assert_eq!(reports[0].status, Status::Pass, "{:?}", reports[0].note);

    let bare = corpus("substrate-bare");
    entry(&bare, "sample-monorepo", &truth, true);
    let reports = run_corpus(&bare, &canned(vitest_document(3), 100)).unwrap();
    assert_eq!(reports[0].status, Status::Fail);
    assert_eq!(
        reports[0].note.as_deref(),
        Some(
            format!("substrate mismatch: truth.json records {INSTALLED}, the clone is bare")
                .as_str()
        )
    );
    assert!(any_failed(&reports));
}

/// Decision 3 of the M3 reopen, on the committed corpus: every local entry records the one
/// substrate it is measured on, the installed clone at its lockfile, and every adjudication
/// it carries names the substrate it was made on, so the join can refuse a row made on
/// another.
#[test]
fn every_local_corpus_entry_records_an_installed_substrate_and_stamps_its_adjudications() {
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus");
    let entries = spirolo_bench::discover(&corpus);
    assert!(!entries.is_empty());
    for (name, dir) in entries {
        let truth = spirolo_bench::Truth::read(&dir.join("truth.json")).unwrap();
        if truth.access != spirolo_bench::Access::Local {
            continue;
        }
        let substrate = truth.substrate.as_deref().unwrap_or("");
        assert!(
            substrate.starts_with("installed "),
            "{name}: substrate {substrate:?}"
        );
        let unstamped = truth
            .adjudications
            .iter()
            .filter(|a| a.substrate.is_none())
            .count();
        assert_eq!(
            unstamped, 0,
            "{name}: {unstamped} adjudications name no substrate"
        );
    }
}

/// On the committed corpus: the M3 budget is `m2_list_ms + 1000`, so the M2 figure
/// must be measured the way the bench now measures, a median of timed runs on an idle box.
/// One entry's `2300` was one wall-clock from the M2 close; the M2-close binary re-run
/// through this harness gives about 2.93 s. An entry with `m2_list_ms` names how it was
/// measured in `m2_list_method`, and that says median.
#[test]
fn every_recorded_m2_timing_says_it_is_a_median_of_timed_runs() {
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus");
    for (name, dir) in spirolo_bench::discover(&corpus) {
        let text = std::fs::read_to_string(dir.join("truth.json")).unwrap();
        let truth: Value = serde_json::from_str(&text).unwrap();
        if truth.get("m2_list_ms").is_none() {
            continue;
        }
        let method = truth["m2_list_method"].as_str().unwrap_or("");
        assert!(
            method.contains("median of"),
            "{name}: m2_list_method {method:?}"
        );
    }
}

/// Decision 2 of the M3 reopen (#127), on the committed corpus: a decided `redundant-test` row
/// counts only when the deletion oracle made it, deleting the members from a re-run over the
/// held-out mutant population or finding that the re-run does not reproduce the report. The
/// rows the simulated-deletion oracle made measure whether Stryker reproduces its own report,
/// not precision, so none may stay in `adjudications`. An `open` row counts for nothing.
#[test]
fn every_redundant_test_adjudication_comes_from_the_held_out_deletion_oracle() {
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus");
    for (name, dir) in spirolo_bench::discover(&corpus) {
        let truth = spirolo_bench::Truth::read(&dir.join("truth.json")).unwrap();
        let simulated: Vec<&str> = truth
            .adjudications
            .iter()
            .filter(|a| a.rule == "redundant-test" && a.verdict != spirolo_bench::Verdict::Open)
            .filter(|a| {
                !a.note.contains("held-out population")
                    && !a.note.contains("does not reproduce the report's kill set")
            })
            .map(|a| a.id.as_str())
            .collect();
        assert!(
            simulated.is_empty(),
            "{name}: {} redundant-test rows not made by the deletion oracle, first {:?}",
            simulated.len(),
            simulated.first()
        );
    }
}
