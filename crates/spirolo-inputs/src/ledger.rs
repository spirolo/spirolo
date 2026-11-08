//! The run ledger's file formats: the normalized run file `spirolo flake ingest` writes under
//! `.spirolo/runs/`, one per run, and the `run.json` sidecar a run's reports may sit beside.
//!
//! A run file is sorted, pretty-printed JSON a human commits: the run's id, tree hash,
//! condition, start and source report, then one entry per test the report named. It keeps
//! no join to the model — that is made again against the tree `check` reads, so a run file
//! never goes stale when a test moves.

use serde::{Deserialize, Serialize};
use spirolo_model::{Diagnostic, Run, RunOutcome, Severity, TestCase};

use crate::runs::{join_case, RunContext, RunTables};

/// The run file format's version, written into every file.
const VERSION: u32 = 1;

/// One normalized run file: a run's context and its tests, in the `Run` table's order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunFile {
    /// The format's version.
    pub version: u32,
    /// The run's id, which names the file.
    pub run_id: String,
    /// Hash of the tree the run was made on.
    pub tree_hash: String,
    /// The label the run was ingested under, for example `idle` or `load`.
    pub condition: Option<String>,
    /// When the run started, in milliseconds since the Unix epoch, as the report records it.
    pub started_at_ms: Option<u64>,
    /// In-tree path of the report the run was read from.
    pub report: String,
    /// Stable runner/configuration identity, absent in older ledgers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runner_home: Option<String>,
    /// One entry per test the report named, sorted by file, title path and attempt.
    pub tests: Vec<RunFileTest>,
}

/// One test's outcome in a run file.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RunFileTest {
    /// Repo-relative, slash-separated path of the test file.
    pub file: String,
    /// The enclosing `describe` titles, then the test's own title.
    pub title_path: Vec<String>,
    /// Which attempt of the run this is, from 0.
    pub attempt: u32,
    /// How it ended.
    pub outcome: RunOutcome,
    /// Wall-clock duration in milliseconds, when the reporter recorded one.
    pub duration_ms: Option<u64>,
    /// Retries the runner performed before settling.
    pub retries: Option<u32>,
}

impl RunFile {
    /// The run file for the rows one report yielded under `context`, which must carry a
    /// tree hash (ingest refuses a run without one before it reads the report).
    #[must_use]
    pub fn new(context: &RunContext, runs: &[Run]) -> Self {
        let mut tests: Vec<RunFileTest> = runs
            .iter()
            .map(|run| RunFileTest {
                file: run.file.clone(),
                title_path: run.title_path.clone(),
                attempt: run.attempt,
                outcome: run.outcome,
                duration_ms: run.duration_ms,
                retries: run.retries,
            })
            .collect();
        tests.sort();
        Self {
            version: VERSION,
            run_id: context.run_id.clone(),
            tree_hash: context.tree_hash.clone().unwrap_or_default(),
            condition: context.condition.clone(),
            started_at_ms: runs.iter().find_map(|run| run.started_at_ms),
            report: context.report.clone(),
            runner_home: context.runner_home.clone(),
            tests,
        }
    }

    /// The file's bytes: pretty-printed JSON with a trailing newline.
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).unwrap_or_default();
        text.push('\n');
        text
    }
}

/// Read one run file (at in-tree `path`) back into `Run` rows, each joined to the
/// `TestCase` of `test_cases` with its file and title path, or kept unjoined.
///
/// A file that does not parse yields one `run_file_unreadable` diagnostic and no rows.
#[must_use]
pub fn read_run_file(path: &str, bytes: &[u8], test_cases: &[TestCase]) -> RunTables {
    let file: RunFile = match serde_json::from_slice(bytes) {
        Ok(file) => file,
        Err(error) => {
            return RunTables {
                runs: Vec::new(),
                diagnostics: vec![Diagnostic {
                    code: "run_file_unreadable".to_owned(),
                    severity: Severity::Error,
                    file: Some(path.to_owned()),
                    span: None,
                    message: format!("{path} is not a readable run file: {error}"),
                }],
            }
        }
    };
    let mut runs: Vec<Run> = file
        .tests
        .into_iter()
        .map(|test| Run {
            title: test.title_path.join(" > "),
            run_id: file.run_id.clone(),
            attempt: test.attempt,
            outcome: test.outcome,
            duration_ms: test.duration_ms,
            retries: test.retries,
            tree_hash: Some(file.tree_hash.clone()),
            started_at_ms: file.started_at_ms,
            condition: file.condition.clone(),
            report: file.report.clone(),
            runner_home: file.runner_home.clone(),
            joined: join_case(&test.file, &test.title_path, test_cases),
            title_path: test.title_path,
            file: test.file,
        })
        .collect();
    runs.sort();
    RunTables {
        runs,
        diagnostics: Vec::new(),
    }
}

/// What a `run.json` sidecar beside a run's reports says about the run.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct Sidecar {
    /// Hash of the tree the run was made on.
    #[serde(default)]
    pub tree_hash: Option<String>,
    /// The label the run was made under, for example `idle` or `load`.
    #[serde(default)]
    pub condition: Option<String>,
    /// Explicit runner/configuration identity for this report.
    #[serde(default)]
    pub runner_home: Option<String>,
    /// Runner homes recorded by a multi-report run. Report filenames identify their
    /// separate streams across run directories when no explicit identity is supplied.
    #[serde(default)]
    pub homes: Vec<SidecarHome>,
}

/// One runner home in a run sidecar. Other execution metadata is ignored.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SidecarHome {
    /// Project-relative runner home.
    pub home: String,
    /// Runner name, such as `vitest` or `jest`.
    pub runner: String,
}

/// Read a `run.json` sidecar. Unrecognized keys are ignored.
///
/// # Errors
///
/// The parser's message when the bytes are not a JSON object of that shape.
pub fn read_sidecar(bytes: &[u8]) -> Result<Sidecar, String> {
    serde_json::from_slice(bytes).map_err(|error| error.to_string())
}

/// The default run id: a content hash of the report's bytes, so ingesting the same report
/// twice names the same run.
#[must_use]
pub fn content_run_id(bytes: &[u8]) -> String {
    format!("{:032x}", xxhash_rust::xxh3::xxh3_128(bytes))
}
