//! Turning one entry's truth and one run's counts into a verdict, and rendering the whole
//! corpus as the table `just bench` prints.
//!
//! A local entry passes when every framework's counts match truth exactly (M1 tolerates
//! no delta — a difference is a model bug for the responsible slice, never absorbed here)
//! and the run came in under budget. A pending or un-cloned entry is skipped, never
//! failed, so an unreachable repo cannot turn the harness red.
//!
//! A file `truth.json` lists under `excluded_files` — one whose tests the runner generates
//! at load time — is taken out of both sides: its recorded `listed_tests` and one file come
//! off the expected count, its rows never reach the actual count, and it is printed as an
//! `open` line under the entry, so what the model cannot decide stays in view.
//!
//! An entry whose `truth.json` carries a `runner` record prints a second row under that
//! framework: the runner's own listing at the pin (`vitest list --json`, `playwright test
//! --list`) against spirolo's tally on the runner's set, with the record's own exclusions.
//! Files must match within `tolerance_files` (zero by default: exactly) and tests within
//! `tolerance_tests`, the attributed static-vs-runtime gap the record states; a runner row
//! over tolerance fails the entry like a counts mismatch does. `counts` stays spirolo's
//! self-regression tally; the runner row is the M1 "runner's own counts" measured.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::time::Duration;

use crate::counts::FrameworkCount;
use crate::truth::Truth;

/// A corpus entry's outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Every compared count matched and the run was within budget.
    Pass,
    /// A count disagreed with truth, or the run ran over budget.
    Fail,
    /// The entry was not measured (pending, or its clone is absent).
    Skipped,
}

impl Status {
    /// The fixed-width tag printed in the table.
    const fn tag(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Fail => "FAIL",
            Self::Skipped => "SKIPPED",
        }
    }
}

/// One framework's expected-vs-actual line within an entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameworkRow {
    /// Framework name, for example `vitest`.
    pub framework: String,
    /// Files truth expects, or `None` when truth gave no number (not compared).
    pub expected_files: Option<u64>,
    /// Tests truth expects, or `None` when truth gave no number (not compared).
    pub expected_tests: Option<u64>,
    /// Files the run produced.
    pub actual_files: u64,
    /// Tests the run produced.
    pub actual_tests: u64,
    /// Whether every compared dimension matched.
    pub matched: bool,
}

/// One framework's runner-listing line within an entry: the runner's own count at the pin
/// against spirolo's tally on the runner's set, within a stated tolerance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunnerRow {
    /// Framework name, for example `vitest`.
    pub framework: String,
    /// Files the runner listed, net of its excluded files.
    pub listed_files: u64,
    /// Tests the runner listed, net of its excluded files' tests; `None` when the runner
    /// lists files only, and then tests are not compared.
    pub listed_tests: Option<u64>,
    /// Files spirolo tallied with the runner record's exclusions.
    pub actual_files: u64,
    /// Tests spirolo tallied with the runner record's exclusions.
    pub actual_tests: u64,
    /// How far the file counts may differ and still match.
    pub tolerance_files: u64,
    /// How far the test counts may differ and still match.
    pub tolerance_tests: u64,
    /// Whether both compared dimensions are within tolerance.
    pub matched: bool,
}

impl RunnerRow {
    /// `actual - listed` for files, signed.
    pub fn delta_files(&self) -> i128 {
        i128::from(self.actual_files) - i128::from(self.listed_files)
    }

    /// `actual - listed` for tests, signed; `None` when the runner listed no tests.
    pub fn delta_tests(&self) -> Option<i128> {
        self.listed_tests
            .map(|listed| i128::from(self.actual_tests) - i128::from(listed))
    }
}

/// The busiest the box may be for a wall-clock to count, as a fraction of all CPUs.
///
/// Read just before the timed runs. Above it another job's load, not spirolo, decides the
/// timing, so the budget is neither passed nor failed on it but refused.
pub const IDLE_BUSY_CEILING: f64 = 0.25;

/// How an entry's `spirolo list` wall-clock was sampled: the timed runs, and the idle-box check
/// taken just before them.
#[derive(Debug, Clone, PartialEq)]
pub struct Wallclock {
    /// Each timed run's wall-clock; the budget reads their median, never one sample on a
    /// shared box.
    pub samples: Vec<Duration>,
    /// The fraction of all CPUs busy just before the timed runs, `None` when unreadable.
    pub busy: Option<f64>,
}

impl From<Duration> for Wallclock {
    /// One timed run with no idle-box check.
    fn from(elapsed: Duration) -> Self {
        Self {
            samples: vec![elapsed],
            busy: None,
        }
    }
}

impl Wallclock {
    /// The median sample (the mean of the middle two for an even count), zero when empty.
    pub fn median(&self) -> Duration {
        let mut sorted = self.samples.clone();
        sorted.sort();
        let middle = sorted.len() / 2;
        match sorted.len() {
            0 => Duration::ZERO,
            n if n % 2 == 1 => sorted[middle],
            _ => (sorted[middle - 1] + sorted[middle]) / 2,
        }
    }
}

/// The wall-clock verdict for a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timing {
    /// The median of the timed `spirolo list` runs, in milliseconds.
    pub elapsed_ms: u64,
    /// How many timed runs the median is over.
    pub samples: usize,
    /// The percentage of all CPUs busy just before the runs when it exceeded
    /// [`IDLE_BUSY_CEILING`]: the timing was refused, `within` is false. `None` on an idle box
    /// or when the check could not be read.
    pub busy_percent: Option<u32>,
    /// The budget from `truth.json`, in milliseconds.
    pub budget_ms: u64,
    /// Whether the run came in within budget.
    pub within: bool,
    /// The `spirolo list` wall-clock recorded at the M2 close, when this entry carries one,
    /// so the entry line can state how much the run added over it.
    pub baseline_ms: Option<u64>,
}

/// A file left out of both sides because its tests are decided at load time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenFile {
    /// Repo-relative path.
    pub path: String,
    /// The framework it was subtracted from.
    pub framework: String,
    /// The runner's own count for it, subtracted from the expected side.
    pub listed_tests: u64,
    /// Why it cannot be counted statically — the first sentence of truth's reason.
    pub reason: String,
}

/// One corpus entry's full outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryReport {
    /// The entry name (its directory under `corpus/`).
    pub name: String,
    /// Pass, fail or skipped.
    pub status: Status,
    /// Per-framework detail, empty when skipped.
    pub rows: Vec<FrameworkRow>,
    /// Per-framework runner-listing rows, one per `runner` record; empty when the entry
    /// has none or was skipped.
    pub runner: Vec<RunnerRow>,
    /// Timing verdict, `None` when skipped.
    pub timing: Option<Timing>,
    /// A one-line aside printed under the entry: why it was skipped, or why the run
    /// failed before counts could be compared. `None` on a clean pass or count mismatch.
    pub note: Option<String>,
    /// Files taken out of both sides, printed as `open` lines.
    pub open: Vec<OpenFile>,
}

impl EntryReport {
    /// A skipped entry with a reason (pending, or clone absent).
    pub fn skipped(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: Status::Skipped,
            rows: Vec::new(),
            runner: Vec::new(),
            timing: None,
            note: Some(reason.into()),
            open: Vec::new(),
        }
    }

    /// A failed entry whose `spirolo list` run never produced a usable document.
    pub fn errored(name: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: Status::Fail,
            rows: Vec::new(),
            runner: Vec::new(),
            timing: None,
            note: Some(error.into()),
            open: Vec::new(),
        }
    }
}

/// The wall-clock verdict: the samples' median against the budget, unless the box was busy.
fn timing(truth: &Truth, wallclock: &Wallclock) -> Timing {
    let elapsed_ms = duration_ms(wallclock.median());
    // With a recorded M2 timing the M3 budget is one added second over it; otherwise the
    // flat ceiling from truth.json stands.
    let budget_ms = truth
        .m2_list_ms
        .map_or(truth.budget_ms, |baseline| baseline + 1000);
    let busy_percent = wallclock
        .busy
        .filter(|&busy| busy > IDLE_BUSY_CEILING)
        // A fraction in (0.25, 1.0]: the percentage is 26..=100, never negative or large.
        .map(|busy| {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let percent = (busy.min(1.0) * 100.0).round() as u32;
            percent
        });
    Timing {
        elapsed_ms,
        samples: wallclock.samples.len(),
        busy_percent,
        budget_ms,
        within: busy_percent.is_none() && elapsed_ms <= budget_ms,
        baseline_ms: truth.m2_list_ms,
    }
}

/// Judge a measured local entry against its truth.
///
/// The frameworks compared are the union of those truth records and those the run
/// produced, so a framework truth expects but the model missed (0 tests) and a framework
/// the model reported but truth never listed both surface as a mismatch.
///
/// `runner_actual` is spirolo's tally per `runner` record, each with that record's own
/// exclusions ([`crate::counts::by_runner_record`]); a record with no tally here reads as
/// zero files and zero tests, a mismatch. An entry without `runner` records gets no
/// runner rows and its output is unchanged.
pub fn evaluate(
    name: impl Into<String>,
    truth: &Truth,
    actual: &BTreeMap<String, FrameworkCount>,
    runner_actual: &BTreeMap<String, FrameworkCount>,
    wallclock: impl Into<Wallclock>,
) -> EntryReport {
    let wallclock = wallclock.into();
    let frameworks: BTreeSet<&str> = truth
        .counts
        .keys()
        .map(String::as_str)
        .chain(actual.keys().map(String::as_str))
        .collect();

    let mut rows = Vec::new();
    let mut all_matched = true;
    for framework in frameworks {
        let expected = truth.counts.get(framework);
        let got = actual.get(framework).copied().unwrap_or_default();
        // The runner listed the excluded files' tests, so they are in truth's numbers;
        // take them off the expected side the way their rows are off the actual side.
        let (open_files, open_tests) = truth
            .excluded_files
            .iter()
            .filter(|file| file.framework == framework)
            .fold((0, 0), |(files, tests), file| {
                (files + 1, tests + file.listed_tests)
            });
        let expected_files = expected
            .and_then(|counts| counts.files)
            .map(|files| files.saturating_sub(open_files));
        let expected_tests = expected
            .and_then(|counts| counts.tests)
            .map(|tests| tests.saturating_sub(open_tests));

        // A framework the model reports but truth never lists is itself a discrepancy;
        // otherwise only the dimensions truth actually gave a number for are compared.
        let mut matched = expected.is_some();
        if let Some(files) = expected_files {
            matched &= got.files == files;
        }
        if let Some(tests) = expected_tests {
            matched &= got.tests == tests;
        }
        all_matched &= matched;

        rows.push(FrameworkRow {
            framework: framework.to_owned(),
            expected_files,
            expected_tests,
            actual_files: got.files,
            actual_tests: got.tests,
            matched,
        });
    }

    let mut runner = Vec::new();
    for (framework, record) in &truth.runner {
        let (listed_files, listed_tests) = record.compared();
        let got = runner_actual.get(framework).copied().unwrap_or_default();
        let matched = got.files.abs_diff(listed_files) <= record.tolerance_files
            && listed_tests
                .is_none_or(|listed| got.tests.abs_diff(listed) <= record.tolerance_tests);
        all_matched &= matched;
        runner.push(RunnerRow {
            framework: framework.clone(),
            listed_files,
            listed_tests,
            actual_files: got.files,
            actual_tests: got.tests,
            tolerance_files: record.tolerance_files,
            tolerance_tests: record.tolerance_tests,
            matched,
        });
    }

    let timing = timing(truth, &wallclock);

    let status = if all_matched && timing.within {
        Status::Pass
    } else {
        Status::Fail
    };

    EntryReport {
        name: name.into(),
        status,
        rows,
        runner,
        timing: Some(timing),
        note: None,
        open: truth
            .excluded_files
            .iter()
            .map(|file| OpenFile {
                path: file.path.clone(),
                framework: file.framework.clone(),
                listed_tests: file.listed_tests,
                reason: first_sentence(&file.reason),
            })
            .collect(),
    }
}

/// The first sentence of a reason, so the open line stays one line.
fn first_sentence(reason: &str) -> String {
    let end = reason
        .find(". ")
        .or_else(|| reason.find(".\n"))
        .unwrap_or(reason.len());
    reason[..end].trim_end_matches('.').to_owned()
}

/// Whether any entry failed — the harness's exit is non-zero when this is true.
pub fn any_failed(reports: &[EntryReport]) -> bool {
    reports.iter().any(|report| report.status == Status::Fail)
}

/// Render the whole corpus as the table `just bench` prints, ending in a tally line.
pub fn render(reports: &[EntryReport]) -> String {
    let name_width = reports
        .iter()
        .map(|report| report.name.len())
        .max()
        .unwrap_or(0);
    let framework_width = reports
        .iter()
        .flat_map(|report| {
            report.rows.iter().map(|row| row.framework.len()).chain(
                report
                    .runner
                    .iter()
                    .map(|row| RUNNER_LABEL.len() + 1 + row.framework.len()),
            )
        })
        .max()
        .unwrap_or(0);

    let mut out = String::new();
    let _ = writeln!(
        out,
        "spirolo-bench · {} corpus {} · counts vs truth.json, wall-clock vs budget",
        reports.len(),
        if reports.len() == 1 {
            "entry"
        } else {
            "entries"
        },
    );

    for report in reports {
        out.push('\n');
        render_entry(&mut out, report, name_width, framework_width);
    }

    let (mut passed, mut failed, mut skipped) = (0, 0, 0);
    for report in reports {
        match report.status {
            Status::Pass => passed += 1,
            Status::Fail => failed += 1,
            Status::Skipped => skipped += 1,
        }
    }
    let _ = write!(
        out,
        "\n{failed} failed · {passed} passed · {skipped} skipped\n"
    );
    out
}

/// Render one entry: a headline `row` (name, timing, status) then a line per framework.
fn render_entry(out: &mut String, report: &EntryReport, name_width: usize, framework_width: usize) {
    let timing = report.timing.map_or_else(
        || "—".to_owned(),
        |timing| {
            let over_m2 = timing.baseline_ms.map_or_else(String::new, |baseline| {
                format!(
                    " (+{} over M2)",
                    format_ms(timing.elapsed_ms.saturating_sub(baseline))
                )
            });
            if let Some(busy) = timing.busy_percent {
                return format!(
                    "not measured: box {busy}% busy (idle ceiling {:.0}%)",
                    IDLE_BUSY_CEILING * 100.0
                );
            }
            format!(
                "{} median of {} / {} budget{}{}",
                format_ms(timing.elapsed_ms),
                timing.samples,
                format_ms(timing.budget_ms),
                if timing.within { "" } else { " OVER" },
                over_m2,
            )
        },
    );
    let _ = writeln!(
        out,
        "{:<name_width$}  {:<20}  {}",
        report.name,
        timing,
        report.status.tag(),
    );
    if let Some(note) = &report.note {
        let _ = writeln!(out, "  {note}");
    }
    // Each framework's counts line, then its runner line directly under it; a runner row
    // for a framework with no counts line (truth listed neither counts nor the model saw
    // it) follows the counts lines.
    let mut rendered_runner = BTreeSet::new();
    for row in &report.rows {
        let _ = writeln!(
            out,
            "  {:<framework_width$}  expected {}  actual {}  {}",
            row.framework,
            format_counts(row.expected_files, row.expected_tests),
            format_counts(Some(row.actual_files), Some(row.actual_tests)),
            if row.matched { "match" } else { "MISMATCH" },
        );
        for runner in report
            .runner
            .iter()
            .filter(|r| r.framework == row.framework)
        {
            render_runner_row(out, runner, framework_width);
            rendered_runner.insert(runner.framework.as_str());
        }
    }
    for runner in &report.runner {
        if !rendered_runner.contains(runner.framework.as_str()) {
            render_runner_row(out, runner, framework_width);
        }
    }
    for open in &report.open {
        let _ = writeln!(
            out,
            "  {:<framework_width$}  {}  {} {} tests left out of both sides: {}",
            "open", open.path, open.listed_tests, open.framework, open.reason,
        );
    }
}

/// The label a runner row carries before its framework name.
const RUNNER_LABEL: &str = "runner";

/// One runner row: what the runner listed, what spirolo tallied on the same set, the signed
/// delta per dimension, the tolerance, and the verdict.
fn render_runner_row(out: &mut String, row: &RunnerRow, framework_width: usize) {
    let _ = writeln!(
        out,
        "  {:<framework_width$}  listed {}  spirolo {} / {}  delta files {} / tests {}  \
         tolerance {} / {}  {}",
        format!("{RUNNER_LABEL} {}", row.framework),
        format_counts(Some(row.listed_files), row.listed_tests),
        row.actual_files,
        row.actual_tests,
        signed(row.delta_files()),
        row.delta_tests().map_or_else(|| "—".to_owned(), signed),
        row.tolerance_files,
        row.tolerance_tests,
        if row.matched { "match" } else { "MISMATCH" },
    );
}

/// A delta with its sign, `0` when there is none.
fn signed(delta: i128) -> String {
    if delta == 0 {
        "0".to_owned()
    } else {
        format!("{delta:+}")
    }
}

/// `files / tests` where a `None` dimension (truth gave no number) reads as `—`.
fn format_counts(files: Option<u64>, tests: Option<u64>) -> String {
    let files = files.map_or_else(|| "—".to_owned(), |value| format!("{value} files"));
    let tests = tests.map_or_else(|| "—".to_owned(), |value| format!("{value} tests"));
    format!("{files} / {tests}")
}

/// Whole milliseconds of a duration, saturating rather than wrapping on an absurd value.
fn duration_ms(elapsed: Duration) -> u64 {
    u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
}

/// Milliseconds as `950ms` under a second, `1.04s` at or above one, so the table reads at
/// the same glance the human `list` line does. Integer math throughout: a float cast here
/// would trip `clippy::cast_precision_loss`, and the two-decimal form needs no float.
fn format_ms(ms: u64) -> String {
    if ms < 1000 {
        format!("{ms}ms")
    } else {
        format!("{}.{:02}s", ms / 1000, (ms % 1000) / 10)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::time::Duration;

    use crate::counts::FrameworkCount;
    use crate::truth::{FrameworkCounts, RunnerCounts, RunnerExcludedFile, Truth};

    use super::{any_failed, evaluate, format_ms, render, Status};

    fn truth(budget_ms: u64, counts: &[(&str, Option<u64>, Option<u64>)]) -> Truth {
        Truth {
            access: crate::truth::Access::Local,
            execute: true,
            never_executed: Vec::new(),
            never_executed_by_rule: std::collections::BTreeMap::new(),
            substrate: None,
            excluded_packages: Vec::new(),
            excluded_files: Vec::new(),
            budget_ms,
            m2_list_ms: None,
            adjudications: Vec::new(),
            findings: Vec::new(),
            replay_none: Vec::new(),
            runner: BTreeMap::new(),
            incident: None,
            ledger: None,
            stratified_samples: std::collections::BTreeMap::new(),
            retired_adjudications: Vec::new(),
            counts: counts
                .iter()
                .map(|(name, files, tests)| {
                    (
                        (*name).to_owned(),
                        FrameworkCounts {
                            files: *files,
                            tests: *tests,
                        },
                    )
                })
                .collect(),
        }
    }

    fn actual(counts: &[(&str, u64, u64)]) -> BTreeMap<String, FrameworkCount> {
        counts
            .iter()
            .map(|(name, files, tests)| {
                (
                    (*name).to_owned(),
                    FrameworkCount {
                        files: *files,
                        tests: *tests,
                    },
                )
            })
            .collect()
    }

    #[test]
    fn an_exact_match_within_budget_passes() {
        let report = evaluate(
            "repo",
            &truth(5000, &[("vitest", Some(2), Some(3))]),
            &actual(&[("vitest", 2, 3)]),
            &BTreeMap::new(),
            Duration::from_millis(1000),
        );
        assert_eq!(report.status, Status::Pass);
        assert!(!any_failed(std::slice::from_ref(&report)));
    }

    #[test]
    fn a_test_count_delta_fails() {
        let report = evaluate(
            "repo",
            &truth(5000, &[("vitest", Some(2), Some(3))]),
            &actual(&[("vitest", 2, 2)]),
            &BTreeMap::new(),
            Duration::from_millis(1000),
        );
        assert_eq!(report.status, Status::Fail);
        assert!(!report.rows[0].matched);
    }

    #[test]
    fn a_run_over_budget_fails_even_with_matching_counts() {
        let report = evaluate(
            "repo",
            &truth(2000, &[("vitest", Some(2), Some(3))]),
            &actual(&[("vitest", 2, 3)]),
            &BTreeMap::new(),
            Duration::from_millis(2500),
        );
        assert_eq!(report.status, Status::Fail);
        assert!(!report.timing.unwrap().within);
    }

    #[test]
    fn an_added_second_over_the_recorded_m2_timing_fails_the_budget() {
        // An entry records its M2 `spirolo list` wall-clock; M3's budget is one added
        // second over it. A run 1.4 s over the baseline fails even though the flat
        // `budget_ms` ceiling is generous enough to pass on its own.
        let mut truth = truth(5000, &[("vitest", Some(2), Some(3))]);
        truth.m2_list_ms = Some(2300);
        let report = evaluate(
            "repo",
            &truth,
            &actual(&[("vitest", 2, 3)]),
            &BTreeMap::new(),
            Duration::from_millis(3700),
        );
        assert_eq!(report.status, Status::Fail);
        assert!(!report.timing.unwrap().within);
    }

    #[test]
    fn the_added_delta_over_m2_prints_on_the_entry_line() {
        // The bench states how much the run added over the recorded M2 timing, so the
        // performance budget's headroom is read off the output, not recomputed by hand.
        let mut truth = truth(5000, &[("vitest", Some(2), Some(3))]);
        truth.m2_list_ms = Some(2300);
        let report = evaluate(
            "repo",
            &truth,
            &actual(&[("vitest", 2, 3)]),
            &BTreeMap::new(),
            Duration::from_millis(2800),
        );
        let rendered = render(&[report]);
        assert!(
            rendered.contains("+500ms over M2"),
            "expected the added delta over M2, got:\n{rendered}"
        );
    }

    #[test]
    fn a_framework_truth_expects_but_the_model_missed_fails() {
        let report = evaluate(
            "repo",
            &truth(5000, &[("playwright", Some(22), Some(381))]),
            &actual(&[]),
            &BTreeMap::new(),
            Duration::from_millis(1000),
        );
        assert_eq!(report.status, Status::Fail);
        let row = &report.rows[0];
        assert_eq!(row.actual_tests, 0);
        assert!(!row.matched);
    }

    #[test]
    fn a_dimension_truth_left_null_is_not_compared() {
        // playwright.files is null in truth: only the test count is checked.
        let report = evaluate(
            "repo",
            &truth(5000, &[("playwright", None, Some(12))]),
            &actual(&[("playwright", 99, 12)]),
            &BTreeMap::new(),
            Duration::from_millis(1000),
        );
        assert_eq!(report.status, Status::Pass);
    }

    #[test]
    fn an_excluded_file_comes_off_the_expected_side_and_prints_as_an_open_line() {
        let mut truth = truth(5000, &[("playwright", Some(22), Some(381))]);
        truth.excluded_files.push(crate::truth::ExcludedFile {
            path: "e2e/smoke.spec.ts".to_owned(),
            framework: "playwright".to_owned(),
            listed_tests: 108,
            reason: "Its tests are generated at load time by a filesystem walk. More detail."
                .to_owned(),
        });
        // The actual side already left the file out (counts.rs does that by path).
        let mut actual = BTreeMap::new();
        actual.insert(
            "playwright".to_owned(),
            FrameworkCount {
                files: 21,
                tests: 273,
            },
        );
        let report = evaluate(
            "repo",
            &truth,
            &actual,
            &BTreeMap::new(),
            Duration::from_millis(100),
        );
        assert_eq!(report.status, Status::Pass);
        assert_eq!(report.rows[0].expected_files, Some(21));
        assert_eq!(report.rows[0].expected_tests, Some(273));
        let rendered = super::render(&[report]);
        assert!(
            rendered.contains(
                "open        e2e/smoke.spec.ts  108 playwright tests left out of both sides: \
                 Its tests are generated at load time by a filesystem walk"
            ),
            "{rendered}"
        );
    }

    /// A `runner` record with a large monorepo's shape: the runner listed 1634 / 18412, spirolo tallies
    /// 1634 / 18277 on the runner's set, the attributed gap is 135, so the row matches.
    fn lab_runner_truth(tolerance_tests: u64) -> Truth {
        let mut truth = truth(5000, &[("vitest", Some(1634), Some(18177))]);
        truth.runner.insert(
            "vitest".to_owned(),
            RunnerCounts {
                files: 1634,
                tests: Some(18412),
                excluded_packages: vec!["apps/dashboard".to_owned()],
                excluded_files: Vec::new(),
                tolerance_tests,
                tolerance_files: 0,
            },
        );
        truth
    }

    #[test]
    fn a_runner_row_within_tolerance_matches_and_prints_under_its_framework() {
        let report = evaluate(
            "repo",
            &lab_runner_truth(135),
            &actual(&[("vitest", 1634, 18177)]),
            &actual(&[("vitest", 1634, 18277)]),
            Duration::from_millis(1000),
        );
        assert_eq!(report.status, Status::Pass);
        assert_eq!(report.runner.len(), 1);
        let row = &report.runner[0];
        assert!(row.matched);
        assert_eq!((row.delta_files(), row.delta_tests()), (0, Some(-135)));
        let rendered = render(&[report]);
        assert!(
            rendered.contains(
                "  vitest         expected 1634 files / 18177 tests  actual 1634 files / 18177 tests  match\n\
                 \x20 runner vitest  listed 1634 files / 18412 tests  spirolo 1634 / 18277  \
                 delta files 0 / tests -135  tolerance 0 / 135  match\n"
            ),
            "{rendered}"
        );
    }

    #[test]
    fn a_runner_row_over_tolerance_is_a_mismatch_that_fails_the_entry() {
        let report = evaluate(
            "repo",
            &lab_runner_truth(134),
            &actual(&[("vitest", 1634, 18177)]),
            &actual(&[("vitest", 1634, 18277)]),
            Duration::from_millis(1000),
        );
        assert_eq!(report.status, Status::Fail);
        assert!(
            report.rows[0].matched,
            "the counts row itself still matches"
        );
        assert!(!report.runner[0].matched);
        assert!(any_failed(std::slice::from_ref(&report)));
        let rendered = render(&[report]);
        assert!(
            rendered.contains("delta files 0 / tests -135  tolerance 0 / 134  MISMATCH"),
            "{rendered}"
        );
    }

    #[test]
    fn a_runner_row_whose_files_are_off_by_one_is_a_mismatch_whatever_the_test_tolerance() {
        let report = evaluate(
            "repo",
            &lab_runner_truth(1000),
            &actual(&[("vitest", 1634, 18177)]),
            &actual(&[("vitest", 1635, 18412)]),
            Duration::from_millis(1000),
        );
        assert_eq!(report.status, Status::Fail);
        assert!(!report.runner[0].matched);
        let rendered = render(&[report]);
        assert!(
            rendered.contains(
                "spirolo 1635 / 18412  delta files +1 / tests 0  tolerance 0 / 1000  MISMATCH"
            ),
            "{rendered}"
        );
    }

    /// The runner's excluded file comes off its side the way the counts record's does, with
    /// the runner's own listed count for it (109 here, where `counts` carries 108).
    #[test]
    fn a_runner_excluded_file_is_netted_with_the_runner_own_listed_count() {
        let mut truth = truth(5000, &[("playwright", Some(22), Some(381))]);
        truth.excluded_files.push(crate::truth::ExcludedFile {
            path: "e2e/smoke.spec.ts".to_owned(),
            framework: "playwright".to_owned(),
            listed_tests: 108,
            reason: "generated at load time".to_owned(),
        });
        truth.runner.insert(
            "playwright".to_owned(),
            RunnerCounts {
                files: 22,
                tests: Some(382),
                excluded_packages: Vec::new(),
                excluded_files: vec![RunnerExcludedFile {
                    path: "e2e/smoke.spec.ts".to_owned(),
                    listed_tests: 109,
                }],
                tolerance_tests: 0,
                tolerance_files: 0,
            },
        );
        let report = evaluate(
            "repo",
            &truth,
            &actual(&[("playwright", 21, 273)]),
            &actual(&[("playwright", 21, 273)]),
            Duration::from_millis(100),
        );
        assert_eq!(report.status, Status::Pass);
        let row = &report.runner[0];
        assert_eq!((row.listed_files, row.listed_tests), (21, Some(273)));
        assert!(row.matched);
    }

    /// No `runner` record: no runner row, and the rendered entry is byte-for-byte what it
    /// was before the row existed.
    #[test]
    fn an_entry_without_a_runner_record_prints_no_runner_row() {
        let report = evaluate(
            "repo",
            &truth(5000, &[("vitest", Some(2), Some(3))]),
            &actual(&[("vitest", 2, 3)]),
            &actual(&[("vitest", 2, 3)]),
            Duration::from_millis(1200),
        );
        assert!(report.runner.is_empty());
        assert_eq!(
            render(&[report]),
            "spirolo-bench · 1 corpus entry · counts vs truth.json, wall-clock vs budget\n\
             \nrepo  1.20s median of 1 / 5.00s budget  PASS\n\
             \x20 vitest  expected 2 files / 3 tests  actual 2 files / 3 tests  match\n\
             \n0 failed · 1 passed · 0 skipped\n"
        );
    }

    #[test]
    fn milliseconds_render_at_a_glance() {
        assert_eq!(format_ms(950), "950ms");
        assert_eq!(format_ms(1040), "1.04s");
        assert_eq!(format_ms(5000), "5.00s");
    }
}
