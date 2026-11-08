//! How `--propose` renders across the four surfaces: a `proposal` object in the JSON
//! findings, a `properties.proposal` bag on the SARIF result, and one related-location-free
//! line in the human and agent views — and, in every surface, nothing at all when the
//! finding carries no proposal.
//!
//! The report crate on hand-built findings, the seam closest to the writer, so the shape is
//! proven without a whole binary run. The binary fixtures (seam 2) prove the same end to end.

use std::time::Duration;

use spirolo_model::Model;
use spirolo_rules::{rule_named, Finding, Location, Outcome, Proposal, Rule, Severity};

/// A hygiene finding at a fixed location carrying `proposal`.
fn finding(rule: &'static str, proposal: Option<Proposal>) -> Finding {
    Finding {
        rule,
        severity: Severity::Warn,
        outcome: Outcome::Finding,
        location: Location {
            file: "src/a.test.ts".to_owned(),
            line: 3,
            column: 1,
        },
        related: Vec::new(),
        message: "a duplicates b".to_owned(),
        evidence: Vec::new(),
        explain: "Merge them.".to_owned(),
        anchor: "h1".to_owned(),
        test: None,
        ordinal: 0,
        proposal,
    }
}

fn check_json(findings: &[Finding]) -> serde_json::Value {
    serde_json::from_str(&spirolo_report::to_check_json(&Model::default(), findings)).unwrap()
}

/// The `proposal` value of the first finding in the JSON document.
fn first_proposal(doc: &serde_json::Value) -> &serde_json::Value {
    &doc["findings"][0]["proposal"]
}

#[test]
fn the_json_renders_a_merge_proposal_as_an_action_what_merges_and_a_target() {
    let doc = check_json(&[finding(
        "duplicate-test",
        Some(Proposal::Merge {
            from: "a (src/a.test.ts:4)".to_owned(),
            into: "b (src/a.test.ts:9)".to_owned(),
        }),
    )]);
    assert_eq!(
        first_proposal(&doc),
        &serde_json::json!({
            "action": "merge",
            "from": "a (src/a.test.ts:4)",
            "target": "b (src/a.test.ts:9)"
        })
    );
}

#[test]
fn the_json_renders_a_bare_delete_as_just_the_action() {
    let doc = check_json(&[finding(
        "unused-mock",
        Some(Proposal::Delete { what: None }),
    )]);
    assert_eq!(
        first_proposal(&doc),
        &serde_json::json!({ "action": "delete" })
    );
}

#[test]
fn the_json_renders_a_delete_with_a_target_and_a_keep_with_a_reason() {
    let delete = check_json(&[finding(
        "orphan-test",
        Some(Proposal::Delete {
            what: Some("./gone".to_owned()),
        }),
    )]);
    assert_eq!(
        first_proposal(&delete),
        &serde_json::json!({ "action": "delete", "target": "./gone" })
    );
    let keep = check_json(&[finding(
        "orphan-test",
        Some(Proposal::Keep {
            reason: "add the include glob".to_owned(),
        }),
    )]);
    assert_eq!(
        first_proposal(&keep),
        &serde_json::json!({ "action": "keep", "reason": "add the include glob" })
    );
}

/// The `duplicate-test` rule box, so the human and agent views have a rule to group under.
fn ran() -> Vec<Box<dyn Rule>> {
    vec![rule_named("duplicate-test").expect("a known rule")]
}

#[test]
fn the_human_view_prints_one_related_location_free_proposal_line() {
    let findings = [finding(
        "duplicate-test",
        Some(Proposal::Merge {
            from: "a (src/a.test.ts:4)".to_owned(),
            into: "b (src/a.test.ts:9)".to_owned(),
        }),
    )];
    let human = spirolo_report::render_check_human(
        &Model::default(),
        Duration::from_millis(1),
        &ran(),
        &findings,
    );
    assert!(
        human.contains("merge a (src/a.test.ts:4) into b (src/a.test.ts:9)"),
        "the proposal is one line under the finding: {human}"
    );
}

#[test]
fn without_the_flag_the_human_view_prints_no_proposal_line() {
    let findings = [finding("duplicate-test", None)];
    let human = spirolo_report::render_check_human(
        &Model::default(),
        Duration::from_millis(1),
        &ran(),
        &findings,
    );
    assert!(
        !human.contains("merge into"),
        "no proposal line without a proposal: {human}"
    );
}

#[test]
fn the_sarif_result_carries_the_proposal_as_a_properties_bag() {
    let with = [finding(
        "duplicate-test",
        Some(Proposal::Merge {
            from: "a (src/a.test.ts:4)".to_owned(),
            into: "b (src/a.test.ts:9)".to_owned(),
        }),
    )];
    let doc: serde_json::Value =
        serde_json::from_str(&spirolo_report::to_sarif(&with, "0.0.1")).unwrap();
    assert_eq!(
        doc["runs"][0]["results"][0]["properties"]["proposal"],
        serde_json::json!({
            "action": "merge",
            "from": "a (src/a.test.ts:4)",
            "target": "b (src/a.test.ts:9)"
        }),
        "the proposal is a properties.proposal bag on the result: {doc}"
    );

    let without = [finding("duplicate-test", None)];
    let doc: serde_json::Value =
        serde_json::from_str(&spirolo_report::to_sarif(&without, "0.0.1")).unwrap();
    assert!(
        doc["runs"][0]["results"][0]["properties"]
            .get("proposal")
            .is_none(),
        "no properties.proposal without a proposal: {doc}"
    );
}

#[test]
fn the_agent_view_prints_the_proposal_as_a_tab_line() {
    let with = [finding(
        "unused-mock",
        Some(Proposal::Delete { what: None }),
    )];
    let agent = spirolo_report::render_check_agent(&with);
    assert!(
        agent.contains("\tproposal\tdelete"),
        "the agent view carries the proposal on its own tab line: {agent:?}"
    );
    let without = [finding("unused-mock", None)];
    assert!(
        !spirolo_report::render_check_agent(&without).contains("proposal"),
        "no proposal line without the flag"
    );
}

#[test]
fn without_a_proposal_the_json_finding_has_no_proposal_key() {
    let doc = check_json(&[finding("duplicate-test", None)]);
    assert!(
        doc["findings"][0].get("proposal").is_none(),
        "the field is absent when the finding carries no proposal: {}",
        doc["findings"][0]
    );
}
