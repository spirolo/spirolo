//! `spirolo list` end to end: what a user (and an agent) gets from the binary itself.

use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::*;

fn spirolo() -> Command {
    Command::cargo_bin("spirolo").unwrap()
}

// Config and two unmodified tests from the dev corpus's NextChat pin (truth.json).
// Jest injects their globals; neither test imports @jest/globals.
#[test]
fn list_attributes_injected_globals_to_the_configured_runner() {
    let project =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/discovery-nextchat");
    let output = spirolo()
        .args(["list", "--format", "json"])
        .arg(project)
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let cases = json["test_cases"].as_array().unwrap();
    assert_eq!(cases.iter().filter(|row| row["kind"] == "test").count(), 8);
    assert!(cases.iter().all(|row| row["framework"] == "jest"));
}

// Unmodified Next.js Jest wrapper and tests from NotionNext's dev pin.
#[test]
fn list_resolves_jest_rootdir_tokens_in_match_and_ignore_patterns() {
    let project =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/discovery-rootdir");
    let output = spirolo()
        .args(["list", "--format", "json"])
        .arg(project)
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let files: std::collections::BTreeSet<_> = json["test_cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["kind"] == "test" && row["framework"] == "jest")
        .map(|row| row["file"].as_str().unwrap())
        .collect();
    assert_eq!(
        files,
        std::collections::BTreeSet::from([
            "__tests__/conf/themeSwitch.xuhome.test.js",
            "__tests__/lib/cache/cache_key_path.test.js",
        ])
    );
}

// UI Kitten's <rootDir>/docs ignore must not hide a nested docs.spec.ts.
#[test]
fn list_keeps_runner_listed_files_named_like_a_root_ignore() {
    let project =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/discovery-root-ignore");
    let listed: Vec<String> =
        serde_json::from_str(&std::fs::read_to_string(project.join("runner-list.json")).unwrap())
            .unwrap();
    let expected: Vec<_> = listed
        .iter()
        .filter(|file| project.join(file).is_file())
        .collect();
    assert_eq!(expected.len(), 1);
    let output = spirolo()
        .args(["list", "--format", "json"])
        .arg(&project)
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let files: Vec<_> = json["test_files"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["framework"] == "jest")
        .map(|row| row["file"].as_str().unwrap())
        .collect();
    assert_eq!(files, expected);
}

fn jest_files(project: &std::path::Path) -> Vec<String> {
    let output = spirolo()
        .args(["list", "--format", "json"])
        .arg(project)
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json["test_cases"]
        .as_array()
        .unwrap()
        .iter()
        .all(|row| row["framework"] == "jest"));
    json["test_files"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["framework"] == "jest")
        .map(|row| row["file"].as_str().unwrap().to_owned())
        .collect()
}

fn copy_tree(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

// Midway's `npm test` is `lerna run test`: each package runs Jest with its own config, and
// packages/web ignores <rootDir>/test/fixtures. The real run collects neither hackernews
// test, although a root `jest --listTests` (runner-list.json) lists both.
#[test]
fn list_follows_package_configs_when_the_root_only_delegates() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/discovery-jest-root-defaults");
    let listed: Vec<String> =
        serde_json::from_str(&std::fs::read_to_string(project.join("runner-list.json")).unwrap())
            .unwrap();
    let root_listed = listed
        .iter()
        .filter(|file| project.join(file).is_file())
        .count();
    assert_eq!(root_listed, 2);
    assert_eq!(jest_files(&project), Vec::<String>::new());
}

// The same tree with a root script that runs Jest: that invocation uses defaults, so the
// nested web config cannot hide the hackernews tests from it.
#[test]
fn list_keeps_root_runner_files_excluded_by_a_nested_config() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/discovery-jest-root-defaults");
    let project = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("list-jest-root-script");
    let _ = std::fs::remove_dir_all(&project);
    copy_tree(&fixture, &project);
    let manifest = project.join("package.json");
    let mut package: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&manifest).unwrap()).unwrap();
    package["scripts"]["test"] = "jest".into();
    std::fs::write(&manifest, package.to_string()).unwrap();
    let listed: Vec<String> =
        serde_json::from_str(&std::fs::read_to_string(project.join("runner-list.json")).unwrap())
            .unwrap();
    let expected: Vec<_> = listed
        .into_iter()
        .filter(|file| project.join(file).is_file())
        .collect();
    assert_eq!(expected.len(), 2);
    assert_eq!(jest_files(&project), expected);
}

// React's test script selects its source config through a Node wrapper.
#[test]
fn list_uses_the_config_selected_by_a_package_runner_wrapper() {
    let project =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/discovery-jest-wrapper");
    let listed: Vec<String> =
        serde_json::from_str(&std::fs::read_to_string(project.join("runner-list.json")).unwrap())
            .unwrap();
    let expected: Vec<_> = listed
        .iter()
        .filter(|file| project.join(file).is_file())
        .collect();
    assert_eq!(expected.len(), 2);
    let output = spirolo()
        .args(["list", "--format", "json"])
        .arg(&project)
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let files: Vec<_> = json["test_files"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["framework"] == "jest")
        .map(|row| row["file"].as_str().unwrap())
        .collect();
    assert_eq!(files, expected);
    assert!(json["test_cases"]
        .as_array()
        .unwrap()
        .iter()
        .all(|row| row["framework"] == "jest"));
    let coffee = "packages/react/src/__tests__/ReactCoffeeScriptClass-test.coffee";
    assert!(json["test_cases"]
        .as_array()
        .unwrap()
        .iter()
        .all(|row| row["file"] != coffee));
    assert!(json["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["file"] == coffee && row["code"] == "language_unsupported"));
}

// MUI keeps JSX in .js, as Babel/Vitest allow. These are its unmodified dev tests.
#[test]
fn list_recognises_jsx_in_javascript_test_files() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/discovery-jsx");
    let output = spirolo()
        .args(["list", "--format", "json"])
        .arg(project)
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let files: std::collections::BTreeSet<_> = json["test_cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["kind"] == "test" && row["framework"] == "vitest")
        .map(|row| row["file"].as_str().unwrap())
        .collect();
    assert_eq!(
        files,
        std::collections::BTreeSet::from([
            "packages/mui-lab/src/TimelineContent/TimelineContent.test.js",
            "packages/mui-lab/src/TimelineDot/TimelineDot.test.js",
        ])
    );
}

// Mantine's shared tests generate cases when Jest loads the file. File discovery
// must still list these files without inventing any static test-case rows.
#[test]
fn list_inventories_collected_files_independently_of_recognised_cases() {
    let project =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/discovery-generated");
    let output = spirolo()
        .args(["list", "--format", "json"])
        .arg(project)
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        json["test_files"],
        serde_json::json!([
            {"file": "packages/@mantine/carousel/src/CarouselSlide/CarouselSlide.test.tsx", "framework": "jest"},
            {"file": "packages/@mantine/charts/src/AreaChart/AreaChart.test.tsx", "framework": "jest"},
        ])
    );
    assert!(!json["test_cases"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["kind"] == "test"));
}

// Metro's testRegex collects -test.js, but not helpers under __tests__.
#[test]
fn list_uses_jest_testregex_instead_of_default_testmatch() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/discovery-regex");
    let output = spirolo()
        .args(["list", "--format", "json"])
        .arg(project)
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        json["test_files"],
        serde_json::json!([
            {"file": "packages/metro-babel-register/src/__tests__/parses-without-transpilation-test.js", "framework": "jest"},
        ])
    );
}

// Nivo installs Jest at the workspace root, and its package tests use globals.
#[test]
fn list_inherits_the_package_runner_when_no_config_governs_the_file() {
    let project =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/discovery-package-runner");
    let output = spirolo()
        .args(["list", "--format", "json"])
        .arg(project)
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        json["test_files"],
        serde_json::json!([
            {"file": "packages/arcs/tests/boundingBox.test.ts", "framework": "jest"},
        ])
    );
    assert!(json["test_cases"]
        .as_array()
        .unwrap()
        .iter()
        .all(|row| row["framework"] == "jest"));
}

// react-spring's inline projects collect both unit tests and type tests.
#[test]
fn list_unions_inline_vitest_projects_including_type_tests() {
    let project =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/discovery-projects");
    let output = spirolo()
        .args(["list", "--format", "json"])
        .arg(project)
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        json["test_files"],
        serde_json::json!([
            {"file": "packages/core/src/components/Spring.test-d.tsx", "framework": "vitest"},
            {"file": "packages/core/src/helpers.test.ts", "framework": "vitest"},
        ])
    );
    assert_eq!(
        json["test_cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["kind"] == "test"
                && row["file"] == "packages/core/src/components/Spring.test-d.tsx")
            .count(),
        2
    );
}

// WatermelonDB uses micromatch's bare (spec|test) alternation after an extglob.
#[test]
fn list_expands_bare_glob_groups_as_runner_alternatives() {
    let project =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/discovery-glob-groups");
    let output = spirolo()
        .args(["list", "--format", "json"])
        .arg(project)
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        json["test_files"],
        serde_json::json!([
            {"file": "src/Collection/test.js", "framework": "jest"},
            {"file": "src/QueryDescription/__tests__/queryWithoutDeleted.test.js", "framework": "jest"},
        ])
    );
}

// react-native-web stores the config in configs/, but sets rootDir to cwd.
#[test]
fn list_scopes_external_configs_from_their_explicit_rootdir() {
    let project =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/discovery-config-root");
    let output = spirolo()
        .args(["list", "--format", "json"])
        .arg(project)
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        json["test_files"],
        serde_json::json!([
            {"file": "packages/react-native-web/src/exports/AppRegistry/__tests__/index-test.js", "framework": "jest"},
        ])
    );
}

// DayPicker's example project excludes timezone tests; the other projects'
// roots must not collect them through their defaults.
#[test]
fn list_keeps_jest_project_roots_and_ignores_separate() {
    let project =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/discovery-project-roots");
    let output = spirolo()
        .args(["list", "--format", "json"])
        .arg(project)
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        json["test_files"],
        serde_json::json!([
            {"file": "examples/AsiaSaigonTimezone.test.tsx", "framework": "jest"},
        ])
    );
    assert!(!json["test_cases"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["file"] == "examples/timezone/TestCase2833.test.tsx"));
}

// Radix's genuine Playwright tests are absent from the onboarding Vitest listing.
#[test]
fn list_preserves_playwright_file_attribution_in_the_inventory() {
    let project =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/discovery-playwright");
    let output = spirolo()
        .args(["list", "--format", "json"])
        .arg(project)
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        json["test_files"],
        serde_json::json!([
            {"file": "e2e/toast.spec.ts", "framework": "playwright"},
            {"file": "e2e/toggle-group.spec.ts", "framework": "playwright"},
        ])
    );
}

#[test]
fn list_uses_explicit_runner_imports_for_files_without_a_config() {
    let project =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/discovery-explicit-import");
    let output = spirolo()
        .args(["list", "--format", "json"])
        .arg(project)
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        json["test_files"],
        serde_json::json!([
            {"file": "test/vision-model-checker.test.ts", "framework": "jest"},
        ])
    );
}

// Kaneo's API package explicitly collects ../../tests/api outside workspace members.
#[test]
fn list_follows_explicit_collection_globs_outside_the_config_package() {
    let project =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/discovery-workspace-reach");
    let output = spirolo()
        .args(["list", "--format", "json"])
        .arg(project)
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        json["test_files"],
        serde_json::json!([
            {"file": "tests/api/column/to-slug.test.ts", "framework": "vitest"},
            {"file": "tests/api/mcp-internal-api-url.test.ts", "framework": "vitest"},
        ])
    );
}

#[test]
fn list_recognises_vite_plus_test_vocabulary_as_vitest() {
    let project =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/discovery-workspace-reach");
    let output = spirolo()
        .args(["list", "--format", "json"])
        .arg(project)
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let cases: Vec<_> = json["test_cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["kind"] == "test" && row["file"] == "tests/api/column/to-slug.test.ts")
        .collect();
    assert_eq!(cases.len(), 7);
    assert!(cases.iter().all(|row| row["framework"] == "vitest"));
}

// Jest 30 lists these worklets .mts files, even though their bodies use node:test.
// Inventory the runner-selected files without inventing Jest cases.
#[test]
fn list_collects_modern_jest_module_extensions_without_fabricating_cases() {
    let project =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/discovery-jest-extensions");
    let output = spirolo()
        .args(["list", "--format", "json"])
        .arg(project)
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        json["test_files"],
        serde_json::json!([
            {"file": "packages/react-native-worklets/plugin-oxc/test/babel-shim.test.mts", "framework": "jest"},
            {"file": "packages/react-native-worklets/plugin-oxc/test/napi.test.mts", "framework": "jest"},
        ])
    );
    assert!(json["test_cases"].as_array().unwrap().is_empty());
}

/// A directory with nothing in it, named after the test that asked for one.
fn empty_project(name: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn list_on_an_empty_directory_summarises_and_exits_clean() {
    let project = empty_project("summarise");

    spirolo()
        .arg("list")
        .arg(&project)
        .assert()
        .success()
        .stdout(
            predicate::str::starts_with(
                "0 files · 0 test files · 0 tests · 0 expects · 0 handlers · 0 configs · ",
            )
            .and(predicate::str::ends_with("ms\n")),
        );
}

#[test]
fn list_defaults_to_the_current_directory() {
    let project = empty_project("default-cwd");

    spirolo()
        .arg("list")
        .current_dir(&project)
        .assert()
        .success()
        .stdout(predicate::str::starts_with(
            "0 files · 0 test files · 0 tests",
        ));
}

#[test]
fn list_as_json_emits_the_versioned_schema() {
    let project = empty_project("json");

    let output = spirolo()
        .args(["list", "--format", "json"])
        .arg(&project)
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["schema_version"], serde_json::json!(2));
    assert_eq!(json["test_cases"], serde_json::json!([]));
}

#[test]
fn list_as_agent_prints_nothing_when_there_is_nothing_to_say() {
    let project = empty_project("agent");

    spirolo()
        .args(["list", "--agent"])
        .arg(&project)
        .assert()
        .success()
        .stdout("");
}

#[test]
fn list_reports_a_missing_directory_as_an_error() {
    spirolo()
        .args(["list", "no-such-directory"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("no-such-directory"));
}

#[test]
fn the_agent_flag_is_shorthand_for_the_agent_format() {
    let project = empty_project("agent-format");

    spirolo()
        .args(["list", "--format", "agent"])
        .arg(&project)
        .assert()
        .success()
        .stdout("");
}

#[test]
fn list_counts_the_files_of_a_real_project_and_lists_them() {
    let project = empty_project("file-counts");
    std::fs::create_dir_all(project.join("src")).unwrap();
    std::fs::write(project.join("src/app.ts"), "export const app = 1;\n").unwrap();
    std::fs::write(
        project.join("src/app.test.ts"),
        "import { app } from \"./app\";\n",
    )
    .unwrap();
    std::fs::write(project.join("README.md"), "not source\n").unwrap();

    spirolo()
        .arg("list")
        .arg(&project)
        .assert()
        .success()
        .stdout(
            predicate::str::starts_with("2 files · 1 test files · 0 tests")
                .and(predicate::str::contains(
                    "\nmodules\n  src/app.test.ts  typescript",
                ))
                .and(predicate::str::contains("\n  src/app.ts  typescript")),
        );
}

#[test]
fn list_counts_the_test_cases_a_suite_declares() {
    let project = empty_project("test-counts");
    std::fs::create_dir_all(project.join("src")).unwrap();
    std::fs::write(
        project.join("src/sum.test.ts"),
        "import { describe, it } from 'vitest'\n\
         describe('sum', () => {\n  it('adds', () => {})\n  it('subtracts', () => {})\n})\n",
    )
    .unwrap();

    spirolo()
        .arg("list")
        .arg(&project)
        .assert()
        .success()
        .stdout(predicate::str::starts_with(
            "1 files · 1 test files · 2 tests",
        ));
}

#[test]
fn list_summary_counts_the_expects_a_suite_declares() {
    let project = empty_project("expect-counts");
    std::fs::create_dir_all(project.join("src")).unwrap();
    std::fs::write(
        project.join("src/sum.test.ts"),
        "import { it, expect } from 'vitest'\n\
         it('adds', () => {\n  expect(add(1, 2)).toBe(3)\n  expect(add(2, 2)).toBe(4)\n})\n\
         it('subtracts', () => {\n  expect(sub(3, 1)).toBe(2)\n})\n",
    )
    .unwrap();

    spirolo()
        .arg("list")
        .arg(&project)
        .assert()
        .success()
        .stdout(predicate::str::starts_with(
            "1 files · 1 test files · 2 tests (2 vitest) · 3 expects",
        ));
}

#[test]
fn list_summary_counts_vitest_and_playwright_frameworks_apart() {
    let project = empty_project("framework-split");
    std::fs::create_dir_all(project.join("e2e")).unwrap();
    std::fs::write(
        project.join("src.test.ts"),
        "import { test } from 'vitest'\ntest('a unit test', () => {})\n",
    )
    .unwrap();
    std::fs::write(
        project.join("e2e/home.spec.ts"),
        "import { test } from '@playwright/test'\n\
         test('opens', async () => {})\ntest('navigates', async () => {})\n",
    )
    .unwrap();

    spirolo()
        .arg("list")
        .arg(&project)
        .assert()
        .success()
        .stdout(predicate::str::starts_with(
            "2 files · 2 test files · 3 tests (2 playwright, 1 vitest)",
        ));
}

#[test]
fn list_as_json_carries_the_recognised_test_cases() {
    let project = empty_project("test-json");
    std::fs::create_dir_all(project.join("src")).unwrap();
    std::fs::write(
        project.join("src/sum.test.ts"),
        "import { test } from 'vitest'\ntest('adds', () => {})\n",
    )
    .unwrap();

    let output = spirolo()
        .args(["list", "--format", "json"])
        .arg(&project)
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let cases = json["test_cases"].as_array().unwrap();
    assert_eq!(cases.len(), 1);
    assert_eq!(cases[0]["title"], serde_json::json!("adds"));
    assert_eq!(cases[0]["kind"], serde_json::json!("test"));
    assert_eq!(cases[0]["framework"], serde_json::json!("vitest"));
}

#[test]
fn list_collects_msw_handlers_from_a_mock_file() {
    let project = empty_project("handlers");
    std::fs::create_dir_all(project.join("src/mocks")).unwrap();
    std::fs::write(
        project.join("src/mocks/handlers.ts"),
        "import { http, HttpResponse } from 'msw'\n\
         export const handlers = [\n\
         \x20 http.get('/api/user', () => HttpResponse.json({})),\n\
         \x20 http.post('/api/user', () => HttpResponse.json({})),\n\
         ]\n",
    )
    .unwrap();

    spirolo()
        .arg("list")
        .arg(&project)
        .assert()
        .success()
        .stdout(
            predicate::str::contains("· 2 handlers ·")
                .and(predicate::str::contains(
                    "\nhandlers\n  src/mocks/handlers.ts:3  GET  /api/user",
                ))
                .and(predicate::str::contains("  POST  /api/user")),
        );
}

#[test]
fn list_reports_a_file_that_does_not_parse_as_a_diagnostic_and_still_counts_it() {
    let project = empty_project("broken-counts");
    std::fs::write(project.join("ok.js"), "module.exports = 1;\n").unwrap();
    std::fs::write(project.join("broken.js"), "function (\n").unwrap();

    spirolo()
        .arg("list")
        .arg(&project)
        .assert()
        .success()
        .stdout(
            predicate::str::starts_with("2 files · ")
                .and(predicate::str::contains(
                    "\ndiagnostics\n  error broken.js:1 ",
                ))
                .and(predicate::str::contains("[parse_failed]\n")),
        );
}

#[test]
fn list_summary_counts_a_config_and_lists_the_gate() {
    let project = empty_project("config-count");
    std::fs::write(
        project.join("vitest.config.ts"),
        "import { defineConfig } from 'vitest/config'\n\
         export default defineConfig({ test: { setupFiles: ['./setup.ts'] } })\n",
    )
    .unwrap();

    spirolo()
        .arg("list")
        .arg(&project)
        .assert()
        .success()
        .stdout(
            predicate::str::contains("· 1 configs ·").and(predicate::str::contains(
                "\ngates\n  vitest.config.ts  vitest",
            )),
        );
}

#[test]
fn list_reads_the_config_gate_into_json() {
    let project = empty_project("config-json");
    std::fs::write(
        project.join("stryker.conf.js"),
        "module.exports = { mutate: ['src/**/*.ts'], testRunner: 'vitest', thresholds: { break: 60 } }\n",
    )
    .unwrap();

    let output = spirolo()
        .args(["list", "--format", "json"])
        .arg(&project)
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let gates = json["gates"].as_array().unwrap();
    assert_eq!(gates.len(), 1);
    assert_eq!(gates[0]["source"], serde_json::json!("stryker.conf.js"));
    assert_eq!(gates[0]["tool"], serde_json::json!("stryker"));
    assert_eq!(gates[0]["test_runner"], serde_json::json!("vitest"));
    assert_eq!(gates[0]["mutation_break"], serde_json::json!(60.0));
    assert_eq!(gates[0]["mutate"], serde_json::json!(["src/**/*.ts"]));
}

#[test]
fn a_vitest_include_glob_makes_a_non_matching_file_a_module_but_not_a_test() {
    let project = empty_project("include-scope");
    std::fs::create_dir_all(project.join("src")).unwrap();
    std::fs::write(
        project.join("vitest.config.ts"),
        "export default { test: { include: ['**/*.unit.ts'] } }\n",
    )
    .unwrap();
    std::fs::write(
        project.join("src/math.unit.ts"),
        "import { test, expect } from 'vitest'\ntest('adds', () => { expect(1).toBe(1) })\n",
    )
    .unwrap();
    // Looks like a test, but the include glob only collects `*.unit.ts`.
    std::fs::write(
        project.join("src/math.test.ts"),
        "import { test, expect } from 'vitest'\ntest('excluded', () => { expect(1).toBe(1) })\n",
    )
    .unwrap();

    let output = spirolo()
        .args(["list", "--format", "json"])
        .arg(&project)
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    let modules: Vec<&str> = json["modules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["path"].as_str().unwrap())
        .collect();
    assert!(
        modules.contains(&"src/math.test.ts"),
        "the non-matching file is still a module: {modules:?}"
    );

    let test_files: Vec<&str> = json["test_cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["file"].as_str().unwrap())
        .collect();
    assert_eq!(
        test_files,
        ["src/math.unit.ts"],
        "only the included file is a test"
    );
}
