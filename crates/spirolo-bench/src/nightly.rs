//! M6 nightly static measurements, read independently of the executing runner.

use std::fmt::Write as _;

use serde_json::{json, Value};

use crate::{by_framework, by_runner_record, evaluate, Truth, Wallclock};

/// Compare a real list document with both regression counts and the runner's listing.
#[must_use]
pub fn nightly_counts(list: &Value, truth: &Truth) -> Value {
    let actual = by_framework(list, &truth.excluded_paths(), &truth.excluded_file_paths());
    let runner = by_runner_record(list, &truth.runner);
    let report = evaluate(
        "nightly",
        truth,
        &actual,
        &runner,
        Wallclock {
            samples: vec![std::time::Duration::ZERO],
            busy: None,
        },
    );
    let actual: serde_json::Map<String, Value> = actual
        .into_iter()
        .map(|(name, count)| (name, json!({"files": count.files, "tests": count.tests})))
        .collect();
    let mismatches: Vec<Value> = report
        .rows
        .iter()
        .filter(|r| !r.matched)
        .map(|r| {
            json!({"framework": r.framework, "expected_files": r.expected_files,
            "expected_tests": r.expected_tests, "actual_files": r.actual_files,
            "actual_tests": r.actual_tests})
        })
        .collect();
    let runner_mismatches: Vec<Value> = report
        .runner
        .iter()
        .filter(|r| !r.matched)
        .map(|r| {
            json!({"framework": r.framework, "expected_files": r.listed_files,
            "expected_tests": r.listed_tests, "actual_files": r.actual_files,
            "actual_tests": r.actual_tests})
        })
        .collect();
    json!({"actual": actual, "count_drift": !mismatches.is_empty(),
        "runner_drift": !runner_mismatches.is_empty(), "mismatches": mismatches,
        "runner_mismatches": runner_mismatches})
}

/// The seven-night row, with every failing entry named.
#[derive(Debug, Default)]
pub struct NightlyScore {
    /// Failures that prevent this row passing.
    pub failures: Vec<String>,
    /// Complete nights among the seven required calendar dates.
    pub nights: usize,
    /// Entry rows in the selected nightly runs.
    pub rows: usize,
}
impl NightlyScore {
    /// Whether any measured run or coverage requirement failed.
    #[must_use]
    pub const fn failed(&self) -> bool {
        !self.failures.is_empty()
    }
}

/// Score canned or measured JSONL rows; runner discrepancies remain reported separately.
#[must_use]
pub fn score_nightly(ledger: &str, expected: &[String], today: &str) -> NightlyScore {
    let mut score = NightlyScore::default();
    let Some(today) = calendar_day(today) else {
        score.failures.push("invalid current UTC date".to_owned());
        return score;
    };
    let mut nights: std::collections::BTreeMap<
        i64,
        std::collections::BTreeMap<String, Vec<Value>>,
    > = std::collections::BTreeMap::new();
    for (line_no, line) in ledger.lines().enumerate() {
        let Ok(row) = serde_json::from_str::<Value>(line) else {
            score
                .failures
                .push(format!("invalid ledger JSON at line {}", line_no + 1));
            continue;
        };
        let Some(day) = row["night"].as_str().and_then(calendar_day) else {
            score
                .failures
                .push(format!("invalid night at line {}", line_no + 1));
            continue;
        };
        if (today - 6..=today).contains(&day) {
            let run = row["run_id"].as_str().unwrap_or_default().to_owned();
            nights
                .entry(day)
                .or_default()
                .entry(run)
                .or_default()
                .push(row);
        }
    }
    for day in today - 6..=today {
        let Some((_, rows)) = nights.get(&day).and_then(|runs| runs.last_key_value()) else {
            score
                .failures
                .push(format!("missing night (calendar day {day})"));
            continue;
        };
        score.rows += rows.len();
        let names: std::collections::BTreeSet<&str> =
            rows.iter().filter_map(|r| r["entry"].as_str()).collect();
        let expected: std::collections::BTreeSet<&str> =
            expected.iter().map(String::as_str).collect();
        if names == expected && names.len() == rows.len() && !names.is_empty() {
            score.nights += 1;
        } else {
            score.failures.push(format!(
                "{}: incomplete or duplicate entry coverage",
                rows[0]["night"]
            ));
        }
        for row in rows {
            score_row(row, &mut score);
        }
    }
    score
}

fn score_row(row: &Value, score: &mut NightlyScore) {
    let valid = row["runs"].as_array().is_some_and(|runs| {
        runs.len() == 4
            && runs.iter().enumerate().all(|(at, r)| {
                let command = if at < 2 { "list" } else { "check" };
                r["command"] == command
                    && (r["exit"] == 0 || (command == "check" && r["exit"] == 1))
                    && r["signal"].is_null()
                    && r["error"].is_null()
                    && r["timeout"] == false
                    && r["panic"] == false
                    && r["wall_ms"]
                        .as_f64()
                        .is_some_and(|n| n.is_finite() && n >= 0.0)
                    && r["bytes"].as_u64().is_some_and(|n| n > 0)
                    && r["sha256"].as_str().is_some_and(|hash| {
                        hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit())
                    })
                    && r["load"].as_array().is_some_and(|load| {
                        load.len() == 3
                            && load
                                .iter()
                                .all(|n| n.as_f64().is_some_and(|n| n.is_finite() && n >= 0.0))
                    })
            })
    });
    if !valid
        || row["runs"][0]["sha256"] != row["runs"][1]["sha256"]
        || row["runs"][2]["sha256"] != row["runs"][3]["sha256"]
    {
        score.failures.push(format!(
            "{} {}: invalid or failed execution",
            row["night"], row["entry"]
        ));
    }
    if row["count_drift"] != false {
        score
            .failures
            .push(format!("{} {}: count drift", row["night"], row["entry"]));
    }
    for field in ["list_equal", "check_equal"] {
        if row[field] != true {
            score
                .failures
                .push(format!("{} {}: {field}", row["night"], row["entry"]));
        }
    }
    if row["runs"]
        .as_array()
        .is_some_and(|runs| runs.iter().any(|r| r["timeout"] == true))
    {
        score
            .failures
            .push(format!("{} {}: timeout", row["night"], row["entry"]));
    }
    if row["runs"]
        .as_array()
        .is_some_and(|runs| runs.iter().any(|r| r["panic"] == true))
    {
        score
            .failures
            .push(format!("{} {}: panic", row["night"], row["entry"]));
    }
}

fn calendar_day(date: &str) -> Option<i64> {
    let mut parts = date.split('-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: usize = parts.next()?.parse().ok()?;
    let day: i64 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=9999).contains(&year) || !(1..=12).contains(&month) {
        return None;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let months = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if !(1..=months[month - 1]).contains(&day) {
        return None;
    }
    let prior = year - 1;
    Some(
        prior * 365 + prior / 4 - prior / 100
            + prior / 400
            + months[..month - 1].iter().sum::<i64>()
            + day,
    )
}

/// Manifest names eligible for the nightly pass, without inspecting truth or clones.
#[must_use]
pub fn nightly_entries(manifest: &str) -> Vec<String> {
    let mut entries = Vec::new();
    let mut name = None;
    let mut held_out = false;
    let mut notes = false;
    for line in manifest.lines().chain(std::iter::once("[[entry]]")) {
        if line.contains("\"\"\"") {
            notes = !notes;
            continue;
        }
        if notes {
            continue;
        }
        if line.trim() == "[[entry]]" {
            if !held_out {
                if let Some(name) = name.take() {
                    entries.push(name);
                }
            }
            name = None;
            held_out = false;
        }
        if let Some((key, value)) = line.split_once('=') {
            let value = value.trim().trim_matches('"');
            match key.trim() {
                "name" => name = Some(value.to_owned()),
                "set" => held_out = value == "held-out",
                _ => {}
            }
        }
    }
    entries.sort();
    entries
}

/// Compact bench row; detailed evidence stays in the ledger and its artifacts.
#[must_use]
pub fn render_nightly(score: &NightlyScore, entries: usize) -> String {
    let verdict = if score.failed() { "FAIL" } else { "PASS" };
    let mut text = format!(
        "M6 nightly  {} of 7 complete nights · {} of {} entry rows  {verdict}\n",
        score.nights,
        score.rows,
        entries * 7
    );
    for failure in score.failures.iter().take(10) {
        let _ = writeln!(text, "  {failure}");
    }
    if score.failures.len() > 10 {
        let _ = writeln!(text, "  … {} more failures", score.failures.len() - 10);
    }
    text
}
