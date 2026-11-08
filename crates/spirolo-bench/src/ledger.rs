//! Ledger scoring (PRD 0004 "Harness", #181): the flake ledger `spirolo` builds from the
//! committed run reports, against the set an independent oracle recomputed from the same
//! raw bytes.
//!
//! The runs live under `corpus/<entry>/runs/<run-id>/`: gzipped vitest JSON reports and a
//! `run.json` sidecar, written by the run generator. The oracle
//! (`corpus/<entry>/adjudicate/flake-ledger.mjs`) reads them with its own parse and writes the
//! tests that both passed and failed on one tree hash into `truth.json` under `ledger`. The
//! bench stages the same reports into a worktree at their tree hash, runs `flake ingest`
//! then `check`, and compares the `flake-observed` findings with that set exactly: the
//! missing and the extra tests by name. The pre-registered live row's line says whether
//! the reports show it flipping and whether the ledger names it.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;

use serde::Deserialize;

use crate::{ReportedFinding, Truth};

/// A function that runs `spirolo` over an entry's staged runs and returns what it said.
///
/// It stages `corpus/<entry>/runs/` into a worktree of the clone (`repo`) at their tree
/// hash, then runs `flake ingest`, `check` and `flake quarantine` twice there. The binary
/// shells out; a test hands back a canned [`LedgerRun`].
pub type LedgerRunner<'a> = dyn Fn(&Path, &Path) -> Result<LedgerRun, String> + 'a;

/// Score an entry's ledger, when its `truth.json` records one.
///
/// The runner's findings over `<dir>/runs/` are compared with the oracle's set. `Some(Err)`
/// when the runner failed, so the ledger row fails loudly rather than vanishing.
///
/// # Errors
/// When the entry's `truth.json` cannot be read.
pub fn score_entry_ledger(
    dir: &Path,
    repo: &Path,
    runner: &LedgerRunner,
) -> Result<Option<Result<LedgerScore, String>>, String> {
    let truth = Truth::read(&dir.join("truth.json"))?;
    Ok(truth
        .ledger
        .map(|ledger| runner(&dir.join("runs"), repo).map(|run| score_ledger(&ledger, &run))))
}

/// The rule the ledger is scored against.
const RULE: &str = "flake-observed";

/// One test the oracle found both passing and failing on one tree hash.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LedgerFlake {
    /// Repo-relative path of the test file.
    pub file: String,
    /// Title path: suite path then title.
    pub test: Vec<String>,
}

/// The pre-registered live row (a `known-flaky.json` row still flaky at the runs' tree) and
/// what the oracle saw of it in the reports.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LiveRow {
    /// The written source the row comes from.
    pub source: String,
    /// Repo-relative path of the test file.
    pub file: String,
    /// Title path: suite path then title.
    pub test: Vec<String>,
    /// Whether some run passed or failed the test.
    pub ran: bool,
    /// Whether the test both passed and failed on one tree in the reports.
    pub flipped: bool,
}

/// The `ledger` section of an entry's `truth.json`, written by the ledger oracle.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LedgerTruth {
    /// The run ids the oracle read.
    pub runs: Vec<String>,
    /// The tests with a pass and a fail on one tree hash.
    pub flakes: Vec<LedgerFlake>,
    /// The pre-registered live row.
    pub live_row: LiveRow,
}

impl LedgerTruth {
    /// Parse a `ledger` section.
    ///
    /// # Errors
    /// When the text is not a ledger section.
    pub fn parse(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|error| format!("ledger truth: {error}"))
    }
}

/// `spirolo flake quarantine` run twice over the ledger: how many entries it wrote, and
/// whether the second run left the file byte-identical.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuarantineRoundTrip {
    /// Entries in `.spirolo/quarantine.json` after the first run.
    pub entries: usize,
    /// Whether the second run rewrote identical bytes.
    pub identical: bool,
}

/// What `spirolo` said over the staged runs: every `check` finding, and the quarantine
/// round-trip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerRun {
    /// Every finding `check` reported after `flake ingest`.
    pub findings: Vec<ReportedFinding>,
    /// The quarantine file written twice.
    pub quarantine: QuarantineRoundTrip,
}

/// The ledger scored: the tests the rule missed and the ones it added, by name
/// (`file > suite > title`), sorted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerScore {
    /// Flakes in the oracle's set.
    pub expected: usize,
    /// Decided `flake-observed` findings.
    pub reported: usize,
    /// Oracle flakes the rule does not name.
    pub missing: Vec<String>,
    /// Rule findings the oracle does not have.
    pub extra: Vec<String>,
    /// The runs the oracle read.
    pub runs: usize,
    /// The live row: what the reports show and whether the ledger names it.
    pub live: LiveStatus,
    /// The quarantine file written twice over the ledger.
    pub quarantine: QuarantineRoundTrip,
}

/// The live row's line: whether it flipped in the reports (the oracle's reading) and
/// whether the ledger names it (the rule's).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveStatus {
    /// The written source the row comes from.
    pub source: String,
    /// The test's name, `file > suite > title`.
    pub name: String,
    /// Whether some run passed or failed it.
    pub ran: bool,
    /// Whether it both passed and failed on one tree in the reports.
    pub flipped: bool,
    /// Whether a decided `flake-observed` finding names it.
    pub named: bool,
}

impl LiveStatus {
    /// The line's verdict. When the reports never flipped the row, the claim "the ledger
    /// names the live row" was not exercised, and the line says so rather than passing.
    #[must_use]
    pub fn status(&self) -> String {
        match (self.ran, self.flipped, self.named) {
            (_, true, true) => "flipped in the reports, named by the ledger".to_owned(),
            (_, true, false) => "flipped in the reports, NOT named by the ledger".to_owned(),
            (true, false, _) => {
                "ran, did not flip in the reports: \"the ledger names the live row\" not exercised"
                    .to_owned()
            }
            (false, false, _) => {
                "did not run in the reports: \"the ledger names the live row\" not exercised"
                    .to_owned()
            }
        }
    }
}

impl LedgerScore {
    /// Whether the rule named exactly the oracle's set.
    #[must_use]
    pub const fn exact(&self) -> bool {
        self.missing.is_empty() && self.extra.is_empty()
    }

    /// Whether the quarantine round-trip held: one entry per ledger flake, and a second
    /// write byte-identical to the first.
    #[must_use]
    pub const fn quarantine_held(&self) -> bool {
        self.quarantine.identical && self.quarantine.entries == self.reported
    }
}

/// PASS or FAIL.
const fn tag(pass: bool) -> &'static str {
    if pass {
        "PASS"
    } else {
        "FAIL"
    }
}

/// The ledger block of an entry's precision section: the exact match (or its missing and
/// extra tests by name), the live row's line, and the quarantine round-trip row.
#[must_use]
pub fn render_ledger(score: &LedgerScore) -> String {
    let mut out = String::new();
    let matched = if score.exact() {
        "exact".to_owned()
    } else {
        format!(
            "{} missing  {} extra",
            score.missing.len(),
            score.extra.len()
        )
    };
    let _ = writeln!(
        out,
        "  ledger {RULE}  {} runs  oracle {}  ledger {}  {matched}  {}",
        score.runs,
        score.expected,
        score.reported,
        tag(score.exact()),
    );
    for name in &score.missing {
        let _ = writeln!(out, "    missing  {name}");
    }
    for name in &score.extra {
        let _ = writeln!(out, "    extra    {name}");
    }
    let live = &score.live;
    let _ = writeln!(
        out,
        "  live row {}  {}  {}",
        live.source,
        live.name,
        live.status()
    );
    let _ = writeln!(
        out,
        "  quarantine round-trip  {} entries of {} flakes  rewrite {}byte-identical  {}",
        score.quarantine.entries,
        score.reported,
        if score.quarantine.identical {
            ""
        } else {
            "NOT "
        },
        tag(score.quarantine_held()),
    );
    out
}

/// A test's name in the score: its file, then its title path, joined by `" > "`.
fn name(file: &str, test: &[String]) -> String {
    std::iter::once(file)
        .chain(test.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(" > ")
}

/// Compare the decided `flake-observed` findings in `run` with the oracle's flakes.
#[must_use]
pub fn score_ledger(truth: &LedgerTruth, run: &LedgerRun) -> LedgerScore {
    let expected: BTreeSet<String> = truth
        .flakes
        .iter()
        .map(|flake| name(&flake.file, &flake.test))
        .collect();
    let reported: BTreeSet<String> = run
        .findings
        .iter()
        .filter(|finding| finding.rule == RULE && finding.decided)
        .map(|finding| name(&finding.file, finding.test.as_deref().unwrap_or_default()))
        .collect();
    let live = &truth.live_row;
    let live_name = name(&live.file, &live.test);
    LedgerScore {
        live: LiveStatus {
            source: live.source.clone(),
            named: reported.contains(&live_name),
            name: live_name,
            ran: live.ran,
            flipped: live.flipped,
        },
        runs: truth.runs.len(),
        quarantine: run.quarantine,
        expected: expected.len(),
        reported: reported.len(),
        missing: expected.difference(&reported).cloned().collect(),
        extra: reported.difference(&expected).cloned().collect(),
    }
}
