//! M4b rows: `impact` scored against the frozen mutation oracle (PRD 0005, Bench; #226).
//!
//! The oracle is `corpus/<entry>/impact/truth.json`, generated and frozen by #221: a seeded
//! sample of killed mutants with their killing test files, Vitest's related-file selection
//! for each mutant's file (the baseline), and a file set with per-file mutation scores. It
//! is read, never written. The rows ask `spirolo impact` about each mutant's file through a
//! runner, so tests replay recorded documents and the binary runs the real executable.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use crate::precision::format_interval;
use crate::{wilson_interval, ReportedFinding, RuleVerdict, Wallclock, IDLE_BUSY_CEILING};

/// Safety and ranking judge a sample of at least this many mutants (PRD 0005, Bench).
pub const IMPACT_MIN_N: u64 = 100;
/// Safety passes at this share of mutants with a killer selected.
pub const SAFETY_PASS_AT: f64 = 0.95;
/// Ranking passes at this share of mutants with a killer ranked first (and not below the
/// baseline's share).
pub const RANKING_PASS_AT: f64 = 0.70;

/// The hotspot row reads this many of the rule's top files within the set.
pub const HOTSPOT_TOP: usize = 20;
/// The hotspot row passes when the Wilson 95 % lower bound of the share below the median is
/// above this.
pub const HOTSPOT_LOWER_ABOVE: f64 = 0.5;

/// The subset safety is also printed on: packages with at least this many test files.
pub const SUBSET_MIN_TEST_FILES: u64 = 3;

/// The frozen impact oracle: `truth.json` under an entry's `impact/` directory.
#[derive(Debug, Clone, Deserialize)]
pub struct ImpactTruth {
    /// The entry commit the oracle was generated at; the clone must be checked out there.
    pub pin: String,
    /// The sampled killed mutants.
    pub mutants: Vec<ImpactMutant>,
    /// Vitest's related test files per sampled file, ordered by path.
    pub baseline: BTreeMap<String, Vec<String>>,
    /// The file set the hotspot row ranks within.
    pub files: FileSet,
}

/// The frozen file set: every reported source file with enough valid mutants, and its
/// mutation score.
#[derive(Debug, Clone, Deserialize)]
pub struct FileSet {
    /// The median mutation score over the set.
    pub median: f64,
    /// One row per file.
    pub rows: Vec<FileScore>,
}

/// A file of the set and its mutation score, (Killed + Timeout) / valid.
#[derive(Debug, Clone, Deserialize)]
pub struct FileScore {
    /// The source file, repo-relative.
    pub file: String,
    /// Its mutation score.
    pub score: f64,
}

/// One sampled killed mutant.
#[derive(Debug, Clone, Deserialize)]
pub struct ImpactMutant {
    /// The package directory, repo-relative.
    pub package: String,
    /// The mutated source file, repo-relative: the changed file `impact` is asked about.
    pub file: String,
    /// Stryker's mutant id within the package's report.
    pub id: String,
    /// The test files that kill it, repo-relative.
    pub killers: Vec<String>,
    /// How many test files the package has.
    pub test_files: u64,
}

impl ImpactTruth {
    /// Read and parse the oracle at `path`.
    ///
    /// # Errors
    /// When the file cannot be read or is not the oracle's shape.
    pub fn read(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))
    }
}

/// How `impact` answered for one sampled mutant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutantResult {
    /// The mutated file.
    pub file: String,
    /// The mutant id.
    pub id: String,
    /// How many test files the mutant's package has.
    pub test_files: u64,
    /// How many test files `impact` selected.
    pub selected: u64,
    /// Whether any killing test file is among `impact`'s selected files.
    pub killer_selected: bool,
    /// Whether `impact`'s first selected file kills the mutant.
    pub killer_first: bool,
    /// Whether the baseline's first related file kills the mutant.
    pub baseline_killer_first: bool,
    /// The runner's error, when `impact` could not answer; the mutant then counts as missed.
    pub error: Option<String>,
}

/// The `impact` rows over one oracle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImpactScore {
    /// One result per sampled mutant, in the oracle's order.
    pub mutants: Vec<MutantResult>,
}

impl ImpactScore {
    /// Safety: mutants with a killer selected, of all sampled.
    #[must_use]
    pub fn safety(&self) -> (u64, u64) {
        self.count(|_| true, |m| m.killer_selected)
    }

    /// Safety on the mutants whose package has at least [`SUBSET_MIN_TEST_FILES`] test
    /// files, so one-test packages cannot carry the claim.
    #[must_use]
    pub fn safety_subset(&self) -> (u64, u64) {
        self.count(
            |m| m.test_files >= SUBSET_MIN_TEST_FILES,
            |m| m.killer_selected,
        )
    }

    /// Ranking: mutants whose killer `impact` ranks first, of all sampled.
    #[must_use]
    pub fn top1(&self) -> (u64, u64) {
        self.count(|_| true, |m| m.killer_first)
    }

    /// The baseline's ranking: mutants whose killer is Vitest's first related file by path.
    #[must_use]
    pub fn baseline_top1(&self) -> (u64, u64) {
        self.count(|_| true, |m| m.baseline_killer_first)
    }

    /// PASS at [`SAFETY_PASS_AT`] with at least [`IMPACT_MIN_N`] mutants.
    #[must_use]
    pub fn safety_verdict(&self) -> RuleVerdict {
        let (k, n) = self.safety();
        judged(n, || share(k, n) >= SAFETY_PASS_AT)
    }

    /// PASS at [`RANKING_PASS_AT`] and not below the baseline's rate, with at least
    /// [`IMPACT_MIN_N`] mutants.
    #[must_use]
    pub fn ranking_verdict(&self) -> RuleVerdict {
        let ((k, n), (base, _)) = (self.top1(), self.baseline_top1());
        ranking(k, base, n)
    }

    /// The median, over the mutants `impact` answered for, of the selected files as a share
    /// of the package's test files; above 1 when importers in other packages are selected.
    /// `None` when no mutant was answered.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // counts far below 2^52
    pub fn median_share(&self) -> Option<f64> {
        let mut shares: Vec<f64> = self
            .mutants
            .iter()
            .filter(|m| m.error.is_none() && m.test_files > 0)
            .map(|m| m.selected as f64 / m.test_files as f64)
            .collect();
        shares.sort_by(f64::total_cmp);
        let mid = shares.len() / 2;
        match shares.len() {
            0 => None,
            len if len % 2 == 1 => Some(shares[mid]),
            _ => Some(f64::midpoint(shares[mid - 1], shares[mid])),
        }
    }

    /// `(k, n)`: of the mutants `within` keeps, how many `hit`.
    fn count(
        &self,
        within: impl Fn(&MutantResult) -> bool,
        hit: impl Fn(&MutantResult) -> bool,
    ) -> (u64, u64) {
        self.mutants
            .iter()
            .filter(|m| within(m))
            .fold((0, 0), |(k, n), m| (k + u64::from(hit(m)), n + 1))
    }
}

/// Runs `spirolo impact --format json <file>` and returns the parsed document.
pub type ImpactRunner<'a> = dyn Fn(&str) -> Result<Value, String> + 'a;

/// Score `impact` against the oracle: ask `run` about each sampled mutant's file (once per
/// file) and compare its selection with the mutant's killers.
#[must_use]
pub fn score_impact(truth: &ImpactTruth, run: &ImpactRunner) -> ImpactScore {
    let mut answers: BTreeMap<&str, Result<Vec<String>, String>> = BTreeMap::new();
    let mutants = truth
        .mutants
        .iter()
        .map(|mutant| {
            let answer = answers
                .entry(mutant.file.as_str())
                .or_insert_with(|| run(&mutant.file).map(|document| selected(&document)));
            let (selected, error) = match answer {
                Ok(selected) => (selected.as_slice(), None),
                Err(error) => (&[][..], Some(error.clone())),
            };
            let kills = |file: Option<&String>| file.is_some_and(|f| mutant.killers.contains(f));
            MutantResult {
                file: mutant.file.clone(),
                id: mutant.id.clone(),
                test_files: mutant.test_files,
                selected: selected.len() as u64,
                killer_selected: selected.iter().any(|file| kills(Some(file))),
                killer_first: kills(selected.first()),
                baseline_killer_first: kills(
                    truth
                        .baseline
                        .get(&mutant.file)
                        .and_then(|related| related.first()),
                ),
                error,
            }
        })
        .collect();
    ImpactScore { mutants }
}

/// The selected test files of an `impact` JSON document, in rank order.
fn selected(document: &Value) -> Vec<String> {
    let mut rows: Vec<(u64, String)> = document
        .get("selected")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            Some((
                row.get("rank")?.as_u64()?,
                row.get("file")?.as_str()?.to_owned(),
            ))
        })
        .collect();
    rows.sort();
    rows.into_iter().map(|(_, file)| file).collect()
}

/// `untested-hotspot`'s findings as a ranking: files by score descending, then by path.
#[must_use]
pub fn hotspot_ranking(findings: &[ReportedFinding]) -> Vec<String> {
    let mut rows: Vec<(f64, &str)> = findings
        .iter()
        .filter(|finding| finding.rule == "untested-hotspot")
        .filter_map(|finding| {
            let score = finding.evidence.get("score")?.parse::<f64>().ok()?;
            Some((score, finding.file.as_str()))
        })
        .collect();
    rows.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.1.cmp(b.1)));
    rows.into_iter().map(|(_, file)| file.to_owned()).collect()
}

/// One of the rule's top files within the set, with its mutation score.
#[derive(Debug, Clone, PartialEq)]
pub struct HotspotRow {
    /// The source file.
    pub file: String,
    /// Its mutation score in the oracle.
    pub score: f64,
    /// Whether that score is below the set's median.
    pub below_median: bool,
}

/// The hotspot row: the rule's top files within the frozen set against the set's median.
#[derive(Debug, Clone, PartialEq)]
pub struct HotspotScore {
    /// How many files of the set the rule ranked at all.
    pub ranked: usize,
    /// The set's median mutation score.
    pub median: f64,
    /// The first [`HOTSPOT_TOP`] ranked files of the set, in rank order.
    pub top: Vec<HotspotRow>,
}

impl HotspotScore {
    /// `(k, n)`: top files scoring below the median, of the top files.
    #[must_use]
    pub fn below(&self) -> (u64, u64) {
        let k = self.top.iter().filter(|row| row.below_median).count();
        (k as u64, self.top.len() as u64)
    }

    /// PASS when the Wilson 95 % lower bound of [`Self::below`] is above
    /// [`HOTSPOT_LOWER_ABOVE`]; INSUFFICIENT when the rule ranks fewer than [`HOTSPOT_TOP`]
    /// files of the set.
    #[must_use]
    pub fn verdict(&self) -> RuleVerdict {
        let (k, n) = self.below();
        if self.ranked < HOTSPOT_TOP {
            RuleVerdict::Insufficient
        } else if wilson_interval(k, n).low > HOTSPOT_LOWER_ABOVE {
            RuleVerdict::Pass
        } else {
            RuleVerdict::Fail
        }
    }
}

/// Score a hotspot `ranking` (files, highest first) within the oracle's file set: files
/// outside the set are skipped, the first [`HOTSPOT_TOP`] inside it are compared with the
/// median.
#[must_use]
pub fn score_hotspots(truth: &ImpactTruth, ranking: &[String]) -> HotspotScore {
    let scores: BTreeMap<&str, f64> = truth
        .files
        .rows
        .iter()
        .map(|row| (row.file.as_str(), row.score))
        .collect();
    let in_set: Vec<(&String, f64)> = ranking
        .iter()
        .filter_map(|file| Some((file, *scores.get(file.as_str())?)))
        .collect();
    let median = truth.files.median;
    HotspotScore {
        ranked: in_set.len(),
        median,
        top: in_set
            .into_iter()
            .take(HOTSPOT_TOP)
            .map(|(file, score)| HotspotRow {
                file: file.clone(),
                score,
                below_median: score < median,
            })
            .collect(),
    }
}

/// The budget row: `spirolo impact <file>` against `spirolo list`, each timed over its own runs
/// on the same clone.
#[derive(Debug, Clone, PartialEq)]
pub struct ImpactBudget {
    /// The changed file `impact` was timed on.
    pub file: String,
    /// `impact`'s timed runs.
    pub impact: Wallclock,
    /// `list`'s timed runs.
    pub list: Wallclock,
}

impl ImpactBudget {
    /// The allowance over `list`'s median (PRD 0005: `list` + 1 s).
    pub const OVER_LIST_MS: u64 = 1_000;

    /// `impact`'s median, in milliseconds.
    #[must_use]
    pub fn impact_ms(&self) -> u64 {
        millis(self.impact.median())
    }

    /// `list`'s median, in milliseconds.
    #[must_use]
    pub fn list_ms(&self) -> u64 {
        millis(self.list.median())
    }

    /// The busiest idle-box check of the two, in percent, when it exceeded
    /// [`IDLE_BUSY_CEILING`]: the timing is refused.
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // a share in [0, 1]
    pub fn busy_percent(&self) -> Option<u32> {
        [self.impact.busy, self.list.busy]
            .into_iter()
            .flatten()
            .filter(|busy| *busy > IDLE_BUSY_CEILING)
            .max_by(f64::total_cmp)
            .map(|busy| (busy * 100.0).round() as u32)
    }

    /// PASS when `impact`'s median is within `list`'s plus [`Self::OVER_LIST_MS`] on an
    /// idle box; a refused timing is a FAIL, as in the counts section.
    #[must_use]
    pub fn verdict(&self) -> RuleVerdict {
        if self.busy_percent().is_none() && self.impact_ms() <= self.list_ms() + Self::OVER_LIST_MS
        {
            RuleVerdict::Pass
        } else {
            RuleVerdict::Fail
        }
    }
}

/// A duration in whole milliseconds, saturating.
fn millis(elapsed: std::time::Duration) -> u64 {
    u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
}

/// An entry's four M4b rows (PRD 0005, Bench).
#[derive(Debug, Clone, PartialEq)]
pub struct ImpactRows {
    /// Safety and ranking over the mutant sample.
    pub score: ImpactScore,
    /// The hotspot row.
    pub hotspots: HotspotScore,
    /// The budget row.
    pub budget: ImpactBudget,
}

impl ImpactRows {
    /// Each row's name and verdict, in print order.
    #[must_use]
    pub fn verdicts(&self) -> [(&'static str, RuleVerdict); 4] {
        [
            ("safety", self.score.safety_verdict()),
            ("ranking", self.score.ranking_verdict()),
            ("hotspots", self.hotspots.verdict()),
            ("budget", self.budget.verdict()),
        ]
    }

    /// The rows that failed, by name.
    #[must_use]
    pub fn failed(&self) -> Vec<&'static str> {
        self.verdicts()
            .into_iter()
            .filter(|(_, verdict)| *verdict == RuleVerdict::Fail)
            .map(|(row, _)| row)
            .collect()
    }
}

/// The M4b block of an entry in the precision section: one line per row with its counts,
/// Wilson interval and verdict, the hotspot row's top files, and each file `impact` could
/// not answer for.
#[must_use]
pub fn render_impact(rows: &ImpactRows) -> String {
    let mut out = String::new();
    let [safety, ranking, hotspots, budget] = rows.verdicts().map(|(_, verdict)| verdict.tag());
    let score = &rows.score;
    let (k, n) = score.safety();
    let (sub_k, sub_n) = score.safety_subset();
    let _ = writeln!(
        out,
        "  M4b safety    killer selected {k} of {n}  interval {}  packages with ≥{SUBSET_MIN_TEST_FILES} test files {sub_k} of {sub_n}  interval {}  {safety}",
        format_interval(wilson_interval(k, n), n > 0),
        format_interval(wilson_interval(sub_k, sub_n), sub_n > 0),
    );
    let ((top, n), (base, _)) = (score.top1(), score.baseline_top1());
    let _ = writeln!(
        out,
        "  M4b ranking   killer first: impact {top} of {n} ({:.3})  baseline {base} of {n} ({:.3})  median selection {} of the package's test files  {ranking}",
        share(top, n),
        share(base, n),
        score
            .median_share()
            .map_or_else(|| "—".to_owned(), |share| format!("{share:.2}×")),
    );
    let hot = &rows.hotspots;
    let (below, top_n) = hot.below();
    let _ = writeln!(
        out,
        "  M4b hotspots  {} of the set ranked  top {top_n} below the median {:.3}: {below} of {top_n}  interval {}  {hotspots}",
        hot.ranked,
        hot.median,
        format_interval(wilson_interval(below, top_n), top_n > 0),
    );
    for (rank, row) in hot.top.iter().enumerate() {
        let _ = writeln!(
            out,
            "    {:>2}  {:.3}  {}  {}",
            rank + 1,
            row.score,
            if row.below_median { "below" } else { "above" },
            row.file,
        );
    }
    let over = &rows.budget;
    let _ = write!(
        out,
        "  M4b budget    impact {} ms  list {} ms + {} ms  on {}",
        over.impact_ms(),
        over.list_ms(),
        ImpactBudget::OVER_LIST_MS,
        over.file,
    );
    if let Some(busy) = over.busy_percent() {
        let _ = write!(out, "  refused: box {busy}% busy");
    }
    let _ = writeln!(out, "  {budget}");
    let mut seen = Vec::new();
    for mutant in &score.mutants {
        if let Some(error) = &mutant.error {
            if !seen.contains(&&mutant.file) {
                seen.push(&mutant.file);
                let _ = writeln!(out, "    not answered  {}: {error}", mutant.file);
            }
        }
    }
    out
}

/// What [`score_entry_impact`] runs against the entry's clone.
pub struct ImpactEntryRunners<'a> {
    /// `spirolo impact --format json <file>` on the clone.
    pub impact: &'a ImpactRunner<'a>,
    /// `spirolo check --rule untested-hotspot --churn <file>` on the clone, reduced to
    /// findings.
    pub hotspots: &'a dyn Fn(&Path) -> Result<Vec<ReportedFinding>, String>,
    /// `spirolo impact <file>` and `spirolo list` timed on the clone.
    pub budget: &'a dyn Fn(&str) -> Result<ImpactBudget, String>,
}

/// An entry's M4b rows, when it records an impact oracle (`impact/truth.json`).
///
/// `None` without one. The clone must be at the oracle's pin (`head` is its `HEAD`); the hotspot
/// check reads the entry's committed `impact/churn.numstat`, and the budget is timed on the
/// first sampled mutant's file.
///
/// The error says why the rows could not be measured: an unreadable oracle, a clone off the
/// pin, a hotspot check or a timing that failed.
#[must_use]
pub fn score_entry_impact(
    dir: &Path,
    head: &str,
    runners: &ImpactEntryRunners,
) -> Option<Result<ImpactRows, String>> {
    let path = dir.join("impact").join("truth.json");
    if !path.is_file() {
        return None;
    }
    Some((|| {
        let truth = ImpactTruth::read(&path)?;
        if !truth.pin.starts_with(head) && !head.starts_with(&truth.pin) {
            return Err(format!(
                "the clone is at {head}, the impact oracle is pinned at {}",
                truth.pin
            ));
        }
        let findings = (runners.hotspots)(&dir.join("impact").join("churn.numstat"))
            .map_err(|error| format!("hotspot check: {error}"))?;
        let first = truth
            .mutants
            .first()
            .ok_or_else(|| "the impact oracle samples no mutant".to_owned())?;
        let budget = (runners.budget)(&first.file).map_err(|error| format!("budget: {error}"))?;
        Ok(ImpactRows {
            score: score_impact(&truth, runners.impact),
            hotspots: score_hotspots(&truth, &hotspot_ranking(&findings)),
            budget,
        })
    })())
}

/// The ranking verdict on `k` killers ranked first by `impact` and `base` by the baseline,
/// of `n` mutants.
fn ranking(k: u64, base: u64, n: u64) -> RuleVerdict {
    judged(n, || share(k, n) >= RANKING_PASS_AT && k >= base)
}

/// `Insufficient` under [`IMPACT_MIN_N`], otherwise `Pass` when `passes`.
fn judged(n: u64, passes: impl FnOnce() -> bool) -> RuleVerdict {
    if n < IMPACT_MIN_N {
        RuleVerdict::Insufficient
    } else if passes() {
        RuleVerdict::Pass
    } else {
        RuleVerdict::Fail
    }
}

/// `k / n`, 0 for an empty `n`.
#[allow(clippy::cast_precision_loss)] // counts far below 2^52
fn share(k: u64, n: u64) -> f64 {
    if n == 0 {
        0.0
    } else {
        k as f64 / n as f64
    }
}

#[cfg(test)]
mod tests {
    use super::{ranking, RuleVerdict};

    #[test]
    fn ranking_passes_at_seventy_percent_not_below_the_baseline_on_a_hundred() {
        assert_eq!(ranking(70, 70, 100), RuleVerdict::Pass);
        assert_eq!(ranking(69, 0, 100), RuleVerdict::Fail);
        // The baseline ranks a killer first more often: FAIL at any rate.
        assert_eq!(ranking(80, 81, 100), RuleVerdict::Fail);
        assert_eq!(ranking(99, 0, 99), RuleVerdict::Insufficient);
    }
}
