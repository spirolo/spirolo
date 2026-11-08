//! GitHub annotations on hand-built findings: the command follows the severity, so an
//! advisory `info` row is a notice, never a warning on a run that exits 0.

#[path = "../../spirolo-rules/tests/support/status.rs"]
mod status_fixture;

use spirolo_report::render_github_annotations;
use spirolo_rules::{Finding, Location, Outcome, Severity};

fn finding(rule: &'static str, severity: Severity, line: u32) -> Finding {
    Finding {
        rule,
        severity,
        outcome: Outcome::Finding,
        location: Location {
            file: "src/a.test.ts".to_owned(),
            line,
            column: 1,
        },
        related: Vec::new(),
        message: format!("{rule} at {line}"),
        evidence: Vec::new(),
        explain: String::new(),
        anchor: format!("{rule}-{line}"),
        test: None,
        ordinal: 0,
        proposal: None,
    }
}

#[test]
fn info_is_a_notice_warn_a_warning_and_error_an_error() {
    // The captured table: dead-skip and weak-assertion are PASS, unused-mock INSUFFICIENT.
    let findings = [
        finding("unused-mock", Severity::Info, 1),
        finding("dead-skip", Severity::Warn, 2),
        finding("weak-assertion", Severity::Error, 3),
    ];
    let report = status_fixture::with(|| render_github_annotations(&findings));
    assert_eq!(
        report,
        "::notice file=src/a.test.ts,line=1,title=unused-mock (INSUFFICIENT)::unused-mock at 1\n\
         ::warning file=src/a.test.ts,line=2,title=dead-skip (PASS)::dead-skip at 2\n\
         ::error file=src/a.test.ts,line=3,title=weak-assertion (PASS)::weak-assertion at 3\n\
         spirolo: 3 findings · 3 annotations · 0 omitted\n"
    );
}

#[test]
fn each_level_caps_at_ten_on_its_own() {
    let findings: Vec<Finding> = (1..=12)
        .map(|line| finding("unused-mock", Severity::Info, line))
        .chain((13..=14).map(|line| finding("dead-skip", Severity::Warn, line)))
        .collect();
    let report = status_fixture::with(|| render_github_annotations(&findings));
    let count = |command: &str| report.lines().filter(|l| l.starts_with(command)).count();
    assert_eq!(count("::notice "), 10, "{report}");
    assert_eq!(count("::warning "), 2, "{report}");
    assert!(
        report.ends_with("spirolo: 14 findings · 12 annotations · 2 omitted\n"),
        "{report}"
    );
}
