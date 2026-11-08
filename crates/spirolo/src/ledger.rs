//! The run ledger on disk: `spirolo flake ingest` reads run reports and writes one normalized
//! run file per run under `.spirolo/runs/`, and `check` reads those files back into the
//! model's `Run` table. The formats live in `spirolo-inputs`; this module finds the files and
//! is the only place that writes them.
//!
//! Every report is one run. Its tree hash comes from `--tree-hash` or from a `run.json`
//! sidecar in the report's directory, and ingest refuses before writing anything when a
//! report has neither: the binary never asks git. The run id defaults to a content hash of
//! the report's bytes, so ingesting the same report again rewrites identical bytes.

use std::collections::BTreeMap;
use std::path::Path;

use spirolo_inputs::{
    content_run_id, read_junit_xml, read_run_file, read_sidecar, read_vitest_json, Added,
    QuarantineEntry, QuarantineFile, RunContext, RunFile, RunTables, TreeEvidence,
};
use spirolo_model::{relative_slash, Diagnostic, Run, TestCase};
use spirolo_rules::Finding;

use crate::inputs;

/// Where the normalized run files live, relative to the project root.
pub const RUNS_DIR: &str = ".spirolo/runs";

/// The quarantine file, relative to the project root.
pub const QUARANTINE_FILE: &str = ".spirolo/quarantine.json";

/// The sidecar a run's reports may sit beside.
const SIDECAR: &str = "run.json";

/// What `spirolo flake ingest` was asked to do.
pub struct Ingest<'a> {
    /// Globs naming the reports, relative to the root.
    pub globs: &'a [String],
    /// The tree hash every report was made on, overriding any sidecar.
    pub tree_hash: Option<&'a str>,
    /// The run id, allowed only when the globs name one report.
    pub run_id: Option<&'a str>,
    /// The condition every report was made under, overriding any sidecar.
    pub condition: Option<&'a str>,
    /// Stable runner/configuration identity, overriding any sidecar.
    pub runner_home: Option<&'a str>,
}

/// Ingest the reports `request` names under `root` into `.spirolo/runs/`, returning the
/// in-tree paths of the run files written.
///
/// # Errors
///
/// A message naming what is missing when no report matches, a report has no tree hash, a
/// sidecar does not parse, `--run-id` is given for more than one report, or a file cannot
/// be read or written. Nothing is written when a report is refused.
pub fn ingest(root: &Path, request: &Ingest<'_>) -> Result<(Vec<String>, Vec<Diagnostic>), String> {
    let reports = inputs::find_reports(root, request.globs);
    if reports.is_empty() {
        return Err(format!("no report matches {}", request.globs.join(" ")));
    }
    if request.run_id.is_some() && reports.len() > 1 {
        return Err(format!(
            "--run-id names one run, but {} reports match; ingest them one at a time",
            reports.len()
        ));
    }
    let mut runs = Vec::new();
    for report in &reports {
        let bytes = std::fs::read(root.join(report))
            .map_err(|error| format!("cannot read {report}: {error}"))?;
        let context = context(root, report, &bytes, request)?;
        runs.push((context, bytes));
    }

    let dir = root.join(RUNS_DIR);
    std::fs::create_dir_all(&dir).map_err(|error| format!("cannot create {RUNS_DIR}: {error}"))?;
    let mut written = Vec::new();
    let mut diagnostics = Vec::new();
    for (context, bytes) in runs {
        let tables = read_report(&context, &bytes);
        diagnostics.extend(tables.diagnostics);
        let file = RunFile::new(&context, &tables.runs);
        let name = format!("{RUNS_DIR}/{}.json", context.run_id);
        std::fs::write(root.join(&name), file.to_json())
            .map_err(|error| format!("cannot write {name}: {error}"))?;
        written.push(name);
    }
    Ok((written, diagnostics))
}

/// The run context of `report`: its tree hash and condition from the flags or its sidecar,
/// its run id from the flag or its bytes.
fn context(
    root: &Path,
    report: &str,
    bytes: &[u8],
    request: &Ingest<'_>,
) -> Result<RunContext, String> {
    let sidecar_path = Path::new(report)
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join(SIDECAR);
    let sidecar = match std::fs::read(root.join(&sidecar_path)) {
        Ok(bytes) => read_sidecar(&bytes).map_err(|error| {
            format!(
                "{} is not a readable sidecar: {error}",
                sidecar_path.display()
            )
        })?,
        Err(_) => spirolo_inputs::Sidecar::default(),
    };
    let tree_hash = request
        .tree_hash
        .map(str::to_owned)
        .or(sidecar.tree_hash)
        .ok_or_else(|| {
            format!(
                "{report} has no tree hash: pass --tree-hash, or put a {SIDECAR} with \
                 \"tree_hash\" beside it; spirolo does not ask git"
            )
        })?;
    let run_id = request
        .run_id
        .map_or_else(|| content_run_id(bytes), str::to_owned);
    let runner_home = request
        .runner_home
        .map(str::to_owned)
        .or(sidecar.runner_home)
        .or_else(|| {
            // A sidecar that records runner homes describes independent report streams.
            // Their stable filenames identify those streams across run directories.
            (!sidecar.homes.is_empty())
                .then(|| Path::new(report).file_name()?.to_str().map(str::to_owned))
                .flatten()
        });
    Ok(RunContext {
        runner_home,
        report: report.to_owned(),
        root: root.to_string_lossy().into_owned(),
        run_id,
        attempt: 0,
        condition: request.condition.map(str::to_owned).or(sidecar.condition),
        tree_hash: Some(tree_hash),
    })
}

/// Read one report with the reader its extension names: `.xml` is `JUnit`, anything else
/// vitest JSON.
fn read_report(context: &RunContext, bytes: &[u8]) -> RunTables {
    let xml = Path::new(&context.report)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("xml"));
    if xml {
        read_junit_xml(context, bytes, &[])
    } else {
        read_vitest_json(context, bytes, &[])
    }
}

/// Read every run file under `.spirolo/runs/` into `Run` rows joined to `test_cases`, sorted.
/// No directory is no runs.
pub fn read_runs(root: &Path, test_cases: &[TestCase]) -> (Vec<Run>, Vec<Diagnostic>) {
    let mut runs = Vec::new();
    let mut diagnostics = Vec::new();
    let Ok(entries) = std::fs::read_dir(root.join(RUNS_DIR)) else {
        return (runs, diagnostics);
    };
    let mut paths: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    paths.sort();
    for path in paths {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Some(name) = relative_slash(root, &path) else {
            continue;
        };
        let tables = read_run_file(&name, &bytes, test_cases);
        runs.extend(tables.runs);
        diagnostics.extend(tables.diagnostics);
    }
    runs.sort();
    (runs, diagnostics)
}

/// Merge the flakes `findings` name (`flake-observed`'s, over `runs`) into the quarantine
/// file under `root`, creating it when there is none.
///
/// # Errors
///
/// A message when the existing file cannot be read or does not parse, in which case it is
/// left as it was, or when the new file cannot be written.
pub fn quarantine(root: &Path, findings: &[Finding], runs: &[Run]) -> Result<(), String> {
    let path = root.join(QUARANTINE_FILE);
    let mut file = match std::fs::read(&path) {
        Ok(bytes) => QuarantineFile::parse(&bytes).map_err(|error| {
            format!("{QUARANTINE_FILE} is not a readable quarantine file, so it is left as it is: {error}")
        })?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => QuarantineFile::default(),
        Err(error) => return Err(format!("cannot read {QUARANTINE_FILE}: {error}")),
    };
    file.merge(&ledger_flakes(findings, runs));
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|error| format!("cannot create .spirolo: {error}"))?;
    }
    std::fs::write(&path, file.to_json())
        .map_err(|error| format!("cannot write {QUARANTINE_FILE}: {error}"))
}

/// One ledger entry per test the findings name, with a count pair per tree it flipped on.
/// A test first seen on several trees at once is first seen on the tree whose earliest run
/// started first, then by tree hash.
fn ledger_flakes(findings: &[Finding], runs: &[Run]) -> Vec<QuarantineEntry> {
    let mut tests: BTreeMap<(String, Vec<String>), Vec<TreeEvidence>> = BTreeMap::new();
    for finding in findings {
        let Some(test) = &finding.test else { continue };
        let mut title_path = test.suite_path.clone();
        title_path.extend(test.title.clone());
        let value = |key: &str| {
            finding
                .evidence
                .iter()
                .find(|evidence| evidence.key == key)
                .map_or("", |evidence| evidence.value.as_str())
        };
        tests
            .entry((finding.location.file.clone(), title_path))
            .or_default()
            .push(TreeEvidence {
                tree_hash: value("tree_hash").to_owned(),
                passed: value("passed").parse().unwrap_or_default(),
                failed: value("failed").parse().unwrap_or_default(),
            });
    }
    tests
        .into_iter()
        .map(|((file, test), mut evidence)| {
            evidence.sort_by(|a, b| a.tree_hash.cmp(&b.tree_hash));
            let first_seen = evidence
                .iter()
                .min_by_key(|tree| {
                    let started = runs
                        .iter()
                        .filter(|run| {
                            run.file == file
                                && run.title_path == test
                                && run.tree_hash.as_deref() == Some(tree.tree_hash.as_str())
                        })
                        .filter_map(|run| run.started_at_ms)
                        .min();
                    (started.is_none(), started, tree.tree_hash.clone())
                })
                .map(|tree| tree.tree_hash.clone());
            QuarantineEntry {
                file,
                test,
                tree_hash_first_seen: first_seen,
                added: Added::Ledger,
                reason: None,
                evidence,
                extra: BTreeMap::new(),
            }
        })
        .collect()
}
