//! Pins the display contract independently of the rule registry.
use spirolo_rules::Family;

#[test]
fn families_name_the_question_their_section_answers() {
    let titles = [
        Family::Audit,
        Family::Gate,
        Family::Hygiene,
        Family::Flake,
        Family::Impact,
    ]
    .map(Family::title);
    assert_eq!(
        titles,
        [
            "Weak tests",
            "Merge gate",
            "Unused tests",
            "Flaky",
            "Health"
        ]
    );
}

#[test]
fn every_registered_rule_declares_the_frozen_subject() {
    use spirolo_rules::Subject::{Gates, HotspotModules, Runs, TestFiles, Tests};
    let actual: Vec<_> = spirolo_rules::registry()
        .iter()
        .map(|rule| (rule.name(), rule.subject()))
        .collect();
    assert_eq!(
        actual,
        vec![
            ("handler-drift", Tests),
            ("generator-walk", Tests),
            ("absence-anchor", Tests),
            ("weak-assertion", Tests),
            ("data-layer-mock", Tests),
            ("fail-closed-setup", Gates),
            ("gate-truth", Gates),
            ("dead-skip", Tests),
            ("duplicate-test", Tests),
            ("redundant-test", Tests),
            ("unused-mock", TestFiles),
            ("orphan-test", TestFiles),
            ("stale-snapshot", TestFiles),
            ("flake-candidate", Tests),
            ("flake-observed", Runs),
            ("untested-hotspot", HotspotModules),
        ]
    );
}
