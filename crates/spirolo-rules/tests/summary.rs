//! `Rule::summary`: every registered rule says, in one sentence, which two things disagree.

#[test]
fn every_rule_has_a_one_sentence_summary() {
    for rule in spirolo_rules::registry() {
        let summary = rule.summary();
        assert!(
            summary.ends_with('.') && !summary.trim_end_matches('.').contains(". "),
            "{}: not one sentence: {summary:?}",
            rule.name()
        );
        assert!(
            summary.len() > 30,
            "{}: too short: {summary:?}",
            rule.name()
        );
    }
}
