//! `gate-truth`'s claims diff on hand-built models: a document line that claims a gate the
//! config does not back is an `info` finding; a line that claims nothing, or one the config
//! backs, is silent.
//!
//! Seam 3 of the PRD's testing decisions for the claims half of `gate-truth` (slice #41). The
//! rule reads two things: the `Gate` table (the runner and mutation configs the model parsed)
//! and the `claim_documents` the binary read from the `claims` paths. Both are built by hand
//! here and the findings read out — never how they were computed. The grammar is narrow: a line
//! asserts a coverage or mutation gate is *enforced* (the words enforced/blocks/fails/gates, or
//! a threshold/floor/score near a number); a line that says report-only or that the thresholds
//! were lifted asserts no gate and never fires.
//!
//! Each branch:
//!
//! - "coverage thresholds are enforced in CI" with no thresholds in config → an info finding at
//!   that line, naming the config field it disagrees with; the same line with thresholds present
//!   *and* a CI step that runs the suite → silent; with thresholds present but no CI step → a
//!   finding (the runner enforces locally but the pipeline does not).
//! - a line that says "report-only" beside a Stryker `break: null` → silent (no gate is
//!   claimed).
//! - a comment that the thresholds are lifted → silent.
//! - a mutation line claiming the score is enforced while `break` is 0 → an info finding; with
//!   `break: 55` → silent.
//! - the finding is `info` and carries the document line and the config field as its two
//!   locations; the config half's findings are untouched by the claims documents.

use spirolo_model::{Gate, GateTool, Model, Threshold};
use spirolo_rules::{ClaimDocument, Config, Finding, GateTruth, Outcome, Rule, Severity};

/// A blank gate at `source` with the given tool: every field empty, nothing configured.
fn gate(source: &str, tool: GateTool) -> Gate {
    Gate::blank(source, tool)
}

/// A Vitest gate with one coverage threshold, so the config backs a coverage claim.
fn vitest_with_thresholds() -> Gate {
    let mut g = gate("vitest.config.ts", GateTool::Vitest);
    g.thresholds = vec![Threshold {
        metric: "lines".to_owned(),
        value: 80.0,
    }];
    g
}

/// A Stryker gate whose `break` is set to `value`, so the config backs a mutation claim.
fn stryker_break(value: f64) -> Gate {
    let mut g = gate("stryker.conf.mjs", GateTool::Stryker);
    g.mutation_break = Some(value);
    g
}

/// A CI gate whose run steps run the test suite, so a CI-scoped claim can be backed.
fn ci_running_tests() -> Gate {
    let mut g = gate(".github/workflows/ci.yml", GateTool::Ci);
    g.steps = vec!["pnpm vitest run --coverage".to_owned()];
    g
}

/// A claims document at `path` with the given lines.
fn document(path: &str, lines: &[&str]) -> ClaimDocument {
    ClaimDocument {
        path: path.to_owned(),
        lines: lines.iter().map(|l| (*l).to_owned()).collect(),
    }
}

/// The findings `gate-truth` reports over `gates` with `documents` as the claims documents.
fn findings_over(gates: Vec<Gate>, documents: Vec<ClaimDocument>) -> Vec<Finding> {
    let model = Model {
        gates,
        ..Model::default()
    };
    let config = Config {
        claim_documents: documents,
        ..Config::default()
    };
    GateTruth.run(&model, &config)
}

/// The `kind` evidence of a finding, the stable slug the report and the adjudicator read.
fn kind(finding: &Finding) -> &str {
    finding
        .evidence
        .iter()
        .find(|e| e.key == "kind")
        .map_or("", |e| e.value.as_str())
}

/// Only the claims-diff findings — the ones whose kind is `claim-unbacked`.
fn claim_findings(findings: &[Finding]) -> Vec<&Finding> {
    findings
        .iter()
        .filter(|f| kind(f) == "claim-unbacked")
        .collect()
}

// ---------------------------------------------------------------- coverage claim

#[test]
fn a_coverage_enforced_claim_with_no_thresholds_is_a_finding_at_the_line() {
    let doc = document(
        "docs/PLAYBOOK.md",
        &[
            "# Testing",
            "coverage thresholds are enforced in CI",
            "and the suite is fast",
        ],
    );
    let findings = findings_over(vec![gate("vitest.config.ts", GateTool::Vitest)], vec![doc]);
    let claims = claim_findings(&findings);
    assert_eq!(claims.len(), 1, "{findings:#?}");
    let finding = claims[0];
    assert_eq!(finding.location.file, "docs/PLAYBOOK.md");
    assert_eq!(finding.location.line, 2, "the claim's own line");
    assert_eq!(finding.severity, Severity::Info);
    assert_eq!(finding.outcome, Outcome::Finding);
    assert_eq!(finding.rule, "gate-truth");
}

#[test]
fn a_coverage_enforced_claim_names_the_config_field_as_the_second_location() {
    let doc = document("docs/PLAYBOOK.md", &["Coverage thresholds enforced"]);
    let findings = findings_over(vec![gate("vitest.config.ts", GateTool::Vitest)], vec![doc]);
    let finding = claim_findings(&findings)[0];
    assert_eq!(
        finding.related.len(),
        1,
        "the config side of the disagreement"
    );
    assert_eq!(finding.related[0].location.file, "vitest.config.ts");
}

#[test]
fn a_coverage_in_ci_claim_with_thresholds_and_ci_running_the_tests_is_silent() {
    // "enforced in CI" is backed only when the runner declares thresholds AND a CI step runs the
    // suite: both hold here, so the document tells the truth.
    let doc = document(
        "docs/PLAYBOOK.md",
        &["coverage thresholds are enforced in CI"],
    );
    let findings = findings_over(
        vec![vitest_with_thresholds(), ci_running_tests()],
        vec![doc],
    );
    assert!(claim_findings(&findings).is_empty(), "{findings:#?}");
}

#[test]
fn a_coverage_in_ci_claim_with_thresholds_but_no_ci_step_fires() {
    // The runner declares thresholds, but no CI step runs coverage, so
    // "enforced in CI" is a belief CI does not hold. A CI gate that lints only is present.
    let mut ci = gate(".github/workflows/ci.yml", GateTool::Ci);
    ci.steps = vec!["pnpm lint".to_owned()];
    let doc = document(
        "docs/PLAYBOOK.md",
        &["coverage thresholds are enforced in CI"],
    );
    let findings = findings_over(vec![vitest_with_thresholds(), ci], vec![doc]);
    assert_eq!(claim_findings(&findings).len(), 1, "{findings:#?}");
}

#[test]
fn a_plain_coverage_claim_with_thresholds_is_silent_without_ci() {
    // Not CI-scoped: the config value alone backs it, no CI step required.
    let doc = document("docs/PLAYBOOK.md", &["We hold a coverage floor of 80%."]);
    let findings = findings_over(vec![vitest_with_thresholds()], vec![doc]);
    assert!(claim_findings(&findings).is_empty(), "{findings:#?}");
}

// ---------------------------------------------------------------- non-claims

#[test]
fn a_report_only_line_beside_break_null_is_silent() {
    // `report-only` asserts no gate: the narrow grammar must not read it as an enforcement
    // claim, so a Stryker `break: null` (unset) sitting beside it is no disagreement.
    let doc = document(
        "docs/PLAYBOOK.md",
        &["Mutation testing is report-only for now; break is null."],
    );
    let findings = findings_over(vec![gate("stryker.conf.mjs", GateTool::Stryker)], vec![doc]);
    assert!(claim_findings(&findings).is_empty(), "{findings:#?}");
}

#[test]
fn a_lifted_thresholds_comment_is_silent() {
    // A strategy document records *why coverage thresholds are lifted*; that is a
    // record of removal, not a claim of enforcement, so the grammar leaves it alone.
    let doc = document(
        "docs/STRATEGY.md",
        &["Coverage thresholds are lifted for the mobile app while it stabilises."],
    );
    let findings = findings_over(vec![gate("vitest.config.ts", GateTool::Vitest)], vec![doc]);
    assert!(claim_findings(&findings).is_empty(), "{findings:#?}");
}

#[test]
fn prose_with_no_gate_vocabulary_is_silent() {
    let doc = document(
        "docs/PLAYBOOK.md",
        &[
            "We write a test for every bug we fix.",
            "Reviews are mandatory.",
        ],
    );
    let findings = findings_over(vec![gate("vitest.config.ts", GateTool::Vitest)], vec![doc]);
    assert!(claim_findings(&findings).is_empty(), "{findings:#?}");
}

#[test]
fn no_claims_documents_means_no_claims_findings() {
    let findings = findings_over(vec![gate("vitest.config.ts", GateTool::Vitest)], Vec::new());
    assert!(claim_findings(&findings).is_empty(), "{findings:#?}");
}

// ---------------------------------------------------------------- mutation claim

#[test]
fn a_mutation_enforced_claim_with_break_zero_is_a_finding() {
    let doc = document(
        "docs/PLAYBOOK.md",
        &["A mutation score below 0.7 fails the build."],
    );
    let findings = findings_over(vec![stryker_break(0.0)], vec![doc]);
    let claims = claim_findings(&findings);
    assert_eq!(claims.len(), 1, "{findings:#?}");
    assert_eq!(claims[0].location.line, 1);
    assert_eq!(claims[0].related[0].location.file, "stryker.conf.mjs");
}

#[test]
fn a_mutation_enforced_claim_with_break_above_zero_is_silent() {
    let doc = document(
        "docs/PLAYBOOK.md",
        &["A mutation score below 55 fails the build."],
    );
    let findings = findings_over(vec![stryker_break(55.0)], vec![doc]);
    assert!(claim_findings(&findings).is_empty(), "{findings:#?}");
}

#[test]
fn a_number_near_a_threshold_word_is_a_coverage_claim() {
    // "coverage floor of 80" carries a number and a floor: an enforcement claim even without
    // the word "enforced".
    let doc = document("docs/PLAYBOOK.md", &["We hold a coverage floor of 80%."]);
    let findings = findings_over(vec![gate("vitest.config.ts", GateTool::Vitest)], vec![doc]);
    assert_eq!(claim_findings(&findings).len(), 1, "{findings:#?}");
}

// ---------------------------------------------------------------- the grammar's lines
//
// Story 15's grammar: "a line that claims a gate, a threshold or a mutation score near a
// number or the word enforced". Each production has a positive and a negative here, read
// through the rule so the test says what the tool reports, not how `classify` works.

#[test]
fn a_mutation_score_near_a_number_is_a_mutation_claim_without_an_enforcement_word() {
    // "score" beside "70%" is the threshold production, on the mutation metric: a claim
    // backed only by a Stryker `break` above zero.
    let doc = document("docs/PLAYBOOK.md", &["The mutation score minimum is 70%."]);
    let findings = findings_over(vec![stryker_break(0.0)], vec![doc.clone()]);
    let claims = claim_findings(&findings);
    assert_eq!(claims.len(), 1, "{findings:#?}");
    assert!(
        claims[0].message.contains("a mutation score gate"),
        "the mutation field, not coverage: {}",
        claims[0].message
    );
    assert_eq!(claims[0].related[0].location.file, "stryker.conf.mjs");

    let backed = findings_over(vec![stryker_break(55.0)], vec![doc]);
    assert!(claim_findings(&backed).is_empty(), "{backed:#?}");
}

#[test]
fn a_threshold_word_with_no_number_and_no_enforcement_word_is_prose() {
    // "thresholds" alone names the metric but asserts nothing: no figure, no verb.
    let doc = document(
        "docs/PLAYBOOK.md",
        &[
            "Coverage thresholds live in vitest.config.ts.",
            "The mutation score is reported on the dashboard.",
        ],
    );
    let findings = findings_over(
        vec![
            gate("vitest.config.ts", GateTool::Vitest),
            stryker_break(0.0),
        ],
        vec![doc],
    );
    assert!(claim_findings(&findings).is_empty(), "{findings:#?}");
}

#[test]
fn an_enforcement_word_with_no_metric_is_not_a_claim() {
    // "enforced", "blocks", "gates" on their own are about something else: lint, types, a
    // review rule. Without coverage or mutation on the line there is no gate to back.
    let doc = document(
        "docs/PLAYBOOK.md",
        &[
            "Lint is enforced in CI.",
            "The type check blocks the merge.",
            "A review from CODEOWNERS is required and gates every PR.",
        ],
    );
    let findings = findings_over(
        vec![
            gate("vitest.config.ts", GateTool::Vitest),
            stryker_break(0.0),
        ],
        vec![doc],
    );
    assert!(claim_findings(&findings).is_empty(), "{findings:#?}");
}

#[test]
fn ci_inside_another_word_does_not_make_a_claim_ci_scoped() {
    // With thresholds present and no CI gate at all, a CI-scoped claim fires and a plain one
    // is silent — so "precision" (which spells `ci` inside a word) must read as plain.
    let plain = document(
        "docs/PLAYBOOK.md",
        &["Coverage thresholds are enforced with precision."],
    );
    let findings = findings_over(vec![vitest_with_thresholds()], vec![plain]);
    assert!(
        claim_findings(&findings).is_empty(),
        "`precision` is not the word `ci`: {findings:#?}"
    );

    let scoped = document(
        "docs/PLAYBOOK.md",
        &["Coverage thresholds are enforced in CI."],
    );
    let findings = findings_over(vec![vitest_with_thresholds()], vec![scoped]);
    assert_eq!(
        claim_findings(&findings).len(),
        1,
        "the word `ci` asks for a CI step: {findings:#?}"
    );
}

#[test]
fn a_bare_thresholds_line_and_a_line_naming_both_metrics_are_coverage_claims() {
    // "thresholds" with no metric word reads as coverage (the playbook phrasing), and a line
    // that names both metrics is coverage too, so the config field a finding points at is
    // deterministic: both point at the runner gate, never at Stryker.
    let doc = document(
        "docs/PLAYBOOK.md",
        &[
            "Thresholds of 80% block the build.",
            "Coverage and mutation gates are enforced.",
        ],
    );
    let findings = findings_over(
        vec![
            gate("vitest.config.ts", GateTool::Vitest),
            stryker_break(55.0),
        ],
        vec![doc],
    );
    let claims = claim_findings(&findings);
    assert_eq!(claims.len(), 2, "{findings:#?}");
    for claim in &claims {
        assert!(
            claim.message.contains("coverage thresholds"),
            "{}",
            claim.message
        );
        assert_eq!(claim.related[0].location.file, "vitest.config.ts");
    }
}

// ---------------------------------------------------------------- determinism

#[test]
fn a_claims_identity_is_its_words_not_its_line() {
    // An edit above the claim moves its line but must not re-key the finding; the same
    // claim re-indented or re-wrapped keeps its id too; two identical lines take ordinals.
    let ids = |lines: &[&str]| -> Vec<String> {
        let findings = findings_over(
            vec![gate("vitest.config.ts", GateTool::Vitest)],
            vec![document("docs/PLAYBOOK.md", lines)],
        );
        claim_findings(&findings)
            .into_iter()
            .map(Finding::id)
            .collect()
    };
    let before = ids(&["# Testing", "- Coverage thresholds are enforced in CI"]);
    let after = ids(&[
        "# Testing",
        "",
        "A new paragraph above the claim.",
        "  *   coverage   thresholds are enforced in CI  ",
    ]);
    assert_eq!(
        before,
        vec!["gate-truth docs/PLAYBOOK.md claim-unbacked:coverage thresholds are enforced in ci#0"]
    );
    assert_eq!(before, after, "line moved and re-indented, id unchanged");

    let twice = ids(&[
        "coverage thresholds are enforced",
        "filler",
        "coverage thresholds are enforced",
    ]);
    assert_eq!(twice.len(), 2);
    assert_ne!(
        twice[0], twice[1],
        "identical lines take ordinals: {twice:?}"
    );
    assert!(twice[1].ends_with("#1"), "{twice:?}");
}

#[test]
fn claims_findings_are_deterministic_and_sorted() {
    let build = || {
        let doc = document(
            "docs/PLAYBOOK.md",
            &[
                "Coverage thresholds enforced",
                "filler",
                "Mutation score below 0.7 blocks the merge",
            ],
        );
        findings_over(
            vec![
                gate("vitest.config.ts", GateTool::Vitest),
                stryker_break(0.0),
            ],
            vec![doc],
        )
    };
    assert_eq!(build(), build(), "two runs are byte-identical");
    let produced = build();
    let claims = claim_findings(&produced);
    assert_eq!(claims.len(), 2, "{produced:#?}");
    assert!(
        claims[0].location.line < claims[1].location.line,
        "claims sorted by line: {claims:#?}"
    );
}
