//! The value-less matcher taxonomy: which assertions check no produced value, and why
//! each one passes. Decided in the PRD 0002 attended review (decision 5) and shared by the
//! recognisers, which record it as `Assertion.carries_value`, and the `weak-assertion`
//! rule, which explains each matcher it names.
//!
//! An assertion is *value-less* when it would pass against a wrong-but-plausible subject:
//! the matcher names no value the code should produce, only a region the value must fall
//! in (truthy, defined, non-null), or, for the Testing Library absence idiom, the mere
//! absence of an element. Argument count is not the test: `toThrow()`, `toBeVisible()`
//! and `toHaveBeenCalled()` take no argument and still assert one specific fact, and
//! `toBeNull()` on a function result asserts the produced value exactly.

/// Why a value-less assertion passes without checking a produced value.
///
/// `None` from [`ValueLess::classify`] means the assertion carries a value. The decision
/// table, with `not.` marking a negated chain and `query` marking a subject that is a
/// `queryBy*` / `queryAllBy*` call:
///
/// | assertion | subject | reason |
/// |---|---|---|
/// | `toBeTruthy`, `not.toBeFalsy`, `assert(x)`, `assert.ok(x)` | any | [`AnyTruthy`](Self::AnyTruthy) |
/// | `toBeDefined`, `not.toBeUndefined` | any | [`AnyDefined`](Self::AnyDefined) |
/// | `not.toBeNull` | any | [`AnyNonNull`](Self::AnyNonNull) |
/// | `toBeNull`, `toBeUndefined`, `toBeFalsy` | query | [`AbsentElement`](Self::AbsentElement) |
/// | `toBeNull`, `toBeUndefined`, `toBeFalsy` | plain | carries a value |
/// | `not.toBeTruthy`, `not.toBeDefined` | any | carries a value (each names one value) |
/// | `toThrow`, `not.toThrow`, `toHaveBeenCalled*`, `not.toHaveBeenCalled` | any | carries a value |
/// | jest-dom state matchers (`toBeInTheDocument`, `toBeVisible`, `toHaveFocus`, ...) | any | carries a value |
/// | `toMatchSnapshot`, `toBe(x)`, `toBeGreaterThan(0)`, every matcher with an argument | any | carries a value |
/// | `assert.*` other than `ok` | any | carries a value |
/// | a matcher not in this table (a custom `expect.extend`) | any | carries a value |
///
/// Unknown matchers carry a value so a custom matcher is never flagged: the rule is
/// conservative about what it has not read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValueLess {
    /// Passes against any truthy value: `toBeTruthy`, `not.toBeFalsy`, `assert(x)`,
    /// `assert.ok(x)`.
    AnyTruthy,
    /// Passes against any value but `undefined`: `toBeDefined`, `not.toBeUndefined`.
    AnyDefined,
    /// Passes against any value but `null`: `not.toBeNull`.
    AnyNonNull,
    /// Passes whenever the queried element is absent, whatever else rendered:
    /// `toBeNull`, `toBeUndefined` or `toBeFalsy` on a `queryBy*` / `queryAllBy*` subject.
    AbsentElement,
}

impl ValueLess {
    /// Classify one assertion from its matcher (`None` for a bare `assert(x)` call), its
    /// negation, and whether its subject is a `queryBy*` / `queryAllBy*` call. `None`
    /// when the assertion carries a value; see the table on [`ValueLess`].
    pub fn classify(matcher: Option<&str>, negated: bool, subject_is_query: bool) -> Option<Self> {
        match (matcher, negated) {
            (None | Some("ok" | "toBeTruthy"), false) | (Some("toBeFalsy"), true) => {
                Some(Self::AnyTruthy)
            }
            (Some("toBeDefined"), false) | (Some("toBeUndefined"), true) => Some(Self::AnyDefined),
            (Some("toBeNull"), true) => Some(Self::AnyNonNull),
            (Some("toBeNull" | "toBeUndefined" | "toBeFalsy"), false) if subject_is_query => {
                Some(Self::AbsentElement)
            }
            _ => None,
        }
    }

    /// What the assertion passes against, as a fragment completing `<matcher> ...`, so a
    /// report can say something literally true of each matcher it names.
    pub const fn passes_against(self) -> &'static str {
        match self {
            Self::AnyTruthy => "passes against any truthy value",
            Self::AnyDefined => "passes against any value but `undefined`",
            Self::AnyNonNull => "passes against any value but `null`",
            Self::AbsentElement => {
                "passes whenever the queried element is absent, whatever else rendered"
            }
        }
    }
}
