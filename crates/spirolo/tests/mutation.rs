//! `spirolo list` end to end with mutation reports (Seam 2): the binary reads the reports
//! `inputs.mutation` names, joins their tests to the model and shows the `Mutants` table,
//! with a diagnostic for every report the rule cannot use.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::Value;

fn spirolo() -> Command {
    Command::cargo_bin("spirolo").unwrap()
}

/// Write `contents` to `root/relative`, creating parent directories.
fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

/// A fixture with three reports at Stryker's default path under three packages: a whole
/// per-test report whose one test joins the model, a bailed report, and a report with no
/// per-test data. Returns the project root.
fn fixture(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();

    // Package a: a real test the good report names, so its test row joins.
    write(
        &root,
        "packages/a/src/add.ts",
        "export const add = (a, b) => a + b\n",
    );
    write(
        &root,
        "packages/a/src/add.test.ts",
        "import { describe, it, expect } from 'vitest'\n\
         describe('add', () => {\n  it('adds two numbers', () => { expect(add(1, 2)).toBe(3) })\n})\n",
    );
    write(
        &root,
        "packages/a/reports/mutation/mutation.json",
        &good_report(),
    );
    write(
        &root,
        "packages/b/reports/mutation/mutation.json",
        &bailed_report(),
    );
    write(
        &root,
        "packages/c/reports/mutation/mutation.json",
        &no_per_test_report(),
    );
    root
}

fn good_report() -> String {
    r#"{
  "schemaVersion": "1.0",
  "framework": {"name": "StrykerJS", "version": "9.6.1"},
  "config": {"disableBail": true},
  "files": {
    "src/add.ts": {
      "source": "export const add = (a, b) => a + b\n",
      "mutants": [
        {"id": "0", "mutatorName": "ArithmeticOperator",
         "location": {"start": {"line": 1, "column": 30}, "end": {"line": 1, "column": 35}},
         "status": "Killed", "coveredBy": ["t0"], "killedBy": ["t0"]}
      ]
    }
  },
  "testFiles": {
    "src/add.test.ts": {
      "tests": [{"id": "t0", "name": "add adds two numbers"}]
    }
  }
}"#
    .to_owned()
}

fn bailed_report() -> String {
    r#"{
  "schemaVersion": "1.0",
  "framework": {"name": "StrykerJS", "version": "9.6.1"},
  "config": {"disableBail": false},
  "files": {
    "src/sub.ts": {
      "source": "export const sub = (a, b) => a - b\n",
      "mutants": [
        {"id": "0", "mutatorName": "ArithmeticOperator",
         "location": {"start": {"line": 1, "column": 30}, "end": {"line": 1, "column": 35}},
         "status": "Killed", "coveredBy": ["t0", "t1"], "killedBy": ["t0"]}
      ]
    }
  },
  "testFiles": {
    "src/sub.test.ts": {
      "tests": [{"id": "t0", "name": "sub > subtracts"}, {"id": "t1", "name": "sub > by zero"}]
    }
  }
}"#
    .to_owned()
}

fn no_per_test_report() -> String {
    r#"{
  "schemaVersion": "1.0",
  "framework": {"name": "StrykerJS", "version": "9.6.1"},
  "config": {"disableBail": true},
  "files": {
    "src/mul.ts": {
      "source": "export const mul = (a, b) => a * b\n",
      "mutants": [
        {"id": "0", "mutatorName": "ArithmeticOperator",
         "location": {"start": {"line": 1, "column": 30}, "end": {"line": 1, "column": 35}},
         "status": "Survived"}
      ]
    }
  }
}"#
    .to_owned()
}

fn list_json(root: &Path) -> Value {
    let output = spirolo()
        .args(["list", "--format", "json"])
        .arg(root)
        .output()
        .unwrap();
    assert!(output.status.success());
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn list_json_shows_the_mutants_table_and_diagnoses_the_unusable_reports() {
    let root = fixture("mutation-list");
    let json = list_json(&root);

    // The schema version is unchanged: adding a table never bumps it.
    assert_eq!(json["schema_version"], serde_json::json!(2));

    // Every report is a row; the mutants table carries every report's mutant.
    assert_eq!(json["mutation_reports"].as_array().unwrap().len(), 3);
    assert_eq!(json["mutants"].as_array().unwrap().len(), 3);

    // The good report's one test joins the model test.
    let joined: Vec<&Value> = json["mutation_tests"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|t| !t["joined"].is_null())
        .collect();
    assert_eq!(joined.len(), 1);
    assert_eq!(joined[0]["name"], serde_json::json!("add adds two numbers"));

    // Exactly the bailed and the no-per-test report earn a diagnostic.
    let codes: Vec<&str> = json["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|d| d["code"].as_str())
        .filter(|c| c.starts_with("mutation_report"))
        .collect();
    assert!(
        codes.contains(&"mutation_report_bailed"),
        "codes: {codes:?}"
    );
    assert!(
        codes.contains(&"mutation_report_no_per_test"),
        "codes: {codes:?}"
    );
}

#[test]
fn two_runs_over_the_same_tree_are_byte_identical() {
    let root = fixture("mutation-determinism");
    let first = spirolo()
        .args(["list", "--format", "json"])
        .arg(&root)
        .output()
        .unwrap();
    let second = spirolo()
        .args(["list", "--format", "json"])
        .arg(&root)
        .output()
        .unwrap();
    assert_eq!(first.stdout, second.stdout);
}

#[test]
fn a_mutation_glob_that_matches_nothing_is_an_empty_table_not_an_error() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("mutation-empty");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    write(
        &root,
        ".spirolorc.jsonc",
        "{ \"inputs\": { \"mutation\": [\"nowhere/**/mutation.json\"] } }\n",
    );

    let json = list_json(&root);
    assert_eq!(json["mutants"], serde_json::json!([]));
    assert_eq!(json["mutation_reports"], serde_json::json!([]));
    assert_eq!(json["mutation_tests"], serde_json::json!([]));
}
