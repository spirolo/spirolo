//! `orphan-test` on hand-built models: one case per branch the acceptance criteria name — a
//! file outside every include, a file an exclude removes, a file a second runner config
//! collects (must not fire), a dead relative import, a dead alias import, and a bare-package
//! import (must not fire).
//!
//! The rule is a pure function of the model, so these tests build the `Uncollected` rows and
//! the `Module` imports by hand and read the findings out. The binary fixture (Seam 2) proves
//! the same rule end to end through `spirolo hygiene --rule orphan-test`.

use spirolo_model::{Import, ImportKind, Model, Module, Span, TestCase, TestKind, Uncollected};
use spirolo_rules::{Config, Finding, OrphanTest, Outcome, Proposal, Rule, Severity};

/// A recognised, collected test case in `file` — a real test, so `file` is a test file.
fn test_case(file: &str) -> TestCase {
    TestCase {
        file: file.to_owned(),
        span: Span::default(),
        framework: Some("vitest".to_owned()),
        title: Some("t".to_owned()),
        kind: TestKind::Test,
        suite_path: Vec::new(),
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

/// An `Uncollected` row for `file`, judged against `config` with the given globs.
fn uncollected(
    file: &str,
    config: Option<&str>,
    include: &[&str],
    exclude: &[&str],
) -> Uncollected {
    Uncollected {
        file: file.to_owned(),
        framework: Some("vitest".to_owned()),
        config: config.map(str::to_owned),
        include: include.iter().map(|g| (*g).to_owned()).collect(),
        exclude: exclude.iter().map(|g| (*g).to_owned()).collect(),
        outside_workspace: false,
    }
}

/// A module at `path` importing each `(specifier, reason)`: `reason` `None` means it
/// resolved, `Some("missing")` a dead project import, `Some("unknown_package")` a bare one.
fn module(path: &str, imports: &[(&str, Option<&str>)]) -> Module {
    Module {
        path: path.to_owned(),
        language: spirolo_model::Language::TypeScript,
        parse_status: spirolo_model::ParseStatus::Parsed,
        imports: imports
            .iter()
            .enumerate()
            .map(|(index, (specifier, reason))| {
                let index = u32::try_from(index).expect("import index fits u32");
                Import {
                    specifier: (*specifier).to_owned(),
                    resolved: None,
                    reason: reason.map(str::to_owned),
                    kind: ImportKind::Static,
                    span: Span {
                        start: index * 100,
                        end: index * 100 + 10,
                        line: index + 1,
                        column: 1,
                    },
                }
            })
            .collect(),
        exports: Vec::new(),
        constants: Vec::new(),
        mocks: Vec::new(),
        references: Vec::new(),
        spies: Vec::new(),
        loads: Vec::new(),
        complexity: 1,
        generated: false,
    }
}

fn findings(model: &Model) -> Vec<Finding> {
    OrphanTest.run(model, &Config::default())
}

/// The findings a run under `--propose` produces.
fn proposed(model: &Model) -> Vec<Finding> {
    OrphanTest.run(
        model,
        &Config {
            propose: true,
            ..Config::default()
        },
    )
}

#[test]
fn under_propose_a_never_collected_file_is_kept_with_the_include_glob_reason() {
    let model = Model {
        uncollected: vec![uncollected(
            "src/orphan.test.ts",
            Some("vitest.config.ts"),
            &["**/*.unit.ts"],
            &[],
        )],
        ..Model::default()
    };
    assert_eq!(
        proposed(&model)[0].proposal,
        Some(Proposal::Keep {
            reason: "add the include glob".to_owned()
        }),
        "a file that exists but no runner collects is kept — the fix is a glob, not a deletion"
    );
}

#[test]
fn under_propose_a_source_gone_test_is_kept_with_the_repairs_the_explain_offers() {
    let model = Model {
        test_cases: vec![test_case("src/a.test.ts")],
        modules: vec![module("src/a.test.ts", &[("./gone", Some("missing"))])],
        ..Model::default()
    };
    assert_eq!(
        proposed(&model)[0].proposal,
        Some(Proposal::Keep {
            reason: "restore the module, fix the path, or delete the test — spirolo cannot tell \
                     a moved module from a removed one"
                .to_owned()
        }),
        "a dead import is not a deletion by itself: the proposal offers what the explain offers"
    );
}

// ------------------------------------------------------------------ never collected

#[test]
fn a_recognised_file_no_runner_collects_is_a_never_collected_finding() {
    let model = Model {
        uncollected: vec![uncollected(
            "src/orphan.test.ts",
            Some("vitest.config.ts"),
            &["**/*.unit.ts"],
            &[],
        )],
        ..Model::default()
    };
    let findings = findings(&model);
    assert_eq!(findings.len(), 1, "{findings:#?}");
    let finding = &findings[0];
    assert_eq!(finding.rule, "orphan-test");
    assert_eq!(finding.severity, Severity::Warn);
    assert_eq!(finding.outcome, Outcome::Finding);
    assert_eq!(finding.location.file, "src/orphan.test.ts");
    let kind = finding.evidence.iter().find(|e| e.key == "kind").unwrap();
    assert_eq!(kind.value, "never-collected");
    let config = finding.evidence.iter().find(|e| e.key == "config").unwrap();
    assert_eq!(
        config.value, "vitest.config.ts",
        "the config it was judged against"
    );
    assert!(
        finding.explain.contains("smaller than it looks"),
        "the explain says the suite is smaller than it looks: {}",
        finding.explain
    );
}

#[test]
fn a_file_an_exclude_removes_is_a_never_collected_finding_naming_the_exclude() {
    let model = Model {
        uncollected: vec![uncollected(
            "src/generated/b.test.ts",
            Some("vitest.config.ts"),
            &["**/*.test.ts"],
            &["**/generated/**"],
        )],
        ..Model::default()
    };
    let findings = findings(&model);
    assert_eq!(findings.len(), 1);
    let exclude = findings[0]
        .evidence
        .iter()
        .find(|e| e.key == "exclude")
        .unwrap();
    assert_eq!(exclude.value, "**/generated/**");
}

#[test]
fn a_file_a_second_runner_config_collects_is_not_uncollected_so_it_does_not_fire() {
    // The file is collected by another config: scoping kept its cases, so it is in
    // `test_cases`, not in `uncollected`. The rule reads `uncollected`, so it never fires.
    let model = Model {
        test_cases: vec![test_case("packages/b/src/y.test.ts")],
        ..Model::default()
    };
    assert!(
        findings(&model).is_empty(),
        "a collected file is not a never-collected orphan"
    );
}

// ------------------------------------------------------------------ source gone

#[test]
fn a_dead_relative_import_in_a_test_file_is_a_source_gone_finding() {
    let model = Model {
        test_cases: vec![test_case("src/a.test.ts")],
        modules: vec![module("src/a.test.ts", &[("./gone", Some("missing"))])],
        ..Model::default()
    };
    let findings = findings(&model);
    assert_eq!(findings.len(), 1, "{findings:#?}");
    let finding = &findings[0];
    assert_eq!(finding.location.file, "src/a.test.ts");
    assert_eq!(
        finding.location.line, 1,
        "reported at the import, not the file head"
    );
    let kind = finding.evidence.iter().find(|e| e.key == "kind").unwrap();
    assert_eq!(kind.value, "source-gone");
    let specifier = finding
        .evidence
        .iter()
        .find(|e| e.key == "specifier")
        .unwrap();
    assert_eq!(specifier.value, "./gone");
}

#[test]
fn any_import_the_model_marks_missing_is_a_source_gone_finding_whatever_its_shape() {
    // The rule keys on the model's `missing` verdict, not the specifier's shape: an
    // alias-shaped import the model marked `missing` fires the same as a relative one. (In
    // practice the model is conservative and marks only relative imports missing — an alias
    // failure is left unresolved without claiming the source is gone — so this is the rule's
    // contract, tested on a hand-built model rather than a claim about what the model emits.)
    let model = Model {
        test_cases: vec![test_case("src/a.test.ts")],
        modules: vec![module(
            "src/a.test.ts",
            &[("@scope/pkg/gone", Some("missing"))],
        )],
        ..Model::default()
    };
    let findings = findings(&model);
    assert_eq!(findings.len(), 1, "{findings:#?}");
    assert_eq!(
        findings[0]
            .evidence
            .iter()
            .find(|e| e.key == "specifier")
            .unwrap()
            .value,
        "@scope/pkg/gone"
    );
}

#[test]
fn a_bare_package_import_does_not_fire() {
    // An unresolved bare package may simply be uninstalled: `unknown_package`, not a
    // source-gone. An `external` and a resolved import never fire either.
    let model = Model {
        test_cases: vec![test_case("src/a.test.ts")],
        modules: vec![module(
            "src/a.test.ts",
            &[
                ("lodash", Some("unknown_package")),
                ("react", Some("external")),
                ("./present", None),
            ],
        )],
        ..Model::default()
    };
    assert!(
        findings(&model).is_empty(),
        "only a missing project import is source-gone: {:#?}",
        findings(&model)
    );
}

#[test]
fn a_missing_import_in_a_non_test_module_does_not_fire() {
    // Source-gone is a fact about a *test* file. A production module with a dead import is
    // not this rule's concern.
    let model = Model {
        test_cases: Vec::new(),
        modules: vec![module("src/helper.ts", &[("./gone", Some("missing"))])],
        ..Model::default()
    };
    assert!(findings(&model).is_empty());
}

#[test]
fn both_kinds_are_reported_together() {
    let model = Model {
        test_cases: vec![test_case("src/a.test.ts")],
        modules: vec![module("src/a.test.ts", &[("./gone", Some("missing"))])],
        uncollected: vec![uncollected(
            "src/orphan.test.ts",
            None,
            &["**/*.unit.ts"],
            &[],
        )],
        ..Model::default()
    };
    let findings = findings(&model);
    assert_eq!(findings.len(), 2, "one of each kind: {findings:#?}");
    assert!(findings.iter().all(|f| f.rule == "orphan-test"));
}
