//! The value-less matcher taxonomy (PRD 0002 review, decision 5): which `(matcher, negated,
//! subject-is-query)` triples carry no expected value, and why each one passes. Pure
//! table tests; the recogniser tests in `spirolo-frameworks` prove the same table reads
//! through from source.

use spirolo_model::ValueLess;

/// The decision as `carries_value` would record it.
fn carries(matcher: Option<&str>, negated: bool, query: bool) -> bool {
    ValueLess::classify(matcher, negated, query).is_none()
}

#[test]
fn the_five_unconditional_value_less_matchers() {
    assert_eq!(
        ValueLess::classify(Some("toBeTruthy"), false, false),
        Some(ValueLess::AnyTruthy)
    );
    assert_eq!(
        ValueLess::classify(Some("toBeDefined"), false, false),
        Some(ValueLess::AnyDefined)
    );
    assert_eq!(
        ValueLess::classify(Some("toBeNull"), true, false),
        Some(ValueLess::AnyNonNull)
    );
    assert_eq!(
        ValueLess::classify(Some("toBeUndefined"), true, false),
        Some(ValueLess::AnyDefined)
    );
    assert_eq!(
        ValueLess::classify(Some("toBeFalsy"), true, false),
        Some(ValueLess::AnyTruthy)
    );
}

#[test]
fn the_unconditional_set_is_value_less_on_a_query_subject_too() {
    assert!(!carries(Some("toBeTruthy"), false, true));
    assert!(!carries(Some("toBeDefined"), false, true));
    assert!(!carries(Some("toBeNull"), true, true));
}

#[test]
fn the_absence_idiom_is_value_less_only_on_a_query_subject() {
    for matcher in ["toBeNull", "toBeUndefined", "toBeFalsy"] {
        assert_eq!(
            ValueLess::classify(Some(matcher), false, true),
            Some(ValueLess::AbsentElement),
            "{matcher} on a queryBy* subject passes whenever the element is absent"
        );
        assert!(
            carries(Some(matcher), false, false),
            "{matcher} on a plain subject asserts the produced value"
        );
    }
}

#[test]
fn the_negated_unconditional_matchers_carry_a_value() {
    // `not.toBeTruthy` means falsy, `not.toBeDefined` means undefined: each names the one
    // value it accepts.
    assert!(carries(Some("toBeTruthy"), true, false));
    assert!(carries(Some("toBeDefined"), true, false));
    assert!(carries(Some("toBeTruthy"), true, true));
}

#[test]
fn argument_less_matchers_outside_the_set_carry_a_value() {
    for (matcher, negated) in [
        ("toThrow", false),
        ("toThrow", true),
        ("toHaveBeenCalled", false),
        ("toHaveBeenCalled", true),
        ("toHaveBeenCalledOnce", false),
        ("toBeInTheDocument", false),
        ("toBeInTheDocument", true),
        ("toBeVisible", false),
        ("toBeDisabled", false),
        ("toHaveFocus", false),
        ("toMatchSnapshot", false),
        ("toBeGreaterThan", false),
        ("toEqualTypeOf", false),
        ("toBeCustomThing", false),
    ] {
        assert!(
            carries(Some(matcher), negated, false),
            "{}{matcher} carries a value",
            if negated { "not." } else { "" }
        );
        assert!(
            carries(Some(matcher), negated, true),
            "{}{matcher} carries a value on a query subject too",
            if negated { "not." } else { "" }
        );
    }
}

#[test]
fn a_bare_assert_and_assert_ok_are_value_less_and_the_rest_of_assert_is_not() {
    assert_eq!(
        ValueLess::classify(None, false, false),
        Some(ValueLess::AnyTruthy),
        "assert(x)"
    );
    assert_eq!(
        ValueLess::classify(Some("ok"), false, false),
        Some(ValueLess::AnyTruthy),
        "assert.ok(x)"
    );
    for matcher in [
        "equal",
        "deepEqual",
        "isTrue",
        "isNull",
        "exists",
        "isDefined",
    ] {
        assert!(carries(Some(matcher), false, false), "assert.{matcher}");
    }
}

#[test]
fn every_reason_says_what_the_assertion_passes_against() {
    for reason in [
        ValueLess::AnyTruthy,
        ValueLess::AnyDefined,
        ValueLess::AnyNonNull,
        ValueLess::AbsentElement,
    ] {
        let text = reason.passes_against();
        assert!(
            text.starts_with("passes "),
            "a fragment that completes `<matcher> ...`: {text}"
        );
        assert!(!text.contains('\u{2014}'), "no em dash: {text}");
    }
    assert!(ValueLess::AnyTruthy.passes_against().contains("truthy"));
    assert!(ValueLess::AnyDefined.passes_against().contains("undefined"));
    assert!(ValueLess::AnyNonNull.passes_against().contains("null"));
    assert!(ValueLess::AbsentElement.passes_against().contains("absent"));
}
