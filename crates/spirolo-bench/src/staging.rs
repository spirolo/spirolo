//! Stage an entry's committed inputs into a worktree before `check` runs at a commit.
//!
//! The corpus commits the mutation reports a rule reads under
//! `corpus/<entry>/inputs/<commit>/<path in tree>/…` so the bench is reproducible offline
//! and Stryker never runs inside it. Before `check` runs at a commit, the harness copies that
//! tree into the worktree at the same relative paths, so the binary reads the report where
//! the project would keep it. A commit with no `inputs/<commit>/` directory runs exactly as
//! before.

use std::path::{Path, PathBuf};

/// Copy `<entry_dir>/inputs/<commit>/` into `worktree` at the same relative paths, returning
/// the number of files staged.
///
/// A commit with no `inputs/<commit>/` directory stages nothing and returns `0`, so the
/// worktree runs exactly as the clone would. An I/O failure while copying is an error.
///
/// # Errors
///
/// Returns the failing path and the OS error when a directory cannot be read or created or a
/// file cannot be copied.
pub fn stage_inputs(entry_dir: &Path, commit: &str, worktree: &Path) -> Result<usize, String> {
    let source = entry_dir.join("inputs").join(commit);
    if !source.is_dir() {
        return Ok(0);
    }
    copy_tree(&source, worktree)
}

/// Recursively copy every file under `source` into `dest`, creating directories as needed and
/// returning the count of files copied.
fn copy_tree(source: &Path, dest: &Path) -> Result<usize, String> {
    let mut staged = 0;
    let entries =
        std::fs::read_dir(source).map_err(|error| format!("read {}: {error}", source.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("read {}: {error}", source.display()))?;
        let from = entry.path();
        let to = dest.join(entry.file_name());
        if from.is_dir() {
            staged += copy_tree(&from, &to)?;
        } else {
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| format!("create {}: {error}", parent.display()))?;
            }
            std::fs::copy(&from, &to)
                .map_err(|error| format!("copy {} to {}: {error}", from.display(), to.display()))?;
            staged += 1;
        }
    }
    Ok(staged)
}

/// Inputs staged into a clone in place: the files and directories staging created, removed
/// again when this is dropped, so the clone is left as it was found.
#[derive(Debug)]
pub struct StagedInputs {
    /// Files staging created, in creation order.
    files: Vec<PathBuf>,
    /// Directories staging created, parents before children.
    dirs: Vec<PathBuf>,
}

impl StagedInputs {
    /// The number of files staging added to the clone.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.files.len()
    }

    /// Whether staging added nothing.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

impl Drop for StagedInputs {
    fn drop(&mut self) {
        for file in &self.files {
            let _ = std::fs::remove_file(file);
        }
        for dir in self.dirs.iter().rev() {
            let _ = std::fs::remove_dir(dir);
        }
    }
}

/// Stage `<entry_dir>/inputs/<commit>/` into the clone `repo` itself, for a check that must
/// run on the clone's own substrate (its installed `node_modules`) rather than a bare
/// worktree (#130).
///
/// A file the clone already holds byte for byte is left alone; a missing one is copied and
/// recorded. Dropping the returned [`StagedInputs`] removes exactly what was added.
///
/// # Errors
///
/// Refuses, before writing anything, when an input would overwrite a different file in the
/// clone, naming its relative path; returns the failing path and the OS error when a file
/// cannot be read or copied.
pub fn stage_inputs_in_place(
    entry_dir: &Path,
    commit: &str,
    repo: &Path,
) -> Result<StagedInputs, String> {
    let source = entry_dir.join("inputs").join(commit);
    let mut inputs = Vec::new();
    if source.is_dir() {
        list_files(&source, Path::new(""), &mut inputs)?;
    }
    let mut missing = Vec::new();
    for relative in inputs {
        let (from, to) = (source.join(&relative), repo.join(&relative));
        if !to.exists() {
            missing.push((from, to));
            continue;
        }
        let same = std::fs::read(&from).ok() == std::fs::read(&to).ok();
        if !same {
            return Err(format!(
                "refusing to overwrite {} in the clone with a different staged input",
                relative.display()
            ));
        }
    }

    let mut staged = StagedInputs {
        files: Vec::new(),
        dirs: Vec::new(),
    };
    for (from, to) in missing {
        if let Some(parent) = to.parent() {
            let mut absent: Vec<PathBuf> = parent
                .ancestors()
                .take_while(|dir| !dir.exists())
                .map(Path::to_path_buf)
                .collect();
            absent.reverse();
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("create {}: {error}", parent.display()))?;
            staged.dirs.extend(absent);
        }
        std::fs::copy(&from, &to)
            .map_err(|error| format!("copy {} to {}: {error}", from.display(), to.display()))?;
        staged.files.push(to);
    }
    Ok(staged)
}

/// Every file under `dir`, as paths relative to the staging root, in directory order.
fn list_files(dir: &Path, relative: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries =
        std::fs::read_dir(dir).map_err(|error| format!("read {}: {error}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("read {}: {error}", dir.display()))?;
        let path = entry.path();
        let name = relative.join(entry.file_name());
        if path.is_dir() {
            list_files(&path, &name, out)?;
        } else {
            out.push(name);
        }
    }
    Ok(())
}

/// Where [`stage_runs`] puts the runs in a worktree, relative to its root.
const STAGED_RUNS: &str = ".spirolo-bench-runs";

/// The runs [`stage_runs`] staged: the glob `flake ingest` reads them by, how many reports,
/// and the one tree hash every sidecar records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedRuns {
    /// The reports, as a glob relative to the worktree.
    pub glob: String,
    /// The number of reports staged.
    pub reports: usize,
    /// The tree hash every run's sidecar records: the commit the worktree must be at.
    pub tree_hash: String,
}

/// Stage every run under `runs_dir` into `worktree/.spirolo-bench-runs/<run-id>/`.
///
/// A run is `<run-id>/run.json` plus `*.json.gz` vitest reports, as the run generator writes
/// them; each report is gunzipped and its sidecar copied beside it.
///
/// vitest writes absolute file paths under the directory the run was made in, which the
/// sidecar records as `root`; staging rewrites that quoted prefix to the worktree and
/// changes nothing else, so `flake ingest` can make the paths root-relative.
///
/// # Errors
///
/// When a directory or file cannot be read or written, a report is not gzip, a sidecar has
/// no `tree_hash` or `root`, or two runs record different tree hashes.
pub fn stage_runs(runs_dir: &Path, worktree: &Path) -> Result<StagedRuns, String> {
    use std::io::Read as _;

    let mut runs: Vec<PathBuf> = std::fs::read_dir(runs_dir)
        .map_err(|error| format!("read {}: {error}", runs_dir.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    runs.sort();

    let mut trees = std::collections::BTreeSet::new();
    let mut reports = 0;
    for run in &runs {
        let sidecar_path = run.join("run.json");
        let sidecar = std::fs::read(&sidecar_path)
            .map_err(|error| format!("read {}: {error}", sidecar_path.display()))?;
        let fields: serde_json::Value = serde_json::from_slice(&sidecar)
            .map_err(|error| format!("{}: {error}", sidecar_path.display()))?;
        let field = |key: &str| {
            fields[key]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{} has no {key}", sidecar_path.display()))
        };
        trees.insert(field("tree_hash")?);
        let from = format!("\"{}/", field("root")?.trim_end_matches('/'));
        let to = format!("\"{}/", worktree.display());

        let dest = worktree
            .join(STAGED_RUNS)
            .join(run.file_name().unwrap_or_default());
        std::fs::create_dir_all(&dest)
            .map_err(|error| format!("create {}: {error}", dest.display()))?;
        std::fs::write(dest.join("run.json"), &sidecar)
            .map_err(|error| format!("write {}: {error}", dest.display()))?;
        let mut names: Vec<String> = std::fs::read_dir(run)
            .map_err(|error| format!("read {}: {error}", run.display()))?
            .filter_map(Result::ok)
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| name.ends_with(".json.gz"))
            .collect();
        names.sort();
        for name in names {
            let path = run.join(&name);
            let bytes = std::fs::read(&path)
                .map_err(|error| format!("read {}: {error}", path.display()))?;
            let mut text = String::new();
            flate2::read::GzDecoder::new(bytes.as_slice())
                .read_to_string(&mut text)
                .map_err(|error| format!("gunzip {}: {error}", path.display()))?;
            let staged = dest.join(name.trim_end_matches(".gz"));
            std::fs::write(&staged, text.replace(&from, &to))
                .map_err(|error| format!("write {}: {error}", staged.display()))?;
            reports += 1;
        }
    }

    let mut trees = trees.into_iter();
    let tree_hash = match (trees.next(), trees.next()) {
        (Some(tree), None) => tree,
        (None, _) => return Err(format!("no run under {}", runs_dir.display())),
        (Some(one), Some(two)) => {
            return Err(format!(
                "the runs under {} record more than one tree hash ({one}, {two}): the bench \
                 checks one tree",
                runs_dir.display()
            ))
        }
    };
    Ok(StagedRuns {
        glob: format!("{STAGED_RUNS}/*/vitest-*.json"),
        reports,
        tree_hash,
    })
}
