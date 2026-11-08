//! `spirolo check`, `audit` and `gate` end to end: what a user (and an agent) gets from the
//! binary when the finding pipeline runs `weak-assertion` over a fixture project.
//!
//! Seam 2 of the PRD's testing decisions: a small fixture with at least one true finding
//! and one near-miss that must not fire, driven through the real binary with `assert_cmd`,
//! its human and JSON views snapshotted with `insta`. The rule's own branches are covered
//! on hand-built models in `spirolo-rules`; here we prove the wiring — config, model, rules,
//! report and exit codes — is whole.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;

#[path = "support/policy.rs"]
mod policy_support;

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
    std::fs::create_dir_all(path.join("src")).unwrap();
    policy_support::info(&path);
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

/// A fixture with two weak tests (`toBeTruthy`/`toBeDefined` and `not.toBeNull`), one
/// strong test (`toBe`), and a fourth test that asserts nothing — the near-misses that must
/// not fire.
fn weak_and_strong(name: &str) -> PathBuf {
    let root = project(name);
    write(
        &root,
        "src/widget.test.ts",
        "import { describe, it, expect } from 'vitest'\n\
         describe('widget', () => {\n\
         \x20 it('exists', () => {\n\
         \x20   expect(widget()).toBeTruthy()\n\
         \x20   expect(widget()).toBeDefined()\n\
         \x20 })\n\
         \x20 it('is not null', () => {\n\
         \x20   expect(widget()).not.toBeNull()\n\
         \x20 })\n\
         \x20 it('has a real value', () => {\n\
         \x20   expect(sum(1, 2)).toBe(3)\n\
         \x20 })\n\
         \x20 it('asserts nothing', () => {})\n\
         })\n",
    );
    root
}

/// A rule a severity override can make gate. A DROPPED rule never gates whatever the config
/// asks, so a test about the override's mechanics must not ride on a rule whose visit may still
/// drop it (weak-assertion's did, #501): `dead-skip`'s visit is closed and it is not DROPPED.
const GATING_RULE: &str = "dead-skip";

/// A project with two unconditional, untracked skips: two `GATING_RULE` findings.
fn untracked_skips(name: &str) -> PathBuf {
    assert_ne!(
        spirolo_rules::status::status(GATING_RULE).verdict,
        spirolo_rules::status::Verdict::Dropped,
        "{GATING_RULE} is DROPPED, so no override can gate it: pick another rule for this test"
    );
    let root = project(name);
    write(
        &root,
        "src/widget.test.ts",
        "import { it, expect } from 'vitest'\n\
         it.skip('exists', () => { expect(sum(1, 2)).toBe(3) })\n\
         it.skip('is not null', () => { expect(sum(1, 2)).toBe(3) })\n\
         it('has a real value', () => { expect(sum(1, 2)).toBe(3) })\n",
    );
    root
}

/// Replace the trailing `· <n>ms` timing of the summary line with `· <t>`, so a human-view
/// snapshot does not depend on the wall clock.
fn redact_timing(stdout: &str) -> String {
    let Some((first, rest)) = stdout.split_once('\n') else {
        return stdout.to_owned();
    };
    let redacted_first = match first.rsplit_once("· ") {
        Some((head, _timing)) => format!("{head}· <t>"),
        None => first.to_owned(),
    };
    format!("{redacted_first}\n{rest}")
}

// -------------------------------------------------------------------- the two weak tests

#[test]
fn check_reports_exactly_the_two_weak_tests_and_names_the_matchers() {
    let root = weak_and_strong("two-weak");

    spirolo()
        .args(["check", "--all"])
        .arg(&root)
        .assert()
        .success()
        .stdout(
            // Both weak tests, by their line, naming the value-less matchers.
            predicate::str::contains(
                ":3 test \"exists\" asserts only value-less matchers (toBeDefined, toBeTruthy)",
            )
            .and(predicate::str::contains(
                ":7 test \"is not null\" asserts only value-less matchers (not.toBeNull)",
            ))
            // The strong test and the assertion-less one never appear.
            .and(predicate::str::contains("has a real value").not())
            .and(predicate::str::contains("asserts nothing").not()),
        );
}

#[test]
fn the_human_view_is_stable() {
    let root = weak_and_strong("human-snapshot");

    let output = spirolo()
        .args(["check", "--all"])
        .arg(&root)
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    // The summary line carries a clock (`· 8ms`); redact the trailing timing so the
    // snapshot is stable. The timing is the last dot-separated field of the first line.
    let redacted = status_support::redact_heading_status(&redact_timing(&stdout));
    insta::assert_snapshot!("check_human_weak_assertion", redacted);
}

// ------------------------------------------------------------------------- the ratio row

#[test]
fn the_ratio_row_reports_above_the_ceiling_and_is_ok_below() {
    let root = weak_and_strong("ratio-above");

    // Default 6 % ceiling: three of four assertions value-less is well above it.
    spirolo()
        .args(["check", "--all"])
        .arg(&root)
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "project 3 of 4 assertions carry no value (75.0%, ceiling 6.0%)",
        ));

    // A ceiling above the measured ratio silences the row; the rule then has only its
    // per-test findings, and the whole run is still `info`, never `ok`.
    write(
        &root,
        ".spirolorc.jsonc",
        "{ \"rules\": { \"weak-assertion\": { \"maxRatio\": 0.9, \"severity\": \"info\" } } }\n",
    );
    spirolo()
        .args(["check", "--all"])
        .arg(&root)
        .assert()
        .success()
        .stdout(
            predicate::str::contains("carry no value")
                .not()
                .and(predicate::str::contains("value-less matchers")),
        );
}

// -------------------------------------------------------------------------- json + agent

#[test]
fn json_adds_findings_and_keeps_the_schema_version() {
    let root = weak_and_strong("json-findings");

    let output = spirolo()
        .args(["check", "--all", "--format", "json"])
        .arg(&root)
        .output()
        .unwrap();
    assert!(output.status.success());

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        json["schema_version"],
        serde_json::json!(spirolo_report::SCHEMA_VERSION),
        "check emits the same schema version as list; adding findings bumps nothing"
    );
    let findings = json["findings"].as_array().unwrap();
    // Two weak tests plus the one project-wide ratio row.
    assert_eq!(findings.len(), 3);
    assert!(findings.iter().all(|f| f["rule"] == "weak-assertion"));
    // The first finding is the model's own JSON with a `findings` key beside the tables.
    assert!(
        json["test_cases"].is_array(),
        "the model's tables are still present"
    );
}

#[test]
fn a_finding_anchored_at_a_test_carries_its_title_path_and_the_ratio_row_none() {
    let root = weak_and_strong("title-path-json");

    let output = spirolo()
        .args(["check", "--all", "--format", "json"])
        .arg(&root)
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let findings = json["findings"].as_array().unwrap();

    // The title path is the suite path then the title, as `spirolo list` names the test case.
    let per_test: Vec<&serde_json::Value> = findings.iter().filter(|f| f["file"] != "").collect();
    assert_eq!(per_test.len(), 2);
    assert_eq!(
        per_test[0]["test"],
        serde_json::json!({ "suite_path": ["widget"], "title": "exists" })
    );
    assert_eq!(
        per_test[1]["test"],
        serde_json::json!({ "suite_path": ["widget"], "title": "is not null" })
    );
    // The project-wide ratio row is anchored at no test: the key is absent, not null.
    let ratio = findings.iter().find(|f| f["file"] == "").unwrap();
    assert!(ratio.get("test").is_none(), "{ratio}");
}

#[test]
fn the_findings_json_is_stable() {
    let root = weak_and_strong("json-snapshot");

    let output = spirolo()
        .args(["check", "--all", "--format", "json"])
        .arg(&root)
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    // The findings array alone: it carries no clock, so it snapshots without redaction.
    insta::assert_json_snapshot!(
        "check_json_findings",
        status_support::redact_status(&json["findings"])
    );
}

#[test]
fn two_runs_produce_byte_equal_json() {
    let root = weak_and_strong("byte-equal");

    let first = spirolo()
        .args(["check", "--all", "--format", "json"])
        .arg(&root)
        .output()
        .unwrap();
    let second = spirolo()
        .args(["check", "--all", "--format", "json"])
        .arg(&root)
        .output()
        .unwrap();
    assert_eq!(
        first.stdout, second.stdout,
        "JSON is byte-identical across runs"
    );
}

#[test]
fn the_agent_view_is_findings_and_explain_only() {
    let root = weak_and_strong("agent-view");

    let status = status_support::rule_status("weak-assertion");
    spirolo()
        .args(["check", "--all", "--agent"])
        .arg(&root)
        .assert()
        .success()
        .stdout(
            // Each finding is a line, its explain the next; no summary line, no `ok` rows.
            predicate::str::contains(format!(
                "weak-assertion\t{status}\tinfo\tsrc/widget.test.ts:3"
            ))
            .and(predicate::str::contains("\texplain\tThe test \"exists\""))
            .and(predicate::str::contains("Do not delete the assertions"))
            .and(predicate::str::contains("files ·").not())
            .and(predicate::str::contains(" ok   ").not()),
        );
}

// --------------------------------------------------------------------------------- sarif

#[test]
fn sarif_emits_a_valid_2_1_document_with_the_findings_as_results() {
    let root = weak_and_strong("sarif-findings");

    let output = spirolo()
        .args(["check", "--all", "--format", "sarif"])
        .arg(&root)
        .output()
        .unwrap();
    assert!(output.status.success());

    let doc: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(doc["version"], "2.1.0");
    assert_eq!(doc["runs"][0]["tool"]["driver"]["name"], "spirolo");

    // Two weak tests plus the project-wide ratio row: three results, all weak-assertion.
    let results = doc["runs"][0]["results"].as_array().unwrap();
    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|r| r["ruleId"] == "weak-assertion"));
    // Info maps to `note`.
    assert!(results.iter().all(|r| r["level"] == "note"));

    // One rules entry, its first finding's explain as help text. The first finding is the
    // project row (no file sorts first), whose explain points at value-carrying matchers.
    let rules = doc["runs"][0]["tool"]["driver"]["rules"]
        .as_array()
        .unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0]["id"], "weak-assertion");
    assert!(rules[0]["help"]["text"]
        .as_str()
        .unwrap()
        .contains("assert the produced value"));

    // No fingerprints of our own: the upload action computes them.
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(!text.contains("Fingerprints"), "no fingerprints: {text}");
}

#[test]
fn sarif_carries_the_title_path_in_the_result_properties() {
    let root = weak_and_strong("title-path-sarif");

    let output = spirolo()
        .args(["check", "--all", "--format", "sarif"])
        .arg(&root)
        .output()
        .unwrap();
    let doc: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let results = doc["runs"][0]["results"].as_array().unwrap();

    // The same object the JSON surface gives, under `properties.test`.
    let tests: Vec<&serde_json::Value> = results
        .iter()
        .filter_map(|r| r["properties"].get("test"))
        .collect();
    assert_eq!(
        tests,
        [
            &serde_json::json!({ "suite_path": ["widget"], "title": "exists" }),
            &serde_json::json!({ "suite_path": ["widget"], "title": "is not null" }),
        ]
    );
    // The file-less ratio row carries no title path, only its rule status.
    let ratio = results
        .iter()
        .find(|r| r["locations"].as_array().unwrap().is_empty())
        .unwrap();
    assert_eq!(
        ratio["properties"],
        serde_json::json!({ "ruleStatus": status_support::rule_status("weak-assertion") }),
        "{ratio}"
    );
}

#[test]
fn two_runs_produce_byte_equal_sarif() {
    let root = weak_and_strong("sarif-byte-equal");

    let first = spirolo()
        .args(["check", "--all", "--format", "sarif"])
        .arg(&root)
        .output()
        .unwrap();
    let second = spirolo()
        .args(["check", "--all", "--format", "sarif"])
        .arg(&root)
        .output()
        .unwrap();
    assert_eq!(
        first.stdout, second.stdout,
        "SARIF is byte-identical across runs"
    );
}

#[test]
fn the_sarif_view_is_stable() {
    let root = weak_and_strong("sarif-snapshot");

    let output = spirolo()
        .args(["check", "--all", "--format", "sarif"])
        .arg(&root)
        .output()
        .unwrap();
    let doc: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    // The version string is the crate's; redact it so the snapshot does not churn on a bump.
    let mut doc = doc;
    doc["runs"][0]["tool"]["driver"]["version"] = serde_json::json!("<version>");
    // The result's primary location carries the test file's real path under CARGO_TARGET_TMPDIR
    // only in `uri`, which is repo-relative here, so no redaction is needed beyond version.
    insta::assert_json_snapshot!("check_sarif", status_support::redact_status(&doc));
}

#[test]
fn the_human_view_prints_ok_rows_for_silent_rules() {
    let root = project("all-strong");
    write(
        &root,
        "src/ok.test.ts",
        "import { it, expect } from 'vitest'\n\
         it('has a value', () => { expect(sum(1, 2)).toBe(3) })\n",
    );

    spirolo().args(["check", "--all"]).arg(&root).assert().success().stdout(
        predicate::str::contains("✓ 16 rules found nothing: handler-drift, generator-walk, absence-anchor, weak-assertion").and(predicate::str::contains(
            "0 findings in 0 of 16 rules · 0 warn · 0 error · exit 0",
        )),
    );
}

// ------------------------------------------------------------------- rule and family select

#[test]
fn rule_selects_the_named_rule_and_family_commands_select_their_family() {
    let root = weak_and_strong("select");

    // `--rule weak-assertion` runs it.
    spirolo()
        .args(["check", "--all", "--rule", "weak-assertion"])
        .arg(&root)
        .assert()
        .success()
        .stdout(predicate::str::contains("value-less matchers"));

    // `audit` runs the audit family, which weak-assertion is in.
    spirolo()
        .args(["audit", "--all"])
        .arg(&root)
        .assert()
        .success()
        .stdout(predicate::str::contains("value-less matchers"));

    // `gate` runs the gate family (fail-closed-setup, gate-truth and dead-skip); this fixture
    // has no skips and no runner config, so no findings and a clean exit — but weak-assertion,
    // an audit rule, never runs under `gate`.
    spirolo()
        .args(["gate", "--all"])
        .arg(&root)
        .assert()
        .success()
        .stdout(predicate::str::contains("value-less matchers").not().and(
            predicate::str::contains(
                "✓ 3 rules found nothing: fail-closed-setup, gate-truth, dead-skip",
            ),
        ));
}

#[test]
fn an_unknown_rule_name_exits_two() {
    let root = weak_and_strong("unknown-rule");

    spirolo()
        .args(["check", "--all", "--rule", "no-such-rule"])
        .arg(&root)
        .assert()
        .code(2)
        .stderr(
            predicate::str::contains("no such rule").and(predicate::str::contains("no-such-rule")),
        );
}

// ------------------------------------------------------------------------------ config path

#[test]
fn config_outside_the_root_is_honoured() {
    let root = untracked_skips("config-external");
    // The config lives beside, not inside, the project — a pinned corpus clone is checked
    // without writing into it.
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("config-external-side");
    std::fs::create_dir_all(&dir).unwrap();
    let config = dir.join("external.jsonc");
    std::fs::write(
        &config,
        format!("{{ \"rules\": {{ \"{GATING_RULE}\": \"error\" }} }}\n"),
    )
    .unwrap();

    // The severity override reaches the rule: an error-severity finding fails the build.
    spirolo()
        .args(["check", "--all", "--config"])
        .arg(&config)
        .arg(&root)
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "skip \"exists\" has no linked issue",
        ));
}

#[test]
fn a_config_path_that_does_not_exist_is_an_error() {
    let root = weak_and_strong("config-missing");

    spirolo()
        .args(["check", "--all", "--config", "no-such-config.jsonc"])
        .arg(&root)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("does not exist"));
}

#[test]
fn a_malformed_config_is_a_diagnostic_not_an_abort() {
    let root = weak_and_strong("config-malformed");
    write(&root, ".spirolorc.jsonc", "{ \"rules\": { not valid\n");

    // The run still produces its findings (a diagnostic, not an abort) but exits 2 because
    // the config could not be read.
    spirolo()
        .args(["check", "--all"])
        .arg(&root)
        .assert()
        .code(2)
        .stdout(predicate::str::contains("value-less matchers"));
}

// ------------------------------------------------------------------------------- exit codes

#[test]
fn exit_zero_when_there_is_nothing_to_report() {
    let root = project("clean");
    write(
        &root,
        "src/ok.test.ts",
        "import { it, expect } from 'vitest'\n\
         it('has a value', () => { expect(sum(1, 2)).toBe(3) })\n",
    );

    spirolo()
        .args(["check", "--all"])
        .arg(&root)
        .assert()
        .success();
}

#[test]
fn an_info_finding_exits_zero_and_an_error_finding_exits_one() {
    let root = untracked_skips("exit-severity");

    // The fixture explicitly opts into info: findings, but a clean exit.
    spirolo()
        .args(["check", "--all"])
        .arg(&root)
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "skip \"exists\" has no linked issue",
        ));

    // Overridden to `error`: the same finding now fails the build.
    write(
        &root,
        ".spirolorc.jsonc",
        &format!("{{ \"rules\": {{ \"{GATING_RULE}\": \"error\" }} }}\n"),
    );
    spirolo()
        .args(["check", "--all"])
        .arg(&root)
        .assert()
        .code(1);
}
