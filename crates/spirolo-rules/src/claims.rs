//! The narrow grammar `gate-truth` reads a claims document with: a line that asserts a
//! coverage or mutation gate is *enforced*.
//!
//! A maintainer's playbook says "coverage thresholds are enforced in CI"; the config sets no
//! thresholds. That disagreement is the claims diff. But a document is prose, and prose lies in
//! both directions: "mutation testing is report-only" claims no gate, "thresholds were lifted"
//! records a removal. The grammar is deliberately narrow so the rule fires only on a line that
//! genuinely asserts enforcement — the PRD's kill criterion is precision, and a claims rule that
//! reads intent from prose is the easiest place to lose it.
//!
//! A line is a claim when it names a metric (coverage/threshold/mutation/score) *as enforced*:
//! either it carries an enforcement word (enforced, blocks, fails, gates, required) or it puts a
//! number beside a threshold/floor/score/minimum word. A line that carries a *waiver* word
//! (report-only, lifted, removed, disabled, aspirational, not enforced) asserts the opposite and
//! is never a claim, so it wins over an enforcement reading on the same line. Everything is
//! matched case-insensitively on the line's words; no line is parsed beyond its vocabulary.

/// Which gate a claim line asserts is enforced, so the rule knows which config fact backs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimKind {
    /// A coverage-threshold claim: backed by a runner gate that declares coverage thresholds.
    Coverage,
    /// A mutation-score claim: backed by a Stryker gate whose `break` is above zero.
    Mutation,
}

/// One gating claim a document line makes: which gate, and whether it says the gate runs *in
/// CI*.
///
/// A line that claims enforcement "in CI" (or "in the pipeline", "on every PR") asks for more
/// than a config value — it asks that a CI step actually run the gate. The rule reads
/// [`Claim::ci_scoped`] to decide which config facts must back the claim: a config value alone,
/// or a config value plus a CI step that runs the suite. Reading the CI scope from the line, not
/// the surrounding prose, keeps the grammar a pure per-line query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Claim {
    /// The gate the line claims is enforced.
    pub kind: ClaimKind,
    /// Whether the line says the gate is enforced in CI, so a CI step must run it too.
    pub ci_scoped: bool,
}

/// The words that turn a metric mention into an assertion that it is enforced.
const ENFORCEMENT_WORDS: [&str; 7] = [
    "enforced", "enforce", "blocks", "block", "fails", "gates", "required",
];

/// The words that make a line a waiver — an assertion that the gate is *not* in force. Any of
/// these anywhere on the line disqualifies it, whatever else it says, so a line that records a
/// removal or a report-only stance is never read as a claim of enforcement.
const WAIVER_WORDS: [&str; 12] = [
    "report-only",
    "report only",
    "reporting only",
    "lifted",
    "removed",
    "disabled",
    "aspirational",
    "not enforced",
    // A line that itself acknowledges the CI gate is not (yet) in force is being honest, not
    // making a false claim — a playbook can say thresholds are enforced by Vitest while "the
    // CircleCI pipeline does not yet gate on coverage". Firing on it would penalise the document
    // for telling the truth, so these self-acknowledged gaps veto the line.
    "does not gate",
    "does not yet gate",
    "not yet gate",
    "does not enforce",
];

/// The words that make a number-bearing line a threshold assertion: a floor/minimum/threshold
/// score put beside a figure is an enforcement claim even without an enforcement verb.
const THRESHOLD_WORDS: [&str; 5] = ["threshold", "thresholds", "floor", "minimum", "score"];

/// The metric words that scope a coverage claim.
const COVERAGE_WORDS: [&str; 2] = ["coverage", "line coverage"];

/// The metric words that scope a mutation claim.
const MUTATION_WORDS: [&str; 2] = ["mutation", "mutants"];

/// The phrases that make a claim CI-scoped — it asks that a CI step run the gate, not merely
/// that a config value exist. `ci` is matched as its own word so `circleci` (a provider name)
/// and prose `ci` both count while an unrelated word does not.
const CI_WORDS: [&str; 5] = ["ci", "pipeline", "pull request", "merge", "pre-merge"];

/// Classify one document line, `None` when it asserts no enforced gate.
///
/// The line is lower-cased once and matched on plain substrings of its words. A waiver word
/// vetoes the line first; then a metric (coverage or mutation) must be present *and* asserted —
/// by an enforcement word or by a number beside a threshold word — for the line to be a claim.
/// A line that names both metrics is read as coverage (the common playbook phrasing); a line
/// with a metric but no assertion is prose and returns `None`. A line that names CI, a pipeline,
/// a pull request or a merge is CI-scoped, so the rule requires a CI step to back it.
#[must_use]
pub fn classify(line: &str) -> Option<Claim> {
    let lower = line.to_ascii_lowercase();
    if WAIVER_WORDS.iter().any(|w| lower.contains(w)) {
        return None;
    }
    if !asserts_enforcement(&lower) {
        return None;
    }
    // A metric must be named; coverage wins over mutation when both appear, so the config field
    // a finding points at is deterministic.
    let kind = if COVERAGE_WORDS.iter().any(|w| lower.contains(w)) || lower.contains("thresholds") {
        ClaimKind::Coverage
    } else if MUTATION_WORDS.iter().any(|w| lower.contains(w)) {
        ClaimKind::Mutation
    } else {
        return None;
    };
    let ci_scoped = CI_WORDS.iter().any(|w| {
        if *w == "ci" {
            contains_word(&lower, "ci") || lower.contains("circleci")
        } else {
            lower.contains(w)
        }
    });
    Some(Claim { kind, ci_scoped })
}

/// Whether a lower-cased line asserts that a gate is enforced: an enforcement word, or a number
/// standing beside a threshold/floor/score/minimum word.
fn asserts_enforcement(lower: &str) -> bool {
    if ENFORCEMENT_WORDS.iter().any(|w| contains_word(lower, w)) {
        return true;
    }
    THRESHOLD_WORDS.iter().any(|w| lower.contains(w)) && has_number(lower)
}

/// Whether the text contains a run of digits — a threshold figure like `0.7`, `55` or `80%`.
fn has_number(text: &str) -> bool {
    text.chars().any(|c| c.is_ascii_digit())
}

/// Whether `needle` appears in `text` as its own word, so `enforce` matches `enforced` only
/// through the shorter stem in the list and never inside an unrelated word.
fn contains_word(text: &str, needle: &str) -> bool {
    text.split(|c: char| !c.is_ascii_alphanumeric() && c != '-')
        .any(|word| word == needle)
}

#[cfg(test)]
mod tests {
    use super::{classify, Claim, ClaimKind};

    #[test]
    fn coverage_enforced_in_ci_is_a_ci_scoped_coverage_claim() {
        assert_eq!(
            classify("coverage thresholds are enforced in CI"),
            Some(Claim {
                kind: ClaimKind::Coverage,
                ci_scoped: true,
            })
        );
    }

    #[test]
    fn a_coverage_floor_with_a_number_is_a_coverage_claim_not_ci_scoped() {
        assert_eq!(
            classify("We hold a coverage floor of 80%."),
            Some(Claim {
                kind: ClaimKind::Coverage,
                ci_scoped: false,
            })
        );
    }

    #[test]
    fn a_mutation_score_that_fails_the_build_is_a_mutation_claim() {
        assert_eq!(
            classify("A mutation score below 0.7 fails the build."),
            Some(Claim {
                kind: ClaimKind::Mutation,
                ci_scoped: false,
            })
        );
    }

    #[test]
    fn circleci_and_pipeline_are_read_as_ci_scope() {
        let claim =
            classify("Thresholds are enforced by Vitest; the CircleCI pipeline gates on coverage.")
                .unwrap();
        assert_eq!(claim.kind, ClaimKind::Coverage);
        assert!(claim.ci_scoped, "CircleCI and pipeline both mark CI scope");
    }

    #[test]
    fn report_only_vetoes_the_line() {
        assert_eq!(
            classify("Mutation testing is report-only for now; break is null."),
            None
        );
    }

    #[test]
    fn lifted_thresholds_are_not_a_claim() {
        assert_eq!(
            classify("Coverage thresholds are lifted while the app stabilises."),
            None
        );
    }

    #[test]
    fn a_line_acknowledging_the_ci_gate_is_absent_is_not_a_claim() {
        // The doc is honest that CI does not gate, so it is not a false claim.
        assert_eq!(
            classify(
                "Thresholds are enforced by Vitest when it runs with --coverage; the \
                 CircleCI pipeline does not yet gate on coverage."
            ),
            None
        );
    }

    #[test]
    fn a_metric_without_an_assertion_is_prose() {
        assert_eq!(
            classify("Coverage is measured on every pull request."),
            None
        );
    }

    #[test]
    fn a_number_without_a_threshold_word_is_not_a_coverage_claim() {
        // A bare figure with no threshold/floor/score word and no enforcement verb is prose.
        assert_eq!(classify("Coverage was 80% last week."), None);
    }

    #[test]
    fn an_enforcement_word_inside_another_word_does_not_count() {
        // `reinforced` contains `enforce` as a substring but is not the word.
        assert_eq!(
            classify("Our coverage story was reinforced by the review."),
            None
        );
    }
}
