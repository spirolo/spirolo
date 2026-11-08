//! `spirolo check` end to end on a `dead-skip` fixture project: what a maintainer and an
//! agent get from the binary when the rule reads a suite's skips.
//!
//! Seam 2 of the PRD's testing decisions: a small fixture with an unconditional skip with no
//! issue (a finding), the same skip carrying `#42` (silent), a CI-scoped `skipIf` (a finding
//! naming the variable) and a `test.only` (error), driven through the real binary with
//! `assert_cmd`, its JSON view snapshotted with `insta`. The rule's own branches are covered
//! on hand-built models in `spirolo-rules`; here we prove the wiring — model, rule, report,
//! exit codes — is whole.

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
    std::fs::create_dir_all(&path).unwrap();
    policy_support::info(&path);
    path
}

#[test]
fn an_info_override_also_demotes_the_rules_only_finding() {
    let root = skips("info-override");
    write(
        &root,
        ".spirolorc.jsonc",
        r#"{"rules":{"dead-skip":{"severity":"info"}}}"#,
    );
    let output = spirolo()
        .args(["check", "--all", "--rule", "dead-skip", "--format", "json"])
        .arg(&root)
        .output()
        .unwrap();
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(document["findings"]
        .as_array()
        .unwrap()
        .iter()
        .all(|f| f["severity"] == "info"));
    assert_eq!(output.status.code(), Some(0));
}

/// Write `contents` to `path` under `root`, creating parents.
fn write(root: &Path, path: &str, contents: &str) {
    let full = root.join(path);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(full, contents).unwrap();
}

/// A fixture whose one spec carries the four cases the acceptance criteria name: an
/// unconditional `it.skip` with no issue (a finding), a second `it.skip` whose title holds
/// `#42` (silent), a `describe.skipIf(!!process.env.CI)` (a CI-scoped finding), and a
/// `test.only` (error).
fn skips(name: &str) -> PathBuf {
    let root = project(name);
    write(
        &root,
        "src/widget.test.ts",
        "import { describe, it, test, expect } from 'vitest'\n\
         describe('widget', () => {\n\
         \x20 it.skip('is dead with no ticket', () => {\n\
         \x20   expect(widget()).toBe(1)\n\
         \x20 })\n\
         \x20 it.skip('is tracked in #42', () => {\n\
         \x20   expect(widget()).toBe(2)\n\
         \x20 })\n\
         \x20 test.only('runs alone', () => {\n\
         \x20   expect(widget()).toBe(3)\n\
         \x20 })\n\
         })\n\
         describe.skipIf(!!process.env.CI)('integration', () => {\n\
         \x20 it('hits the backend', () => {\n\
         \x20   expect(widget()).toBe(4)\n\
         \x20 })\n\
         })\n",
    );
    root
}

// ---------------------------------------------------------------- the four cases, human view

#[test]
fn check_reports_the_dead_skip_the_ci_scope_and_the_only_and_leaves_the_tracked_skip_alone() {
    let root = skips("cases");

    spirolo()
        .args(["check", "--all"])
        .arg(&root)
        .assert()
        .code(0)
        .stdout(
            // The unconditional skip with no ticket is a finding.
            predicate::str::contains("dead-skip")
                .and(predicate::str::contains("no linked issue"))
                // The CI-scoped skipIf is reported, naming the variable.
                .and(predicate::str::contains("CI-scoped"))
                .and(predicate::str::contains("CI"))
                // The `only` is reported at the fixture's explicit info severity.
                .and(predicate::str::contains("skips every other case"))
                // The `#42`-tracked skip is listed, never flagged.
                .and(predicate::str::contains(
                    "\"is tracked in #42\": unconditional · listed",
                ))
                .and(predicate::str::contains("\"is tracked in #42\" has no linked issue").not()),
        );
}

#[test]
fn the_rule_heading_counts_files_and_breaks_the_outcome_mix_down() {
    // The info override applies to all three findings; listing rows remain separate.
    let root = skips("heading");
    let output = spirolo()
        .args(["check", "--all"])
        .arg(&root)
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    // The tier label after the breakdown follows the shipped status; its forms are tested
    // under a supplied table in the binary's display tests.
    assert!(
        stdout.contains("● dead-skip (1 file, 6 findings) · 3 info · 3 listed"),
        "{stdout}"
    );
    assert!(
        stdout.contains("· 3 listed · exit 0\n"),
        "the closing line counts the listing rows apart: {stdout}"
    );
    assert!(
        stdout.contains(": unconditional · listed\n"),
        "a listing row is marked as such: {stdout}"
    );
}

// -------------------------------------------------------------------------- the findings json

#[test]
fn the_findings_json_names_the_kinds_severities_and_the_ci_variable() {
    let root = skips("cases-json");

    let output = spirolo()
        .args(["check", "--all", "--format", "json", "--rule", "dead-skip"])
        .arg(&root)
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let all = json["findings"].as_array().unwrap();
    assert!(all.iter().all(|f| f["rule"] == "dead-skip"));

    // Every skip is listed: the two `it.skip`s and the `skipIf` (the `only` is not a skip).
    let listed: Vec<&serde_json::Value> = all.iter().filter(|f| f["outcome"] == "listed").collect();
    assert_eq!(listed.len(), 3, "{all:#?}");
    assert!(listed.iter().all(|f| f["severity"] == "info"));
    let tracked = listed
        .iter()
        .find(|f| f["message"].as_str().unwrap().contains("is tracked in #42"))
        .expect("the tracked skip is listed even though it is not flagged");
    let tracked_evidence = tracked["evidence"].as_array().unwrap();
    assert!(
        tracked_evidence
            .iter()
            .any(|e| e["key"] == "tracked" && e["value"] == "yes"),
        "{tracked:#?}"
    );

    // Three findings: the untracked skip, the CI-scoped skipIf, the only. The `#42` skip is
    // absent.
    let findings: Vec<&serde_json::Value> =
        all.iter().filter(|f| f["outcome"] == "finding").collect();
    assert_eq!(findings.len(), 3, "{findings:#?}");

    let only = findings
        .iter()
        .find(|f| {
            f["message"]
                .as_str()
                .unwrap()
                .contains("skips every other case")
        })
        .expect("the only finding");
    assert_eq!(only["severity"], "info");

    let ci = findings
        .iter()
        .find(|f| f["message"].as_str().unwrap().contains("CI-scoped"))
        .expect("the CI-scoped finding");
    assert_eq!(ci["severity"], "info");
    let variable = ci["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["key"] == "variable")
        .expect("a variable evidence entry");
    assert_eq!(variable["value"], "CI");

    let dead = findings
        .iter()
        .find(|f| f["message"].as_str().unwrap().contains("no linked issue"))
        .expect("the untracked skip finding");
    assert_eq!(dead["severity"], "info");
    // The explain names the kind and the missing reference, and never says to just delete.
    assert!(dead["explain"].as_str().unwrap().contains("skip"));
    assert!(dead["explain"].as_str().unwrap().contains("#123"));
}

#[test]
fn every_row_carries_the_title_path_of_the_case_it_is_anchored_at() {
    let root = skips("ds-title-path");

    let output = spirolo()
        .args(["check", "--all", "--format", "json", "--rule", "dead-skip"])
        .arg(&root)
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let findings = json["findings"].as_array().unwrap();

    // Every row, the listed one and the suite-level `describe.skipIf` included, names the
    // case row at its location by its suite path and title, as the model names it.
    for finding in findings {
        let case = json["test_cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["file"] == finding["file"] && c["span"]["line"] == finding["line"])
            .unwrap_or_else(|| panic!("no test case at {finding}"));
        assert_eq!(
            finding["test"],
            serde_json::json!({ "suite_path": case["suite_path"], "title": case["title"] }),
        );
    }
    assert!(findings.len() >= 3, "{findings:#?}");
}

#[test]
fn the_findings_json_is_stable() {
    let root = skips("cases-snapshot");

    let output = spirolo()
        .args(["check", "--all", "--format", "json", "--rule", "dead-skip"])
        .arg(&root)
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    insta::assert_json_snapshot!(
        "check_json_dead_skip",
        status_support::redact_status(&json["findings"])
    );
}

// ----------------------------------------------------------------------- exit code + family

#[test]
fn an_only_finding_at_info_does_not_fail_the_build() {
    // The explicit info override also silences the intrinsic `only` error.
    let root = skips("exit");
    spirolo()
        .args(["check", "--all"])
        .arg(&root)
        .assert()
        .code(0);
}

#[test]
fn dead_skip_runs_under_the_gate_family() {
    let root = skips("gate-family");
    // `spirolo gate` runs the gate family, which dead-skip is in.
    spirolo()
        .args(["gate", "--all"])
        .arg(&root)
        .assert()
        .code(0)
        .stdout(predicate::str::contains("dead-skip"));
    // `spirolo audit` runs the audit family, which dead-skip is not in.
    spirolo()
        .args(["audit", "--all"])
        .arg(&root)
        .assert()
        .success()
        .stdout(predicate::str::contains("dead-skip").not());
}

/// What `dead-skip` says about the one spec `source`, written to `src/real.test.tsx` in a
/// fresh project: the messages of its flagged rows, and the `tracked` evidence of each listing
/// row.
fn judged(name: &str, source: &str) -> (Vec<String>, Vec<String>) {
    let root = project(&format!("dead-skip-{name}"));
    write(&root, "src/real.test.tsx", source);
    let output = spirolo()
        .args(["check", "--all", "--format", "json", "--rule", "dead-skip"])
        .arg(&root)
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let all = json["findings"].as_array().unwrap();
    let flagged = all
        .iter()
        .filter(|f| f["outcome"] != "listed")
        .map(|f| f["message"].as_str().unwrap().to_owned())
        .collect();
    let tracked = all
        .iter()
        .filter(|f| f["outcome"] == "listed")
        .flat_map(|f| f["evidence"].as_array().unwrap().iter())
        .filter(|e| e["key"] == "tracked")
        .map(|e| e["value"].as_str().unwrap().to_owned())
        .collect();
    (flagged, tracked)
}

#[test]
fn an_issue_url_in_the_comment_line_above_the_skip_tracks_it() {
    // marmelab__react-admin packages/ra-ui-materialui/src/input/FileInput.spec.tsx:100-101.
    let (flagged, tracked) = judged(
        "comment-above",
        "describe('<FileInput />', () => {\n\
         \x20   // Skipped until https://github.com/jsdom/jsdom/issues/1568 is fixed\n\
         \x20   it.skip('should correctly update upon drop when allowing multiple files', async () => {\n\
         \x20       const onSubmit = jest.fn();\n\
         \x20   });\n\
         });\n",
    );
    assert_eq!(flagged, Vec::<String>::new());
    assert_eq!(tracked, ["yes"]);
}

#[test]
fn a_comment_above_a_skip_nested_two_suites_deep_tracks_it() {
    // marmelab__react-admin packages/ra-ui-materialui/src/input/ImageInput.spec.tsx:424-425.
    let (flagged, tracked) = judged(
        "comment-above-nested",
        "describe('<ImageInput />', () => {\n\
         \x20   describe('Image Preview', () => {\n\
         \x20       // Skipped until https://github.com/jsdom/jsdom/issues/1568 is fixed\n\
         \x20       it.skip('should update previews when dropping a file', async () => {\n\
         \x20           const onSubmit = jest.fn();\n\
         \x20       });\n\
         \x20   });\n\
         });\n",
    );
    assert_eq!(flagged, Vec::<String>::new());
    assert_eq!(tracked, ["yes"]);
}

#[test]
fn a_reference_in_the_second_comment_line_above_a_describe_skip_tracks_it() {
    // gatsbyjs__gatsby integration-tests/structured-logging/__tests__/to-do.js:279-281.
    let (flagged, tracked) = judged(
        "comment-above-describe",
        "describe(`structured-logging`, () => {\n\
         \x20   // Skipping for now because we don't handle these correctly\n\
         \x20   // Documented in https://github.com/gatsbyjs/gatsby/issues/18383\n\
         \x20   describe.skip(`unhandledRejection`, () => {\n\
         \x20     let events = []\n\
         \x20   })\n\
         })\n",
    );
    assert_eq!(flagged, Vec::<String>::new());
    assert_eq!(tracked, ["yes"]);
}

#[test]
fn an_issue_url_in_a_trailing_comment_on_the_skip_tracks_it() {
    let (flagged, tracked) = judged(
        "trailing-url",
        "it.skip('t', () => {}) // https://github.com/o/r/issues/12\n",
    );
    assert_eq!(flagged, Vec::<String>::new());
    assert_eq!(tracked, ["yes"]);
}

#[test]
fn a_hash_number_in_a_trailing_comment_on_a_todo_tracks_it() {
    let (flagged, tracked) = judged("trailing-hash", "it.todo('t') // #12\n");
    assert_eq!(flagged, Vec::<String>::new());
    assert_eq!(tracked, ["yes"]);
}

#[test]
fn a_pull_request_url_in_the_comment_lines_above_tracks_the_skip() {
    // mobxjs__mobx packages/mobx/__tests__/base/typescript-tests.ts:1731-1737.
    let (flagged, tracked) = judged(
        "pull-url-above",
        "// 19.12.2020 @urugator:\n\
         // All annotated non-observable fields are not writable.\n\
         // All annotated fields of non-plain objects are non-configurable.\n\
         // https://github.com/mobxjs/mobx/pull/2641\n\
         test.skip(\"actions are reassignable\", () => {\n\
         \x20   // See #1398 and #1545, make actions reassignable to support stubbing\n\
         \x20   class A {}\n\
         })\n",
    );
    assert_eq!(flagged, Vec::<String>::new());
    assert_eq!(tracked, ["yes"]);
}

#[test]
fn a_pull_request_url_under_a_todo_comment_above_a_describe_skip_tracks_it() {
    // radix-ui__primitives packages/react/slot/src/slot.test.tsx:194-196.
    let (flagged, tracked) = judged(
        "pull-url-under-todo",
        "// TODO: Unskip when underlying issue is resolved\n\
         // Reverted in https://github.com/radix-ui/primitives/pull/3554\n\
         describe.skip('given an Input', () => {\n\
         \x20 const handleRef = vi.fn();\n\
         });\n",
    );
    assert_eq!(flagged, Vec::<String>::new());
    assert_eq!(tracked, ["yes"]);
}

#[test]
fn a_pull_request_url_in_the_title_tracks_the_skip() {
    let (flagged, tracked) = judged(
        "pull-url-title",
        "it.skip('waits on https://github.com/o/r/pull/7', () => {})\n",
    );
    assert_eq!(flagged, Vec::<String>::new());
    assert_eq!(tracked, ["yes"]);
}

#[test]
fn a_comment_above_with_no_reference_leaves_the_skip_flagged() {
    // expo__expo packages/expo-router/src/native-tabs/__tests__/render.test.ios.tsx:710-712,
    // gatsbyjs__gatsby packages/gatsby/src/schema/__tests__/build-schema.js:602-603 and
    // radix-ui__primitives packages/react/select/src/select.test.tsx:1558-1559.
    let (flagged, tracked) = judged(
        "untracked-comments-above",
        "describe('native tabs', () => {\n\
         \x20   // TODO(@ubax): Investigate why this test fails in the test environment\n\
         \x20   // When testing in the actual app, it works as expected.\n\
         \x20   it.skip('Stack navigation state is reset when parent tabs remount', () => {\n\
         \x20     const onMount = jest.fn();\n\
         \x20   });\n\
         \x20   // FIXME: Fails for PluginDefined.qux with PluginDefinedNested\n\
         \x20   it.skip(`merges user-defined type (graphql-js) with plugin-defined type`, async () => {\n\
         \x20     createTypes()\n\
         \x20   })\n\
         \x20 // TODO: Fix this\n\
         \x20 it.todo(\"applies the consumer's `className` and `style`\");\n\
         });\n",
    );
    assert_eq!(flagged.len(), 3, "{flagged:#?}");
    assert_eq!(tracked, ["no", "no", "no"]);
}

#[test]
fn a_reference_separated_from_the_skip_by_a_blank_or_code_line_leaves_it_flagged() {
    let (flagged, tracked) = judged(
        "separated-reference",
        "// https://github.com/o/r/issues/12\n\
         \n\
         it.skip('after a blank line', () => {})\n\
         // https://github.com/o/r/issues/13\n\
         const fixture = 1\n\
         it.skip('after a code line', () => {})\n",
    );
    assert_eq!(flagged.len(), 2, "{flagged:#?}");
    assert_eq!(tracked, ["no", "no"]);
}

#[test]
fn a_reference_only_inside_the_test_body_leaves_the_skip_flagged() {
    // mobxjs__mobx packages/mobx/__tests__/base/typescript-tests.ts:1736-1737.
    let (flagged, tracked) = judged(
        "body-reference",
        "test.skip(\"actions are reassignable\", () => {\n\
         \x20   // See #1398 and #1545, make actions reassignable to support stubbing\n\
         \x20   class A {}\n\
         })\n",
    );
    assert_eq!(flagged.len(), 1, "{flagged:#?}");
    assert_eq!(tracked, ["no"]);
}
