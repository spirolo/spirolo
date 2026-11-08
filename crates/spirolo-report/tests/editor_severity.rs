//! `editor_severity`: PRD 0006's Editor severity table, every cell, on hand-built findings.
//!
//! `weak-assertion` is PASS and not carried in the captured fixture table (proven);
//! `duplicate-test` is FAIL carried to M6 (not proven, hidden); `fail-closed-setup` is
//! INSUFFICIENT at 14/0 (not proven, experimental: one decided finding under the floor);
//! `orphan-test` is INSUFFICIENT at 2/0 (experimental). [`advisory`] lifts `fail-closed-setup`
//! to 19/0, its advisory-tier numbers in the dogfood table, in a table built here.

#[path = "../../spirolo-rules/tests/support/status.rs"]
mod status_fixture;

use spirolo_report::{editor_severity, EditorSeverity};
use spirolo_rules::{Finding, Location, Outcome, Severity};

fn finding(rule: &'static str, outcome: Outcome, severity: Severity) -> Finding {
    Finding {
        rule,
        severity,
        outcome,
        location: Location {
            file: "src/a.test.ts".to_owned(),
            line: 3,
            column: 1,
        },
        related: Vec::new(),
        message: "m".to_owned(),
        evidence: Vec::new(),
        explain: "e".to_owned(),
        anchor: "a".to_owned(),
        test: None,
        ordinal: 0,
        proposal: None,
    }
}

/// The table row for one rule and outcome: the editor severity for error, warn and info.
fn row(rule: &'static str, outcome: Outcome) -> [Option<EditorSeverity>; 3] {
    [Severity::Error, Severity::Warn, Severity::Info]
        .map(|severity| status_fixture::with(|| editor_severity(&finding(rule, outcome, severity))))
}

use EditorSeverity::{Error, Hint, Information, Warning};

/// Render under the captured table with `fail-closed-setup` at 19/0, so it is advisory.
fn advisory<R>(run: impl FnOnce() -> R) -> R {
    static TABLE: std::sync::OnceLock<spirolo_rules::status::Table> = std::sync::OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut table = status_fixture::table().clone();
        let row = table.rules.get_mut("fail-closed-setup").unwrap();
        row.truths = 19;
        row.falses = 0;
        table
    });
    spirolo_rules::status::with_table(table, run)
}

#[test]
fn a_proven_rules_finding_keeps_its_severity() {
    assert_eq!(
        row("weak-assertion", Outcome::Finding),
        [Some(Error), Some(Warning), Some(Information)]
    );
}

#[test]
fn an_unproven_rules_finding_is_at_most_information() {
    assert_eq!(
        row("duplicate-test", Outcome::Finding),
        [Some(Information), Some(Information), Some(Hint)]
    );
}

#[test]
fn an_advisory_rule_lists_info_as_information() {
    advisory(|| {
        assert_eq!(
            spirolo_rules::status::tier("fail-closed-setup"),
            spirolo_rules::status::Tier::Advisory
        );
    });
    assert_eq!(
        [Severity::Error, Severity::Warn, Severity::Info].map(|severity| advisory(|| {
            editor_severity(&finding("fail-closed-setup", Outcome::Finding, severity))
        })),
        [Some(Information), Some(Information), Some(Information)]
    );
}

#[test]
fn an_experimental_rule_keeps_info_a_hint_even_when_its_interval_leans_true() {
    // 14/0 with an interval from 0.785: under the decided floor, so shown only by `--all`.
    status_fixture::with(|| assert!(spirolo_rules::status::leans_true("fail-closed-setup")));
    for rule in ["fail-closed-setup", "orphan-test"] {
        assert_eq!(
            row(rule, Outcome::Finding),
            [Some(Information), Some(Information), Some(Hint)],
            "{rule}"
        );
    }
}

#[test]
fn could_not_decide_is_a_hint_for_any_rule() {
    for rule in ["weak-assertion", "duplicate-test", "fail-closed-setup"] {
        assert_eq!(
            row(rule, Outcome::CouldNotDecide),
            [Some(Hint); 3],
            "{rule}"
        );
    }
}

#[test]
fn listed_is_never_published() {
    for rule in ["weak-assertion", "duplicate-test"] {
        assert_eq!(row(rule, Outcome::Listed), [None; 3], "{rule}");
    }
}

#[test]
fn lsp_codes_are_one_to_four() {
    assert_eq!(
        [Error, Warning, Information, Hint].map(EditorSeverity::code),
        [1, 2, 3, 4]
    );
}
