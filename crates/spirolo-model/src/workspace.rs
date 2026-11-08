//! The workspace's member packages, read from `pnpm-workspace.yaml` or the root
//! `package.json`'s `workspaces`.
//!
//! A monorepo's runner is started per member package (`pnpm -r`, `turbo run test`), so
//! a directory the workspace does not list — an archive, a prototype, a scripts folder —
//! is never a place the runner looks for tests, whatever configs or `*.test.ts` files it
//! carries. The model reads the member globs and answers one question: is this path
//! inside a member package? A project with no workspace definition has no boundary, and
//! every path is inside.
//!
//! Two files are read here, both at the root: `pnpm-workspace.yaml` and `package.json`.
//! Whether a named directory really is a package is answered from the manifests the walk
//! found, not by touching the disk again.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Mutex;

use globset::{Glob, GlobSet, GlobSetBuilder};

/// The member globs of one workspace, compiled, with a memo of the directories already
/// decided.
#[derive(Debug)]
pub struct Workspace {
    members: GlobSet,
    negated: GlobSet,
    /// The directories holding a `package.json` the walk found.
    package_dirs: HashSet<String>,
    /// Directory → whether it is a member package. A package is a directory the globs
    /// name that carries a `package.json`, the way pnpm and npm decide it.
    decided: Mutex<HashMap<String, bool>>,
}

impl Workspace {
    /// Read the workspace rooted at `root`, or `None` when it declares no members.
    /// `package_dirs` are the directories holding a `package.json` the walk found.
    ///
    /// `pnpm-workspace.yaml` wins over `package.json` `workspaces` when both exist, as
    /// pnpm ignores the latter.
    pub fn read(root: &Path, package_dirs: &HashSet<String>) -> Option<Self> {
        let globs = pnpm_globs(root).or_else(|| package_json_globs(root))?;
        if globs.is_empty() {
            return None;
        }
        let mut members = GlobSetBuilder::new();
        let mut negated = GlobSetBuilder::new();
        for glob in &globs {
            let (builder, pattern) = glob
                .strip_prefix('!')
                .map_or((&mut members, glob.as_str()), |pattern| {
                    (&mut negated, pattern)
                });
            let pattern = pattern.strip_prefix("./").unwrap_or(pattern);
            let pattern = pattern.trim_end_matches('/');
            if let Ok(compiled) = Glob::new(pattern) {
                builder.add(compiled);
            }
        }
        Some(Self {
            members: members.build().ok()?,
            negated: negated.build().ok()?,
            package_dirs: package_dirs.clone(),
            decided: Mutex::new(HashMap::new()),
        })
    }

    /// Whether `path` (repo-relative, slash-separated) lies inside a member package.
    ///
    /// Every ancestor directory is tried, so a file deep inside `packages/a/src/` is
    /// inside when `packages/a` is a member.
    pub fn contains(&self, path: &str) -> bool {
        let mut end = path.len();
        while let Some(slash) = path[..end].rfind('/') {
            let dir = &path[..slash];
            if self.is_member(dir) {
                return true;
            }
            end = slash;
        }
        // A path with no directory is a root-level file; the root is a member only when
        // the globs name it (`.`), which they never do in practice.
        self.is_member(path)
    }

    /// Whether `dir` itself is a member package: named by the globs, not negated, and
    /// carrying a `package.json`.
    fn is_member(&self, dir: &str) -> bool {
        if let Some(decided) = self
            .decided
            .lock()
            .ok()
            .and_then(|memo| memo.get(dir).copied())
        {
            return decided;
        }
        let named = self.members.is_match(dir) && !self.negated.is_match(dir);
        let member = named && self.package_dirs.contains(dir);
        if let Ok(mut memo) = self.decided.lock() {
            memo.insert(dir.to_owned(), member);
        }
        member
    }
}

/// The `packages:` list of `pnpm-workspace.yaml`, when the file exists and has one.
///
/// The file is a flat YAML list; reading it as lines (`- 'packages/*'`) covers every
/// spelling pnpm documents without a YAML dependency. A `packages:` block ends at the
/// next top-level key.
fn pnpm_globs(root: &Path) -> Option<Vec<String>> {
    let text = std::fs::read_to_string(root.join("pnpm-workspace.yaml")).ok()?;
    let mut globs = Vec::new();
    let mut in_packages = false;
    for line in text.lines() {
        let trimmed = line.trim_end();
        if trimmed.starts_with("packages:") {
            in_packages = true;
            continue;
        }
        if !in_packages {
            continue;
        }
        // Another top-level key ends the list.
        if !trimmed.is_empty() && !trimmed.starts_with(' ') && !trimmed.starts_with('-') {
            break;
        }
        let item = trimmed.trim_start();
        let Some(item) = item.strip_prefix('-') else {
            continue;
        };
        let item = item.split('#').next().unwrap_or("").trim();
        let item = item.trim_matches(|c| c == '\'' || c == '"');
        if !item.is_empty() {
            globs.push(item.to_owned());
        }
    }
    Some(globs)
}

/// The `workspaces` field of the root `package.json`: an array, or `{ packages: [...] }`.
fn package_json_globs(root: &Path) -> Option<Vec<String>> {
    let text = std::fs::read_to_string(root.join("package.json")).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let workspaces = value.get("workspaces")?;
    let list = workspaces
        .as_array()
        .or_else(|| workspaces.get("packages")?.as_array())?;
    Some(
        list.iter()
            .filter_map(|item| item.as_str().map(str::to_owned))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::Workspace;

    fn project(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir()
            .join("spirolo-workspace-tests")
            .join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// The directories holding a `package.json`, as the walk would have found them.
    fn packages(dirs: &[&str]) -> std::collections::HashSet<String> {
        dirs.iter().map(|dir| (*dir).to_owned()).collect()
    }

    #[test]
    fn pnpm_members_are_read_and_an_unlisted_directory_is_outside() {
        let root = project("ws-pnpm");
        std::fs::write(
            root.join("pnpm-workspace.yaml"),
            "packages:\n  - 'packages/*'\n  - \"apps/web\"   # comment\n  - '!packages/skipped'\n",
        )
        .unwrap();
        let dirs = packages(&["packages/a", "packages/skipped", "apps/web", "archive/old"]);

        let workspace = Workspace::read(&root, &dirs).expect("a workspace");
        assert!(workspace.contains("packages/a/src/x.test.ts"));
        assert!(workspace.contains("apps/web/vitest.config.ts"));
        assert!(!workspace.contains("packages/skipped/x.test.ts"), "negated");
        assert!(!workspace.contains("archive/old/x.test.ts"), "unlisted");
        assert!(!workspace.contains("scripts/x.test.ts"));
        assert!(!workspace.contains("root.test.ts"));
    }

    #[test]
    fn a_named_directory_without_a_package_json_is_not_a_member() {
        let root = project("ws-no-manifest");
        std::fs::write(
            root.join("pnpm-workspace.yaml"),
            "packages:\n  - packages/*\n",
        )
        .unwrap();
        let workspace = Workspace::read(&root, &packages(&[])).expect("a workspace");
        assert!(!workspace.contains("packages/bare/src/x.test.ts"));
    }

    #[test]
    fn package_json_workspaces_are_read_in_both_shapes() {
        let root = project("ws-npm-array");
        std::fs::write(root.join("package.json"), r#"{ "workspaces": ["libs/*"] }"#).unwrap();
        let dirs = packages(&["libs/a"]);
        assert!(Workspace::read(&root, &dirs)
            .unwrap()
            .contains("libs/a/x.ts"));

        let root = project("ws-npm-object");
        std::fs::write(
            root.join("package.json"),
            r#"{ "workspaces": { "packages": ["libs/*"] } }"#,
        )
        .unwrap();
        assert!(Workspace::read(&root, &dirs)
            .unwrap()
            .contains("libs/a/x.ts"));
    }

    #[test]
    fn a_project_without_a_workspace_has_no_boundary() {
        let root = project("ws-none");
        std::fs::write(root.join("package.json"), "{}\n").unwrap();
        assert!(Workspace::read(&root, &packages(&[""])).is_none());
    }
}
