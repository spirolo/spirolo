//! `unused-mock` on hand-built models: one case per branch of its two kinds — a module mock
//! reachable by a direct import, one reachable through a transitive project import, an
//! unreachable project mock, an unreachable external mock, an external mock reached by a
//! subpath import, a read spy, an unread spy, and a `setupFiles` mock reported once.
//!
//! The rule is a pure function of the model, so these tests build `Module` rows by hand and
//! read the findings out — never how they were computed. The binary fixture (Seam 2) proves
//! the same rule end to end through `spirolo hygiene`.

use spirolo_model::{
    Gate, GateTool, IdentRef, Import, ImportKind, Mock, MockKind, Model, Module, Span, Spy,
    TestCase, TestKind,
};

/// A spy binding of `name`.
fn spy(name: &str, line: u32) -> Spy {
    Spy {
        binding: name.to_owned(),
        span: span(line),
    }
}

/// An identifier reference (read) of `name`.
fn reference(name: &str, line: u32) -> IdentRef {
    IdentRef {
        name: name.to_owned(),
        span: span(line),
    }
}
use spirolo_rules::{
    gate_proposals, Config, Finding, Outcome, Proposal, Rule, Severity, UnusedMock,
    PROPOSALS_CLEARED,
};

/// A span whose byte offset tracks `line`, so file-and-span ordering is well defined.
const fn span(line: u32) -> Span {
    Span {
        start: line * 100,
        end: line * 100 + 10,
        line,
        column: 1,
    }
}

/// A module mock of `target`, resolved to `resolved` (a project path) or left external.
fn module_mock(target: &str, resolved: Option<&str>, line: u32) -> Mock {
    Mock {
        target: Some(target.to_owned()),
        resolved: resolved.map(str::to_owned),
        reason: None,
        kind: MockKind::Module,
        factory: false,
        span: span(line),
    }
}

/// A static import of `specifier`, resolved to a project path or carrying a `reason`.
fn import(specifier: &str, resolved: Option<&str>, reason: Option<&str>, line: u32) -> Import {
    Import {
        specifier: specifier.to_owned(),
        resolved: resolved.map(str::to_owned),
        reason: reason.map(str::to_owned),
        kind: ImportKind::Static,
        span: span(line),
    }
}

/// A module at `path` with the given imports, mocks, references and spies.
fn module(
    path: &str,
    imports: Vec<Import>,
    mocks: Vec<Mock>,
    references: Vec<IdentRef>,
    spies: Vec<Spy>,
) -> Module {
    Module {
        path: path.to_owned(),
        language: spirolo_model::Language::TypeScript,
        parse_status: spirolo_model::ParseStatus::Parsed,
        imports,
        exports: Vec::new(),
        constants: Vec::new(),
        mocks,
        references,
        spies,
        loads: Vec::new(),
        complexity: 1,
        generated: false,
    }
}

/// A `test(…)` row in `file`, so the file counts as a test file.
fn test_case(file: &str, line: u32) -> TestCase {
    TestCase {
        file: file.to_owned(),
        span: span(line),
        framework: Some("vitest".to_owned()),
        title: Some("a test".to_owned()),
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

/// A model of the given modules and test files.
fn model(modules: Vec<Module>, cases: Vec<TestCase>) -> Model {
    Model {
        modules,
        test_cases: cases,
        ..Model::default()
    }
}

/// The `unused-mock` findings the rule reports over `model`.
fn findings(model: &Model) -> Vec<Finding> {
    UnusedMock.run(model, &Config::default())
}

/// The findings a run under `--propose` produces.
fn proposed(model: &Model) -> Vec<Finding> {
    UnusedMock.run(
        model,
        &Config {
            propose: true,
            ..Config::default()
        },
    )
}

#[test]
fn under_propose_an_unused_mock_is_deleted() {
    let model = model(
        vec![module(
            "foo.test.ts",
            Vec::new(),
            vec![module_mock("./gone", Some("src/gone.ts"), 2)],
            Vec::new(),
            Vec::new(),
        )],
        vec![test_case("foo.test.ts", 4)],
    );
    assert_eq!(
        proposed(&model)[0].proposal,
        Some(Proposal::Delete { what: None }),
        "a mock nothing reaches is dead configuration: delete it"
    );
}

#[test]
fn the_gate_keeps_a_delete_until_the_rule_is_cleared_and_leaves_an_undecided_keep_alone() {
    // One unused mock (a delete) and one that may be reached (a could-not-decide keep).
    let model = model(
        vec![
            module(
                "foo.test.ts",
                Vec::new(),
                vec![module_mock("./gone", Some("src/gone.ts"), 2)],
                Vec::new(),
                Vec::new(),
            ),
            module(
                "bar.test.ts",
                vec![import(
                    "@/supabaseAdminClient",
                    None,
                    Some("unknown_package"),
                    1,
                )],
                vec![module_mock("@supabase/supabase-js", None, 2)],
                Vec::new(),
                Vec::new(),
            ),
        ],
        vec![test_case("foo.test.ts", 4), test_case("bar.test.ts", 4)],
    );
    let raw = proposed(&model);
    let undecided = |found: &[Finding]| {
        found
            .iter()
            .find(|f| f.outcome == Outcome::CouldNotDecide)
            .unwrap()
            .proposal
            .clone()
    };

    let mut cleared = raw.clone();
    gate_proposals(&mut cleared, &["unused-mock"]);
    assert_eq!(cleared, raw, "a cleared rule's proposals pass unchanged");

    assert!(
        PROPOSALS_CLEARED.is_empty(),
        "no hygiene rule has an independent-oracle PASS yet"
    );
    let mut gated = raw.clone();
    gate_proposals(&mut gated, PROPOSALS_CLEARED);
    let reported = gated
        .iter()
        .find(|f| f.outcome == Outcome::Finding)
        .unwrap();
    assert!(
        matches!(&reported.proposal, Some(Proposal::Keep { reason }) if reason.contains("independent oracle")),
        "an uncleared rule's delete becomes a keep saying why: {gated:?}"
    );
    assert_eq!(
        undecided(&gated),
        undecided(&raw),
        "a could-not-decide keep keeps its own reason"
    );
}

#[test]
fn an_unreachable_project_module_mock_is_reported() {
    let model = model(
        vec![module(
            "foo.test.ts",
            Vec::new(),
            vec![module_mock("./gone", Some("src/gone.ts"), 2)],
            Vec::new(),
            Vec::new(),
        )],
        vec![test_case("foo.test.ts", 4)],
    );
    let found = findings(&model);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].location.file, "foo.test.ts");
    assert_eq!(found[0].outcome, Outcome::Finding);
    assert_eq!(found[0].severity, Severity::Warn);
}

#[test]
fn a_module_mock_reached_by_a_direct_import_is_not_reported() {
    let model = model(
        vec![module(
            "foo.test.ts",
            vec![import("./foo", Some("src/foo.ts"), None, 1)],
            vec![module_mock("./foo", Some("src/foo.ts"), 2)],
            Vec::new(),
            Vec::new(),
        )],
        vec![test_case("foo.test.ts", 4)],
    );
    assert!(findings(&model).is_empty(), "{:?}", findings(&model));
}

#[test]
fn an_unreachable_external_module_mock_is_reported() {
    let model = model(
        vec![module(
            "foo.test.ts",
            vec![import("./foo", Some("src/foo.ts"), None, 1)],
            vec![module_mock("@supabase/supabase-js", None, 2)],
            Vec::new(),
            Vec::new(),
        )],
        vec![test_case("foo.test.ts", 4)],
    );
    let found = findings(&model);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].anchor, "@supabase/supabase-js");
}

#[test]
fn an_external_mock_reached_by_a_subpath_import_is_not_reported() {
    let model = model(
        vec![module(
            "foo.test.ts",
            vec![import(
                "@supabase/supabase-js/dist/module",
                None,
                Some("external"),
                1,
            )],
            vec![module_mock("@supabase/supabase-js", None, 2)],
            Vec::new(),
            Vec::new(),
        )],
        vec![test_case("foo.test.ts", 4)],
    );
    assert!(findings(&model).is_empty(), "{:?}", findings(&model));
}

#[test]
fn a_module_mock_reached_through_a_transitive_project_import_is_not_reported() {
    let model = model(
        vec![
            module(
                "foo.test.ts",
                vec![import("./bar", Some("src/bar.ts"), None, 1)],
                vec![module_mock("./foo", Some("src/foo.ts"), 2)],
                Vec::new(),
                Vec::new(),
            ),
            module(
                "src/bar.ts",
                vec![import("./foo", Some("src/foo.ts"), None, 1)],
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ),
        ],
        vec![test_case("foo.test.ts", 4)],
    );
    assert!(findings(&model).is_empty(), "{:?}", findings(&model));
}

#[test]
fn an_unreached_mock_behind_an_unresolved_bare_import_is_could_not_decide() {
    // #133: the test reaches `@x/kit`, a package the model could not resolve (a workspace
    // package it did not walk into, an alias, an uninstalled dependency). What `@x/kit`
    // imports is unknown, so whether `axios` is reached is too: never a finding.
    let model = model(
        vec![
            module(
                "foo.test.ts",
                vec![import("./run", Some("src/run.ts"), None, 1)],
                vec![module_mock("axios", None, 2)],
                Vec::new(),
                Vec::new(),
            ),
            module(
                "src/run.ts",
                vec![import("@x/kit", None, Some("unknown_package"), 1)],
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ),
        ],
        vec![test_case("foo.test.ts", 4)],
    );
    let found = findings(&model);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].outcome, Outcome::CouldNotDecide, "{found:?}");
    assert_eq!(found[0].anchor, "axios");
}

#[test]
fn an_installed_external_import_leaves_the_closure_complete() {
    // The near-miss of the case above: `vitest` resolved to an installed package under
    // `node_modules` is a leaf the runner never mocks inside, not a gap, so the unreached
    // `axios` mock is still decided.
    let model = model(
        vec![
            module(
                "foo.test.ts",
                vec![
                    import("vitest", None, Some("external"), 1),
                    import("./foo", Some("src/foo.ts"), None, 2),
                ],
                vec![module_mock("axios", None, 3)],
                Vec::new(),
                Vec::new(),
            ),
            module("src/foo.ts", Vec::new(), Vec::new(), Vec::new(), Vec::new()),
        ],
        vec![test_case("foo.test.ts", 5)],
    );
    let found = findings(&model);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].outcome, Outcome::Finding, "{found:?}");
}

#[test]
fn an_unreached_mock_behind_a_computed_import_is_could_not_decide() {
    // A kit's cli.test.ts mocks `../dispatch` and loads its CLI
    // through `import(specifier)`: the module loaded is decided at run time, so whether the
    // mock is reached is not known from the imports.
    let model = model(
        vec![module(
            "src/__tests__/cli.test.ts",
            vec![
                import("vitest", None, Some("external"), 2),
                import("specifier", None, Some("computed"), 60),
            ],
            vec![module_mock("../dispatch", Some("src/dispatch.ts"), 35)],
            Vec::new(),
            Vec::new(),
        )],
        vec![test_case("src/__tests__/cli.test.ts", 72)],
    );
    let found = findings(&model);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].outcome, Outcome::CouldNotDecide, "{found:?}");
}

/// A module mock of `target` the resolver could not resolve, for `reason`.
fn unresolved_mock(target: &str, reason: &str, line: u32) -> Mock {
    Mock {
        reason: Some(reason.to_owned()),
        ..module_mock(target, None, line)
    }
}

#[test]
fn a_mock_whose_target_did_not_resolve_is_could_not_decide() {
    // A dashboard tsconfig excludes `__tests__`, so a test's `@/lib/email-client`
    // alias resolves to nothing: it is no installed package, and which module it names is
    // unknown, so whether the closure reaches it is too.
    let model = model(
        vec![module(
            "foo.test.ts",
            vec![import("../route", Some("src/route.ts"), None, 1)],
            vec![unresolved_mock("@/lib/email-client", "unknown_package", 2)],
            Vec::new(),
            Vec::new(),
        )],
        vec![test_case("foo.test.ts", 4)],
    );
    let found = findings(&model);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].outcome, Outcome::CouldNotDecide, "{found:?}");
}

#[test]
fn a_mock_whose_unresolved_target_the_closure_writes_verbatim_is_not_reported() {
    // An API route test mocks `@/lib/ai-provider-helpers`, which does
    // not resolve from the test file, while the route it imports writes the same specifier
    // (resolved from there): the runner resolves the mock and the import alike, so the mock
    // is reached.
    let model = model(
        vec![
            module(
                "foo.test.ts",
                vec![import("../route", Some("src/route.ts"), None, 1)],
                vec![unresolved_mock(
                    "@/lib/ai-provider-helpers",
                    "unknown_package",
                    2,
                )],
                Vec::new(),
                Vec::new(),
            ),
            module(
                "src/route.ts",
                vec![import(
                    "@/lib/ai-provider-helpers",
                    Some("src/lib/ai-provider-helpers.ts"),
                    None,
                    3,
                )],
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ),
        ],
        vec![test_case("foo.test.ts", 4)],
    );
    assert!(findings(&model).is_empty(), "{:?}", findings(&model));
}

#[test]
fn a_node_builtin_import_leaves_the_closure_complete() {
    // `node:fs/promises` is Node's own module: it imports nothing of the project, so it is
    // no gap, and the unreached `./db` mock is decided.
    let model = model(
        vec![module(
            "foo.test.ts",
            vec![import("node:fs/promises", None, Some("builtin"), 1)],
            vec![module_mock("./db", Some("src/db.ts"), 2)],
            Vec::new(),
            Vec::new(),
        )],
        vec![test_case("foo.test.ts", 4)],
    );
    let found = findings(&model);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].outcome, Outcome::Finding, "{found:?}");
}

#[test]
fn an_unreached_mock_behind_a_missing_relative_import_is_could_not_decide() {
    let model = model(
        vec![module(
            "foo.test.ts",
            vec![import("./gone", None, Some("missing"), 1)],
            vec![module_mock("./db", Some("src/db.ts"), 2)],
            Vec::new(),
            Vec::new(),
        )],
        vec![test_case("foo.test.ts", 4)],
    );
    let found = findings(&model);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].outcome, Outcome::CouldNotDecide, "{found:?}");
}

#[test]
fn under_propose_an_undecided_mock_is_kept() {
    let model = model(
        vec![module(
            "foo.test.ts",
            vec![import(
                "@/supabaseAdminClient",
                None,
                Some("unknown_package"),
                1,
            )],
            vec![module_mock("@supabase/supabase-js", None, 2)],
            Vec::new(),
            Vec::new(),
        )],
        vec![test_case("foo.test.ts", 4)],
    );
    let found = proposed(&model);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        matches!(found[0].proposal, Some(Proposal::Keep { .. })),
        "a mock that may be reached is never proposed for deletion: {found:?}"
    );
}

#[test]
fn a_spy_bound_to_a_name_nothing_reads_is_reported() {
    // `const orphan = vi.fn()` with no later read of `orphan`.
    let model = model(
        vec![module(
            "foo.test.ts",
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![spy("orphan", 2)],
        )],
        vec![test_case("foo.test.ts", 4)],
    );
    let found = findings(&model);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].anchor, "orphan");
    assert_eq!(found[0].outcome, Outcome::Finding);
}

#[test]
fn a_spy_bound_to_a_name_the_file_reads_is_not_reported() {
    // `const spy = vi.fn(); expect(spy).toHaveBeenCalled()` — `spy` is read.
    let model = model(
        vec![module(
            "foo.test.ts",
            Vec::new(),
            Vec::new(),
            vec![reference("spy", 3)],
            vec![spy("spy", 2)],
        )],
        vec![test_case("foo.test.ts", 4)],
    );
    assert!(findings(&model).is_empty(), "{:?}", findings(&model));
}

/// A Vitest gate at `source` loading `setup_files`, matching test files by `include`.
fn gate(source: &str, setup_files: Vec<&str>, include: Vec<&str>) -> Gate {
    let mut gate = Gate::blank(source, GateTool::Vitest);
    gate.setup_files = setup_files.into_iter().map(str::to_owned).collect();
    gate.include = include.into_iter().map(str::to_owned).collect();
    gate
}

#[test]
fn a_setup_file_module_mock_is_reported_once_at_the_setup_file() {
    // A mock of `@supabase/supabase-js` in the setup file that both test files load; neither
    // reaches supabase, so it is reported ONCE at the setup file, not once per test file.
    let mut model = model(
        vec![
            module(
                "src/setup.ts",
                Vec::new(),
                vec![module_mock("@supabase/supabase-js", None, 1)],
                Vec::new(),
                Vec::new(),
            ),
            module("a.test.ts", Vec::new(), Vec::new(), Vec::new(), Vec::new()),
            module("b.test.ts", Vec::new(), Vec::new(), Vec::new(), Vec::new()),
        ],
        vec![test_case("a.test.ts", 4), test_case("b.test.ts", 4)],
    );
    model.gates = vec![gate(
        "vitest.config.ts",
        vec!["src/setup.ts"],
        vec!["**/*.test.ts"],
    )];
    let found = findings(&model);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].location.file, "src/setup.ts");
    assert_eq!(found[0].anchor, "@supabase/supabase-js");
}

#[test]
fn the_rule_declares_its_identity() {
    assert_eq!(UnusedMock.name(), "unused-mock");
    assert_eq!(UnusedMock.family(), spirolo_rules::Family::Hygiene);
    assert_eq!(UnusedMock.default_severity(), Severity::Warn);
}
