//! `spirolo explain`, end to end on fixture projects (PRD 0006, seam 2).
//!
//! A rule name prints the rule: its family, default severity, summary and a Status block
//! (verdict, tier, shown precision, corpus entries, the proven bar and the rule's page).
//! A finding id prints the finding exactly as `check --format json` renders it, plus the
//! rule's status and the finding's editor severity, looked up before the baseline filter.
//! An unknown id is exit 2 with a message on stderr.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use spirolo_rules::status::{RuleStatus, Verdict};

#[allow(dead_code)]
#[path = "support/status.rs"]
mod status_support;

#[path = "support/policy.rs"]
mod policy_support;

fn spirolo() -> Command {
    Command::cargo_bin("spirolo").unwrap()
}

/// Run `spirolo explain` with `args`; return its stdout, asserting the exit code, and that a
/// second run is byte-equal to the first.
fn explain(args: &[&str], code: i32) -> String {
    let run = || {
        let output = spirolo().arg("explain").args(args).output().unwrap();
        assert_eq!(
            output.status.code(),
            Some(code),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    };
    let first = run();
    assert_eq!(first, run(), "two runs are byte-equal");
    first
}

// ---------------------------------------------------------------------------- by rule name

#[test]
fn explain_names_the_measured_corpus_and_the_effective_default() {
    let json: serde_json::Value =
        serde_json::from_str(&explain(&["weak-assertion", "--format", "json"], 0)).unwrap();
    let row = &status_support::current().rules["weak-assertion"];
    assert_eq!(
        json["default_severity"],
        if row.verdict == spirolo_rules::status::Verdict::Pass {
            "warn"
        } else {
            "info"
        }
    );
    assert_eq!(
        json["status"]["measured_on"],
        serde_json::json!(row.measured_on)
    );
    let human = explain(&["weak-assertion"], 0);
    if !row.measured_on.is_empty() {
        assert!(human.contains(&format!("measured on  {}\n", row.measured_on.join(", "))));
    }
}

#[test]
fn a_rule_name_prints_the_rule_and_its_status_as_json() {
    let json = explain(&["weak-assertion", "--format", "json"], 0);
    let rule = spirolo_rules::registry()
        .into_iter()
        .find(|r| r.name() == "weak-assertion")
        .unwrap();
    assert_eq!(
        json,
        spirolo_rules::status::with_table(status_support::current(), || {
            spirolo_report::explain_rule_json(rule.as_ref())
        })
    );
    let json = status_support::captured::with(|| spirolo_report::explain_rule_json(rule.as_ref()));
    insta::assert_snapshot!("explain_rule_json", json);
}

#[test]
fn a_rule_name_prints_family_severity_summary_and_status_line() {
    let human = explain(&["duplicate-test"], 0);
    let rule = spirolo_rules::registry()
        .into_iter()
        .find(|r| r.name() == "duplicate-test")
        .unwrap();
    assert_eq!(
        human,
        spirolo_rules::status::with_table(status_support::current(), || {
            spirolo_report::explain_rule_human(rule.as_ref())
        })
    );
    let human =
        status_support::captured::with(|| spirolo_report::explain_rule_human(rule.as_ref()));
    insta::assert_snapshot!("explain_rule_human", human);
}

/// One status row, built here; the interval is the one the bench wrote for these counts.
fn row(
    verdict: Verdict,
    truths: u32,
    falses: u32,
    interval: Option<[f64; 2]>,
    measured_on: &[&str],
) -> RuleStatus {
    RuleStatus {
        measured_on: measured_on.iter().map(|&entry| entry.to_owned()).collect(),
        verdict,
        carried: None,
        precision: interval.map(|_| f64::from(truths) / f64::from(truths + falses)),
        interval,
        truths,
        falses,
    }
}

/// `explain <rule>` as human text and JSON, with `rule`'s row replaced by `status`.
fn explain_under(rule: &str, status: RuleStatus) -> (String, serde_json::Value) {
    let mut table = status_support::captured::table().clone();
    table.rules.insert(rule.to_owned(), status);
    let table = Box::leak(Box::new(table));
    let rule = spirolo_rules::rule_named(rule).unwrap();
    spirolo_rules::status::with_table(table, || {
        (
            spirolo_report::explain_rule_human(rule.as_ref()),
            serde_json::from_str(&spirolo_report::explain_rule_json(rule.as_ref())).unwrap(),
        )
    })
}

/// The Status block's line labelled `label`, without the label.
fn line<'a>(human: &'a str, label: &str) -> &'a str {
    human
        .lines()
        .find_map(|line| line.strip_prefix(&format!("  {label}")))
        .unwrap_or_else(|| panic!("no {label} line in {human}"))
        .trim_start()
}

/// The JSON `status` fields every tier shares: the floors from the status module's
/// constants, the bar, the advisory sentence and the rule's page.
fn assert_shared_fields(rule: &str, human: &str, json: &serde_json::Value) {
    let status = &json["status"];
    let (numerator, denominator) = spirolo_rules::status::SHOWN_PRECISION_FLOOR;
    assert_eq!(
        status["floors"],
        serde_json::json!({
            "decided": spirolo_rules::status::SHOWN_DECIDED_FLOOR,
            "precision": f64::from(numerator) / f64::from(denominator),
        })
    );
    assert_eq!(status["proven_bar"], line(human, "proven bar"));
    assert!(line(human, "proven bar").contains("≥35 decided with 0 false"));
    assert_eq!(status["advisory"], line(human, "advisory"));
    let page = format!("docs/rules/{rule}.md");
    assert!(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(&page)
            .is_file(),
        "{page}"
    );
    assert_eq!(status["docs"], page.as_str());
    assert_eq!(line(human, "docs"), page);
}

#[test]
fn a_proven_rule_says_so_with_its_wilson_interval() {
    let (human, json) = explain_under(
        "weak-assertion",
        row(
            Verdict::Pass,
            62,
            0,
            Some([0.942, 1.0]),
            &["expo__expo", "mobxjs__mobx"],
        ),
    );
    assert_eq!(json["default_severity"], "warn");
    assert_eq!(json["status"]["tier"], "proven");
    assert_eq!(json["status"]["decided"], 62);
    assert_eq!(
        json["status"]["measured_on"],
        serde_json::json!(["expo__expo", "mobxjs__mobx"])
    );
    assert_shared_fields("weak-assertion", &human, &json);
    insta::assert_snapshot!("explain_rule_status_proven", human);
}

#[test]
fn an_advisory_rule_is_shown_by_default_at_the_floors() {
    let (human, json) = explain_under(
        "duplicate-test",
        row(
            Verdict::Unadjudicated,
            15,
            3,
            Some([0.608, 0.942]),
            &["expo__expo"],
        ),
    );
    assert_eq!(json["default_severity"], "info");
    assert_eq!(json["status"]["tier"], "advisory");
    assert_eq!(json["status"]["decided"], 18);
    assert_eq!(json["status"]["truths"], 15);
    assert_eq!(json["status"]["falses"], 3);
    assert_eq!(
        line(&human, "precision"),
        "15/18 = 0.833, Wilson 95% [0.608, 0.942]"
    );
    assert_shared_fields("duplicate-test", &human, &json);
    insta::assert_snapshot!("explain_rule_status_advisory", human);
}

#[test]
fn an_experimental_rule_is_shown_only_with_all() {
    let (human, json) = explain_under(
        "gate-truth",
        row(
            Verdict::Unadjudicated,
            12,
            6,
            Some([0.437, 0.837]),
            &["expo__expo", "react__react"],
        ),
    );
    assert_eq!(json["status"]["tier"], "experimental");
    assert!(line(&human, "tier").contains("--all"));
    assert_shared_fields("gate-truth", &human, &json);
    insta::assert_snapshot!("explain_rule_status_experimental", human);
}

#[test]
fn a_dropped_or_failing_rule_is_hidden_and_never_gates() {
    for verdict in [Verdict::Dropped, Verdict::Fail] {
        let (human, json) = explain_under(
            "orphan-test",
            row(verdict, 4, 10, Some([0.117, 0.546]), &["expo__expo"]),
        );
        assert_eq!(json["status"]["tier"], "hidden", "{verdict:?}");
        assert_eq!(json["status"]["verdict"], verdict.name());
        assert!(line(&human, "tier").contains("never gates"), "{verdict:?}");
        assert_shared_fields("orphan-test", &human, &json);
        if verdict == Verdict::Dropped {
            insta::assert_snapshot!("explain_rule_status_hidden", human);
        }
    }
}

#[test]
fn a_rule_with_no_decided_rows_has_no_precision_and_no_corpus() {
    let (human, json) = explain_under("unused-mock", row(Verdict::Insufficient, 0, 0, None, &[]));
    assert_eq!(json["status"]["tier"], "experimental");
    assert_eq!(json["status"]["decided"], 0);
    assert_eq!(json["status"]["precision"], serde_json::Value::Null);
    assert_eq!(json["status"]["interval"], serde_json::Value::Null);
    assert_eq!(json["status"]["measured_on"], serde_json::json!([]));
    assert_eq!(line(&human, "precision"), "none decided");
    assert_eq!(line(&human, "measured on"), "no corpus entry recorded");
    assert_shared_fields("unused-mock", &human, &json);
    insta::assert_snapshot!("explain_rule_status_undecided", human);
}

// ------------------------------------------------------------------------------ by finding id

/// A fresh project directory under the target tree; names are prefixed `explain-` so no
/// other test's fixture shares the directory.
fn project(name: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("explain-{name}"));
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

/// Two weak tests sharing one title in one suite, so their ids differ only by `#0` / `#1`,
/// plus a weak test in a second file.
fn weak_project(name: &str) -> PathBuf {
    let root = project(name);
    write(
        &root,
        "src/a.test.ts",
        "import { describe, it, expect } from 'vitest'\n\
         describe('widget panel', () => {\n\
         \x20 it('renders the widget', () => { expect(widget()).toBeTruthy() })\n\
         \x20 it('renders the widget', () => { expect(widget()).toBeDefined() })\n\
         })\n",
    );
    write(
        &root,
        "src/b.test.ts",
        "import { it, expect } from 'vitest'\n\
         it('b one', () => { expect(widget()).toBeTruthy() })\n",
    );
    root
}

/// The `findings` array `check --format json` prints for `root`.
fn check_findings(root: &Path) -> Vec<serde_json::Value> {
    let output = spirolo()
        .args(["check", "--all", "--format", "json"])
        .arg(root)
        .output()
        .unwrap();
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    document["findings"].as_array().unwrap().clone()
}

#[test]
fn every_check_finding_round_trips_through_explain_json() {
    let root = weak_project("round-trip");
    let findings = check_findings(&root);
    let ids: Vec<&str> = findings.iter().map(|f| f["id"].as_str().unwrap()).collect();
    assert!(
        ids.iter().any(|id| id.contains(' ') && id.ends_with("#1")),
        "the fixture has an id with spaces and #1: {ids:?}"
    );

    for finding in &findings {
        let id = finding["id"].as_str().unwrap();
        let json = explain(&[id, root.to_str().unwrap(), "--format", "json"], 0);
        let mut explained: serde_json::Value = serde_json::from_str(&json).unwrap();
        let object = explained.as_object_mut().unwrap();
        let status = object.remove("status").unwrap();
        let editor_severity = object.remove("editor_severity").unwrap();
        assert_eq!(&explained, finding, "{id}");
        let rule = finding["rule"].as_str().unwrap();
        assert_eq!(
            status,
            serde_json::to_value(&status_support::current().rules[rule]).unwrap(),
            "{id}"
        );
        let expected_severity =
            spirolo_rules::status::with_table(status_support::current(), || {
                spirolo_report::editor_severity(&status_support::finding(finding))
                    .map(spirolo_report::EditorSeverity::code)
            });
        assert_eq!(
            editor_severity,
            serde_json::json!(expected_severity),
            "{id}"
        );
    }
}

#[test]
fn an_id_with_spaces_and_an_ordinal_prints_that_finding_as_json() {
    let root = weak_project("ordinal");
    let json = explain(
        &[
            "weak-assertion src/a.test.ts widget panel › renders the widget#1",
            root.to_str().unwrap(),
            "--format",
            "json",
        ],
        0,
    );
    let finding = status_support::finding(&serde_json::from_str(&json).unwrap());
    assert_eq!(
        json,
        spirolo_rules::status::with_table(status_support::current(), || {
            spirolo_report::explain_json(&finding)
        })
    );
    let json = status_support::captured::with(|| spirolo_report::explain_json(&finding));
    insta::assert_snapshot!("explain_id_json", json);
}

#[test]
fn an_unknown_id_exits_two_with_a_message() {
    let root = weak_project("unknown");
    let output = spirolo()
        .args(["explain", "weak-assertion src/a.test.ts gone#0"])
        .arg(&root)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "spirolo explain: no finding with id 'weak-assertion src/a.test.ts gone#0'\n"
    );
}

#[test]
fn a_baselined_finding_is_still_explained() {
    let root = weak_project("baselined");
    spirolo().arg("ratchet").arg(&root).assert().success();
    assert!(
        check_findings(&root).is_empty(),
        "check surfaces nothing once the baseline records every finding"
    );

    let json = explain(
        &[
            "weak-assertion src/b.test.ts b one#0",
            root.to_str().unwrap(),
            "--format",
            "json",
        ],
        0,
    );
    let explained: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(explained["id"], "weak-assertion src/b.test.ts b one#0");
    assert_eq!(explained["line"], 2);
}

#[test]
fn an_id_prints_rule_location_message_related_evidence_explain_and_status() {
    let root = weak_project("human");
    let human = explain(
        &[
            "duplicate-test src/a.test.ts 65e363a0c4bb4c37#0",
            root.to_str().unwrap(),
        ],
        0,
    );
    let finding = status_support::check(&root)
        .1
        .into_iter()
        .find(|finding| finding.id() == "duplicate-test src/a.test.ts 65e363a0c4bb4c37#0")
        .unwrap();
    assert_eq!(
        human,
        spirolo_rules::status::with_table(status_support::current(), || {
            spirolo_report::explain_human(&finding)
        })
    );
    let human = status_support::captured::with(|| spirolo_report::explain_human(&finding));
    insta::assert_snapshot!("explain_id_human", human);
}

#[test]
fn listed_and_info_rows_use_the_reported_status_table() {
    let root = project("listed");
    write(
        &root,
        "src/a.test.ts",
        "import { it, expect } from 'vitest'\n\
         it.skip('later', () => { expect(add(1, 2)).toBe(3) })\n\
         it.skipIf(process.env.CI)('local only', () => { expect(add(1, 2)).toBe(3) })\n",
    );
    let editor_severity = |id: &str| {
        let json = explain(&[id, root.to_str().unwrap(), "--format", "json"], 0);
        serde_json::from_str::<serde_json::Value>(&json).unwrap()["editor_severity"].clone()
    };
    let finding = status_support::check(&root)
        .1
        .into_iter()
        .find(|finding| finding.id() == "dead-skip src/a.test.ts local only#0")
        .unwrap();
    let expected = spirolo_rules::status::with_table(status_support::current(), || {
        spirolo_report::editor_severity(&finding).map(spirolo_report::EditorSeverity::code)
    });
    assert_eq!(editor_severity(&finding.id()), serde_json::json!(expected));
    assert_eq!(
        editor_severity("dead-skip src/a.test.ts listed:later#0"),
        serde_json::Value::Null
    );
}
