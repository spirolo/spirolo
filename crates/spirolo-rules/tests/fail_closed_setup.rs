//! `fail-closed-setup` on hand-built models: one case per branch of its semantics — a gate
//! whose four invariants all hold (silent), each invariant removed in turn (one finding
//! naming that invariant and the setup file), a project without MSW (only the two runner
//! invariants judged), a monorepo whose three gates compose one preset (one finding at the
//! preset with `packages: 3`), an inheritor that overrides (not counted, the rest still
//! lift), an MSW gap the inheritor introduced (stays at the inheritor) and an unreadable
//! gate (one could-not-decide row).
//!
//! The rule is a pure function of the model, so these tests build `Gate` rows by hand and read
//! the findings out — never how they were computed. The setup-file analysis that fills
//! `Gate.setup` is proven separately in `spirolo-model`; the binary fixture (Seam 2) proves the
//! whole path through `spirolo check`.

use spirolo_model::{Gate, GateTool, Invariant, Model, SetupInvariants};
use spirolo_rules::{Config, FailClosedSetup, Family, Finding, Outcome, Rule, Severity};

/// A Vitest gate at `source` carrying the given setup invariants.
fn gate(source: &str, setup: SetupInvariants) -> Gate {
    Gate {
        setup,
        ..Gate::blank(source, GateTool::Vitest)
    }
}

/// Every invariant present, MSW present, read from `setup_file`.
fn all_present(setup_file: &str) -> SetupInvariants {
    SetupInvariants {
        unhandled_request_error: Invariant::Present,
        console_guard: Invariant::Present,
        restore_mocks: Invariant::Present,
        global_server: Invariant::Present,
        msw_present: true,
        msw_inherited: false,
        setup_file: Some(setup_file.to_owned()),
    }
}

/// A model carrying exactly the given gates.
fn model(gates: Vec<Gate>) -> Model {
    Model {
        gates,
        ..Model::default()
    }
}

fn run(model: &Model) -> Vec<Finding> {
    FailClosedSetup.run(model, &Config::default())
}

#[test]
fn all_four_invariants_present_is_silent() {
    let model = model(vec![gate("vitest.config.ts", all_present("src/setup.ts"))]);
    assert!(
        run(&model).is_empty(),
        "a setup that fails closed on all four classes is not a finding"
    );
}

#[test]
fn a_missing_unhandled_request_error_is_one_finding_naming_it() {
    let mut setup = all_present("src/setup.ts");
    setup.unhandled_request_error = Invariant::Absent;
    let found = run(&model(vec![gate("vitest.config.ts", setup)]));
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].rule, "fail-closed-setup");
    assert!(
        found[0].message.contains("onUnhandledRequest"),
        "the message names the invariant: {}",
        found[0].message
    );
    assert_eq!(
        found[0].location.file, "src/setup.ts",
        "the finding points at the setup file"
    );
    assert!(found[0]
        .evidence
        .iter()
        .any(|e| e.key == "invariant" && e.value == "unhandled-request-error"));
}

#[test]
fn a_missing_console_guard_is_one_finding_naming_it() {
    let mut setup = all_present("src/setup.ts");
    setup.console_guard = Invariant::Absent;
    let found = run(&model(vec![gate("vitest.config.ts", setup)]));
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].message.contains("console"), "{}", found[0].message);
    assert_eq!(found[0].location.file, "src/setup.ts");
}

#[test]
fn a_missing_restore_mocks_is_one_finding_naming_it() {
    let mut setup = all_present("src/setup.ts");
    setup.restore_mocks = Invariant::Absent;
    let found = run(&model(vec![gate("vitest.config.ts", setup)]));
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].message.contains("restore"), "{}", found[0].message);
}

#[test]
fn a_missing_global_server_is_one_finding_naming_it() {
    let mut setup = all_present("src/setup.ts");
    setup.global_server = Invariant::Absent;
    let found = run(&model(vec![gate("vitest.config.ts", setup)]));
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].message.contains("server"), "{}", found[0].message);
}

#[test]
fn all_four_removed_reports_four_findings() {
    let setup = SetupInvariants {
        unhandled_request_error: Invariant::Absent,
        console_guard: Invariant::Absent,
        restore_mocks: Invariant::Absent,
        global_server: Invariant::Absent,
        msw_present: true,
        msw_inherited: false,
        setup_file: Some("src/setup.ts".to_owned()),
    };
    let found = run(&model(vec![gate("vitest.config.ts", setup)]));
    assert_eq!(
        found.len(),
        4,
        "one finding per missing invariant: {found:#?}"
    );
}

#[test]
fn without_msw_only_the_two_runner_invariants_are_judged() {
    // No MSW: the two request invariants are NotApplicable, so even absent they never fire;
    // only the console guard and mock reset are judged. Both absent here → two findings.
    let setup = SetupInvariants {
        unhandled_request_error: Invariant::NotApplicable,
        console_guard: Invariant::Absent,
        restore_mocks: Invariant::Absent,
        global_server: Invariant::NotApplicable,
        msw_present: false,
        msw_inherited: false,
        setup_file: Some("src/setup.ts".to_owned()),
    };
    let found = run(&model(vec![gate("vitest.config.ts", setup)]));
    assert_eq!(
        found.len(),
        2,
        "a project without MSW is judged only on the two runner invariants: {found:#?}"
    );
    assert!(
        found
            .iter()
            .all(|f| !f.message.contains("onUnhandledRequest") && !f.message.contains("server")),
        "the MSW invariants are not reported without a mock server: {found:#?}"
    );
}

#[test]
fn a_present_invariant_is_never_a_finding() {
    // MSW present, request invariants hold, only the console guard is absent → one finding,
    // and it is the console guard, not either request invariant.
    let mut setup = all_present("src/setup.ts");
    setup.console_guard = Invariant::Absent;
    let found = run(&model(vec![gate("vitest.config.ts", setup)]));
    assert_eq!(found.len(), 1);
    assert!(found[0].message.contains("console"));
}

#[test]
fn three_packages_composing_one_preset_report_once_at_the_preset() {
    // Three kits each merge the same preset chain and each lacks the same invariant
    // (restoreMocks). The rule reports one finding at the shared preset with packages: 3,
    // not three findings.
    let preset = vec!["packages/config/base.mjs".to_owned()];
    let mut gates = Vec::new();
    for pkg in ["a", "b", "c"] {
        let mut setup = all_present(&format!("packages/{pkg}/setup.ts"));
        setup.restore_mocks = Invariant::Absent;
        let mut g = gate(&format!("packages/{pkg}/vitest.config.ts"), setup);
        g.preset_chain = preset.clone();
        gates.push(g);
    }
    let found = run(&model(gates));
    let restore: Vec<&Finding> = found
        .iter()
        .filter(|f| f.message.contains("restore"))
        .collect();
    assert_eq!(
        restore.len(),
        1,
        "one decision, one finding — not one per package: {found:#?}"
    );
    assert_eq!(
        restore[0].location.file, "packages/config/base.mjs",
        "the finding is located at the preset"
    );
    assert!(
        restore[0]
            .evidence
            .iter()
            .any(|e| e.key == "packages" && e.value == "3"),
        "the inheriting package count is evidence: {:#?}",
        restore[0].evidence
    );
}

#[test]
fn an_inheritor_that_overrides_is_not_counted_and_the_rest_still_lift() {
    // Two kits share a preset; one sets restoreMocks in its own config, the other adds
    // nothing. The gap is still the preset's decision: one finding at the preset counting
    // the one inheritor that lacks it — no finding at that package, none for the override.
    let preset = vec!["packages/config/base.mjs".to_owned()];
    let mut fixed = all_present("packages/a/setup.ts");
    fixed.restore_mocks = Invariant::Present;
    let mut a = gate("packages/a/vitest.config.ts", fixed);
    a.preset_chain = preset.clone();

    let mut missing = all_present("packages/b/setup.ts");
    missing.restore_mocks = Invariant::Absent;
    let mut b = gate("packages/b/vitest.config.ts", missing);
    b.preset_chain = preset;

    let found = run(&model(vec![a, b]));
    let restore: Vec<&Finding> = found
        .iter()
        .filter(|f| f.message.contains("restore"))
        .collect();
    assert_eq!(restore.len(), 1, "{found:#?}");
    assert_eq!(
        restore[0].location.file, "packages/config/base.mjs",
        "the gap is decided at the preset the lacking package inherits"
    );
    assert!(
        restore[0]
            .evidence
            .iter()
            .any(|e| e.key == "packages" && e.value == "1"),
        "only the inheritor that adds nothing is counted: {:#?}",
        restore[0].evidence
    );
    assert_eq!(
        restore[0].location.line, 0,
        "no line: the preset as a whole"
    );
    assert_eq!(restore[0].anchor, "restore-mocks");
}

#[test]
fn a_preset_finding_explains_that_the_repair_belongs_at_the_preset() {
    let mut setup = all_present("packages/a/setup.ts");
    setup.console_guard = Invariant::Absent;
    let mut a = gate("packages/a/vitest.config.ts", setup);
    a.preset_chain = vec!["packages/config/base.mjs".to_owned()];
    let found = run(&model(vec![a]));
    assert_eq!(found.len(), 1);
    assert!(
        found[0].explain.contains("1 package") && found[0].explain.contains("preset"),
        "{}",
        found[0].explain
    );
}

#[test]
fn an_msw_gap_the_inheritor_introduced_stays_at_the_inheritor() {
    // The preset has no mock server; this kit's own setup starts one without
    // `onUnhandledRequest: 'error'`. That is the kit's decision, at the kit's setup file —
    // not a gap of a preset that has no server at all.
    let mut setup = all_present("packages/a/src/setup.ts");
    setup.unhandled_request_error = Invariant::Absent;
    setup.msw_inherited = false;
    let mut a = gate("packages/a/vitest.config.ts", setup);
    a.preset_chain = vec!["packages/config/base.mjs".to_owned()];
    let found = run(&model(vec![a]));
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].location.file, "packages/a/src/setup.ts");
    assert!(
        !found[0].message.contains("preset"),
        "not lifted: {}",
        found[0].message
    );
}

#[test]
fn an_msw_gap_of_the_presets_own_server_lifts_to_the_preset() {
    // The preset's setup starts the mock server (msw_inherited) without the error option:
    // every inheritor lacks it because the preset does, so it is one finding at the preset.
    let preset = vec!["packages/config/base.mjs".to_owned()];
    let mut gates = Vec::new();
    for pkg in ["a", "b"] {
        let mut setup = all_present("packages/config/setup.ts");
        setup.unhandled_request_error = Invariant::Absent;
        setup.msw_inherited = true;
        let mut g = gate(&format!("packages/{pkg}/vitest.config.ts"), setup);
        g.preset_chain = preset.clone();
        gates.push(g);
    }
    let found = run(&model(gates));
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].location.file, "packages/config/base.mjs");
    assert!(found[0]
        .evidence
        .iter()
        .any(|e| e.key == "packages" && e.value == "2"));
}

#[test]
fn a_gate_with_no_setup_file_locates_the_finding_at_the_config() {
    let setup = SetupInvariants {
        unhandled_request_error: Invariant::NotApplicable,
        console_guard: Invariant::Absent,
        restore_mocks: Invariant::Present,
        global_server: Invariant::NotApplicable,
        msw_present: false,
        msw_inherited: false,
        setup_file: None,
    };
    let found = run(&model(vec![gate("vitest.config.ts", setup)]));
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].location.file, "vitest.config.ts",
        "with no setup file the finding falls back to the config"
    );
}

#[test]
fn an_unreadable_gate_is_one_could_not_decide_row_not_four_absences() {
    // The reader could not follow the config's preset, so its setup files are unknown and
    // every invariant would read as absent — which is not a fact about the setup. One
    // could-not-decide row per gate, saying what could not be read; no decided finding.
    let mut g = gate("packages/x/vitest.config.mts", SetupInvariants::unread());
    g.could_not_read(
        "preset '@x/config/base' could not be resolved from packages/x/vitest.config.mts",
    );
    let found = run(&model(vec![g]));
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].outcome, Outcome::CouldNotDecide);
    assert_eq!(found[0].location.file, "packages/x/vitest.config.mts");
    assert_eq!(found[0].anchor, "setup-unread");
    assert!(
        found[0]
            .explain
            .contains("'@x/config/base' could not be resolved"),
        "{}",
        found[0].explain
    );
    assert!(found[0]
        .evidence
        .iter()
        .any(|e| e.key == "unread" && e.value.contains("@x/config/base")));
}

#[test]
fn an_unreadable_gate_does_not_join_its_preset_group() {
    // Two inheritors of one preset lack the same invariant; a third names the preset but
    // could not be read whole. The lift counts the two decided ones; the third is a
    // could-not-decide row of its own, not a third inheritor of the gap.
    let preset = vec!["packages/config/base.mjs".to_owned()];
    let mut gates = Vec::new();
    for pkg in ["a", "b"] {
        let mut setup = all_present(&format!("packages/{pkg}/setup.ts"));
        setup.restore_mocks = Invariant::Absent;
        let mut g = gate(&format!("packages/{pkg}/vitest.config.ts"), setup);
        g.preset_chain = preset.clone();
        gates.push(g);
    }
    let mut unread = gate("packages/c/vitest.config.ts", SetupInvariants::unread());
    unread.preset_chain = preset;
    unread.could_not_read("a spread (`...`) in the config object");
    gates.push(unread);
    let found = run(&model(gates));
    let restore: Vec<&Finding> = found
        .iter()
        .filter(|f| f.anchor == "restore-mocks")
        .collect();
    assert_eq!(restore.len(), 1, "{found:#?}");
    assert!(restore[0]
        .evidence
        .iter()
        .any(|e| e.key == "packages" && e.value == "2"));
    assert_eq!(
        found
            .iter()
            .filter(|f| f.outcome == Outcome::CouldNotDecide)
            .count(),
        1
    );
}

#[test]
fn a_non_runner_gate_is_ignored() {
    // A Playwright or Stryker gate has no setup invariants; the rule reads only Vitest/Jest.
    let mut g = gate("playwright.config.ts", SetupInvariants::unread());
    g.tool = GateTool::Playwright;
    assert!(run(&model(vec![g])).is_empty());
}

#[test]
fn findings_carry_the_could_not_decide_free_outcome_and_a_repair_explain() {
    let mut setup = all_present("src/setup.ts");
    setup.console_guard = Invariant::Absent;
    let found = run(&model(vec![gate("vitest.config.ts", setup)]));
    assert_eq!(found[0].outcome, Outcome::Finding);
    assert!(
        !found[0].explain.is_empty() && !found[0].explain.to_lowercase().contains("remove the"),
        "the explain repairs rather than deletes: {}",
        found[0].explain
    );
}

#[test]
fn identity_family_and_severity() {
    assert_eq!(FailClosedSetup.name(), "fail-closed-setup");
    assert_eq!(FailClosedSetup.family(), Family::Gate);
    assert_eq!(FailClosedSetup.default_severity(), Severity::Warn);
}
