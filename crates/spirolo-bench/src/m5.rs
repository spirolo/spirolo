//! The M5 rows (PRD 0006, Bench): what `spirolo lsp` and `spirolo explain` answer on a corpus
//! entry against what `check` says about the same tree.
//!
//! The session itself runs in `corpus/lsp/m5.mjs` (the frozen client and recomputation of
//! #238); this module reads its summary, adds the `explain` round trip and the timings, and
//! turns them into rows and verdicts.

use std::fmt::Write as _;

use serde::Deserialize;
use serde_json::Value;

use crate::report::Timing;

/// A finding id no rule can produce: `explain` must exit 2 on it.
pub const UNKNOWN_ID: &str = "weak-assertion no/such.test.ts nothing#0";

/// One `spirolo explain <query> --format json` run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplainAnswer {
    /// The exit code.
    pub code: i32,
    /// What it printed on stdout.
    pub stdout: String,
    /// What it printed on stderr.
    pub stderr: String,
}

/// Runs `explain` for one query.
pub type ExplainRunner<'a> = dyn Fn(&str) -> Result<ExplainAnswer, String> + Sync + 'a;

/// The `explain` round trip over every finding id of a `check` document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplainScore {
    /// Ids whose answer is the finding `check` printed plus its status and editor severity.
    pub held: usize,
    /// Every finding id in the document.
    pub total: usize,
    /// Why an id did not hold, first ones only.
    pub misses: Vec<String>,
    /// Why the unknown id was not refused with exit 2, `None` when it was.
    pub unknown: Option<String>,
}

/// How many misses a score keeps.
const MISSES: usize = 10;

/// Whether `rule` is proven in `status`: its verdict is PASS and its floor is not carried.
fn proven(status: &Value, rule: &str) -> bool {
    let row = &status["rules"][rule];
    row["verdict"] == "PASS" && row["carried"].is_null()
}

/// Whether an unproven `rule` is advisory in `status`, so the default view shows it: not FAIL
/// or DROPPED, at least 15 decided findings, and shown precision at least 4/5.
fn advisory(status: &Value, rule: &str) -> bool {
    let row = &status["rules"][rule];
    let truths = row["truths"].as_u64().unwrap_or(0);
    let decided = truths + row["falses"].as_u64().unwrap_or(0);
    row["verdict"] != "FAIL"
        && row["verdict"] != "DROPPED"
        && decided >= 15
        && truths * 5 >= decided * 4
}

/// PRD 0006's editor severity for a finding (1–4), or null for a `listed` row.
fn editor_severity(finding: &Value, status: &Value) -> Value {
    let rule = finding["rule"].as_str().unwrap_or_default();
    let severity = finding["severity"].as_str().unwrap_or_default();
    let level = match finding["outcome"].as_str() {
        Some("listed") => return Value::Null,
        Some("could_not_decide") => 4,
        _ if proven(status, rule) => match severity {
            "error" => 1,
            "warn" => 2,
            _ => 3,
        },
        _ if severity == "info" && !advisory(status, rule) => 4,
        _ => 3,
    };
    Value::from(level)
}

/// Why `answer` is not `finding` round-tripped, or `None` when it is: exit 0, and the JSON is
/// the finding exactly as `check` printed it plus `status` (the table's object for the rule)
/// and `editor_severity`.
fn explain_miss(finding: &Value, status: &Value, answer: &ExplainAnswer) -> Option<String> {
    if answer.code != 0 {
        return Some(format!("exit {}: {}", answer.code, answer.stderr.trim()));
    }
    let Ok(Value::Object(mut got)) = serde_json::from_str::<Value>(&answer.stdout) else {
        return Some("stdout is not a JSON object".to_owned());
    };
    let rule = finding["rule"].as_str().unwrap_or_default();
    let got_status = got.remove("status").unwrap_or(Value::Null);
    let got_severity = got.remove("editor_severity").unwrap_or(Value::Null);
    if Value::Object(got) != *finding {
        return Some("the finding differs from check's".to_owned());
    }
    if got_status != status["rules"][rule] {
        return Some(format!("status {got_status} is not the table's"));
    }
    let want = editor_severity(finding, status);
    (got_severity != want).then(|| format!("editor_severity {got_severity}, expected {want}"))
}

/// Run `explain` on every finding id of `check` (on `workers` threads) and on
/// [`UNKNOWN_ID`], against `status`, the rule-status file the binary embeds.
#[must_use]
pub fn explain_round_trip(
    check: &Value,
    status: &Value,
    run: &ExplainRunner,
    workers: usize,
) -> ExplainScore {
    let findings: Vec<&Value> = check["findings"]
        .as_array()
        .map(|all| all.iter().collect())
        .unwrap_or_default();
    let chunk = findings.len().div_ceil(workers.max(1)).max(1);
    let mut misses: Vec<(usize, String)> = std::thread::scope(|scope| {
        // Collected so every worker is spawned before the first join.
        #[allow(clippy::needless_collect)]
        let handles: Vec<_> = findings
            .chunks(chunk)
            .enumerate()
            .map(|(at, part)| {
                scope.spawn(move || {
                    part.iter()
                        .enumerate()
                        .filter_map(|(offset, finding)| {
                            let id = finding["id"].as_str().unwrap_or_default();
                            let why = match run(id) {
                                Ok(answer) => explain_miss(finding, status, &answer)?,
                                Err(error) => error,
                            };
                            Some((at * chunk + offset, format!("{id}: {why}")))
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("an explain worker panicked"))
            .collect()
    });
    misses.sort();
    let held = findings.len() - misses.len();
    let wanted = format!("no finding with id '{UNKNOWN_ID}'");
    let unknown = match run(UNKNOWN_ID) {
        Ok(answer) if answer.code == 2 && answer.stderr.contains(&wanted) => None,
        Ok(answer) => Some(format!(
            "exit {}, stderr {:?}",
            answer.code,
            answer.stderr.trim()
        )),
        Err(error) => Some(error),
    };
    ExplainScore {
        held,
        total: findings.len(),
        misses: misses
            .into_iter()
            .take(MISSES)
            .map(|(_, why)| why)
            .collect(),
        unknown,
    }
}

/// One rule's status in a mismatch line: `<verdict> carried <to|null> <truths>/<falses>`.
fn status_brief(row: &Value) -> String {
    if row.is_null() {
        return "absent".to_owned();
    }
    let carried = row["carried"].as_str().unwrap_or("null");
    format!(
        "{} carried {carried} {}/{}",
        row["verdict"].as_str().unwrap_or("?"),
        row["truths"],
        row["falses"],
    )
}

/// Where the committed rule-status `file` disagrees with the one this `run` generates.
///
/// Its entries, then each rule whose object differs, in name order. The commit the file was
/// written at is not compared (it is never this run's).
#[must_use]
pub fn status_mismatches(file: &Value, run: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if file["entries"] != run["entries"] {
        out.push(format!(
            "entries: file {}, run {}",
            file["entries"], run["entries"]
        ));
    }
    let names = |value: &Value| -> Vec<String> {
        value["rules"]
            .as_object()
            .map(|rules| rules.keys().cloned().collect())
            .unwrap_or_default()
    };
    let mut rules = names(file);
    rules.extend(names(run));
    rules.sort();
    rules.dedup();
    for rule in rules {
        let (want, got) = (&file["rules"][&rule], &run["rules"][&rule]);
        if want == got {
            continue;
        }
        let (want_brief, got_brief) = (status_brief(want), status_brief(got));
        if want_brief == got_brief {
            out.push(format!("{rule}: file {want}, run {got}"));
        } else {
            out.push(format!("{rule}: file {want_brief}, run {got_brief}"));
        }
    }
    out
}

/// The load at or above which a timing is not a measurement (PRD 0006: every timing at load
/// < 1.5).
pub const LOAD_CEILING: f64 = 1.5;

/// The fewest pre-registered edits the live row passes on (PRD 0006: k = n with n ≥ 10).
pub const MIN_EDITS: usize = 10;

/// `corpus/lsp/m5.mjs`'s summary of one entry's session.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct M5Summary {
    /// The publishable diagnostics the recomputation expects after the first pass.
    pub publishable: u64,
    /// The first pass against the recomputation.
    pub diagnostics: DiagnosticDiff,
    /// Hover at every expected line, when the session hovered.
    pub hover: Option<HoverScore>,
    /// `spirolo/tree` against the recomputation.
    pub tree: TreeScore,
    /// `spirolo/file` for every test file against the recomputation.
    pub files: PanelScore,
    /// The pass after each edit against a fresh check on the edited tree, when there were
    /// edits.
    pub live: Option<Vec<LiveEdit>>,
    /// Diagnostics of an unproven rule above Information, over every pass.
    pub loud: Loud,
    /// The one-minute load average when the session started, when read.
    pub load: Option<f64>,
    /// Each timed save → `spirolo/checked`, in milliseconds.
    pub latencies_ms: Vec<f64>,
}

/// Expected against published diagnostics, as counts.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DiagnosticDiff {
    /// Expected and not published.
    #[serde(default)]
    pub missing: u64,
    /// Published and not expected.
    #[serde(default)]
    pub extra: u64,
    /// Published with another severity.
    #[serde(default)]
    pub severity: u64,
    /// Why there was nothing to compare (no pass), when there was not.
    #[serde(default)]
    pub error: Option<String>,
}

impl DiagnosticDiff {
    /// Nothing missing, extra or at another severity, and something was compared.
    const fn equal(&self) -> bool {
        self.error.is_none() && self.missing == 0 && self.extra == 0 && self.severity == 0
    }
}

/// Hovered lines whose answer matched.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct HoverScore {
    /// Lines whose hover matched.
    pub matched: u64,
    /// Lines with a publishable finding.
    pub lines: u64,
    /// The first misses.
    #[serde(default)]
    pub misses: Vec<HoverMiss>,
}

/// One hovered line that did not match.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct HoverMiss {
    /// The document.
    pub uri: String,
    /// The 0-based line.
    pub line: u64,
    /// Why it did not match.
    pub why: String,
}

/// `spirolo/tree`'s differences.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TreeScore {
    /// How many differences.
    pub differences: u64,
    /// The first ones.
    #[serde(default)]
    pub examples: Vec<String>,
}

/// `spirolo/file` over every test file.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PanelScore {
    /// Panels equal to the recomputation.
    pub equal: u64,
    /// Test files.
    pub total: u64,
    /// The first differing panels: the path and its differences.
    #[serde(default)]
    pub examples: Vec<(String, Vec<String>)>,
}

/// The pass after one edit against a fresh check.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LiveEdit {
    /// The edit, 1-based.
    pub edit: u64,
    /// The comparison.
    #[serde(flatten)]
    pub diff: DiagnosticDiff,
}

/// Loud diagnostics of unproven rules.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Loud {
    /// How many.
    pub count: u64,
}

/// The save → `spirolo/checked` budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveBudget {
    /// A fixed budget (for example 500 ms).
    Fixed {
        /// The budget.
        ms: u64,
    },
    /// `check`'s median plus a margin (for example + 500 ms).
    OverCheck {
        /// `check`'s median over its timed runs.
        check_ms: u64,
        /// The margin.
        over_ms: u64,
    },
}

impl SaveBudget {
    /// The budget in milliseconds.
    #[must_use]
    pub const fn ms(self) -> u64 {
        match self {
            Self::Fixed { ms } => ms,
            Self::OverCheck { check_ms, over_ms } => check_ms + over_ms,
        }
    }
}

/// The budget row's inputs besides the session's timed saves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct M5Budget {
    /// The save budget.
    pub save: SaveBudget,
    /// The entry's `spirolo list` timing from the counts section, when it was timed.
    pub list: Option<Timing>,
}

/// One entry's M5 rows.
#[derive(Debug, Clone, PartialEq)]
pub struct M5Rows {
    /// The session, scored.
    pub summary: M5Summary,
    /// The `explain` round trip.
    pub explain: ExplainScore,
    /// Where the committed status file disagrees with this run's verdicts.
    pub status: Vec<String>,
    /// The budget.
    pub budget: M5Budget,
}

/// The median of `samples` (the mean of the middle two for an even count).
fn median(samples: &[f64]) -> Option<f64> {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    match n {
        0 => None,
        _ if n % 2 == 1 => Some(sorted[n / 2]),
        _ => Some(f64::midpoint(sorted[n / 2 - 1], sorted[n / 2])),
    }
}

impl M5Rows {
    /// Whether the diagnostics row passes: 0 missing, 0 extra, 0 at another severity, and
    /// every hovered line matching when the session hovered.
    fn diagnostics_pass(&self) -> bool {
        self.summary.diagnostics.equal()
            && self
                .summary
                .hover
                .as_ref()
                .is_none_or(|hover| hover.lines > 0 && hover.matched == hover.lines)
    }

    /// Whether the live row passes: at least [`MIN_EDITS`] edits, each equal to a fresh check.
    fn live_pass(edits: &[LiveEdit]) -> bool {
        edits.len() >= MIN_EDITS && edits.iter().all(|edit| edit.diff.equal())
    }

    /// Whether the sidebar row passes: the tree equal and every test file's panel equal.
    const fn sidebar_pass(&self) -> bool {
        let files = &self.summary.files;
        self.summary.tree.differences == 0 && files.total > 0 && files.equal == files.total
    }

    /// Whether the honesty row passes: the status file is the run's, nothing loud, every id
    /// round-trips and the unknown id is refused.
    const fn honesty_pass(&self) -> bool {
        self.status.is_empty()
            && self.summary.loud.count == 0
            && self.explain.total > 0
            && self.explain.held == self.explain.total
            && self.explain.unknown.is_none()
    }

    /// The median save → `spirolo/checked`.
    fn save_median(&self) -> Option<f64> {
        median(&self.summary.latencies_ms)
    }

    /// Whether the budget row passes: timed at load < 1.5, the median save within its budget
    /// and `list` within its own.
    fn budget_pass(&self) -> bool {
        self.summary.load.is_some_and(|load| load < LOAD_CEILING)
            && self
                .save_median()
                .is_some_and(|ms| ms <= as_ms(self.budget.save.ms()))
            && self.budget.list.is_some_and(|list| list.within)
    }

    /// Each row's name and whether it passes, in print order; `live` only for an entry with
    /// edits.
    #[must_use]
    pub fn verdicts(&self) -> Vec<(&'static str, bool)> {
        let mut out = vec![("diagnostics", self.diagnostics_pass())];
        if let Some(edits) = &self.summary.live {
            out.push(("live", Self::live_pass(edits)));
        }
        out.push(("sidebar", self.sidebar_pass()));
        out.push(("honesty", self.honesty_pass()));
        out.push(("budget", self.budget_pass()));
        out
    }

    /// The rows that failed.
    #[must_use]
    pub fn failed(&self) -> Vec<&'static str> {
        self.verdicts()
            .into_iter()
            .filter(|(_, pass)| !pass)
            .map(|(row, _)| row)
            .collect()
    }
}

/// `PASS` or `FAIL`.
const fn tag(pass: bool) -> &'static str {
    if pass {
        "PASS"
    } else {
        "FAIL"
    }
}

/// At most `max` characters of `text`, with an ellipsis when cut.
fn clip(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_owned()
    } else {
        format!("{}…", text.chars().take(max).collect::<String>())
    }
}

/// Milliseconds, as `97 ms` or `2.40 s`.
fn format_ms(ms: f64) -> String {
    if ms < 1000.0 {
        format!("{ms:.0} ms")
    } else {
        format!("{:.2} s", ms / 1000.0)
    }
}

/// How many examples a failing row prints.
const SHOWN: usize = 3;

/// A millisecond count as a float, for medians and printing (never near 2^52).
#[allow(clippy::cast_precision_loss)]
const fn as_ms(ms: u64) -> f64 {
    ms as f64
}

/// One entry's M5 rows, each ending in its verdict; a failing row is followed by its first
/// differences.
#[must_use]
pub fn render_m5(rows: &M5Rows) -> String {
    let verdicts: std::collections::BTreeMap<&str, bool> = rows.verdicts().into_iter().collect();
    let mut out = String::new();
    render_diagnostics(&mut out, &rows.summary, verdicts["diagnostics"]);
    if let Some(edits) = &rows.summary.live {
        render_live(&mut out, edits, verdicts["live"]);
    }
    render_sidebar(&mut out, &rows.summary, verdicts["sidebar"]);
    render_honesty(&mut out, rows, verdicts["honesty"]);
    render_budget(&mut out, rows, verdicts["budget"]);
    out
}

/// The diagnostics row, then the first hover misses when it failed.
fn render_diagnostics(out: &mut String, summary: &M5Summary, pass: bool) {
    let diff = &summary.diagnostics;
    let compared = diff.error.as_ref().map_or_else(
        || {
            format!(
                "missing {} · extra {} · severity {}",
                diff.missing, diff.extra, diff.severity
            )
        },
        |error| format!("error: {error}"),
    );
    let hover = summary.hover.as_ref().map_or_else(String::new, |hover| {
        format!(" · hover {} of {} lines", hover.matched, hover.lines)
    });
    let _ = writeln!(
        out,
        "  M5 diagnostics  {} expected: {compared}{hover}  {}",
        summary.publishable,
        tag(pass),
    );
    if let Some(hover) = summary.hover.as_ref().filter(|_| !pass) {
        for miss in hover.misses.iter().take(SHOWN) {
            let file = miss.uri.rsplit('/').next().unwrap_or(&miss.uri);
            let _ = writeln!(
                out,
                "    hover {file}:{}  {}",
                miss.line,
                clip(&miss.why, 140)
            );
        }
    }
}

/// The live row, then the first edits that did not hold.
fn render_live(out: &mut String, edits: &[LiveEdit], pass: bool) {
    let held = edits.iter().filter(|edit| edit.diff.equal()).count();
    let _ = writeln!(
        out,
        "  M5 live         {held} of {} edits equal a fresh check  {}",
        edits.len(),
        tag(pass),
    );
    for edit in edits.iter().filter(|edit| !edit.diff.equal()).take(SHOWN) {
        let _ = writeln!(
            out,
            "    edit {}  missing {} · extra {} · severity {}{}",
            edit.edit,
            edit.diff.missing,
            edit.diff.extra,
            edit.diff.severity,
            edit.diff
                .error
                .as_ref()
                .map_or_else(String::new, |error| format!(" · {error}")),
        );
    }
}

/// The sidebar row, then the first tree and panel differences.
fn render_sidebar(out: &mut String, summary: &M5Summary, pass: bool) {
    let tree = if summary.tree.differences == 0 {
        "tree equal".to_owned()
    } else {
        format!("tree {} differences", summary.tree.differences)
    };
    let _ = writeln!(
        out,
        "  M5 sidebar      {tree} · files {} of {} equal  {}",
        summary.files.equal,
        summary.files.total,
        tag(pass),
    );
    for line in summary.tree.examples.iter().take(SHOWN) {
        let _ = writeln!(out, "    tree  {}", clip(line, 140));
    }
    for (path, lines) in summary.files.examples.iter().take(SHOWN) {
        let first = lines.first().map_or("", String::as_str);
        let _ = writeln!(out, "    file {path}  {}", clip(first, 140));
    }
}

/// The honesty row, then the first status mismatches and `explain` misses.
fn render_honesty(out: &mut String, rows: &M5Rows, pass: bool) {
    let status = if rows.status.is_empty() {
        "status file equals the run".to_owned()
    } else {
        format!("status file differs on {}", rows.status.len())
    };
    let unknown = rows.explain.unknown.as_ref().map_or_else(
        || "unknown id exit 2".to_owned(),
        |why| format!("unknown id {why}"),
    );
    let _ = writeln!(
        out,
        "  M5 honesty      {status} · {} above Information from unproven rules · explain {} of {} · {unknown}  {}",
        rows.summary.loud.count,
        rows.explain.held,
        rows.explain.total,
        tag(pass),
    );
    for line in rows.status.iter().chain(&rows.explain.misses).take(SHOWN) {
        let _ = writeln!(out, "    {}", clip(line, 160));
    }
}

/// The budget row: the median save against its budget, the load, and `list`'s timing.
fn render_budget(out: &mut String, rows: &M5Rows, pass: bool) {
    let budget = match rows.budget.save {
        SaveBudget::Fixed { ms } => format_ms(as_ms(ms)),
        SaveBudget::OverCheck { check_ms, over_ms } => format!(
            "{} (check median {} + {})",
            format_ms(as_ms(check_ms + over_ms)),
            format_ms(as_ms(check_ms)),
            format_ms(as_ms(over_ms)),
        ),
    };
    let median = rows.save_median().map_or_else(|| "—".to_owned(), format_ms);
    let load = rows
        .summary
        .load
        .map_or_else(|| "unread".to_owned(), |load| format!("{load:.2}"));
    let list = rows.budget.list.map_or_else(
        || "list not timed".to_owned(),
        |list| {
            format!(
                "list {} / {}{}",
                format_ms(as_ms(list.elapsed_ms)),
                format_ms(as_ms(list.budget_ms)),
                if list.within { "" } else { " over" },
            )
        },
    );
    let _ = writeln!(
        out,
        "  M5 budget       save→checked median of {}: {median} / {budget} at load {load} · {list}  {}",
        rows.summary.latencies_ms.len(),
        tag(pass),
    );
}
