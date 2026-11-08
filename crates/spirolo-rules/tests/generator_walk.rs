//! `generator-walk` on hand-built models: the rule reads the `SagaWalk` table the saga
//! recogniser fills and reports one `info` finding per step-walk site, plus a listed count
//! row per file.
//!
//! The rule is a pure function of the model, so these tests build the `SagaWalk` rows by
//! hand and read the findings out — never how they were computed. The recogniser's own
//! branches (which shapes are steps) are covered on snippets in `spirolo-frameworks`; the
//! binary fixture (Seam 2) proves the rule end to end through `spirolo check`.

use spirolo_model::{Model, SagaStepKind, SagaWalk, Span, TestCase, TestKind};
use spirolo_rules::{Finding, GeneratorWalk, Outcome, Rule, Severity};

/// A span at a line, enough to sort and locate by.
const fn span(line: u32) -> Span {
    Span {
        start: line,
        end: line + 1,
        line,
        column: 1,
    }
}

/// A `gen.next().value` step-walk site.
fn gen_next(file: &str, effect: &str, line: u32) -> SagaWalk {
    SagaWalk {
        file: file.to_owned(),
        span: span(line),
        kind: SagaStepKind::GenNext,
        effect: Some(effect.to_owned()),
    }
}

/// A `testSaga` chain step.
fn test_saga(file: &str, effect: &str, line: u32) -> SagaWalk {
    SagaWalk {
        file: file.to_owned(),
        span: span(line),
        kind: SagaStepKind::TestSaga,
        effect: Some(effect.to_owned()),
    }
}

/// A model carrying the given saga walks.
fn model(walks: Vec<SagaWalk>) -> Model {
    Model {
        saga_walks: walks,
        ..Model::default()
    }
}

/// A test case in `file` spanning lines `first..=last` (the hand-built spans put a line's
/// site at byte `line`), under `suites`.
fn test_case(file: &str, suites: &[&str], title: &str, first: u32, last: u32) -> TestCase {
    TestCase {
        file: file.to_owned(),
        span: Span {
            start: first,
            end: last + 1,
            line: first,
            column: 1,
        },
        framework: Some("vitest".to_owned()),
        title: Some(title.to_owned()),
        kind: TestKind::Test,
        suite_path: suites.iter().map(ToString::to_string).collect(),
        assertions: Vec::new(),
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

/// The per-site findings the rule produces (the count rows apart).
fn findings(model: &Model) -> Vec<Finding> {
    GeneratorWalk
        .run(model, &spirolo_rules::Config::default())
        .into_iter()
        .filter(|f| f.outcome == Outcome::Finding)
        .collect()
}

/// The listed count rows the rule produces.
fn counts(model: &Model) -> Vec<Finding> {
    GeneratorWalk
        .run(model, &spirolo_rules::Config::default())
        .into_iter()
        .filter(|f| f.outcome == Outcome::Listed)
        .collect()
}

// ------------------------------------------------------------------------ the acceptance tests

#[test]
fn one_finding_per_site_at_info() {
    let model = model(vec![
        gen_next("src/sagas.test.ts", "call", 3),
        gen_next("src/sagas.test.ts", "put", 4),
        test_saga("src/other.test.ts", "isDone", 5),
    ]);
    let findings = findings(&model);
    assert_eq!(findings.len(), 3, "one finding per step-walk site");
    assert!(
        findings.iter().all(|f| f.severity == Severity::Info),
        "generator-walk is info"
    );
    assert!(
        findings.iter().all(|f| f.outcome == Outcome::Finding),
        "a step-walk is always a decided finding, never could-not-decide"
    );
    assert_eq!(findings[0].rule, "generator-walk");
}

#[test]
fn the_finding_names_the_count_and_the_idiom() {
    let model = model(vec![gen_next("src/sagas.test.ts", "call", 3)]);
    let finding = &findings(&model)[0];
    assert_eq!(finding.location.file, "src/sagas.test.ts");
    // The evidence carries the effect and the idiom, so a consumer reads them structurally.
    assert!(
        finding
            .evidence
            .iter()
            .any(|e| e.key == "effect" && e.value == "call"),
        "{:?}",
        finding.evidence
    );
    assert!(
        finding.evidence.iter().any(|e| e.key == "idiom"),
        "the idiom is named in the evidence"
    );
}

#[test]
fn the_explain_points_to_expect_saga_run() {
    let model = model(vec![gen_next("src/sagas.test.ts", "call", 3)]);
    let finding = &findings(&model)[0];
    // The explain steers towards running the saga rather than deleting the assertion.
    assert!(
        finding.explain.contains("expectSaga"),
        "{}",
        finding.explain
    );
    assert!(finding.explain.contains(".run()"), "{}", finding.explain);
    assert!(
        !finding.explain.to_lowercase().contains("delete the"),
        "explain never says to delete"
    );
}

#[test]
fn no_saga_walks_is_no_finding() {
    assert!(findings(&model(Vec::new())).is_empty(), "nothing to report");
    assert!(counts(&model(Vec::new())).is_empty(), "no count row either");
}

#[test]
fn one_listed_count_row_per_file_carries_the_sites() {
    // Two files: the count row is per file, at the file's first site, listed (never a
    // finding for precision), with the number and the site list in the evidence.
    let model = model(vec![
        gen_next("src/b.test.ts", "call", 9),
        gen_next("src/a.test.ts", "put", 5),
        test_saga("src/a.test.ts", "take", 2),
    ]);
    let counts = counts(&model);
    assert_eq!(counts.len(), 2, "one per file");
    let a = &counts[0];
    assert_eq!(a.location.file, "src/a.test.ts");
    assert_eq!(a.location.line, 2, "at the first site");
    assert_eq!(a.severity, Severity::Info);
    assert_eq!(a.id(), "generator-walk src/a.test.ts count#0");
    assert!(
        a.message.starts_with("2 saga step-walk sites"),
        "{}",
        a.message
    );
    assert!(a
        .evidence
        .iter()
        .any(|e| e.key == "kind" && e.value == "walk-count"));
    assert!(a
        .evidence
        .iter()
        .any(|e| e.key == "sites" && e.value == "2"));
    assert!(
        a.evidence
            .iter()
            .any(|e| e.key == "site_list" && e.value == "2:testSaga:take, 5:gen.next:put"),
        "{:?}",
        a.evidence
    );
    assert_eq!(counts[1].location.file, "src/b.test.ts");
    assert!(counts[1]
        .evidence
        .iter()
        .any(|e| e.key == "sites" && e.value == "1"));
    // The per-site rows are untouched.
    assert_eq!(findings(&model).len(), 3);
}

#[test]
fn findings_are_sorted_by_file_then_span() {
    let model = model(vec![
        gen_next("src/b.test.ts", "call", 9),
        gen_next("src/a.test.ts", "put", 5),
        gen_next("src/a.test.ts", "take", 2),
    ]);
    let findings = findings(&model);
    assert_eq!(findings.len(), 3);
    assert_eq!(findings[0].location.file, "src/a.test.ts");
    assert_eq!(findings[0].location.line, 2, "sorted by span within a file");
    assert_eq!(findings[1].location.line, 5);
    assert_eq!(findings[2].location.file, "src/b.test.ts");
}

#[test]
fn a_test_saga_step_is_reported_with_its_fluent_effect() {
    let model = model(vec![test_saga("src/sagas.test.ts", "put", 3)]);
    let finding = &findings(&model)[0];
    assert!(
        finding
            .evidence
            .iter()
            .any(|e| e.key == "idiom" && e.value == "testSaga"),
        "{:?}",
        finding.evidence
    );
    assert!(finding
        .evidence
        .iter()
        .any(|e| e.key == "effect" && e.value == "put"));
}

#[test]
fn the_identity_is_anchored_on_the_enclosing_test_and_the_effect() {
    // Two tests each step `call` then `put`. A site's id names its test, its idiom and its
    // effect; the ordinal counts the yields of that effect within that test only.
    let mut model = model(vec![
        gen_next("src/sagas.test.ts", "call", 3),
        gen_next("src/sagas.test.ts", "put", 4),
        gen_next("src/sagas.test.ts", "put", 5),
        gen_next("src/sagas.test.ts", "call", 12),
        gen_next("src/sagas.test.ts", "put", 13),
    ]);
    model.test_cases = vec![
        test_case("src/sagas.test.ts", &["fetchUser"], "loads the user", 2, 6),
        test_case(
            "src/sagas.test.ts",
            &["fetchUser"],
            "handles the error",
            11,
            14,
        ),
    ];
    let ids: Vec<String> = findings(&model).iter().map(Finding::id).collect();
    assert_eq!(
        ids,
        [
            "generator-walk src/sagas.test.ts fetchUser › loads the user › gen.next:call#0",
            "generator-walk src/sagas.test.ts fetchUser › loads the user › gen.next:put#0",
            "generator-walk src/sagas.test.ts fetchUser › loads the user › gen.next:put#1",
            "generator-walk src/sagas.test.ts fetchUser › handles the error › gen.next:call#0",
            "generator-walk src/sagas.test.ts fetchUser › handles the error › gen.next:put#0",
        ]
    );
}

#[test]
fn inserting_a_site_in_another_test_does_not_re_key_the_ones_below() {
    // The second test's ids are the same whether or not the first test grows a step, which
    // the file-level `idiom:effect#n` ordinal could not promise.
    let second = |model: &Model| -> Vec<String> {
        findings(model)
            .iter()
            .filter(|f| f.location.line >= 11)
            .map(Finding::id)
            .collect()
    };
    let cases = vec![
        test_case("src/sagas.test.ts", &[], "first", 2, 6),
        test_case("src/sagas.test.ts", &[], "second", 11, 14),
    ];
    let mut before = model(vec![
        gen_next("src/sagas.test.ts", "put", 3),
        gen_next("src/sagas.test.ts", "put", 12),
    ]);
    before.test_cases.clone_from(&cases);
    let mut after = model(vec![
        gen_next("src/sagas.test.ts", "put", 3),
        gen_next("src/sagas.test.ts", "put", 4),
        gen_next("src/sagas.test.ts", "put", 12),
    ]);
    after.test_cases = cases;
    assert_eq!(second(&before), second(&after));
    assert_eq!(
        second(&after),
        ["generator-walk src/sagas.test.ts second › gen.next:put#0"]
    );
}

#[test]
fn a_site_outside_any_test_anchors_on_the_idiom_and_effect_alone() {
    // A helper at module scope: no enclosing case, so the anchor is `idiom:effect` numbered
    // within the file, as before.
    let mut model = model(vec![gen_next("src/sagas.test.ts", "call", 3)]);
    model.test_cases = vec![test_case("src/sagas.test.ts", &[], "elsewhere", 10, 20)];
    assert_eq!(
        findings(&model)[0].id(),
        "generator-walk src/sagas.test.ts gen.next:call#0"
    );
}

#[test]
fn identities_are_distinct_across_sites_in_one_file() {
    // Two sites in one file must not collide on identity: the harness joins adjudications by
    // identity, so a collision would drop one verdict. The anchor plus ordinal keeps them
    // apart.
    let model = model(vec![
        gen_next("src/sagas.test.ts", "call", 3),
        gen_next("src/sagas.test.ts", "call", 4),
    ]);
    let findings = findings(&model);
    assert_eq!(findings.len(), 2);
    assert_ne!(findings[0].id(), findings[1].id(), "distinct identities");
}
