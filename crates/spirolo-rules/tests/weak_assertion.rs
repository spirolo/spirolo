//! `weak-assertion` on hand-built models: one case per branch of its semantics.
//!
//! The rule is a pure function of the model, so these tests build `TestCase` rows by hand
//! and read the findings out — never how they were computed. The binary fixtures (Seam 2)
//! prove the same rule end to end through `spirolo check`.

use spirolo_model::{Assertion, Model, Span, TestCase, TestKind};
use spirolo_rules::{Config, Family, Finding, Outcome, Rule, RuleConfig, Severity, WeakAssertion};

/// An assertion with a matcher, a negation flag, and whether it carries a value.
fn assertion(matcher: &str, negated: bool, carries_value: bool, line: u32) -> Assertion {
    Assertion {
        matcher: Some(matcher.to_owned()),
        argument: None,
        subject: Some("subject".to_owned()),
        subject_is_query: false,
        subject_is_call: false,
        negated,
        carries_value,
        span: Span {
            start: line,
            end: line + 1,
            line,
            column: 3,
        },
    }
}

/// A test case in `file`, with a title and a list of assertions.
fn test_case(file: &str, title: &str, assertions: Vec<Assertion>) -> TestCase {
    TestCase {
        file: file.to_owned(),
        span: Span {
            start: 1,
            end: 2,
            line: 1,
            column: 1,
        },
        framework: Some("vitest".to_owned()),
        title: Some(title.to_owned()),
        kind: TestKind::Test,
        suite_path: Vec::new(),
        assertions,
        throwing_queries: Vec::new(),
        mocks: Vec::new(),
        skip: None,
        each: None,
        loops: Vec::new(),
        modifiers: Vec::new(),
        tags: Vec::new(),
        body_hash: None,
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

/// Every finding a config-less run of the rule produces.
fn findings(model: &Model) -> Vec<Finding> {
    WeakAssertion.run(model, &Config::default())
}

/// The per-test findings only (the project-wide ratio row has an empty file).
fn test_findings(model: &Model) -> Vec<Finding> {
    findings(model)
        .into_iter()
        .filter(|f| !f.location.file.is_empty())
        .collect()
}

#[test]
fn a_test_whose_every_assertion_carries_no_value_is_weak() {
    let m = model(vec![test_case(
        "a.test.ts",
        "is present",
        vec![
            assertion("toBeDefined", false, false, 2),
            assertion("toBeTruthy", false, false, 3),
        ],
    )]);

    let found = test_findings(&m);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].rule, "weak-assertion");
    assert_eq!(found[0].location.file, "a.test.ts");
    assert_eq!(found[0].severity, Severity::Warn);
    assert_eq!(found[0].outcome, Outcome::Finding);
}

#[test]
fn a_test_with_one_value_carrying_assertion_is_not_weak() {
    let m = model(vec![test_case(
        "a.test.ts",
        "checks the value",
        vec![
            assertion("toBeDefined", false, false, 2),
            assertion("toBe", false, true, 3),
        ],
    )]);

    assert!(
        test_findings(&m).is_empty(),
        "one value-carrying matcher anchors the test"
    );
}

#[test]
fn a_test_with_no_assertions_is_not_weak() {
    let m = model(vec![test_case("a.test.ts", "asserts nothing", Vec::new())]);
    assert!(
        test_findings(&m).is_empty(),
        "no assertion is not weak-here"
    );
}

#[test]
fn only_tests_are_judged_never_hooks_suites_benches_or_steps() {
    // A `beforeEach(() => { expect(fixture).toBeDefined() })` is a guard on the setup, not
    // a test that could pass against a wrong value; a `bench` measures, it does not assert
    // behaviour. Neither is a per-test finding, and neither counts in the ratio.
    for kind in [
        TestKind::Hook,
        TestKind::Suite,
        TestKind::Benchmark,
        TestKind::Step,
    ] {
        let mut row = test_case(
            "a.test.ts",
            "setup",
            vec![assertion("toBeDefined", false, false, 2)],
        );
        row.kind = kind;
        let found = findings(&model(vec![row]));
        assert!(found.is_empty(), "a {kind:?} row is not judged: {found:#?}");
    }
}

#[test]
fn the_ratio_counts_assertions_in_tests_only() {
    // One test with a value-carrying assertion, one hook with three value-less ones: the
    // ratio reads 0 of 1, not 3 of 4, so the hook cannot push the project over the ceiling.
    let mut hook = test_case(
        "a.test.ts",
        "setup",
        vec![
            assertion("toBeDefined", false, false, 2),
            assertion("toBeDefined", false, false, 3),
            assertion("toBeDefined", false, false, 4),
        ],
    );
    hook.kind = TestKind::Hook;
    let m = model(vec![
        hook,
        test_case(
            "a.test.ts",
            "strong",
            vec![assertion("toBe", false, true, 6)],
        ),
    ]);
    assert!(
        findings(&m).is_empty(),
        "the hook's assertions are not in the denominator or the numerator"
    );
}

#[test]
fn a_negated_value_less_matcher_shows_in_the_message_with_its_not_prefix() {
    // `not.toBeNull` carries no value: the matcher takes no argument, the negation is
    // separate. It is still weak, and the report should name it as `not.toBeNull`.
    let m = model(vec![test_case(
        "a.test.ts",
        "is not null",
        vec![assertion("toBeNull", true, false, 2)],
    )]);

    let found = test_findings(&m);
    assert_eq!(found.len(), 1);
    assert!(
        found[0].message.contains("not.toBeNull"),
        "message names the negated matcher: {}",
        found[0].message
    );
    assert!(
        found[0].explain.to_lowercase().contains("not delete")
            || found[0].explain.to_lowercase().contains("do not delete"),
        "explain never says to delete the assertion: {}",
        found[0].explain
    );
}

#[test]
fn the_explain_names_the_matchers_and_repairs_rather_than_deletes() {
    let m = model(vec![test_case(
        "a.test.ts",
        "is present",
        vec![assertion("toBeTruthy", false, false, 2)],
    )]);

    let found = test_findings(&m);
    assert!(found[0].explain.contains("toBeTruthy"));
    assert!(
        found[0].explain.contains("toBe") || found[0].explain.contains("toEqual"),
        "explain points at a value-carrying matcher: {}",
        found[0].explain
    );
}

/// The explain of the one finding for a test whose assertions are `rows`.
fn explain_of(rows: Vec<Assertion>) -> String {
    let found = test_findings(&model(vec![test_case("a.test.ts", "t", rows)]));
    assert_eq!(found.len(), 1);
    found[0].explain.clone()
}

#[test]
fn the_explain_is_true_for_each_matcher_it_names() {
    // The rule is the first consumer's gate, so the wording must hold for every matcher
    // named: `toBeTruthy` passes against any truthy value, `toBeDefined` against anything
    // but `undefined`, `not.toBeNull` against anything but `null`, and `toBeNull` on a
    // `queryBy*` subject whenever the element is absent.
    let truthy = explain_of(vec![assertion("toBeTruthy", false, false, 2)]);
    assert!(
        truthy.contains("`toBeTruthy` passes against any truthy value"),
        "{truthy}"
    );

    let defined = explain_of(vec![assertion("toBeDefined", false, false, 2)]);
    assert!(
        defined.contains("`toBeDefined` passes against any value but `undefined`"),
        "{defined}"
    );
    assert!(
        !defined.contains("truthy"),
        "toBeDefined is not a truthiness check: {defined}"
    );

    let not_null = explain_of(vec![assertion("toBeNull", true, false, 2)]);
    assert!(
        not_null.contains("`not.toBeNull` passes against any value but `null`"),
        "{not_null}"
    );

    let absent = explain_of(vec![Assertion {
        subject: Some("screen.queryByText('x')".to_owned()),
        subject_is_query: true,
        subject_is_call: false,
        ..assertion("toBeNull", false, false, 2)
    }]);
    assert!(
        absent.contains(
            "`toBeNull` on a `queryBy*` subject passes whenever the queried element is absent"
        ),
        "{absent}"
    );

    // Several matchers: one fragment each, every one present, no em dash anywhere.
    let mixed = explain_of(vec![
        assertion("toBeDefined", false, false, 2),
        assertion("toBeTruthy", false, false, 3),
        assertion("toBeUndefined", true, false, 4),
    ]);
    for fragment in [
        "`toBeDefined` passes against any value but `undefined`",
        "`toBeTruthy` passes against any truthy value",
        "`not.toBeUndefined` passes against any value but `undefined`",
    ] {
        assert!(mixed.contains(fragment), "missing {fragment:?} in {mixed}");
    }
    assert!(!mixed.contains('\u{2014}'), "no em dash: {mixed}");
}

#[test]
fn a_value_less_row_the_taxonomy_cannot_place_gets_a_generic_fragment() {
    // A model row can say `carries_value: false` for a matcher outside the table (another
    // recogniser, a hand-built model). The explain still names it and says only what is
    // known: it carries no expected value.
    let explain = explain_of(vec![assertion("toBeSomething", false, false, 2)]);
    assert!(
        explain.contains("`toBeSomething` carries no expected value"),
        "{explain}"
    );
}

#[test]
fn two_weak_tests_and_one_strong_test_report_exactly_the_two() {
    let m = model(vec![
        test_case(
            "a.test.ts",
            "weak one",
            vec![assertion("toBeDefined", false, false, 2)],
        ),
        test_case(
            "b.test.ts",
            "strong",
            vec![assertion("toBe", false, true, 2)],
        ),
        test_case(
            "c.test.ts",
            "weak two",
            vec![assertion("toBeTruthy", false, false, 2)],
        ),
    ]);

    let files: Vec<String> = test_findings(&m)
        .into_iter()
        .map(|f| f.location.file)
        .collect();
    assert_eq!(files, ["a.test.ts", "c.test.ts"]);
}

#[test]
fn findings_are_sorted_by_file() {
    let m = model(vec![
        test_case(
            "z.test.ts",
            "z",
            vec![assertion("toBeDefined", false, false, 2)],
        ),
        test_case(
            "a.test.ts",
            "a",
            vec![assertion("toBeDefined", false, false, 2)],
        ),
    ]);

    let files: Vec<String> = test_findings(&m)
        .into_iter()
        .map(|f| f.location.file)
        .collect();
    assert_eq!(
        files,
        ["a.test.ts", "z.test.ts"],
        "sorted, not source order"
    );
}

#[test]
fn the_ratio_row_fires_above_the_ceiling_and_is_silent_at_or_below() {
    // Four assertions, one value-less: ratio 25 %, well above the 6 % default.
    let m = model(vec![test_case(
        "a.test.ts",
        "strong",
        vec![
            assertion("toBe", false, true, 2),
            assertion("toBe", false, true, 3),
            assertion("toBe", false, true, 4),
            assertion("toBeDefined", false, false, 5),
        ],
    )]);

    let ratio: Vec<&Finding> = findings(&m)
        .iter()
        .filter(|f| f.location.file.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .leak()
        .iter()
        .collect();
    assert_eq!(ratio.len(), 1, "the ratio row fires above the ceiling");
    assert!(ratio[0].message.contains("1 of 4"), "{}", ratio[0].message);

    // A generous ceiling silences it: config sets maxRatio to 50 %.
    let mut config = Config::default();
    config.rules.insert(
        "weak-assertion".to_owned(),
        RuleConfig {
            severity: None,
            max_ratio: Some(0.5),
        },
    );
    let with_ceiling = WeakAssertion.run(&m, &config);
    assert!(
        with_ceiling.iter().all(|f| !f.location.file.is_empty()),
        "no ratio row at or below the ceiling"
    );
}

#[test]
fn the_ratio_row_sorts_by_the_crate_key_with_the_other_findings() {
    // The rule's contract is findings sorted by file, line, column, anchor. The ratio row
    // has no file, so it sorts before every located finding, not appended after them.
    let m = model(vec![
        test_case(
            "z.test.ts",
            "z",
            vec![assertion("toBeDefined", false, false, 2)],
        ),
        test_case(
            "a.test.ts",
            "a",
            vec![assertion("toBeTruthy", false, false, 2)],
        ),
    ]);

    let files: Vec<String> = findings(&m).into_iter().map(|f| f.location.file).collect();
    assert_eq!(
        files,
        ["", "a.test.ts", "z.test.ts"],
        "the project row takes its place in the sort, first"
    );
}

#[test]
fn the_configured_severity_overrides_the_default() {
    let mut config = Config::default();
    config.rules.insert(
        "weak-assertion".to_owned(),
        RuleConfig {
            severity: Some(Severity::Error),
            max_ratio: None,
        },
    );
    let m = model(vec![test_case(
        "a.test.ts",
        "weak",
        vec![assertion("toBeDefined", false, false, 2)],
    )]);

    let found = WeakAssertion.run(&m, &config);
    assert!(found.iter().all(|f| f.severity == Severity::Error));
}

#[test]
fn the_rule_declares_its_identity() {
    assert_eq!(WeakAssertion.name(), "weak-assertion");
    assert_eq!(WeakAssertion.family(), Family::Audit);
    assert_eq!(WeakAssertion.default_severity(), Severity::Warn);
}

#[test]
fn colliding_findings_take_distinct_ordinals() {
    // Two title-less tests in one file collide on the anchor's fallback; the ordinal must
    // keep their ids distinct.
    let mut a = test_case(
        "a.test.ts",
        "x",
        vec![assertion("toBeDefined", false, false, 2)],
    );
    a.title = None;
    a.span = Span {
        start: 10,
        end: 11,
        line: 10,
        column: 1,
    };
    let mut b = test_case(
        "a.test.ts",
        "x",
        vec![assertion("toBeDefined", false, false, 2)],
    );
    b.title = None;
    b.span = Span {
        start: 20,
        end: 21,
        line: 20,
        column: 1,
    };

    let ids: Vec<String> = test_findings(&model(vec![a, b]))
        .into_iter()
        .map(|f| f.id())
        .collect();
    assert_eq!(ids.len(), 2);
    assert_ne!(ids[0], ids[1], "ordinals keep colliding ids apart: {ids:?}");
}
