//! What the model *says* about a project once the runner's own view is applied: which
//! recognised cases survive `scope`, with the recognisers wired in as the binary wires
//! them. Fixture projects isolate one concern each; every assertion reads the tables.

use std::path::{Path, PathBuf};

use spirolo_frameworks::recognisers;
use spirolo_model::{BuildOptions, Gate, Model, TestCase, TestKind};

/// The fixture project of that name, as a model root.
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn build(name: &str) -> Model {
    Model::build_with(&fixture(name), &BuildOptions::default(), &recognisers())
}

/// The files that still carry a test after scoping, sorted, one entry per file.
fn test_files(model: &Model) -> Vec<&str> {
    let mut files: Vec<&str> = model
        .test_cases
        .iter()
        .filter(|case| case.kind == TestKind::Test)
        .map(|case| case.file.as_str())
        .collect();
    files.sort_unstable();
    files.dedup();
    files
}

fn gate<'a>(model: &'a Model, source: &str) -> &'a Gate {
    model
        .gates
        .iter()
        .find(|gate| gate.source == source)
        .unwrap_or_else(|| panic!("a gate at {source}"))
}

// ---------------------------------------------------------- mergeConfig + imported preset

#[test]
fn a_merge_config_gate_inherits_the_globs_of_its_imported_preset_chain() {
    let model = build("preset-chain");
    let gate = gate(&model, "packages/kit/vitest.config.mts");
    // The kit's own inline part is read by the recogniser; the preset chain (react →
    // base) is followed through the workspace resolver and its globs concatenated on, the
    // preset's first, as `mergeConfig` concatenates arrays.
    assert_eq!(gate.include, ["**/*.unit.{ts,tsx}"]);
    assert_eq!(
        gate.exclude,
        ["**/node_modules/**", "**/skip/**", "**/*.unit.tsx"]
    );
    assert_eq!(gate.setup_files, ["./src/setup.ts"]);
}

#[test]
fn a_package_scoped_by_an_imported_preset_keeps_exactly_what_the_runner_collects() {
    let model = build("preset-chain");
    assert_eq!(
        test_files(&model),
        ["packages/kit/src/a.unit.ts"],
        "b is outside the include, c under the preset's exclude, d under the kit's own"
    );
}

#[test]
fn preset_chain_snapshot() {
    let model = build("preset-chain");
    insta::assert_json_snapshot!("preset_chain_gates", model.gates);
}

// ---------------------------------------------------------- opaque include → defaults

#[test]
fn an_unreadable_config_falls_back_to_the_runner_defaults_instead_of_dropping_the_package() {
    let model = build("opaque-config");
    assert!(
        gate(&model, "vitest.config.ts").opaque,
        "a function export is opaque"
    );
    assert_eq!(
        test_files(&model),
        ["src/a.test.ts", "src/c.spec.mts"],
        "Vitest's default include decides: `*.test.ts` and `*.spec.mts` yes, `*.unit.ts` no"
    );
}

// ---------------------------------------------------------- workspace boundary

#[test]
fn files_and_configs_outside_the_workspace_members_are_not_the_runners_business() {
    let model = build("workspace-boundary");
    assert_eq!(
        test_files(&model),
        ["packages/a/src/a.test.ts"],
        "the archived package's own config does not make its tests count, nor does scripts/"
    );
    // The archived config is still a fact about the tree: tabled, just not applied.
    assert!(model
        .gates
        .iter()
        .any(|gate| gate.source == "archive/old-kit/vitest.config.ts"));
}

// ---------------------------------------------------------- Playwright scope

#[test]
fn playwright_specs_answer_to_test_dir_and_vitest_includes_never_drop_them() {
    let model = build("playwright-scope");
    let by_framework = |framework: &str| -> Vec<&str> {
        let mut files: Vec<&str> = model
            .test_cases
            .iter()
            .filter(|case| case.kind == TestKind::Test)
            .filter(|case| case.framework.as_deref() == Some(framework))
            .map(|case| case.file.as_str())
            .collect();
        files.sort_unstable();
        files.dedup();
        files
    };
    assert_eq!(
        by_framework("playwright"),
        ["packages/e2e/tests/home.spec.ts"],
        "only specs under testDir; the sibling package's vitest include is not consulted"
    );
    assert_eq!(by_framework("vitest"), ["packages/unit/src/x.unit.ts"]);
    let playwright_tests = model
        .test_cases
        .iter()
        .filter(|case| case.framework.as_deref() == Some("playwright"))
        .filter(|case| case.kind == TestKind::Test)
        .count();
    assert_eq!(
        playwright_tests, 2,
        "three browser projects do not multiply the two declared tests"
    );
}

// ---------------------------------------------------------- tables read from bindings

/// The rows of one file, reduced to what the count depends on.
fn tables(model: &Model, file: &str) -> Vec<serde_json::Value> {
    model
        .test_cases
        .iter()
        .filter(|case| case.file == file)
        .filter(|case| matches!(case.kind, TestKind::Test | TestKind::Suite))
        .map(|case: &TestCase| {
            serde_json::json!({
                "kind": case.kind,
                "title": case.title,
                "suite_path": case.suite_path,
                "each": case.each,
                "loops": case.loops,
            })
        })
        .collect()
}

#[test]
fn tables_read_from_other_bindings_are_counted_through_imports_and_re_exports() {
    let model = build("each-sources");
    insta::assert_json_snapshot!("each_sources_tables", tables(&model, "src/tables.test.ts"));
}

#[test]
fn an_imported_table_counts_its_elements_spreads_filters_and_keys() {
    let model = build("each-sources");
    let rows = tables(&model, "src/tables.test.ts");
    let cases = |title: &str| -> Option<u64> {
        rows.iter()
            .find(|row| row["title"] == title)
            .and_then(|row| {
                let table = if row["each"].is_null() {
                    &row["loops"][0]
                } else {
                    &row["each"]
                };
                table["cases"].as_u64()
            })
    };
    assert_eq!(
        cases("handles %s"),
        Some(3),
        "ROWS, an `as const` array re-exported"
    );
    assert_eq!(
        cases("%s"),
        Some(3),
        "LEXICAL = CASES.filter((c) => !c.hard): 4 - 1"
    );
    assert_eq!(
        cases("maps %s"),
        Some(2),
        "Object.keys of an object literal"
    );
    assert_eq!(
        cases("local %i"),
        Some(3),
        "a block-scoped `as const` array"
    );
    assert_eq!(
        cases("runtime %s"),
        None,
        "a computed table stays uncounted"
    );
    let loop_row = rows
        .iter()
        .find(|row| row["loops"][0]["source"] == "PACK.phrases")
        .expect("the for-of over PACK.phrases");
    assert_eq!(
        loop_row["loops"][0]["cases"], 4,
        "two spreads (2 + 1) and one inline element, through a property path"
    );
}

#[test]
fn a_table_the_model_cannot_count_is_a_diagnostic_naming_the_binding() {
    let model = build("each-sources");
    let open: Vec<&spirolo_model::Diagnostic> = model
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "test_count_undecidable")
        .collect();
    assert_eq!(
        open.len(),
        1,
        "one runtime-decided table in the fixture: {open:?}"
    );
    assert_eq!(open[0].file.as_deref(), Some("src/tables.test.ts"));
    assert!(
        open[0].message.contains("`rows`") && open[0].message.contains("undecidable"),
        "names the binding and says why: {}",
        open[0].message
    );
    assert_eq!(open[0].severity, spirolo_model::Severity::Warning);
}

#[test]
fn a_loop_around_a_suite_multiplies_the_suite_not_each_case_in_it() {
    let model = build("each-sources");
    let rows = tables(&model, "src/tables.test.ts");
    let inherited = rows
        .iter()
        .find(|row| row["title"] == "inherits the suite table, not a second copy of it")
        .expect("the case under describe.each");
    assert!(inherited["each"].is_null());
    assert_eq!(inherited["loops"], serde_json::json!([]));
}

// ---------------------------------------------------------- Playwright specs are not paired

#[test]
fn a_spec_playwright_collects_is_not_a_pairing_candidate() {
    let model = build("playwright-scope");
    let unpaired: Vec<&str> = model
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "unpaired_test")
        .filter_map(|diagnostic| diagnostic.file.as_deref())
        .collect();
    assert!(
        !unpaired.contains(&"packages/e2e/tests/home.spec.ts"),
        "a spec under Playwright's testDir exercises the app, not a module: no warning; saw {unpaired:?}"
    );
    assert!(
        !model
            .pairings
            .iter()
            .any(|pairing| pairing.test_file == "packages/e2e/tests/home.spec.ts"),
        "and no pairing either"
    );
    assert!(
        unpaired.contains(&"packages/e2e/src/helper.spec.ts"),
        "a .spec outside every Playwright config's scope is still judged by name"
    );
}

// ---------------------------------------------------------- JSON configs become gates, never modules

#[test]
fn json_configs_are_gates_that_scope_the_way_their_tool_does_and_never_modules() {
    let model = build("json-gates");
    let sources: Vec<&str> = model
        .gates
        .iter()
        .map(|gate| gate.source.as_str())
        .collect();
    assert_eq!(
        sources,
        [
            "packages/a/package.json",
            "packages/b/jest.config.json",
            "packages/b/stryker.config.json",
            "stryker.conf.json",
        ]
    );
    assert!(
        model
            .modules
            .iter()
            .all(|module| std::path::Path::new(&module.path).extension() != Some("json".as_ref())),
        "a JSON file is read as data, never tabled as source: {:?}",
        model.modules.iter().map(|m| &m.path).collect::<Vec<_>>()
    );
    assert_eq!(
        test_files(&model),
        ["packages/a/src/x.unit.js", "packages/b/src/y.check.js"],
        "the package.json jest key and jest.config.json each scope their own package"
    );
    assert_eq!(
        gate(&model, "stryker.conf.json")
            .mutation_break
            .map(|b| b.to_string()),
        Some("60".to_owned())
    );
    assert_eq!(
        gate(&model, "packages/a/package.json").thresholds[0]
            .value
            .to_string(),
        "90"
    );
}

// ---------------------------------------------------------- mock targets resolve like imports

#[test]
fn a_module_mocks_target_resolves_through_the_same_resolver_as_an_import() {
    let model = build("mock-targets");
    let case = model
        .test_cases
        .iter()
        .find(|case| case.title.as_deref() == Some("runs"))
        .expect("the test");
    let resolved: Vec<(Option<&str>, Option<&str>)> = case
        .mocks
        .iter()
        .map(|mock| (mock.target.as_deref(), mock.resolved.as_deref()))
        .collect();
    assert_eq!(
        resolved,
        [
            (Some("./db"), Some("src/db.ts")),
            (Some("@/mail"), Some("src/mail.ts")),
            (Some("axios"), None),
            (Some("./missing"), None),
        ],
        "relative and tsconfig-aliased targets resolve; an external package and a missing file do not"
    );
    let unresolved: Vec<&spirolo_model::Diagnostic> = model
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "unresolved_mock")
        .collect();
    assert_eq!(
        unresolved.len(),
        1,
        "only the missing relative target: {unresolved:?}"
    );
    assert_eq!(unresolved[0].file.as_deref(), Some("src/app.test.ts"));
    assert!(unresolved[0].message.contains("./missing"));
}
