//! The precision harness as a library: discover corpus entries, run `spirolo list` over the
//! reachable ones, and diff the per-framework counts against each entry's `truth.json`.
//!
//! This is M1's done-condition expressed as code. The binary ([`crate`]'s `main`) wires
//! [`run_corpus`] to the real `spirolo` executable; tests wire it to a canned runner, so the
//! diff, the verdict and the table are exercised without cloning a corpus repo.
//!
//! `model ← frameworks ← … ← bench`: the harness (this library and the `spirolo-bench`
//! binary) never links the model. It reads the same `spirolo list --format json` document any
//! other consumer gets and tallies that. The crate's criterion bench
//! (`benches/parse_extract.rs`) is the one exception: it links `spirolo-model` and
//! `spirolo-frameworks` as dev-dependencies to time `Model::build_with` on a fixture.

mod clone;
mod counts;
mod impact;
mod known_flaky;
mod ledger;
mod m5;
mod nightly;
mod pool;
mod precision;
mod selection;
pub use crate::pool::{pooled_sample, PoolEntry, ENTRY_CAP, POOL_TARGET};
pub use crate::selection::{held_out_names, memberships, Membership};
mod report;
mod staging;
mod status;
mod substrate;
mod truth;

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::Value;

pub use crate::clone::ensure_commit;
pub use crate::counts::{by_framework, by_runner_record, tally, FrameworkCount};
pub use crate::impact::{
    hotspot_ranking, render_impact, score_entry_impact, score_hotspots, score_impact, FileScore,
    FileSet, HotspotRow, HotspotScore, ImpactBudget, ImpactEntryRunners, ImpactMutant, ImpactRows,
    ImpactRunner, ImpactScore, ImpactTruth, MutantResult,
};
pub use crate::known_flaky::{
    score_known_flaky, CheckAt, FixSide, KnownFlaky, KnownFlakyDropped, KnownFlakyRow,
    KnownFlakyRowResult, KnownFlakyScore,
};
pub use crate::ledger::{
    render_ledger, score_entry_ledger, score_ledger, LedgerFlake, LedgerRun, LedgerRunner,
    LedgerScore, LedgerTruth, LiveRow, LiveStatus, QuarantineRoundTrip,
};
pub use crate::m5::{
    explain_round_trip, render_m5, status_mismatches, DiagnosticDiff, ExplainAnswer, ExplainRunner,
    ExplainScore, HoverMiss, HoverScore, LiveEdit, Loud, M5Budget, M5Rows, M5Summary, PanelScore,
    SaveBudget, TreeScore, LOAD_CEILING, MIN_EDITS, UNKNOWN_ID,
};
pub use crate::nightly::{
    nightly_counts, nightly_entries, render_nightly, score_nightly, NightlyScore,
};
pub use crate::precision::{
    corpus_verdicts, evaluate_entry, evaluate_entry_with_incident, render as render_precision,
    replay_pairs, reported_findings, section_failed, stratified_sample, verdict, wilson_interval,
    Carried, Interval, JoinWarning, PrecisionReport, RecallFloor, ReplayResult, ReportedFinding,
    RuleFloor, RulePrecision, RuleVerdict, Stratum, KNOWN_FLAKY_FLOOR, RULE_FLOORS,
};
pub use crate::report::{
    any_failed, evaluate, render, EntryReport, FrameworkRow, OpenFile, RunnerRow, Status, Timing,
    Wallclock, IDLE_BUSY_CEILING,
};
pub use crate::staging::{
    stage_inputs, stage_inputs_in_place, stage_runs, StagedInputs, StagedRuns,
};
pub use crate::status::{partition_precision, policy_tables, rule_status, REGISTERED_RULES};
pub use crate::substrate::substrate_of;
pub use crate::truth::{
    Access, Adjudication, ExcludedFile, ExcludedPackage, Expected, Finding, FrameworkCounts,
    Incident, IncidentCounts, IncidentFile, IncidentPair, IncidentTest, ReplayNone, ReplayPair,
    RunnerCounts, RunnerExcludedFile, StratifiedSample, Truth, Verdict,
};

/// Why the run measured nothing, or `None` when at least one entry was measured.
///
/// A corpus whose every entry is pending or un-cloned produces only skipped counts rows and
/// empty precision reports; the verdicts over it are vacuous (`0 of 0 rules`, `none
/// failed`) and must not read as a pass. Nothing was measured when no entry produced a
/// counts row (every entry skipped, or no entry at all) or no entry produced a precision
/// row (every [`PrecisionReport::rules`] empty). The message names the entries by why they
/// were skipped, so the operator knows whether to run `just corpus` or wait on access. The
/// binary prints it to stderr and exits with its error code; the decision lives here so it
/// is testable without a process.
#[must_use]
pub fn nothing_measured(counts: &[EntryReport], precision: &[PrecisionReport]) -> Option<String> {
    if counts.is_empty() {
        return Some("no corpus entry found (a directory with a truth.json)".to_owned());
    }
    if counts.iter().all(|report| report.status == Status::Skipped) {
        // Group the skipped entries by the reason before the colon of their note
        // (`pending: …`, `not cloned: …`), in first-seen order.
        let mut reasons: Vec<(&str, usize)> = Vec::new();
        for report in counts {
            let reason = report
                .note
                .as_deref()
                .and_then(|note| note.split(':').next())
                .unwrap_or("skipped");
            match reasons.iter_mut().find(|(known, _)| *known == reason) {
                Some((_, count)) => *count += 1,
                None => reasons.push((reason, 1)),
            }
        }
        let breakdown: Vec<String> = reasons
            .iter()
            .map(|(reason, count)| format!("{count} {reason}"))
            .collect();
        return Some(format!(
            "{} skipped: {}",
            plural_entries(counts.len()),
            breakdown.join(", ")
        ));
    }
    if precision.iter().all(|report| report.rules.is_empty()) {
        return Some(format!(
            "{} counted but no entry produced a precision row",
            plural_entries(counts.len())
        ));
    }
    None
}

/// `1 entry` or `n entries`.
fn plural_entries(n: usize) -> String {
    if n == 1 {
        "1 entry".to_owned()
    } else {
        format!("{n} entries")
    }
}

/// `spirolo list --format json` over one entry: its parsed document and its timed runs.
pub struct Run {
    /// The parsed JSON `spirolo list --format json` wrote to stdout.
    pub json: Value,
    /// Wall-clock time of each timed run; their median is checked against the budget.
    pub samples: Vec<Duration>,
    /// The fraction of all CPUs busy just before the timed runs, `None` when unreadable.
    pub busy: Option<f64>,
}

/// A function that runs `spirolo list --format json` over a repo and returns its document.
///
/// The harness owns *what* to run and *how to judge it*; the caller owns *how to run it*.
/// The binary shells out to `spirolo`; a test hands back a canned [`Run`]. An `Err` means the
/// command never produced a usable document — the entry fails with that message.
pub type ListRunner<'a> = dyn Fn(&Path) -> Result<Run, String> + 'a;

/// Every corpus entry under `corpus_dir`: a subdirectory holding a `truth.json`, by name.
///
/// Sorted, so the table is byte-stable across machines and filesystems. A subdirectory
/// without a `truth.json` (a stray, or the gitignored `repo/` of an entry read by its
/// parent) is not an entry.
pub fn discover(corpus_dir: &Path) -> Vec<(String, PathBuf)> {
    let mut entries = Vec::new();
    let held_out = match held_out_names(corpus_dir) {
        Ok(names) => names,
        Err(error) => {
            eprintln!("spirolo-bench: corpus selection: {error}");
            return entries;
        }
    };
    let Ok(read_dir) = std::fs::read_dir(corpus_dir) else {
        return entries;
    };
    for item in read_dir.flatten() {
        let dir = item.path();
        if dir
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| held_out.contains(n))
        {
            continue;
        }
        if dir.join("truth.json").is_file() {
            if let Some(name) = dir.file_name().and_then(|name| name.to_str()) {
                entries.push((name.to_owned(), dir));
            }
        }
    }
    entries.sort();
    entries
}

/// Run the whole corpus and return one report per entry, in name order.
///
/// A pending entry, or a local entry whose `repo/` clone is absent, is skipped rather than
/// failed — an unreachable repo cannot turn the harness red. A local entry with a clone is
/// measured through `run_list` and diffed against its truth, unless its clone is not on the
/// substrate the truth records ([`substrate_of`]): that entry fails naming both, since its
/// counts were recorded on another substrate. A `truth.json` that will not
/// parse is the one hard error: it is committed ground truth and must be valid.
pub fn run_corpus(corpus_dir: &Path, run_list: &ListRunner) -> Result<Vec<EntryReport>, String> {
    let mut reports = Vec::new();
    for (name, dir) in discover(corpus_dir) {
        let truth =
            Truth::read(&dir.join("truth.json")).map_err(|error| format!("{name}: {error}"))?;

        if truth.access == Access::Pending {
            reports.push(EntryReport::skipped(
                name,
                "pending: not reachable here, counts copied from a document of record",
            ));
            continue;
        }

        let repo = dir.join("repo");
        if !repo.is_dir() {
            reports.push(EntryReport::skipped(
                name,
                "not cloned: run `just corpus` to fetch the pinned commit",
            ));
            continue;
        }

        // Counts recorded on one substrate are not compared on another (M3 reopen, #130).
        if let Some(recorded) = &truth.substrate {
            let clone = substrate_of(&repo);
            if clone != *recorded {
                reports.push(EntryReport::errored(
                    name,
                    format!(
                        "substrate mismatch: truth.json records {recorded}, the clone is {clone}"
                    ),
                ));
                continue;
            }
        }

        match run_list(&repo) {
            Ok(run) => {
                let actual = by_framework(
                    &run.json,
                    &truth.excluded_paths(),
                    &truth.excluded_file_paths(),
                );
                let runner_actual = by_runner_record(&run.json, &truth.runner);
                let wallclock = Wallclock {
                    samples: run.samples,
                    busy: run.busy,
                };
                reports.push(evaluate(name, &truth, &actual, &runner_actual, wallclock));
            }
            Err(error) => reports.push(EntryReport::errored(
                name,
                format!("spirolo list did not produce a usable document: {error}"),
            )),
        }
    }
    Ok(reports)
}

/// A function that runs `spirolo check --format json --config <cfg> <repo>`.
///
/// Returns the parsed document. The binary shells out to `spirolo`; a test hands back a
/// canned document. An `Err` means the command never produced a usable document.
pub type CheckRunner<'a> = dyn Fn(&Path, &Path) -> Result<Value, String> + 'a;

/// A function that answers whether a rule's finding is present at a commit of the clone.
///
/// The binary checks the finding out in a temporary git worktree and re-runs `check`; a
/// test hands back a canned answer. The arguments are the clone, the pair, and the commit;
/// an `Err` fails the replay pair (a missing commit, a worktree that would not create).
pub type ReplayRunner<'a> = dyn Fn(&Path, &ReplayPair, &str) -> Result<bool, String> + 'a;

/// A function that returns the run's findings at a commit of the clone.
///
/// The binary checks the commit out in a temporary git worktree, stages the commit's
/// `inputs/`, re-runs `check` there and reduces the document to [`ReportedFinding`]s; a test
/// hands back a canned list. Its uses are the incident-ref recall (a `redundant-test`
/// positive on the recorded incident is reproduced by these findings, not by the pin's) and
/// the known-flaky rows, each checked on a bare worktree at its `flaky_commit` and
/// `fix_commit`. An `Err` means the commit could not be measured: the incident recall
/// reads it as no findings (the positives print as missed) rather than aborting the whole
/// section, and a known-flaky row carries it as its error and fails the set.
pub type CommitCheckRunner<'a> = dyn Fn(&Path, &str) -> Result<Vec<ReportedFinding>, String> + 'a;

/// Run the precision section over the whole corpus: per entry, run `check` and join its
/// findings to the entry's adjudications, then replay each recorded commit pair.
///
/// `rules` is the registry order the section prints in, so a rule that reported nothing on
/// every entry still shows `SKIPPED`. A pending or un-cloned entry is skipped, exactly as
/// the counts section skips it. A replay pair whose runner errors at either commit carries
/// the error and fails, so the pair fails rather than vanishing. An entry whose clone is not
/// on the substrate its truth records ([`substrate_of`]) gets no rule rows and says why.
pub fn run_precision(
    corpus_dir: &Path,
    rules: &[String],
    run_check: &CheckRunner,
    replay: &ReplayRunner,
    incident_check: &CommitCheckRunner,
) -> Result<Vec<PrecisionReport>, String> {
    let mut reports = Vec::new();
    let membership = memberships(corpus_dir)?;
    let mut pool = Vec::new();
    let mut incidents = std::collections::BTreeMap::new();
    for (name, dir) in discover(corpus_dir) {
        let truth =
            Truth::read(&dir.join("truth.json")).map_err(|error| format!("{name}: {error}"))?;

        let repo = dir.join("repo");
        if truth.access == Access::Pending || !repo.is_dir() {
            // The section still lists the entry, but with no rows: nothing was measured.
            reports.push(PrecisionReport {
                name,
                rules: Vec::new(),
                warnings: Vec::new(),
                replays: Vec::new(),
                known_flaky: None,
                ledger: None,
                impact: None,
                m5: None,
                not_measured: None,
            });
            continue;
        }

        // A clone off the substrate its truth records reports another population: its rules
        // are not measured, so its findings cannot enter a corpus verdict (#130). Its replay
        // pairs still run; they check other commits out in a worktree of their own.
        if let Some(recorded) = &truth.substrate {
            let clone = substrate_of(&repo);
            if clone != *recorded {
                // Like the replay pairs, the known-flaky rows check other commits in a
                // worktree of their own, so they are still scored.
                let known_flaky = score_entry_known_flaky(&dir, &repo, incident_check)
                    .map_err(|error| format!("{name}: {error}"))?;
                reports.push(PrecisionReport {
                    name,
                    rules: Vec::new(),
                    warnings: Vec::new(),
                    replays: run_replays(&repo, &truth, replay),
                    known_flaky,
                    ledger: None,
                    impact: None,
                    m5: None,
                    not_measured: Some(format!(
                        "substrate mismatch: truth.json records {recorded}, the clone is {clone}"
                    )),
                });
                continue;
            }
        }

        let config = dir.join("spirolorc.jsonc");
        let reported = match run_check(&repo, &config) {
            Ok(json) => reported_findings(&json),
            Err(error) => {
                return Err(format!(
                    "{name}: spirolo check did not produce a document: {error}"
                ));
            }
        };

        // An entry that records an incident has recall positives measured there, not
        // at the pin: check the incident ref out (staged) and reduce it to findings. A failure
        // to measure it is not fatal — the positives print as missed, loudly on stderr.
        let incident_reported = truth.incident.as_ref().map_or_else(Vec::new, |incident| {
            incident_check(&repo, &incident.sha).unwrap_or_else(|error| {
                eprintln!(
                    "spirolo-bench: {name}: incident recall check at {}: {error}",
                    incident.sha
                );
                Vec::new()
            })
        });

        let mut report =
            evaluate_entry_with_incident(&name, rules, &reported, &incident_reported, &truth);
        report.replays = run_replays(&repo, &truth, replay);
        report.known_flaky = score_entry_known_flaky(&dir, &repo, incident_check)
            .map_err(|error| format!("{name}: {error}"))?;
        if let Some(member) = membership
            .get(&name)
            .filter(|m| m.source == "oss" && m.set == "dev")
        {
            incidents.insert(name.clone(), incident_reported);
            pool.push(PoolEntry {
                name: name.clone(),
                source: member.source.clone(),
                set: member.set.clone(),
                reported,
                truth,
            });
        }
        reports.push(report);
    }
    apply_pool(&mut reports, &pool, &incidents, rules);
    Ok(reports)
}

fn apply_pool(
    reports: &mut [PrecisionReport],
    pool: &[PoolEntry],
    incidents: &std::collections::BTreeMap<String, Vec<ReportedFinding>>,
    rules: &[String],
) {
    let mut draws: std::collections::BTreeMap<
        String,
        std::collections::BTreeMap<String, std::collections::BTreeSet<String>>,
    > = std::collections::BTreeMap::new();
    for rule in rules {
        for (entry, finding) in pooled_sample(pool, rule) {
            draws
                .entry(entry)
                .or_default()
                .entry(rule.clone())
                .or_default()
                .insert(finding.id);
        }
    }
    for entry in pool {
        let empty = std::collections::BTreeMap::new();
        let mut pooled = crate::precision::evaluate_with_draw(
            entry.name.clone(),
            rules,
            &entry.reported,
            &incidents[&entry.name],
            &entry.truth,
            Some(draws.get(&entry.name).unwrap_or(&empty)),
        );
        let report = reports
            .iter_mut()
            .find(|r| r.name == entry.name)
            .expect("pooled entry has a report");
        pooled.replays = std::mem::take(&mut report.replays);
        pooled.known_flaky = report.known_flaky.take();
        *report = pooled;
    }
}

/// Score an entry's `known-flaky.json`, when it records one, against `flake-candidate`:
/// each row is checked through `check_at` (a bare staged worktree of the clone at the
/// commit, as the incident ref is) at its `flaky_commit` and, when flagged, its
/// `fix_commit`. The file is read, never written.
///
/// # Errors
/// When the entry's `known-flaky.json` exists but is not a valid set.
fn score_entry_known_flaky(
    dir: &Path,
    repo: &Path,
    check_at: &CommitCheckRunner,
) -> Result<Option<KnownFlakyScore>, String> {
    let path = dir.join("known-flaky.json");
    if !path.is_file() {
        return Ok(None);
    }
    let set = KnownFlaky::read(&path)?;
    Ok(Some(score_known_flaky(&set, &|commit: &str| {
        check_at(repo, commit)
    })))
}

/// Replay each recorded commit pair of an entry: the finding must be present at the shipped
/// commit and absent at the fixed one. A runner error at either commit is recorded on the
/// result and makes the pair a failure, so an un-checkable pair fails loudly rather than
/// reading as absent-at-both and skipping silently. Both ends are still checked, so a pair
/// that errors twice reports both messages.
fn run_replays(repo: &Path, truth: &Truth, replay: &ReplayRunner) -> Vec<ReplayResult> {
    replay_pairs(truth)
        .into_iter()
        .map(|pair| {
            let mut errors = Vec::new();
            // An errored end records the failing answer for that end (absent at shipped,
            // present at fixed); the error itself is what fails the pair.
            let present_at_shipped =
                replay(repo, &pair, &pair.shipped_commit).unwrap_or_else(|error| {
                    errors.push(format!("shipped commit: {error}"));
                    false
                });
            let present_at_fixed =
                replay(repo, &pair, &pair.fixed_commit).unwrap_or_else(|error| {
                    errors.push(format!("fixed commit: {error}"));
                    true
                });
            ReplayResult {
                rule: pair.rule,
                anchor: pair.anchor,
                present_at_shipped,
                present_at_fixed,
                error: (!errors.is_empty()).then(|| errors.join("; ")),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{nothing_measured, EntryReport, PrecisionReport, RulePrecision, RuleVerdict};

    fn precision(name: &str, rules: Vec<RulePrecision>) -> PrecisionReport {
        PrecisionReport {
            name: name.to_owned(),
            rules,
            warnings: Vec::new(),
            replays: Vec::new(),
            known_flaky: None,
            ledger: None,
            impact: None,
            m5: None,
            not_measured: None,
        }
    }

    fn rule_row(rule: &str) -> RulePrecision {
        RulePrecision {
            rule: rule.to_owned(),
            reported: 0,
            executable: 0,
            sampled: 0,
            truths: 0,
            falses: 0,
            open: 0,
            could_not_decide: 0,
            inspected_truths: 0,
            inspected_falses: 0,
            recalled: 0,
            positives: 0,
            verdict: RuleVerdict::Skipped,
            strata: Vec::new(),
        }
    }

    /// Every entry skipped → nothing measured, with the skips broken down by reason in
    /// first-seen order; the message is what the binary prints before exiting non-zero.
    #[test]
    fn every_entry_skipped_is_nothing_measured_with_the_reasons_counted() {
        let counts = [
            EntryReport::skipped(
                "a",
                "not cloned: run `just corpus` to fetch the pinned commit",
            ),
            EntryReport::skipped("b", "pending: not reachable here"),
            EntryReport::skipped(
                "c",
                "not cloned: run `just corpus` to fetch the pinned commit",
            ),
        ];
        let precision = [
            precision("a", Vec::new()),
            precision("b", Vec::new()),
            precision("c", Vec::new()),
        ];
        assert_eq!(
            nothing_measured(&counts, &precision).as_deref(),
            Some("3 entries skipped: 2 not cloned, 1 pending")
        );
        assert_eq!(
            nothing_measured(&counts[1..2], &precision[1..2]).as_deref(),
            Some("1 entry skipped: 1 pending")
        );
    }

    /// No entry at all is nothing measured too: an empty corpus directory must not pass.
    #[test]
    fn an_empty_corpus_is_nothing_measured() {
        assert_eq!(
            nothing_measured(&[], &[]).as_deref(),
            Some("no corpus entry found (a directory with a truth.json)")
        );
    }

    /// A counted entry whose precision report carries no rule row is still nothing
    /// measured on the precision side; one rule row anywhere is a measurement.
    #[test]
    fn a_counted_entry_with_no_precision_row_is_nothing_measured() {
        let counts = [
            EntryReport::errored("a", "spirolo list did not produce a usable document"),
            EntryReport::skipped("b", "pending: not reachable here"),
        ];
        let empty = [precision("a", Vec::new()), precision("b", Vec::new())];
        assert_eq!(
            nothing_measured(&counts, &empty).as_deref(),
            Some("2 entries counted but no entry produced a precision row")
        );

        let measured = [
            precision("a", vec![rule_row("weak-assertion")]),
            precision("b", Vec::new()),
        ];
        assert_eq!(nothing_measured(&counts, &measured), None);
    }
}
