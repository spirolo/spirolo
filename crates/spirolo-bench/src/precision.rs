//! The precision section: join a rule's reported findings to a `truth.json` adjudication
//! sample, compute precision over the adjudicated sampled findings and recall over the
//! recorded positives, print the 95 % Wilson interval, and give each rule a verdict.
//!
//! This is the M2 done-condition expressed as code, the way the counts section is M1's.
//! The library here is exercised with canned `check --format json` documents and canned
//! adjudication arrays, so the join, the deterministic sampling, the statistics and the
//! verdicts are tested without cloning a corpus repo (the binary wires it to the real
//! `spirolo check` and to git-worktree replay).
//!
//! A finding is a true positive only when an adjudication with a `true` verdict matches its
//! identity; a `false` verdict is a false positive; a sampled finding with no adjudication
//! is `open` and never counts as true; an adjudicator that could not decide is a separate
//! column and never counts as true either. Precision is `true / (true + false)` over the
//! adjudicated sampled findings; recall is the recorded positives the run reproduced over
//! all recorded positives — a positive is reproduced when a decided finding of its rule is
//! located in its file (narrowed by its invariants when it names any), the same predicate
//! ([`ReportedFinding::matches`]) a replay pair's present-check applies at each commit.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Write as _};

use serde_json::Value;

use crate::impact::{render_impact, ImpactRows};
use crate::known_flaky::{FixSide, KnownFlakyScore};
use crate::ledger::{render_ledger, LedgerScore};
use crate::m5::{render_m5, M5Rows};
use crate::truth::{Adjudication, Expected, ReplayPair, StratifiedSample, Truth, Verdict};

/// The sample target for a rule that reports at least this many findings; below it, the
/// smaller target is used, and a rule that reports fewer than the smaller target samples
/// all of them. The two targets follow the PRD: 50 where a rule reports that many, 30
/// otherwise.
const SAMPLE_TARGET_LARGE: usize = 50;
/// The sample target for a rule reporting fewer than [`SAMPLE_TARGET_LARGE`] findings.
const SAMPLE_TARGET_SMALL: usize = 30;
/// The precision at or above which a rule with enough adjudications passes.
const PASS_PRECISION: f64 = 0.9;
/// The number of adjudicated findings a rule needs across the corpus before its precision
/// is judged rather than called insufficient.
const MIN_ADJUDICATED: u64 = 20;
/// The z-score for a two-sided 95 % Wilson interval.
const WILSON_Z: f64 = 1.96;

/// One reported finding, reduced to what the precision join needs.
///
/// It carries the rule, the identity (the `id` the report writes, which the adjudication
/// array joins on), and whether it was a decided finding or a "could not decide" row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportedFinding {
    /// The rule that produced it.
    pub rule: String,
    /// The finding's stable identity, joined against `adjudications[].id`.
    pub id: String,
    /// The repo-relative file the finding is located in, so a replay pair can scope its
    /// present-check to the file its bug lived in — a rule that fires broadly (dead-skip
    /// flags every untracked skip) would otherwise read as "present" at the fixed commit on
    /// an unrelated finding elsewhere in the tree.
    pub file: String,
    /// True when the row is a decided finding, false when it is "could not decide". The
    /// replay present-check counts only decided findings: a rule that says "could not
    /// decide" at both commits has not reproduced a shipped bug.
    pub decided: bool,
    /// The title path (suite path then title) of the test the finding is anchored at, as
    /// `check` writes it under `test`; `None` for a finding anchored elsewhere.
    pub test: Option<Vec<String>>,
    /// The finding's scalar evidence rows, key to value, as `check` writes them; a
    /// stratified sample reads a finding's stratum here (`kind` for `flake-candidate`).
    pub evidence: BTreeMap<String, String>,
}

impl ReportedFinding {
    /// Whether this finding is the one a truth row records: a decided finding of `rule`,
    /// located in `file`, whose identity carries one of `invariants`.
    ///
    /// This is the single predicate both a replay pair's present-check (at each commit of
    /// the pair) and a recall positive's reproduced-check (at the pin) apply, so the two
    /// cannot drift. Only a decided finding matches: a "could not decide" row is honesty,
    /// not a reproduced bug. An empty `file` scopes to the whole tree, for a rule whose
    /// pair fires on a single site (handler-drift); a named file must match the finding's
    /// repo-relative path exactly, so a rule that fires broadly (dead-skip flags every
    /// untracked skip) is not read as present on an unrelated finding elsewhere. Empty
    /// `invariants` scope to any finding of the rule in the file; named ones narrow to a
    /// finding whose `id` contains one of them, so a row proving the console guard and
    /// unhandled-request invariants is not matched by a third (mock reset) that legally
    /// still fires in the same setup file. A named `test` narrows to a finding anchored at
    /// exactly that title path, so a candidate on another test of the same file is not the
    /// recorded one; `None` scopes to any finding in the file.
    #[must_use]
    pub fn matches(
        &self,
        rule: &str,
        file: &str,
        invariants: &[String],
        test: Option<&[String]>,
    ) -> bool {
        self.rule == rule
            && self.decided
            && (file.is_empty() || self.file == file)
            && test.is_none_or(|test| self.test.as_deref() == Some(test))
            && (invariants.is_empty()
                || invariants
                    .iter()
                    .any(|invariant| self.id.contains(invariant.as_str())))
    }
}

/// A rule's verdict on the precision it reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleVerdict {
    /// Precision at or above 0.9 on at least the minimum adjudicated findings.
    Pass,
    /// Precision below 0.9 on at least the minimum adjudicated findings.
    Fail,
    /// The corpus supplied fewer than the minimum findings, so no sample can reach the floor.
    Insufficient,
    /// The corpus supplied at least the minimum findings, but fewer than the minimum were
    /// adjudicated by execution: the oracle did not cover the sample. Unlike
    /// [`Self::Insufficient`] this never satisfies the done-condition (PRD 0003, amended
    /// 2026-09-23).
    Unadjudicated,
    /// Nothing reported, so nothing to judge.
    Skipped,
}

impl RuleVerdict {
    /// The tag printed in the table.
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Fail => "FAIL",
            Self::Insufficient => "INSUFFICIENT",
            Self::Unadjudicated => "UNADJUDICATED",
            Self::Skipped => "SKIPPED",
        }
    }
}

/// Where a rule's precision floor is judged: the milestone that owns the floor, and, when
/// that milestone concluded without meeting it, where the floor was carried and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleFloor {
    /// The rule id.
    pub rule: &'static str,
    /// The milestone whose done-condition names the floor.
    pub milestone: &'static str,
    /// Set when the floor was carried to a later milestone: the rule is still measured and
    /// printed, but its verdict does not gate the bench.
    pub carried: Option<Carried>,
}

/// A floor carried forward from a milestone that concluded without meeting it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Carried {
    /// The milestone the floor now belongs to.
    pub to: &'static str,
    /// Why the floor does not gate now, printed on the carried line.
    pub why: &'static str,
}

/// Why M3's hygiene floors do not gate the bench (docs/roadmap.md §M3, "Concluded").
const M3_CARRIED: Carried = Carried {
    to: "M6",
    why: "M3 concluded not passed 2026-09-24, its floors carried to M6 (docs/roadmap.md §M3)",
};

/// Why M4's `flake-candidate` floors, precision and known-flaky recall, do not gate the
/// bench (docs/roadmap.md §M4, the decision on #184).
const M4_CARRIED: Carried = Carried {
    to: "M6",
    why: "M4 concluded not passed 2026-09-26, flake-candidate's floors carried to M6 (docs/roadmap.md §M4)",
};

/// Why M4b's hotspots floor (`untested-hotspot`) does not gate the bench (docs/roadmap.md
/// §M4b, #235). The other M4b rows passed at the close and keep gating.
const M4B_CARRIED: Carried = Carried {
    to: "M6",
    why: "M4b concluded not passed 2026-09-27, the hotspots floor carried to M6 on a fresh file set (docs/roadmap.md §M4b, #235)",
};

/// The M4b row whose floor [`M4B_CARRIED`] carries, and the rule it measures.
const M4B_CARRIED_ROW: (&str, &str) = ("hotspots", "untested-hotspot");

/// Which milestone owns each rule's floor, in registry order.
///
/// The bench owns this table, not the rule crates: a rule does not decide whether it
/// gates. A rule missing from the table gates, so a new rule is judged until someone
/// records otherwise.
pub const RULE_FLOORS: &[RuleFloor] = &[
    RuleFloor {
        rule: "weak-assertion",
        milestone: "M2",
        carried: None,
    },
    RuleFloor {
        rule: "absence-anchor",
        milestone: "M2",
        carried: None,
    },
    RuleFloor {
        rule: "data-layer-mock",
        milestone: "M2",
        carried: None,
    },
    RuleFloor {
        rule: "handler-drift",
        milestone: "M2",
        carried: None,
    },
    RuleFloor {
        rule: "generator-walk",
        milestone: "M2",
        carried: None,
    },
    RuleFloor {
        rule: "fail-closed-setup",
        milestone: "M2",
        carried: None,
    },
    RuleFloor {
        rule: "gate-truth",
        milestone: "M2",
        carried: None,
    },
    RuleFloor {
        rule: "dead-skip",
        milestone: "M2",
        carried: None,
    },
    RuleFloor {
        rule: "duplicate-test",
        milestone: "M3",
        carried: Some(M3_CARRIED),
    },
    RuleFloor {
        rule: "redundant-test",
        milestone: "M3",
        carried: Some(M3_CARRIED),
    },
    RuleFloor {
        rule: "unused-mock",
        milestone: "M3",
        carried: Some(M3_CARRIED),
    },
    RuleFloor {
        rule: "orphan-test",
        milestone: "M3",
        carried: Some(M3_CARRIED),
    },
    RuleFloor {
        rule: "stale-snapshot",
        milestone: "M3",
        carried: Some(M3_CARRIED),
    },
    RuleFloor {
        rule: "flake-candidate",
        milestone: "M4",
        carried: Some(M4_CARRIED),
    },
];

/// Where `rule`'s floor was carried, or `None` when it gates (including a rule `floors`
/// does not list).
fn carried(floors: &[RuleFloor], rule: &str) -> Option<Carried> {
    floors
        .iter()
        .find(|floor| floor.rule == rule)
        .and_then(|floor| floor.carried)
}

/// A closed 95 % Wilson score interval on a proportion.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    /// The lower bound, clamped to `[0, 1]`.
    pub low: f64,
    /// The upper bound, clamped to `[0, 1]`.
    pub high: f64,
}

/// The 95 % Wilson score interval for `successes` out of `total`.
///
/// The Wilson interval, not the normal approximation: it stays inside `[0, 1]` and is
/// honest at the small samples M2 gates on (27 of 30, 45 of 50). `total == 0` yields
/// `[0, 0]` — no data, no claim.
#[must_use]
pub fn wilson_interval(successes: u64, total: u64) -> Interval {
    if total == 0 {
        return Interval {
            low: 0.0,
            high: 0.0,
        };
    }
    #[allow(clippy::cast_precision_loss)]
    let n = total as f64;
    #[allow(clippy::cast_precision_loss)]
    let p = successes as f64 / n;
    let z = WILSON_Z;
    let z2 = z * z;
    let denom = 1.0 + z2 / n;
    let centre = p + z2 / (2.0 * n);
    let margin = z * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt();
    Interval {
        low: ((centre - margin) / denom).clamp(0.0, 1.0),
        high: ((centre + margin) / denom).clamp(0.0, 1.0),
    }
}

/// Deterministically sample a rule's reported findings down to a target size.
///
/// The findings are sorted by identity first, so two runs on two machines see the same
/// order; then the finding at index `floor(i * n / target)` is taken for each `i` in
/// `0..target`, which spreads the sample evenly across the sorted list rather than
/// clustering it at the front and yields exactly `min(n, target)` findings — a stride of
/// `ceil(n / target)` undershot (215 → 43, 31 → 16) and could keep a rule below the
/// 20-adjudicated floor however many findings it reported. A rule reporting at most the
/// target returns all of its findings, unchanged in order. The target is 50 when the rule
/// reports at least 50, else 30, else all — the PRD's rule. The corpus adjudicators'
/// `lib.mjs::sample` applies the same arithmetic, so the two sample the same set.
#[must_use]
pub fn sample(mut findings: Vec<ReportedFinding>) -> Vec<ReportedFinding> {
    findings.sort_by(|a, b| a.id.cmp(&b.id));
    let n = findings.len();
    let target = target_for(n);
    if n <= target {
        return findings;
    }
    // The kept indices are `i * n / target` on `usize` (no overflow: `n * target` is far
    // below `usize::MAX` for any report that fits in memory). With `n > target` they are
    // strictly increasing, so each `i` keeps a distinct finding and the walk below keeps
    // exactly `target` of them.
    let mut taken = 0;
    findings
        .into_iter()
        .enumerate()
        .filter(|(index, _)| {
            let keep = taken < target && *index == taken * n / target;
            taken += usize::from(keep);
            keep
        })
        .map(|(_, finding)| finding)
        .collect()
}

/// A rule's findings sampled by stratum (#203), with one [`Stratum`] row per stratum `spec`
/// names, in its order.
///
/// A finding's stratum is its `spec.by` evidence value; inside a stratum the findings are
/// drawn in the order of their [`seeded_rank`] under `spec.seed`, ties by id. With `k` strata that hold findings,
/// each gives up to `ceil(spec.target / k)` of them. When that leaves fewer than
/// `spec.target` rows an executed oracle can cover (`executes`: a stratum short of its quota,
/// or quota rows under a never-executed directory), the sample is topped up one executable
/// row per stratum in turn, in `spec` order, until it reaches the target or no stratum has
/// one left; a never-executed row is never a top-up. A `retired` finding counts as reported
/// but is never drawn: its rows were already adjudicated and retired.
#[must_use]
pub fn stratified_sample(
    findings: Vec<ReportedFinding>,
    spec: &StratifiedSample,
    retired: &BTreeSet<&str>,
    executes: &dyn Fn(&ReportedFinding) -> bool,
) -> (Vec<ReportedFinding>, Vec<Stratum>) {
    let mut by_stratum: BTreeMap<&str, Vec<ReportedFinding>> = spec
        .strata
        .iter()
        .map(|stratum| (stratum.as_str(), Vec::new()))
        .collect();
    for finding in findings {
        let stratum = finding.evidence.get(&spec.by).map(String::as_str);
        if let Some(members) = stratum.and_then(|stratum| by_stratum.get_mut(stratum)) {
            members.push(finding);
        }
    }
    let with_findings = by_stratum
        .values()
        .filter(|members| !members.is_empty())
        .count();
    let quota = spec.target.div_ceil(with_findings.max(1));

    // Per stratum: its row, the findings drawn, and the executable ones left to top up from.
    let mut strata: Vec<(Stratum, Vec<ReportedFinding>, Vec<ReportedFinding>)> = Vec::new();
    for stratum in &spec.strata {
        let mut members = by_stratum.remove(stratum.as_str()).unwrap_or_default();
        members.sort_by_cached_key(|f| (seeded_rank(spec.seed, &f.id), f.id.clone()));
        let row = Stratum {
            value: stratum.clone(),
            reported: members.len() as u64,
            executable: members.iter().filter(|f| executes(f)).count() as u64,
            sampled: 0,
            sampled_executable: 0,
            truths: 0,
            falses: 0,
            open: 0,
        };
        members.retain(|finding| !retired.contains(finding.id.as_str()));
        let rest = members.split_off(quota.min(members.len()));
        let spare = rest.into_iter().filter(|f| executes(f)).rev().collect();
        strata.push((row, members, spare));
    }
    let mut executable = strata
        .iter()
        .flat_map(|(_, drawn, _)| drawn)
        .filter(|f| executes(f))
        .count();
    while executable < spec.target {
        let mut topped = false;
        for (_, drawn, spare) in &mut strata {
            if executable == spec.target {
                break;
            }
            if let Some(finding) = spare.pop() {
                drawn.push(finding);
                executable += 1;
                topped = true;
            }
        }
        if !topped {
            break;
        }
    }

    let mut sampled = Vec::new();
    let mut rows = Vec::new();
    for (mut row, drawn, _) in strata {
        row.sampled = drawn.len() as u64;
        row.sampled_executable = drawn.iter().filter(|f| executes(f)).count() as u64;
        rows.push(row);
        sampled.extend(drawn);
    }
    (sampled, rows)
}

/// A rule's precision sample: stratified when the entry's truth says so (#203), with its
/// stratum rows, else [`sample`]'s even stride with none.
fn sample_rule(
    decided: Vec<ReportedFinding>,
    rule: &str,
    truth: &Truth,
) -> (Vec<ReportedFinding>, Vec<Stratum>) {
    match truth.stratified_samples.get(rule) {
        Some(spec) => stratified_sample(decided, spec, &truth.retired_ids(rule), &|f| {
            truth.executes_for(rule, &f.file)
        }),
        None => (sample(decided), Vec::new()),
    }
}

fn sample_with_draw(
    decided: Vec<ReportedFinding>,
    rule: &str,
    truth: &Truth,
    draw: Option<&BTreeMap<String, BTreeSet<String>>>,
) -> (Vec<ReportedFinding>, Vec<Stratum>) {
    let (mut sampled, mut strata) = sample_rule(decided.clone(), rule, truth);
    if let Some(draw) = draw {
        sampled = decided
            .into_iter()
            .filter(|f| draw.get(rule).is_some_and(|ids| ids.contains(&f.id)))
            .collect();
        if let Some(spec) = truth.stratified_samples.get(rule) {
            for stratum in &mut strata {
                let drawn: Vec<_> = sampled
                    .iter()
                    .filter(|f| f.evidence.get(&spec.by) == Some(&stratum.value))
                    .collect();
                stratum.sampled = drawn.len() as u64;
                stratum.sampled_executable = drawn
                    .iter()
                    .filter(|f| truth.executes_for(rule, &f.file))
                    .count() as u64;
            }
        }
    }
    (sampled, strata)
}

/// Count a drawn `finding`'s verdict on its stratum's line: an executed true or false, and
/// anything else (no verdict, `open`, refused by the join, by inspection) as open.
fn count_in_stratum(
    strata: &mut [Stratum],
    spec: &StratifiedSample,
    finding: &ReportedFinding,
    executed_verdict: Option<Verdict>,
) {
    let value = finding.evidence.get(&spec.by);
    if let Some(stratum) = strata.iter_mut().find(|s| Some(&s.value) == value) {
        match executed_verdict {
            Some(Verdict::True) => stratum.truths += 1,
            Some(Verdict::False) => stratum.falses += 1,
            _ => stratum.open += 1,
        }
    }
}

/// A finding's place in a seeded draw: the 32-bit FNV-1a hash of `"<seed>:<id>"` in UTF-8.
/// The adjudicators' `lib.mjs::seededRank` computes the same number, so both sides draw the
/// same rows.
pub fn seeded_rank(seed: u64, id: &str) -> u32 {
    format!("{seed}:{id}")
        .bytes()
        .fold(0x811c_9dc5_u32, |hash, byte| {
            (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)
        })
}

/// The sample target for a rule that reported `reported` findings.
const fn target_for(reported: usize) -> usize {
    if reported >= SAMPLE_TARGET_LARGE {
        SAMPLE_TARGET_LARGE
    } else {
        SAMPLE_TARGET_SMALL
    }
}

/// One stratum's line under a rule whose sample is stratified (#203): a `flake-candidate`
/// kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stratum {
    /// The stratum's evidence value (the kind).
    pub value: String,
    /// Decided findings the run reported in it.
    pub reported: u64,
    /// Of those, the findings an executed oracle could cover.
    pub executable: u64,
    /// Of those reported, the findings the sample drew.
    pub sampled: u64,
    /// Of those drawn, the executable ones.
    pub sampled_executable: u64,
    /// Drawn rows an executed adjudication found true.
    pub truths: u64,
    /// Drawn rows an executed adjudication found false.
    pub falses: u64,
    /// Drawn rows with no executed true or false verdict: not adjudicated, adjudicated
    /// `open`, or refused by the join.
    pub open: u64,
}

/// One rule's precision line within an entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RulePrecision {
    /// The rule name.
    pub rule: String,
    /// Findings the run reported for this rule.
    pub reported: u64,
    /// Of [`Self::reported`], the findings an executed oracle could cover: those on an entry
    /// that executes its tests and under none of its `never_executed` directories. Only these count toward the 20 that make a rule
    /// `UNADJUDICATED`; a finding no oracle can run is the corpus being too small.
    pub executable: u64,
    /// Findings kept in the deterministic sample.
    pub sampled: u64,
    /// Sampled findings adjudicated `true`.
    pub truths: u64,
    /// Sampled findings adjudicated `false`.
    pub falses: u64,
    /// Sampled findings with no adjudication (never counted as true).
    pub open: u64,
    /// Sampled findings an adjudicator could not decide (a separate column, never true).
    pub could_not_decide: u64,
    /// Sampled findings adjudicated `true` by inspection, on an entry whose tests are not
    /// executed (`Truth::execute == false`). Counted apart: never in precision, the
    /// interval or the floor.
    pub inspected_truths: u64,
    /// Sampled findings adjudicated `false` by inspection, on an entry whose tests are not
    /// executed. Counted apart, like [`Self::inspected_truths`].
    pub inspected_falses: u64,
    /// Recorded positives this run reproduced.
    pub recalled: u64,
    /// Recorded positives in truth for this rule.
    pub positives: u64,
    /// The verdict on the precision reached.
    pub verdict: RuleVerdict,
    /// One row per stratum when the rule's sample is stratified (#203), else empty.
    pub strata: Vec<Stratum>,
}

impl RulePrecision {
    /// Precision over the adjudicated sampled findings, `None` when none were adjudicated.
    #[must_use]
    pub fn precision(&self) -> Option<f64> {
        let adjudicated = self.truths + self.falses;
        if adjudicated == 0 {
            None
        } else {
            #[allow(clippy::cast_precision_loss)]
            Some(self.truths as f64 / adjudicated as f64)
        }
    }

    /// Recall over the recorded positives, `None` when truth records none for this rule.
    #[must_use]
    pub fn recall(&self) -> Option<f64> {
        if self.positives == 0 {
            None
        } else {
            #[allow(clippy::cast_precision_loss)]
            Some(self.recalled as f64 / self.positives as f64)
        }
    }

    /// The 95 % Wilson interval on this rule's precision.
    #[must_use]
    pub fn interval(&self) -> Interval {
        wilson_interval(self.truths, self.truths + self.falses)
    }
}

/// A recall floor: the share of a recorded set a rule must reproduce.
///
/// It is judged only when the set has at least a minimum number of rows. Below the minimum the verdict is
/// [`RuleVerdict::Insufficient`], whatever the share, since a handful of rows cannot
/// support a rate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RecallFloor {
    /// The smallest set the floor judges.
    pub min_n: u64,
    /// The share of the set, k/n, at or above which the floor passes.
    pub pass_at: f64,
}

/// The rule a known-flaky set is scored against, whose floor its recall shares.
const KNOWN_FLAKY_RULE: &str = "flake-candidate";

/// The floor for a known-flaky set (PRD 0004): half the rows flagged, on at least 10.
pub const KNOWN_FLAKY_FLOOR: RecallFloor = RecallFloor {
    min_n: 10,
    pass_at: 0.5,
};

impl RecallFloor {
    /// The verdict on `k` reproduced of `n` recorded.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // counts far below 2^52
    pub fn verdict(&self, k: u64, n: u64) -> RuleVerdict {
        if n < self.min_n || n == 0 {
            RuleVerdict::Insufficient
        } else if k as f64 / n as f64 >= self.pass_at {
            RuleVerdict::Pass
        } else {
            RuleVerdict::Fail
        }
    }
}

/// A replay pair's outcome: whether the finding was present at each commit of the pair,
/// and whether the runner could answer at all.
///
/// A pair passes when the finding is present at the shipped commit and absent at the fixed
/// one — the disagreement is proven by the history. It is skipped when the finding is absent
/// at *both* commits: the rule it proves is not yet implemented (an implemented rule would
/// fire at the shipped commit), so it cannot mask a regression. Otherwise it fails. A pair
/// the runner could not check at either end (a missing commit, a worktree that would not
/// create, a `check` that produced no document) carries the runner's message in `error`
/// and fails whatever the two booleans say: an un-checkable pair is never a skip, since
/// "absent at both" would otherwise be indistinguishable from "could not look".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayResult {
    /// The rule the pair proves.
    pub rule: String,
    /// A short anchor for the pair, from the truth entry.
    pub anchor: String,
    /// Whether the finding was present at `shipped_commit` (it should be). An end the
    /// runner errored at records the failing answer here, `false`.
    pub present_at_shipped: bool,
    /// Whether the finding was present at `fixed_commit` (it should not be). An end the
    /// runner errored at records the failing answer here, `true`.
    pub present_at_fixed: bool,
    /// The runner's error, when it could not check an end: the message of whichever end
    /// failed first, both messages joined when both did, each prefixed with its end. `Some`
    /// makes the pair a failure.
    pub error: Option<String>,
}

impl ReplayResult {
    /// A pair passes when the finding is present at the shipped commit and absent at the
    /// fixed one, and the runner answered at both ends.
    #[must_use]
    pub const fn passed(&self) -> bool {
        self.error.is_none() && self.present_at_shipped && !self.present_at_fixed
    }

    /// A pair is skipped, not failed, when the finding is absent at *both* commits: the rule
    /// it proves is not yet implemented, so it fires nowhere. A pair whose rule lands
    /// (slices #36, #39, #42) then flips to PASS or FAIL on the evidence. A runner error is
    /// never a skip.
    #[must_use]
    pub const fn skipped(&self) -> bool {
        self.error.is_none() && !self.present_at_shipped && !self.present_at_fixed
    }

    /// A pair fails when the runner errored at either end, or when it is neither passing
    /// nor skipped — present where it should be absent, or absent at the shipped commit
    /// while present at the fixed one.
    #[must_use]
    pub const fn failed(&self) -> bool {
        !self.passed() && !self.skipped()
    }

    /// The tag printed for the pair.
    const fn tag(&self) -> &'static str {
        if self.passed() {
            "PASS"
        } else if self.skipped() {
            "SKIPPED"
        } else {
            "FAIL"
        }
    }
}

/// A defect in the adjudication join that is printed, not scored.
///
/// The section stays the colour its rows give it, but the line makes the defect visible
/// instead of letting it vanish from the join. Rendered by its [`fmt::Display`] after the
/// entry's rule rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JoinWarning {
    /// A `truth.json` adjudication whose `id` matches no finding the run reported, decided
    /// or not — the finding it adjudicated has moved, been renamed or disappeared since
    /// the adjudication was written, so its verdict counts for nothing.
    UnmatchedAdjudication {
        /// The rule the adjudication names.
        rule: String,
        /// The id that matched nothing.
        id: String,
    },
    /// An adjudication whose `rule` is not the rule of the sampled finding its `id`
    /// joined to — a verdict written for one rule cannot be evidence for another, so the
    /// finding is counted `open` and the verdict is not counted at all.
    RuleMismatch {
        /// The finding id both sides share.
        id: String,
        /// The rule the adjudication names.
        adjudication_rule: String,
        /// The rule the sampled finding belongs to.
        finding_rule: String,
    },
    /// An adjudication made on another substrate than the one the entry records and is
    /// measured on (decision 3 of the M3 reopen): whether a finding exists, and what its
    /// oracle says, depends on the installed dependencies, so the verdict is refused and
    /// the finding is counted `open`.
    SubstrateMismatch {
        /// The finding id both sides share.
        id: String,
        /// The substrate the adjudication records, `None` when it records none.
        adjudication_substrate: Option<String>,
        /// The substrate the entry records, `None` when it records none.
        entry_substrate: Option<String>,
    },
}

impl fmt::Display for JoinWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnmatchedAdjudication { rule, id } => {
                write!(f, "unmatched adjudication: {rule} {id}")
            }
            Self::RuleMismatch {
                id,
                adjudication_rule,
                finding_rule,
            } => write!(
                f,
                "rule mismatch on {id}: adjudication says {adjudication_rule}, finding says {finding_rule}"
            ),
            Self::SubstrateMismatch {
                id,
                adjudication_substrate,
                entry_substrate,
            } => write!(
                f,
                "substrate mismatch on {id}: adjudication made on {}, entry measured on {}",
                adjudication_substrate.as_deref().unwrap_or("an unrecorded substrate"),
                entry_substrate.as_deref().unwrap_or("an unrecorded substrate"),
            ),
        }
    }
}

/// The precision section for one corpus entry: a line per rule, the join's warnings, and
/// a line per replay pair.
#[derive(Debug, Clone, PartialEq)]
pub struct PrecisionReport {
    /// The entry name.
    pub name: String,
    /// One row per rule that ran, in the order the harness was given them.
    pub rules: Vec<RulePrecision>,
    /// The join's defects: rule mismatches in rule-then-sample order, then the unmatched
    /// adjudications in `truth.json` order — never a failure, always printed.
    pub warnings: Vec<JoinWarning>,
    /// One result per recorded replay pair.
    pub replays: Vec<ReplayResult>,
    /// Why the entry's rules were not measured, when they were not: a clone off the
    /// substrate its `truth.json` records reports another population, so it gets no rows.
    pub not_measured: Option<String>,
    /// The entry's `known-flaky.json` scored against `flake-candidate`, when it records one.
    pub known_flaky: Option<KnownFlakyScore>,
    /// The entry's run ledger scored against the oracle's set, when its truth records one;
    /// `Err` when `spirolo` could not be run over the staged runs.
    pub ledger: Option<Result<LedgerScore, String>>,
    /// M4b: the entry's `impact` rows against its `impact/truth.json`, or why they could not
    /// be measured; `None` when the entry records no impact oracle.
    pub impact: Option<Result<ImpactRows, String>>,
    /// M5: the entry's surface rows (`spirolo lsp`, `explain`) against a fresh `check`, or why
    /// they could not be measured; `None` when the entry records no M5 session.
    pub m5: Option<Result<M5Rows, String>>,
}

/// Join a run's reported findings and a truth's adjudications into per-rule precision.
///
/// `reported` is every finding `spirolo check` produced for the entry; `rules` is the rule
/// order the section prints in (a rule with no reported findings still gets a `SKIPPED`
/// row). Sampling is deterministic; the join is by finding identity. A rule's verdict is
/// decided here per entry on this entry's adjudicated count, then re-decided across the
/// corpus by [`corpus_verdicts`] once every entry is in. An adjudication whose id matches
/// no reported finding, or whose rule is not the sampled finding's, is reported as a
/// [`JoinWarning`], not dropped: the mismatched one leaves its finding `open`.
///
/// On an entry whose tests are not executed (`truth.execute == false`) every `true` and
/// `false` verdict is by inspection: it is routed to
/// [`RulePrecision::inspected_truths`] / [`RulePrecision::inspected_falses`] and never to
/// `truths` / `falses`, so it enters neither precision nor the floor. Such an entry's
/// per-entry verdict is therefore `Insufficient` whenever the rule reported anything:
/// its findings are not executable, so they never make a rule `Unadjudicated`,
/// and nothing executed was adjudicated, so never `Pass` or `Fail`.
#[must_use]
pub fn evaluate_entry(
    name: impl Into<String>,
    rules: &[String],
    reported: &[ReportedFinding],
    truth: &Truth,
) -> PrecisionReport {
    evaluate_entry_with_incident(name, rules, reported, &[], truth)
}

/// [`evaluate_entry`], additionally measuring the entry's incident-ref recall positives.
///
/// A recall positive that names a `present_commit` (the recorded incident ref) is not present
/// at the pin — its bug lives on a stranded branch the pin does not carry — so it is measured
/// against `incident_reported`: the findings the run reported when that commit was checked out
/// with its `inputs/` staged. Every other positive is measured against `reported` at the pin,
/// exactly as before. The two feed one recall figure per rule, so `redundant-test`'s recall is
/// over its three re-labelled incident pairs and prints with no floor. When `incident_reported`
/// is empty (no incident, or the run never checked one out) an incident positive is simply not
/// reproduced.
#[must_use]
pub fn evaluate_entry_with_incident(
    name: impl Into<String>,
    rules: &[String],
    reported: &[ReportedFinding],
    incident_reported: &[ReportedFinding],
    truth: &Truth,
) -> PrecisionReport {
    evaluate_with_draw(name.into(), rules, reported, incident_reported, truth, None)
}

/// Join precision using an optional corpus-wide draw instead of the legacy entry sample.
pub fn evaluate_with_draw(
    name: String,
    rules: &[String],
    reported: &[ReportedFinding],
    incident_reported: &[ReportedFinding],
    truth: &Truth,
    draw: Option<&BTreeMap<String, BTreeSet<String>>>,
) -> PrecisionReport {
    let adjudications = index_adjudications(&truth.adjudications);
    let positives = positives_by_rule(truth, reported, incident_reported);
    let mut warnings = Vec::new();

    let mut rows = Vec::new();
    for rule in rules {
        // A "could not decide" finding is the rule's honesty, not a claim of a drift, so it is
        // counted apart and never sampled for precision — a conservative rule is not penalised
        // for the URLs it declined to judge.
        let (decided, undecided): (Vec<ReportedFinding>, Vec<ReportedFinding>) = reported
            .iter()
            .filter(|f| &f.rule == rule)
            .cloned()
            .partition(|f| f.decided);
        let reported_count = decided.len() as u64;
        let executable = decided
            .iter()
            .filter(|f| truth.executes_for(rule, &f.file))
            .count() as u64;
        let (sampled, mut strata) = sample_with_draw(decided, rule, truth, draw);

        let (mut truths, mut falses, mut open) = (0, 0, 0);
        let (mut inspected_truths, mut inspected_falses) = (0, 0);
        let mut could_not_decide = undecided.len() as u64;
        for finding in &sampled {
            let mut executed_verdict = None;
            let tally = match adjudications.get(finding.id.as_str()) {
                // A verdict written for another rule is no evidence for this one: the
                // finding stays open and the disagreement is printed.
                Some(adjudication) if adjudication.rule != *rule => {
                    warnings.push(JoinWarning::RuleMismatch {
                        id: finding.id.clone(),
                        adjudication_rule: adjudication.rule.clone(),
                        finding_rule: rule.clone(),
                    });
                    &mut open
                }
                // A verdict made on another substrate judged another population: refused.
                Some(adjudication) if adjudication.substrate != truth.substrate => {
                    warnings.push(JoinWarning::SubstrateMismatch {
                        id: finding.id.clone(),
                        adjudication_substrate: adjudication.substrate.clone(),
                        entry_substrate: truth.substrate.clone(),
                    });
                    &mut open
                }
                // An inspection verdict (the entry never runs its tests) is its own column.
                Some(adjudication) => match (adjudication.verdict, truth.execute) {
                    (Verdict::True, true) => {
                        executed_verdict = Some(Verdict::True);
                        &mut truths
                    }
                    (Verdict::False, true) => {
                        executed_verdict = Some(Verdict::False);
                        &mut falses
                    }
                    (Verdict::True, false) => &mut inspected_truths,
                    (Verdict::False, false) => &mut inspected_falses,
                    (Verdict::Open, _) => &mut could_not_decide,
                },
                None => &mut open,
            };
            *tally += 1;
            if let Some(spec) = truth.stratified_samples.get(rule) {
                count_in_stratum(&mut strata, spec, finding, executed_verdict);
            }
        }

        let (recalled, rule_positives) = positives.get(rule).copied().unwrap_or((0, 0));
        rows.push(RulePrecision {
            rule: rule.clone(),
            reported: reported_count,
            executable,
            sampled: sampled.len() as u64,
            truths,
            falses,
            open,
            could_not_decide,
            inspected_truths,
            inspected_falses,
            recalled,
            positives: rule_positives,
            // A per-entry verdict is provisional; the corpus pass re-decides on the sum.
            // Inspection verdicts are not in `truths`/`falses` and its findings are not
            // executable, so an execute=false entry is at most INSUFFICIENT here.
            verdict: verdict(reported_count, executable, truths, falses),
            strata,
        });
    }

    // An adjudication that joins to nothing the run reported (decided or not) is a verdict
    // on a finding that no longer exists under that id: say so, in truth.json order.
    let reported_ids: BTreeSet<&str> = reported.iter().map(|f| f.id.as_str()).collect();
    warnings.extend(
        truth
            .adjudications
            .iter()
            .filter(|adjudication| !reported_ids.contains(adjudication.id.as_str()))
            .map(|adjudication| JoinWarning::UnmatchedAdjudication {
                rule: adjudication.rule.clone(),
                id: adjudication.id.clone(),
            }),
    );

    PrecisionReport {
        name,
        rules: rows,
        warnings,
        replays: Vec::new(),
        known_flaky: None,
        ledger: None,
        impact: None,
        m5: None,
        not_measured: None,
    }
}

/// The verdict for a rule given what it reported and how its sample adjudicated.
///
/// `SKIPPED` when nothing was reported; `INSUFFICIENT` when fewer than 20 of the reported
/// findings are `executable` (on an entry that executes its tests: a finding no executed
/// oracle can cover is the corpus being too small); `UNADJUDICATED` when at least
/// 20 executable findings were reported but fewer than 20 adjudicated;
/// `PASS` at precision ≥ 0.9; `FAIL` below. The `open` and could-not-decide
/// findings are not in `truths + falses`, so honesty is never scored as a false positive.
#[must_use]
pub fn verdict(reported: u64, executable: u64, truths: u64, falses: u64) -> RuleVerdict {
    if reported == 0 {
        return RuleVerdict::Skipped;
    }
    let adjudicated = truths + falses;
    if adjudicated < MIN_ADJUDICATED {
        return if executable < MIN_ADJUDICATED {
            RuleVerdict::Insufficient
        } else {
            RuleVerdict::Unadjudicated
        };
    }
    #[allow(clippy::cast_precision_loss)]
    let precision = truths as f64 / adjudicated as f64;
    if precision >= PASS_PRECISION {
        RuleVerdict::Pass
    } else {
        RuleVerdict::Fail
    }
}

/// The corpus-wide verdict per rule, summing adjudications across every entry.
///
/// The PRD gates on 20 adjudicated findings *across the corpus*, not per entry, so a rule
/// under-sampled on each entry can still reach a verdict on the sum. Only executed
/// verdicts (`truths` / `falses`) are summed; inspection verdicts stay out of the floor.
/// Returns the rules in the order first seen, each with its summed verdict.
#[must_use]
pub fn corpus_verdicts(reports: &[PrecisionReport]) -> Vec<(String, RuleVerdict)> {
    let mut order = Vec::new();
    let mut totals: BTreeMap<String, (u64, u64, u64, u64)> = BTreeMap::new();
    for report in reports {
        for row in &report.rules {
            let entry = totals.entry(row.rule.clone()).or_insert_with(|| {
                order.push(row.rule.clone());
                (0, 0, 0, 0)
            });
            entry.0 += row.reported;
            entry.1 += row.executable;
            entry.2 += row.truths;
            entry.3 += row.falses;
        }
    }
    order
        .into_iter()
        .map(|rule| {
            let (reported, executable, truths, falses) = totals[&rule];
            (rule, verdict(reported, executable, truths, falses))
        })
        .collect()
}

/// Every finding a `check --format json` document reported, reduced to `(rule, id)`.
///
/// The harness reads the same `findings` array any consumer of `check` gets and keeps only
/// the rule and the identity — the two things the precision join needs. A malformed row
/// (no `rule` or no `id`) is dropped rather than guessed at.
#[must_use]
pub fn reported_findings(check_json: &Value) -> Vec<ReportedFinding> {
    check_json
        .get("findings")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter_map(|finding| {
            let rule = finding.get("rule").and_then(Value::as_str)?;
            let id = finding.get("id").and_then(Value::as_str)?;
            let file = finding
                .get("file")
                .and_then(Value::as_str)
                .unwrap_or_default();
            // A row with no `outcome` predates the field or is a decided finding; the
            // explicit `could_not_decide` token marks an undecided row. A `listed` row is
            // an inventory line (a skip and its condition), not a finding at all, so it
            // never enters the count, the sample or the could-not-decide column.
            let outcome = finding.get("outcome").and_then(Value::as_str);
            if outcome == Some("listed") {
                return None;
            }
            let decided = outcome != Some("could_not_decide");
            Some(ReportedFinding {
                rule: rule.to_owned(),
                id: id.to_owned(),
                file: file.to_owned(),
                decided,
                test: finding.get("test").and_then(title_path),
                evidence: evidence(finding),
            })
        })
        .collect()
}

/// A `check` finding's scalar `evidence` rows (`[{key, value}]`) as a map; a row with no
/// key or a non-scalar value is left out rather than guessed at.
fn evidence(finding: &Value) -> BTreeMap<String, String> {
    finding
        .get("evidence")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter_map(|row| {
            let key = row.get("key")?.as_str()?;
            let value = match row.get("value")? {
                Value::String(value) => value.clone(),
                value @ (Value::Number(_) | Value::Bool(_)) => value.to_string(),
                _ => return None,
            };
            Some((key.to_owned(), value))
        })
        .collect()
}

/// A `check` finding's `test` object (`suite_path` then `title`) as one title path; a
/// malformed object reads as no test rather than a guessed one.
fn title_path(test: &Value) -> Option<Vec<String>> {
    let mut path: Vec<String> = test
        .get("suite_path")?
        .as_array()?
        .iter()
        .map(|suite| suite.as_str().map(str::to_owned))
        .collect::<Option<_>>()?;
    path.push(test.get("title")?.as_str()?.to_owned());
    Some(path)
}

/// Whether the precision section is red.
///
/// Red when any gating rule with enough adjudications failed, any gating rule with enough
/// findings went unadjudicated, any replay pair failed, a known-flaky set failed, or a
/// ledger failed. A rule whose floor `floors` carries to a later milestone is printed, never
/// gated ([`RULE_FLOORS`] is the committed table); the known-flaky recall is
/// `flake-candidate`'s floor and is carried with it.
#[must_use]
pub fn section_failed(reports: &[PrecisionReport], floors: &[RuleFloor]) -> bool {
    corpus_verdicts(reports)
        .iter()
        .filter(|(rule, _)| carried(floors, rule).is_none())
        .any(|(_, verdict)| matches!(verdict, RuleVerdict::Fail | RuleVerdict::Unadjudicated))
        || reports
            .iter()
            .flat_map(|report| report.replays.iter())
            .any(ReplayResult::failed)
        || (carried(floors, KNOWN_FLAKY_RULE).is_none()
            && reports
                .iter()
                .filter_map(|report| report.known_flaky.as_ref())
                .any(|score| score.verdict() == RuleVerdict::Fail))
        || reports.iter().any(ledger_failed)
        || reports.iter().any(impact_failed)
        || reports.iter().any(|report| !m5_failed(report).is_empty())
}

/// An entry's failed M5 rows, `["error"]` when they could not be measured.
fn m5_failed(report: &PrecisionReport) -> Vec<&'static str> {
    match &report.m5 {
        Some(Ok(rows)) => rows.failed(),
        Some(Err(_)) => vec!["error"],
        None => Vec::new(),
    }
}

/// The M4b rows that failed, without the carried one.
fn gating_failed(rows: &ImpactRows) -> Vec<&'static str> {
    rows.failed()
        .into_iter()
        .filter(|row| *row != M4B_CARRIED_ROW.0)
        .collect()
}

/// Whether an entry's M4b rows failed: a gating one is FAIL, or they could not be measured.
fn impact_failed(report: &PrecisionReport) -> bool {
    match &report.impact {
        Some(Ok(rows)) => !gating_failed(rows).is_empty(),
        Some(Err(_)) => true,
        None => false,
    }
}

/// Whether an entry's ledger failed: `spirolo` could not be run over its runs, its findings
/// are not exactly the oracle's set, or the quarantine round-trip did not hold.
const fn ledger_failed(report: &PrecisionReport) -> bool {
    match &report.ledger {
        Some(Ok(score)) => !score.exact() || !score.quarantine_held(),
        Some(Err(_)) => true,
        None => false,
    }
}

/// Index the adjudications by finding id, so the join is a lookup; the whole row is kept
/// so the join can check its `rule` against the finding's. A later duplicate id
/// overwrites an earlier one; the corpus tooling writes one verdict per id.
fn index_adjudications(adjudications: &[Adjudication]) -> BTreeMap<&str, &Adjudication> {
    adjudications.iter().map(|a| (a.id.as_str(), a)).collect()
}

/// Recorded positives per rule, and how many of them the run reproduced: recall's
/// `(reproduced, total)`.
///
/// A recall positive is a recorded finding expected `present` whose fix is not in history
/// (`fixed_commit` null): it is present at the pin, so the run must report it. A replay
/// pair is silent at the pin and belongs to the replay section; a true negative (`expected:
/// absent`) is a site the rule must stay silent on and is not a positive at all — neither
/// enters the denominator. A positive is reproduced when the run reported a finding that
/// [`ReportedFinding::matches`] its rule, file and invariants. A positive that records no
/// file cannot be located, so no finding can ever match it: it is left out of the
/// denominator rather than counted as missed (a rule firing somewhere in the tree is no
/// evidence it found *this* site, and a positive nothing can match is no evidence either
/// way); recording the file is what makes a positive measurable.
///
/// A positive that names a `present_commit` (the recorded incident ref) is measured against
/// `incident_reported` — the run's findings at that checkout — rather than the pin, because
/// its site is absent at the pin. Both kinds fold into one recall figure per rule.
fn positives_by_rule(
    truth: &Truth,
    reported: &[ReportedFinding],
    incident_reported: &[ReportedFinding],
) -> BTreeMap<String, (u64, u64)> {
    let mut by_rule: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    for finding in &truth.findings {
        if finding.fixed_commit.is_some() || finding.expected == Expected::Absent {
            continue;
        }
        // A positive that names a `present_commit` (the incident ref) is measured against the
        // incident run, not the pin: its site is absent at the pin, so reading it there would
        // count a real recall positive as missed.
        let against = if finding.present_commit.is_some() {
            incident_reported
        } else {
            reported
        };
        let Some(file) = finding.file.as_deref() else {
            continue;
        };
        let reproduced = against
            .iter()
            .any(|candidate| candidate.matches(&finding.rule, file, &finding.invariants, None));
        let entry = by_rule.entry(finding.rule.clone()).or_default();
        entry.0 += u64::from(reproduced);
        entry.1 += 1;
    }
    by_rule
}

/// The recorded replay pairs of a truth, in file order.
#[must_use]
pub fn replay_pairs(truth: &Truth) -> Vec<ReplayPair> {
    truth
        .findings
        .iter()
        .filter_map(|finding| {
            let shipped = finding.shipped_commit.clone()?;
            let fixed = finding.fixed_commit.clone()?;
            Some(ReplayPair {
                rule: finding.rule.clone(),
                anchor: finding.anchor.clone().unwrap_or_default(),
                file: finding.file.clone().unwrap_or_default(),
                invariants: finding.invariants.clone(),
                shipped_commit: shipped,
                fixed_commit: fixed,
            })
        })
        .collect()
}

/// Render the precision section for the whole corpus.
///
/// A header, then per entry a line per rule, a `warn` line per join warning (with a count
/// of the unmatched adjudications when there is at least one) and a line per replay pair
/// (ending in `error: <message>` when the runner could not check it), then the corpus
/// verdict line for the milestone.
#[must_use]
pub fn render(reports: &[PrecisionReport], floors: &[RuleFloor]) -> String {
    let rule_width = reports
        .iter()
        .flat_map(|report| report.rules.iter().map(|row| row.rule.len()))
        .chain(
            reports
                .iter()
                .flat_map(|report| report.replays.iter().map(|replay| replay.rule.len())),
        )
        .max()
        .unwrap_or(0);

    let mut out = String::new();
    let _ = writeln!(
        out,
        "precision · check findings vs truth.json adjudications"
    );

    for report in reports {
        let _ = writeln!(out, "\n{}", report.name);
        if let Some(why) = &report.not_measured {
            let _ = writeln!(out, "  not measured: {why}");
        }
        for row in &report.rules {
            render_rule(&mut out, row, rule_width);
        }
        for warning in &report.warnings {
            let _ = writeln!(out, "  warn  {warning}");
        }
        let unmatched = report
            .warnings
            .iter()
            .filter(|warning| matches!(warning, JoinWarning::UnmatchedAdjudication { .. }))
            .count();
        if unmatched > 0 {
            let _ = writeln!(
                out,
                "  {unmatched} adjudication(s) matched no reported finding"
            );
        }
        for replay in &report.replays {
            let _ = write!(
                out,
                "  replay {:<rule_width$}  shipped {}  fixed {}  {}",
                replay.rule,
                if replay.present_at_shipped {
                    "present"
                } else {
                    "absent"
                },
                if replay.present_at_fixed {
                    "present"
                } else {
                    "absent"
                },
                replay.tag(),
            );
            if let Some(error) = &replay.error {
                let _ = write!(out, "  error: {error}");
            }
            out.push('\n');
        }
        if let Some(score) = &report.known_flaky {
            render_known_flaky(&mut out, score);
        }
        match &report.ledger {
            Some(Ok(score)) => out.push_str(&render_ledger(score)),
            Some(Err(error)) => {
                let _ = writeln!(out, "  ledger flake-observed  error: {error}  FAIL");
            }
            None => {}
        }
        match &report.impact {
            Some(Ok(rows)) => out.push_str(&render_impact(rows)),
            Some(Err(error)) => {
                let _ = writeln!(out, "  M4b impact  error: {error}  FAIL");
            }
            None => {}
        }
        match &report.m5 {
            Some(Ok(rows)) => out.push_str(&render_m5(rows)),
            Some(Err(error)) => {
                let _ = writeln!(out, "  M5  error: {error}  FAIL");
            }
            None => {}
        }
    }

    out.push('\n');
    render_milestone(&mut out, reports, floors);
    out
}

/// The known-flaky block: k of n flagged at the rows' `flaky_commit`s with the Wilson
/// interval and the `small-n` tag (recall on a handful of rows, never a rate), the fix-side
/// checks that held, the verdict, then one line per row.
fn render_known_flaky(out: &mut String, score: &KnownFlakyScore) {
    let (k, n) = (score.flagged() as u64, score.rows.len() as u64);
    let checked = score
        .rows
        .iter()
        .filter(|row| row.fix != FixSide::NotChecked)
        .count();
    let held = score
        .rows
        .iter()
        .filter(|row| row.fix == FixSide::Absent)
        .count();
    let _ = writeln!(
        out,
        "  known-flaky flake-candidate  flagged {k} of {n}  interval {}  small-n  \
         fix side {held} of {checked} absent  {}",
        format_interval(wilson_interval(k, n), n > 0),
        score.verdict().tag(),
    );
    for row in &score.rows {
        let fix = match row.fix {
            FixSide::NotChecked => "",
            FixSide::Absent => "fix absent",
            FixSide::Present => "fix present",
        };
        let test = row
            .test
            .as_ref()
            .map_or_else(String::new, |path| format!(" > {}", path.join(" > ")));
        let _ = write!(
            out,
            "    {:<11}  {fix:<11}  {}{test}",
            if row.flagged {
                "flagged"
            } else {
                "not flagged"
            },
            row.file,
        );
        if let Some(error) = &row.error {
            let _ = write!(out, "  error: {error}");
        }
        out.push('\n');
    }
}

/// One rule's precision line. `insp t/f` is the inspection verdicts of an entry whose tests
/// are not executed, printed beside the executed columns and never summed into them.
fn render_rule(out: &mut String, row: &RulePrecision, rule_width: usize) {
    let _ = writeln!(
        out,
        "  {:<rule_width$}  reported {:>3}  sampled {:>3}  true {:>3}  false {:>3}  open {:>3}  \
         cnd {:>3}  insp {:>3}/{:<3}  precision {}  interval {}  recall {}  {}",
        row.rule,
        row.reported,
        row.sampled,
        row.truths,
        row.falses,
        row.open,
        row.could_not_decide,
        row.inspected_truths,
        row.inspected_falses,
        format_ratio(row.precision()),
        format_interval(row.interval(), row.precision().is_some()),
        format_ratio(row.recall()),
        row.verdict.tag(),
    );
    let kind_width = row.strata.iter().map(|s| s.value.len()).max().unwrap_or(0);
    for stratum in &row.strata {
        if stratum.reported == 0 {
            let _ = writeln!(out, "      {:<kind_width$}  no finding", stratum.value);
            continue;
        }
        let _ = write!(
            out,
            "      {:<kind_width$}  reported {:>3}  executable {:>3}  sampled {:>3}  \
             true {:>3}  false {:>3}  open {:>3}",
            stratum.value,
            stratum.reported,
            stratum.executable,
            stratum.sampled,
            stratum.truths,
            stratum.falses,
            stratum.open,
        );
        if stratum.executable == 0 {
            out.push_str("  INSUFFICIENT (never executed)");
        }
        out.push('\n');
    }
}

/// The milestone line: the corpus verdict per rule and whether any rule or replay failed.
///
/// Every verdict class is counted, so a rule the corpus could not feed (`insufficient`), a
/// rule whose findings the oracle did not cover (`unadjudicated`, named and red) or one that
/// never fired (`skipped`) is visible in the one line the milestone is read from, not
/// only in the rows above it; replay pairs are `held of total`, so a skipped or failed pair
/// is a visible gap too. The green form ends in `none failed`; the red form leads with the
/// failing rules and pairs and still carries the counts. With no rule row and no replay
/// pair anywhere — every entry pending or un-cloned — the line says `nothing measured`
/// rather than the vacuous `0 of 0 rules … none failed`.
fn render_milestone(out: &mut String, reports: &[PrecisionReport], floors: &[RuleFloor]) {
    let (verdicts, carried_verdicts): (Vec<_>, Vec<_>) = corpus_verdicts(reports)
        .into_iter()
        .partition(|(rule, _)| carried(floors, rule).is_none());
    let replays: Vec<&ReplayResult> = reports
        .iter()
        .flat_map(|report| report.replays.iter())
        .collect();
    let known_flaky: Vec<&KnownFlakyScore> = reports
        .iter()
        .filter_map(|report| report.known_flaky.as_ref())
        .collect();
    let ledgers: Vec<&Result<LedgerScore, String>> = reports
        .iter()
        .filter_map(|report| report.ledger.as_ref())
        .collect();
    if verdicts.is_empty()
        && carried_verdicts.is_empty()
        && !reports.iter().any(measured_beyond_rules)
    {
        let _ = writeln!(out, "milestone · nothing measured");
        return;
    }
    let (carried_entries, known_flaky) =
        carried_entries(&carried_verdicts, known_flaky, floors, reports);
    render_carried(out, &carried_entries);

    let count = |wanted: RuleVerdict| {
        verdicts
            .iter()
            .filter(|(_, verdict)| *verdict == wanted)
            .count()
    };
    let failing: Vec<&str> = verdicts
        .iter()
        .filter(|(_, verdict)| *verdict == RuleVerdict::Fail)
        .map(|(rule, _)| rule.as_str())
        .collect();
    let unadjudicated: Vec<&str> = verdicts
        .iter()
        .filter(|(_, verdict)| *verdict == RuleVerdict::Unadjudicated)
        .map(|(rule, _)| rule.as_str())
        .collect();
    let failed_replays: Vec<&str> = replays
        .iter()
        .filter(|replay| replay.failed())
        .map(|replay| replay.rule.as_str())
        .collect();
    let held = replays.iter().filter(|replay| replay.passed()).count();

    let mut counts = format!(
        "{} of {} rules at ≥0.9 precision · {} insufficient · {} unadjudicated · {} skipped · {held} of {} replay pairs hold",
        count(RuleVerdict::Pass),
        verdicts.len(),
        count(RuleVerdict::Insufficient),
        unadjudicated.len(),
        count(RuleVerdict::Skipped),
        replays.len(),
    );
    // M4: the static flake recall on each entry's known-flaky set, as k of n (small-n).
    for score in &known_flaky {
        let _ = write!(counts, " · {}", known_flaky_counts(score));
    }

    // M4: the run ledger against the oracle's recomputed set, exact or not.
    counts.push_str(&ledger_counts(&ledgers));

    // M4b and M5: each entry's impact and surface rows, by verdict.
    let impacts: Vec<&Result<ImpactRows, String>> = reports
        .iter()
        .filter_map(|report| report.impact.as_ref())
        .collect();
    counts.push_str(&impact_counts(&impacts));
    counts.push_str(&m5_counts(reports));

    let mut parts = Vec::new();
    if !failing.is_empty() {
        parts.push(format!("rules below 0.9: {}", failing.join(", ")));
    }
    if !unadjudicated.is_empty() {
        parts.push(format!("rules unadjudicated: {}", unadjudicated.join(", ")));
    }
    if !failed_replays.is_empty() {
        parts.push(format!(
            "replay pairs failed: {}",
            failed_replays.join(", ")
        ));
    }
    if known_flaky
        .iter()
        .any(|score| score.verdict() == RuleVerdict::Fail)
    {
        parts.push("known-flaky failed".to_owned());
    }
    if reports.iter().any(ledger_failed) {
        parts.push("ledger failed".to_owned());
    }
    parts.extend(
        impacts
            .iter()
            .filter_map(|impact| impact_failed_part(impact)),
    );
    parts.extend(reports.iter().filter_map(m5_failed_part));
    let red = !parts.is_empty();
    parts.push(counts);
    if !red {
        parts.push("none failed".to_owned());
    }
    let _ = writeln!(out, "milestone · {}", parts.join(" · "));
}

/// Whether an entry measured anything besides its rule rows: a replay pair, a known-flaky
/// set, a ledger, impact rows or M5 rows.
const fn measured_beyond_rules(report: &PrecisionReport) -> bool {
    !report.replays.is_empty()
        || report.known_flaky.is_some()
        || report.ledger.is_some()
        || report.impact.is_some()
        || report.m5.is_some()
}

/// The carried line's entry for an entry's M4b hotspots row: `untested-hotspot <verdict>`.
fn carried_hotspots(report: &PrecisionReport) -> Option<(Carried, String)> {
    let Some(Ok(rows)) = &report.impact else {
        return None;
    };
    let (_, verdict) = rows
        .verdicts()
        .into_iter()
        .find(|(row, _)| *row == M4B_CARRIED_ROW.0)?;
    Some((
        M4B_CARRIED,
        format!("{} {}", M4B_CARRIED_ROW.1, verdict.tag()),
    ))
}

/// `M5 failed: <entry> <rows>` for an entry whose M5 rows failed or could not be measured.
fn m5_failed_part(report: &PrecisionReport) -> Option<String> {
    let failed = m5_failed(report);
    (!failed.is_empty()).then(|| format!("M5 failed: {} {}", report.name, failed.join(", ")))
}

/// The milestone line's M5 part: ` · M5 <entry> diagnostics PASS · live PASS · …` per entry
/// with M5 rows, or ` · M5 <entry> error`.
fn m5_counts(reports: &[PrecisionReport]) -> String {
    let mut out = String::new();
    for report in reports {
        match &report.m5 {
            Some(Ok(rows)) => {
                let _ = write!(out, " · M5 {}", report.name);
                for (i, (row, pass)) in rows.verdicts().into_iter().enumerate() {
                    let _ = write!(
                        out,
                        "{} {row} {}",
                        if i == 0 { "" } else { " ·" },
                        if pass { "PASS" } else { "FAIL" }
                    );
                }
            }
            Some(Err(_)) => {
                let _ = write!(out, " · M5 {} error", report.name);
            }
            None => {}
        }
    }
    out
}

/// The milestone line's ledger part: ` · ledger exact`, the missing and extra counts, or
/// ` · ledger error`, per entry with a ledger.
fn ledger_counts(ledgers: &[&Result<LedgerScore, String>]) -> String {
    let mut out = String::new();
    for ledger in ledgers {
        match ledger {
            Ok(score) if score.exact() => out.push_str(" · ledger exact"),
            Ok(score) => {
                let _ = write!(
                    out,
                    " · ledger {} missing {} extra",
                    score.missing.len(),
                    score.extra.len()
                );
            }
            Err(_) => out.push_str(" · ledger error"),
        }
    }
    out
}

/// The carried line's entries: each carried rule's corpus verdict, each entry's M4b hotspots
/// row (its floor carried to M6) and, once
/// `flake-candidate`'s floor is carried, each known-flaky set's counts (its recall is that
/// rule's floor). Returns them with the known-flaky sets still gating, none when carried.
fn carried_entries<'a>(
    carried_verdicts: &[(String, RuleVerdict)],
    known_flaky: Vec<&'a KnownFlakyScore>,
    floors: &[RuleFloor],
    reports: &[PrecisionReport],
) -> (Vec<(Carried, String)>, Vec<&'a KnownFlakyScore>) {
    let mut entries: Vec<(Carried, String)> = carried_verdicts
        .iter()
        .filter_map(|(rule, verdict)| {
            carried(floors, rule).map(|floor| (floor, format!("{rule} {}", verdict.tag())))
        })
        .chain(reports.iter().filter_map(carried_hotspots))
        .collect();
    match carried(floors, KNOWN_FLAKY_RULE) {
        Some(floor) => {
            entries.extend(
                known_flaky
                    .iter()
                    .map(|score| (floor, known_flaky_counts(score))),
            );
            (entries, Vec::new())
        }
        None => (entries, known_flaky),
    }
}

/// `M4b failed: <rows>` for an entry whose M4b rows failed or could not be measured.
fn impact_failed_part(impact: &Result<ImpactRows, String>) -> Option<String> {
    let failed = impact
        .as_ref()
        .map_or_else(|_| vec!["error"], gating_failed);
    (!failed.is_empty()).then(|| format!("M4b failed: {}", failed.join(", ")))
}

/// The milestone line's M4b part: ` · M4b safety PASS · ranking PASS · …` per entry with an
/// impact oracle, or ` · M4b error`.
fn impact_counts(impacts: &[&Result<ImpactRows, String>]) -> String {
    let mut out = String::new();
    for impact in impacts {
        match impact {
            Ok(rows) => {
                out.push_str(" · M4b");
                let gating = rows
                    .verdicts()
                    .into_iter()
                    .filter(|(row, _)| *row != M4B_CARRIED_ROW.0);
                for (i, (row, verdict)) in gating.enumerate() {
                    let _ = write!(
                        out,
                        "{} {row} {}",
                        if i == 0 { "" } else { " ·" },
                        verdict.tag()
                    );
                }
            }
            Err(_) => out.push_str(" · M4b error"),
        }
    }
    out
}

/// `known-flaky k of n flagged small-n VERDICT`, the milestone line's part for one set.
fn known_flaky_counts(score: &KnownFlakyScore) -> String {
    format!(
        "known-flaky {} of {} flagged small-n {}",
        score.flagged(),
        score.rows.len(),
        score.verdict().tag()
    )
}

/// One line per carried floor, before the milestone line: the milestone it was carried to,
/// why it does not gate, and each carried entry (a rule's corpus verdict, or a known-flaky
/// set's counts), grouped by floor in first-seen order.
fn render_carried(out: &mut String, entries: &[(Carried, String)]) {
    let mut lines: Vec<(Carried, Vec<&str>)> = Vec::new();
    for (floor, entry) in entries {
        match lines.iter_mut().find(|(seen, _)| seen == floor) {
            Some((_, rules)) => rules.push(entry),
            None => lines.push((*floor, vec![entry])),
        }
    }
    for (floor, rules) in lines {
        let _ = writeln!(
            out,
            "carried to {} · not gating: {} · {}",
            floor.to,
            floor.why,
            rules.join(", ")
        );
    }
}

/// A proportion as `0.900`, or `—` when there is nothing to divide.
fn format_ratio(value: Option<f64>) -> String {
    value.map_or_else(|| "  —  ".to_owned(), |value| format!("{value:.3}"))
}

/// A Wilson interval as `[0.744, 0.965]`, or `—` when there is no adjudicated proportion.
pub fn format_interval(interval: Interval, has_data: bool) -> String {
    if has_data {
        format!("[{:.3}, {:.3}]", interval.low, interval.high)
    } else {
        "     —      ".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        corpus_verdicts, evaluate_entry, render, replay_pairs, reported_findings, sample,
        section_failed, verdict, wilson_interval, JoinWarning, RecallFloor, ReplayResult,
        ReportedFinding, RuleVerdict, RULE_FLOORS,
    };
    use crate::truth::{Adjudication, Expected, Finding, Truth, Verdict};
    use std::collections::BTreeMap;

    fn reported(rule: &str, ids: &[&str]) -> Vec<ReportedFinding> {
        ids.iter()
            .map(|id| ReportedFinding {
                rule: rule.to_owned(),
                id: (*id).to_owned(),
                file: String::new(),
                decided: true,
                test: None,
                evidence: BTreeMap::new(),
            })
            .collect()
    }

    /// A decided reported finding of `rule`, identified by `id`, located in `file`.
    fn reported_in(rule: &str, id: &str, file: &str) -> ReportedFinding {
        ReportedFinding {
            rule: rule.to_owned(),
            id: id.to_owned(),
            file: file.to_owned(),
            decided: true,
            test: None,
            evidence: BTreeMap::new(),
        }
    }

    /// A present-at-pin recall positive of `rule` located in `file` (`None` for a row that
    /// records no file), expected present unless overridden by the test.
    fn positive(rule: &str, file: Option<&str>) -> Finding {
        Finding {
            rule: rule.to_owned(),
            anchor: Some("a site".to_owned()),
            file: file.map(str::to_owned),
            invariants: Vec::new(),
            shipped_commit: None,
            fixed_commit: None,
            present_commit: None,
            expected: Expected::Present,
        }
    }

    fn positive_at(rule: &str, file: &str, commit: &str) -> Finding {
        Finding {
            present_commit: Some(commit.to_owned()),
            ..positive(rule, Some(file))
        }
    }

    fn truth_with(adjudications: Vec<Adjudication>) -> Truth {
        Truth {
            access: crate::truth::Access::Local,
            execute: true,
            never_executed: Vec::new(),
            never_executed_by_rule: std::collections::BTreeMap::new(),
            substrate: None,
            excluded_packages: Vec::new(),
            excluded_files: Vec::new(),
            counts: std::collections::BTreeMap::new(),
            budget_ms: 5000,
            m2_list_ms: None,
            adjudications,
            findings: Vec::new(),
            replay_none: Vec::new(),
            runner: std::collections::BTreeMap::new(),
            incident: None,
            ledger: None,
            stratified_samples: BTreeMap::new(),
            retired_adjudications: Vec::new(),
        }
    }

    fn adj(id: &str, verdict: Verdict) -> Adjudication {
        Adjudication {
            rule: "weak-assertion".to_owned(),
            id: id.to_owned(),
            verdict,
            note: String::new(),
            substrate: None,
        }
    }

    /// 27 true of 30 sampled → precision 0.900, interval [0.744, 0.965], PASS.
    #[test]
    fn twenty_seven_true_of_thirty_prints_the_expected_interval_and_passes() {
        let ids: Vec<String> = (0..30).map(|i| format!("weak-assertion f{i:02}")).collect();
        let reported = reported(
            "weak-assertion",
            &ids.iter().map(String::as_str).collect::<Vec<_>>(),
        );
        let adjudications = ids
            .iter()
            .enumerate()
            .map(|(i, id)| {
                adj(
                    id,
                    if i < 27 {
                        Verdict::True
                    } else {
                        Verdict::False
                    },
                )
            })
            .collect();
        let report = evaluate_entry(
            "canned",
            &["weak-assertion".to_owned()],
            &reported,
            &truth_with(adjudications),
        );
        let row = &report.rules[0];
        assert_eq!(row.sampled, 30);
        assert_eq!(row.truths, 27);
        assert_eq!(row.falses, 3);
        assert!((row.precision().unwrap() - 0.9).abs() < 1e-9);
        let interval = row.interval();
        assert_eq!(format!("{:.3}", interval.low), "0.744");
        assert_eq!(format!("{:.3}", interval.high), "0.965");
        assert_eq!(row.verdict, RuleVerdict::Pass);
        assert_eq!(
            corpus_verdicts(std::slice::from_ref(&report))[0].1,
            RuleVerdict::Pass
        );
    }

    /// The hygiene family's first row: a canned `duplicate-test` check of 20 findings with 18
    /// `true` / 2 `false` adjudications prints n, true/false, precision, interval and a PASS
    /// verdict, and the rendered section names the rule. Guards the seam-1 acceptance
    /// criterion that the precision section shows a `duplicate-test` row.
    #[test]
    fn duplicate_test_row_shows_in_the_precision_section() {
        let ids: Vec<String> = (0..20).map(|i| format!("duplicate-test f{i:02}")).collect();
        let reported = reported(
            "duplicate-test",
            &ids.iter().map(String::as_str).collect::<Vec<_>>(),
        );
        let adjudications = ids
            .iter()
            .enumerate()
            .map(|(i, id)| Adjudication {
                rule: "duplicate-test".to_owned(),
                id: id.clone(),
                verdict: if i < 18 {
                    Verdict::True
                } else {
                    Verdict::False
                },
                note: String::new(),
                substrate: None,
            })
            .collect();
        let report = evaluate_entry(
            "canned",
            &["duplicate-test".to_owned()],
            &reported,
            &truth_with(adjudications),
        );
        let row = &report.rules[0];
        assert_eq!(row.rule, "duplicate-test");
        assert_eq!(row.reported, 20);
        assert_eq!(row.sampled, 20);
        assert_eq!(row.truths, 18);
        assert_eq!(row.falses, 2);
        assert!((row.precision().unwrap() - 0.9).abs() < 1e-9);
        assert_eq!(row.verdict, RuleVerdict::Pass);
        let out = render(std::slice::from_ref(&report), RULE_FLOORS);
        assert!(out.contains("duplicate-test  reported  20"), "{out}");
        assert!(out.contains("true  18  false   2"), "{out}");
        assert!(out.contains("PASS"), "{out}");
    }

    /// The `unused-mock` row prints in the precision section (seam-1 acceptance criterion:
    /// "spirolo-bench prints the unused-mock row as measured"). A canned check of 20 findings
    /// adjudicated 19 true / 1 false is PASS at 0.95.
    #[test]
    fn unused_mock_row_shows_in_the_precision_section() {
        let ids: Vec<String> = (0..20).map(|i| format!("unused-mock f{i:02}")).collect();
        let reported = reported(
            "unused-mock",
            &ids.iter().map(String::as_str).collect::<Vec<_>>(),
        );
        let adjudications = ids
            .iter()
            .enumerate()
            .map(|(i, id)| Adjudication {
                rule: "unused-mock".to_owned(),
                id: id.clone(),
                verdict: if i < 19 {
                    Verdict::True
                } else {
                    Verdict::False
                },
                note: String::new(),
                substrate: None,
            })
            .collect();
        let report = evaluate_entry(
            "canned",
            &["unused-mock".to_owned()],
            &reported,
            &truth_with(adjudications),
        );
        let row = &report.rules[0];
        assert_eq!(row.rule, "unused-mock");
        assert_eq!(row.reported, 20);
        assert_eq!(row.truths, 19);
        assert_eq!(row.falses, 1);
        assert_eq!(row.verdict, RuleVerdict::Pass);
        let out = render(std::slice::from_ref(&report), RULE_FLOORS);
        assert!(out.contains("unused-mock  reported  20"), "{out}");
    }

    /// 26 true of 30 → precision 0.867, FAIL.
    #[test]
    fn twenty_six_true_of_thirty_fails() {
        let ids: Vec<String> = (0..30).map(|i| format!("weak-assertion f{i:02}")).collect();
        let reported = reported(
            "weak-assertion",
            &ids.iter().map(String::as_str).collect::<Vec<_>>(),
        );
        let adjudications = ids
            .iter()
            .enumerate()
            .map(|(i, id)| {
                adj(
                    id,
                    if i < 26 {
                        Verdict::True
                    } else {
                        Verdict::False
                    },
                )
            })
            .collect();
        let report = evaluate_entry(
            "canned",
            &["weak-assertion".to_owned()],
            &reported,
            &truth_with(adjudications),
        );
        assert_eq!(report.rules[0].verdict, RuleVerdict::Fail);
        assert!(section_failed(std::slice::from_ref(&report), RULE_FLOORS));
    }

    /// 15 adjudicated (of 15 reported) → INSUFFICIENT, below the 20-across-the-corpus floor.
    #[test]
    fn fifteen_adjudicated_is_insufficient() {
        let ids: Vec<String> = (0..15).map(|i| format!("weak-assertion f{i:02}")).collect();
        let reported = reported(
            "weak-assertion",
            &ids.iter().map(String::as_str).collect::<Vec<_>>(),
        );
        let adjudications = ids.iter().map(|id| adj(id, Verdict::True)).collect();
        let report = evaluate_entry(
            "canned",
            &["weak-assertion".to_owned()],
            &reported,
            &truth_with(adjudications),
        );
        assert_eq!(report.rules[0].truths, 15);
        assert_eq!(report.rules[0].verdict, RuleVerdict::Insufficient);
        assert!(!section_failed(std::slice::from_ref(&report), RULE_FLOORS));
    }

    /// An entry that does not execute its tests adjudicates by inspection: its 25 `true`
    /// verdicts land in `inspected_truths`, `truths` stays 0, the row is INSUFFICIENT (no
    /// executed oracle can cover a finding on it, so it is the corpus being too small,
    /// never PASS), and the corpus verdict over it plus an executed entry with 20
    /// trues passes on the executed 20 alone — the inspection 25 neither help nor hurt the floor.
    #[test]
    fn inspection_verdicts_are_counted_apart_and_never_reach_the_floor() {
        let entry = |name: &str, count: usize, execute: bool| {
            let ids: Vec<String> = (0..count)
                .map(|i| format!("weak-assertion {name}{i:02}"))
                .collect();
            let reported = reported(
                "weak-assertion",
                &ids.iter().map(String::as_str).collect::<Vec<_>>(),
            );
            let mut truth = truth_with(ids.iter().map(|id| adj(id, Verdict::True)).collect());
            truth.execute = execute;
            evaluate_entry(name, &["weak-assertion".to_owned()], &reported, &truth)
        };

        let inspected = entry("sample", 25, false);
        let row = &inspected.rules[0];
        assert_eq!((row.truths, row.falses), (0, 0), "nothing executed");
        assert_eq!((row.inspected_truths, row.inspected_falses), (25, 0));
        assert_eq!(row.verdict, RuleVerdict::Insufficient);
        assert!(
            row.precision().is_none(),
            "no executed adjudication, no precision"
        );
        assert_eq!(
            corpus_verdicts(std::slice::from_ref(&inspected))[0].1,
            RuleVerdict::Insufficient,
            "25 inspection trues alone do not reach the floor"
        );
        assert!(
            render(std::slice::from_ref(&inspected), RULE_FLOORS)
                .contains("cnd   0  insp  25/0    precision   —  "),
            "{}",
            render(std::slice::from_ref(&inspected), RULE_FLOORS)
        );

        let executed = entry("local", 20, true);
        assert_eq!(executed.rules[0].truths, 20);
        assert_eq!(executed.rules[0].inspected_truths, 0);
        assert_eq!(
            corpus_verdicts(&[inspected, executed])[0].1,
            RuleVerdict::Pass,
            "the executed 20 alone carry the verdict"
        );
    }

    /// Sampling is deterministic: two builds of the sample from the same reported findings,
    /// in different input orders, produce the same sample.
    #[test]
    fn sampling_is_deterministic_across_input_order() {
        let ids: Vec<String> = (0..137)
            .map(|i| format!("weak-assertion f{i:03}"))
            .collect();
        let forward = reported(
            "weak-assertion",
            &ids.iter().map(String::as_str).collect::<Vec<_>>(),
        );
        let mut reversed = forward.clone();
        reversed.reverse();
        let first = sample(forward);
        let second = sample(reversed);
        assert_eq!(first, second);
        // 137 findings → the large target, 50, exactly.
        assert_eq!(first.len(), 50);
    }

    /// The sample has exactly `min(n, target)` findings, whatever `n` is — the sizes the
    /// review measured undershooting (31 → 16, 215 → 43, 1173 → 49) now reach the target.
    #[test]
    fn sampling_reaches_the_target_exactly() {
        let sampled = |n: usize| {
            let ids: Vec<String> = (0..n).map(|i| format!("weak-assertion f{i:04}")).collect();
            sample(reported(
                "weak-assertion",
                &ids.iter().map(String::as_str).collect::<Vec<_>>(),
            ))
        };
        for (n, expected) in [
            (12, 12),
            (30, 30),
            (31, 30),
            (42, 30),
            (49, 30),
            (50, 50),
            (63, 50),
            (137, 50),
            (215, 50),
            (1173, 50),
        ] {
            let sample = sampled(n);
            assert_eq!(sample.len(), expected, "{n} findings");
            let mut ids: Vec<&str> = sample.iter().map(|f| f.id.as_str()).collect();
            ids.dedup();
            assert_eq!(
                ids.len(),
                expected,
                "{n} findings: every kept id is distinct"
            );
        }
    }

    /// The sample is spread across the sorted list, not clustered at its front: the first
    /// kept finding is the first id and the last kept one lies in the final stretch.
    #[test]
    fn sampling_spreads_across_the_sorted_list() {
        let ids: Vec<String> = (0..137)
            .map(|i| format!("weak-assertion f{i:03}"))
            .collect();
        let sample = sample(reported(
            "weak-assertion",
            &ids.iter().map(String::as_str).collect::<Vec<_>>(),
        ));
        assert_eq!(sample[0].id, "weak-assertion f000");
        // floor(49 * 137 / 50) = 134.
        assert_eq!(sample[49].id, "weak-assertion f134");
        // floor(25 * 137 / 50) = 68: the middle of the sample is the middle of the list.
        assert_eq!(sample[25].id, "weak-assertion f068");
    }

    /// An unadjudicated sampled finding is `open` and never true; a could-not-decide
    /// verdict is its own column and never true.
    #[test]
    fn open_and_could_not_decide_are_separate_and_never_true() {
        let reported = reported("weak-assertion", &["weak-assertion a", "weak-assertion b"]);
        // Only `a` is adjudicated, and as open (could not decide).
        let report = evaluate_entry(
            "canned",
            &["weak-assertion".to_owned()],
            &reported,
            &truth_with(vec![adj("weak-assertion a", Verdict::Open)]),
        );
        let row = &report.rules[0];
        assert_eq!(row.truths, 0);
        assert_eq!(row.falses, 0);
        assert_eq!(row.could_not_decide, 1, "the Open verdict");
        assert_eq!(row.open, 1, "the unadjudicated finding `b`");
        assert!(
            row.precision().is_none(),
            "nothing adjudicated true or false"
        );
    }

    /// An adjudication whose id matches no reported finding is a warning line in truth.json
    /// order, never a dropped row: it is printed with a per-entry count, and the section
    /// does not redden on it. An adjudication of a could-not-decide row is matched (the run
    /// did report that id) and so is not warned about.
    #[test]
    fn an_unmatched_adjudication_is_warned_about_not_dropped() {
        let mut reported = reported("weak-assertion", &["weak-assertion a", "weak-assertion b"]);
        reported.push(ReportedFinding {
            rule: "handler-drift".to_owned(),
            id: "handler-drift z".to_owned(),
            file: String::new(),
            decided: false,
            test: None,
            evidence: BTreeMap::new(),
        });
        let mut stale = adj("weak-assertion gone", Verdict::True);
        stale.rule = "weak-assertion".to_owned();
        let mut renamed = adj("dead-skip moved", Verdict::False);
        renamed.rule = "dead-skip".to_owned();
        let mut undecided = adj("handler-drift z", Verdict::Open);
        undecided.rule = "handler-drift".to_owned();
        let report = evaluate_entry(
            "canned",
            &["weak-assertion".to_owned()],
            &reported,
            &truth_with(vec![
                adj("weak-assertion a", Verdict::True),
                stale,
                renamed,
                undecided,
            ]),
        );
        assert_eq!(
            report.warnings,
            vec![
                JoinWarning::UnmatchedAdjudication {
                    rule: "weak-assertion".to_owned(),
                    id: "weak-assertion gone".to_owned(),
                },
                JoinWarning::UnmatchedAdjudication {
                    rule: "dead-skip".to_owned(),
                    id: "dead-skip moved".to_owned(),
                },
            ],
            "truth.json order; the matched `a` and the could-not-decide `z` are not warned"
        );
        assert_eq!(
            report.rules[0].truths, 1,
            "the matched verdict still counts"
        );
        assert!(!section_failed(std::slice::from_ref(&report), RULE_FLOORS));

        let rendered = render(std::slice::from_ref(&report), RULE_FLOORS);
        assert!(
            rendered
                .contains("  warn  unmatched adjudication: weak-assertion weak-assertion gone\n"),
            "{rendered}"
        );
        assert!(
            rendered.contains("  warn  unmatched adjudication: dead-skip dead-skip moved\n"),
            "{rendered}"
        );
        assert!(
            rendered.contains("  2 adjudication(s) matched no reported finding\n"),
            "{rendered}"
        );
        let warn_at = rendered.find("  warn  ").unwrap();
        let row_at = rendered.find("  weak-assertion  reported").unwrap();
        assert!(row_at < warn_at, "warnings follow the rule rows");

        // No warnings → no count line either.
        let clean = evaluate_entry(
            "canned",
            &["weak-assertion".to_owned()],
            &reported,
            &truth_with(vec![adj("weak-assertion a", Verdict::True)]),
        );
        assert!(clean.warnings.is_empty());
        assert!(!render(std::slice::from_ref(&clean), RULE_FLOORS)
            .contains("matched no reported finding"));
    }

    /// An adjudication whose rule is not the sampled finding's rule is not counted: the
    /// finding stays `open`, the verdict (here `true`) is not a truth, and a warning names
    /// both rules. The same id under the right rule counts as usual.
    #[test]
    fn a_rule_mismatch_on_the_join_leaves_the_finding_open_and_warns() {
        let reported = reported("weak-assertion", &["weak-assertion a", "weak-assertion b"]);
        let mut wrong_rule = adj("weak-assertion a", Verdict::True);
        wrong_rule.rule = "dead-skip".to_owned();
        let report = evaluate_entry(
            "canned",
            &["weak-assertion".to_owned()],
            &reported,
            &truth_with(vec![wrong_rule, adj("weak-assertion b", Verdict::True)]),
        );
        let row = &report.rules[0];
        assert_eq!(
            (row.truths, row.falses, row.open, row.could_not_decide),
            (1, 0, 1, 0),
            "`b` counts, `a` is open"
        );
        assert_eq!(
            report.warnings,
            vec![JoinWarning::RuleMismatch {
                id: "weak-assertion a".to_owned(),
                adjudication_rule: "dead-skip".to_owned(),
                finding_rule: "weak-assertion".to_owned(),
            }]
        );
        let rendered = render(std::slice::from_ref(&report), RULE_FLOORS);
        assert!(
            rendered.contains(
                "  warn  rule mismatch on weak-assertion a: adjudication says dead-skip, finding says weak-assertion\n"
            ),
            "{rendered}"
        );
        assert!(
            !rendered.contains("matched no reported finding"),
            "a mismatch is not an unmatched id: the count line is for unmatched ids only"
        );
        assert!(!section_failed(std::slice::from_ref(&report), RULE_FLOORS));
    }

    /// A rule that reported nothing is SKIPPED, not judged.
    #[test]
    fn a_rule_that_reported_nothing_is_skipped() {
        let report = evaluate_entry(
            "canned",
            &["handler-drift".to_owned()],
            &[],
            &truth_with(Vec::new()),
        );
        assert_eq!(report.rules[0].verdict, RuleVerdict::Skipped);
        assert_eq!(report.rules[0].reported, 0);
    }

    /// A replay pair present-at-shipped and absent-at-fixed passes; present-at-fixed fails;
    /// absent-at-both is skipped (the rule is not yet implemented) and never reddens.
    #[test]
    fn a_replay_pair_passes_present_then_absent_and_fails_the_reverse() {
        let replay = |shipped: bool, fixed: bool| ReplayResult {
            rule: "handler-drift".to_owned(),
            anchor: "BASE".to_owned(),
            present_at_shipped: shipped,
            present_at_fixed: fixed,
            error: None,
        };
        let good = replay(true, false);
        assert!(good.passed() && !good.failed() && !good.skipped());
        // Present where the fix should have removed it → a real failure.
        let bad = replay(true, true);
        assert!(bad.failed() && !bad.passed() && !bad.skipped());
        // Absent at both commits → the rule fires nowhere: skipped, not failed.
        let unimplemented = replay(false, false);
        assert!(unimplemented.skipped() && !unimplemented.failed() && !unimplemented.passed());

        // A report whose replay failed is red; one whose replay is only skipped is not.
        let report = |result: ReplayResult| super::PrecisionReport {
            name: "canned".to_owned(),
            rules: Vec::new(),
            warnings: Vec::new(),
            replays: vec![result],
            known_flaky: None,
            ledger: None,
            impact: None,
            m5: None,
            not_measured: None,
        };
        assert!(section_failed(
            std::slice::from_ref(&report(bad)),
            RULE_FLOORS
        ));
        assert!(!section_failed(
            std::slice::from_ref(&report(unimplemented)),
            RULE_FLOORS
        ));
    }

    /// A pair the runner errored on fails whatever its two booleans say — the answers a
    /// pass or a skip would need are no evidence once the runner could not look — and the
    /// rendered line carries the message after the tag.
    #[test]
    fn a_replay_runner_error_fails_the_pair_and_is_printed() {
        let errored = |shipped: bool, fixed: bool| ReplayResult {
            rule: "handler-drift".to_owned(),
            anchor: "BASE".to_owned(),
            present_at_shipped: shipped,
            present_at_fixed: fixed,
            error: Some("shipped commit: git worktree add aaaaaaa: not a valid object".to_owned()),
        };
        // The booleans of a pass, and of a skip: neither survives an error.
        for result in [errored(true, false), errored(false, false)] {
            assert!(result.failed(), "{result:?}");
            assert!(!result.passed() && !result.skipped(), "{result:?}");
        }
        let report = super::PrecisionReport {
            name: "canned".to_owned(),
            rules: Vec::new(),
            warnings: Vec::new(),
            replays: vec![errored(false, false)],
            known_flaky: None,
            ledger: None,
            impact: None,
            m5: None,
            not_measured: None,
        };
        assert!(section_failed(std::slice::from_ref(&report), RULE_FLOORS));
        let rendered = render(std::slice::from_ref(&report), RULE_FLOORS);
        assert!(
            rendered.contains(
                "  replay handler-drift  shipped absent  fixed absent  FAIL  error: shipped commit: git worktree add aaaaaaa: not a valid object\n"
            ),
            "{rendered}"
        );
    }

    /// A positive that names a `present_commit` (the recorded incident ref) is measured at that
    /// commit, not at the pin: its recall is over the findings the run reported when the incident
    /// was checked out, and a pin finding of the same rule and file does not reproduce it. Three
    /// such positives, two matched at the incident ref → recall 2 of 3, printed with no floor.
    #[test]
    fn a_positive_at_the_incident_ref_is_measured_against_the_incident_run() {
        let mut truth = truth_with(Vec::new());
        truth.findings = vec![
            positive_at("redundant-test", "a.test.ts", "061f8c1d"),
            positive_at("redundant-test", "b.test.ts", "061f8c1d"),
            positive_at("redundant-test", "c.test.ts", "061f8c1d"),
        ];
        // The pin reports a redundant-test finding on a.test.ts, but an incident positive is not
        // reproduced at the pin — it is measured only against the incident run.
        let pin = [reported_in(
            "redundant-test",
            "redundant-test a.test.ts deadbeef#0",
            "a.test.ts",
        )];
        let incident = [
            reported_in(
                "redundant-test",
                "redundant-test a.test.ts feed#0",
                "a.test.ts",
            ),
            reported_in(
                "redundant-test",
                "redundant-test b.test.ts feed#0",
                "b.test.ts",
            ),
        ];
        let report = super::evaluate_entry_with_incident(
            "canned",
            &["redundant-test".to_owned()],
            &pin,
            &incident,
            &truth,
        );
        let row = &report.rules[0];
        assert_eq!(row.positives, 3, "the three re-labelled pairs");
        assert_eq!(
            row.recalled, 2,
            "a.test.ts and b.test.ts fired at the incident ref"
        );
        assert!((row.recall().unwrap() - 2.0 / 3.0).abs() < 1e-9);
        assert!(
            render(std::slice::from_ref(&report), RULE_FLOORS).contains("recall 0.667"),
            "the incident recall figure prints with no floor applied"
        );
    }

    /// A recall floor judges k of n only at or above its minimum n: below it the set is too
    /// small to read a rate from, whatever k is (even n of n); at or above it, k/n at the
    /// floor passes and below it fails.
    #[test]
    fn a_recall_floor_is_insufficient_below_its_minimum_n() {
        let floor = RecallFloor {
            min_n: 10,
            pass_at: 0.5,
        };
        assert_eq!(floor.verdict(0, 0), RuleVerdict::Insufficient);
        assert_eq!(floor.verdict(9, 9), RuleVerdict::Insufficient);
        assert_eq!(floor.verdict(5, 10), RuleVerdict::Pass);
        assert_eq!(floor.verdict(4, 10), RuleVerdict::Fail);
        assert_eq!(floor.verdict(6, 11), RuleVerdict::Pass);
        assert_eq!(floor.verdict(5, 11), RuleVerdict::Fail);
    }

    /// Recall counts the recorded positives (present-at-pin) truth carries for a rule, and
    /// how many of them the run reproduced: two sites recorded, one reported → 1 of 2.
    #[test]
    fn recall_is_over_the_recorded_positives() {
        let mut truth = truth_with(Vec::new());
        truth.findings = vec![
            positive("generator-walk", Some("src/sagas/one.ts")),
            positive("generator-walk", Some("src/sagas/two.ts")),
            // A replay pair (has a fixed commit) is not a recall positive at the pin.
            Finding {
                rule: "generator-walk".to_owned(),
                anchor: Some("a pair".to_owned()),
                file: None,
                invariants: Vec::new(),
                shipped_commit: Some("aaa".to_owned()),
                fixed_commit: Some("bbb".to_owned()),
                present_commit: None,
                expected: Expected::Present,
            },
        ];
        let reported = [reported_in(
            "generator-walk",
            "generator-walk src/sagas/one.ts walk#0",
            "src/sagas/one.ts",
        )];
        let report = evaluate_entry("canned", &["generator-walk".to_owned()], &reported, &truth);
        let row = &report.rules[0];
        assert_eq!(row.positives, 2, "the two present-at-pin sites");
        assert_eq!(row.recalled, 1, "only site one was reported");
        assert!((row.recall().unwrap() - 0.5).abs() < 1e-9);
        assert!(
            render(std::slice::from_ref(&report), RULE_FLOORS).contains("recall 0.500"),
            "the ratio is printed, not a dash"
        );
    }

    /// A positive is reproduced when the run reported a decided finding of its rule located
    /// in its file; a finding of the same rule in another file does not reproduce it.
    #[test]
    fn a_positive_is_reproduced_by_a_decided_finding_of_its_rule_in_its_file() {
        let mut truth = truth_with(Vec::new());
        truth.findings = vec![positive("gate-truth", Some("apps/web/vitest.config.ts"))];
        let elsewhere = [reported_in(
            "gate-truth",
            "gate-truth apps/api/vitest.config.ts thresholds#0",
            "apps/api/vitest.config.ts",
        )];
        let report = evaluate_entry("canned", &["gate-truth".to_owned()], &elsewhere, &truth);
        assert_eq!(
            (report.rules[0].recalled, report.rules[0].positives),
            (0, 1)
        );

        let mut undecided = reported_in(
            "gate-truth",
            "gate-truth apps/web/vitest.config.ts thresholds#0",
            "apps/web/vitest.config.ts",
        );
        undecided.decided = false;
        let report = evaluate_entry(
            "canned",
            &["gate-truth".to_owned()],
            std::slice::from_ref(&undecided),
            &truth,
        );
        assert_eq!(
            report.rules[0].recalled, 0,
            "a could-not-decide row does not reproduce a positive"
        );

        let here = [reported_in(
            "gate-truth",
            "gate-truth apps/web/vitest.config.ts thresholds#0",
            "apps/web/vitest.config.ts",
        )];
        let report = evaluate_entry("canned", &["gate-truth".to_owned()], &here, &truth);
        assert_eq!(
            (report.rules[0].recalled, report.rules[0].positives),
            (1, 1)
        );
        assert!((report.rules[0].recall().unwrap() - 1.0).abs() < 1e-9);
    }

    /// A run that reported nothing for the rule reproduces none of its positives: the
    /// denominator stays, the numerator is zero, and recall prints 0.000, not a dash.
    #[test]
    fn a_positive_the_run_did_not_report_is_not_reproduced() {
        let mut truth = truth_with(Vec::new());
        truth.findings = vec![positive("dead-skip", Some("src/a.test.ts"))];
        let report = evaluate_entry("canned", &["dead-skip".to_owned()], &[], &truth);
        let row = &report.rules[0];
        assert_eq!((row.recalled, row.positives), (0, 1));
        assert!((row.recall().unwrap()).abs() < f64::EPSILON);
        assert!(render(std::slice::from_ref(&report), RULE_FLOORS).contains("recall 0.000"));
    }

    /// A row marked `expected: absent` is a true negative, not a positive: it is left out of
    /// the denominator, so a rule whose only rows are true negatives has no recall to print.
    #[test]
    fn an_expected_absent_row_is_not_in_the_recall_denominator() {
        let mut truth = truth_with(Vec::new());
        let mut negative = positive("gate-truth", Some("apps/api/stryker.config.json"));
        negative.expected = Expected::Absent;
        truth.findings = vec![
            negative,
            positive("gate-truth", Some("apps/web/vitest.config.ts")),
        ];
        // The run fires on both files: the true negative must not inflate the numerator
        // either.
        let reported = [
            reported_in(
                "gate-truth",
                "gate-truth apps/api/stryker.config.json break#0",
                "apps/api/stryker.config.json",
            ),
            reported_in(
                "gate-truth",
                "gate-truth apps/web/vitest.config.ts thresholds#0",
                "apps/web/vitest.config.ts",
            ),
        ];
        let report = evaluate_entry("canned", &["gate-truth".to_owned()], &reported, &truth);
        assert_eq!(
            (report.rules[0].recalled, report.rules[0].positives),
            (1, 1)
        );

        // Only true negatives recorded → nothing to divide, the column prints a dash.
        let mut only_negative = positive("gate-truth", Some("apps/api/stryker.config.json"));
        only_negative.expected = Expected::Absent;
        truth.findings = vec![only_negative];
        let report = evaluate_entry("canned", &["gate-truth".to_owned()], &reported, &truth);
        assert_eq!(
            (report.rules[0].recalled, report.rules[0].positives),
            (0, 0)
        );
        assert!(report.rules[0].recall().is_none());
    }

    /// A positive that names invariants is reproduced only by a finding whose id carries one
    /// of them — the same narrowing a replay pair applies, so a third invariant firing in
    /// the same file does not read as the recorded one.
    #[test]
    fn an_invariant_narrowed_positive_matches_only_an_id_that_carries_the_invariant() {
        let mut truth = truth_with(Vec::new());
        let mut narrowed = positive("fail-closed-setup", Some("src/test-setup.ts"));
        narrowed.invariants = vec!["console-guard".to_owned()];
        truth.findings = vec![narrowed];

        let other = [reported_in(
            "fail-closed-setup",
            "fail-closed-setup src/test-setup.ts mock-reset#0",
            "src/test-setup.ts",
        )];
        let report = evaluate_entry("canned", &["fail-closed-setup".to_owned()], &other, &truth);
        assert_eq!(
            (report.rules[0].recalled, report.rules[0].positives),
            (0, 1)
        );

        let named = [reported_in(
            "fail-closed-setup",
            "fail-closed-setup src/test-setup.ts console-guard#0",
            "src/test-setup.ts",
        )];
        let report = evaluate_entry("canned", &["fail-closed-setup".to_owned()], &named, &truth);
        assert_eq!(
            (report.rules[0].recalled, report.rules[0].positives),
            (1, 1)
        );
    }

    /// A positive that records no file cannot be located, so it is left out of recall
    /// rather than counted as missed: a corpus `duplicate-test` row recorded as one aggregate
    /// over 84 re-looped tests with `file: null` can never be matched by a finding, so counting it
    /// printed recall 0.000 whatever the rule reported. A located positive the
    /// rule missed still counts against it.
    #[test]
    fn a_positive_with_no_file_is_left_out_of_recall_and_a_located_miss_still_counts() {
        // The aggregate row as the corpus records it: no file, no line, a shipped commit.
        let corpus = Truth::parse(
            r#"{ "access": "local", "budget_ms": 5000, "findings": [
                { "rule": "duplicate-test", "file": null, "line": null,
                  "shipped_commit": "0123456789abcdef0123456789abcdef01234567",
                  "note": "one aggregate over 84 re-looped tests" },
                { "rule": "weak-assertion", "file": "packages/c/src/z.test.ts" }
            ] }"#,
        )
        .unwrap();
        let mut truth = truth_with(Vec::new());
        truth.findings = corpus
            .findings
            .into_iter()
            .filter(|finding| finding.rule == "duplicate-test")
            .collect();
        assert_eq!(truth.findings.len(), 1, "the aggregate row");
        assert_eq!(truth.findings[0].file, None);
        let reported = [reported_in(
            "duplicate-test",
            "duplicate-test packages/a/src/x.test.ts group#0",
            "packages/a/src/x.test.ts",
        )];
        let rules = ["duplicate-test".to_owned()];

        let report = evaluate_entry("sample-monorepo", &rules, &reported, &truth);
        assert_eq!(
            (report.rules[0].recalled, report.rules[0].positives),
            (0, 0)
        );
        assert_eq!(
            report.rules[0].recall(),
            None,
            "no measurable positive: no recall"
        );

        truth
            .findings
            .push(positive("duplicate-test", Some("packages/b/src/y.test.ts")));
        let report = evaluate_entry("sample-monorepo", &rules, &reported, &truth);
        assert_eq!(
            (report.rules[0].recalled, report.rules[0].positives),
            (0, 1),
            "the rule missed the located positive"
        );
    }

    /// The one predicate replay and recall share: rule, decided, file (empty scopes to the
    /// tree), invariants (empty scopes to any finding in the file).
    #[test]
    fn matches_scopes_by_rule_decision_file_and_invariants() {
        let finding = reported_in(
            "fail-closed-setup",
            "fail-closed-setup src/test-setup.ts console-guard#0",
            "src/test-setup.ts",
        );
        assert!(
            finding.matches("fail-closed-setup", "", &[], None),
            "tree scope"
        );
        assert!(finding.matches("fail-closed-setup", "src/test-setup.ts", &[], None));
        assert!(
            !finding.matches("dead-skip", "src/test-setup.ts", &[], None),
            "rule"
        );
        assert!(
            !finding.matches("fail-closed-setup", "src/other-setup.ts", &[], None),
            "file"
        );
        assert!(finding.matches(
            "fail-closed-setup",
            "src/test-setup.ts",
            &["mock-reset".to_owned(), "console-guard".to_owned()],
            None
        ));
        assert!(
            !finding.matches(
                "fail-closed-setup",
                "src/test-setup.ts",
                &["mock-reset".to_owned()],
                None
            ),
            "invariant"
        );
        let mut undecided = finding;
        undecided.decided = false;
        assert!(
            !undecided.matches("fail-closed-setup", "", &[], None),
            "decided"
        );
    }

    /// A replay pair carries the finding's file so the present-check can scope to it; a pair
    /// with no file scopes to the whole tree. This is what keeps a broadly-firing rule
    /// (dead-skip) from reading as "present" at the fixed commit on an unrelated finding.
    #[test]
    fn a_replay_pair_carries_the_findings_file() {
        let mut truth = truth_with(Vec::new());
        truth.findings = vec![
            Finding {
                rule: "dead-skip".to_owned(),
                anchor: Some("describe.skipIf(!!process.env.CI)".to_owned()),
                file: Some("src/__tests__/msw-saga-integration.test.tsx".to_owned()),
                invariants: Vec::new(),
                shipped_commit: Some("aaa".to_owned()),
                fixed_commit: Some("bbb".to_owned()),
                present_commit: None,
                expected: Expected::Present,
            },
            Finding {
                rule: "handler-drift".to_owned(),
                anchor: Some("*/api/v1".to_owned()),
                file: None,
                invariants: Vec::new(),
                shipped_commit: Some("ccc".to_owned()),
                fixed_commit: Some("ddd".to_owned()),
                present_commit: None,
                expected: Expected::Present,
            },
        ];
        let pairs = replay_pairs(&truth);
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].file, "src/__tests__/msw-saga-integration.test.tsx");
        assert_eq!(pairs[1].file, "", "a pair with no file scopes to the tree");
    }

    /// Corpus verdict sums adjudications across entries: 12 + 12 true (of 12 + 12) reaches
    /// the 20-across-the-corpus floor and passes where each entry alone is INSUFFICIENT.
    #[test]
    fn corpus_verdict_sums_across_entries() {
        let entry = |name: &str| {
            let ids: Vec<String> = (0..12)
                .map(|i| format!("weak-assertion {name}{i}"))
                .collect();
            let reported = reported(
                "weak-assertion",
                &ids.iter().map(String::as_str).collect::<Vec<_>>(),
            );
            let adjudications = ids.iter().map(|id| adj(id, Verdict::True)).collect();
            evaluate_entry(
                name,
                &["weak-assertion".to_owned()],
                &reported,
                &truth_with(adjudications),
            )
        };
        let reports = [entry("a"), entry("b")];
        assert_eq!(reports[0].rules[0].verdict, RuleVerdict::Insufficient);
        assert_eq!(corpus_verdicts(&reports)[0].1, RuleVerdict::Pass);
    }

    /// The 27/30 line renders the interval the acceptance criteria name.
    #[test]
    fn render_prints_the_interval_and_verdict() {
        let ids: Vec<String> = (0..30).map(|i| format!("weak-assertion f{i:02}")).collect();
        let reported = reported(
            "weak-assertion",
            &ids.iter().map(String::as_str).collect::<Vec<_>>(),
        );
        let adjudications = ids
            .iter()
            .enumerate()
            .map(|(i, id)| {
                adj(
                    id,
                    if i < 27 {
                        Verdict::True
                    } else {
                        Verdict::False
                    },
                )
            })
            .collect();
        let report = evaluate_entry(
            "canned",
            &["weak-assertion".to_owned()],
            &reported,
            &truth_with(adjudications),
        );
        let rendered = render(std::slice::from_ref(&report), RULE_FLOORS);
        assert!(rendered.contains("precision 0.900"), "{rendered}");
        assert!(rendered.contains("[0.744, 0.965]"), "{rendered}");
        assert!(rendered.contains("PASS"), "{rendered}");
    }

    #[test]
    fn wilson_interval_of_no_data_is_a_point_at_zero() {
        let interval = wilson_interval(0, 0);
        assert!(interval.low.abs() < f64::EPSILON);
        assert!(interval.high.abs() < f64::EPSILON);
    }

    #[test]
    fn verdict_thresholds() {
        assert_eq!(verdict(0, 0, 0, 0), RuleVerdict::Skipped);
        assert_eq!(verdict(19, 19, 15, 4), RuleVerdict::Insufficient);
        assert_eq!(verdict(30, 30, 15, 4), RuleVerdict::Unadjudicated);
        assert_eq!(verdict(30, 30, 27, 3), RuleVerdict::Pass);
        assert_eq!(verdict(30, 30, 26, 4), RuleVerdict::Fail);
    }

    #[test]
    fn reported_findings_marks_a_could_not_decide_row_as_undecided() {
        // The replay present-check counts only decided findings, so a `could_not_decide`
        // outcome (handler-drift on an opaque base) is parsed as `decided: false` and a
        // missing outcome as decided.
        let json = serde_json::json!({
            "findings": [
                { "rule": "handler-drift", "id": "handler-drift a */api/v1#0", "file": "src/mocks/handlers/sops.ts", "outcome": "finding" },
                { "rule": "handler-drift", "id": "handler-drift b */x#0", "outcome": "could_not_decide" },
                { "rule": "weak-assertion", "id": "weak-assertion c#0" },
                { "rule": "dead-skip", "id": "dead-skip d listed:t#0", "outcome": "listed" },
            ]
        });
        let reported = reported_findings(&json);
        assert_eq!(reported.len(), 3, "a listed row is not a reported finding");
        assert!(
            reported[0].decided,
            "an explicit finding outcome is decided"
        );
        assert_eq!(
            reported[0].file, "src/mocks/handlers/sops.ts",
            "the finding's file is read for replay file-scoping"
        );
        assert!(!reported[1].decided, "could_not_decide is undecided");
        assert!(
            reported[2].decided,
            "a row with no outcome reads as decided"
        );
    }
}
