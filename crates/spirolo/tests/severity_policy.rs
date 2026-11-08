//! M6 severity policy through the binary, using a project actually run by Vitest.

use std::path::PathBuf;
use std::time::Duration;

use assert_cmd::Command;
use serde_json::Value;
use spirolo_rules::status::{Table, Tier};
use spirolo_rules::{Finding, Outcome, Severity};

#[allow(dead_code)]
#[path = "support/status.rs"]
mod status_support;

fn project(name: &str) -> PathBuf {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/flake-observed-vitest-2.1.9");
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("severity-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    for path in ["package.json", "src/ledger.test.ts"] {
        std::fs::copy(fixture.join(path), root.join(path)).unwrap();
    }
    root
}

#[test]
fn default_check_severity_obeys_the_embedded_verdict() {
    let root = project("default");
    let output = Command::cargo_bin("spirolo")
        .unwrap()
        .args([
            "check",
            "--all",
            "--rule",
            "duplicate-test",
            "--format",
            "json",
        ])
        .arg(&root)
        .output()
        .unwrap();
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    let findings = document["findings"].as_array().unwrap();
    assert!(
        !findings.is_empty(),
        "real Vitest project produces findings"
    );
    let pass = status_support::current().rules["duplicate-test"].verdict
        == spirolo_rules::status::Verdict::Pass;
    assert!(findings
        .iter()
        .all(|f| f["severity"] == if pass { "warn" } else { "info" }));
    assert_eq!(output.status.code(), Some(i32::from(pass)));
}

#[test]
fn a_warn_override_opts_the_same_finding_into_gating() {
    let root = project("warn-override");
    std::fs::write(
        root.join(".spirolorc.jsonc"),
        r#"{"rules":{"duplicate-test":{"severity":"warn"}}}"#,
    )
    .unwrap();
    let output = Command::cargo_bin("spirolo")
        .unwrap()
        .args([
            "check",
            "--all",
            "--rule",
            "duplicate-test",
            "--format",
            "json",
        ])
        .arg(&root)
        .output()
        .unwrap();
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    let findings = document["findings"].as_array().unwrap();
    assert!(!findings.is_empty());
    assert!(findings.iter().all(|f| f["severity"] == "warn"));
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn init_preserves_the_release_policy_instead_of_opting_rules_into_gating() {
    let root = project("init");
    let check = || {
        Command::cargo_bin("spirolo")
            .unwrap()
            .args([
                "check",
                "--all",
                "--rule",
                "duplicate-test",
                "--format",
                "json",
            ])
            .arg(&root)
            .output()
            .unwrap()
    };
    let before = check();
    Command::cargo_bin("spirolo")
        .unwrap()
        .arg("init")
        .arg(&root)
        .assert()
        .success();
    let after = check();
    let findings = |output: &std::process::Output| {
        serde_json::from_slice::<Value>(&output.stdout).unwrap()["findings"].clone()
    };
    assert_eq!(findings(&after), findings(&before));
    assert_eq!(after.status.code(), before.status.code());
}

#[test]
fn a_baseline_gates_new_warn_findings_and_suppresses_recorded_ones() {
    let root = project("baseline");
    // Record an empty project, then restore the actual Vitest source unchanged.
    let source = std::fs::read(root.join("src/ledger.test.ts")).unwrap();
    std::fs::remove_file(root.join("src/ledger.test.ts")).unwrap();
    Command::cargo_bin("spirolo")
        .unwrap()
        .arg("ratchet")
        .arg(&root)
        .assert()
        .success();
    std::fs::write(root.join("src/ledger.test.ts"), source).unwrap();
    std::fs::write(
        root.join(".spirolorc.jsonc"),
        r#"{"rules":{"duplicate-test":{"severity":"warn"}}}"#,
    )
    .unwrap();
    Command::cargo_bin("spirolo")
        .unwrap()
        .args(["check", "--all", "--rule", "duplicate-test"])
        .arg(&root)
        .assert()
        .code(1);
    Command::cargo_bin("spirolo")
        .unwrap()
        .arg("ratchet")
        .arg(&root)
        .assert()
        .success();
    Command::cargo_bin("spirolo")
        .unwrap()
        .args(["check", "--all", "--rule", "duplicate-test"])
        .arg(&root)
        .assert()
        .success();
}

#[test]
fn human_reports_group_real_findings_by_family_tier_severity_and_registry_order() {
    let root = project("order");
    for (source, dest) in [
        ("engine.test.ts", "src/__tests__/templates/engine.test.ts"),
        ("engine.ts", "src/templates/engine.ts"),
    ] {
        let dest = root.join(dest);
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
        std::fs::copy(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/redundant-test-email-kit-engine")
                .join(source),
            dest,
        )
        .unwrap();
    }
    // Explicit user overrides keep this real check independent of the shipped measurements,
    // except for a DROPPED or FAIL rule: its rows report at info whatever the override says,
    // and a note says the override was not applied.
    let overrides: serde_json::Map<String, Value> = spirolo_rules::registry()
        .iter()
        .map(|rule| {
            (
                rule.name().into(),
                Value::from(rule.default_severity().name()),
            )
        })
        .collect();
    std::fs::write(
        root.join(".spirolorc.jsonc"),
        serde_json::json!({"rules": overrides}).to_string(),
    )
    .unwrap();
    let (model, findings) = status_support::check(&root);
    assert!(findings.iter().any(|f| f.rule == "weak-assertion"));
    assert!(findings.iter().any(|f| f.rule == "duplicate-test"));
    let ran = spirolo_rules::registry();
    // The overrides a table leaves unapplied: those asking a DROPPED or FAIL rule to gate.
    let not_applied = |table: &Table| -> Vec<&'static str> {
        ran.iter()
            .filter(|rule| rule.default_severity() != Severity::Info)
            .filter(|rule| table.rules[rule.name()].tier() == Tier::Hidden)
            .map(|rule| rule.name())
            .collect()
    };
    let render = |findings: &[Finding], not_applied: &[&str]| {
        spirolo_report::render_check_human_with_options(
            &model,
            Duration::ZERO,
            &ran,
            findings,
            options(not_applied),
        )
    };
    let live = not_applied(status_support::current());
    let current =
        spirolo_rules::status::with_table(status_support::current(), || render(&findings, &live));
    let output = Command::cargo_bin("spirolo")
        .unwrap()
        .args(["check", "--all"])
        .arg(&root)
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout.split_once('\n').unwrap().1,
        current.split_once('\n').unwrap().1
    );
    assert_human_order(&current, &findings, status_support::current());
    // The CLI's severities follow the shipped table; restate them for the captured one.
    let table = status_support::captured::table();
    let findings = overridden_under(table, findings);
    let unapplied = not_applied(table);
    let captured = status_support::captured::with(|| render(&findings, &unapplied));
    assert_human_order(&captured, &findings, table);
    let diff = spirolo_report::Baseline::default().diff(&findings);
    let baseline = status_support::captured::with(|| {
        spirolo_report::render_check_human_baselined_with_options(
            &model,
            Duration::ZERO,
            &ran,
            &diff,
            false,
            options(&unapplied),
        )
    });
    assert_human_order(&baseline, &diff.surfaced, status_support::captured::table());
    insta::assert_snapshot!("proven_first_baselined_human", baseline);
    insta::assert_snapshot!("proven_first_human", captured);
}

/// The human options of a run whose config overrides `not_applied` could not apply.
fn options<'a>(not_applied: &'a [&'a str]) -> spirolo_report::HumanOptions<'a> {
    spirolo_report::HumanOptions {
        not_applied,
        ..Default::default()
    }
}

/// The findings of a check whose config sets every rule to its default severity, with the
/// severities `table` gives them: the overrides apply to every rule but its DROPPED and FAIL
/// ones, whose rows report at info (#506).
fn overridden_under(table: &Table, findings: Vec<Finding>) -> Vec<Finding> {
    findings
        .into_iter()
        .map(|mut finding| {
            if table.rules[finding.rule].tier() == Tier::Hidden {
                finding.severity = Severity::Info;
            } else if finding.outcome == Outcome::Finding {
                finding.severity = spirolo_rules::rule_named(finding.rule)
                    .unwrap()
                    .default_severity();
            }
            finding
        })
        .collect()
}

fn assert_human_order(
    report: &str,
    findings: &[spirolo_rules::Finding],
    table: &spirolo_rules::status::Table,
) {
    let mut expected: Vec<_> = spirolo_rules::registry()
        .into_iter()
        .filter(|r| findings.iter().any(|f| f.rule == r.name()))
        .collect();
    expected.sort_by_key(|r| {
        // Proven, advisory, experimental, then DROPPED and FAIL, as the table's rows say.
        let tier = table.rules[r.name()].tier() as u8;
        (
            r.family(),
            tier,
            findings
                .iter()
                .filter(|f| f.rule == r.name())
                .map(|f| f.severity)
                .min(),
        )
    });
    let expected: Vec<_> = expected.iter().map(|r| r.name()).collect();
    let actual: Vec<_> = report
        .lines()
        .filter_map(|line| {
            line.strip_prefix("● ")
                .map(|line| line.split_once(" (").unwrap().0)
        })
        .collect();
    assert_eq!(actual, expected);
}
