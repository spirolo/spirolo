//! The run report readers: a vitest JSON or `JUnit` XML report in, the model's `Run` rows
//! and diagnostics out. One call reads one report, which is one run. Both readers make
//! absolute paths root-relative (or say why they cannot), join each row to the `TestCase`
//! with its file and title path, and keep a row that joins nothing.

use std::collections::HashSet;

use serde::Deserialize;
use spirolo_model::{Diagnostic, JoinedCase, Run, RunOutcome, Severity, TestCase, TestKind};

/// What the binary knows about a run that its report does not say.
#[derive(Debug, Clone, Default)]
pub struct RunContext {
    /// Stable runner/configuration identity; only outcomes in one home are comparable.
    pub runner_home: Option<String>,
    /// In-tree, slash-separated path of the report; every row is tagged with it.
    pub report: String,
    /// Absolute path of the project root, which absolute paths in the report are made
    /// relative to.
    pub root: String,
    /// The run's id.
    pub run_id: String,
    /// Which attempt of the run the report records, from 0.
    pub attempt: u32,
    /// The label the run is ingested under, for example `idle` or `load`.
    pub condition: Option<String>,
    /// Hash of the tree the run was made on.
    pub tree_hash: Option<String>,
}

/// The rows and diagnostics one run report yields, rows sorted.
#[derive(Debug, Default)]
pub struct RunTables {
    /// One row per test the report names, plus one file-level row (empty title path,
    /// failed) per file that failed outside any test.
    pub runs: Vec<Run>,
    /// What the reader could not decide or wants the reader to know.
    pub diagnostics: Vec<Diagnostic>,
}

/// Read one vitest JSON report (the Jest-compatible `testResults[].assertionResults[]`
/// shape vitest 2.1 writes) into `Run` rows.
///
/// A file vitest marks `failed` with a message of its own failed outside any test: a hook
/// threw or the module did not load. It gets a file-level row (empty title path, failed),
/// and the tests vitest reports as skipped in it read as failed, since they never got to run.
///
/// The bytes are never trusted to be well-formed: a report that does not parse yields a
/// single `run_report_unreadable` diagnostic and no rows.
#[must_use]
pub fn read_vitest_json(context: &RunContext, bytes: &[u8], test_cases: &[TestCase]) -> RunTables {
    let root: VitestRoot = match serde_json::from_slice(bytes) {
        Ok(root) => root,
        Err(error) => return unreadable(context, "vitest JSON", &error),
    };
    let mut runs = Vec::new();
    let mut diagnostics = Vec::new();
    for file_result in &root.test_results {
        let Some(file) = relative(&context.root, &file_result.name) else {
            diagnostics.push(outside_root(
                context,
                &file_result.name,
                file_result.assertion_results.len(),
            ));
            continue;
        };
        let failed_outside_tests = file_result.failed_outside_tests();
        if failed_outside_tests {
            // The file itself failed: one row with an empty title path records it, so a
            // file that failed at load, and so has no results, still leaves a trace.
            runs.push(Run {
                file: file.clone(),
                title: String::new(),
                run_id: context.run_id.clone(),
                attempt: context.attempt,
                title_path: Vec::new(),
                outcome: RunOutcome::Failed,
                duration_ms: None,
                retries: None,
                tree_hash: context.tree_hash.clone(),
                started_at_ms: root.start_time.map(whole_ms),
                condition: context.condition.clone(),
                report: context.report.clone(),
                runner_home: context.runner_home.clone(),
                joined: None,
            });
        }
        for result in &file_result.assertion_results {
            let mut outcome = vitest_outcome(result);
            if failed_outside_tests && outcome == RunOutcome::Skipped {
                outcome = RunOutcome::Failed;
            }
            let mut title_path = result.ancestor_titles.clone();
            title_path.push(result.title.clone());
            runs.push(Run {
                file: file.clone(),
                title: title_path.join(" > "),
                run_id: context.run_id.clone(),
                attempt: context.attempt,
                outcome,
                duration_ms: result.duration.map(whole_ms),
                retries: vitest_retries(result),
                tree_hash: context.tree_hash.clone(),
                started_at_ms: root.start_time.map(whole_ms),
                condition: context.condition.clone(),
                report: context.report.clone(),
                runner_home: context.runner_home.clone(),
                joined: join_case(&file, &title_path, test_cases),
                title_path,
            });
        }
    }
    runs.sort();
    RunTables { runs, diagnostics }
}

/// Read one `JUnit` XML report (`testsuite`/`testcase`, as vitest 2.1's `junit` reporter
/// writes it) into `Run` rows.
///
/// `classname` names the file and `name` the title path, its
/// parts separated by `" > "`; a `failure` or `error` child means failed, a `skipped`
/// child means skipped, and a retry child (`flakyFailure`, `rerunFailure` and their
/// `Error` twins) with no final failure means passed on retry.
/// Vitest 4's synthetic failed testcase named after the file/suite becomes a file-level
/// row with an empty title path; skipped sibling tests read as failed, as in the JSON reader.
#[must_use]
pub fn read_junit_xml(context: &RunContext, bytes: &[u8], test_cases: &[TestCase]) -> RunTables {
    let text = String::from_utf8_lossy(bytes);
    let document = match roxmltree::Document::parse(&text) {
        Ok(document) => document,
        Err(error) => return unreadable(context, "JUnit XML", &error),
    };
    let started_at_ms = document
        .descendants()
        .filter(|node| node.has_tag_name("testsuite"))
        .filter_map(|suite| suite.attribute("timestamp").and_then(utc_ms))
        .min();
    let failed_suites: HashSet<_> = document
        .descendants()
        .filter(|case| case.has_tag_name("testcase") && junit_file_failure(*case))
        .filter_map(|case| case.parent())
        .map(|suite| suite.id())
        .collect();
    let mut runs = Vec::new();
    let mut diagnostics = Vec::new();
    for case in document
        .descendants()
        .filter(|node| node.has_tag_name("testcase"))
    {
        let Some(classname) = case.attribute("classname") else {
            diagnostics.push(Diagnostic {
                code: "run_report_testcase_without_file".to_owned(),
                severity: Severity::Warning,
                file: Some(context.report.clone()),
                span: None,
                message: format!(
                    "{} has a testcase {:?} with no classname; it is not recorded",
                    context.report,
                    case.attribute("name").unwrap_or_default()
                ),
            });
            continue;
        };
        let Some(file) = relative(&context.root, classname) else {
            diagnostics.push(outside_root(context, classname, 1));
            continue;
        };
        let file_failure = junit_file_failure(case);
        let mut outcome = junit_outcome(case);
        if outcome == RunOutcome::Skipped
            && case
                .parent()
                .is_some_and(|suite| failed_suites.contains(&suite.id()))
        {
            outcome = RunOutcome::Failed;
        }
        let title_path: Vec<String> = if file_failure {
            Vec::new()
        } else {
            case.attribute("name")
                .unwrap_or_default()
                .split(" > ")
                .map(str::to_owned)
                .collect()
        };
        runs.push(Run {
            title: title_path.join(" > "),
            run_id: context.run_id.clone(),
            attempt: context.attempt,
            outcome,
            duration_ms: case
                .attribute("time")
                .and_then(|seconds| seconds.parse::<f64>().ok())
                .map(|seconds| whole_ms(seconds * 1000.0)),
            retries: junit_retries(case),
            tree_hash: context.tree_hash.clone(),
            started_at_ms,
            condition: context.condition.clone(),
            report: context.report.clone(),
            runner_home: context.runner_home.clone(),
            joined: if file_failure {
                None
            } else {
                join_case(&file, &title_path, test_cases)
            },
            title_path,
            file,
        });
    }
    runs.sort();
    RunTables { runs, diagnostics }
}

/// Vitest 4 records a failure outside tests as a synthetic testcase named after its file
/// and suite. It is a file failure, not a test whose title happens to be a path.
fn junit_file_failure(case: roxmltree::Node<'_, '_>) -> bool {
    let Some(file) = case.attribute("classname") else {
        return false;
    };
    case.attribute("name") == Some(file)
        && case.parent().is_some_and(|suite| {
            suite.has_tag_name("testsuite") && suite.attribute("name") == Some(file)
        })
        && junit_outcome(case) == RunOutcome::Failed
}

/// The children a `JUnit` writer (Maven Surefire's shape) records for an attempt that
/// failed before a rerun: without a final `failure` or `error`, the case passed on a retry.
const RETRY_CHILDREN: [&str; 4] = ["flakyFailure", "flakyError", "rerunFailure", "rerunError"];

/// How many of a `testcase`'s children carry one of `tags`.
fn children_count(case: roxmltree::Node<'_, '_>, tags: &[&str]) -> u32 {
    let n = case
        .children()
        .filter(|child| tags.iter().any(|tag| child.has_tag_name(*tag)))
        .count();
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// A `testcase`'s outcome from its children.
fn junit_outcome(case: roxmltree::Node<'_, '_>) -> RunOutcome {
    if children_count(case, &["failure", "error"]) > 0 {
        RunOutcome::Failed
    } else if children_count(case, &["skipped"]) > 0 {
        RunOutcome::Skipped
    } else if children_count(case, &RETRY_CHILDREN) > 0 {
        RunOutcome::PassedOnRetry
    } else {
        RunOutcome::Passed
    }
}

/// Retries a `testcase` took. Vitest writes one `failure` per failed attempt, so a failed
/// case took one retry less than its failures, plus any retry children; a pass on retry
/// took one per retry child; a plain pass shows none; a skip never ran.
fn junit_retries(case: roxmltree::Node<'_, '_>) -> Option<u32> {
    let retried = children_count(case, &RETRY_CHILDREN);
    match junit_outcome(case) {
        RunOutcome::Failed => Some(
            children_count(case, &["failure", "error"])
                .saturating_sub(1)
                .saturating_add(retried),
        ),
        RunOutcome::Skipped => None,
        RunOutcome::Passed | RunOutcome::PassedOnRetry => Some(retried),
    }
}

/// Milliseconds since the Unix epoch of a UTC timestamp as `JUnit` writers stamp a suite,
/// `YYYY-MM-DDTHH:MM:SS[.fff]Z`. A timestamp with no `Z` names no zone, so it is not read.
fn utc_ms(stamp: &str) -> Option<u64> {
    let stamp = stamp.strip_suffix('Z')?;
    let (date, time) = stamp.split_once('T')?;
    let mut date = date.splitn(3, '-').map(str::parse::<i64>);
    let (year, month, day) = (date.next()?.ok()?, date.next()?.ok()?, date.next()?.ok()?);
    let (clock, fraction) = time.split_once('.').unwrap_or((time, ""));
    let mut clock = clock.splitn(3, ':').map(str::parse::<i64>);
    let (hour, minute, second) = (
        clock.next()?.ok()?,
        clock.next()?.ok()?,
        clock.next()?.ok()?,
    );
    let millis = if fraction.is_empty() {
        0
    } else {
        format!("{fraction:0<3}").get(..3)?.parse::<i64>().ok()?
    };
    let seconds = days_from_civil(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second;
    u64::try_from(seconds * 1000 + millis).ok()
}

/// Days from 1970-01-01 to a proleptic Gregorian date (Howard Hinnant's `days_from_civil`).
const fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The whole result for a report the reader cannot parse: one diagnostic, no rows.
fn unreadable(context: &RunContext, format: &str, error: &dyn std::fmt::Display) -> RunTables {
    let path = &context.report;
    RunTables {
        runs: Vec::new(),
        diagnostics: vec![Diagnostic {
            code: "run_report_unreadable".to_owned(),
            severity: Severity::Error,
            file: Some(path.clone()),
            span: None,
            message: format!("{path} is not a readable {format} report: {error}"),
        }],
    }
}

/// A result's outcome. Vitest keeps the messages of failed attempts on a result that
/// passed on retry, so a `passed` result with any message passed only on retry.
fn vitest_outcome(result: &AssertionResult) -> RunOutcome {
    match result.status.as_str() {
        "passed" if result.failure_messages.is_empty() => RunOutcome::Passed,
        "passed" => RunOutcome::PassedOnRetry,
        "failed" => RunOutcome::Failed,
        _ => RunOutcome::Skipped,
    }
}

/// Join a row to the model's `TestCase` with the same file and the same title path: its
/// `describe` titles, then its own title, compared part by part. Only kind-test cases with
/// a literal title are candidates; a row the model cannot see stays unjoined, not guessed.
///
/// `test_cases` is sorted by file, as the model keeps its `TestCase` table, so the file's
/// cases are found by binary search: a ledger of thousands of runs joins in one pass over its
/// rows, not one pass over every test per row.
pub fn join_case(file: &str, title_path: &[String], test_cases: &[TestCase]) -> Option<JoinedCase> {
    let start = test_cases.partition_point(|case| case.file.as_str() < file);
    let end = start + test_cases[start..].partition_point(|case| case.file == file);
    test_cases[start..end]
        .iter()
        .filter(|case| case.kind == TestKind::Test)
        .find(|case| {
            case.title.as_ref().is_some_and(|title| {
                title_path.split_last().is_some_and(|(last, suites)| {
                    last == title && suites == case.suite_path.as_slice()
                })
            })
        })
        .map(|case| JoinedCase {
            file: case.file.clone(),
            span: case.span,
        })
}

/// Retries a result took. Vitest keeps one message per failed attempt: a pass after `n`
/// messages took `n` retries, a fail after `n` messages took `n - 1`. A skipped result
/// never ran, so it has none to count.
fn vitest_retries(result: &AssertionResult) -> Option<u32> {
    let messages = u32::try_from(result.failure_messages.len()).unwrap_or(u32::MAX);
    match result.status.as_str() {
        "passed" => Some(messages),
        "failed" => Some(messages.saturating_sub(1)),
        _ => None,
    }
}

/// Milliseconds as a report writes them (a float) to whole milliseconds, rounded; a
/// negative or non-finite value reads as 0.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn whole_ms(ms: f64) -> u64 {
    if ms.is_finite() && ms > 0.0 {
        ms.round() as u64
    } else {
        0
    }
}

/// A report's file path as a root-relative one. A relative path is taken as relative to
/// the root already; an absolute one must lie under `root`, or it has no in-tree name.
fn relative(root: &str, path: &str) -> Option<String> {
    if !path.starts_with('/') {
        return Some(path.to_owned());
    }
    path.strip_prefix(root.trim_end_matches('/'))
        .and_then(|rest| rest.strip_prefix('/'))
        .map(str::to_owned)
}

/// The diagnostic for a report file whose absolute path is not under the root: its
/// `results` rows are dropped rather than attributed to a guessed path.
fn outside_root(context: &RunContext, path: &str, results: usize) -> Diagnostic {
    Diagnostic {
        code: "run_report_path_outside_root".to_owned(),
        severity: Severity::Warning,
        file: Some(context.report.clone()),
        span: None,
        message: format!(
            "{} names {path}, which is not under {}; its {results} results are not recorded",
            context.report, context.root
        ),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VitestRoot {
    #[serde(default)]
    start_time: Option<f64>,
    #[serde(default)]
    test_results: Vec<FileResult>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileResult {
    name: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    message: String,
    #[serde(default)]
    assertion_results: Vec<AssertionResult>,
}

impl FileResult {
    /// Whether the file failed outside any test body: vitest marks the file `failed` and
    /// gives it its own message when a hook threw or the module failed to load. A file that
    /// failed only through its tests carries an empty message.
    fn failed_outside_tests(&self) -> bool {
        self.status == "failed" && !self.message.is_empty()
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssertionResult {
    #[serde(default)]
    ancestor_titles: Vec<String>,
    title: String,
    status: String,
    #[serde(default)]
    duration: Option<f64>,
    #[serde(default)]
    failure_messages: Vec<String>,
}
