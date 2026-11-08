//! `spirolo check` end to end on a `data-layer-mock` fixture project: what a maintainer and an
//! agent get from the binary when the rule flags a test that module-mocks the data layer.
//!
//! Seam 2 of the PRD's testing decisions: a small fixture with an alias mock of the data
//! layer in a scoped integration spec (a finding), the same mock in an unscoped unit test
//! (silent), a relative specifier resolving to the same module (a finding), and a mock of an
//! unlisted module (silent), driven through the real binary with `assert_cmd`, its JSON view
//! snapshotted with `insta`. The rule's own branches are covered on hand-built models in
//! `spirolo-rules`; here we prove the wiring — config (`dataLayer`, `dataLayerScope`), the
//! resolver filling `Mock.resolved`, the rule and the report — is whole.

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

/// Write `contents` to `path` under `root`, creating parents.
fn write(root: &Path, path: &str, contents: &str) {
    let full = root.join(path);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(full, contents).unwrap();
}

/// A fixture whose data layer is `src/supabaseClient.ts`, reached through the `@/*` alias.
/// It carries the four cases the acceptance criteria name:
///
/// - `src/checkout.integration.test.ts`: `vi.mock('@/supabaseClient')` in a scoped file →
///   finding, the alias resolving to `src/supabaseClient.ts`.
/// - `src/features/cart/cart.integration.test.ts`: a relative `../../supabaseClient` resolving
///   to the same module in a scoped file → finding.
/// - `src/supabaseClient.test.ts`: the same alias mock in an unscoped unit test → nothing.
/// - `src/checkout.integration.test.ts` also mocks `@/logger`, an unlisted module → nothing.
///
/// The `.spirolorc.jsonc` names the data layer and leaves the scope at its default, so the
/// `*.integration.*` files are covered and the plain `*.test.ts` is not.
fn data_layer(name: &str) -> PathBuf {
    let root = project(name);
    write(
        &root,
        "tsconfig.json",
        "{ \"compilerOptions\": { \"paths\": { \"@/*\": [\"./src/*\"] } } }\n",
    );
    write(
        &root,
        "src/supabaseClient.ts",
        "export const supabase = { from: () => ({}) }\n",
    );
    write(
        &root,
        "src/logger.ts",
        "export const log = (m: string) => m\n",
    );
    // A scoped integration spec: an alias mock of the data layer (a finding) and a mock of an
    // unlisted module (silent).
    write(
        &root,
        "src/checkout.integration.test.ts",
        "import { describe, it, expect, vi } from 'vitest'\n\
         vi.mock('@/supabaseClient')\n\
         vi.mock('@/logger')\n\
         describe('checkout', () => {\n\
         \x20 it('loads the cart', () => {\n\
         \x20   expect(true).toBe(true)\n\
         \x20 })\n\
         })\n",
    );
    // A scoped integration spec reaching the same module through a relative specifier.
    write(
        &root,
        "src/features/cart/cart.integration.test.ts",
        "import { describe, it, expect, vi } from 'vitest'\n\
         vi.mock('../../supabaseClient')\n\
         describe('cart', () => {\n\
         \x20 it('reads the row', () => {\n\
         \x20   expect(true).toBe(true)\n\
         \x20 })\n\
         })\n",
    );
    // An unscoped unit test: the identical alias mock, which must stay silent.
    write(
        &root,
        "src/supabaseClient.test.ts",
        "import { describe, it, expect, vi } from 'vitest'\n\
         vi.mock('@/supabaseClient')\n\
         describe('client', () => {\n\
         \x20 it('stubs the client', () => {\n\
         \x20   expect(true).toBe(true)\n\
         \x20 })\n\
         })\n",
    );
    write(
        &root,
        ".spirolorc.jsonc",
        "{\n  \"dataLayer\": [\"src/supabaseClient\"],\n  \"rules\": {\"data-layer-mock\": \"info\"}\n}\n",
    );
    root
}

// -------------------------------------------------------------------------- the human view

#[test]
fn check_flags_the_scoped_data_layer_mocks_and_leaves_the_unit_test_alone() {
    let root = data_layer("dlm-cases");

    // Scoped to the data-layer-mock rule: `@/logger` is an unlisted mock this rule ignores.
    // (The whole-suite `check` also runs `unused-mock`, which legitimately flags `@/logger` as
    // unreachable — a different rule's finding, so the scope keeps this assertion about this one.)
    spirolo()
        .args(["check", "--all", "--rule", "data-layer-mock"])
        .arg(&root)
        .assert()
        .success()
        .stdout(
            predicate::str::contains("data-layer-mock")
                // The resolved data-layer module is named.
                .and(predicate::str::contains("supabaseClient"))
                // The unlisted `@/logger` mock never surfaces.
                .and(predicate::str::contains("logger").not()),
        );
}

// -------------------------------------------------------------------------- the findings json

#[test]
fn the_findings_json_reports_the_resolved_target_and_the_specifier_as_written() {
    let root = data_layer("dlm-cases-json");

    let output = spirolo()
        .args([
            "check",
            "--all",
            "--format",
            "json",
            "--rule",
            "data-layer-mock",
        ])
        .arg(&root)
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let findings = json["findings"].as_array().unwrap();

    // Exactly two findings: the alias mock and the relative mock, both in scoped files. The
    // unscoped unit test and the `@/logger` mock are absent.
    assert_eq!(findings.len(), 2, "{findings:#?}");
    assert!(findings.iter().all(|f| f["rule"] == "data-layer-mock"));
    assert!(findings.iter().all(|f| f["severity"] == "info"));

    // The alias mock, in the integration spec: its specifier is `@/supabaseClient`, its
    // resolved target `src/supabaseClient.ts`, pointed at by a "data layer" related location.
    let alias = findings
        .iter()
        .find(|f| f["file"] == "src/checkout.integration.test.ts")
        .expect("the alias mock finding");
    let specifier = alias["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["key"] == "specifier")
        .expect("a specifier evidence entry");
    assert_eq!(specifier["value"], "@/supabaseClient");
    let target = alias["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["key"] == "target")
        .expect("a target evidence entry");
    assert_eq!(target["value"], "src/supabaseClient.ts");
    assert!(
        alias["related"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| { r["label"] == "data layer" && r["file"] == "src/supabaseClient.ts" }),
        "the finding points at the resolved data-layer module: {alias:#?}"
    );

    // The relative mock resolves to the same module.
    let relative = findings
        .iter()
        .find(|f| f["file"] == "src/features/cart/cart.integration.test.ts")
        .expect("the relative mock finding");
    let relative_specifier = relative["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["key"] == "specifier")
        .expect("a specifier evidence entry");
    assert_eq!(relative_specifier["value"], "../../supabaseClient");
    let relative_target = relative["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["key"] == "target")
        .expect("a target evidence entry");
    assert_eq!(relative_target["value"], "src/supabaseClient.ts");
}

#[test]
fn a_mock_inside_a_test_carries_its_title_path_and_a_hoisted_mock_none() {
    let root = data_layer("dlm-title-path");
    // A `vi.doMock` written in a test's body is anchored at that test.
    write(
        &root,
        "src/refund.integration.test.ts",
        "import { describe, it, expect, vi } from 'vitest'\n\
         describe('refund', () => {\n\
         \x20 it('reads the ledger', async () => {\n\
         \x20   vi.doMock('@/supabaseClient')\n\
         \x20   expect(1).toBe(1)\n\
         \x20 })\n\
         })\n",
    );

    let output = spirolo()
        .args([
            "check",
            "--all",
            "--format",
            "json",
            "--rule",
            "data-layer-mock",
        ])
        .arg(&root)
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let findings = json["findings"].as_array().unwrap();

    let in_body = findings
        .iter()
        .find(|f| f["file"] == "src/refund.integration.test.ts")
        .unwrap_or_else(|| panic!("{findings:#?}"));
    assert_eq!(
        in_body["test"],
        serde_json::json!({ "suite_path": ["refund"], "title": "reads the ledger" })
    );
    // A module mock hoisted to file scope reaches every test in the file, so it is anchored
    // at the file: no title path.
    let hoisted: Vec<_> = findings
        .iter()
        .filter(|f| f["file"] != "src/refund.integration.test.ts")
        .collect();
    assert_eq!(hoisted.len(), 2, "{findings:#?}");
    assert!(
        hoisted.iter().all(|f| f.get("test").is_none()),
        "{hoisted:#?}"
    );
}

#[test]
fn the_findings_json_is_stable() {
    let root = data_layer("dlm-cases-snapshot");

    let output = spirolo()
        .args([
            "check",
            "--all",
            "--format",
            "json",
            "--rule",
            "data-layer-mock",
        ])
        .arg(&root)
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    insta::assert_json_snapshot!(
        "check_json_data_layer_mock",
        status_support::redact_status(&json["findings"])
    );
}

// ----------------------------------------------------------------------- scope + family

#[test]
fn an_empty_data_layer_reports_nothing_through_the_binary() {
    // With no `dataLayer` key the rule is silent even in a scoped file: the tool does not
    // guess the data layer.
    let root = data_layer("dlm-unconfigured");
    write(&root, ".spirolorc.jsonc", "{}\n");

    let output = spirolo()
        .args([
            "check",
            "--all",
            "--format",
            "json",
            "--rule",
            "data-layer-mock",
        ])
        .arg(&root)
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json["findings"].as_array().unwrap().is_empty());
}

#[test]
fn data_layer_mock_runs_under_the_audit_family() {
    let root = data_layer("dlm-audit-family");
    // `spirolo audit` runs the audit family, which data-layer-mock is in.
    spirolo()
        .args(["audit", "--all"])
        .arg(&root)
        .assert()
        .success()
        .stdout(predicate::str::contains("data-layer-mock"));
    // `spirolo gate` runs the gate family, which data-layer-mock is not in.
    spirolo()
        .args(["gate", "--all"])
        .arg(&root)
        .assert()
        .success()
        .stdout(predicate::str::contains("data-layer-mock").not());
}
