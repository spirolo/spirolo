//! Shared Git command construction for disposable test repositories.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const REPOSITORY_ENV: &[&str] = &[
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_COMMON_DIR",
    "GIT_PREFIX",
    // The remaining repository-local overrides listed by `git rev-parse --local-env-vars`.
    "GIT_CONFIG",
    "GIT_CONFIG_PARAMETERS",
    "GIT_CONFIG_COUNT",
    "GIT_IMPLICIT_WORK_TREE",
    "GIT_GRAFT_FILE",
    "GIT_NO_REPLACE_OBJECTS",
    "GIT_REPLACE_REF_BASE",
    "GIT_SHALLOW_FILE",
    // Receive hooks can also export quarantine state.
    "GIT_QUARANTINE_PATH",
];

pub fn command(root: &Path) -> Command {
    let mut command = Command::new("git");
    command.current_dir(root);
    remove_repository_env(&mut command);
    command
}

pub fn remove_repository_env(command: &mut Command) {
    // Hooks export repository overrides; current_dir (or git -C) does not defeat them.
    for name in REPOSITORY_ENV {
        command.env_remove(name);
    }
}

fn git(root: &Path, args: &[&str]) -> String {
    let output = command(root).args(args).output().unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn repository_bytes(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut files = BTreeMap::new();
    for entry in std::fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(repository_bytes(&path));
        } else {
            files.insert(path.clone(), std::fs::read(path).unwrap());
        }
    }
    files
}

/// Run an actual fixture test with each hook override and with all of them together.
pub fn assert_hook_safe(test_name: &str) {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("sentinel-{test_name}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    git(&root, &["init", "-q"]);
    git(&root, &["config", "user.name", "sentinel"]);
    git(&root, &["config", "user.email", "sentinel@example.com"]);
    git(&root, &["commit", "-q", "--allow-empty", "-m", "sentinel"]);
    let head = git(&root, &["rev-parse", "HEAD"]);
    let config = std::fs::read(root.join(".git/config")).unwrap();
    let before = repository_bytes(&root);
    let overrides: [(&str, OsString); 16] = [
        ("GIT_DIR", root.join(".git").into()),
        ("GIT_WORK_TREE", root.clone().into()),
        ("GIT_INDEX_FILE", root.join(".git/index").into()),
        ("GIT_OBJECT_DIRECTORY", root.join(".git/objects").into()),
        (
            "GIT_ALTERNATE_OBJECT_DIRECTORIES",
            root.join(".git/objects").into(),
        ),
        ("GIT_COMMON_DIR", root.join(".git").into()),
        ("GIT_PREFIX", "hook-prefix/".into()),
        ("GIT_CONFIG", root.join(".git/config").into()),
        ("GIT_CONFIG_PARAMETERS", "'core.bare'='true'".into()),
        ("GIT_CONFIG_COUNT", "1".into()),
        ("GIT_IMPLICIT_WORK_TREE", "0".into()),
        ("GIT_GRAFT_FILE", root.join(".git/info/grafts").into()),
        ("GIT_NO_REPLACE_OBJECTS", "1".into()),
        ("GIT_REPLACE_REF_BASE", "refs/sentinel/".into()),
        ("GIT_SHALLOW_FILE", root.join(".git/shallow").into()),
        ("GIT_QUARANTINE_PATH", root.join(".git/objects").into()),
    ];

    // A child process avoids changing the environment shared by parallel tests.
    for case in 0..=overrides.len() {
        let mut child = Command::new(std::env::current_exe().unwrap());
        remove_repository_env(&mut child);
        child
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .args(["--exact", test_name])
            .env("SPIROLO_GIT_ENV_CHILD", "1")
            .env("GIT_CONFIG_KEY_0", "core.bare")
            .env("GIT_CONFIG_VALUE_0", "true");
        let selected = overrides
            .get(case)
            .map_or(&overrides[..], std::slice::from_ref);
        child.envs(selected.iter().map(|(name, value)| (name, value)));
        let output = child.output().unwrap();

        assert_eq!(
            git(&root, &["rev-parse", "HEAD"]),
            head,
            "sentinel HEAD changed with {selected:?}"
        );
        assert_eq!(
            std::fs::read(root.join(".git/config")).unwrap(),
            config,
            "sentinel config changed with {selected:?}"
        );
        // HEAD alone misses objects or an index written into the sentinel without
        // moving a ref. Compare the real repository's bytes, including those files.
        assert!(
            repository_bytes(&root) == before,
            "sentinel files changed with {selected:?}"
        );
        assert!(
            output.status.success() && String::from_utf8_lossy(&output.stdout).contains("1 passed"),
            "fixture test {test_name} failed with {selected:?}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
