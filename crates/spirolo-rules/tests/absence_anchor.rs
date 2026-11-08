//! `absence-anchor` on hand-built models: one case per branch of its semantics — a test whose
//! every DOM assertion is negated with no throwing query (a finding), the same test with a
//! throwing query in the body (silent), the same test with a positive DOM assertion (silent),
//! a negation reached through `waitFor` still counted, the near-misses that must not fire
//! (a test with no DOM assertion at all, a non-DOM negated assertion), and the unread
//! assertions that silence the rule (a value-bearing or value-less matcher on a plain
//! subject beside the negation: the anchor the model cannot see).
//!
//! The rule is a pure function of the model, so these tests build `TestCase` rows by hand and
//! read the findings out. The binary fixture (Seam 2) proves the same rule end to end through
//! `spirolo check`, where the recogniser fills `assertions` and `throwing_queries` from source.

use spirolo_model::{Assertion, Model, Span, TestCase, TestKind, ThrowingQuery};
use spirolo_rules::{AbsenceAnchor, Config, Family, Finding, Outcome, Rule, Severity};

/// A DOM assertion: a jest-dom matcher (`toBeInTheDocument` and kin), possibly negated.
fn dom_assertion(matcher: &str, negated: bool) -> Assertion {
    Assertion {
        matcher: Some(matcher.to_owned()),
        argument: None,
        subject: Some("screen.queryByText('x')".to_owned()),
        subject_is_query: false,
        subject_is_call: false,
        negated,
        carries_value: false,
        span: Span {
            start: 40,
            end: 80,
            line: 5,
            column: 1,
        },
    }
}

/// A throwing Testing Library query in a body.
fn throwing_query(name: &str) -> ThrowingQuery {
    ThrowingQuery {
        name: name.to_owned(),
        within: false,
        span: Span {
            start: 30,
            end: 50,
            line: 4,
            column: 1,
        },
    }
}

/// A test case carrying `assertions` and `throwing_queries`.
fn test_case(
    file: &str,
    title: &str,
    assertions: Vec<Assertion>,
    throwing_queries: Vec<ThrowingQuery>,
) -> TestCase {
    TestCase {
        file: file.to_owned(),
        span: Span {
            start: 10,
            end: 100,
            line: 2,
            column: 1,
        },
        framework: Some("vitest".to_owned()),
        title: Some(title.to_owned()),
        kind: TestKind::Test,
        suite_path: Vec::new(),
        assertions,
        throwing_queries,
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

/// Every finding the rule produces for `model` under the default config.
fn findings(cases: Vec<TestCase>) -> Vec<Finding> {
    AbsenceAnchor.run(&model(cases), &Config::default())
}

// ---------------------------------------------------------------- the acceptance criteria

#[test]
fn a_test_with_only_a_negated_dom_assertion_is_a_finding() {
    // `expect(screen.queryByText('x')).not.toBeInTheDocument()` and nothing else → finding:
    // an empty body after a crashed render satisfies the negation trivially.
    let found = findings(vec![test_case(
        "src/widget.test.tsx",
        "hides the banner",
        vec![dom_assertion("toBeInTheDocument", true)],
        Vec::new(),
    )]);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].rule, "absence-anchor");
    assert_eq!(found[0].severity, Severity::Warn);
    assert_eq!(found[0].outcome, Outcome::Finding);
    assert_eq!(found[0].location.file, "src/widget.test.tsx");
}

#[test]
fn a_throwing_query_in_the_body_anchors_the_render_so_no_finding() {
    // The same negated assertion, but the body also calls `getByRole` — a throwing query that
    // anchors the render, so the negation is meaningful. No finding.
    let found = findings(vec![test_case(
        "src/widget.test.tsx",
        "hides the banner",
        vec![dom_assertion("toBeInTheDocument", true)],
        vec![throwing_query("getByRole")],
    )]);
    assert!(found.is_empty(), "a throwing query anchors it: {found:#?}");
}

#[test]
fn a_positive_dom_assertion_anchors_the_render_so_no_finding() {
    // A non-negated DOM assertion alongside the negated one is a real anchor: the render
    // produced something the test asserts on. No finding.
    let found = findings(vec![test_case(
        "src/widget.test.tsx",
        "swaps the banner",
        vec![
            dom_assertion("toBeInTheDocument", true),
            dom_assertion("toBeVisible", false),
        ],
        Vec::new(),
    )]);
    assert!(
        found.is_empty(),
        "a positive DOM assertion anchors it: {found:#?}"
    );
}

#[test]
fn a_negation_inside_wait_for_is_counted() {
    // A negated DOM assertion collected from inside a `waitFor(…)` callback is still the
    // case's assertion (the collector descends the subtree), so a test whose only DOM
    // assertion is that negation, with no throwing query, is a finding.
    let found = findings(vec![test_case(
        "src/async.test.tsx",
        "removes on click",
        vec![dom_assertion("toHaveTextContent", true)],
        Vec::new(),
    )]);
    assert_eq!(found.len(), 1, "the waitFor negation counts: {found:#?}");
}

// --------------------------------------------------------------- the queryBy absence idiom

/// An assertion on a `queryBy*` / `queryAllBy*` subject: `matcher`, its negation and its
/// first argument as written.
fn query_assertion(matcher: &str, negated: bool, argument: Option<&str>) -> Assertion {
    Assertion {
        matcher: Some(matcher.to_owned()),
        argument: argument.map(str::to_owned),
        subject: Some("screen.queryByText('x')".to_owned()),
        subject_is_query: true,
        subject_is_call: false,
        negated,
        carries_value: false,
        span: Span {
            start: 40,
            end: 80,
            line: 5,
            column: 1,
        },
    }
}

#[test]
fn the_query_absence_idiom_alone_is_a_finding() {
    // `expect(screen.queryByText('x')).toBeNull()` is the most common way to assert absence
    // and holds against an empty body exactly as `not.toBeInTheDocument` does: a test made
    // only of it, with no anchor, is a finding. Every spelling of the idiom counts.
    for (matcher, negated, argument) in [
        ("toBeNull", false, None),
        ("toBeUndefined", false, None),
        ("toBeFalsy", false, None),
        ("toBeTruthy", true, None),
        ("toHaveLength", false, Some("0")),
        ("toEqual", false, Some("[]")),
        ("toStrictEqual", false, Some("[]")),
    ] {
        let found = findings(vec![test_case(
            "src/widget.test.tsx",
            "hides the banner",
            vec![query_assertion(matcher, negated, argument)],
            Vec::new(),
        )]);
        assert_eq!(found.len(), 1, "{matcher} on a query subject: {found:#?}");
        let spelled = &found[0].evidence[0].value;
        assert!(
            spelled.starts_with("queryBy*."),
            "the evidence spells the idiom on its subject: {spelled}"
        );
    }
}

#[test]
fn the_same_matchers_off_a_query_subject_are_not_absence() {
    // `expect(f(x)).toBeNull()` asserts a produced value, not a missing element: without
    // the query subject the row is not a DOM read and the rule stays silent.
    for (matcher, negated, argument) in [
        ("toBeNull", false, None),
        ("toBeTruthy", true, None),
        ("toHaveLength", false, Some("0")),
        ("toEqual", false, Some("[]")),
    ] {
        let found = findings(vec![test_case(
            "src/logic.test.ts",
            "returns nothing",
            vec![Assertion {
                subject: Some("f(x)".to_owned()),
                subject_is_query: false,
                subject_is_call: false,
                ..query_assertion(matcher, negated, argument)
            }],
            Vec::new(),
        )]);
        assert!(found.is_empty(), "{matcher} on a plain subject: {found:#?}");
    }
}

#[test]
fn a_query_subject_with_a_presence_matcher_is_not_absence() {
    // `expect(screen.queryByText('x')).not.toBeNull()` asserts presence; it fails against
    // an empty body, so it is not an absence read (nor a DOM anchor: the rule reads the
    // jest-dom matchers only). `toHaveLength(2)` on a `queryAllBy*` likewise names a count
    // an empty body does not have.
    for (matcher, negated, argument) in [
        ("toBeNull", true, None),
        ("toBeTruthy", false, None),
        ("toHaveLength", false, Some("2")),
        ("toEqual", false, Some("[row]")),
    ] {
        let found = findings(vec![test_case(
            "src/widget.test.tsx",
            "shows the banner",
            vec![query_assertion(matcher, negated, argument)],
            Vec::new(),
        )]);
        assert!(found.is_empty(), "{matcher} names presence: {found:#?}");
    }
}

#[test]
fn the_idiom_and_a_negated_dom_matcher_together_are_one_finding() {
    let found = findings(vec![test_case(
        "src/widget.test.tsx",
        "hides everything",
        vec![
            query_assertion("toBeNull", false, None),
            dom_assertion("toBeInTheDocument", true),
        ],
        Vec::new(),
    )]);
    assert_eq!(found.len(), 1, "one finding per test: {found:#?}");
    assert_eq!(
        found[0].evidence[0].value,
        "not.toBeInTheDocument, queryBy*.toBeNull"
    );
}

// --------------------------------------------------------------------- toBeEmptyDOMElement

#[test]
fn to_be_empty_dom_element_does_not_anchor_a_render() {
    // `expect(container).toBeEmptyDOMElement()` holds against an empty container after a
    // crash as readily as before it: it is an absence read, not an anchor. Next to a
    // negated `toBeInTheDocument` it leaves the test unanchored; alone it is a finding too.
    let found = findings(vec![test_case(
        "src/widget.test.tsx",
        "renders nothing for an empty list",
        vec![
            dom_assertion("toBeInTheDocument", true),
            dom_assertion("toBeEmptyDOMElement", false),
        ],
        Vec::new(),
    )]);
    assert_eq!(
        found.len(),
        1,
        "toBeEmptyDOMElement is no anchor: {found:#?}"
    );
    assert_eq!(
        found[0].evidence[0].value,
        "not.toBeInTheDocument, toBeEmptyDOMElement"
    );

    let alone = findings(vec![test_case(
        "src/widget.test.tsx",
        "renders nothing for an empty list",
        vec![dom_assertion("toBeEmptyDOMElement", false)],
        Vec::new(),
    )]);
    assert_eq!(
        alone.len(),
        1,
        "alone it asserts only emptiness: {alone:#?}"
    );
}

#[test]
fn a_negated_to_be_empty_dom_element_anchors_the_render() {
    // `expect(container).not.toBeEmptyDOMElement()` fails against an empty body: the one
    // jest-dom negation that anchors. It silences the negated `toBeInTheDocument` beside it.
    let found = findings(vec![test_case(
        "src/widget.test.tsx",
        "hides the banner but renders the rest",
        vec![
            dom_assertion("toBeInTheDocument", true),
            dom_assertion("toBeEmptyDOMElement", true),
        ],
        Vec::new(),
    )]);
    assert!(
        found.is_empty(),
        "not.toBeEmptyDOMElement anchors: {found:#?}"
    );
}

// ------------------------------------------------------------------------ spy assertions

/// A call assertion on a spy: `matcher`, its negation and its first argument as written.
fn spy_assertion(matcher: &str, negated: bool, argument: Option<&str>) -> Assertion {
    Assertion {
        matcher: Some(matcher.to_owned()),
        argument: argument.map(str::to_owned),
        subject: Some("onClose".to_owned()),
        subject_is_query: false,
        subject_is_call: false,
        negated,
        carries_value: true,
        span: Span {
            start: 45,
            end: 85,
            line: 6,
            column: 1,
        },
    }
}

#[test]
fn a_plain_call_assertion_on_a_spy_anchors_the_render() {
    // `expect(onClose).toHaveBeenCalled()` after the render: a component that crashed on
    // mount made no call, so the assertion fails against the empty body and anchors the
    // negation beside it. Every plain call, return and count spelling counts, Vitest and
    // Jest alike, a count only when it is not the literal 0.
    for (matcher, argument) in [
        ("toHaveBeenCalled", None),
        ("toHaveBeenCalledWith", Some("'a'")),
        ("toHaveBeenCalledTimes", Some("1")),
        ("toHaveBeenCalledTimes", Some("n")),
        ("toHaveBeenCalledOnce", None),
        ("toHaveBeenCalledExactlyOnceWith", Some("'a'")),
        ("toHaveBeenLastCalledWith", Some("'a'")),
        ("toHaveBeenNthCalledWith", Some("1")),
        ("toHaveReturned", None),
        ("toHaveReturnedWith", Some("1")),
        ("toHaveResolved", None),
        ("toBeCalled", None),
        ("toBeCalledWith", Some("'a'")),
        ("toBeCalledTimes", Some("2")),
        ("lastCalledWith", Some("'a'")),
        ("nthCalledWith", Some("1")),
        ("toReturn", None),
    ] {
        let found = findings(vec![test_case(
            "src/dialog.test.tsx",
            "closes on escape",
            vec![
                dom_assertion("toBeInTheDocument", true),
                spy_assertion(matcher, false, argument),
            ],
            Vec::new(),
        )]);
        assert!(found.is_empty(), "{matcher} anchors: {found:#?}");
    }
}

#[test]
fn a_negated_or_zero_count_call_assertion_does_not_anchor() {
    // `not.toHaveBeenCalled()` and `toHaveBeenCalledTimes(0)` hold when nothing rendered:
    // they are not anchors, so the negated DOM read beside them is still a finding.
    for (matcher, negated, argument) in [
        ("toHaveBeenCalled", true, None),
        ("toHaveBeenCalledWith", true, Some("'a'")),
        ("toHaveBeenCalledTimes", false, Some("0")),
        ("toBeCalledTimes", false, Some("0")),
        ("toHaveReturnedTimes", false, Some("0")),
        ("toHaveBeenCalledTimes", true, Some("1")),
    ] {
        let found = findings(vec![test_case(
            "src/dialog.test.tsx",
            "does not close on click outside",
            vec![
                dom_assertion("toBeInTheDocument", true),
                spy_assertion(matcher, negated, argument),
            ],
            Vec::new(),
        )]);
        assert_eq!(
            found.len(),
            1,
            "{}{matcher}({}) is no anchor: {found:#?}",
            if negated { "not." } else { "" },
            argument.unwrap_or_default()
        );
    }
}

/// An assertion the rule does not read: a plain-subject matcher, with or without a value.
fn plain_assertion(
    matcher: &str,
    negated: bool,
    argument: Option<&str>,
    subject: &str,
) -> Assertion {
    Assertion {
        matcher: Some(matcher.to_owned()),
        argument: argument.map(str::to_owned),
        subject: Some(subject.to_owned()),
        subject_is_query: false,
        subject_is_call: false,
        negated,
        carries_value: argument.is_some(),
        span: Span {
            start: 90,
            end: 120,
            line: 7,
            column: 1,
        },
    }
}

#[test]
fn a_value_bearing_assertion_beside_the_negation_is_left_unread_so_no_finding() {
    // `expect(container.innerHTML).toBe('<div…>')`, `expect(captured).toHaveLength(1)` and
    // `expect(window.location.href).toBe('/account')` are values the render may have
    // produced through a side effect — the model cannot tell them from a value the test
    // built, and the crash oracle showed they usually anchor. Any such assertion silences
    // the rule: the finding is made only when every assertion is an absence read or inert.
    for (matcher, argument, subject) in [
        ("toBe", "'<div class=\"x\"></div>'", "container.innerHTML"),
        ("toHaveLength", "1", "captured"),
        ("toEqual", "{ columns: [] }", "captured[0].props"),
        ("toBe", "'/account'", "window.location.href"),
        ("toMatchSnapshot", "", "container"),
    ] {
        let found = findings(vec![test_case(
            "src/block.test.tsx",
            "renders no title when absent",
            vec![
                dom_assertion("toBeInTheDocument", true),
                plain_assertion(
                    matcher,
                    false,
                    (!argument.is_empty()).then_some(argument),
                    subject,
                ),
            ],
            Vec::new(),
        )]);
        assert!(
            found.is_empty(),
            "expect({subject}).{matcher}({argument}) is unread and silences the rule: {found:#?}"
        );
    }
}

#[test]
fn a_value_less_or_unknown_assertion_on_a_plain_subject_is_left_unread_too() {
    // `expect(container.querySelector('.x')).not.toBeNull()` fails on an empty body while
    // `expect(flag).toBeTruthy()` does not; the rule cannot tell them apart and reads
    // neither, so each silences it. So does a matcher the tables do not know.
    for (matcher, negated, subject) in [
        ("toBeNull", true, "container.querySelector('.spinner')"),
        ("toBeTruthy", false, "flag"),
        ("toBeDefined", false, "result.current"),
        ("toBeAccessible", false, "container"),
    ] {
        let found = findings(vec![test_case(
            "src/block.test.tsx",
            "shows the spinner while pending",
            vec![
                dom_assertion("toBeInTheDocument", true),
                plain_assertion(matcher, negated, None, subject),
            ],
            Vec::new(),
        )]);
        assert!(
            found.is_empty(),
            "expect({subject}).{}{matcher}() is unread and silences the rule: {found:#?}",
            if negated { "not." } else { "" }
        );
    }
}

#[test]
fn an_inert_spy_assertion_beside_the_negation_leaves_the_finding_standing() {
    // `not.toHaveBeenCalled()` holds against an empty body whatever rendered: it is read
    // (inert), not unread, so it does not silence the rule the way a plain-subject
    // assertion does. The finding names only the absence reads.
    let found = findings(vec![test_case(
        "src/panel.test.tsx",
        "renders nothing without a brandId",
        vec![
            dom_assertion("toBeEmptyDOMElement", false),
            spy_assertion("toHaveBeenCalled", true, None),
        ],
        Vec::new(),
    )]);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(
        found[0]
            .evidence
            .iter()
            .any(|e| e.value == "toBeEmptyDOMElement"),
        "{:#?}",
        found[0].evidence
    );
}

#[test]
fn a_spy_assertion_alone_is_not_a_finding() {
    // No absence read, no finding: the spy assertion is an anchor, not a trigger.
    let found = findings(vec![test_case(
        "src/dialog.test.tsx",
        "calls onClose",
        vec![spy_assertion("toHaveBeenCalled", false, None)],
        Vec::new(),
    )]);
    assert!(found.is_empty(), "{found:#?}");
}

// --------------------------------------------------------------- hook-level throwing queries

/// A hook row (`beforeEach` and kin) in `file` under `suite_path`, carrying
/// `throwing_queries` and nothing else.
fn hook(file: &str, suite_path: &[&str], throwing_queries: Vec<ThrowingQuery>) -> TestCase {
    TestCase {
        title: None,
        kind: TestKind::Hook,
        suite_path: suite_path.iter().map(|s| (*s).to_owned()).collect(),
        span: Span {
            start: 5,
            end: 9,
            line: 1,
            column: 1,
        },
        ..test_case(file, "", Vec::new(), throwing_queries)
    }
}

/// The negated-only test under `suite_path` in `file`.
fn unanchored(file: &str, suite_path: &[&str]) -> TestCase {
    TestCase {
        suite_path: suite_path.iter().map(|s| (*s).to_owned()).collect(),
        ..test_case(
            file,
            "hides the banner",
            vec![dom_assertion("toBeInTheDocument", true)],
            Vec::new(),
        )
    }
}

#[test]
fn a_throwing_query_in_an_enclosing_hook_anchors_the_test() {
    // `beforeEach(() => { render(<X/>); screen.getByRole('main') })` throws for every test
    // under it when the render crashed, so the hook anchors each of them: in the same
    // describe, in an outer describe, and at file level.
    for hook_path in [&["Widget", "collapsed"][..], &["Widget"][..], &[][..]] {
        let found = findings(vec![
            hook(
                "src/widget.test.tsx",
                hook_path,
                vec![throwing_query("getByRole")],
            ),
            unanchored("src/widget.test.tsx", &["Widget", "collapsed"]),
        ]);
        assert!(
            found.is_empty(),
            "a hook at {hook_path:?} anchors: {found:#?}"
        );
    }
}

#[test]
fn a_hook_elsewhere_does_not_anchor_the_test() {
    // A hook in a sibling describe, a deeper describe, or another file runs around other
    // tests, not this one; a hook with no throwing query anchors nothing.
    for (file, hook_path, queries) in [
        (
            "src/widget.test.tsx",
            &["Widget", "expanded"][..],
            vec![throwing_query("getByRole")],
        ),
        (
            "src/widget.test.tsx",
            &["Widget", "collapsed", "inner"][..],
            vec![throwing_query("getByRole")],
        ),
        (
            "src/other.test.tsx",
            &["Widget", "collapsed"][..],
            vec![throwing_query("getByRole")],
        ),
        (
            "src/widget.test.tsx",
            &["Widget", "collapsed"][..],
            Vec::new(),
        ),
    ] {
        let found = findings(vec![
            hook(file, hook_path, queries),
            unanchored("src/widget.test.tsx", &["Widget", "collapsed"]),
        ]);
        assert_eq!(
            found.len(),
            1,
            "a hook in {file} at {hook_path:?} does not anchor: {found:#?}"
        );
    }
}

#[test]
fn a_plain_dom_assertion_in_an_enclosing_hook_anchors_the_test() {
    // `beforeEach(() => expect(screen.getByRole('main')).toBeInTheDocument())`: the hook's
    // assertion fails the test when the render crashed, the same way its query does.
    let mut row = hook("src/widget.test.tsx", &["Widget"], Vec::new());
    row.assertions = vec![dom_assertion("toBeInTheDocument", false)];
    let found = findings(vec![row, unanchored("src/widget.test.tsx", &["Widget"])]);
    assert!(
        found.is_empty(),
        "a hook's anchor assertion anchors: {found:#?}"
    );

    let mut negated = hook("src/widget.test.tsx", &["Widget"], Vec::new());
    negated.assertions = vec![dom_assertion("toBeInTheDocument", true)];
    let found = findings(vec![
        negated,
        unanchored("src/widget.test.tsx", &["Widget"]),
    ]);
    assert_eq!(
        found.len(),
        1,
        "a hook's absence read anchors nothing: {found:#?}"
    );
}

// ------------------------------------------------------------------------- the near-misses

#[test]
fn a_test_with_no_dom_assertion_is_not_a_finding() {
    // A test that negates a non-DOM matcher (`expect(x).not.toBe(1)`) is not a render
    // absence-anchor: the rule reads DOM matchers only.
    let non_dom = Assertion {
        matcher: Some("toBe".to_owned()),
        argument: Some("1".to_owned()),
        subject: Some("x".to_owned()),
        subject_is_query: false,
        subject_is_call: false,
        negated: true,
        carries_value: true,
        span: Span {
            start: 40,
            end: 60,
            line: 5,
            column: 1,
        },
    };
    let found = findings(vec![test_case(
        "src/logic.test.ts",
        "does not equal one",
        vec![non_dom],
        Vec::new(),
    )]);
    assert!(
        found.is_empty(),
        "a non-DOM negation is not a render anchor: {found:#?}"
    );
}

#[test]
fn a_test_with_no_assertions_is_not_a_finding() {
    let found = findings(vec![test_case(
        "src/empty.test.tsx",
        "does nothing",
        Vec::new(),
        Vec::new(),
    )]);
    assert!(found.is_empty(), "no assertions, no finding: {found:#?}");
}

#[test]
fn only_tests_are_judged_never_hooks_suites_benches_or_steps() {
    // A `beforeEach` that asserts an element is gone is not a test: it has no title to
    // stand behind and the render it guards belongs to the tests that follow. The rule
    // reads `TestKind::Test` rows only.
    for kind in [
        TestKind::Hook,
        TestKind::Suite,
        TestKind::Benchmark,
        TestKind::Step,
    ] {
        let mut row = test_case(
            "src/widget.test.tsx",
            "resets",
            vec![dom_assertion("toBeInTheDocument", true)],
            Vec::new(),
        );
        row.kind = kind;
        let found = findings(vec![row]);
        assert!(found.is_empty(), "a {kind:?} row is not judged: {found:#?}");
    }
}

#[test]
fn several_negated_dom_matchers_and_still_no_anchor_is_one_finding() {
    // Two different negated DOM matchers, no positive one, no throwing query → still one
    // finding for the test, not one per matcher.
    let found = findings(vec![test_case(
        "src/widget.test.tsx",
        "shows nothing",
        vec![
            dom_assertion("toBeInTheDocument", true),
            dom_assertion("toBeVisible", true),
        ],
        Vec::new(),
    )]);
    assert_eq!(found.len(), 1, "one finding per test: {found:#?}");
}

#[test]
fn a_within_scoped_throwing_query_anchors_the_render() {
    // A `within(...).getByTestId(...)` query is a throwing anchor too, so it silences the rule.
    let found = findings(vec![test_case(
        "src/widget.test.tsx",
        "scoped absence",
        vec![dom_assertion("toBeInTheDocument", true)],
        vec![ThrowingQuery {
            name: "getByTestId".to_owned(),
            within: true,
            span: Span {
                start: 30,
                end: 55,
                line: 4,
                column: 1,
            },
        }],
    )]);
    assert!(found.is_empty(), "a within query anchors it: {found:#?}");
}

// --------------------------------------------------------------------------- identity & shape

#[test]
fn the_rule_belongs_to_the_audit_family() {
    assert_eq!(AbsenceAnchor.family(), Family::Audit);
    assert_eq!(AbsenceAnchor.name(), "absence-anchor");
    assert_eq!(AbsenceAnchor.default_severity(), Severity::Warn);
}

#[test]
fn the_finding_explains_the_reported_error_mechanism() {
    let found = findings(vec![test_case(
        "src/widget.test.tsx",
        "hides the banner",
        vec![dom_assertion("toBeInTheDocument", true)],
        Vec::new(),
    )]);
    assert_eq!(found.len(), 1);
    let explain = &found[0].explain;
    // The explain text says WHY: an uncaught render error is reported, not rethrown, so an
    // empty body satisfies the negation (prep doc A7).
    assert!(
        explain.contains("reportError") || explain.to_lowercase().contains("reported"),
        "explain names the reported-not-rethrown mechanism: {explain}"
    );
    // The evidence names the negated DOM matchers.
    assert!(
        found[0]
            .evidence
            .iter()
            .any(|e| e.value.contains("toBeInTheDocument")),
        "evidence names the matcher: {:#?}",
        found[0].evidence
    );
}

#[test]
fn a_severity_override_is_honoured() {
    use spirolo_rules::RuleConfig;
    let mut config = Config::default();
    config.rules.insert(
        "absence-anchor".to_owned(),
        RuleConfig {
            severity: Some(Severity::Error),
            ..RuleConfig::default()
        },
    );
    let found = AbsenceAnchor.run(
        &model(vec![test_case(
            "src/widget.test.tsx",
            "hides the banner",
            vec![dom_assertion("toBeInTheDocument", true)],
            Vec::new(),
        )]),
        &config,
    );
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].severity,
        Severity::Error,
        "override to error applied"
    );
}

#[test]
fn a_playwright_test_is_never_a_finding() {
    // expo `e2e/playwright/prod/static-loader.test.ts:122`, `await
    // expect(suspenseFallback).not.toBeVisible()`, with the locator held outside the test's
    // callback so the subject reads as a plain name: a Playwright test drives a page
    // (`page.goto`, `page.click`) that fails on its own, so it is no render test to anchor.
    let mut case = test_case(
        "e2e/playwright/prod/static-loader.test.ts",
        "shows suspense fallback while loading",
        vec![Assertion {
            subject: Some("suspenseFallback".to_owned()),
            ..dom_assertion("toBeVisible", true)
        }],
        Vec::new(),
    );
    case.framework = Some("playwright".to_owned());
    let found = findings(vec![case]);
    assert!(found.is_empty(), "{found:#?}");
}
