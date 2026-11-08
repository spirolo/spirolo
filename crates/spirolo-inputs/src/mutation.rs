//! The mutation report reader: a Stryker `mutation.json` (the mutation-testing-elements
//! schema) in, the model's three mutation row kinds and diagnostics out.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use spirolo_model::{
    Bail, Diagnostic, JoinedCase, MutantStatus, Mutants, MutationReport, MutationTest, Severity,
    Span, TestCase, TestKind,
};

/// The rows and diagnostics one mutation report yields. The binary merges the tables of
/// several reports and sorts them; a single call fills one [`MutationReport`] row.
#[derive(Debug, Default, Serialize)]
pub struct MutationTables {
    /// One row per mutant the report describes.
    pub mutants: Vec<Mutants>,
    /// One row per test the report names.
    pub tests: Vec<MutationTest>,
    /// The report's own provenance row, unless the bytes did not parse.
    pub reports: Vec<MutationReport>,
    /// What the reader could not decide or wants the reader to know.
    pub diagnostics: Vec<Diagnostic>,
}

/// Read one mutation report from its bytes into the model's rows.
///
/// Every row is tagged with `path` (the report's in-tree, slash-separated location).
/// `test_cases` are the model's tests, used to join each named test to the row it
/// describes; pass an empty slice when no join is wanted. The bytes are never trusted to be
/// well-formed: a report that does not parse yields a single `mutation_report_unreadable`
/// diagnostic and no rows.
#[must_use]
pub fn read_mutation_report(path: &str, bytes: &[u8], test_cases: &[TestCase]) -> MutationTables {
    let root: Root = match serde_json::from_slice(bytes) {
        Ok(root) => root,
        Err(error) => {
            return MutationTables {
                diagnostics: vec![Diagnostic {
                    code: "mutation_report_unreadable".to_owned(),
                    severity: Severity::Error,
                    file: Some(path.to_owned()),
                    span: None,
                    message: format!("{path} is not a readable mutation report: {error}"),
                }],
                ..MutationTables::default()
            };
        }
    };

    let package = package_dir(path, root.config.as_ref());
    let in_tree = |key: &str| format!("{package}{key}");

    let mut mutants = Vec::new();
    for (file, result) in &root.files {
        let source = result.source.as_deref().unwrap_or("");
        for mutant in &result.mutants {
            mutants.push(Mutants {
                file: in_tree(file),
                span: mutant.location.to_span(source),
                id: mutant.id.clone(),
                mutator: mutant.mutator_name.clone(),
                status: status_of(&mutant.status),
                killed_by: sorted(&mutant.killed_by),
                covered_by: sorted(&mutant.covered_by),
                report: path.to_owned(),
            });
        }
    }

    let mut tests = Vec::new();
    for (file, test_file) in &root.test_files {
        let file = in_tree(file);
        for test in &test_file.tests {
            tests.push(MutationTest {
                report: path.to_owned(),
                id: test.id.clone(),
                name: test.name.clone(),
                joined: join_case(&file, &test.name, test_cases),
                file: file.clone(),
            });
        }
    }

    let per_test_data = !tests.is_empty();
    let bail = bail_of(root.config.as_ref(), &mutants);

    let mut diagnostics = Vec::new();
    if !per_test_data {
        diagnostics.push(Diagnostic {
            code: "mutation_report_no_per_test".to_owned(),
            severity: Severity::Warning,
            file: Some(path.to_owned()),
            span: None,
            message: format!("{path} carries no per-test data; redundant-test cannot use it"),
        });
    }
    if matches!(bail, Bail::Declared | Bail::Inferred) {
        let how = if bail == Bail::Inferred {
            "appears to have been"
        } else {
            "was"
        };
        diagnostics.push(Diagnostic {
            code: "mutation_report_bailed".to_owned(),
            severity: Severity::Warning,
            file: Some(path.to_owned()),
            span: None,
            message: format!(
                "{path} {how} produced with bail; re-run with disableBail so every killing test is recorded"
            ),
        });
    }

    let report = MutationReport {
        path: path.to_owned(),
        schema_version: root.schema_version.clone(),
        tool: root.framework.as_ref().and_then(|f| f.name.clone()),
        tool_version: root.framework.as_ref().and_then(|f| f.version.clone()),
        per_test_data,
        bail,
    };

    MutationTables {
        mutants,
        tests,
        reports: vec![report],
        diagnostics,
    }
}

/// Where Stryker writes its JSON report, relative to the package it ran in, unless the
/// config's `jsonReporter.fileName` says otherwise.
const DEFAULT_REPORT_FILE: &str = "reports/mutation/mutation.json";

/// The in-tree directory of the package the report was produced in, with a trailing `/`
/// (empty at the tree root). Stryker keys `files` and `testFiles` relative to the directory
/// it ran in and writes the report at `jsonReporter.fileName` under it, so the package is
/// the report's path with that file name taken off its end.
fn package_dir(path: &str, config: Option<&serde_json::Value>) -> String {
    let file_name = config
        .and_then(|c| c.get("jsonReporter"))
        .and_then(|r| r.get("fileName"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or(DEFAULT_REPORT_FILE);
    let file_name = file_name.trim_start_matches("./");
    path.strip_suffix(file_name)
        .filter(|dir| dir.is_empty() || dir.ends_with('/'))
        .unwrap_or("")
        .to_owned()
}

/// Decide whether the report's kill sets are whole or truncated by bail.
///
/// The config's `disableBail` is believed when the report carries it. Absent it, bail is
/// inferred from the shape the truncation leaves: every killed mutant naming exactly one
/// killer while at least one mutant is covered by more than one test.
fn bail_of(config: Option<&serde_json::Value>, mutants: &[Mutants]) -> Bail {
    if let Some(disable_bail) = config
        .and_then(|c| c.get("disableBail"))
        .and_then(serde_json::Value::as_bool)
    {
        return if disable_bail {
            Bail::Disabled
        } else {
            Bail::Declared
        };
    }

    let killed: Vec<&Mutants> = mutants
        .iter()
        .filter(|m| m.status == MutantStatus::Killed)
        .collect();
    if killed.is_empty() {
        return Bail::Unknown;
    }
    let every_killed_has_one_killer = killed.iter().all(|m| m.killed_by.len() == 1);
    let some_covered_by_many = mutants.iter().any(|m| m.covered_by.len() > 1);
    if every_killed_has_one_killer && some_covered_by_many {
        Bail::Inferred
    } else {
        Bail::Unknown
    }
}

/// Join a report's test to the model's `TestCase` that names it: same file, and the
/// case's `describe` titles and own title, read as one sequence of whitespace-separated
/// words, equal to the report's `name` read the same way. Stryker's runners name a test by
/// its suite titles and title joined with single spaces. Only kind-test rows are
/// candidates; a name the model does not carry stays unjoined rather than guessed.
fn join_case(file: &str, name: &str, test_cases: &[TestCase]) -> Option<JoinedCase> {
    let words: Vec<&str> = name.split_whitespace().collect();
    test_cases
        .iter()
        .filter(|case| case.kind == TestKind::Test && case.file == file)
        .find(|case| full_name_words(case).is_some_and(|case_words| case_words == words))
        .map(|case| JoinedCase {
            file: case.file.clone(),
            span: case.span,
        })
}

/// A `TestCase`'s full name as words: its `describe` titles, then its own title, each split
/// on whitespace. `None` when the title is not a literal, so it never matches.
fn full_name_words(case: &TestCase) -> Option<Vec<&str>> {
    let title = case.title.as_ref()?;
    Some(
        case.suite_path
            .iter()
            .chain(std::iter::once(title))
            .flat_map(|part| part.split_whitespace())
            .collect(),
    )
}

/// Sort and deduplicate a list of test ids so a row is byte-stable across runs.
fn sorted(ids: &[String]) -> Vec<String> {
    let mut ids = ids.to_vec();
    ids.sort();
    ids.dedup();
    ids
}

/// Map a Stryker status string to the model's enum; an unknown status is treated as
/// `Pending` (recognised, nothing measured) rather than dropped.
fn status_of(status: &str) -> MutantStatus {
    match status {
        "Killed" => MutantStatus::Killed,
        "Survived" => MutantStatus::Survived,
        "NoCoverage" => MutantStatus::NoCoverage,
        "Timeout" => MutantStatus::Timeout,
        "RuntimeError" => MutantStatus::RuntimeError,
        "CompileError" => MutantStatus::CompileError,
        "Ignored" => MutantStatus::Ignored,
        _ => MutantStatus::Pending,
    }
}

// ------------------------------------------------------------- the schema

/// The top of a mutation-testing-elements report; only the fields the reader reads.
#[derive(Debug, Deserialize)]
struct Root {
    #[serde(rename = "schemaVersion")]
    schema_version: Option<String>,
    files: BTreeMap<String, FileResult>,
    #[serde(default, rename = "testFiles")]
    test_files: BTreeMap<String, TestFile>,
    framework: Option<Framework>,
    config: Option<serde_json::Value>,
}

/// The tool that wrote the report; Stryker fills this with its own name and version.
#[derive(Debug, Deserialize)]
struct Framework {
    name: Option<String>,
    version: Option<String>,
}

/// One test file's block, naming the tests the report's ids reference.
#[derive(Debug, Deserialize)]
struct TestFile {
    #[serde(default)]
    tests: Vec<TestDefinition>,
}

/// One test the report names.
#[derive(Debug, Deserialize)]
struct TestDefinition {
    id: String,
    name: String,
}

/// One mutated file's result block.
#[derive(Debug, Deserialize)]
struct FileResult {
    source: Option<String>,
    mutants: Vec<MutantResult>,
}

/// One mutant.
#[derive(Debug, Deserialize)]
struct MutantResult {
    id: String,
    #[serde(rename = "mutatorName")]
    mutator_name: Option<String>,
    location: Location,
    status: String,
    #[serde(default, rename = "killedBy")]
    killed_by: Vec<String>,
    #[serde(default, rename = "coveredBy")]
    covered_by: Vec<String>,
}

/// A mutant's source range, in the schema's one-based line/column pairs.
#[derive(Debug, Deserialize)]
struct Location {
    start: Position,
    end: Position,
}

impl Location {
    /// Resolve the range to a byte [`Span`] against the file's source, so mutants sort by
    /// position the way every other table does. A source the report omits leaves the byte
    /// offsets at zero but keeps the line and column.
    fn to_span(&self, source: &str) -> Span {
        let start = self.start.offset(source);
        let end = self.end.offset(source);
        Span {
            start,
            end,
            line: self.start.line,
            column: self.start.column,
        }
    }
}

/// A one-based line/column pair.
#[derive(Debug, Deserialize)]
struct Position {
    line: u32,
    column: u32,
}

impl Position {
    /// The byte offset of this one-based line/column in `source`, clamped to its length.
    fn offset(&self, source: &str) -> u32 {
        let target = self.line as usize;
        let column = self.column.saturating_sub(1) as usize;
        let mut offset = 0usize;
        for (index, line) in source.split_inclusive('\n').enumerate() {
            if index + 1 == target {
                offset += column.min(line.len());
                return u32::try_from(offset).unwrap_or(u32::MAX);
            }
            offset += line.len();
        }
        u32::try_from(offset.min(source.len())).unwrap_or(u32::MAX)
    }
}
