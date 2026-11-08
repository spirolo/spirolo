//! `duplicate-test` on hand-built models: one case per branch of its semantics — a group of
//! two, a group of three, a loop-defined test excluded, a `.each` row excluded, a cross-file
//! group with two paired sources, and first-member ordering.
//!
//! The rule is a pure function of the model, so these tests build `TestCase` rows by hand and
//! read the findings out — never how they were computed. The binary fixtures (Seam 2) prove
//! the same rule end to end through `spirolo check`.

use spirolo_model::{Assertion, Each, Model, Pairing, PairingReason, Span, TestCase, TestKind};
use spirolo_rules::{Config, DuplicateTest, Finding, Outcome, Proposal, Rule, Severity};

/// A test case in `file` at `line` with a title, suite path and body hash. The span's byte
/// offset tracks the line so file-and-span ordering is well defined across rows in one file.
fn test_case(file: &str, line: u32, title: &str, suite: &[&str], hash: Option<&str>) -> TestCase {
    TestCase {
        file: file.to_owned(),
        span: Span {
            start: line * 100,
            end: line * 100 + 10,
            line,
            column: 1,
        },
        framework: Some("vitest".to_owned()),
        title: Some(title.to_owned()),
        kind: TestKind::Test,
        suite_path: suite.iter().map(|s| (*s).to_owned()).collect(),
        assertions: Vec::new(),
        throwing_queries: Vec::new(),
        mocks: Vec::new(),
        skip: None,
        each: None,
        loops: Vec::new(),
        modifiers: Vec::new(),
        tags: Vec::new(),
        body_hash: hash.map(str::to_owned),
        empty: false,
        context: Vec::new(),
    }
}

/// A model carrying exactly the given test cases.
fn model(cases: Vec<TestCase>) -> Model {
    Model {
        test_cases: cases,
        ..Model::default()
    }
}

/// The findings a config-less run of the rule produces.
fn findings(model: &Model) -> Vec<Finding> {
    DuplicateTest.run(model, &Config::default())
}

/// The findings a run under `--propose` produces, so a test can read each finding's proposal.
fn proposed(model: &Model) -> Vec<Finding> {
    DuplicateTest.run(
        model,
        &Config {
            propose: true,
            ..Config::default()
        },
    )
}

#[test]
fn without_propose_a_finding_carries_no_proposal() {
    let model = model(vec![
        test_case("a.test.ts", 1, "adds", &[], Some("h1")),
        test_case("a.test.ts", 5, "sums", &[], Some("h1")),
    ]);
    assert_eq!(
        findings(&model)[0].proposal,
        None,
        "off by default: no proposal without the flag"
    );
}

#[test]
fn under_propose_a_tie_on_assertions_merges_the_others_into_the_first_member() {
    let mut second = test_case("a.test.ts", 5, "sums", &[], Some("h1"));
    second.assertions = vec![assertion()];
    let mut first = test_case("a.test.ts", 1, "adds", &[], Some("h1"));
    first.assertions = vec![assertion()];
    let model = model(vec![first, second]);
    assert_eq!(
        proposed(&model)[0].proposal,
        Some(Proposal::Merge {
            from: "sums (a.test.ts:5)".to_owned(),
            into: "adds (a.test.ts:1)".to_owned()
        }),
        "when two members carry the same assertion count, the first in order wins, and the \
         proposal names what merges into it rather than merging the finding into itself"
    );
}

#[test]
fn under_propose_a_group_merges_into_the_member_with_the_most_assertions() {
    let mut rich = test_case("a.test.ts", 5, "sums", &[], Some("h1"));
    rich.assertions = vec![assertion(), assertion()];
    let model = model(vec![
        test_case("a.test.ts", 1, "adds", &[], Some("h1")),
        rich,
    ]);
    assert_eq!(
        proposed(&model)[0].proposal,
        Some(Proposal::Merge {
            from: "adds (a.test.ts:1)".to_owned(),
            into: "sums (a.test.ts:5)".to_owned()
        }),
        "the richer test — the one with the most assertions — is the merge target"
    );
}

#[test]
fn two_tests_with_the_same_body_hash_are_one_finding() {
    let model = model(vec![
        test_case("a.test.ts", 1, "adds", &[], Some("h1")),
        test_case("a.test.ts", 5, "sums", &[], Some("h1")),
    ]);
    let findings = findings(&model);
    assert_eq!(findings.len(), 1, "one group is one finding");
    let finding = &findings[0];
    assert_eq!(finding.rule, "duplicate-test");
    assert_eq!(finding.severity, Severity::Warn);
    assert_eq!(finding.outcome, Outcome::Finding);
    assert_eq!(finding.location.file, "a.test.ts");
    assert_eq!(
        finding.location.line, 1,
        "the finding sits at the first member"
    );
    assert_eq!(
        finding.related.len(),
        1,
        "the other member is a related location"
    );
}

#[test]
fn a_group_of_three_is_one_finding_with_two_related_locations() {
    let model = model(vec![
        test_case("a.test.ts", 1, "adds", &[], Some("h1")),
        test_case("a.test.ts", 5, "sums", &[], Some("h1")),
        test_case("a.test.ts", 9, "totals", &[], Some("h1")),
    ]);
    let findings = findings(&model);
    assert_eq!(findings.len(), 1, "three clones are still one group");
    assert_eq!(
        findings[0].related.len(),
        2,
        "the other two members are related"
    );
}

#[test]
fn a_loop_defined_test_is_excluded() {
    let mut looped = test_case("a.test.ts", 5, "sums", &[], Some("h1"));
    looped.loops = vec![Each {
        cases: None,
        params: None,
        source: None,
    }];
    let model = model(vec![
        test_case("a.test.ts", 1, "adds", &[], Some("h1")),
        looped,
    ]);
    assert!(
        findings(&model).is_empty(),
        "a test the runner instantiates once per loop element is one definition, not a clone"
    );
}

#[test]
fn an_each_table_row_is_excluded() {
    let mut table = test_case("a.test.ts", 5, "sums", &[], Some("h1"));
    table.each = Some(Each {
        cases: Some(3),
        params: Some(1),
        source: None,
    });
    let model = model(vec![
        test_case("a.test.ts", 1, "adds", &[], Some("h1")),
        table,
    ]);
    assert!(
        findings(&model).is_empty(),
        "a `.each` table is one definition with many runs, not a clone"
    );
}

/// A pairing joining a test file to the source module it exercises.
fn pairing(test_file: &str, source: &str) -> Pairing {
    Pairing {
        test_file: test_file.to_owned(),
        source_module: source.to_owned(),
        reason: PairingReason::DirectImport,
        ambiguous: false,
    }
}

fn evidence<'a>(finding: &'a Finding, key: &str) -> Option<&'a str> {
    finding
        .evidence
        .iter()
        .find(|e| e.key == key)
        .map(|e| e.value.as_str())
}

#[test]
fn a_cross_file_group_names_each_members_paired_source() {
    let model = Model {
        test_cases: vec![
            test_case("a.test.ts", 1, "adds", &[], Some("h1")),
            test_case("b.test.ts", 1, "sums", &[], Some("h1")),
        ],
        pairings: vec![
            pairing("a.test.ts", "src/a.ts"),
            pairing("b.test.ts", "src/b.ts"),
        ],
        ..Model::default()
    };
    let findings = findings(&model);
    assert_eq!(findings.len(), 1);
    let finding = &findings[0];
    assert_eq!(
        evidence(finding, "size"),
        Some("2"),
        "the group size is evidence"
    );
    assert_eq!(
        evidence(finding, "cross-file"),
        Some("yes"),
        "a group spanning two files is marked cross-file"
    );
    let sources = evidence(finding, "sources").unwrap();
    assert!(
        sources.contains("src/a.ts"),
        "the first member's paired source: {sources}"
    );
    assert!(
        sources.contains("src/b.ts"),
        "the second member's paired source: {sources}"
    );
}

#[test]
fn the_finding_sits_at_the_first_member_in_file_and_span_order() {
    // Fed out of order: b before a, and a later span before an earlier one within a file.
    let model = model(vec![
        test_case("b.test.ts", 1, "beta", &["Suite"], Some("h1")),
        test_case("a.test.ts", 9, "late", &["Suite"], Some("h1")),
        test_case("a.test.ts", 2, "early", &["Suite"], Some("h1")),
    ]);
    let findings = findings(&model);
    assert_eq!(findings.len(), 1);
    let finding = &findings[0];
    assert_eq!(finding.location.file, "a.test.ts");
    assert_eq!(
        finding.location.line, 2,
        "the earliest member in a.test.ts leads"
    );
    assert_eq!(finding.anchor, "h1", "identity is the body hash");
    // The related members follow in file-and-span order: a.test.ts:9 then b.test.ts:1.
    let related: Vec<_> = finding
        .related
        .iter()
        .map(|r| (r.location.file.as_str(), r.location.line))
        .collect();
    assert_eq!(related, vec![("a.test.ts", 9), ("b.test.ts", 1)]);
}

/// An assertion carrying a matcher, so a member's assertion count can be set.
fn assertion() -> Assertion {
    Assertion {
        matcher: Some("toBe".to_owned()),
        argument: Some("1".to_owned()),
        subject: Some("x".to_owned()),
        subject_is_query: false,
        subject_is_call: false,
        negated: false,
        carries_value: true,
        span: Span::new(0, 1),
    }
}

#[test]
fn the_explain_names_both_tests_and_the_richer_one_to_keep() {
    let mut rich = test_case("a.test.ts", 5, "sums", &[], Some("h1"));
    rich.assertions = vec![assertion(), assertion()];
    let model = model(vec![
        test_case("a.test.ts", 1, "adds", &[], Some("h1")),
        rich,
    ]);
    let explain = &findings(&model)[0].explain;
    assert!(explain.contains("adds"), "names the first test: {explain}");
    assert!(explain.contains("sums"), "names the other test: {explain}");
    assert!(
        explain.contains("a.test.ts"),
        "says where each lives: {explain}"
    );
    assert!(
        explain.contains("Keep one — sums"),
        "says to keep the richer test (more assertions): {explain}"
    );
}
