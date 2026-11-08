//! Which files on disk the model is allowed to see.
//!
//! The walk decides what the model reads as source and as data: every module the tables
//! carry, and every `package.json` the package boundary is drawn from, comes off the list
//! this module returns, so what is skipped here is skipped there. It is not the model's
//! only contact with the filesystem — the resolver, the preset reader and the workspace
//! reader open files of their own; the crate docs list every one.

use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

use crate::Language;

/// Directories never walked, whatever `.gitignore` says.
///
/// `node_modules`, `dist` and `coverage` are other people's output; `.git` is not
/// source at all. A project that genuinely keeps sources in `dist/` is a config knob
/// for a later milestone, not a default.
pub const SKIPPED_DIRECTORIES: [&str; 4] = [".git", "node_modules", "dist", "coverage"];

/// Files read as data rather than parsed as modules: each carries facts the model needs
/// — a package's name and directory, or a runner/mutation config written as JSON — but
/// is never a `Module` row.
pub const DATA_FILES: [&str; 4] = [
    "package.json",
    "jest.config.json",
    "stryker.conf.json",
    "stryker.config.json",
];

/// Every JS/TS file under `root`, sorted, with its language, plus every [`DATA_FILES`]
/// entry as [`Language::Json`].
///
/// Sorting happens here rather than after extraction so the parallel phase cannot make
/// the output depend on thread scheduling. Unreadable directories are skipped silently:
/// a walk error is not a fact about the project's tests.
pub fn discover(root: &Path, respect_gitignore: bool) -> Vec<(PathBuf, Language)> {
    let mut builder = WalkBuilder::new(root);
    builder
        // Hidden files are walked: `.github/workflows` is a gate source.
        .hidden(false)
        // Only the project's own ignore files count. Honouring the ignore files of
        // whatever directory the project happens to be checked out under would make
        // the model depend on the machine.
        .parents(false)
        .require_git(false)
        .git_ignore(respect_gitignore)
        .git_exclude(respect_gitignore)
        .git_global(false)
        .ignore(respect_gitignore)
        .filter_entry(|entry| {
            !entry
                .file_name()
                .to_str()
                .is_some_and(|name| SKIPPED_DIRECTORIES.contains(&name))
        });

    let mut files: Vec<(PathBuf, Language)> = builder
        .build()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_some_and(|kind| kind.is_file()))
        .filter_map(|entry| {
            let language = language_of(entry.path())?;
            Some((entry.into_path(), language))
        })
        .collect();

    files.sort();
    files
}

/// Every GitHub Actions workflow file under `root`, sorted: a `.yml`/`.yaml` directly under
/// a `.github/workflows` directory. These are read as YAML data by [`crate::ci`], never as
/// modules, so they are discovered on their own rather than joining the JS/TS/JSON walk.
///
/// The same skip list and ignore settings as [`discover`] apply, so a workflow under
/// `node_modules` (a vendored action's fixtures) is not read.
pub fn discover_workflows(root: &Path, respect_gitignore: bool) -> Vec<PathBuf> {
    let mut builder = WalkBuilder::new(root);
    builder
        .hidden(false)
        .parents(false)
        .require_git(false)
        .git_ignore(respect_gitignore)
        .git_exclude(respect_gitignore)
        .git_global(false)
        .ignore(respect_gitignore)
        .filter_entry(|entry| {
            !entry
                .file_name()
                .to_str()
                .is_some_and(|name| SKIPPED_DIRECTORIES.contains(&name))
        });

    let mut files: Vec<PathBuf> = builder
        .build()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_some_and(|kind| kind.is_file()))
        .filter(|entry| is_workflow_path(entry.path()))
        .map(ignore::DirEntry::into_path)
        .collect();
    files.sort();
    files
}

/// Whether a path is a `.yml`/`.yaml` directly inside a `.github/workflows` directory.
fn is_workflow_path(path: &Path) -> bool {
    let is_yaml = path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext == "yml" || ext == "yaml");
    if !is_yaml {
        return false;
    }
    path.parent().is_some_and(|parent| {
        parent.file_name().and_then(|name| name.to_str()) == Some("workflows")
            && parent
                .parent()
                .and_then(|grand| grand.file_name())
                .and_then(|name| name.to_str())
                == Some(".github")
    })
}

/// The dialect the extension announces, [`Language::Json`] for a data file the model
/// reads by name, or `None` when the file is neither.
fn language_of(path: &Path) -> Option<Language> {
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| DATA_FILES.contains(&name))
    {
        return Some(Language::Json);
    }
    match path.extension()?.to_str()? {
        "ts" | "mts" | "cts" => Some(Language::TypeScript),
        "tsx" => Some(Language::Tsx),
        "js" | "mjs" | "cjs" => Some(Language::JavaScript),
        "jsx" => Some(Language::Jsx),
        // A Jest transform can collect CoffeeScript. Inventory it even though the
        // JS/TS parser cannot extract cases from its body.
        "coffee" => Some(Language::Unknown),
        _ => None,
    }
}
