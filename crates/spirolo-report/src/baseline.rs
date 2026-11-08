//! The ratchet baseline: a count of today's findings per file per rule, plus a ratio
//! ceiling per ratio rule, so `check` fails only on findings above what was recorded.
//!
//! The shape is `ESLint`'s bulk-suppressions file, not a hash scheme: `{ file: { rule:
//! count } }`, matched per file and per rule, never per line, so a line shift in an
//! unrelated edit does not turn an old finding into a new one. A file whose count for a
//! rule rises reports *every* finding of that rule in that file — the tool never guesses
//! which one is new. A count that falls is a prune candidate; a quarter or more of the
//! baseline's entries matching nothing is staleness. Ratio rules (`weak-assertion`'s
//! project-wide row) carry no file, so they live in a separate `ratios` map keyed by rule
//! and are governed by a ceiling rather than a count.
//!
//! The finding's identity plays no part here: the baseline counts, it does not name. That
//! identity still exists for `explain`, SARIF and the harness join; the baseline simply
//! does not depend on it, which is what keeps it line-shift immune.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use spirolo_rules::{Finding, Outcome};

/// The evidence key `weak-assertion` (and any later ratio rule) stores its measured ratio
/// under; the baseline reads it to record and compare ceilings.
const RATIO_EVIDENCE_KEY: &str = "ratio";

/// A recorded baseline: the count of findings per file per rule, plus a ceiling per ratio
/// rule.
///
/// `files` maps a repo-relative path to the rules that had findings in it and how many;
/// `ratios` maps a ratio rule to the ceiling recorded for it (the ratio measured when the
/// baseline was written or last tightened). Both are `BTreeMap`s so serialization is
/// sorted and byte-stable, with no line numbers anywhere.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Baseline {
    /// File to rule to count of that rule's findings in that file.
    #[serde(default)]
    pub files: BTreeMap<String, BTreeMap<String, u32>>,
    /// Ratio rule to its ceiling: a project-wide ratio at or below this is not new.
    #[serde(default)]
    pub ratios: BTreeMap<String, f64>,
}

/// How a rule's count in one file compares between the baseline and the current run, and
/// how a ratio rule's ceiling compares — the raw material the report and the exit code
/// read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diff {
    /// The findings the run should surface: every finding in a `(file, rule)` whose count
    /// rose above the baseline, and every ratio finding above its ceiling. A `(file, rule)`
    /// whose count is unchanged or fell surfaces nothing.
    pub surfaced: Vec<Finding>,
    /// Baseline `(file, rule)` entries whose current count is strictly lower than recorded
    /// — prune candidates, reported as a warning and dropped by `--prune`.
    pub fallen: Vec<Fallen>,
    /// Baseline entries (file-rule counts and ratio ceilings) the current run matched
    /// nothing for at all: the numerator of the staleness fraction.
    pub unmatched: usize,
    /// The total number of baseline entries: the denominator of the staleness fraction.
    pub total: usize,
}

/// One baseline entry whose current count fell below what was recorded: a prune candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fallen {
    /// The file the entry counts findings in.
    pub file: String,
    /// The rule the entry counts.
    pub rule: String,
    /// The count the baseline recorded.
    pub was: u32,
    /// The count the current run measured (strictly lower than `was`).
    pub now: u32,
}

impl Diff {
    /// Whether a quarter or more of the baseline's entries matched nothing this run — the
    /// staleness threshold fallow uses. Zero entries is never stale.
    pub const fn is_stale(&self) -> bool {
        self.total > 0 && self.unmatched * 4 >= self.total
    }
}

impl Baseline {
    /// Record a run's findings as a baseline: count per file per rule, and a ceiling per
    /// ratio rule taken from that rule's measured ratio.
    ///
    /// Findings with no file (the project-wide ratio row) do not enter the per-file counts;
    /// their ratio evidence becomes a `ratios` ceiling instead. `could not decide` and
    /// listing outcomes are not baselined — they never fail the build, so there is nothing
    /// to suppress.
    pub fn from_findings(findings: &[Finding]) -> Self {
        let mut baseline = Self::default();
        for finding in findings {
            if finding.outcome != Outcome::Finding {
                continue;
            }
            if finding.location.file.is_empty() {
                if let Some(ratio) = ratio_of(finding) {
                    baseline.ratios.insert(finding.rule.to_owned(), ratio);
                }
                continue;
            }
            *baseline
                .files
                .entry(finding.location.file.clone())
                .or_default()
                .entry(finding.rule.to_owned())
                .or_insert(0) += 1;
        }
        baseline
    }

    /// Serialize as sorted, byte-stable JSON with a trailing newline and no line numbers.
    ///
    /// # Panics
    ///
    /// Never in practice: the baseline is plain maps of owned values, which `serde_json`
    /// serializes infallibly.
    pub fn to_json(&self) -> String {
        let mut json = serde_json::to_string_pretty(self).expect("a baseline serializes");
        json.push('\n');
        json
    }

    /// Parse a baseline from its JSON, as `check` reads `.spirolo/baseline.json`.
    pub fn from_json(source: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(source)
    }

    /// Diff the current run's findings against this baseline: what to surface, what fell,
    /// and how many entries matched nothing.
    ///
    /// A `(file, rule)` whose current count exceeds the recorded one surfaces *all* of that
    /// rule's findings in that file; equal or fewer surfaces none. A ratio rule surfaces its
    /// row only when the measured ratio exceeds the recorded ceiling. An entry the run
    /// produced no finding for counts toward staleness.
    pub fn diff(&self, findings: &[Finding]) -> Diff {
        let counts = count_by_file_rule(findings);

        let mut surfaced = Vec::new();
        let mut fallen = Vec::new();
        let mut unmatched = 0;
        let total = self.entry_count();

        for (file, rules) in &self.files {
            for (rule, &was) in rules {
                let now = counts
                    .get(&(file.clone(), rule.clone()))
                    .copied()
                    .unwrap_or(0);
                if now == 0 {
                    unmatched += 1;
                }
                if now < was {
                    fallen.push(Fallen {
                        file: file.clone(),
                        rule: rule.clone(),
                        was,
                        now,
                    });
                }
            }
        }

        // A `(file, rule)` whose count rose above the baseline (recorded count, or zero when
        // the pair is not baselined) surfaces every one of that rule's findings in the file.
        for finding in findings {
            if finding.outcome != Outcome::Finding {
                continue;
            }
            if finding.location.file.is_empty() {
                if self.ratio_exceeded(finding) {
                    surfaced.push(finding.clone());
                }
                continue;
            }
            let was = self
                .files
                .get(&finding.location.file)
                .and_then(|rules| rules.get(finding.rule))
                .copied()
                .unwrap_or(0);
            let now = counts
                .get(&(finding.location.file.clone(), finding.rule.to_owned()))
                .copied()
                .unwrap_or(0);
            if now > was {
                surfaced.push(finding.clone());
            }
        }

        // A ratio ceiling the run produced no row above is an unmatched entry too.
        for rule in self.ratios.keys() {
            let above = findings
                .iter()
                .any(|f| f.location.file.is_empty() && f.rule == rule && self.ratio_exceeded(f));
            if !above {
                unmatched += 1;
            }
        }

        Diff {
            surfaced,
            fallen,
            unmatched,
            total,
        }
    }

    /// Lower every count to the current run's count and every ratio ceiling to the current
    /// ratio: `ratchet --tighten`. Counts that are already lower stay; a count with no
    /// current finding drops to zero (and is then a prune candidate).
    pub fn tighten(&mut self, findings: &[Finding]) {
        let counts = count_by_file_rule(findings);
        for (file, rules) in &mut self.files {
            for (rule, count) in rules.iter_mut() {
                let now = counts
                    .get(&(file.clone(), rule.clone()))
                    .copied()
                    .unwrap_or(0);
                if now < *count {
                    *count = now;
                }
            }
        }
        for (rule, ceiling) in &mut self.ratios {
            if let Some(now) = current_ratio(findings, rule) {
                if now < *ceiling {
                    *ceiling = now;
                }
            }
        }
    }

    /// Drop every entry the current run matched nothing for: `ratchet --prune`. A
    /// `(file, rule)` count with no current finding goes, an empty file map goes, and a
    /// ratio ceiling with no current row goes.
    pub fn prune(&mut self, findings: &[Finding]) {
        let counts = count_by_file_rule(findings);
        self.files.retain(|file, rules| {
            rules.retain(|rule, _| {
                counts
                    .get(&(file.clone(), rule.clone()))
                    .copied()
                    .unwrap_or(0)
                    > 0
            });
            !rules.is_empty()
        });
        self.ratios
            .retain(|rule, _| current_ratio(findings, rule).is_some());
    }

    /// Whether a ratio finding's measured ratio exceeds this baseline's recorded ceiling
    /// for its rule. A rule with no recorded ceiling has an implicit ceiling of zero, so any
    /// ratio row surfaces.
    fn ratio_exceeded(&self, finding: &Finding) -> bool {
        let Some(measured) = ratio_of(finding) else {
            return false;
        };
        let ceiling = self.ratios.get(finding.rule).copied().unwrap_or(0.0);
        measured > ceiling
    }

    /// The number of baseline entries: every `(file, rule)` count plus every ratio ceiling.
    fn entry_count(&self) -> usize {
        self.files.values().map(BTreeMap::len).sum::<usize>() + self.ratios.len()
    }
}

/// Count non-ratio, decided findings by `(file, rule)`.
fn count_by_file_rule(findings: &[Finding]) -> BTreeMap<(String, String), u32> {
    let mut counts: BTreeMap<(String, String), u32> = BTreeMap::new();
    for finding in findings {
        if finding.outcome != Outcome::Finding || finding.location.file.is_empty() {
            continue;
        }
        *counts
            .entry((finding.location.file.clone(), finding.rule.to_owned()))
            .or_insert(0) += 1;
    }
    counts
}

/// The measured ratio of the current run's ratio row for `rule`, if it produced one.
fn current_ratio(findings: &[Finding], rule: &str) -> Option<f64> {
    findings
        .iter()
        .find(|f| f.location.file.is_empty() && f.rule == rule)
        .and_then(ratio_of)
}

/// The ratio a ratio finding measured, read from its `ratio` evidence entry.
fn ratio_of(finding: &Finding) -> Option<f64> {
    finding
        .evidence
        .iter()
        .find(|e| e.key == RATIO_EVIDENCE_KEY)
        .and_then(|e| e.value.parse().ok())
}
