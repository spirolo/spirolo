//! `spirolo hygiene` / `spirolo check` end to end on a `duplicate-test` fixture project: what a
//! maintainer and an agent get from the binary when the rule groups clones by body hash.
//!
//! Seam 2 of the PRD's testing decisions: a fixture with a within-file clone pair, a
//! cross-file pair over two sources, a loop-defined test and a `.each` table (both excluded)
//! and a near-miss (shared arrange, different assert) that must not fire, driven through the
//! real binary with `assert_cmd`, its four views snapshotted with `insta`. The rule's own
//! branches are covered on hand-built models in `spirolo-rules`; here we prove the wiring —
//! model, rule, report, exit codes, the family and the command — is whole.

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

/// The fixture the acceptance criteria name. `format.test.ts` holds a within-file clone pair
/// (`formats the value` / `renders the value the same way`, identical once the local name is
/// normalised), a loop-defined test and a `.each` table that share or take their own body but
/// are excluded as one-definition-many-runs, and a near-miss pair whose only difference is the
/// asserted literal. `alpha.test.ts` and `beta.test.ts` are a cross-file clone pair over one
/// source module, `parse.ts`, as the two `NotionNext` suites over `getPostBlocks` are.
fn clones(name: &str) -> PathBuf {
    let root = project(name);
    write(
        &root,
        "src/format.ts",
        "export function format(value: string): string { return value }\n",
    );
    write(
        &root,
        "src/parse.ts",
        "export function parse(input: string): number { return Number(input) }\n",
    );
    write(
        &root,
        "src/format.test.ts",
        "import { describe, it, expect } from 'vitest'\n\
         import { format } from './format'\n\
         const samples = ['a', 'b']\n\
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
         \x20 for (const value of samples) {\n\
         \x20   it(`formats ${value}`, () => {\n\
         \x20     const out = format(value)\n\
         \x20     expect(out).toBe('ok')\n\
         \x20   })\n\
         \x20 }\n\
         \x20 it.each([[1, 2], [3, 4]])('adds %i and %i', (a, b) => {\n\
         \x20   expect(add(a, b)).toBe(a + b)\n\
         \x20 })\n\
         \x20 it('shares arrange but asserts one thing', () => {\n\
         \x20   const out = format(value)\n\
         \x20   expect(out).toBe('one')\n\
         \x20 })\n\
         \x20 it('shares arrange but asserts another', () => {\n\
         \x20   const out = format(value)\n\
         \x20   expect(out).toBe('two')\n\
         \x20 })\n\
         })\n",
    );
    write(
        &root,
        "src/alpha.test.ts",
        "import { it, expect } from 'vitest'\n\
         import { parse } from './parse'\n\
         \n\
         it('parses alpha input', () => {\n\
         \x20 const n = parse(input)\n\
         \x20 expect(n).toBe(42)\n\
         })\n",
    );
    write(
        &root,
        "src/beta.test.ts",
        "import { it, expect } from 'vitest'\n\
         import { parse } from './parse'\n\
         \n\
         it('parses beta input', () => {\n\
         \x20 const value = parse(input)\n\
         \x20 expect(value).toBe(42)\n\
         })\n",
    );
    root
}

/// The `duplicate-test` findings of a `--format json` run, as parsed JSON values.
fn duplicate_findings(root: &Path, subcommand: &str) -> Vec<serde_json::Value> {
    let output = spirolo()
        .args([
            subcommand,
            "--all",
            "--format",
            "json",
            "--rule",
            "duplicate-test",
        ])
        .arg(root)
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    json["findings"].as_array().unwrap().clone()
}

#[test]
fn hygiene_reports_the_two_clone_groups_and_leaves_the_loop_each_and_near_miss_alone() {
    let root = clones("groups");
    let findings = duplicate_findings(&root, "hygiene");

    // Exactly two groups: the cross-file pair and the within-file pair.
    assert_eq!(findings.len(), 2, "{findings:#?}");
    assert!(findings.iter().all(|f| f["rule"] == "duplicate-test"));
    assert!(findings.iter().all(|f| f["severity"] == "info"));

    // The within-file group is size two: the loop-defined test that shares its body is
    // excluded, so the group did not grow to three.
    let within = findings
        .iter()
        .find(|f| f["file"] == "src/format.test.ts")
        .expect("the within-file group");
    let size = within["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["key"] == "size")
        .unwrap();
    assert_eq!(size["value"], "2", "the loop-defined clone is excluded");

    // The cross-file group spans two files and names each member's paired source: one module.
    let cross = findings
        .iter()
        .find(|f| f["file"] == "src/alpha.test.ts")
        .expect("the cross-file group");
    let evidence = cross["evidence"].as_array().unwrap();
    assert!(evidence
        .iter()
        .any(|e| e["key"] == "cross-file" && e["value"] == "yes"));
    let sources = evidence.iter().find(|e| e["key"] == "sources").unwrap();
    assert_eq!(
        sources["value"],
        "src/alpha.test.ts: src/parse.ts; src/beta.test.ts: src/parse.ts"
    );

    // The near-miss pair (same arrange, different asserted literal) never grouped.
    assert!(
        !findings
            .iter()
            .any(|f| f["message"].as_str().unwrap().contains("shares arrange")),
        "shared-arrange-different-assert is not a clone: {findings:#?}"
    );
}

#[test]
fn info_findings_leave_the_exit_code_at_zero() {
    let root = clones("exit");
    spirolo()
        .args(["hygiene", "--all"])
        .arg(&root)
        .assert()
        .code(0);
    spirolo()
        .args(["check", "--all"])
        .arg(&root)
        .assert()
        .code(0);
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

// ------------------------------------------------------------------ the four snapshotted views

#[test]
fn each_group_carries_the_title_path_of_the_member_it_is_located_at() {
    let root = clones("dup-title-path");
    let output = spirolo()
        .args([
            "check",
            "--all",
            "--format",
            "json",
            "--rule",
            "duplicate-test",
        ])
        .arg(&root)
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let findings = json["findings"].as_array().unwrap();

    // A group is located at its first member: the title path is that member's suite path
    // and title, as the model's case row names it; the other members stay related locations.
    assert!(!findings.is_empty());
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
}

#[test]
fn the_findings_json_is_stable() {
    let root = clones("json-snapshot");
    let output = spirolo()
        .args([
            "check",
            "--all",
            "--format",
            "json",
            "--rule",
            "duplicate-test",
        ])
        .arg(&root)
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    insta::assert_json_snapshot!(
        "check_json_duplicate_test",
        status_support::redact_status(&json["findings"])
    );
}

#[test]
fn the_sarif_is_stable() {
    let root = clones("sarif-snapshot");
    let output = spirolo()
        .args([
            "check",
            "--all",
            "--format",
            "sarif",
            "--rule",
            "duplicate-test",
        ])
        .arg(&root)
        .output()
        .unwrap();
    let sarif = String::from_utf8(output.stdout).unwrap();
    // The driver version churns on a bump; redact it so the snapshot does not.
    let redacted = sarif.replace(env!("CARGO_PKG_VERSION"), "<version>");
    insta::assert_snapshot!(
        "check_sarif_duplicate_test",
        status_support::redact_status_text(&redacted)
    );
}

#[test]
fn the_human_view_is_stable() {
    let root = clones("human-snapshot");
    let output = spirolo()
        .args(["check", "--all", "--rule", "duplicate-test"])
        .arg(&root)
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    insta::assert_snapshot!(
        "check_human_duplicate_test",
        status_support::redact_heading_status(&redact_timing(&stdout))
    );
}

#[test]
fn the_agent_view_is_stable() {
    let root = clones("agent-snapshot");
    let output = spirolo()
        .args(["check", "--all", "--agent", "--rule", "duplicate-test"])
        .arg(&root)
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    insta::assert_snapshot!(
        "check_agent_duplicate_test",
        status_support::redact_agent_status(&stdout)
    );
}

// ------------------------------------------------------------------ the four views under --propose

#[test]
fn the_findings_json_under_propose_is_stable() {
    let root = clones("dup-json-propose");
    let output = spirolo()
        .args([
            "check",
            "--all",
            "--propose",
            "--format",
            "json",
            "--rule",
            "duplicate-test",
        ])
        .arg(&root)
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    insta::assert_json_snapshot!(
        "check_json_duplicate_test_propose",
        status_support::redact_status(&json["findings"])
    );
}

#[test]
fn the_sarif_under_propose_is_stable() {
    let root = clones("dup-sarif-propose");
    let output = spirolo()
        .args([
            "check",
            "--all",
            "--propose",
            "--format",
            "sarif",
            "--rule",
            "duplicate-test",
        ])
        .arg(&root)
        .output()
        .unwrap();
    let sarif = String::from_utf8(output.stdout).unwrap();
    let redacted = sarif.replace(env!("CARGO_PKG_VERSION"), "<version>");
    insta::assert_snapshot!(
        "check_sarif_duplicate_test_propose",
        status_support::redact_status_text(&redacted)
    );
}

#[test]
fn the_human_view_under_propose_is_stable() {
    let root = clones("dup-human-propose");
    let output = spirolo()
        .args(["check", "--all", "--propose", "--rule", "duplicate-test"])
        .arg(&root)
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    insta::assert_snapshot!(
        "check_human_duplicate_test_propose",
        status_support::redact_heading_status(&redact_timing(&stdout))
    );
}

#[test]
fn the_agent_view_under_propose_is_stable() {
    let root = clones("dup-agent-propose");
    let output = spirolo()
        .args([
            "check",
            "--all",
            "--propose",
            "--agent",
            "--rule",
            "duplicate-test",
        ])
        .arg(&root)
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    insta::assert_snapshot!(
        "check_agent_duplicate_test_propose",
        status_support::redact_agent_status(&stdout)
    );
}

// ------------------------------------------------------------------- the family and the command

#[test]
fn hygiene_runs_the_family_and_rule_narrows_it_and_check_includes_it() {
    let root = clones("family");

    // `spirolo hygiene` runs the hygiene family; duplicate-test is its only rule so far.
    spirolo()
        .args(["hygiene", "--all"])
        .arg(&root)
        .assert()
        .success()
        .stdout(predicate::str::contains("duplicate-test"));

    // `--rule duplicate-test` narrows the family to that one rule.
    spirolo()
        .args(["hygiene", "--all", "--rule", "duplicate-test"])
        .arg(&root)
        .assert()
        .success()
        .stdout(predicate::str::contains("duplicate-test"));

    // A rule from another family is rejected under `hygiene`.
    spirolo()
        .args(["hygiene", "--all", "--rule", "weak-assertion"])
        .arg(&root)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("not in the hygiene family"));

    // `spirolo check` includes the hygiene family.
    spirolo()
        .args(["check", "--all"])
        .arg(&root)
        .assert()
        .success()
        .stdout(predicate::str::contains("duplicate-test"));
}

#[test]
fn a_config_raising_a_hygiene_rule_to_error_is_rejected_with_a_diagnostic() {
    let root = clones("reject-error");
    write(
        &root,
        ".spirolorc.jsonc",
        "{ \"rules\": { \"duplicate-test\": \"error\" } }\n",
    );
    // The config error is refused the way a malformed config is: exit 2.
    spirolo()
        .args(["check", "--all"])
        .arg(&root)
        .assert()
        .code(2);

    // The refusal is a diagnostic naming the rule, carried in every structured view.
    let output = spirolo()
        .args(["check", "--all", "--format", "json"])
        .arg(&root)
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let diagnostic = json["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["code"] == "hygiene_severity_rejected")
        .expect("a diagnostic rejecting the hygiene severity");
    assert!(diagnostic["message"]
        .as_str()
        .unwrap()
        .contains("duplicate-test"));

    // The refused override falls back to the current release policy, never error.
    let expected = if spirolo_rules::status::is_proven("duplicate-test") {
        "warn"
    } else {
        "info"
    };
    let findings = json["findings"].as_array().unwrap();
    assert!(findings
        .iter()
        .filter(|f| f["rule"] == "duplicate-test")
        .all(|f| f["severity"] == expected));
}

#[test]
fn two_runs_are_byte_identical() {
    let root = clones("determinism");
    let once = spirolo()
        .args(["check", "--all", "--format", "json"])
        .arg(&root)
        .output()
        .unwrap()
        .stdout;
    let twice = spirolo()
        .args(["check", "--all", "--format", "json"])
        .arg(&root)
        .output()
        .unwrap()
        .stdout;
    assert_eq!(
        once, twice,
        "two runs over the same tree are byte-identical"
    );
}

// ------------------------------------------------------------- shapes from round 1 (#404)

/// A Jest project holding `files`, under a `dup-` prefixed name so it never shares a
/// directory with another rule's fixture.
fn jest_project(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = project(&format!("dup-{name}"));
    write(
        &root,
        "package.json",
        "{ \"name\": \"fixture\", \"devDependencies\": { \"jest\": \"^29.0.0\" } }\n",
    );
    for (path, contents) in files {
        write(&root, path, contents);
    }
    root
}

#[test]
fn empty_skipped_placeholders_are_not_duplicates() {
    // WatermelonDB `src/Database/test.js:119`: placeholders that test nothing yet.
    let root = jest_project(
        "placeholders",
        &[(
            "src/Database/test.js",
            "describe('Database', () => {\n\
         \x20 describe('unsafeResetDatabase', () => {\n\
         \x20   it.skip('Cancels withChangesForTables observation during reset', async () => {})\n\
         \x20   it.skip('Cancels Collection change observation during reset', async () => {})\n\
         \x20   it.skip('Cancels Collection experimental subscribers during reset', async () => {})\n\
         \x20 })\n\
         })\n",
        )],
    );
    let findings = duplicate_findings(&root, "check");
    assert!(findings.is_empty(), "{findings:#?}");
}

#[test]
fn identical_tests_under_a_conditional_in_body_skip_are_still_duplicates() {
    // A Playwright `test.skip(condition, reason)` skips on one browser and runs on the rest:
    // the twins still run, so they are not placeholders.
    let body = |title: &str| {
        format!(
            "test('{title}', async ({{ page, browserName }}) => {{\n\
             \x20 test.skip(browserName === 'webkit', 'flaky on webkit')\n\
             \x20 await page.goto('/')\n\
             \x20 await expect(page.locator('h1')).toHaveText('Home')\n\
             }})\n"
        )
    };
    let root = project("dup-conditional-skip");
    write(
        &root,
        "package.json",
        "{ \"name\": \"fixture\", \"devDependencies\": { \"@playwright/test\": \"^1.40.0\" } }\n",
    );
    write(
        &root,
        "e2e/home.spec.ts",
        &format!(
            "import {{ test, expect }} from '@playwright/test'\n{}{}",
            body("opens home"),
            body("opens home again")
        ),
    );
    let findings = duplicate_findings(&root, "check");
    assert_eq!(findings.len(), 1, "{findings:#?}");
}

#[test]
fn the_same_playwright_body_over_a_port_of_each_describe_is_not_a_duplicate() {
    // react-router `integration/rsc/rsc-nojs-test.ts:232` and `rsc-test.ts:1911`: each
    // file's describe declares its own `port`, and its own `beforeAll` starts its own server.
    let file = |suite: &str| {
        format!(
            "import {{ test, expect }} from '@playwright/test'\n\
             import getPort from 'get-port'\n\
             \n\
             test.describe('{suite}', () => {{\n\
             \x20 let port: number\n\
             \n\
             \x20 test.beforeAll(async () => {{\n\
             \x20   port = await getPort()\n\
             \x20 }})\n\
             \n\
             \x20 test('Support throwing redirect Response from render', async ({{ page }}) => {{\n\
             \x20   await page.goto(`http://localhost:${{port}}/render-redirect`)\n\
             \x20   await expect(page.getByText('home')).toBeAttached()\n\
             \x20 }})\n\
             }})\n"
        )
    };
    let root = project("dup-playwright-port");
    write(
        &root,
        "package.json",
        "{ \"name\": \"fixture\", \"devDependencies\": { \"@playwright/test\": \"^1.40.0\" } }\n",
    );
    write(&root, "integration/rsc/rsc-nojs.spec.ts", &file("RSC nojs"));
    write(&root, "integration/rsc/rsc.spec.ts", &file("RSC"));
    let listed = spirolo()
        .args(["list", "--format", "json"])
        .arg(&root)
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&listed.stdout).contains("Support throwing redirect"),
        "the fixture parses to no cases"
    );
    let findings = duplicate_findings(&root, "check");
    assert!(findings.is_empty(), "{findings:#?}");
}

#[test]
fn one_test_two_recognisers_list_is_not_a_duplicate_of_itself() {
    // mui `test/e2e-website/material-icons.spec.ts:6`: with Vitest and Playwright both in
    // the project, both recognisers list the one definition.
    let root = project("dup-two-recognisers");
    write(
        &root,
        "package.json",
        "{ \"name\": \"fixture\", \"devDependencies\": { \"@playwright/test\": \"^1.40.0\", \"vitest\": \"^3.0.0\" } }\n",
    );
    write(
        &root,
        "test/e2e-website/material-icons.spec.ts",
        "import { test as base, expect } from '@playwright/test';\n\
         import { TestFixture } from './playwright.config';\n\
         \n\
         const test = base.extend<TestFixture>({});\n\
         \n\
         test('should see the selected icon popup that match the query', async ({ page }) => {\n\
         \x20 await page.goto('/material-ui/material-icons/?selected=AcUnit');\n\
         \n\
         \x20 await expect(page.locator('.MuiDialog-container h2:has-text(\"AcUnit\")')).toBeVisible();\n\
         });\n",
    );
    let findings = duplicate_findings(&root, "check");
    assert!(findings.is_empty(), "{findings:#?}");
}

#[test]
fn empty_bodies_that_run_are_not_duplicates_either() {
    // carbon `packages/web-components/src/components/accordion/__tests__/accordion-test.js:162`.
    let root = jest_project(
        "empty-bodies",
        &[(
            "src/accordion/__tests__/accordion-test.js",
            "describe('cds-accordion', () => {\n\
             \x20 describe('Renders as expected', () => {\n\
             \x20   it('should be an ol if prop ordered is passed as true', () => {});\n\
             \x20   it('should be a ul if prop ordered is passed as false', () => {});\n\
             \x20 });\n\
             });\n",
        )],
    );
    let findings = duplicate_findings(&root, "check");
    assert!(findings.is_empty(), "{findings:#?}");
}

#[test]
fn jsx_bodies_differing_only_in_an_attribute_name_are_not_duplicates() {
    // grommet `Box-layout-test.tsx:402` (`gridArea="header"`) vs `Box-test.tsx:26` (`as="header"`).
    let body = |title: &str, attribute: &str| {
        format!(
            "import React from 'react';\n\
             import {{ render }} from '@testing-library/react';\n\
             import {{ Grommet }} from '../../Grommet';\n\
             import {{ Box }} from '..';\n\
             \n\
             describe('Box', () => {{\n\
             \x20 test('{title}', () => {{\n\
             \x20   const {{ container }} = render(\n\
             \x20     <Grommet>\n\
             \x20       <Box {attribute}=\"header\" />\n\
             \x20     </Grommet>,\n\
             \x20   );\n\
             \n\
             \x20   expect(container.firstChild).toMatchSnapshot();\n\
             \x20 }});\n\
             }});\n"
        )
    };
    let layout = body("gridArea", "gridArea");
    let plain = body("as string", "as");
    let root = jest_project(
        "jsx-attribute",
        &[
            (
                "src/js/components/Grommet/index.ts",
                "export const Grommet = null;\n",
            ),
            (
                "src/js/components/Box/index.ts",
                "export const Box = null;\n",
            ),
            (
                "src/js/components/Box/__tests__/Box-layout-test.tsx",
                &layout,
            ),
            ("src/js/components/Box/__tests__/Box-test.tsx", &plain),
        ],
    );
    let findings = duplicate_findings(&root, "check");
    assert!(findings.is_empty(), "{findings:#?}");
}

#[test]
fn identical_jsx_bodies_at_two_nesting_depths_are_duplicates() {
    // The same multi-line JSX once at top level and once inside a describe: only the
    // indentation between the tags differs, and JSX drops that whitespace.
    let root = jest_project(
        "jsx-indentation",
        &[(
            "src/jsx.test.jsx",
            "import { render } from '@testing-library/react'\n\
             it('renders the list', () => {\n\
             \x20 const { container } = render(\n\
             \x20   <ul>\n\
             \x20     <li>one</li>\n\
             \x20   </ul>\n\
             \x20 )\n\
             \x20 expect(container.firstChild).toBeTruthy()\n\
             })\n\
             describe('nested', () => {\n\
             \x20 it('renders the list again', () => {\n\
             \x20   const { container } = render(\n\
             \x20     <ul>\n\
             \x20       <li>one</li>\n\
             \x20     </ul>\n\
             \x20   )\n\
             \x20   expect(container.firstChild).toBeTruthy()\n\
             \x20 })\n\
             })\n",
        )],
    );
    let findings = duplicate_findings(&root, "check");
    assert_eq!(findings.len(), 1, "{findings:#?}");
}

#[test]
fn the_explain_names_the_copy_paste_case() {
    // react-native-paper `Checkbox.test.tsx:30` and `:38`: the second title promises an
    // unchecked box its body never renders.
    let root = jest_project(
        "copy-paste",
        &[
            ("src/test-utils.tsx", "export const render = null;\n"),
            ("src/components/Checkbox.tsx", "export default null;\n"),
            (
                "src/components/__tests__/Checkbox/Checkbox.test.tsx",
                "import { expect, it } from '@jest/globals';\n\
                 \n\
                 import { render } from '../../../test-utils';\n\
                 import Checkbox from '../../Checkbox';\n\
                 \n\
                 it('renders checked Checkbox with color', async () => {\n\
                 \x20 const tree = (\n\
                 \x20   await render(<Checkbox status=\"checked\" color=\"red\" />)\n\
                 \x20 ).toJSON();\n\
                 \n\
                 \x20 expect(tree).toMatchSnapshot();\n\
                 });\n\
                 \n\
                 it('renders unchecked Checkbox with color', async () => {\n\
                 \x20 const tree = (\n\
                 \x20   await render(<Checkbox status=\"checked\" color=\"red\" />)\n\
                 \x20 ).toJSON();\n\
                 \n\
                 \x20 expect(tree).toMatchSnapshot();\n\
                 });\n",
            ),
        ],
    );
    let findings = duplicate_findings(&root, "check");
    assert_eq!(findings.len(), 1, "{findings:#?}");
    let explain = findings[0]["explain"].as_str().unwrap();
    assert!(
        explain.contains("if a title promises a case its body does not test"),
        "{explain}"
    );
    insta::assert_snapshot!("explain_copy_paste", explain);
}

/// The `WatermelonDB` `encodeQuery` suite as each adapter writes it (`lokijs/worker/encodeQuery/test.js:376`,
/// `sqlite/encodeQuery/test.js`): the same body over a file-level `encoded` helper bound to
/// that adapter's own `encodeQueryRaw`.
fn encode_query_suite(adapter: &str) -> String {
    format!(
        "import * as Q from '../../../QueryDescription'\n\
         import encodeQueryRaw from './index'\n\
         \n\
         const encoded = (clauses) => encodeQueryRaw(clauses)\n\
         \n\
         describe('{adapter} encodeQuery', () => {{\n\
         \x20 it(`fails to encode nested on without explicit joinTables`, () => {{\n\
         \x20   expect(() => encoded([Q.or(Q.on('projects', 'is_followed', true))])).toThrow(\n\
         \x20     'explicitly declare Q.experimentalJoinTables',\n\
         \x20   )\n\
         \x20 }})\n\
         }})\n"
    )
}

#[test]
fn the_same_body_over_a_file_level_helper_of_each_file_is_not_a_duplicate() {
    let lokijs = encode_query_suite("LokiJS");
    let sqlite = encode_query_suite("SQLite");
    let root = jest_project(
        "file-helper",
        &[
            ("src/QueryDescription/index.js", "export const or = null\n"),
            (
                "src/adapters/lokijs/encodeQuery/index.js",
                "export default null\n",
            ),
            (
                "src/adapters/sqlite/encodeQuery/index.js",
                "export default null\n",
            ),
            ("src/adapters/lokijs/encodeQuery/test.js", &lokijs),
            ("src/adapters/sqlite/encodeQuery/test.js", &sqlite),
        ],
    );
    let findings = duplicate_findings(&root, "check");
    assert!(findings.is_empty(), "{findings:#?}");
}

/// The decap `VisualEditor` spec as each widget package writes it
/// (`decap-cms-widget-markdown/…/VisualEditor.spec.js:7`, `decap-cms-widget-richtext/…`): one
/// text over `mergeMediaConfig` imported from `../VisualEditor`, its own package's module.
const VISUAL_EDITOR_SPEC: &str = "import { Map, fromJS } from 'immutable';\n\
     \n\
     import { mergeMediaConfig } from '../VisualEditor';\n\
     \n\
     describe('VisualEditor', () => {\n\
     \x20 describe('mergeMediaConfig', () => {\n\
     \x20   it('should copy editor media settings to image component', () => {\n\
     \x20     const editorComponents = Map({\n\
     \x20       image: { id: 'image', fields: fromJS([{ name: 'image', widget: 'image' }]) },\n\
     \x20     });\n\
     \x20     const field = fromJS({ media_library: { config: { max_file_size: 1234 } } });\n\
     \n\
     \x20     mergeMediaConfig(editorComponents, field);\n\
     \n\
     \x20     expect(editorComponents.get('image').fields).toEqual(\n\
     \x20       fromJS([{ name: 'image', widget: 'image', media_library: { config: { max_file_size: 1234 } } }]),\n\
     \x20     );\n\
     \x20   });\n\
     \x20 });\n\
     });\n";

#[test]
fn the_same_body_over_one_specifier_resolving_to_two_modules_is_not_a_duplicate() {
    let root = jest_project(
        "two-modules",
        &[
            (
                "packages/widget-markdown/src/MarkdownControl/VisualEditor.js",
                "export function mergeMediaConfig() {}\n",
            ),
            (
                "packages/widget-richtext/src/RichtextControl/VisualEditor.js",
                "export function mergeMediaConfig() {}\n",
            ),
            (
                "packages/widget-markdown/src/MarkdownControl/__tests__/VisualEditor.spec.js",
                VISUAL_EDITOR_SPEC,
            ),
            (
                "packages/widget-richtext/src/RichtextControl/__tests__/VisualEditor.spec.js",
                VISUAL_EDITOR_SPEC,
            ),
        ],
    );
    let findings = duplicate_findings(&root, "check");
    assert!(findings.is_empty(), "{findings:#?}");
}

#[test]
fn importing_the_runner_vocabulary_is_the_same_context_as_its_globals() {
    let root = jest_project(
        "runner-imports",
        &[
            (
                "src/parse.js",
                "export function parse(input) { return Number(input) }\n",
            ),
            (
                "src/alpha.test.js",
                "import { it, expect } from '@jest/globals'\n\
                 import { parse } from './parse'\n\
                 \n\
                 it('parses alpha input', () => {\n\
                 \x20 expect(parse('42')).toBe(42)\n\
                 })\n",
            ),
            (
                "src/beta.test.js",
                "import { parse } from './parse'\n\
                 \n\
                 it('parses beta input', () => {\n\
                 \x20 expect(parse('42')).toBe(42)\n\
                 })\n",
            ),
        ],
    );
    let findings = duplicate_findings(&root, "check");
    assert_eq!(findings.len(), 1, "{findings:#?}");
}

#[test]
fn the_same_relative_import_in_two_directories_is_not_a_duplicate() {
    // orpc `packages/client/src/adapters/fetch/index.test.ts:1` and its websocket twin.
    let body = "it('exports RPCLink', async () => {\n\
                \x20 await expect(import('.')).resolves.toHaveProperty('RPCLink')\n\
                })\n";
    let root = project("dup-relative-import");
    write(
        &root,
        "packages/client/src/adapters/fetch/index.ts",
        "export const RPCLink = 1\n",
    );
    write(
        &root,
        "packages/client/src/adapters/websocket/index.ts",
        "export const RPCLink = 2\n",
    );
    write(
        &root,
        "packages/client/src/adapters/fetch/index.test.ts",
        body,
    );
    write(
        &root,
        "packages/client/src/adapters/websocket/index.test.ts",
        body,
    );
    let findings = duplicate_findings(&root, "check");
    assert!(findings.is_empty(), "{findings:#?}");
}

#[test]
fn the_same_body_over_a_suite_instance_of_each_file_is_not_a_duplicate() {
    // ui-kitten `nativeDate.service.spec.ts:58` and `momentDate.service.spec.ts`: one text
    // over a describe-scoped `dateService` each file builds from its own service.
    let suite = |title: &str, service: &str, module: &str| {
        format!(
            "import {{ {service} }} from '{module}';\n\
             \n\
             describe('{title}', () => {{\n\
             \x20 let dateService;\n\
             \n\
             \x20 beforeEach(() => {{\n\
             \x20   dateService = new {service}();\n\
             \x20 }});\n\
             \n\
             \x20 it('* should validate as incorrect if date string is invalid according to the format', () => {{\n\
             \x20   const isValid = dateService.isValidDateString('23.04.2018', 'MM.DD.YYYY');\n\
             \x20   expect(isValid).toBeFalsy();\n\
             \x20 }});\n\
             }});\n"
        )
    };
    let native = suite(
        "@native-date: service checks",
        "NativeDateService",
        "./nativeDate.service",
    );
    let moment = suite(
        "@moment-date: service checks",
        "MomentDateService",
        "./momentDate.service",
    );
    let root = jest_project(
        "suite-instance",
        &[
            (
                "src/calendar/nativeDate.service.ts",
                "export class NativeDateService {}\n",
            ),
            (
                "src/moment/momentDate.service.ts",
                "export class MomentDateService {}\n",
            ),
            ("src/calendar/nativeDate.service.spec.ts", &native),
            ("src/moment/momentDate.service.spec.ts", &moment),
        ],
    );
    let findings = duplicate_findings(&root, "check");
    assert!(findings.is_empty(), "{findings:#?}");
}

#[test]
fn the_same_body_over_a_request_each_file_builds_is_not_a_duplicate() {
    // midway `packages/express-session/test/index.test.ts:59` and `packages/session/test/…:47`.
    let suite = "import { createLegacyApp, createHttpRequest, close } from '@midwayjs/mock';\n\
                 import { join } from 'path';\n\
                 \n\
                 describe('test/index.test.ts', function () {\n\
                 \x20 describe('should test cookie session', () => {\n\
                 \x20   let app;\n\
                 \x20   let request;\n\
                 \x20   beforeAll(async () => {\n\
                 \x20     app = await createLegacyApp(join(__dirname, 'fixtures/cookie-session'));\n\
                 \x20     request = createHttpRequest(app);\n\
                 \x20   })\n\
                 \n\
                 \x20   it('should ctx.session.key= change the session', async () => {\n\
                 \x20     await request\n\
                 \x20       .get('/setKey?key=bar')\n\
                 \x20       .expect(200)\n\
                 \x20       .expect({ key: 'bar', foo: 'bar' });\n\
                 \x20   });\n\
                 \x20 });\n\
                 });\n";
    let root = jest_project(
        "request-per-file",
        &[
            ("packages/express-session/test/index.test.ts", suite),
            ("packages/session/test/index.test.ts", suite),
        ],
    );
    let findings = duplicate_findings(&root, "check");
    assert!(findings.is_empty(), "{findings:#?}");
}

#[test]
fn the_same_assertion_after_two_different_before_each_hooks_is_not_a_duplicate() {
    // react-day-picker `examples/Keyboard.test.tsx:140` and `:187`.
    let root = jest_project(
        "two-hooks",
        &[
            ("examples/Keyboard.tsx", "export function Keyboard() { return null }\n"),
            (
                "examples/Keyboard.test.tsx",
                "import React from \"react\";\n\
                 \n\
                 import { activeElement, grid } from \"./elements\";\n\
                 import { render } from \"./render\";\n\
                 import { user } from \"./user\";\n\
                 import { Keyboard } from \"./Keyboard\";\n\
                 \n\
                 describe.each([\"ltr\", \"rtl\"])(\"when text direction is %s\", (dir: string) => {\n\
                 \x20 beforeEach(() => {\n\
                 \x20   render(<Keyboard mode=\"single\" dir={dir} />);\n\
                 \x20 });\n\
                 \x20 describe(\"when the first day is focused\", () => {\n\
                 \x20   describe(\"when the Shift + Arrow Up is pressed\", () => {\n\
                 \x20     beforeEach(() => user.type(activeElement(), \"{shift>}{arrowup}\"));\n\
                 \x20     test(\"should display the previous year\", () => {\n\
                 \x20       expect(grid(\"June 2021\")).toBeInTheDocument();\n\
                 \x20     });\n\
                 \x20   });\n\
                 \x20   describe(\"when Shift + Page Up is pressed\", () => {\n\
                 \x20     beforeEach(() => user.type(activeElement(), \"{shift>}{pageup}\"));\n\
                 \x20     test(\"should display the previous year\", () => {\n\
                 \x20       expect(grid(\"June 2021\")).toBeInTheDocument();\n\
                 \x20     });\n\
                 \x20   });\n\
                 \x20 });\n\
                 });\n",
            ),
            ("examples/elements.ts", "export const activeElement = null; export const grid = null;\n"),
            ("examples/render.ts", "export const render = null;\n"),
            ("examples/user.ts", "export const user = null;\n"),
        ],
    );
    let findings = duplicate_findings(&root, "check");
    assert!(findings.is_empty(), "{findings:#?}");
}

#[test]
fn the_same_body_over_a_const_of_each_describe_is_not_a_duplicate() {
    // instantsearch `Paginator-test.ts:109` and `:131`: each describe builds its own `pager`.
    let root = jest_project(
        "describe-const",
        &[
            (
                "src/pagination/Paginator.ts",
                "export default class Paginator {}\n",
            ),
            (
                "src/pagination/__tests__/Paginator-test.ts",
                "import Paginator from '../Paginator';\n\
                 \n\
                 describe('paginator: simple cases', () => {\n\
                 \x20 describe('on last page', () => {\n\
                 \x20   const pager = new Paginator({\n\
                 \x20     currentPage: 9,\n\
                 \x20     total: 10,\n\
                 \x20     padding: 2,\n\
                 \x20   });\n\
                 \n\
                 \x20   it('should be the last page', () => {\n\
                 \x20     expect(pager.isLastPage()).toBe(true);\n\
                 \x20   });\n\
                 \x20 });\n\
                 \n\
                 \x20 describe('past last page', () => {\n\
                 \x20   const pager = new Paginator({\n\
                 \x20     currentPage: 10,\n\
                 \x20     total: 10,\n\
                 \x20     padding: 2,\n\
                 \x20   });\n\
                 \n\
                 \x20   it('should be the last page', () => {\n\
                 \x20     expect(pager.isLastPage()).toBe(true);\n\
                 \x20   });\n\
                 \x20 });\n\
                 });\n",
            ),
        ],
    );
    let findings = duplicate_findings(&root, "check");
    assert!(findings.is_empty(), "{findings:#?}");
}

#[test]
fn a_callback_parameter_of_the_same_name_leaves_the_describe_const_free() {
    // The instantsearch shape, with an inner `pager` parameter beside the suite's `pager`:
    // the body still reads each describe's own `pager` in its `expect`.
    let root = jest_project(
        "describe-const-shadowed",
        &[(
            "src/shadow.test.js",
            "describe('a', () => {\n\
             \x20 const pager = { last: () => true }\n\
             \x20 it('last', () => {\n\
             \x20   expect(pager.last()).toBe(true)\n\
             \x20   ;[1].forEach((pager) => pager)\n\
             \x20 })\n\
             })\n\
             describe('b', () => {\n\
             \x20 const pager = { last: () => false }\n\
             \x20 it('last', () => {\n\
             \x20   expect(pager.last()).toBe(true)\n\
             \x20   ;[1].forEach((pager) => pager)\n\
             \x20 })\n\
             })\n",
        )],
    );
    let findings = duplicate_findings(&root, "check");
    assert!(findings.is_empty(), "{findings:#?}");
}

#[test]
fn the_same_body_over_a_reducer_of_each_describe_is_not_a_duplicate() {
    // boardgame.io `src/core/reducer.test.ts:349` and `:385`.
    let root = jest_project(
        "describe-reducer",
        &[
            (
                "src/core/reducer.ts",
                "export function CreateGameReducer() {}\nexport function InitializeGame() {}\n",
            ),
            (
                "src/core/reducer.test.ts",
                "import { CreateGameReducer, InitializeGame } from './reducer';\n\
                 \n\
                 describe('Plugin Invalid Action API', () => {\n\
                 \x20 const game = {};\n\
                 \n\
                 \x20 let state;\n\
                 \x20 beforeEach(() => {\n\
                 \x20   state = InitializeGame({ game });\n\
                 \x20 });\n\
                 \n\
                 \x20 describe('multiplayer client', () => {\n\
                 \x20   const reducer = CreateGameReducer({ game });\n\
                 \n\
                 \x20   test('move is processed if no plugin declares it invalid', () => {\n\
                 \x20     state = reducer(state, makeMove('setValue', [15], '0'));\n\
                 \x20     expect(state.G).toMatchObject({ value: 15 });\n\
                 \x20     expect(state['transients']).toBeUndefined();\n\
                 \x20   });\n\
                 \x20 });\n\
                 \n\
                 \x20 describe('local client', () => {\n\
                 \x20   const reducer = CreateGameReducer({ game, isClient: true });\n\
                 \n\
                 \x20   test('move is processed if no plugin declares it invalid', () => {\n\
                 \x20     state = reducer(state, makeMove('setValue', [15], '0'));\n\
                 \x20     expect(state.G).toMatchObject({ value: 15 });\n\
                 \x20     expect(state['transients']).toBeUndefined();\n\
                 \x20   });\n\
                 \x20 });\n\
                 });\n",
            ),
        ],
    );
    let findings = duplicate_findings(&root, "check");
    assert!(findings.is_empty(), "{findings:#?}");
}

#[test]
fn identical_bodies_in_two_hookless_sibling_describes_are_duplicates() {
    // expo `packages/@expo/require-utils/src/__tests__/stacktrace-test.ts:80` and `:597`.
    let root = jest_project(
        "hookless-siblings",
        &[
            (
                "src/stacktrace.ts",
                "export function installSourceMapStackTrace() {}\n",
            ),
            (
                "src/__tests__/stacktrace-test.ts",
                "import { installSourceMapStackTrace } from '../stacktrace';\n\
                 \n\
                 installSourceMapStackTrace();\n\
                 const prepareStackTrace = Error.prepareStackTrace as (\n\
                 \x20 error: unknown,\n\
                 \x20 callSites: NodeJS.CallSite[]\n\
                 ) => string;\n\
                 \n\
                 describe('error header', () => {\n\
                 \x20 it('formats Error with name and message', () => {\n\
                 \x20   expect(prepareStackTrace(new Error('boom'), [])).toMatchInlineSnapshot(`\"Error: boom\"`);\n\
                 \x20 });\n\
                 });\n\
                 \n\
                 describe('empty stack', () => {\n\
                 \x20 it('returns just the error header', () => {\n\
                 \x20   expect(prepareStackTrace(new Error('boom'), [])).toMatchInlineSnapshot(`\"Error: boom\"`);\n\
                 \x20 });\n\
                 });\n",
            ),
        ],
    );
    let findings = duplicate_findings(&root, "check");
    assert_eq!(findings.len(), 1, "{findings:#?}");
}

#[test]
fn identical_bodies_under_one_shared_before_each_are_duplicates() {
    // react-admin `ra-router-tanstack/src/tanStackRouterProvider.spec.tsx:531` and `:758`.
    let root = jest_project(
        "shared-hook",
        &[
            (
                "src/tanStackRouterProvider.stories.tsx",
                "export const BasicStandalone = () => null;\n",
            ),
            (
                "src/tanStackRouterProvider.spec.tsx",
                "import * as React from 'react';\n\
                 import { render, screen, waitFor, cleanup } from '@testing-library/react';\n\
                 import { BasicStandalone } from './tanStackRouterProvider.stories';\n\
                 \n\
                 describe('tanStackRouterProvider', () => {\n\
                 \x20   // Reset hash before each test to ensure clean state\n\
                 \x20   beforeEach(() => {\n\
                 \x20       window.location.hash = '';\n\
                 \x20   });\n\
                 \x20   afterEach(() => {\n\
                 \x20       cleanup();\n\
                 \x20   });\n\
                 \x20   describe('RouterWrapper', () => {\n\
                 \x20       describe('standalone mode', () => {\n\
                 \x20           it('should render the post list', async () => {\n\
                 \x20               render(<BasicStandalone />);\n\
                 \x20               await waitFor(() => {\n\
                 \x20                   expect(screen.getByText('Posts')).toBeInTheDocument();\n\
                 \x20               });\n\
                 \x20           });\n\
                 \x20       });\n\
                 \x20   });\n\
                 \x20   describe('Routes', () => {\n\
                 \x20       describe('resource routes', () => {\n\
                 \x20           it('should match list routes', async () => {\n\
                 \x20               render(<BasicStandalone />);\n\
                 \x20               await waitFor(() => {\n\
                 \x20                   expect(screen.getByText('Posts')).toBeInTheDocument();\n\
                 \x20               });\n\
                 \x20           });\n\
                 \x20       });\n\
                 \x20   });\n\
                 });\n",
            ),
        ],
    );
    let findings = duplicate_findings(&root, "check");
    assert_eq!(findings.len(), 1, "{findings:#?}");
}

#[test]
fn an_import_and_a_require_of_one_module_are_the_same_context() {
    // NotionNext `__tests__/lib/db/notion/getPostBlocks.test.js:33` imports the function;
    // `__tests__/lib/notion-data-format.test.js:1421` requires it in a describe.
    let body = "it('converts video → embed for Apple Music song URLs', () => {\n\
                \x20 const block = {\n\
                \x20   type: 'video',\n\
                \x20   properties: {\n\
                \x20     source: [['https://embed.music.apple.com/us/song/neon-blue/324357768']]\n\
                \x20   }\n\
                \x20 }\n\
                \x20 normalizeExternalMediaBlock(block)\n\
                \x20 expect(block.type).toBe('embed')\n\
                })\n";
    let imported = format!(
        "import {{ normalizeExternalMediaBlock }} from '../lib/normalizeExternalMediaBlock'\n\
         \n\
         describe('formatNotionBlock', () => {{\n{body}}})\n"
    );
    let required = format!(
        "describe('normalizeExternalMediaBlock — Apple Music song embeds', () => {{\n\
         \x20 const {{ normalizeExternalMediaBlock, isAppleMusicEmbedUrl }} =\n\
         \x20   require('../lib/normalizeExternalMediaBlock')\n\
         \n\
         {body}}})\n"
    );
    let root = jest_project(
        "import-and-require",
        &[
            (
                "lib/normalizeExternalMediaBlock.js",
                "export function normalizeExternalMediaBlock() {}\n",
            ),
            ("__tests__/getPostBlocks.test.js", &imported),
            ("__tests__/notion-data-format.test.js", &required),
        ],
    );
    let findings = duplicate_findings(&root, "check");
    assert_eq!(findings.len(), 1, "{findings:#?}");
}

// ------------------------------------------------- partners under one heading (#546)

/// A small codec suite with two clone pairs that interleave: `:10` with `:30` and `:16`
/// with `:26`. The human view listed `:10` and `:16` under one `codec.test.ts (2)`
/// heading without their partners, and that read as one pair.
fn partners(name: &str) -> PathBuf {
    const FIXTURE: &str = "tests/fixtures/dup-partners";
    let root = project(name);
    for path in [
        "package.json",
        "src/errors.ts",
        "src/shapes/codec.ts",
        "src/shapes/codec.test.ts",
    ] {
        let from = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(FIXTURE)
            .join(path);
        write(&root, path, &std::fs::read_to_string(from).unwrap());
    }
    root
}

/// Each finding's location line and its related lines, sorted.
fn pairs(findings: &[serde_json::Value]) -> Vec<(u64, Vec<u64>)> {
    let mut pairs: Vec<(u64, Vec<u64>)> = findings
        .iter()
        .map(|finding| {
            let related = finding["related"]
                .as_array()
                .unwrap()
                .iter()
                .map(|related| related["line"].as_u64().unwrap())
                .collect();
            (finding["line"].as_u64().unwrap(), related)
        })
        .collect();
    pairs.sort();
    pairs
}

#[test]
fn tests_with_different_subjects_inputs_and_matchers_do_not_pair() {
    // `:10` calls `decodeShape` and `toThrowError`; `:16` calls `decodeShapes("[]")`
    // and `toEqual([])`. Each has a byte-identical copy further down (`:30`, `:26`),
    // and those are the two pairs.
    let findings = duplicate_findings(&partners("partners-pairs"), "check");
    assert_eq!(
        pairs(&findings),
        vec![(10, vec![30]), (16, vec![26])],
        "{findings:#?}"
    );
    // The adjudicator reads the primary title by removing this suffix. Related
    // locations already carry every partner in the structured report.
    assert!(findings.iter().all(|finding| finding["message"]
        .as_str()
        .unwrap()
        .ends_with(" duplicates 1 other test")));
}

#[test]
fn the_human_view_names_where_each_other_member_is() {
    let output = spirolo()
        .args(["check", "--all", "--rule", "duplicate-test"])
        .arg(partners("partners-human"))
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains(
            ":10 decodeShape › rejects a shape it has never heard of duplicates 1 other test, at :30"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(":16 decodeShapes › reads an empty array duplicates 1 other test, at :26"),
        "{stdout}"
    );
}
