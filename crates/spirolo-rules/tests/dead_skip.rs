//! `dead-skip` on hand-built models: one case per branch of its semantics — every skip
//! kind, the issue-reference forms, the env-scoped condition, and `only`.
//!
//! The rule is a pure function of the model, so these tests build `TestCase` rows by hand
//! and read the findings out — never how they were computed. The binary fixtures (Seam 2)
//! prove the same rule end to end through `spirolo check`.

use spirolo_model::{Model, Skip, SkipKind, Span, TestCase, TestKind};
use spirolo_rules::{Config, DeadSkip, Family, Finding, Outcome, Rule, RuleConfig, Severity};

/// A definition-level skip modifier of `kind`, with an optional condition. Its span sits on
/// the test definition's own line (line 1, as `test_case` builds), the way a chain modifier
/// `it.skip('t', …)` does — so the rule reads it as a definition skip, not a body annotation.
fn skip(kind: SkipKind, condition: Option<&str>) -> Skip {
    Skip {
        kind,
        condition: condition.map(str::to_owned),
        reason: None,
        comments: Vec::new(),
        span: Span {
            start: 5,
            end: 15,
            line: 1,
            column: 3,
        },
    }
}

/// A runtime in-body skip modifier: its span sits on a later line than the definition, the
/// way a `test.skip(cond)` statement inside the callback does.
fn body_skip(kind: SkipKind, condition: Option<&str>) -> Skip {
    Skip {
        kind,
        condition: condition.map(str::to_owned),
        reason: None,
        comments: Vec::new(),
        span: Span {
            start: 40,
            end: 50,
            line: 5,
            column: 7,
        },
    }
}

/// A test case in `file` with a title and an optional skip.
fn test_case(file: &str, title: Option<&str>, skip: Option<Skip>) -> TestCase {
    TestCase {
        file: file.to_owned(),
        span: Span {
            start: 1,
            end: 2,
            line: 1,
            column: 1,
        },
        framework: Some("vitest".to_owned()),
        title: title.map(str::to_owned),
        kind: TestKind::Test,
        suite_path: Vec::new(),
        assertions: Vec::new(),
        throwing_queries: Vec::new(),
        mocks: Vec::new(),
        skip,
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

/// Every flagged finding a config-less run of the rule produces: the rows with
/// `Outcome::Finding`, leaving the per-skip listing rows to [`listed`].
fn findings(model: &Model) -> Vec<Finding> {
    DeadSkip
        .run(model, &Config::default())
        .into_iter()
        .filter(|f| f.outcome == Outcome::Finding)
        .collect()
}

/// The listing rows a config-less run produces, one per skip site.
fn listed(model: &Model) -> Vec<Finding> {
    DeadSkip
        .run(model, &Config::default())
        .into_iter()
        .filter(|f| f.outcome == Outcome::Listed)
        .collect()
}

/// One evidence value by key, or `""`.
fn evidence<'a>(finding: &'a Finding, key: &str) -> &'a str {
    finding
        .evidence
        .iter()
        .find(|e| e.key == key)
        .map_or("", |e| e.value.as_str())
}

#[test]
fn an_unconditional_skip_with_no_issue_is_a_finding() {
    let m = model(vec![test_case(
        "a.test.ts",
        Some("does the thing"),
        Some(skip(SkipKind::Skip, None)),
    )]);

    let found = findings(&m);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].rule, "dead-skip");
    assert_eq!(found[0].severity, Severity::Warn);
    assert_eq!(found[0].outcome, Outcome::Finding);
    // Located at the skip modifier (line 1, the definition line the modifier sits on).
    assert_eq!(found[0].location.line, 1);
    assert!(
        found[0].message.contains("no linked issue"),
        "{}",
        found[0].message
    );
}

#[test]
fn todo_and_fixme_are_unconditional_too() {
    for kind in [SkipKind::Todo, SkipKind::Fixme] {
        let m = model(vec![test_case(
            "a.test.ts",
            Some("later"),
            Some(skip(kind, None)),
        )]);
        assert_eq!(findings(&m).len(), 1, "{kind:?} with no issue is a finding");
    }
}

#[test]
fn a_hash_number_in_the_title_silences_the_finding() {
    let m = model(vec![test_case(
        "a.test.ts",
        Some("skipped pending #42"),
        Some(skip(SkipKind::Skip, None)),
    )]);
    assert!(findings(&m).is_empty(), "#42 links the skip to an issue");
}

#[test]
fn a_ticket_key_in_the_title_silences_the_finding() {
    let m = model(vec![test_case(
        "a.test.ts",
        Some("blocked by PROJ-1234"),
        Some(skip(SkipKind::Skip, None)),
    )]);
    assert!(findings(&m).is_empty(), "PROJ-1234 is a linked ticket");
}

#[test]
fn an_issue_url_in_the_title_silences_the_finding() {
    let m = model(vec![test_case(
        "a.test.ts",
        Some("see https://github.com/o/r/issues/17"),
        Some(skip(SkipKind::Skip, None)),
    )]);
    assert!(findings(&m).is_empty(), "the issue URL links the skip");
}

#[test]
fn a_bare_word_that_is_not_a_ticket_does_not_count_as_a_reference() {
    // `TODO` is upper-case but has no `-number`; a lone `#` with no digit is not a ref.
    let m = model(vec![test_case(
        "a.test.ts",
        Some("TODO: fix this later, tracked in issue # somewhere"),
        Some(skip(SkipKind::Skip, None)),
    )]);
    assert_eq!(findings(&m).len(), 1, "no #123, no KEY-123, no URL");
}

#[test]
fn a_skip_true_condition_is_unconditional() {
    // A Playwright `test.skip(true, "reason")` is captured as a Skip with condition "true".
    let m = model(vec![test_case(
        "a.test.ts",
        Some("broken"),
        Some(skip(SkipKind::SkipIf, Some("true"))),
    )]);
    let found = findings(&m);
    assert_eq!(found.len(), 1, "skip(true) never runs → unconditional");
    assert!(found[0].message.contains("no linked issue"));
}

#[test]
fn a_ci_scoped_skip_if_names_the_variable_at_info() {
    let m = model(vec![test_case(
        "a.test.ts",
        Some("integration"),
        Some(skip(SkipKind::SkipIf, Some("!!process.env.CI"))),
    )]);
    let found = findings(&m);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].severity, Severity::Info, "CI-scoped is context");
    assert!(
        found[0].message.contains("CI-scoped") && found[0].message.contains("CI"),
        "names the variable: {}",
        found[0].message
    );
    assert!(
        found[0].explain.contains("CI"),
        "explain names the condition: {}",
        found[0].explain
    );
}

#[test]
fn a_run_if_reading_an_env_var_is_ci_scoped() {
    let m = model(vec![test_case(
        "a.test.ts",
        Some("only in ci"),
        Some(skip(SkipKind::RunIf, Some("process.env.RUN_SLOW === '1'"))),
    )]);
    let found = findings(&m);
    assert_eq!(found.len(), 1);
    assert!(
        found[0].message.contains("RUN_SLOW"),
        "{}",
        found[0].message
    );
}

#[test]
fn a_bracket_env_read_names_the_variable() {
    let m = model(vec![test_case(
        "a.test.ts",
        Some("t"),
        Some(skip(SkipKind::SkipIf, Some("process.env[\"NO_NET\"]"))),
    )]);
    let found = findings(&m);
    assert_eq!(found.len(), 1);
    assert!(found[0].message.contains("NO_NET"), "{}", found[0].message);
}

#[test]
fn an_import_meta_env_read_is_ci_scoped() {
    let m = model(vec![test_case(
        "a.test.ts",
        Some("t"),
        Some(skip(SkipKind::SkipIf, Some("import.meta.env.DEV"))),
    )]);
    let found = findings(&m);
    assert_eq!(found.len(), 1);
    assert!(found[0].message.contains("DEV"), "{}", found[0].message);
}

#[test]
fn a_data_driven_skip_if_that_reads_no_env_is_left_alone() {
    // A `skipIf` on a computed condition that is neither literal-true nor an env read is not
    // something the rule can call dead or CI-scoped: it stays silent rather than guess.
    let m = model(vec![test_case(
        "a.test.ts",
        Some("t"),
        Some(skip(SkipKind::SkipIf, Some("features.betaEnabled"))),
    )]);
    assert!(findings(&m).is_empty(), "no env read, no literal true");
}

#[test]
fn a_runtime_in_body_skip_is_left_alone() {
    // A `test.skip()` or `test.skip(true, 'reason')` written as a statement inside the test
    // body (its modifier span on a later line than the definition) usually sits under an
    // `if` the rule cannot see — a conditional early-return, not a dead definition skip. The
    // rule does not flag it, even though the modifier reads as unconditional.
    for condition in [None, Some("true")] {
        let m = model(vec![test_case(
            "a.test.ts",
            Some("guarded at runtime"),
            Some(body_skip(SkipKind::Skip, condition)),
        )]);
        assert!(
            findings(&m).is_empty(),
            "a body-level test.skip({}) is not a dead definition skip",
            condition.unwrap_or("")
        );
    }
}

#[test]
fn a_reference_in_the_reason_silences_the_finding() {
    let mut skip = skip(SkipKind::Fixme, Some("true"));
    skip.reason = Some("tracked in #42".to_owned());
    let m = model(vec![test_case("a.test.ts", Some("broken"), Some(skip))]);
    assert!(findings(&m).is_empty(), "the reason links the skip");
}

#[test]
fn a_pull_or_merge_request_url_in_the_reason_silences_the_finding() {
    for url in [
        "https://github.com/o/r/pull/7",
        "https://bitbucket.org/o/r/pulls/7",
        "https://gitlab.com/o/r/-/merge_requests/7",
    ] {
        let mut skip = skip(SkipKind::Fixme, Some("true"));
        skip.reason = Some(format!("waits on {url}"));
        let m = model(vec![test_case("a.test.ts", Some("broken"), Some(skip))]);
        assert!(findings(&m).is_empty(), "{url} links the skip");
    }
}

#[test]
fn the_condition_text_is_not_a_reason() {
    // Only the title and the reason carry a reference; a condition that happens to spell
    // `#1` or a ticket-looking name does not make the skip tracked.
    let m = model(vec![test_case(
        "a.test.ts",
        Some("t"),
        Some(skip(SkipKind::SkipIf, Some("true /* ISSUE-12 */"))),
    )]);
    assert!(
        findings(&m).is_empty(),
        "`true /* … */` is not the bare literal, so the skip is conditional, not flagged"
    );
    let m = model(vec![test_case(
        "a.test.ts",
        Some("t"),
        Some(skip(SkipKind::SkipIf, Some("true"))),
    )]);
    assert_eq!(
        findings(&m).len(),
        1,
        "skip(true) with no reason and no title ref"
    );
}

#[test]
fn a_playwright_conditional_skip_is_not_flagged() {
    // `test.skip(browserName === 'webkit', 'why')` is a conditional skip: neither dead nor
    // CI-scoped, so the rule does not flag it — whatever its title says.
    for condition in [
        "browserName === 'webkit'",
        "({ browserName }) => browserName === 'webkit'",
        "isMobile",
    ] {
        let m = model(vec![test_case(
            "a.spec.ts",
            Some("no ticket here"),
            Some(body_skip(SkipKind::Skip, Some(condition))),
        )]);
        assert!(
            findings(&m).is_empty(),
            "{condition}: a conditional skip is not unconditional"
        );
    }
}

#[test]
fn a_playwright_conditional_skip_is_listed_as_conditional_with_its_condition() {
    // Story 16: the same `test.skip(browserName === 'webkit', 'why')` that is not flagged is
    // still inventoried — one listing row, scope `conditional`, the condition and the reason
    // carried, not the `runtime` scope a bare in-body `test.skip()` takes.
    let mut skip = body_skip(SkipKind::Skip, Some("browserName === 'webkit'"));
    skip.reason = Some("no webkit yet".to_owned());
    let m = model(vec![test_case(
        "a.spec.ts",
        Some("mobile layout"),
        Some(skip),
    )]);
    let all = DeadSkip.run(&m, &Config::default());
    assert_eq!(all.len(), 1, "one listing row, no flagged row: {all:#?}");
    let row = &all[0];
    assert_eq!(row.outcome, Outcome::Listed);
    assert_eq!(evidence(row, "scope"), "conditional");
    assert_eq!(evidence(row, "condition"), "browserName === 'webkit'");
    assert_eq!(evidence(row, "reason"), "no webkit yet");
    assert_eq!(evidence(row, "tracked"), "no");
    assert!(
        row.message
            .contains("conditional on browserName === 'webkit'"),
        "{}",
        row.message
    );
}

#[test]
fn a_playwright_annotation_reading_the_environment_is_ci_scoped() {
    // `test.skip(!process.env.X, why)` and `test.fixme(!process.env.X, why)` in a body are
    // CI-scoped the way a `skipIf(process.env.X)` is: flagged at info, the variable named,
    // and listed with scope `ci`.
    for kind in [SkipKind::Skip, SkipKind::Fixme] {
        let m = model(vec![test_case(
            "a.spec.ts",
            Some("needs the service"),
            Some(body_skip(kind, Some("!process.env.SERVICE_URL"))),
        )]);
        let found = findings(&m);
        assert_eq!(found.len(), 1, "{kind:?}: {found:#?}");
        assert_eq!(found[0].severity, Severity::Info);
        assert!(
            found[0].message.contains("CI-scoped") && found[0].message.contains("SERVICE_URL"),
            "{}",
            found[0].message
        );
        assert_eq!(evidence(&found[0], "variable"), "SERVICE_URL");
        let rows = listed(&m);
        assert_eq!(rows.len(), 1, "{kind:?}: {rows:#?}");
        assert_eq!(evidence(&rows[0], "scope"), "ci");
    }
}

#[test]
fn run_if_false_is_unconditional_and_skip_if_false_is_not_a_skip() {
    let never = model(vec![test_case(
        "a.test.ts",
        Some("t"),
        Some(skip(SkipKind::RunIf, Some("false"))),
    )]);
    let found = findings(&never);
    assert_eq!(found.len(), 1, "runIf(false) never runs: {found:#?}");
    assert!(found[0].message.contains("no linked issue"));

    for (kind, condition) in [(SkipKind::SkipIf, "false"), (SkipKind::RunIf, "true")] {
        let always = model(vec![test_case(
            "a.test.ts",
            Some("t"),
            Some(skip(kind, Some(condition))),
        )]);
        assert!(
            findings(&always).is_empty(),
            "{kind:?}({condition}) always runs, so it is not a skip"
        );
    }
}

#[test]
fn only_is_an_error() {
    let m = model(vec![test_case(
        "a.test.ts",
        Some("just this one"),
        Some(skip(SkipKind::Only, None)),
    )]);
    let found = findings(&m);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].severity, Severity::Error, "only fails the build");
    assert!(found[0].message.contains("only"), "{}", found[0].message);
}

#[test]
fn fails_and_slow_are_not_reported() {
    for kind in [SkipKind::Fails, SkipKind::Slow] {
        let m = model(vec![test_case(
            "a.test.ts",
            Some("t"),
            Some(skip(kind, None)),
        )]);
        assert!(findings(&m).is_empty(), "{kind:?} is not a dead skip");
    }
}

#[test]
fn a_case_with_no_skip_is_not_reported() {
    let m = model(vec![test_case("a.test.ts", Some("plain"), None)]);
    assert!(findings(&m).is_empty());
}

#[test]
fn the_explain_names_every_place_the_rule_reads_for_a_reference() {
    let m = model(vec![test_case(
        "a.test.ts",
        Some("does the thing"),
        Some(skip(SkipKind::Skip, None)),
    )]);
    let explain = &findings(&m)[0].explain;
    for place in [
        "title",
        "reason",
        "trailing comment",
        "comment lines just above",
    ] {
        assert!(
            explain.contains(place),
            "explain names the {place}: {explain}"
        );
    }
}

#[test]
fn the_explain_names_the_kind_and_the_missing_reference() {
    let m = model(vec![test_case(
        "a.test.ts",
        Some("does the thing"),
        Some(skip(SkipKind::Todo, None)),
    )]);
    let found = findings(&m);
    assert!(found[0].explain.contains("todo"), "{}", found[0].explain);
    assert!(
        found[0].explain.contains("#123") || found[0].explain.contains("issue"),
        "explain names the missing reference: {}",
        found[0].explain
    );
}

#[test]
fn findings_are_sorted_by_file() {
    let m = model(vec![
        test_case("z.test.ts", Some("z"), Some(skip(SkipKind::Skip, None))),
        test_case("a.test.ts", Some("a"), Some(skip(SkipKind::Skip, None))),
    ]);
    let files: Vec<String> = findings(&m).into_iter().map(|f| f.location.file).collect();
    assert_eq!(
        files,
        ["a.test.ts", "z.test.ts"],
        "sorted, not source order"
    );
}

#[test]
fn the_configured_severity_overrides_the_unconditional_default() {
    // The default severity governs the unconditional finding; `only` stays error and the
    // CI-scoped stays info regardless (they carry their own severity by design).
    let mut config = Config::default();
    config.rules.insert(
        "dead-skip".to_owned(),
        RuleConfig {
            severity: Some(Severity::Error),
            max_ratio: None,
        },
    );
    let m = model(vec![test_case(
        "a.test.ts",
        Some("t"),
        Some(skip(SkipKind::Skip, None)),
    )]);
    let found: Vec<Finding> = DeadSkip
        .run(&m, &config)
        .into_iter()
        .filter(|f| f.outcome == Outcome::Finding)
        .collect();
    assert_eq!(found[0].severity, Severity::Error);
}

#[test]
fn the_rule_declares_its_identity() {
    assert_eq!(DeadSkip.name(), "dead-skip");
    assert_eq!(DeadSkip.family(), Family::Gate);
    assert_eq!(DeadSkip.default_severity(), Severity::Warn);
}

// ---------------------------------------------------------------- the listing (story 16)

#[test]
fn every_skip_is_listed_with_its_condition_and_scope() {
    let m = model(vec![
        test_case("a.test.ts", Some("dead"), Some(skip(SkipKind::Skip, None))),
        test_case("a.test.ts", Some("later"), Some(skip(SkipKind::Todo, None))),
        test_case(
            "a.test.ts",
            Some("in ci"),
            Some(skip(SkipKind::SkipIf, Some("!!process.env.CI"))),
        ),
        test_case(
            "a.test.ts",
            Some("slow box"),
            Some(skip(SkipKind::RunIf, Some("os.cpus().length > 4"))),
        ),
        test_case(
            "a.spec.ts",
            Some("no brands"),
            Some(body_skip(SkipKind::Skip, Some("true"))),
        ),
        test_case(
            "a.test.ts",
            Some("always"),
            Some(skip(SkipKind::SkipIf, Some("false"))),
        ),
        test_case("a.test.ts", Some("one"), Some(skip(SkipKind::Only, None))),
        test_case(
            "a.test.ts",
            Some("flaky"),
            Some(skip(SkipKind::Fails, None)),
        ),
    ]);
    let rows = listed(&m);
    // `only`, `fails` and `slow` are not skips: six sites listed, not eight.
    assert_eq!(rows.len(), 6, "{rows:#?}");
    assert!(rows.iter().all(|r| r.severity == Severity::Info));
    assert!(rows.iter().all(|r| evidence(r, "kind") == "skip-listed"));

    let by_title = |title: &str| {
        rows.iter()
            .find(|r| r.message.contains(&format!("{title:?}")))
            .unwrap_or_else(|| panic!("{title} listed: {rows:#?}"))
    };
    let dead = by_title("dead");
    assert_eq!(evidence(dead, "modifier"), "skip");
    assert_eq!(evidence(dead, "scope"), "unconditional");
    assert_eq!(evidence(dead, "condition"), "none");
    assert_eq!(evidence(dead, "tracked"), "no");
    assert_eq!(evidence(by_title("later"), "modifier"), "todo");
    let ci = by_title("in ci");
    assert_eq!(evidence(ci, "modifier"), "skipIf");
    assert_eq!(evidence(ci, "scope"), "ci");
    assert_eq!(evidence(ci, "condition"), "!!process.env.CI");
    assert!(ci.message.contains("CI-scoped on CI"), "{}", ci.message);
    let conditional = by_title("slow box");
    assert_eq!(evidence(conditional, "scope"), "conditional");
    assert_eq!(evidence(conditional, "modifier"), "runIf");
    assert_eq!(evidence(by_title("no brands"), "scope"), "runtime");
    assert_eq!(evidence(by_title("always"), "scope"), "open");
}

#[test]
fn a_listing_row_carries_the_reason_and_whether_it_is_tracked() {
    let mut tracked = skip(SkipKind::Fixme, Some("true"));
    tracked.reason = Some("see PROJ-7".to_owned());
    let m = model(vec![test_case("a.spec.ts", Some("broken"), Some(tracked))]);
    let rows = listed(&m);
    assert_eq!(rows.len(), 1);
    assert_eq!(evidence(&rows[0], "reason"), "see PROJ-7");
    assert_eq!(evidence(&rows[0], "tracked"), "yes");
    // Tracked, so no flagged row beside it.
    assert!(findings(&m).is_empty());
}

#[test]
fn a_listing_row_never_fails_the_build_and_keeps_the_flagged_id() {
    let m = model(vec![test_case(
        "a.test.ts",
        Some("dead"),
        Some(skip(SkipKind::Skip, None)),
    )]);
    let all = DeadSkip.run(&m, &Config::default());
    assert_eq!(
        all.len(),
        2,
        "one listing row and one flagged row: {all:#?}"
    );
    let flagged = all.iter().find(|f| f.outcome == Outcome::Finding).unwrap();
    let listing = all.iter().find(|f| f.outcome == Outcome::Listed).unwrap();
    assert_eq!(
        flagged.id(),
        "dead-skip a.test.ts dead#0",
        "the flagged row keeps the id it always had"
    );
    assert_eq!(listing.id(), "dead-skip a.test.ts listed:dead#0");
    assert_eq!(listing.severity, Severity::Info);
    assert!(listing.explain.contains("never fails the build"));
}

#[test]
fn colliding_findings_take_distinct_ordinals() {
    // Two title-less definition-level skips in one file: their skip modifier sits on each
    // definition's own line, so both are definition-level and the ordinal keeps them apart.
    let mut a = test_case("a.test.ts", None, Some(skip(SkipKind::Skip, None)));
    a.span = Span {
        start: 10,
        end: 11,
        line: 10,
        column: 1,
    };
    a.skip.as_mut().unwrap().span.line = 10;
    let mut b = test_case("a.test.ts", None, Some(skip(SkipKind::Skip, None)));
    b.span = Span {
        start: 40,
        end: 41,
        line: 40,
        column: 1,
    };
    b.skip.as_mut().unwrap().span.line = 40;

    let ids: Vec<String> = findings(&model(vec![a, b]))
        .into_iter()
        .map(|f| f.id())
        .collect();
    assert_eq!(ids.len(), 2);
    assert_ne!(ids[0], ids[1], "ordinals keep colliding ids apart: {ids:?}");
}
