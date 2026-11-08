//! Reading the artifact files `spirolo-inputs` parses: the binary finds them on disk and
//! hands the bytes over, so the parser crate stays free of I/O.
//!
//! Today the one artifact is the mutation report. The globs come from `inputs.mutation`
//! (or its default); every matching file is read and parsed, and the rows are merged into
//! the model. Reports live where the runner keeps them, often under a `reports/` directory
//! a project `.gitignore`s, so the walk does not consult `.gitignore` — it only prunes the
//! dependency and VCS directories no report is ever kept in.

use std::collections::HashSet;
use std::path::Path;

use globset::{Glob, GlobSetBuilder};
use ignore::WalkBuilder;
use spirolo_frameworks::{imported_calls, read_snapshot};
use spirolo_inputs::{read_mutation_report, read_numstat};
use spirolo_model::{
    relative_slash, Churn, Diagnostic, Module, Mutants, MutationReport, MutationTest, Severity,
    Snapshot, TestCase,
};

/// Read the churn input: `flag` when given (a path, or `-` for stdin), otherwise `rc_file`
/// relative to `root`. `None` rows when neither names one. An input that cannot be read is
/// one `churn_unreadable` diagnostic and `None` rows, never an abort; the lines the reader
/// cannot use are its own diagnostics, labelled with the input as it was named.
pub fn read_churn(
    root: &Path,
    rc_file: Option<&str>,
    flag: Option<&Path>,
) -> (Option<Vec<Churn>>, Vec<Diagnostic>) {
    let (label, bytes) = match (flag, rc_file) {
        (Some(flag), _) if flag == Path::new("-") => {
            let mut bytes = Vec::new();
            let read = std::io::Read::read_to_end(&mut std::io::stdin(), &mut bytes);
            ("-".to_owned(), read.map(|_| bytes))
        }
        (Some(flag), _) => (flag.to_string_lossy().into_owned(), std::fs::read(flag)),
        (None, Some(file)) => (file.to_owned(), std::fs::read(root.join(file))),
        (None, None) => return (None, Vec::new()),
    };
    match bytes {
        Ok(bytes) => {
            let table = read_numstat(&label, &bytes);
            (Some(table.churn), table.diagnostics)
        }
        Err(error) => (
            None,
            vec![Diagnostic {
                code: "churn_unreadable".to_owned(),
                severity: Severity::Error,
                file: Some(label),
                span: None,
                message: format!("cannot read the churn input: {error}"),
            }],
        ),
    }
}

/// The mutation rows and diagnostics gathered from every report the globs matched, sorted
/// so two runs over the same tree are byte-identical.
#[derive(Debug, Default)]
pub struct Mutation {
    /// One row per mutant, across every report.
    pub mutants: Vec<Mutants>,
    /// One row per test a report named, across every report.
    pub tests: Vec<MutationTest>,
    /// One row per report read.
    pub reports: Vec<MutationReport>,
    /// What the reader could not decide, plus one glob-invalid diagnostic per bad glob.
    pub diagnostics: Vec<Diagnostic>,
}

/// Find the mutation reports under `root` that `globs` name, read each, and merge the rows.
///
/// `test_cases` are the model's tests, so each report's tests join to the row they
/// describe. A glob that does not compile is one `mutation_glob_invalid` diagnostic and is
/// skipped; a report that does not parse is the reader's own diagnostic. A glob that
/// matches nothing yields empty tables and no error.
pub fn read_mutation_reports(root: &Path, globs: &[String], test_cases: &[TestCase]) -> Mutation {
    let mut mutation = Mutation::default();

    let (set, invalid) = compile_globs(globs);
    for glob in invalid {
        mutation.diagnostics.push(Diagnostic {
            code: "mutation_glob_invalid".to_owned(),
            severity: spirolo_model::Severity::Warning,
            file: None,
            span: None,
            message: format!("inputs.mutation glob {glob:?} is not valid"),
        });
    }
    let Some(set) = set else {
        return mutation;
    };

    for relative in walk_matching(root, &set) {
        let Ok(bytes) = std::fs::read(root.join(&relative)) else {
            continue;
        };
        let tables = read_mutation_report(&relative, &bytes, test_cases);
        mutation.mutants.extend(tables.mutants);
        mutation.tests.extend(tables.tests);
        mutation.reports.extend(tables.reports);
        mutation.diagnostics.extend(tables.diagnostics);
    }

    mutation.mutants.sort();
    mutation.tests.sort();
    mutation.reports.sort();
    mutation.diagnostics.sort();
    mutation
}

/// The repo-relative, slash-separated paths under `root` that any of `globs` match, sorted.
/// Used by `init` to detect which reports a project already keeps.
pub fn find_reports(root: &Path, globs: &[String]) -> Vec<String> {
    let (Some(set), _) = compile_globs(globs) else {
        return Vec::new();
    };
    walk_matching(root, &set)
}

/// The `.snap` rows and diagnostics gathered from every `__snapshots__/*.snap` file under
/// the root, sorted so two runs over the same tree are byte-identical.
#[derive(Debug, Default)]
pub struct Snapshots {
    /// One row per `.snap` file read.
    pub snapshots: Vec<Snapshot>,
    /// One `snapshot_key_malformed` diagnostic per unparsable key, across every file.
    pub diagnostics: Vec<Diagnostic>,
}

/// Walk the `__snapshots__/*.snap` files under `root`, read each with the frameworks reader,
/// and pair each with its sibling test file.
///
/// The sibling of `dir/__snapshots__/name.snap` is `dir/name`; it is recorded only when the
/// model read it as a `module`, so a `.snap` whose test file is gone reads as an orphan
/// (`sibling: None`). The walk honours the project's own ignore files, as module discovery
/// does: a `.snap` under an ignored directory (an agent's worktree copy, a build output) is
/// not the project's snapshot, and its test file was never read as a module, so without
/// this every such file would read as an orphan.
pub fn read_snapshots(root: &Path, modules: &[Module]) -> Snapshots {
    let module_paths: HashSet<&str> = modules.iter().map(|module| module.path.as_str()).collect();
    let mut result = Snapshots::default();
    for relative in walk_snapshots(root) {
        let Ok(text) = std::fs::read_to_string(root.join(&relative)) else {
            continue;
        };
        let facts = read_snapshot(&relative, &text);
        let sibling = sibling_of(&relative).filter(|path| module_paths.contains(path.as_str()));
        let imported_calls = sibling
            .as_deref()
            .and_then(|path| Some((path, std::fs::read_to_string(root.join(path)).ok()?)))
            .map(|(path, source)| imported_calls(path, &source))
            .unwrap_or_default();
        result.snapshots.push(Snapshot {
            file: relative,
            sibling,
            entries: facts.entries,
            imported_calls,
        });
        result.diagnostics.extend(facts.diagnostics);
    }
    result.snapshots.sort();
    result.diagnostics.sort();
    result
}

/// The sibling test file of a `__snapshots__/<name>.snap` path: the `<name>` beside the
/// `__snapshots__` directory. `None` for a `.snap` that is not directly inside one.
fn sibling_of(relative: &str) -> Option<String> {
    let (dir, name) = relative.rsplit_once('/')?;
    let parent = dir.strip_suffix("__snapshots__")?;
    let parent = parent.strip_suffix('/').unwrap_or(parent);
    let stem = name.strip_suffix(".snap")?;
    if parent.is_empty() {
        Some(stem.to_owned())
    } else {
        Some(format!("{parent}/{stem}"))
    }
}

/// The repo-relative paths of every `.snap` file directly inside a `__snapshots__`
/// directory under `root` and outside the project's ignored paths, sorted so two runs read
/// in one order.
fn walk_snapshots(root: &Path) -> Vec<String> {
    // The same ignore settings as the model's walk: the project's own ignore files count,
    // nothing above the root or in the user's global config does.
    let mut paths: Vec<String> = WalkBuilder::new(root)
        .hidden(false)
        .parents(false)
        .require_git(false)
        .git_global(false)
        .filter_entry(|entry| entry.file_name() != "node_modules" && entry.file_name() != ".git")
        .build()
        .flatten()
        .filter(|entry| entry.file_type().is_some_and(|kind| kind.is_file()))
        .filter_map(|entry| relative_slash(root, entry.path()))
        // `sibling_of` succeeds only for a `.snap` directly inside a `__snapshots__`
        // directory, so it is the whole filter.
        .filter(|relative| sibling_of(relative).is_some())
        .collect();
    paths.sort();
    paths
}

/// Compile the globs into a set, returning the globs that did not compile so the caller can
/// diagnose them. A leading `./` is noise once matched against repo-relative paths.
fn compile_globs(globs: &[String]) -> (Option<globset::GlobSet>, Vec<String>) {
    let mut builder = GlobSetBuilder::new();
    let mut invalid = Vec::new();
    for glob in globs {
        match Glob::new(glob.trim_start_matches("./")) {
            Ok(compiled) => {
                builder.add(compiled);
            }
            Err(_) => invalid.push(glob.clone()),
        }
    }
    (builder.build().ok(), invalid)
}

/// Walk `root` (ignoring `.gitignore`, pruning `node_modules` and `.git`) and return the
/// repo-relative paths of the files `set` matches, sorted so two runs read in one order.
fn walk_matching(root: &Path, set: &globset::GlobSet) -> Vec<String> {
    let mut paths: Vec<String> = WalkBuilder::new(root)
        .standard_filters(false)
        .filter_entry(|entry| entry.file_name() != "node_modules" && entry.file_name() != ".git")
        .build()
        .flatten()
        .filter(|entry| entry.file_type().is_some_and(|kind| kind.is_file()))
        .filter_map(|entry| relative_slash(root, entry.path()))
        .filter(|relative| set.is_match(relative))
        .collect();
    paths.sort();
    paths
}
