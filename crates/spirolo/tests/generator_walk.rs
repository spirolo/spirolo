//! `spirolo check` end to end on a `generator-walk` fixture project: what a maintainer and an
//! agent get from the binary when the saga recogniser finds step-walk sites.
//!
//! Seam 2 of the PRD's testing decisions: a small fixture with a `gen.next().value` site
//! against an effect creator, a `testSaga` chain, and a plain assertion that must not fire,
//! driven through the real binary with `assert_cmd`, its findings JSON snapshotted with
//! `insta`. The rule's own branches are covered on hand-built models in `spirolo-rules` and
//! the recogniser's on snippets in `spirolo-frameworks`; here we prove the wiring — recogniser,
//! model, rule, report — is whole.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(dead_code)]
#[path = "support/status.rs"]
mod status_support;

fn spirolo() -> Command {
    Command::cargo_bin("spirolo").unwrap()
}

/// A fresh project directory under the target tree, named after the test that asked for it.
fn project(name: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
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

/// A fixture with three saga test sites and one plain assertion that must not fire: two
/// `gen.next().value` assertions against `call` and `put` from `redux-saga/effects`, one
/// `testSaga` chain with two `.next()` steps, and a `toEqual` against a plain object.
fn sagas(name: &str) -> PathBuf {
    let root = project(name);
    write(
        &root,
        "src/api/sagas.test.ts",
        "import { call, put } from 'redux-saga/effects'\n\
         import { testSaga } from 'redux-saga-test-plan'\n\
         import { fetchUser, watchUser } from './sagas'\n\
         it('walks the fetch saga', () => {\n\
         \x20 const gen = fetchUser(1)\n\
         \x20 expect(gen.next().value).toEqual(call(api, 1))\n\
         \x20 expect(gen.next(user).value).toEqual(put(loaded(user)))\n\
         })\n\
         it('walks the watcher with testSaga', () => {\n\
         \x20 testSaga(watchUser).next().take('FETCH').next().isDone()\n\
         })\n\
         it('asserts a value, not a step', () => {\n\
         \x20 expect(result).toEqual({ ok: true })\n\
         })\n",
    );
    write(
        &root,
        "src/api/sagas.ts",
        "export function* fetchUser(id) { yield id }\n\
         export function* watchUser() { yield 'FETCH' }\n",
    );
    root
}

// ---------------------------------------------------------------------------- the human run

#[test]
fn check_reports_the_step_walk_sites_at_info() {
    let root = sagas("human");
    spirolo()
        .args(["check", "--all"])
        .arg(&root)
        .assert()
        // generator-walk is info, so a step-walk is reported but the run exits 0.
        .success()
        .stdout(predicate::str::contains("● generator-walk ("));
}

// ---------------------------------------------------------------------------- the JSON run

#[test]
fn the_findings_json_counts_the_sites_and_carries_explain() {
    let root = sagas("json");
    let output = spirolo()
        .args([
            "check",
            "--all",
            "--format",
            "json",
            "--rule",
            "generator-walk",
        ])
        .arg(&root)
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let rows = json["findings"].as_array().unwrap();
    assert!(
        rows.iter().all(|f| f["rule"] == "generator-walk"),
        "every row is generator-walk"
    );

    // The one file's count row: listed, four sites, each named in the evidence.
    let listed: Vec<&serde_json::Value> =
        rows.iter().filter(|f| f["outcome"] == "listed").collect();
    assert_eq!(
        listed.len(),
        1,
        "one count row for the one file: {listed:?}"
    );
    assert_eq!(
        listed[0]["id"],
        "generator-walk src/api/sagas.test.ts count#0"
    );
    let count_evidence = listed[0]["evidence"].as_array().unwrap();
    assert!(count_evidence
        .iter()
        .any(|e| e["key"] == "sites" && e["value"] == "4"));
    assert!(
        count_evidence.iter().any(|e| e["key"] == "site_list"
            && e["value"]
                == "6:gen.next:call, 7:gen.next:put, 10:testSaga:take, 10:testSaga:isDone"),
        "{count_evidence:?}"
    );

    // Two gen.next().value sites + two testSaga steps = four; the plain toEqual is not one.
    let findings: Vec<&serde_json::Value> =
        rows.iter().filter(|f| f["outcome"] == "finding").collect();
    assert_eq!(findings.len(), 4, "four step-walk sites: {findings:?}");
    // Every finding carries a repair-oriented explain pointing at expectSaga(...).run().
    assert!(
        findings
            .iter()
            .all(|f| f["explain"].as_str().unwrap().contains("expectSaga")),
        "explain points to expectSaga"
    );
    // The effects the fixture yields are named in the evidence.
    let effects: Vec<String> = findings
        .iter()
        .flat_map(|f| f["evidence"].as_array().unwrap())
        .filter(|e| e["key"] == "effect")
        .map(|e| e["value"].as_str().unwrap().to_owned())
        .collect();
    assert!(effects.contains(&"call".to_owned()), "{effects:?}");
    assert!(effects.contains(&"put".to_owned()), "{effects:?}");
    assert!(effects.contains(&"isDone".to_owned()), "{effects:?}");
}

#[test]
fn the_findings_json_is_stable() {
    let root = sagas("snapshot");
    let output = spirolo()
        .args([
            "check",
            "--all",
            "--format",
            "json",
            "--rule",
            "generator-walk",
        ])
        .arg(&root)
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    insta::assert_json_snapshot!(
        "check_json_generator_walk",
        status_support::redact_status(&json["findings"])
    );
}

// ---------------------------------------------------------------------------- the agent run

#[test]
fn the_agent_view_prints_each_site_with_its_explain() {
    let root = sagas("agent");
    spirolo()
        .args(["check", "--all", "--agent", "--rule", "generator-walk"])
        .arg(&root)
        .assert()
        .success()
        .stdout(
            predicate::str::contains("generator-walk").and(predicate::str::contains("expectSaga")),
        );
}
