//! `--propose` end to end: the flag adds a `proposal` to every hygiene finding in every
//! format, gates merge and delete on a rule's independent-oracle PASS, is off by default, is
//! rejected on commands other than `check` and `hygiene`, writes nothing to the tree, and is
//! deterministic.
//!
//! Seam 2 of the PRD's testing decisions, the cross-cutting half: the per-rule fixtures prove
//! each proposal's shape in the four views; here we prove the flag's wiring — which commands
//! accept it, that the field is absent without it, that the tree is untouched, and that two
//! runs match byte for byte.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;

#[path = "support/git.rs"]
mod git_support;

fn spirolo() -> Command {
    Command::cargo_bin("spirolo").unwrap()
}

/// A fresh project directory under the target tree, named after the test that asked for it.
fn project(name: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("propose-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}

/// Write `contents` to `path` under `root`, creating parents.
fn write(root: &Path, path: &str, contents: &str) {
    let full = root.join(path);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(full, contents).unwrap();
}

/// A project with a within-file duplicate pair, so `duplicate-test` fires at least once.
fn duplicates(name: &str) -> PathBuf {
    let root = project(name);
    write(
        &root,
        "src/format.ts",
        "export function format(value: string): string { return value }\n",
    );
    write(
        &root,
        "src/format.test.ts",
        "import { describe, it, expect } from 'vitest'\n\
         import { format } from './format'\n\
         \n\
         describe('format', () => {\n\
         \x20 it('formats the value', () => {\n\
         \x20   const out = format(value)\n\
         \x20   expect(out).toBe('ok')\n\
         \x20 })\n\
         \x20 it('renders the value the same way', () => {\n\
         \x20   const rendered = format(value)\n\
         \x20   expect(rendered).toBe('ok')\n\
         \x20 })\n\
         })\n",
    );
    root
}

/// The findings of a `--format json` run of `args` over `root`.
fn findings(root: &Path, args: &[&str]) -> Vec<serde_json::Value> {
    let output = spirolo().args(args).arg(root).output().unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    json["findings"].as_array().unwrap().clone()
}

#[test]
fn check_propose_keeps_every_finding_of_a_rule_no_independent_oracle_has_passed() {
    // No hygiene rule has a PASS verdict on an independent oracle yet (M3 reopen, #134), so
    // the flag may attach a proposal but never a merge or a delete: an agent acting on the
    // agent view would otherwise remove tests on a rule measured as failing.
    let root = duplicates("adds");
    let findings = findings(
        &root,
        &[
            "check",
            "--all",
            "--propose",
            "--format",
            "json",
            "--rule",
            "duplicate-test",
        ],
    );
    assert!(
        !findings.is_empty(),
        "the fixture has a clone: {findings:#?}"
    );
    assert!(
        findings.iter().all(|f| f["proposal"]["action"] == "keep"
            && f["proposal"]["reason"]
                .as_str()
                .is_some_and(|reason| reason.contains("independent oracle"))),
        "every duplicate-test finding is kept, saying why: {findings:#?}"
    );
}

#[test]
fn hygiene_accepts_the_flag_too() {
    let root = duplicates("hygiene");
    let findings = findings(
        &root,
        &["hygiene", "--all", "--propose", "--format", "json"],
    );
    assert!(
        findings.iter().any(|f| f.get("proposal").is_some()),
        "hygiene --propose carries proposals: {findings:#?}"
    );
}

/// The parsed JSON document of a `--format json` run of `args` over `root`.
fn document(root: &Path, args: &[&str]) -> serde_json::Value {
    let output = spirolo().args(args).arg(root).output().unwrap();
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn without_the_flag_no_finding_carries_a_proposal_and_the_schema_is_unchanged() {
    let root = duplicates("off");
    let plain = document(root.as_path(), &["check", "--all", "--format", "json"]);
    assert!(
        plain["findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f.get("proposal").is_none()),
        "off by default: no finding carries a proposal: {plain}"
    );
    // The proposal field never bumps the schema version: the flagged run reports the same
    // version as the plain one.
    let proposed = document(
        root.as_path(),
        &["check", "--all", "--propose", "--format", "json"],
    );
    assert_eq!(
        plain["schema_version"], proposed["schema_version"],
        "the proposal field does not change the JSON schema version"
    );
}

#[test]
fn propose_is_rejected_on_commands_other_than_check_and_hygiene() {
    let root = duplicates("rejected");
    for command in ["audit", "gate", "list", "ratchet"] {
        spirolo()
            .args([command, "--propose"])
            .arg(&root)
            .assert()
            .failure()
            .stderr(predicate::str::contains("--propose"));
    }
}

#[test]
fn check_propose_writes_nothing_to_the_tree() {
    assert_propose_writes_nothing_to_the_tree(&duplicates("git-clean"));
}

fn assert_propose_writes_nothing_to_the_tree(root: &Path) {
    // A committed tree, so any file the tool touched would show in `git status`.
    for args in [
        &["init", "-q"][..],
        &["config", "user.email", "t@t"][..],
        &["config", "user.name", "t"][..],
        &["add", "-A"][..],
        &["commit", "-q", "-m", "fixture"][..],
    ] {
        let ok = git_support::command(root)
            .args(args)
            .status()
            .unwrap()
            .success();
        assert!(ok, "git {args:?} failed");
    }

    for view in [
        &["check", "--all", "--propose"][..],
        &["check", "--all", "--propose", "--format", "json"][..],
        &["check", "--all", "--propose", "--format", "sarif"][..],
        &["hygiene", "--all", "--propose", "--agent"][..],
    ] {
        spirolo().args(view).arg(root).assert().success();
    }

    let status = git_support::command(root)
        .args(["status", "--porcelain"])
        .output()
        .unwrap();
    assert!(
        status.stdout.is_empty(),
        "the tree is untouched after check --propose: {}",
        String::from_utf8_lossy(&status.stdout)
    );
}

#[test]
fn two_propose_runs_are_byte_identical() {
    let root = duplicates("determinism");
    let run = || {
        spirolo()
            .args(["check", "--all", "--propose", "--format", "json"])
            .arg(&root)
            .output()
            .unwrap()
            .stdout
    };
    assert_eq!(run(), run(), "two --propose runs are byte-identical");
}

#[test]
fn fixture_writer_leaves_the_hook_repository_untouched() {
    if std::env::var_os("SPIROLO_GIT_ENV_CHILD").is_some() {
        assert_propose_writes_nothing_to_the_tree(&duplicates("hook-git-clean"));
    } else {
        git_support::assert_hook_safe("fixture_writer_leaves_the_hook_repository_untouched");
    }
}
