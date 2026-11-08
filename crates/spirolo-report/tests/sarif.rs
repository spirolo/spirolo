//! What the SARIF surface promises consumers: valid SARIF 2.1, a `rules` entry per rule
//! carrying its explain as help text, a result per finding with its primary and related
//! locations, severity mapped to a SARIF level, no fingerprints of our own, and byte-stable
//! output across runs.
//!
//! Seam of the PRD's testing decisions closest to the writer: the report crate on
//! hand-built findings, so the SARIF shape is proven without a whole binary run.

#[path = "../../spirolo-rules/tests/support/status.rs"]
mod status_fixture;

use spirolo_rules::{Evidence, Finding, Location, Outcome, RelatedLocation, Severity};

/// A hand-built finding with a primary location, two related locations with labels, and an
/// explain — the shape `handler-drift` will produce.
fn drift_finding() -> Finding {
    Finding {
        rule: "handler-drift",
        severity: Severity::Error,
        outcome: Outcome::Finding,
        location: Location {
            file: "src/app.test.ts".to_owned(),
            line: 12,
            column: 3,
        },
        related: vec![
            RelatedLocation {
                label: "handler".to_owned(),
                location: Location {
                    file: "src/mocks/handlers.ts".to_owned(),
                    line: 4,
                    column: 1,
                },
            },
            RelatedLocation {
                label: "production url".to_owned(),
                location: Location {
                    file: "src/api.ts".to_owned(),
                    line: 7,
                    column: 9,
                },
            },
        ],
        message: "the test calls /v2/users but the handler mocks /v1/users".to_owned(),
        evidence: vec![Evidence::new("called", "/v2/users")],
        explain: "Point the handler at /v2/users, the URL the code calls.".to_owned(),
        anchor: "app › calls users".to_owned(),
        test: None,
        ordinal: 0,
        proposal: None,
    }
}

/// A second finding, a `warn` from another rule with no related locations, so the run
/// carries two rules and two levels.
fn weak_finding() -> Finding {
    Finding {
        rule: "weak-assertion",
        severity: Severity::Warn,
        outcome: Outcome::Finding,
        location: Location {
            file: "src/widget.test.ts".to_owned(),
            line: 3,
            column: 5,
        },
        related: Vec::new(),
        message: "test \"exists\" asserts only value-less matchers (toBeTruthy)".to_owned(),
        evidence: Vec::new(),
        explain: "Assert the produced value, not merely that something exists.".to_owned(),
        anchor: "widget › exists".to_owned(),
        test: None,
        ordinal: 0,
        proposal: None,
    }
}

fn sarif(findings: &[Finding]) -> serde_json::Value {
    serde_json::from_str(&spirolo_report::to_sarif(findings, "0.0.1")).unwrap()
}

#[test]
fn the_document_is_sarif_2_1_with_the_spirolo_driver() {
    let doc = sarif(&[drift_finding()]);

    assert_eq!(doc["version"], "2.1.0");
    assert!(
        doc["$schema"].as_str().unwrap().contains("sarif-2.1.0"),
        "carries the 2.1 schema url: {doc}"
    );
    let driver = &doc["runs"][0]["tool"]["driver"];
    assert_eq!(driver["name"], "spirolo");
    assert_eq!(driver["version"], "0.0.1");
}

#[test]
fn one_rules_entry_per_rule_with_explain_as_help_text() {
    let doc = sarif(&[drift_finding(), weak_finding()]);
    let rules = doc["runs"][0]["tool"]["driver"]["rules"]
        .as_array()
        .unwrap();

    // One entry per distinct rule that produced a finding, sorted by id.
    let ids: Vec<&str> = rules.iter().map(|r| r["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["handler-drift", "weak-assertion"]);

    // The explain of the rule's finding is the help text.
    let drift = &rules[0];
    assert_eq!(
        drift["help"]["text"],
        "Point the handler at /v2/users, the URL the code calls."
    );
    // Severity maps to a default level on the rule descriptor.
    assert_eq!(drift["defaultConfiguration"]["level"], "error");
}

#[test]
fn a_result_per_finding_with_the_primary_location() {
    let doc = sarif(&[drift_finding()]);
    let results = doc["runs"][0]["results"].as_array().unwrap();
    assert_eq!(results.len(), 1);

    let result = &results[0];
    assert_eq!(result["ruleId"], "handler-drift");
    assert_eq!(result["level"], "error");
    assert_eq!(
        result["message"]["text"],
        "the test calls /v2/users but the handler mocks /v1/users"
    );
    let physical = &result["locations"][0]["physicalLocation"];
    assert_eq!(physical["artifactLocation"]["uri"], "src/app.test.ts");
    assert_eq!(physical["region"]["startLine"], 12);
    assert_eq!(physical["region"]["startColumn"], 3);
}

#[test]
fn related_locations_appear_with_their_labels_as_messages() {
    let doc = sarif(&[drift_finding()]);
    let related = doc["runs"][0]["results"][0]["relatedLocations"]
        .as_array()
        .unwrap();
    assert_eq!(related.len(), 2);

    assert_eq!(related[0]["message"]["text"], "handler");
    assert_eq!(
        related[0]["physicalLocation"]["artifactLocation"]["uri"],
        "src/mocks/handlers.ts"
    );
    assert_eq!(related[0]["physicalLocation"]["region"]["startLine"], 4);
    assert_eq!(related[1]["message"]["text"], "production url");
}

#[test]
fn the_severity_levels_map_error_warn_info() {
    let mut info = weak_finding();
    info.rule = "gate-truth";
    info.severity = Severity::Info;
    let doc = sarif(&[drift_finding(), weak_finding(), info]);
    let results = doc["runs"][0]["results"].as_array().unwrap();

    // Results are sorted by rule, so gate-truth (info) < handler-drift (error) < weak (warn).
    let level = |rule: &str| -> String {
        results.iter().find(|r| r["ruleId"] == rule).unwrap()["level"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    assert_eq!(level("handler-drift"), "error");
    assert_eq!(level("weak-assertion"), "warning");
    assert_eq!(level("gate-truth"), "note");
}

#[test]
fn no_fingerprints_of_our_own() {
    let json = spirolo_report::to_sarif(&[drift_finding()], "0.0.1");
    assert!(
        !json.contains("partialFingerprints") && !json.contains("fingerprints"),
        "the upload action computes fingerprints; we emit none: {json}"
    );
}

#[test]
fn two_runs_are_byte_equal() {
    let findings = [drift_finding(), weak_finding()];
    assert_eq!(
        spirolo_report::to_sarif(&findings, "0.0.1").as_bytes(),
        spirolo_report::to_sarif(&findings, "0.0.1").as_bytes(),
        "SARIF is byte-identical across runs"
    );
}

#[test]
fn a_could_not_decide_outcome_is_not_a_result() {
    let mut undecided = drift_finding();
    undecided.outcome = Outcome::CouldNotDecide;
    let doc = sarif(&[undecided]);

    // An honest "could not decide" is not a code-scanning result and never fails a gate.
    assert!(
        doc["runs"][0]["results"].as_array().unwrap().is_empty(),
        "could-not-decide is not surfaced as a result: {doc}"
    );
}

#[test]
fn a_listed_outcome_is_not_a_result() {
    let mut listed = drift_finding();
    listed.outcome = Outcome::Listed;
    let doc = sarif(&[listed]);

    // A listing row inventories a fact and claims nothing, so it is not a code-scanning
    // result either.
    assert!(
        doc["runs"][0]["results"].as_array().unwrap().is_empty(),
        "a listed row is not surfaced as a result: {doc}"
    );
}

#[test]
fn a_file_less_finding_carries_an_empty_locations_array() {
    // The project-wide ratio row has no file: a physical location with a zero line would
    // violate the schema's one-based `startLine`, so the result carries no location.
    let mut project_wide = weak_finding();
    project_wide.location = Location {
        file: String::new(),
        line: 0,
        column: 0,
    };
    let doc = sarif(&[project_wide]);
    let result = &doc["runs"][0]["results"][0];

    assert!(
        result["locations"].as_array().unwrap().is_empty(),
        "a file-less finding has no physical location: {result}"
    );
    // It is still a result, referencing its rule.
    assert_eq!(result["ruleId"], "weak-assertion");
}

#[test]
fn an_empty_run_is_a_valid_document_with_no_results() {
    let doc = sarif(&[]);
    assert_eq!(doc["version"], "2.1.0");
    assert!(doc["runs"][0]["results"].as_array().unwrap().is_empty());
    assert!(doc["runs"][0]["tool"]["driver"]["rules"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn each_result_carries_its_rules_status_and_an_advisory_row_stays_a_note() {
    // The captured table: handler-drift is SKIPPED, weak-assertion PASS.
    let [mut drift, mut weak] = [drift_finding(), weak_finding()];
    drift.severity = Severity::Info;
    weak.severity = Severity::Info;
    let doc = status_fixture::with(|| sarif(&[drift, weak]));
    let results = doc["runs"][0]["results"].as_array().unwrap();
    assert_eq!(results[0]["properties"]["ruleStatus"], "SKIPPED", "{doc}");
    assert_eq!(results[0]["level"], "note");
    assert_eq!(results[1]["properties"]["ruleStatus"], "PASS", "{doc}");
    assert_eq!(results[1]["level"], "note");
}

#[test]
fn the_sarif_snapshots() {
    let doc = status_fixture::with(|| sarif(&[drift_finding(), weak_finding()]));
    insta::assert_json_snapshot!("sarif_two_rules", doc);
}
