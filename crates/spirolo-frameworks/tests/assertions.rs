//! Assertions on a `TestCase`: `expect` chains, `assert*` and `expectTypeOf`, each with
//! its matcher, negation flag and span. Reads only the rows the recogniser returns.

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use spirolo_frameworks::VitestJest;
use spirolo_model::{Assertion, FileContext, Language, Recogniser, TestCase, TestKind};

/// Run the recogniser over one TypeScript snippet, as if it were `src/example.test.ts`.
fn recognise(source: &str) -> Vec<TestCase> {
    let allocator = Allocator::default();
    let path = "src/example.test.ts";
    let parsed = Parser::new(&allocator, source, SourceType::from_path(path).unwrap()).parse();
    let file = FileContext {
        path,
        language: Language::TypeScript,
        source,
        program: &parsed.program,
        handler_globs: None,
    };
    VitestJest.recognise(&file)
}

/// The assertions of the first test in a snippet.
fn assertions_of(source: &str) -> Vec<Assertion> {
    recognise(source)
        .into_iter()
        .find(|row| row.kind == TestKind::Test)
        .expect("a test")
        .assertions
}

// ----------------------------------------------------------------- one snapshot per idiom

#[test]
fn expect_to_be() {
    insta::assert_json_snapshot!(recognise(
        "import { it } from 'vitest'\nit('t', () => { expect(result).toBe(42) })\n"
    ));
}

#[test]
fn expect_not_negates() {
    insta::assert_json_snapshot!(recognise(
        "import { it } from 'vitest'\nit('t', () => { expect(value).not.toBe(0) })\n"
    ));
}

#[test]
fn value_less_matcher_carries_no_value() {
    insta::assert_json_snapshot!(recognise(
        "import { it } from 'vitest'\nit('t', () => { expect(value).toBeDefined() })\n"
    ));
}

#[test]
fn resolves_matcher_reads_through() {
    insta::assert_json_snapshot!(recognise(
        "import { it } from 'vitest'\nit('t', async () => { await expect(promise).resolves.toBe(1) })\n"
    ));
}

#[test]
fn assert_style() {
    insta::assert_json_snapshot!(recognise(
        "import { assert } from 'vitest'\nit('t', () => { assert.equal(a, b); assert(ok) })\n"
    ));
}

#[test]
fn expect_type_of() {
    insta::assert_json_snapshot!(recognise(
        "import { it, expectTypeOf } from 'vitest'\nit('t', () => { expectTypeOf(x).toEqualTypeOf<number>() })\n"
    ));
}

#[test]
fn many_expects_in_one_test() {
    insta::assert_json_snapshot!(recognise(
        "import { it } from 'vitest'\n\
         it('t', () => {\n  expect(a).toBe(1)\n  expect(b).not.toEqual(2)\n  expect(c).toBeNull()\n})\n"
    ));
}

#[test]
fn expect_soft_reads_through() {
    insta::assert_json_snapshot!(recognise(
        "import { it, expect } from 'vitest'\nit('t', () => { expect.soft(a).toBe(1) })\n"
    ));
}

#[test]
fn expect_poll_reads_through() {
    insta::assert_json_snapshot!(recognise(
        "import { it, expect } from 'vitest'\nit('t', async () => { await expect.poll(() => x).toBe(1) })\n"
    ));
}

// ------------------------------------------------------------ what the snapshots must hold

#[test]
fn a_soft_or_polled_expect_is_an_assertion_with_its_matcher() {
    let soft = assertions_of(
        "import { it, expect } from 'vitest'\nit('t', () => { expect.soft(a).not.toBe(1) })\n",
    );
    assert_eq!(soft.len(), 1, "expect.soft(x).matcher() is one assertion");
    assert_eq!(soft[0].matcher.as_deref(), Some("toBe"));
    assert_eq!(soft[0].subject.as_deref(), Some("a"));
    assert!(soft[0].negated);

    let polled = assertions_of(
        "import { it, expect } from 'vitest'\nit('t', async () => { await expect.poll(() => x).toBeTruthy() })\n",
    );
    assert_eq!(
        polled.len(),
        1,
        "expect.poll(fn).matcher() is one assertion"
    );
    assert_eq!(polled[0].matcher.as_deref(), Some("toBeTruthy"));
    assert_eq!(polled[0].subject.as_deref(), Some("() => x"));
    assert!(!polled[0].carries_value);

    let element = assertions_of(
        "import { it, expect } from 'vitest'\nit('t', async () => { await expect.element(page.getByText('x')).toBeVisible() })\n",
    );
    assert_eq!(
        element.len(),
        1,
        "expect.element(locator).matcher() is one assertion"
    );
    assert_eq!(element[0].matcher.as_deref(), Some("toBeVisible"));
}

#[test]
fn an_unknown_expect_static_call_is_not_an_assertion() {
    let rows = assertions_of(
        "import { it, expect } from 'vitest'\nit('t', () => { expect.assertions(2); expect.extend({}) })\n",
    );
    assert!(
        rows.is_empty(),
        "expect.assertions / expect.extend configure expect, they assert nothing: {rows:?}"
    );
}

#[test]
fn a_matcher_records_its_name_negation_and_span() {
    let assertions = assertions_of(
        "import { it } from 'vitest'\nit('t', () => { expect(value).not.toBe(0) })\n",
    );

    assert_eq!(assertions.len(), 1);
    let row = &assertions[0];
    assert_eq!(row.matcher.as_deref(), Some("toBe"));
    assert!(row.negated, "the .not is recorded");
    assert!(row.carries_value, "toBe(0) carries a value");
    assert_eq!(row.subject.as_deref(), Some("value"));
    assert!(
        row.span.start > 0 && row.span.end > row.span.start,
        "a real span"
    );
}

#[test]
fn a_double_negation_reads_as_positive() {
    // `.not.not` cancels the way the runner evaluates it, so the row is not negated.
    let assertions = assertions_of(
        "import { it } from 'vitest'\nit('t', () => { expect(v).not.not.toBe(1) })\n",
    );
    assert_eq!(assertions.len(), 1);
    assert!(!assertions[0].negated, "two nots cancel");
}

#[test]
fn a_value_less_matcher_is_marked_as_carrying_none() {
    let assertions =
        assertions_of("import { it } from 'vitest'\nit('t', () => { expect(v).toBeDefined() })\n");
    assert_eq!(assertions.len(), 1);
    assert!(
        !assertions[0].carries_value,
        "toBeDefined() carries no value"
    );
}

// ------------------------------------------------- the value-less taxonomy, read from source

/// The `carries_value` of the single assertion in `body`, inside one test.
fn carries_value(body: &str) -> bool {
    let assertions = assertions_of(&format!(
        "import {{ it, expect }} from 'vitest'\nit('t', async () => {{ {body} }})\n"
    ));
    assert_eq!(assertions.len(), 1, "one assertion in {body}");
    assertions[0].carries_value
}

#[test]
fn the_unconditional_value_less_set_reads_through() {
    for body in [
        "expect(v).toBeTruthy()",
        "expect(v).toBeDefined()",
        "expect(v).not.toBeNull()",
        "expect(v).not.toBeUndefined()",
        "expect(v).not.toBeFalsy()",
        "expect(f(x)).toBeTruthy()",
        "await expect(promise).resolves.toBeDefined()",
    ] {
        assert!(!carries_value(body), "{body} is value-less");
    }
}

#[test]
fn an_argument_less_matcher_outside_the_set_carries_a_value() {
    // The argument count is not the test: each of these asserts one specific fact.
    for body in [
        "expect(parseJudgeReport({})).toBeNull()",
        "expect(f(x)).toBeUndefined()",
        "expect(f(x)).toBeFalsy()",
        "expect(() => f()).toThrow()",
        "expect(() => f()).not.toThrow()",
        "expect(spy).toHaveBeenCalled()",
        "expect(freeArticleSpy).not.toHaveBeenCalled()",
        "expect(spy).toHaveBeenCalledOnce()",
        "expect(screen.getByText('x')).toBeInTheDocument()",
        "expect(screen.queryByText('x')).not.toBeInTheDocument()",
        "expect(el).toBeVisible()",
        "expect(el).toHaveFocus()",
        "expect(tree).toMatchSnapshot()",
        "expect(v).not.toBeTruthy()",
        "expect(v).not.toBeDefined()",
        "expect(v).toBeCustomMatcher()",
        "expectTypeOf(x).toEqualTypeOf<number>()",
    ] {
        assert!(carries_value(body), "{body} carries a value");
    }
}

#[test]
fn the_absence_idiom_on_a_query_subject_is_value_less() {
    // `queryBy*` returns null for a missing element, so `toBeNull` on it passes whenever the
    // element is absent, whatever else rendered: on `screen`, bare, through `within(...)`, on
    // a render result, and awaited.
    for body in [
        "expect(screen.queryByText('x')).toBeNull()",
        "expect(screen.queryAllByRole('row')).toBeFalsy()",
        "expect(queryByTestId('x')).toBeNull()",
        "expect(queryByLabelText('x')).toBeUndefined()",
        "expect(within(row).queryByText('x')).toBeNull()",
        "expect(view.queryByText('x')).toBeNull()",
        "expect(await screen.queryByText('x')).toBeNull()",
        "expect((screen.queryByText('x'))).toBeNull()",
        "expect(screen.queryByText('x')).toBeTruthy()",
        "expect(screen.queryByText('x')).not.toBeNull()",
    ] {
        assert!(!carries_value(body), "{body} is value-less");
    }
}

#[test]
fn the_absence_idiom_needs_a_real_query_call_as_the_subject() {
    // A query that is not the subject, a query-like name that is not one, and a throwing
    // query all leave `toBeNull` carrying its value.
    for body in [
        "expect(screen.getByText('x')).toBeNull()",
        "expect(screen.findByText('x')).toBeNull()",
        "expect(screen.queryByText).toBeNull()",
        "expect(queryByText).toBeNull()",
        "expect(query('x')).toBeNull()",
        "expect(screen.queryby('x')).toBeNull()",
        "expect(screen.queryByText('x')?.textContent).toBeNull()",
        "expect(f(screen.queryByText('x'))).toBeNull()",
    ] {
        assert!(carries_value(body), "{body} carries a value");
    }
}

#[test]
fn a_bare_assert_and_assert_ok_are_value_less_and_the_rest_carry_a_value() {
    let rows = assertions_of(
        "import { assert } from 'vitest'\nit('t', () => { assert(ok); assert.ok(ok); assert.equal(a, b); assert.isTrue(ok); assert.isNull(x) })\n",
    );
    let read: Vec<(Option<&str>, bool)> = rows
        .iter()
        .map(|row| (row.matcher.as_deref(), row.carries_value))
        .collect();
    assert_eq!(
        read,
        [
            (None, false),
            (Some("ok"), false),
            (Some("equal"), true),
            (Some("isTrue"), true),
            (Some("isNull"), true),
        ]
    );
}

#[test]
fn a_chain_counts_once_at_its_matcher() {
    let assertions = assertions_of(
        "import { it } from 'vitest'\nit('t', () => { expect(a).toEqual([1, 2]) })\n",
    );
    assert_eq!(
        assertions.len(),
        1,
        "the inner expect() call is not a second row"
    );
}

#[test]
fn a_test_with_no_assertions_is_representable() {
    let assertions =
        assertions_of("import { it } from 'vitest'\nit('t', () => { doSomething() })\n");
    assert!(assertions.is_empty(), "zero assertions, not an error");
}

#[test]
fn the_expect_count_is_the_sum_across_tests() {
    let rows = recognise(
        "import { it } from 'vitest'\n\
         it('a', () => { expect(1).toBe(1); expect(2).toBe(2) })\n\
         it('b', () => { expect(3).toBe(3) })\n",
    );
    let total: usize = rows.iter().map(|row| row.assertions.len()).sum();
    assert_eq!(total, 3, "two plus one");
}

// ------------------------------------------------------- the query subject and the argument

/// The one assertion of a test body.
fn assertion(body: &str) -> Assertion {
    let mut assertions = assertions_of(&format!(
        "import {{ it, expect, assert }} from 'vitest'\nit('t', async () => {{ {body} }})\n"
    ));
    assert_eq!(assertions.len(), 1, "one assertion in {body}");
    assertions.remove(0)
}

#[test]
fn a_query_subject_is_recorded_on_the_row() {
    // The fact the value-less table reads is on the row too, so a rule that turns on the
    // absence idiom (`absence-anchor`) does not re-derive it from the subject text.
    for body in [
        "expect(screen.queryByText('x')).toBeNull()",
        "expect(screen.queryAllByRole('row')).toHaveLength(0)",
        "expect(within(row).queryByText('x')).not.toBeTruthy()",
        "expect(await screen.queryByText('x')).toBeFalsy()",
    ] {
        assert!(
            assertion(body).subject_is_query,
            "{body} has a query subject"
        );
    }
    for body in [
        "expect(screen.getByText('x')).toBeInTheDocument()",
        "expect(screen.queryByText).toBeNull()",
        "expect(f(screen.queryByText('x'))).toBeNull()",
        "assert.isNull(screen.queryByText('x'))",
    ] {
        assert!(
            !assertion(body).subject_is_query,
            "{body} has no query subject"
        );
    }
}

#[test]
fn the_matcher_argument_is_recorded_verbatim() {
    // The first matcher argument as written; `None` when the matcher takes none. For an
    // `assert.*` call the expected value is the second argument, the first being the subject.
    let expected = [
        ("expect(x).toBe(42)", Some("42")),
        ("expect(spy).toHaveBeenCalledTimes(0)", Some("0")),
        ("expect(rows).toHaveLength(0)", Some("0")),
        (
            "expect(spy).toHaveBeenCalledWith('a', { b: 1 })",
            Some("'a'"),
        ),
        ("expect(x).toBeNull()", None),
        ("expect(x).not.toBe(y + 1)", Some("y + 1")),
        ("assert.equal(a, b)", Some("b")),
        ("assert.ok(a)", None),
        ("assert(a, 'why')", None),
    ];
    for (body, argument) in expected {
        assert_eq!(
            assertion(body).argument.as_deref(),
            argument,
            "the argument of {body}"
        );
    }
}
