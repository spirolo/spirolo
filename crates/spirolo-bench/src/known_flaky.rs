//! Reading an entry's `known-flaky.json`: the tests the team already wrote down as flaky,
//! pre-registered from written sources as the oracle for M4's static flake claim (#161).
//!
//! The bench scores the set against the static rule: each row is checked on a bare
//! worktree at its `flaky_commit`, where the flake was live, and is flagged when a decided
//! `flake-candidate` finding matches it ([`crate::ReportedFinding::matches`], the predicate
//! replay pairs and recall share).

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::{ReportedFinding, RuleVerdict, KNOWN_FLAKY_FLOOR};

/// The rule a known-flaky row is scored against.
const RULE: &str = "flake-candidate";

/// One test (or, with [`KnownFlakyRow::test`] `None`, one file) a written source records
/// as flaky.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnownFlakyRow {
    /// Repo-relative path of the test file at [`Self::flaky_commit`].
    pub file: String,
    /// Full title path (suite path then title) as `spirolo list` names it at
    /// [`Self::flaky_commit`]; `None` when the source names the file but no single test.
    pub test: Option<Vec<String>>,
    /// The commit the test is recorded flaky at: the fix's parent, or HEAD while unfixed.
    pub flaky_commit: String,
    /// The commit that fixed (or gated) it; `None` while unfixed.
    pub fix_commit: Option<String>,
    /// URL of the issue, PR or commit that writes the flakiness down.
    pub source: String,
    /// One line quoting the source's own account of why it flakes.
    pub mechanism: String,
    /// When the source recorded it (`YYYY-MM-DD`).
    pub recorded: String,
    /// What verification against the entry's mirror found beyond the fields.
    #[serde(default)]
    pub note: Option<String>,
}

/// A candidate the sources mention that was left out of the set, and why.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnownFlakyDropped {
    /// What was left out.
    pub what: String,
    /// The source that mentions it.
    pub source: String,
    /// Why it is not a row.
    pub reason: String,
}

/// An entry's pre-registered known-flaky set.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnownFlaky {
    /// The corpus entry the set belongs to.
    pub entry: String,
    /// When the set was pre-registered (`YYYY-MM-DD`).
    pub recorded: String,
    /// How the rows were found and verified.
    pub method: String,
    /// The recorded flaky tests.
    pub rows: Vec<KnownFlakyRow>,
    /// Candidates left out, each with its reason.
    pub dropped: Vec<KnownFlakyDropped>,
}

impl KnownFlaky {
    /// Parse a `known-flaky.json` document.
    ///
    /// # Errors
    /// When the text is not a valid set, a row's `source` is not a URL a reader can
    /// follow (a row without a written source is not pre-registered), or a row's commit
    /// is not a full sha (an abbreviation or a ref can come to mean another commit).
    pub fn parse(text: &str) -> Result<Self, String> {
        let set: Self = serde_json::from_str(text).map_err(|e| format!("known-flaky.json: {e}"))?;
        for row in &set.rows {
            if !is_url(&row.source) {
                return Err(format!(
                    "known-flaky.json: {} has no written source (source {:?} is not an https URL)",
                    row.file, row.source
                ));
            }
            for commit in std::iter::once(&row.flaky_commit).chain(&row.fix_commit) {
                if !is_full_sha(commit) {
                    return Err(format!(
                        "known-flaky.json: {} names commit {commit:?}, not a full 40-hex sha",
                        row.file
                    ));
                }
            }
        }
        Ok(set)
    }

    /// Read and parse a `known-flaky.json` file.
    ///
    /// # Errors
    /// When the file cannot be read or is not a valid set.
    pub fn read(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("reading {}: {e}", path.display()))?;
        Self::parse(&text)
    }
}

/// Whether `source` is a single `https://` URL with something after the scheme.
fn is_url(source: &str) -> bool {
    source
        .strip_prefix("https://")
        .is_some_and(|rest| !rest.is_empty() && !rest.contains(char::is_whitespace))
}

/// Whether `commit` is a full lowercase 40-hex git sha.
fn is_full_sha(commit: &str) -> bool {
    commit.len() == 40
        && commit
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// A function that returns the `check` findings on a bare worktree at a commit; an `Err`
/// means the commit could not be measured.
pub type CheckAt<'a> = dyn Fn(&str) -> Result<Vec<ReportedFinding>, String> + 'a;

/// One row's outcome against the static rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownFlakyRowResult {
    /// The row's file.
    pub file: String,
    /// The row's title path, `None` when it names only the file.
    pub test: Option<Vec<String>>,
    /// Whether a decided `flake-candidate` finding matched the row at its `flaky_commit`.
    pub flagged: bool,
    /// The check at the row's `fix_commit`, made only for a flagged row that has one.
    pub fix: FixSide,
    /// The runner's error when a commit of the row could not be checked; `Some` fails the
    /// set, since an un-checkable row must not read as "not flagged".
    pub error: Option<String>,
}

/// A flagged row's check at its `fix_commit`: the finding must be gone there, as a replay
/// pair's must be at its fixed commit, or a rule that flags everything would pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixSide {
    /// Not checked: the row was not flagged, or it has no `fix_commit` (still live).
    NotChecked,
    /// No decided finding matches the row at `fix_commit`: the check holds.
    Absent,
    /// A decided finding still matches the row at `fix_commit`: the check fails.
    Present,
}

/// An entry's known-flaky set scored against the static rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownFlakyScore {
    /// One result per row, in the set's order.
    pub rows: Vec<KnownFlakyRowResult>,
}

impl KnownFlakyScore {
    /// The rows flagged at their `flaky_commit`: k of the set's n.
    #[must_use]
    pub fn flagged(&self) -> usize {
        self.rows.iter().filter(|row| row.flagged).count()
    }

    /// The set's verdict: FAIL when a row could not be checked or a flagged row is still
    /// matched at its fix, whatever
    /// n is (a rule that flags everything flags the fix too); otherwise k of n against
    /// [`KNOWN_FLAKY_FLOOR`], INSUFFICIENT below its minimum n.
    #[must_use]
    pub fn verdict(&self) -> RuleVerdict {
        if self
            .rows
            .iter()
            .any(|row| row.fix == FixSide::Present || row.error.is_some())
        {
            return RuleVerdict::Fail;
        }
        KNOWN_FLAKY_FLOOR.verdict(self.flagged() as u64, self.rows.len() as u64)
    }
}

/// Score every row of `set` against the static rule.
///
/// Each row's `flaky_commit` is checked through `check_at` for a
/// decided `flake-candidate` finding that matches the row's file and, when it names one,
/// its exact title path; for a flagged row with a `fix_commit`, check that commit too.
/// Each commit is checked once, however many rows share it.
#[must_use]
pub fn score_known_flaky(set: &KnownFlaky, check_at: &CheckAt) -> KnownFlakyScore {
    let mut checked: BTreeMap<String, Result<Vec<ReportedFinding>, String>> = BTreeMap::new();
    let mut rows = Vec::new();
    for row in &set.rows {
        let mut matched_at = |commit: &str| -> Result<bool, String> {
            let findings = checked
                .entry(commit.to_owned())
                .or_insert_with(|| check_at(commit))
                .as_ref()
                .map_err(|error| format!("{commit}: {error}"))?;
            Ok(findings
                .iter()
                .any(|finding| finding.matches(RULE, &row.file, &[], row.test.as_deref())))
        };
        let mut error = None;
        let flagged = matched_at(&row.flaky_commit).unwrap_or_else(|e| {
            error = Some(format!("flaky commit {e}"));
            false
        });
        let fix = match &row.fix_commit {
            Some(fix_commit) if flagged => match matched_at(fix_commit) {
                Ok(true) => FixSide::Present,
                Ok(false) => FixSide::Absent,
                Err(e) => {
                    error = Some(format!("fix commit {e}"));
                    FixSide::NotChecked
                }
            },
            _ => FixSide::NotChecked,
        };
        rows.push(KnownFlakyRowResult {
            file: row.file.clone(),
            test: row.test.clone(),
            flagged,
            fix,
            error,
        });
    }
    KnownFlakyScore { rows }
}
