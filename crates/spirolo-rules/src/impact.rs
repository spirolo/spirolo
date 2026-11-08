//! `impact`: the test files a change reaches, ranked by a fixed order of evidence
//! (PRD 0005, "Ranking order, fixed a priori"). Not a finding rule: it has its own output
//! type, and the binary calls it with the changed paths it was handed.
//!
//! Blind to mutation data by construction: it reads the `Module`, `TestCase`, `Pairing` and
//! `Run` tables and nothing else. With no run ingested, keys 3 and 5 are inert.

use std::collections::{BTreeMap, HashMap, HashSet};

use spirolo_model::{Diagnostic, Model, RunOutcome, Severity};

/// What `impact` says about one set of changed files.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Impact {
    /// The changed paths the model knows, repo-relative and normalized, sorted.
    pub changed: Vec<String>,
    /// The selected test files, in rank order.
    pub selected: Vec<Selected>,
    /// Changed files no test file reaches, by import or pairing, sorted.
    pub unreached: Vec<String>,
    /// Changed paths that were outside the root or unknown to the model.
    pub diagnostics: Vec<Diagnostic>,
}

/// One selected test file and the evidence it was ranked on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selected {
    /// 1-based position in the ranking.
    pub rank: usize,
    /// Repo-relative path of the test file.
    pub file: String,
    /// Import hops to the nearest changed file: 0 for a changed test file, 1 for a direct
    /// import; `None` when the file is selected only by its pairing.
    pub distance: Option<u32>,
    /// True when the file is paired with a changed file.
    pub paired: bool,
    /// True when the file had a failed row in its most recent ingested run (the latest run
    /// naming it); false with no runs ingested.
    pub last_failed: bool,
    /// Median duration of the file over the ingested runs, in milliseconds; `None` when no
    /// run recorded one.
    pub median_ms: Option<u64>,
}

/// Select and rank the test files `changed` reaches in `model`.
pub fn impact(model: &Model, changed: &[String]) -> Impact {
    let known: HashSet<&str> = model.modules.iter().map(|m| m.path.as_str()).collect();
    let root = model.root.to_string_lossy();
    let mut diagnostics = Vec::new();
    let mut changed: Vec<String> = changed
        .iter()
        .filter_map(|raw| match normalize(&root, raw) {
            None => {
                diagnostics.push(diagnostic(
                    "impact_path_outside_root",
                    None,
                    format!("changed path {raw:?} is outside the project root"),
                ));
                None
            }
            Some(path) if !known.contains(path.as_str()) => {
                diagnostics.push(diagnostic(
                    "impact_path_unknown",
                    Some(path),
                    format!("changed path {raw:?} is not a module the model read"),
                ));
                None
            }
            Some(path) => Some(path),
        })
        .collect();
    diagnostics.sort();
    changed.sort();
    changed.dedup();
    let refs: Vec<&str> = changed.iter().map(String::as_str).collect();
    let importers = model.importers();
    let mut candidates: BTreeMap<String, (Option<u32>, bool)> = importers
        .test_distances(&refs)
        .into_iter()
        .map(|(file, distance)| (file, (Some(distance), false)))
        .collect();
    for pairing in &model.pairings {
        if refs.contains(&pairing.source_module.as_str()) && importers.is_test(&pairing.test_file) {
            candidates.entry(pairing.test_file.clone()).or_default().1 = true;
        }
    }

    let failed = last_failed(model);
    let medians = median_durations(model);
    let mut selected: Vec<Selected> = candidates
        .into_iter()
        .map(|(file, (distance, paired))| Selected {
            rank: 0,
            last_failed: failed.contains(file.as_str()),
            median_ms: medians.get(file.as_str()).copied(),
            file,
            distance,
            paired,
        })
        .collect();
    // PRD 0005's fixed order.
    selected.sort_by(|a, b| {
        let changed_test = |s: &Selected| s.distance != Some(0);
        changed_test(a)
            .cmp(&changed_test(b))
            .then(b.paired.cmp(&a.paired))
            .then(b.last_failed.cmp(&a.last_failed))
            .then(
                a.distance
                    .unwrap_or(u32::MAX)
                    .cmp(&b.distance.unwrap_or(u32::MAX)),
            )
            .then(
                a.median_ms
                    .unwrap_or(u64::MAX)
                    .cmp(&b.median_ms.unwrap_or(u64::MAX)),
            )
            .then(a.file.cmp(&b.file))
    });
    for (index, selected) in selected.iter_mut().enumerate() {
        selected.rank = index + 1;
    }
    let unreached = changed
        .iter()
        .filter(|path| {
            importers.test_distances(&[path.as_str()]).is_empty()
                && !model.pairings.iter().any(|pairing| {
                    pairing.source_module == **path && importers.is_test(&pairing.test_file)
                })
        })
        .cloned()
        .collect();
    Impact {
        changed,
        selected,
        unreached,
        diagnostics,
    }
}

/// The test files with a failed row in their most recent ingested run: for each file, the
/// run with the latest `started_at_ms` (the greater run id on a tie) among the runs that have
/// a row for it. Read per file because a monorepo runner writes one report, so one ingested
/// run, per package: the single latest run would name one package's files only. Empty when
/// no run is ingested.
fn last_failed(model: &Model) -> HashSet<&str> {
    let mut latest: HashMap<&str, (Option<u64>, &str)> = HashMap::new();
    for run in &model.runs {
        let key = (run.started_at_ms, run.run_id.as_str());
        let entry = latest.entry(run.file.as_str()).or_insert(key);
        *entry = (*entry).max(key);
    }
    model
        .runs
        .iter()
        .filter(|run| {
            run.outcome == RunOutcome::Failed
                && latest.get(run.file.as_str()) == Some(&(run.started_at_ms, run.run_id.as_str()))
        })
        .map(|run| run.file.as_str())
        .collect()
}

/// Each test file's median test duration over every ingested run, in milliseconds: the
/// median of the durations its rows recorded, skipped rows left out (they did not run).
/// The mean of the two middle values, rounded down, for an even count. A file with no
/// recorded duration has no entry.
fn median_durations(model: &Model) -> HashMap<&str, u64> {
    let mut durations: HashMap<&str, Vec<u64>> = HashMap::new();
    for run in &model.runs {
        if let (Some(duration), false) = (run.duration_ms, run.outcome == RunOutcome::Skipped) {
            durations
                .entry(run.file.as_str())
                .or_default()
                .push(duration);
        }
    }
    durations
        .into_iter()
        .map(|(file, mut values)| {
            values.sort_unstable();
            let middle = values.len() / 2;
            let median = if values.len() % 2 == 1 {
                values[middle]
            } else {
                values[middle - 1].midpoint(values[middle])
            };
            (file, median)
        })
        .collect()
}

/// `raw` as a repo-relative, slash-separated path with `.` and `..` folded away, or `None`
/// when it leaves the root. An absolute path must start with `root`; a relative one is
/// read from the root, as `git diff --name-only` prints it.
fn normalize(root: &str, raw: &str) -> Option<String> {
    let relative = if raw.starts_with('/') {
        raw.strip_prefix(root)?.strip_prefix('/')?
    } else {
        raw
    };
    let mut parts: Vec<&str> = Vec::new();
    for part in relative.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    Some(parts.join("/"))
}

/// A warning about one changed path `impact` could not use.
fn diagnostic(code: &str, file: Option<String>, message: String) -> Diagnostic {
    Diagnostic {
        code: code.to_owned(),
        severity: Severity::Warning,
        file,
        span: None,
        message,
    }
}
