//! `weak-assertion`: tests whose every assertion carries no value, and the project-wide
//! ratio of such assertions against a ceiling.
//!
//! An assertion "carries no value" when it checks no produced value, only a region the
//! value must fall in: `toBeTruthy`, `toBeDefined`, `not.toBeNull`, `not.toBeUndefined`,
//! `not.toBeFalsy`, a bare `assert(x)`, and `toBeNull` / `toBeUndefined` / `toBeFalsy` on
//! a `queryBy*` subject (the absence idiom). The table is [`ValueLess`] in the model,
//! which records the answer as `Assertion.carries_value == false`; argument count is not
//! the test, so `toThrow()`, `toHaveBeenCalled()` and `toBeInTheDocument()` carry a value.
//! A test is weak when it has at least one assertion and *every* one of them carries no
//! value: the whole test could pass against a wrong-but-plausible subject. The explain
//! names each matcher seen with what it passes against, so the wording is literally true
//! for every matcher it quotes. The ratio finding is one project-wide row: the
//! count of value-less assertions over all assertions, reported only when it exceeds the
//! configured `maxRatio`, so a project can ratchet the ratio down instead of jumping to
//! zero. Both halves read `test(…)` / `it(…)` rows only: a hook's or a benchmark's
//! assertions are neither a weak test nor part of the ratio.

use spirolo_model::{Assertion, Model, TestCase, ValueLess};

use crate::finding::{number_collisions, sort, Evidence, Finding, Location, Outcome, TestPath};
use crate::{tests, Config, Family, Rule, Severity};

/// The default ceiling for the project-wide value-less ratio: six percent, the figure the
/// architecture memo's sample run gates on. Overridable per project with `maxRatio`.
const DEFAULT_MAX_RATIO: f64 = 0.06;

/// The `weak-assertion` rule. See the module docs for the semantics.
pub struct WeakAssertion;

impl Rule for WeakAssertion {
    fn name(&self) -> &'static str {
        "weak-assertion"
    }

    fn family(&self) -> Family {
        Family::Audit
    }

    fn subject(&self) -> crate::Subject {
        crate::Subject::Tests
    }

    fn summary(&self) -> &'static str {
        "A test's assertions check no produced value, only that one exists, so a wrong value still passes."
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn run(&self, model: &Model, config: &Config) -> Vec<Finding> {
        let severity = config.severity(self.name(), self.default_severity());
        let mut findings: Vec<Finding> = tests(model)
            .filter(|case| is_weak(case))
            .map(|case| self.weak_test_finding(case, severity))
            .collect();

        // The ratio is project-wide, one row with no file; it sorts with the rest by the
        // crate's key (an empty file first), so the rule's output keeps the contract every
        // consumer relies on rather than carrying one row appended out of order.
        findings.extend(self.ratio_finding(model, config, severity));

        sort(&mut findings);
        number_collisions(&mut findings);
        findings
    }
}

/// A test is weak when it asserts at least once and every assertion carries no value.
fn is_weak(case: &TestCase) -> bool {
    !case.assertions.is_empty() && case.assertions.iter().all(|a| !a.carries_value)
}

impl WeakAssertion {
    /// The finding for one weak test: located at the test call, naming the value-less
    /// matchers in the evidence and how to repair rather than delete.
    fn weak_test_finding(&self, case: &TestCase, severity: Severity) -> Finding {
        let matchers = value_less_matchers(case);
        let anchor = anchor_of(case);
        let title = case.title.clone().unwrap_or_else(|| "<dynamic>".to_owned());
        Finding {
            rule: self.name(),
            severity,
            outcome: Outcome::Finding,
            location: Location::at(case.file.clone(), case.span),
            related: Vec::new(),
            message: format!(
                "test {title:?} asserts only value-less matchers ({})",
                matchers.join(", ")
            ),
            evidence: vec![
                Evidence::new("matchers", matchers.join(", ")),
                Evidence::new("assertions", case.assertions.len().to_string()),
            ],
            explain: format!(
                "The test {title:?} checks no produced value: {}. Assert the value the code \
                 should produce (`toBe`, `toEqual`, `toHaveTextContent`), or anchor a render \
                 with a throwing `getBy*` query, rather than only that something exists or is \
                 absent. Do not delete the assertions.",
                passes_against(case)
            ),
            anchor,
            test: Some(TestPath::of(case)),
            ordinal: 0,
            proposal: None,
        }
    }

    /// The project-wide ratio finding, or `None` when the ratio is at or below the ceiling
    /// (the row reads `ok` then). Located at the project by an empty file, with the count,
    /// the total and the ceiling in the evidence.
    fn ratio_finding(&self, model: &Model, config: &Config, severity: Severity) -> Option<Finding> {
        let total: usize = tests(model).map(|case| case.assertions.len()).sum();
        if total == 0 {
            return None;
        }
        let value_less: usize = tests(model)
            .flat_map(|case| &case.assertions)
            .filter(|a| !a.carries_value)
            .count();
        #[allow(clippy::cast_precision_loss)]
        let ratio = value_less as f64 / total as f64;
        let ceiling = config.max_ratio(self.name(), DEFAULT_MAX_RATIO);
        if ratio <= ceiling {
            return None;
        }
        Some(Finding {
            rule: self.name(),
            severity,
            outcome: Outcome::Finding,
            location: Location {
                file: String::new(),
                line: 0,
                column: 0,
            },
            related: Vec::new(),
            message: format!(
                "{value_less} of {total} assertions carry no value ({:.1}%, ceiling {:.1}%)",
                ratio * 100.0,
                ceiling * 100.0
            ),
            evidence: vec![
                Evidence::new("ceiling", format!("{ceiling:.4}")),
                Evidence::new("ratio", format!("{ratio:.4}")),
                Evidence::new("total", total.to_string()),
                Evidence::new("value_less", value_less.to_string()),
            ],
            explain: format!(
                "{value_less} of the project's {total} assertions carry no expected value \
                 ({:.1}%), above the {:.1}% ceiling. Lower the ceiling in `.spirolorc.jsonc` \
                 only after replacing value-less matchers with ones that assert the \
                 produced value; ratchet it down, do not delete assertions to hit it.",
                ratio * 100.0,
                ceiling * 100.0
            ),
            anchor: "ratio".to_owned(),
            test: None,
            ordinal: 0,
            proposal: None,
        })
    }
}

/// A value-less matcher as the report spells it: a leading `not.` when the assertion is
/// negated, so it reads `not.toBeNull` rather than a bare `toBeNull`; `assert` for a bare
/// `assert(x)` call.
fn spelled(assertion: &Assertion) -> String {
    let matcher = assertion.matcher.as_deref().unwrap_or("assert");
    if assertion.negated {
        format!("not.{matcher}")
    } else {
        matcher.to_owned()
    }
}

/// The distinct value-less matcher names in a test, sorted and spelled by [`spelled`].
fn value_less_matchers(case: &TestCase) -> Vec<String> {
    let mut names: Vec<String> = case
        .assertions
        .iter()
        .filter(|a| !a.carries_value)
        .map(spelled)
        .collect();
    names.sort();
    names.dedup();
    names
}

/// One clause per distinct value-less matcher in the test, joined with `; `, each saying
/// what that matcher passes against, from the [`ValueLess`] table. A value-less row the
/// model marked but the table cannot place (another recogniser's matcher, a hand-built
/// model) says only what is known: it carries no expected value.
///
/// The table is keyed on the subject too: a non-negated `toBeNull` / `toBeUndefined` /
/// `toBeFalsy` is value-less only on a `queryBy*` subject, which the row records as
/// `subject_is_query`, and the clause says so.
fn passes_against(case: &TestCase) -> String {
    let mut clauses: Vec<(String, String)> = case
        .assertions
        .iter()
        .filter(|a| !a.carries_value)
        .map(|a| {
            let name = spelled(a);
            let classified =
                ValueLess::classify(a.matcher.as_deref(), a.negated, a.subject_is_query);
            let clause = match classified {
                Some(reason @ ValueLess::AbsentElement) => format!(
                    "`{name}` on a `queryBy*` subject {}",
                    reason.passes_against()
                ),
                Some(reason) => format!("`{name}` {}", reason.passes_against()),
                None => format!("`{name}` carries no expected value"),
            };
            (name, clause)
        })
        .collect();
    clauses.sort();
    clauses.dedup();
    clauses
        .into_iter()
        .map(|(_, clause)| clause)
        .collect::<Vec<_>>()
        .join("; ")
}

/// The semantic anchor for a test: its suite path joined with its title, so the identity
/// is stable across runs and unrelated edits. A dynamic title falls back to the span line
/// so two title-less tests in a file do not collide before the ordinal is assigned.
fn anchor_of(case: &TestCase) -> String {
    let mut parts = case.suite_path.clone();
    parts.push(
        case.title
            .clone()
            .unwrap_or_else(|| format!("<dynamic>:{}", case.span.line)),
    );
    parts.join(" › ")
}
