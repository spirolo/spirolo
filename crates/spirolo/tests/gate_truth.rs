//! `spirolo check` end to end on `gate-truth` fixtures: what a maintainer and an agent get from
//! the binary when a project's gates are weaker than they look.
//!
//! Seam 2 of the PRD's testing decisions. Each fixture is a real project driven through the
//! binary with `assert_cmd`, its findings JSON read back:
//!
//! - `weak_gates`: a Vitest config with no thresholds, a Stryker `break: 0`, a Playwright
//!   config with no `forbidOnly`, and a CI workflow whose only steps lint and type-check →
//!   thresholds-absent, mutation-break-zero and ci-runs-nothing. The unused Playwright
//!   config contributes no browser-gate finding.
//! - `strong_gates`: thresholds set, `break: 55`, `forbidOnly: true`, and a workflow that runs
//!   `vitest run --coverage` → silent.
//!
//! The rule's branches are covered on hand-built models in `spirolo-rules`; here we prove the
//! wiring — the config recogniser, the CI reader, the rule and the report — is whole, including
//! that the CI reader reads a real `.github/workflows/*.yml` off disk.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

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

/// The `findings` array of `spirolo check --format json --rule gate-truth` on `root`.
fn findings(root: &Path) -> Vec<Value> {
    let output = spirolo()
        .args(["check", "--all", "--format", "json", "--rule", "gate-truth"])
        .arg(root)
        .output()
        .unwrap();
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    json["findings"].as_array().cloned().unwrap_or_default()
}

/// The `findings` array with an explicit `.spirolorc.jsonc` config written at `root`.
///
/// The claims half reads the documents the config names, so the fixture must carry a config with
/// a `claims` key; `check` reads it from the project root the same way a real project would.
fn findings_with_config(root: &Path, config: &str) -> Vec<Value> {
    write(root, ".spirolorc.jsonc", config);
    findings(root)
}

/// The `kind` evidence value of a finding.
fn kind(finding: &Value) -> &str {
    finding["evidence"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|e| e["key"] == "kind")
        .and_then(|e| e["value"].as_str())
        .unwrap_or("")
}

/// Whether any finding carries `slug` as its kind.
fn has_kind(found: &[Value], slug: &str) -> bool {
    found.iter().any(|f| kind(f) == slug)
}

/// A project whose four gates are all weaker than they look, plus a source file so the runner
/// has something to gate.
fn weak_project(name: &str) -> PathBuf {
    let root = project(name);
    // Vitest with no coverage thresholds.
    write(
        &root,
        "vitest.config.ts",
        "import { defineConfig } from 'vitest/config'\n\
         export default defineConfig({\n\
         \x20 test: { include: ['src/**/*.test.ts'] },\n\
         })\n",
    );
    // Stryker with break: 0.
    write(
        &root,
        "stryker.conf.mjs",
        "export default { mutate: ['src/**/*.ts'], thresholds: { break: 0 } }\n",
    );
    // Playwright with no forbidOnly.
    write(
        &root,
        "playwright.config.ts",
        "import { defineConfig } from '@playwright/test'\n\
         export default defineConfig({ testDir: './e2e' })\n",
    );
    // A CI workflow that lints and type-checks but never runs the tests.
    write(
        &root,
        ".github/workflows/ci.yml",
        "name: CI\n\
         on: [push]\n\
         jobs:\n\
         \x20 check:\n\
         \x20   runs-on: ubuntu-latest\n\
         \x20   steps:\n\
         \x20     - uses: actions/checkout@v4\n\
         \x20     - run: pnpm install\n\
         \x20     - run: pnpm lint\n\
         \x20     - run: pnpm tsc --noEmit\n",
    );
    write(
        &root,
        "src/sum.ts",
        "export const sum = (a: number, b: number) => a + b\n",
    );
    write(
        &root,
        "src/sum.test.ts",
        "import { it, expect } from 'vitest'\n\
         import { sum } from './sum'\n\
         it('adds', () => { expect(sum(1, 2)).toBe(3) })\n",
    );
    root
}

#[test]
fn a_weak_project_reports_gate_weaknesses_but_not_an_unused_browser_config() {
    let found = findings(&weak_project("gt-weak"));
    assert!(has_kind(&found, "thresholds-absent"), "{found:#?}");
    assert!(has_kind(&found, "mutation-break-zero"), "{found:#?}");
    assert!(!has_kind(&found, "forbid-only"), "unused in CI: {found:#?}");
    assert!(has_kind(&found, "ci-runs-nothing"), "{found:#?}");
    assert!(
        found.iter().all(|f| f["rule"] == "gate-truth"),
        "only gate-truth findings: {found:#?}"
    );
}

#[test]
fn the_ci_runs_nothing_finding_sits_at_the_workflows_directory_and_names_the_workflow() {
    let found = findings(&weak_project("gt-ci-name"));
    let nothing: Vec<&Value> = found
        .iter()
        .filter(|f| kind(f) == "ci-runs-nothing")
        .collect();
    assert_eq!(nothing.len(), 1, "{found:#?}");
    assert_eq!(nothing[0]["file"], ".github/workflows", "{found:#?}");
    let workflows = nothing[0]["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["key"] == "workflows")
        .expect("a workflows evidence entry");
    assert_eq!(workflows["value"], ".github/workflows/ci.yml");
}

/// A project whose four gates all hold: thresholds set, break above zero, forbidOnly true, and
/// a workflow that runs the tests with coverage.
fn strong_project(name: &str) -> PathBuf {
    let root = project(name);
    write(
        &root,
        "vitest.config.ts",
        "import { defineConfig } from 'vitest/config'\n\
         export default defineConfig({\n\
         \x20 test: {\n\
         \x20   include: ['src/**/*.test.ts'],\n\
         \x20   coverage: { thresholds: { lines: 80, branches: 70 } },\n\
         \x20 },\n\
         })\n",
    );
    write(
        &root,
        "stryker.conf.mjs",
        "export default { mutate: ['src/**/*.ts'], thresholds: { break: 55 } }\n",
    );
    write(
        &root,
        "playwright.config.ts",
        "import { defineConfig } from '@playwright/test'\n\
         export default defineConfig({ testDir: './e2e', forbidOnly: true })\n",
    );
    write(
        &root,
        ".github/workflows/ci.yml",
        "name: CI\n\
         on: [push]\n\
         jobs:\n\
         \x20 test:\n\
         \x20   runs-on: ubuntu-latest\n\
         \x20   steps:\n\
         \x20     - uses: actions/checkout@v4\n\
         \x20     - run: pnpm vitest run --coverage\n",
    );
    write(
        &root,
        "src/sum.ts",
        "export const sum = (a: number, b: number) => a + b\n",
    );
    write(
        &root,
        "src/sum.test.ts",
        "import { it, expect } from 'vitest'\n\
         import { sum } from './sum'\n\
         it('adds', () => { expect(sum(1, 2)).toBe(3) })\n",
    );
    root
}

/// A monorepo whose three packages each `export default preset` (a bare re-export, no
/// `mergeConfig`) of one shared preset that declares no thresholds and no fail-closed setup;
/// `packages/c` adds its own thresholds.
fn bare_reexport_monorepo(name: &str) -> PathBuf {
    let root = project(name);
    write(
        &root,
        "pnpm-workspace.yaml",
        "packages:\n  - 'packages/*'\n",
    );
    write(
        &root,
        "package.json",
        "{ \"name\": \"root\", \"private\": true }\n",
    );
    write(
        &root,
        "packages/config/package.json",
        "{ \"name\": \"@fixture/config\", \"exports\": { \"./base\": \"./base.mjs\" } }\n",
    );
    write(
        &root,
        "packages/config/base.mjs",
        "import { defineConfig } from 'vitest/config'\n\
         export default defineConfig({ test: { include: ['src/**/*.test.ts'] } })\n",
    );
    for pkg in ["a", "b", "c"] {
        write(
            &root,
            &format!("packages/{pkg}/package.json"),
            &format!("{{ \"name\": \"@fixture/{pkg}\" }}\n"),
        );
        let config = if pkg == "c" {
            "import { defineConfig, mergeConfig } from 'vitest/config'\n\
             import base from '@fixture/config/base'\n\
             export default mergeConfig(base, defineConfig({ test: { coverage: { thresholds: { lines: 50 } } } }))\n"
        } else {
            "import base from '@fixture/config/base'\nexport default base\n"
        };
        write(&root, &format!("packages/{pkg}/vitest.config.mts"), config);
        write(
            &root,
            &format!("packages/{pkg}/src/example.test.ts"),
            "import { it, expect } from 'vitest'\nit('adds', () => { expect(1 + 1).toBe(2) })\n",
        );
    }
    root
}

#[test]
fn bare_re_exports_of_a_threshold_less_preset_lift_to_one_finding_at_the_preset() {
    let root = bare_reexport_monorepo("gt-bare-reexports");
    let found = findings(&root);
    let absent: Vec<&Value> = found
        .iter()
        .filter(|f| kind(f) == "thresholds-absent")
        .collect();
    assert_eq!(absent.len(), 1, "{found:#?}");
    assert_eq!(absent[0]["file"], "packages/config/base.mjs");
    assert_eq!(absent[0]["outcome"], "finding");
    let packages = absent[0]["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["key"] == "packages")
        .expect("a packages evidence entry");
    assert_eq!(
        packages["value"], "2",
        "a and b inherit the gap; c overrides: {:#?}",
        absent[0]
    );
    assert_eq!(
        absent[0]["id"], "gate-truth packages/config/base.mjs thresholds-absent#0",
        "identity is the preset path and the slug, no line"
    );
    assert!(
        !found.iter().any(|f| f["outcome"] == "could_not_decide"),
        "every bare re-export was read whole: {found:#?}"
    );
}

#[test]
fn bare_re_exports_lift_the_setup_gap_to_the_preset_too() {
    let root = bare_reexport_monorepo("gt-bare-reexports-setup");
    let output = spirolo()
        .args([
            "check",
            "--all",
            "--format",
            "json",
            "--rule",
            "fail-closed-setup",
        ])
        .arg(&root)
        .output()
        .unwrap();
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let found = json["findings"].as_array().cloned().unwrap_or_default();
    let mut rows: Vec<(String, String, String)> = found
        .iter()
        .map(|f| {
            let evidence = f["evidence"].as_array().unwrap();
            let get = |key: &str| {
                evidence
                    .iter()
                    .find(|e| e["key"] == key)
                    .map_or(String::new(), |e| e["value"].as_str().unwrap().to_owned())
            };
            (
                f["file"].as_str().unwrap().to_owned(),
                get("invariant"),
                get("packages"),
            )
        })
        .collect();
    rows.sort();
    assert_eq!(
        rows,
        vec![
            (
                "packages/config/base.mjs".to_owned(),
                "console-guard".to_owned(),
                "3".to_owned()
            ),
            (
                "packages/config/base.mjs".to_owned(),
                "restore-mocks".to_owned(),
                "3".to_owned()
            ),
        ],
        "one finding per missing invariant at the preset, none per package: {found:#?}"
    );
}

#[test]
fn an_unreadable_config_is_a_could_not_decide_row_that_says_why() {
    let root = project("gt-unread");
    write(
        &root,
        "vitest.config.ts",
        "import { defineConfig } from 'vitest/config'\n\
         export default defineConfig(() => ({ test: { include: ['src/**/*.test.ts'] } }))\n",
    );
    write(
        &root,
        "src/example.test.ts",
        "import { it, expect } from 'vitest'\nit('adds', () => { expect(1 + 1).toBe(2) })\n",
    );
    let found = findings(&root);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0]["outcome"], "could_not_decide");
    assert_eq!(kind(&found[0]), "config-unread");
    assert_eq!(found[0]["file"], "vitest.config.ts");
    assert!(
        found[0]["explain"]
            .as_str()
            .unwrap()
            .contains("`defineConfig(…)` is passed a function the reader does not evaluate"),
        "{:#?}",
        found[0]
    );
    // A could-not-decide row never fails the build and is counted apart in the human view.
    spirolo()
        .args(["check", "--all"])
        .args(["--rule", "gate-truth"])
        .arg(&root)
        .assert()
        .success()
        .stdout(predicate::str::contains("could not decide"));
}

#[test]
fn a_strong_project_reports_no_gate_weakness() {
    let found = findings(&strong_project("gt-strong"));
    assert!(
        found.is_empty(),
        "every gate holds: no finding expected, got {found:#?}"
    );
}

#[test]
fn the_findings_json_is_stable() {
    let root = weak_project("gt-snapshot");
    insta::assert_json_snapshot!(
        "check_json_gate_truth",
        status_support::redact_status(&serde_json::Value::Array(findings(&root)))
    );
}

/// The claims-diff findings — the ones whose `kind` evidence is `claim-unbacked`.
fn claim_findings(found: &[Value]) -> Vec<&Value> {
    found
        .iter()
        .filter(|f| kind(f) == "claim-unbacked")
        .collect()
}

/// A minimal Vitest project with a CI workflow that runs the suite, a config naming
/// `docs/PLAYBOOK.md` as a claims document and a `docs/PLAYBOOK.md` carrying `playbook_line`.
/// `thresholds` controls whether the Vitest config declares coverage thresholds, so the same
/// "enforced in CI" claim can be tested backed (thresholds + a CI step that runs coverage) and
/// unbacked (no thresholds).
fn claims_project(name: &str, playbook_line: &str, thresholds: bool) -> PathBuf {
    let root = project(name);
    let coverage = if thresholds {
        "\x20   coverage: { thresholds: { lines: 80 } },\n"
    } else {
        ""
    };
    write(
        &root,
        "vitest.config.ts",
        &format!(
            "import {{ defineConfig }} from 'vitest/config'\n\
             export default defineConfig({{\n\
             \x20 test: {{\n\
             \x20   include: ['src/**/*.test.ts'],\n\
             {coverage}\
             \x20 }},\n\
             }})\n"
        ),
    );
    // A CI workflow that runs the suite with coverage, so a "enforced in CI" claim is backed
    // whenever the runner also declares thresholds — the other case (thresholds set but
    // CI runs nothing) is tested on the hand-built model in `spirolo-rules`.
    write(
        &root,
        ".github/workflows/ci.yml",
        "name: CI\n\
         on: [push]\n\
         jobs:\n\
         \x20 test:\n\
         \x20   runs-on: ubuntu-latest\n\
         \x20   steps:\n\
         \x20     - run: pnpm vitest run --coverage\n",
    );
    write(
        &root,
        "src/sum.ts",
        "export const sum = (a: number, b: number) => a + b\n",
    );
    write(
        &root,
        "src/sum.test.ts",
        "import { it, expect } from 'vitest'\n\
         import { sum } from './sum'\n\
         it('adds', () => { expect(sum(1, 2)).toBe(3) })\n",
    );
    write(
        &root,
        "docs/PLAYBOOK.md",
        &format!("# Testing playbook\n\n{playbook_line}\n\nWe review every PR.\n"),
    );
    write(
        &root,
        ".spirolorc.jsonc",
        "{ \"claims\": [\"docs/PLAYBOOK.md\"], \"rules\": {\"gate-truth\": \"info\"} }",
    );
    root
}

#[test]
fn a_coverage_enforced_claim_with_no_thresholds_reports_at_the_doc_line() {
    let root = claims_project(
        "gt-claim-unbacked",
        "coverage thresholds are enforced in CI",
        false,
    );
    let found = findings(&root);
    let claims = claim_findings(&found);
    assert_eq!(claims.len(), 1, "{found:#?}");
    assert_eq!(claims[0]["file"], "docs/PLAYBOOK.md", "{found:#?}");
    assert_eq!(claims[0]["line"], 3, "the claim's own line: {found:#?}");
    assert_eq!(claims[0]["severity"], "info", "{found:#?}");
}

#[test]
fn the_same_claim_with_thresholds_present_is_silent() {
    let root = claims_project(
        "gt-claim-backed",
        "coverage thresholds are enforced in CI",
        true,
    );
    let found = findings(&root);
    assert!(claim_findings(&found).is_empty(), "{found:#?}");
}

#[test]
fn a_report_only_line_beside_break_null_reports_nothing() {
    // The narrow grammar reads `report-only` as a waiver, not a claim, so a Stryker `break: null`
    // (unset) sitting beside it is no disagreement.
    let root = project("gt-claim-report-only");
    write(
        &root,
        "stryker.conf.mjs",
        "export default { mutate: ['src/**/*.ts'] }\n", // no `break` at all: null/unset
    );
    write(
        &root,
        "src/sum.ts",
        "export const sum = (a: number, b: number) => a + b\n",
    );
    write(
        &root,
        "docs/PLAYBOOK.md",
        "# Mutation\n\nMutation testing is report-only for now; break is null.\n",
    );
    let found = findings_with_config(
        &root,
        "{ \"claims\": [\"docs/PLAYBOOK.md\"], \"rules\": {\"gate-truth\": \"info\"} }",
    );
    assert!(claim_findings(&found).is_empty(), "{found:#?}");
}

#[test]
fn claims_findings_are_stable() {
    let root = claims_project(
        "gt-claim-snapshot",
        "Coverage thresholds are enforced in CI, and mutation score below 0.7 blocks the merge.",
        false,
    );
    insta::assert_json_snapshot!(
        "check_json_gate_truth_claims",
        status_support::redact_status(&serde_json::Value::Array(
            claim_findings(&findings(&root))
                .into_iter()
                .cloned()
                .collect()
        ))
    );
}

#[test]
fn gate_truth_runs_under_the_gate_family() {
    let root = weak_project("gt-gate-family");
    // `spirolo gate` runs the gate family, which gate-truth is in.
    spirolo()
        .args(["gate", "--all"])
        .arg(&root)
        .assert()
        .stdout(predicate::str::contains("gate-truth"));
    // `spirolo audit` runs the audit family, which gate-truth is not in.
    spirolo()
        .args(["audit", "--all"])
        .arg(&root)
        .assert()
        .stdout(predicate::str::contains("gate-truth").not());
}
