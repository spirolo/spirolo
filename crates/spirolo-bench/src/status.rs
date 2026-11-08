//! The rule-status file (PRD 0006, "The rule-status file"): what the surfaces say about each
//! rule's measured precision, generated from this run's own verdicts.
//!
//! `spirolo-bench status --write` writes it to `corpus/rule-status.json`; the binary embeds that
//! file, so a surface can never claim a precision the corpus did not measure.

use std::collections::BTreeMap;

use serde_json::{json, Map, Value};

use crate::precision::{corpus_verdicts, wilson_interval, PrecisionReport, RuleFloor, RuleVerdict};

/// Pooled executed verdicts: `(truths, falses)`.
type Counts = (u64, u64);

/// The status file for `reports`: every registered rule, its pooled verdict, where its floor
/// was carried, and its pooled precision and interval over the executed verdicts.
///
/// `spirolo_commit` is the sha the bench ran; `entries` are the measured entries as
/// `name@short-commit`.
#[must_use]
pub fn rule_status(
    reports: &[PrecisionReport],
    floors: &[RuleFloor],
    spirolo_commit: &str,
    entries: &[String],
) -> Value {
    let verdicts: BTreeMap<String, &'static str> = corpus_verdicts(reports)
        .into_iter()
        .map(|(rule, verdict)| (rule, verdict.tag()))
        .collect();
    let mut counts: BTreeMap<&str, Counts> = BTreeMap::new();
    for row in reports.iter().flat_map(|report| report.rules.iter()) {
        let pooled = counts.entry(row.rule.as_str()).or_default();
        pooled.0 += row.truths;
        pooled.1 += row.falses;
    }

    let mut rows: BTreeMap<&str, (&'static str, Counts)> = BTreeMap::new();
    if let Some(ledger) = ledger_row(reports) {
        rows.insert(LEDGER_RULE, ledger);
    }
    if let Some(hotspots) = hotspot_row(reports) {
        rows.insert(HOTSPOT_RULE, hotspots);
    }

    let mut rules = Map::new();
    for rule in REGISTERED_RULES {
        let (verdict, (truths, falses)) = rows.get(rule).copied().unwrap_or_else(|| {
            (
                verdicts.get(*rule).copied().unwrap_or("UNMEASURED"),
                counts.get(rule).copied().unwrap_or_default(),
            )
        });
        let carried = floors
            .iter()
            .find(|floor| floor.rule == *rule)
            .and_then(|floor| floor.carried)
            .map(|carried| carried.to)
            .or_else(|| {
                CARRIED_ROWS
                    .iter()
                    .find(|(row_rule, _)| row_rule == rule)
                    .map(|(_, to)| *to)
            });
        let mut entry = status_entry(verdict, carried, truths, falses);
        let measured_on: Vec<_> = reports
            .iter()
            .filter(|r| {
                r.rules
                    .iter()
                    .any(|row| row.rule == *rule && row.reported > 0)
                    || (*rule == LEDGER_RULE && r.ledger.is_some())
                    || (*rule == HOTSPOT_RULE && r.impact.is_some())
            })
            .map(|r| r.name.clone())
            .collect();
        entry["measured_on"] = json!(measured_on);
        rules.insert((*rule).to_owned(), entry);
    }
    json!({
        "schema": 1,
        "spirolo_commit": spirolo_commit,
        "entries": entries,
        "rules": rules,
    })
}

/// Separate OSS-dev release measurements from private regression measurements.
///
/// Both the printed bench summaries and embedded status use these populations, so
/// private precision cannot prove or refute an OSS release verdict. Held-out and
/// unknown OSS membership are excluded from both populations.
#[must_use]
pub fn partition_precision(
    reports: &[PrecisionReport],
    membership: &BTreeMap<String, crate::Membership>,
) -> (Vec<PrecisionReport>, Vec<PrecisionReport>) {
    let policy: Vec<_> = reports
        .iter()
        .filter(|r| {
            membership
                .get(&r.name)
                .is_some_and(|m| m.source == "oss" && m.set == "dev")
        })
        .cloned()
        .collect();
    let private: Vec<_> = reports
        .iter()
        .filter(|r| {
            membership
                .get(&r.name)
                .is_none_or(|m| m.source.is_empty() || m.source == "private")
        })
        .cloned()
        .collect();
    (policy, private)
}

/// Generate the embedded OSS-dev policy and a separate private regression table.
/// Held-out and unknown membership are never admitted to release policy.
#[must_use]
pub fn policy_tables(
    reports: &[PrecisionReport],
    membership: &BTreeMap<String, crate::Membership>,
    floors: &[RuleFloor],
    spirolo_commit: &str,
    entries: &[String],
) -> (Value, Value) {
    let (policy, private) = partition_precision(reports, membership);
    let table = |reports: &[PrecisionReport]| {
        let names: Vec<_> = entries
            .iter()
            .filter(|e| reports.iter().any(|r| e.split('@').next() == Some(&r.name)))
            .cloned()
            .collect();
        rule_status(reports, floors, spirolo_commit, &names)
    };
    (table(&policy), table(&private))
}

/// The rule the M4 ledger row measures.
const LEDGER_RULE: &str = "flake-observed";

/// `flake-observed`'s row pooled over every entry's ledger: PASS when every ledger named
/// exactly the oracle's flakes and its quarantine round-trip held, FAIL otherwise (a ledger
/// `spirolo` could not run is FAIL, as it is on the bench). The named flakes the oracle has are
/// truths, the ones it does not are falses. `None` when no entry records a ledger.
fn ledger_row(reports: &[PrecisionReport]) -> Option<(&'static str, Counts)> {
    let ledgers: Vec<_> = reports
        .iter()
        .filter_map(|report| report.ledger.as_ref())
        .collect();
    if ledgers.is_empty() {
        return None;
    }
    let mut pass = true;
    let mut counts: Counts = (0, 0);
    for ledger in ledgers {
        match ledger {
            Ok(score) => {
                pass &= score.exact() && score.quarantine_held();
                let extra = score.extra.len() as u64;
                counts.0 += (score.reported as u64).saturating_sub(extra);
                counts.1 += extra;
            }
            Err(_) => pass = false,
        }
    }
    Some((if pass { "PASS" } else { "FAIL" }, counts))
}

/// Rules measured by a row outside [`RuleFloor`]'s table whose floor a milestone carried:
/// M4b closed with the hotspots floor carried to M6 on a fresh file set (docs/roadmap.md
/// §M4b, #235).
const CARRIED_ROWS: &[(&str, &str)] = &[(HOTSPOT_RULE, "M6")];

/// The rule the M4b hotspots row measures.
const HOTSPOT_RULE: &str = "untested-hotspot";

/// `untested-hotspot`'s row pooled over every entry's M4b hotspots row: FAIL when any entry's
/// row failed or could not be measured, else INSUFFICIENT when any ranked too few files, else
/// PASS. The top files below the set's median are truths, the others falses. `None` when no
/// entry records an impact oracle.
fn hotspot_row(reports: &[PrecisionReport]) -> Option<(&'static str, Counts)> {
    let rows: Vec<_> = reports
        .iter()
        .filter_map(|report| report.impact.as_ref())
        .collect();
    if rows.is_empty() {
        return None;
    }
    let mut verdicts = Vec::new();
    let mut counts: Counts = (0, 0);
    for row in rows {
        match row {
            Ok(rows) => {
                verdicts.push(rows.hotspots.verdict());
                let (below, top) = rows.hotspots.below();
                counts.0 += below;
                counts.1 += top - below;
            }
            Err(_) => verdicts.push(RuleVerdict::Fail),
        }
    }
    let verdict = [RuleVerdict::Fail, RuleVerdict::Insufficient]
        .into_iter()
        .find(|worst| verdicts.contains(worst))
        .unwrap_or(RuleVerdict::Pass);
    Some((verdict.tag(), counts))
}

/// One rule's object; `precision` and `interval` are null with no decided finding.
fn status_entry(verdict: &str, carried: Option<&str>, truths: u64, falses: u64) -> Value {
    let total = truths + falses;
    let (precision, interval) = if total == 0 {
        (Value::Null, Value::Null)
    } else {
        #[allow(clippy::cast_precision_loss)]
        let precision = truths as f64 / total as f64;
        let interval = wilson_interval(truths, total);
        (
            json!(round3(precision)),
            json!([round3(interval.low), round3(interval.high)]),
        )
    };
    json!({
        "verdict": verdict,
        "carried": carried,
        "precision": precision,
        "interval": interval,
        "truths": truths,
        "falses": falses,
    })
}

/// Rounded to three decimals, the precision every surface prints.
fn round3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

/// Every rule `spirolo_rules::registry()` registers, in registry order.
///
/// The bench does not link the rules crate; the embedded table's own unit test (spirolo-rules, #239) fails when a
/// registered rule is missing from the file this list generates.
pub const REGISTERED_RULES: &[&str] = &[
    "weak-assertion",
    "absence-anchor",
    "data-layer-mock",
    "handler-drift",
    "generator-walk",
    "fail-closed-setup",
    "gate-truth",
    "dead-skip",
    "duplicate-test",
    "redundant-test",
    "unused-mock",
    "orphan-test",
    "stale-snapshot",
    "flake-candidate",
    "flake-observed",
    "untested-hotspot",
];
