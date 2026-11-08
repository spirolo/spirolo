//! Regressions from the frozen gate-truth visit rows, through the public CLI.

use std::path::PathBuf;

use assert_cmd::Command;
use serde_json::Value;

fn findings(fixture: &str) -> Vec<Value> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/gate-truth-reliability")
        .join(fixture);
    let output = Command::cargo_bin("spirolo")
        .unwrap()
        .args(["check", "--all", "--format", "json", "--rule", "gate-truth"])
        .arg(root)
        .output()
        .unwrap();
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    json["findings"].as_array().unwrap().clone()
}

fn has_kind(found: &[Value], kind: &str) -> bool {
    found.iter().any(|f| {
        f["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["key"] == "kind" && e["value"] == kind)
    })
}

#[test]
fn react_native_web_runs_jest_through_the_unit_script_glob() {
    let found = findings("react-native-web");
    assert!(!has_kind(&found, "ci-runs-nothing"), "{found:#?}");
}

#[test]
fn lint_and_typecheck_scripts_do_not_count_as_running_the_suite() {
    let found = findings("react-native-web-lint");
    assert!(has_kind(&found, "ci-runs-nothing"), "{found:#?}");
}

#[test]
fn gatsby_runs_the_suite_in_circleci_beside_a_stale_bot_workflow() {
    let found = findings("gatsby-circleci");
    assert!(!has_kind(&found, "ci-runs-nothing"), "{found:#?}");
}

#[test]
fn nivo_runs_jest_through_a_make_target() {
    let found = findings("nivo-make");
    assert!(!has_kind(&found, "ci-runs-nothing"), "{found:#?}");
}

#[test]
fn nivo_runs_cypress_through_a_test_action() {
    let found = findings("nivo-cypress");
    assert!(!has_kind(&found, "ci-runs-nothing"), "{found:#?}");
}

#[test]
fn installing_cypress_without_running_it_is_still_a_missing_ci_gate() {
    let found = findings("nivo-cypress-install");
    assert!(has_kind(&found, "ci-runs-nothing"), "{found:#?}");
}

#[test]
fn carbon_ci_retries_only_the_root_playwright_config() {
    let found = findings("carbon-playwright");
    let retries: Vec<_> = found
        .iter()
        .filter(|f| {
            f["evidence"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["key"] == "kind" && e["value"] == "retries")
        })
        .map(|f| f["file"].as_str().unwrap())
        .collect();
    assert_eq!(retries, ["playwright.config.js"], "{found:#?}");
}

#[test]
fn instantsearch_keeps_retries_reached_through_a_workspace_script_and_explicit_config() {
    let found = findings("instantsearch-playwright");
    assert!(
        found.iter().any(|f| {
            f["file"] == "tests/e2e/playwright/playwright.config.ts"
                && f["evidence"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|e| e["key"] == "kind" && e["value"] == "retries")
        }),
        "{found:#?}"
    );
}

#[test]
fn react_playground_keeps_both_ci_findings_under_the_default_working_directory() {
    let found = findings("react-playground");
    assert!(has_kind(&found, "retries"), "{found:#?}");
    assert!(has_kind(&found, "forbid-only"), "{found:#?}");
}

#[test]
fn kaneo_keeps_retries_when_the_script_changes_directory_with_pnpm() {
    let found = findings("kaneo-playwright");
    assert!(has_kind(&found, "retries"), "{found:#?}");
}

#[test]
fn react_devtools_keeps_the_focused_test_guard_finding_through_its_node_wrapper() {
    let found = findings("react-devtools");
    assert!(has_kind(&found, "forbid-only"), "{found:#?}");
}

#[test]
fn tududi_does_not_claim_a_local_only_browser_suite_passes_ci_with_focused_tests() {
    let found = findings("tududi-local-playwright");
    assert!(!has_kind(&found, "forbid-only"), "{found:#?}");
}

#[test]
fn radix_keeps_retries_because_the_pinned_ci_script_actually_runs_e2e() {
    let found = findings("radix-playwright");
    assert!(has_kind(&found, "retries"), "{found:#?}");
}

#[test]
fn extension_keeps_retries_reached_through_xvfb_timeout_and_a_node_wrapper() {
    let found = findings("extension-playwright");
    assert!(has_kind(&found, "retries"), "{found:#?}");
}
