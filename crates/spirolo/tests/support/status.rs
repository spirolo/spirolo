//! Supply status tables to surface tests; inputs are real CLI documents.

use std::{path::Path, process::Command, sync::OnceLock};

use serde_json::Value;
use spirolo_rules::{Evidence, Finding, Location, Outcome, RelatedLocation, Severity, TestPath};

#[path = "../../../spirolo-rules/tests/support/status.rs"]
pub mod captured;

/// The table this test binary's CLI actually reports, without assuming any verdict.
pub fn current() -> &'static spirolo_rules::status::Table {
    static TABLE: OnceLock<spirolo_rules::status::Table> = OnceLock::new();
    TABLE.get_or_init(|| {
        let output = Command::new(env!("CARGO_BIN_EXE_spirolo"))
            .args(["rule-status", "--format", "json"])
            .output()
            .unwrap();
        assert!(output.status.success());
        spirolo_rules::status::parse(std::str::from_utf8(&output.stdout).unwrap()).unwrap()
    })
}

/// The verdict token the binary's own table gives `rule`: the `rule_status` its formats print.
pub fn rule_status(rule: &str) -> &'static str {
    spirolo_rules::status::with_table(current(), || {
        spirolo_rules::status::status(rule).verdict.name()
    })
}

/// What a snapshot holds in place of a rule status, which moves whenever the corpus does.
pub const REDACTED_STATUS: &str = "<rule_status>";

/// `document` with each check-JSON `rule_status` and each SARIF `properties.ruleStatus`
/// replaced by [`REDACTED_STATUS`], after asserting it is the binary's verdict for its rule.
pub fn redact_status(document: &Value) -> Value {
    let mut document = document.clone();
    redact_in(&mut document);
    document
}

fn redact_in(value: &mut Value) {
    match value {
        Value::Array(items) => items.iter_mut().for_each(redact_in),
        Value::Object(object) => {
            if let Some(rule) = object.get("rule").and_then(Value::as_str) {
                if let Some(status) = object.get("rule_status") {
                    assert_eq!(status, rule_status(rule), "{rule}");
                    object.insert("rule_status".into(), REDACTED_STATUS.into());
                }
            }
            if let Some(rule) = object
                .get("ruleId")
                .and_then(Value::as_str)
                .map(str::to_owned)
            {
                if let Some(properties) =
                    object.get_mut("properties").and_then(Value::as_object_mut)
                {
                    if let Some(status) = properties.get("ruleStatus") {
                        assert_eq!(status, rule_status(&rule), "{rule}");
                        properties.insert("ruleStatus".into(), REDACTED_STATUS.into());
                    }
                }
            }
            object.values_mut().for_each(redact_in);
        }
        _ => {}
    }
}

/// A printed JSON document (SARIF) with its rule statuses redacted as [`redact_status`] does,
/// keeping the binary's own layout: the document is canonical pretty JSON with a newline.
pub fn redact_status_text(text: &str) -> String {
    let document: Value = serde_json::from_str(text).unwrap();
    let mut redacted = serde_json::to_string_pretty(&redact_status(&document)).unwrap();
    redacted.push('\n');
    assert_eq!(
        serde_json::to_string_pretty(&document).unwrap() + "\n",
        text,
        "the document re-prints byte-equal"
    );
    redacted
}

/// An `--agent` view with each finding row's status column (the second) replaced by
/// [`REDACTED_STATUS`], after asserting it is the binary's verdict for the row's rule.
pub fn redact_agent_status(text: &str) -> String {
    let mut out = String::new();
    for line in text.lines() {
        if line.starts_with('\t') {
            out.push_str(line);
        } else {
            let mut fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(fields[1], rule_status(fields[0]), "{line}");
            fields[1] = REDACTED_STATUS;
            out.push_str(&fields.join("\t"));
        }
        out.push('\n');
    }
    out
}

/// What a snapshot holds in place of a human heading's trust suffix (` · advisory: …`,
/// ` · experimental: …`, ` · rule: DROPPED`, or nothing for a proven rule).
pub const REDACTED_TIER: &str = " · <tier>";

/// A human `check` view with each rule heading's trust suffix replaced by [`REDACTED_TIER`],
/// after asserting it is the label the binary's own table gives the rule. The label forms
/// are pinned by the seam tests under supplied tables; these snapshots pin the layout.
pub fn redact_heading_status(text: &str) -> String {
    let mut out = String::new();
    for line in text.split_inclusive('\n') {
        let Some(rule) = line
            .strip_prefix("● ")
            .and_then(|rest| rest.split_once(" ("))
            .map(|(rule, _)| rule)
        else {
            out.push_str(line);
            continue;
        };
        let (body, newline) = line
            .strip_suffix('\n')
            .map_or((line, ""), |body| (body, "\n"));
        let label =
            spirolo_rules::status::with_table(current(), || spirolo_report::tier_label(rule));
        let heading = body
            .strip_suffix(label.as_str())
            .unwrap_or_else(|| panic!("{body:?} does not end with {label:?}"));
        out.push_str(heading);
        out.push_str(REDACTED_TIER);
        out.push_str(newline);
    }
    out
}

/// Execute a real check and recover its model and findings for a supplied-table rendering.
pub fn check(root: &Path) -> (spirolo_model::Model, Vec<Finding>) {
    let output = Command::new(env!("CARGO_BIN_EXE_spirolo"))
        .args(["check", "--all", "--format", "json"])
        .arg(root)
        .output()
        .unwrap();
    assert_ne!(output.status.code(), Some(2));
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    let mut model: spirolo_model::Model = serde_json::from_value(document.clone()).unwrap();
    root.clone_into(&mut model.root);
    (
        model,
        document["findings"]
            .as_array()
            .unwrap()
            .iter()
            .map(finding)
            .collect(),
    )
}

/// Decode the public check finding shape; never fabricate a status-dependent finding.
pub fn finding(value: &Value) -> Finding {
    let string = |key: &str| value[key].as_str().unwrap().to_owned();
    let id = string("id");
    let rule = spirolo_rules::registry()
        .into_iter()
        .find(|rule| rule.name() == value["rule"].as_str().unwrap())
        .unwrap()
        .name();
    let file = string("file");
    let (anchor, ordinal) = id
        .strip_prefix(&format!("{rule} {file} "))
        .unwrap()
        .rsplit_once('#')
        .unwrap();
    assert!(
        value.get("proposal").is_none(),
        "these checks do not request proposals"
    );
    Finding {
        rule,
        severity: match value["severity"].as_str().unwrap() {
            "error" => Severity::Error,
            "warn" => Severity::Warn,
            "info" => Severity::Info,
            other => panic!("unknown severity {other}"),
        },
        outcome: match value["outcome"].as_str().unwrap() {
            "finding" => Outcome::Finding,
            "could_not_decide" => Outcome::CouldNotDecide,
            "listed" => Outcome::Listed,
            other => panic!("unknown outcome {other}"),
        },
        location: location(value),
        related: value["related"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| RelatedLocation {
                label: row["label"].as_str().unwrap().into(),
                location: location(row),
            })
            .collect(),
        evidence: value["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| Evidence::new(row["key"].as_str().unwrap(), row["value"].as_str().unwrap()))
            .collect(),
        message: string("message"),
        explain: string("explain"),
        anchor: anchor.into(),
        ordinal: ordinal.parse().unwrap(),
        test: value.get("test").map(|test| TestPath {
            suite_path: serde_json::from_value(test["suite_path"].clone()).unwrap(),
            title: serde_json::from_value(test["title"].clone()).unwrap(),
        }),
        proposal: None,
    }
}

fn location(value: &Value) -> Location {
    Location {
        file: value["file"].as_str().unwrap().into(),
        line: value["line"].as_u64().unwrap().try_into().unwrap(),
        column: value["column"].as_u64().unwrap().try_into().unwrap(),
    }
}
