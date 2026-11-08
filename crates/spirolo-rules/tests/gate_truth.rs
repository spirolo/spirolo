//! `gate-truth` on hand-built models: one case per branch of its config-half semantics.
//!
//! The rule is a pure function of the `Gate` table (the runner/mutation configs the model read
//! by AST, and the CI workflows the model read as YAML), so these tests build `Gate` rows by
//! hand and read the findings out — never how they were computed. Each branch:
//!
//! - a runner gate with no coverage thresholds → a `thresholds-absent` finding; with
//!   thresholds → silent.
//! - a Stryker gate whose `break` is `0` → a `mutation-break-zero` finding; `break: 55` →
//!   silent; `break: null` or absent → a `mutation-break-null` finding (story 14: the score
//!   is computed but never gates), unless the gate is opaque.
//! - a Playwright gate whose `forbidOnly` is false/unset while a CI gate exists → a
//!   `forbid-only` finding; `forbidOnly: true` → silent; no CI present → silent.
//! - CI gates present but no run step in any of them runs the tests, coverage or mutation →
//!   one project-wide `ci-runs-nothing` finding at the workflows directory naming every
//!   workflow; a step that runs `vitest run --coverage` in any workflow → silent.
//! - any gate with `retries` set above zero → a `retries` finding at info severity.
//!
//! The claims diff is slice #41 and is out of scope here.

use spirolo_model::{Gate, GateFlag, GateTool, Model, Threshold};
use spirolo_rules::{Config, Family, Finding, GateTruth, Outcome, Rule, Severity};

/// A blank gate at `source` with the given tool: every field empty, nothing configured.
fn gate(source: &str, tool: GateTool) -> Gate {
    Gate::blank(source, tool)
}

/// The findings `gate-truth` reports over a model built from `gates`.
fn findings_over(gates: Vec<Gate>) -> Vec<Finding> {
    let model = Model {
        gates,
        ..Model::default()
    };
    GateTruth.run(&model, &Config::default())
}

/// The `kind` evidence of a finding, the stable slug the report and the adjudicator read.
fn kind(finding: &Finding) -> &str {
    finding
        .evidence
        .iter()
        .find(|e| e.key == "kind")
        .map_or("", |e| e.value.as_str())
}

/// Whether any finding carries the given kind.
fn has_kind(findings: &[Finding], slug: &str) -> bool {
    findings.iter().any(|f| kind(f) == slug)
}

// ---------------------------------------------------------------- identity

#[test]
fn the_rule_declares_its_identity() {
    assert_eq!(GateTruth.name(), "gate-truth");
    assert_eq!(GateTruth.family(), Family::Gate);
    assert_eq!(GateTruth.default_severity(), Severity::Warn);
}

// ---------------------------------------------------------------- thresholds

#[test]
fn a_runner_gate_with_no_coverage_thresholds_is_a_finding() {
    let findings = findings_over(vec![gate("vitest.config.ts", GateTool::Vitest)]);
    let threshold: Vec<&Finding> = findings
        .iter()
        .filter(|f| kind(f) == "thresholds-absent")
        .collect();
    assert_eq!(threshold.len(), 1, "{findings:#?}");
    assert_eq!(threshold[0].location.file, "vitest.config.ts");
    assert_eq!(threshold[0].rule, "gate-truth");
    assert_eq!(threshold[0].outcome, Outcome::Finding);
}

#[test]
fn a_runner_gate_with_coverage_thresholds_is_silent_on_thresholds() {
    let mut g = gate("vitest.config.ts", GateTool::Vitest);
    g.thresholds = vec![Threshold {
        metric: "lines".to_owned(),
        value: 80.0,
    }];
    let findings = findings_over(vec![g]);
    assert!(!has_kind(&findings, "thresholds-absent"), "{findings:#?}");
}

#[test]
fn a_stryker_gate_never_reports_absent_thresholds() {
    // Thresholds-absent is a coverage claim about the runner, not the mutation engine.
    let findings = findings_over(vec![gate("stryker.conf.mjs", GateTool::Stryker)]);
    assert!(!has_kind(&findings, "thresholds-absent"), "{findings:#?}");
}

#[test]
fn inheritors_of_one_preset_with_no_thresholds_lift_to_one_finding_at_the_preset() {
    // Three kits compose one preset that declares no coverage thresholds, and none adds
    // any. Thresholds would live at the preset: one finding there with the count, none per
    // kit.
    let preset = vec!["packages/config/base.mjs".to_owned()];
    let mut gates = Vec::new();
    for pkg in ["a", "b", "c"] {
        let mut g = gate(
            &format!("packages/{pkg}/vitest.config.mts"),
            GateTool::Vitest,
        );
        g.preset_chain = preset.clone();
        gates.push(g);
    }
    let findings = findings_over(gates);
    let absent: Vec<&Finding> = findings
        .iter()
        .filter(|f| kind(f) == "thresholds-absent")
        .collect();
    assert_eq!(absent.len(), 1, "{findings:#?}");
    assert_eq!(absent[0].location.file, "packages/config/base.mjs");
    assert_eq!(absent[0].location.line, 0);
    assert_eq!(absent[0].anchor, "thresholds-absent");
    assert!(
        absent[0]
            .evidence
            .iter()
            .any(|e| e.key == "packages" && e.value == "3"),
        "{:#?}",
        absent[0].evidence
    );
    assert!(
        absent[0].message.contains("3 inheriting configs"),
        "{}",
        absent[0].message
    );
    assert!(
        absent[0].explain.contains("preset"),
        "the repair is at the preset: {}",
        absent[0].explain
    );
}

#[test]
fn an_inheritor_with_its_own_thresholds_is_not_counted_in_the_lift() {
    let preset = vec!["packages/config/base.mjs".to_owned()];
    let mut a = gate("packages/a/vitest.config.mts", GateTool::Vitest);
    a.preset_chain = preset.clone();
    a.thresholds = vec![Threshold {
        metric: "lines".to_owned(),
        value: 80.0,
    }];
    let mut b = gate("packages/b/vitest.config.mts", GateTool::Vitest);
    b.preset_chain = preset;
    let findings = findings_over(vec![a, b]);
    let absent: Vec<&Finding> = findings
        .iter()
        .filter(|f| kind(f) == "thresholds-absent")
        .collect();
    assert_eq!(absent.len(), 1, "{findings:#?}");
    assert_eq!(absent[0].location.file, "packages/config/base.mjs");
    assert!(absent[0]
        .evidence
        .iter()
        .any(|e| e.key == "packages" && e.value == "1"));
}

#[test]
fn a_config_that_composes_no_preset_keeps_its_own_finding_beside_a_lift() {
    let mut inheritor = gate("packages/a/vitest.config.mts", GateTool::Vitest);
    inheritor.preset_chain = vec!["packages/config/base.mjs".to_owned()];
    let standalone = gate("apps/web/vitest.config.ts", GateTool::Vitest);
    let findings = findings_over(vec![inheritor, standalone]);
    let mut files: Vec<&str> = findings
        .iter()
        .filter(|f| kind(f) == "thresholds-absent")
        .map(|f| f.location.file.as_str())
        .collect();
    files.sort_unstable();
    assert_eq!(
        files,
        ["apps/web/vitest.config.ts", "packages/config/base.mjs"]
    );
}

#[test]
fn an_unreadable_runner_gate_is_could_not_decide_not_thresholds_absent() {
    // The reader could not see into the config (a spread it does not evaluate). Whether it
    // declares thresholds is undecided: never a decided "absent", one could-not-decide row
    // that says what could not be read.
    let mut g = gate("vitest.config.ts", GateTool::Vitest);
    g.could_not_read("a spread (`...`) in the config object");
    let findings = findings_over(vec![g]);
    assert_eq!(findings.len(), 1, "{findings:#?}");
    assert_eq!(findings[0].outcome, Outcome::CouldNotDecide);
    assert_eq!(kind(&findings[0]), "config-unread");
    assert_eq!(findings[0].location.file, "vitest.config.ts");
    assert_eq!(findings[0].anchor, "config-unread");
    assert!(
        findings[0]
            .explain
            .contains("a spread (`...`) in the config object"),
        "the explain says what could not be read: {}",
        findings[0].explain
    );
    assert!(findings[0]
        .evidence
        .iter()
        .any(|e| e.key == "unread" && e.value.contains("spread")));
    assert!(!has_kind(&findings, "thresholds-absent"));
}

#[test]
fn an_unreadable_gate_with_thresholds_it_did_read_is_still_silent_on_thresholds() {
    // Opaque for a reason unrelated to thresholds (a computed setup file) but with numeric
    // thresholds read: the thresholds fact is decided (present), only the rest is unread.
    let mut g = gate("vitest.config.ts", GateTool::Vitest);
    g.thresholds = vec![Threshold {
        metric: "lines".to_owned(),
        value: 80.0,
    }];
    g.could_not_read("an entry of `setupFiles` that is not a string literal");
    let findings = findings_over(vec![g]);
    assert!(
        findings.is_empty(),
        "thresholds are present whatever else was unread: {findings:#?}"
    );
}

// ---------------------------------------------------------------- mutation break

#[test]
fn a_stryker_break_of_zero_is_a_finding() {
    let mut g = gate("stryker.conf.mjs", GateTool::Stryker);
    g.mutation_break = Some(0.0);
    let findings = findings_over(vec![g]);
    let brk: Vec<&Finding> = findings
        .iter()
        .filter(|f| kind(f) == "mutation-break-zero")
        .collect();
    assert_eq!(brk.len(), 1, "{findings:#?}");
    assert_eq!(brk[0].location.file, "stryker.conf.mjs");
}

#[test]
fn a_stryker_break_above_zero_is_silent() {
    let mut g = gate("stryker.conf.mjs", GateTool::Stryker);
    g.mutation_break = Some(55.0);
    let findings = findings_over(vec![g]);
    assert!(!has_kind(&findings, "mutation-break-zero"), "{findings:#?}");
}

#[test]
fn a_stryker_break_of_null_is_a_finding() {
    // `break: null` is Stryker's default and means the score is computed but never gates;
    // story 14 names `0` or `null` alike.
    let g = gate("stryker.conf.mjs", GateTool::Stryker); // mutation_break: None
    let findings = findings_over(vec![g]);
    let null: Vec<&Finding> = findings
        .iter()
        .filter(|f| kind(f) == "mutation-break-null")
        .collect();
    assert_eq!(null.len(), 1, "{findings:#?}");
    assert_eq!(null[0].location.file, "stryker.conf.mjs");
    assert_eq!(null[0].outcome, Outcome::Finding);
    assert!(
        null[0]
            .explain
            .contains("computes and reports a mutation score")
            && null[0]
                .explain
                .contains("no score, however low, ever fails the build"),
        "{}",
        null[0].explain
    );
    assert!(!has_kind(&findings, "mutation-break-zero"), "{findings:#?}");
}

#[test]
fn an_opaque_stryker_gate_with_no_break_read_is_silent() {
    // A preset the reader could not follow may set `break`; the rule does not claim it null.
    let mut g = gate("stryker.conf.mjs", GateTool::Stryker);
    g.could_not_read("a preset the reader does not evaluate");
    let findings = findings_over(vec![g]);
    assert!(!has_kind(&findings, "mutation-break-null"), "{findings:#?}");
}

// ---------------------------------------------------------------- forbidOnly

#[test]
fn a_playwright_gate_without_forbid_only_and_with_ci_is_a_finding() {
    let findings = findings_over(vec![
        gate("playwright.config.ts", GateTool::Playwright), // forbid_only: None
        {
            let mut ci = gate(".github/workflows/ci.yml", GateTool::Ci);
            ci.steps = vec!["pnpm playwright test".to_owned()];
            ci
        },
    ]);
    let forbid: Vec<&Finding> = findings
        .iter()
        .filter(|f| kind(f) == "forbid-only")
        .collect();
    assert_eq!(forbid.len(), 1, "{findings:#?}");
    assert_eq!(forbid[0].location.file, "playwright.config.ts");
}

#[test]
fn a_playwright_gate_with_forbid_only_true_is_silent() {
    let mut g = gate("playwright.config.ts", GateTool::Playwright);
    g.forbid_only = Some(GateFlag::True);
    let findings = findings_over(vec![g, gate(".github/workflows/ci.yml", GateTool::Ci)]);
    assert!(!has_kind(&findings, "forbid-only"), "{findings:#?}");
}

#[test]
fn a_ci_conditional_forbid_only_is_set_where_it_matters() {
    // `forbidOnly: !!process.env.CI` is Playwright's own scaffold: true in the CI build the
    // rule judges, so it is not a finding.
    let mut g = gate("playwright.config.ts", GateTool::Playwright);
    g.forbid_only = Some(GateFlag::CiConditional);
    let findings = findings_over(vec![g, gate(".github/workflows/ci.yml", GateTool::Ci)]);
    assert!(!has_kind(&findings, "forbid-only"), "{findings:#?}");
}

#[test]
fn a_literal_false_forbid_only_is_a_finding() {
    let mut g = gate("playwright.config.ts", GateTool::Playwright);
    g.forbid_only = Some(GateFlag::False);
    let mut ci = gate(".github/workflows/ci.yml", GateTool::Ci);
    ci.steps = vec!["yarn playwright test --project chromium --grep @avt".to_owned()];
    let findings = findings_over(vec![g, ci]);
    assert!(has_kind(&findings, "forbid-only"), "{findings:#?}");
}

/// Whether `forbid-only` is reported for `apps/web/playwright.config.ts` under one CI step.
fn web_config_is_judged_under(step: &str) -> bool {
    let mut ci = gate(".github/workflows/ci.yml", GateTool::Ci);
    ci.steps = vec![step.to_owned()];
    let findings = findings_over(vec![
        gate("apps/web/playwright.config.ts", GateTool::Playwright),
        ci,
    ]);
    has_kind(&findings, "forbid-only")
}

#[test]
fn a_ci_step_that_runs_another_playwright_config_leaves_this_one_silent() {
    assert!(!web_config_is_judged_under("cd e2e && npx playwright test"));
    assert!(!web_config_is_judged_under(
        "npx playwright install --with-deps"
    ));
}

#[test]
fn a_wrapped_playwright_run_in_the_config_directory_is_a_finding() {
    assert!(web_config_is_judged_under(
        "cd apps/web && xvfb-run -a timeout 20m npx playwright test"
    ));
}

#[test]
fn a_config_directory_passed_to_playwright_selects_the_config_inside_it() {
    // Playwright loads `playwright.config.*` from a `--config` that names a directory.
    assert!(web_config_is_judged_under(
        "npx playwright test --config apps/web"
    ));
    assert!(web_config_is_judged_under(
        "npx playwright test -c apps/web/"
    ));
    assert!(web_config_is_judged_under(
        "cd apps && npx playwright test --config=./web"
    ));
    assert!(!web_config_is_judged_under(
        "npx playwright test --config apps/docs"
    ));
}

#[test]
fn the_last_config_passed_to_playwright_is_the_one_it_loads() {
    // Playwright keeps the last `-c`/`--config`, so arguments forwarded to a script override its own.
    assert!(web_config_is_judged_under(
        "npx playwright test -c playwright.config.ts -c apps/web"
    ));
    assert!(web_config_is_judged_under(
        "npx playwright test --config playwright.config.ts --config=apps/web/playwright.config.ts"
    ));
    assert!(!web_config_is_judged_under(
        "npx playwright test --config apps/web -c playwright.config.ts"
    ));
}

#[test]
fn a_playwright_run_the_reader_cannot_place_keeps_the_finding() {
    // Unknown runners and computed directories may select this config: keep the row.
    assert!(web_config_is_judged_under(
        "pnpm --filter web exec playwright test"
    ));
    assert!(web_config_is_judged_under("bunx playwright test"));
    assert!(web_config_is_judged_under(
        "cd ${{ matrix.app }} && npx playwright test"
    ));
    assert!(web_config_is_judged_under("npx playwright test -c $CONFIG"));
    // A run quoted as another program's argument: a server wrapper, a parallel runner, a shell.
    assert!(web_config_is_judged_under(
        "start-server-and-test dev 3000 \"playwright test\""
    ));
    assert!(web_config_is_judged_under(
        "concurrently -k \"npm:dev\" \"playwright test\""
    ));
    assert!(web_config_is_judged_under("sh -c 'npx playwright test'"));
}

#[test]
fn forbid_only_is_not_judged_without_ci() {
    // The finding is "CI runs it but a stray .only would silently pass"; with no CI there is no
    // build to gate, so the rule stays silent.
    let findings = findings_over(vec![gate("playwright.config.ts", GateTool::Playwright)]);
    assert!(!has_kind(&findings, "forbid-only"), "{findings:#?}");
}

// ---------------------------------------------------------------- CI runs the tests

/// The `workflows` evidence of a finding.
fn workflows_of(finding: &Finding) -> &str {
    finding
        .evidence
        .iter()
        .find(|e| e.key == "workflows")
        .map_or("", |e| e.value.as_str())
}

#[test]
fn ci_that_never_runs_the_tests_is_one_finding_at_the_workflows_directory() {
    let mut ci = gate(".github/workflows/ci.yml", GateTool::Ci);
    ci.steps = vec![
        "pnpm install".to_owned(),
        "pnpm lint".to_owned(),
        "pnpm tsc --noEmit".to_owned(),
    ];
    let findings = findings_over(vec![ci]);
    let nothing: Vec<&Finding> = findings
        .iter()
        .filter(|f| kind(f) == "ci-runs-nothing")
        .collect();
    assert_eq!(nothing.len(), 1, "{findings:#?}");
    assert_eq!(nothing[0].location.file, ".github/workflows");
    assert_eq!(nothing[0].outcome, Outcome::Finding);
    assert_eq!(workflows_of(nothing[0]), ".github/workflows/ci.yml");
    assert_eq!(
        nothing[0].id(),
        "gate-truth .github/workflows ci-runs-nothing#0"
    );
}

#[test]
fn two_workflows_that_run_nothing_are_one_finding_naming_both() {
    let mut lint = gate(".github/workflows/lint.yml", GateTool::Ci);
    lint.steps = vec!["pnpm lint".to_owned()];
    let mut types = gate(".github/workflows/types.yml", GateTool::Ci);
    types.steps = vec!["pnpm tsc --noEmit".to_owned()];
    let findings = findings_over(vec![types, lint]);
    let nothing: Vec<&Finding> = findings
        .iter()
        .filter(|f| kind(f) == "ci-runs-nothing")
        .collect();
    assert_eq!(
        nothing.len(),
        1,
        "project-wide, not per workflow: {findings:#?}"
    );
    assert_eq!(
        workflows_of(nothing[0]),
        ".github/workflows/types.yml, .github/workflows/lint.yml"
    );
    assert!(
        nothing[0].message.contains("2 workflows"),
        "{}",
        nothing[0].message
    );
}

#[test]
fn a_path_or_plugin_naming_a_runner_does_not_count_as_running_it() {
    // A turbo `type-check` fan-out reaches `tsc` inside a `config-vitest` package; a lint
    // plugin or a config file named after a runner runs no tests either.
    let mut ci = gate(".github/workflows/types.yml", GateTool::Ci);
    ci.steps = vec![
        "npx turbo type-check --concurrency=2".to_owned(),
        "cd packages/config-vitest && tsc --noEmit".to_owned(),
        "eslint --plugin eslint-plugin-jest src".to_owned(),
        "prettier --check vitest.config.ts".to_owned(),
    ];
    let findings = findings_over(vec![ci]);
    assert!(has_kind(&findings, "ci-runs-nothing"), "{findings:#?}");
}

#[test]
fn a_runner_named_by_path_version_or_quoted_script_counts() {
    for step in [
        "node_modules/.bin/jest --ci",
        "npx vitest@3 run",
        "sh -c \"vitest run\"",
        "cd packages/a && npx @stryker-mutator/core run",
    ] {
        let mut ci = gate(".github/workflows/ci.yml", GateTool::Ci);
        ci.steps = vec![step.to_owned()];
        let findings = findings_over(vec![ci]);
        assert!(
            !has_kind(&findings, "ci-runs-nothing"),
            "{step}: {findings:#?}"
        );
    }
}

#[test]
fn a_lint_workflow_beside_a_test_workflow_is_silent() {
    let mut lint = gate(".github/workflows/lint.yml", GateTool::Ci);
    lint.steps = vec!["pnpm lint".to_owned()];
    let mut test = gate(".github/workflows/test.yml", GateTool::Ci);
    test.steps = vec!["pnpm vitest run".to_owned()];
    let findings = findings_over(vec![lint, test]);
    assert!(!has_kind(&findings, "ci-runs-nothing"), "{findings:#?}");
}

#[test]
fn a_workflow_nested_in_a_package_is_not_ci() {
    // GitHub runs only the root `.github/workflows`; a package's own workflow file is read
    // into the model but never runs, so it neither counts as CI nor runs the suite.
    let mut nested = gate("packages/a/.github/workflows/ci.yml", GateTool::Ci);
    nested.steps = vec!["pnpm vitest run".to_owned()];
    let mut types = gate(".github/workflows/types.yml", GateTool::Ci);
    types.steps = vec!["pnpm tsc --noEmit".to_owned()];
    let findings = findings_over(vec![nested.clone(), types]);
    let nothing: Vec<&Finding> = findings
        .iter()
        .filter(|f| kind(f) == "ci-runs-nothing")
        .collect();
    assert_eq!(nothing.len(), 1, "{findings:#?}");
    assert_eq!(workflows_of(nothing[0]), ".github/workflows/types.yml");

    // Alone, the nested workflow is no CI at all: nothing about CI is reported, and a
    // Playwright gate is not judged for forbidOnly.
    let findings = findings_over(vec![
        nested,
        gate("playwright.config.ts", GateTool::Playwright),
    ]);
    assert!(!has_kind(&findings, "ci-runs-nothing"), "{findings:#?}");
    assert!(!has_kind(&findings, "forbid-only"), "{findings:#?}");
}

#[test]
fn ci_that_runs_vitest_with_coverage_is_silent() {
    let mut ci = gate(".github/workflows/ci.yml", GateTool::Ci);
    ci.steps = vec![
        "pnpm install".to_owned(),
        "pnpm vitest run --coverage".to_owned(),
    ];
    let findings = findings_over(vec![ci]);
    assert!(!has_kind(&findings, "ci-runs-nothing"), "{findings:#?}");
}

#[test]
fn ci_that_runs_jest_is_silent() {
    let mut ci = gate(".github/workflows/ci.yml", GateTool::Ci);
    ci.steps = vec!["npm test".to_owned()];
    let findings = findings_over(vec![ci]);
    assert!(!has_kind(&findings, "ci-runs-nothing"), "{findings:#?}");
}

#[test]
fn ci_that_runs_stryker_counts_as_running_the_tests() {
    let mut ci = gate(".github/workflows/ci.yml", GateTool::Ci);
    ci.steps = vec!["pnpm exec stryker run".to_owned()];
    let findings = findings_over(vec![ci]);
    assert!(!has_kind(&findings, "ci-runs-nothing"), "{findings:#?}");
}

#[test]
fn an_opaque_ci_gate_makes_the_row_could_not_decide() {
    // An unreadable workflow could not be decided; the conservative rule does not claim it runs
    // nothing, and says which file it could not read.
    let mut ci = gate(".github/workflows/ci.yml", GateTool::Ci);
    ci.could_not_read("the workflow is not valid YAML");
    let mut lint = gate(".github/workflows/lint.yml", GateTool::Ci);
    lint.steps = vec!["pnpm lint".to_owned()];
    let findings = findings_over(vec![ci, lint]);
    let rows: Vec<&Finding> = findings
        .iter()
        .filter(|f| kind(f) == "ci-runs-nothing")
        .collect();
    assert_eq!(rows.len(), 1, "{findings:#?}");
    assert_eq!(rows[0].outcome, Outcome::CouldNotDecide);
    assert!(
        rows[0]
            .explain
            .contains(".github/workflows/ci.yml could not be parsed"),
        "{}",
        rows[0].explain
    );
}

#[test]
fn no_ci_gate_at_all_reports_nothing_about_ci() {
    let findings = findings_over(vec![gate("vitest.config.ts", GateTool::Vitest)]);
    assert!(!has_kind(&findings, "ci-runs-nothing"), "{findings:#?}");
}

// ---------------------------------------------------------------- retries (info)

#[test]
fn a_gate_with_retries_is_an_info_finding() {
    let mut g = gate("playwright.config.ts", GateTool::Playwright);
    g.retries = Some(2);
    let findings = findings_over(vec![g]);
    let retries: Vec<&Finding> = findings.iter().filter(|f| kind(f) == "retries").collect();
    assert_eq!(retries.len(), 1, "{findings:#?}");
    assert_eq!(retries[0].severity, Severity::Info);
    assert_eq!(
        retries[0]
            .evidence
            .iter()
            .find(|e| e.key == "retries")
            .map(|e| e.value.as_str()),
        Some("2")
    );
}

#[test]
fn zero_retries_is_not_a_finding() {
    let mut g = gate("playwright.config.ts", GateTool::Playwright);
    g.retries = Some(0);
    let findings = findings_over(vec![g]);
    assert!(!has_kind(&findings, "retries"), "{findings:#?}");
}

// ---------------------------------------------------------------- determinism / severity

#[test]
fn findings_are_sorted_and_deterministic() {
    let build = || {
        let mut vitest = gate("vitest.config.ts", GateTool::Vitest);
        vitest.retries = Some(3);
        let mut stryker = gate("stryker.conf.mjs", GateTool::Stryker);
        stryker.mutation_break = Some(0.0);
        let mut ci = gate(".github/workflows/ci.yml", GateTool::Ci);
        ci.steps = vec!["pnpm lint".to_owned()];
        findings_over(vec![stryker, ci, vitest])
    };
    assert_eq!(
        build(),
        build(),
        "two runs over the same model are byte-identical"
    );
    let produced = build();
    let files: Vec<&str> = produced.iter().map(|f| f.location.file.as_str()).collect();
    let mut sorted = files.clone();
    sorted.sort_unstable();
    assert_eq!(files, sorted, "findings are sorted by file");
}

#[test]
fn the_severity_override_applies_to_the_non_info_findings() {
    let mut config = Config::default();
    config.rules.insert(
        "gate-truth".to_owned(),
        spirolo_rules::RuleConfig {
            severity: Some(Severity::Error),
            max_ratio: None,
        },
    );
    let model = Model {
        gates: vec![gate("vitest.config.ts", GateTool::Vitest)],
        ..Model::default()
    };
    let findings = GateTruth.run(&model, &config);
    let threshold = findings
        .iter()
        .find(|f| kind(f) == "thresholds-absent")
        .unwrap();
    assert_eq!(
        threshold.severity,
        Severity::Error,
        "the override lifts the config-half findings"
    );
}
