//! `untested-hotspot` on hand-built models: churn × complexity × weakness per PRD 0005, one
//! case per weakness branch, the churn fallback, the order, and the mutation-blindness guard.

use spirolo_model::{
    Churn, Export, ExportKind, IdentRef, Import, ImportKind, Language, Model, Module, Pairing,
    PairingReason, ParseStatus, Span, TestCase, TestKind,
};
use spirolo_rules::{hotspots, Config, Finding, Rule, UntestedHotspot};

/// A module at `path` with `complexity`, whose imports resolve to each of `targets`.
fn module(path: &str, complexity: u32, targets: &[&str]) -> Module {
    Module {
        path: path.to_owned(),
        language: Language::TypeScript,
        parse_status: ParseStatus::Parsed,
        imports: targets
            .iter()
            .map(|target| Import {
                specifier: format!("./{target}"),
                resolved: Some((*target).to_owned()),
                reason: None,
                kind: ImportKind::Static,
                span: Span::default(),
            })
            .collect(),
        exports: Vec::new(),
        constants: Vec::new(),
        mocks: Vec::new(),
        references: Vec::new(),
        spies: Vec::new(),
        loads: Vec::new(),
        complexity,
        generated: false,
    }
}

/// A collected vitest test in `file`, which makes `file` a test file.
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

fn model(modules: Vec<Module>, tests: &[&str]) -> Model {
    Model {
        modules,
        test_cases: tests.iter().map(|file| test_case(file)).collect(),
        ..Model::default()
    }
}

fn run(model: &Model) -> Vec<Finding> {
    UntestedHotspot.run(model, &Config::default())
}

/// The finding's evidence value for `key`.
fn evidence<'f>(finding: &'f Finding, key: &str) -> &'f str {
    finding.evidence.iter().find(|e| e.key == key).map_or_else(
        || panic!("no {key} evidence on {}", finding.location.file),
        |e| e.value.as_str(),
    )
}

#[test]
fn a_module_no_test_reaches_is_fully_weak_and_scores_churn_times_complexity() {
    let model = model(vec![module("src/a.ts", 7, &[])], &[]);

    let findings = run(&model);

    assert_eq!(findings.len(), 1);
    let finding = &findings[0];
    assert_eq!(finding.rule, "untested-hotspot");
    assert_eq!(finding.location.file, "src/a.ts");
    assert_eq!(evidence(finding, "churn"), "1");
    assert_eq!(evidence(finding, "complexity"), "7");
    assert_eq!(evidence(finding, "weakness"), "1.00");
    assert_eq!(evidence(finding, "score"), "7.00");
}

#[test]
fn churn_is_added_plus_deleted_plus_one_and_a_module_the_numstat_never_names_counts_one() {
    let mut model = model(
        vec![module("src/a.ts", 2, &[]), module("src/b.ts", 3, &[])],
        &[],
    );
    model.churn = Some(vec![Churn {
        path: "src/a.ts".to_owned(),
        added: 10,
        deleted: 4,
    }]);

    let findings = run(&model);

    let a = findings
        .iter()
        .find(|f| f.location.file == "src/a.ts")
        .unwrap();
    assert_eq!(evidence(a, "churn"), "15");
    assert_eq!(evidence(a, "score"), "30.00");
    let b = findings
        .iter()
        .find(|f| f.location.file == "src/b.ts")
        .unwrap();
    assert_eq!(evidence(b, "churn"), "1");
}

#[test]
fn only_non_test_js_ts_source_modules_are_candidates() {
    let mut json = module("package.json", 1, &[]);
    json.language = Language::Json;
    let model = model(
        vec![
            module("src/a.ts", 1, &[]),
            module("src/a.test.ts", 1, &[]),
            module("src/helpers.test.ts", 1, &[]),
            module("src/__tests__/fixture.ts", 1, &[]),
            module("src/types.d.ts", 1, &[]),
            module("vitest.config.ts", 1, &[]),
            module("eslint.config.mjs", 1, &[]),
            json,
        ],
        &["src/a.test.ts"],
    );

    let files: Vec<String> = run(&model).into_iter().map(|f| f.location.file).collect();

    assert_eq!(files, ["src/a.ts"]);
}

/// `module` exporting each of `names` with `kind`.
fn exporting(mut module: Module, names: &[&str], kind: ExportKind) -> Module {
    module.exports.extend(names.iter().map(|name| Export {
        name: (*name).to_owned(),
        kind,
        origin: None,
        resolved: None,
        span: Span::default(),
    }));
    module
}

/// `module` reading each of `names` as an identifier.
fn reading(mut module: Module, names: &[&str]) -> Module {
    module.references.extend(names.iter().map(|name| IdentRef {
        name: (*name).to_owned(),
        span: Span::default(),
    }));
    module
}

fn weakness_of(model: &Model, file: &str) -> String {
    let findings = run(model);
    let finding = findings
        .iter()
        .find(|f| f.location.file == file)
        .unwrap_or_else(|| panic!("no finding on {file}"));
    evidence(finding, "weakness").to_owned()
}

#[test]
fn a_reached_module_is_as_weak_as_the_share_of_its_named_exports_no_test_references() {
    let source = exporting(
        module("src/a.ts", 4, &[]),
        &["one", "two", "three", "four"],
        ExportKind::Named,
    );
    // Default and type exports are not named exports: they count on neither side.
    let source = exporting(source, &["default"], ExportKind::Default);
    let source = exporting(source, &["Shape"], ExportKind::Type);
    let test = reading(
        module("src/a.test.ts", 1, &["src/a.ts"]),
        &["one", "default", "Shape"],
    );
    let model = model(vec![source, test], &["src/a.test.ts"]);

    assert_eq!(weakness_of(&model, "src/a.ts"), "0.75");
    let findings = run(&model);
    assert_eq!(evidence(&findings[0], "score"), "3.00");
}

#[test]
fn a_reached_module_with_no_named_exports_is_half_weak() {
    let source = exporting(
        module("src/a.ts", 1, &[]),
        &["default"],
        ExportKind::Default,
    );
    let test = reading(module("src/a.test.ts", 1, &["src/a.ts"]), &["default"]);
    let model = model(vec![source, test], &["src/a.test.ts"]);

    assert_eq!(weakness_of(&model, "src/a.ts"), "0.50");
}

#[test]
fn a_module_paired_with_a_test_that_does_not_import_it_is_reached() {
    let source = exporting(
        module("src/a.ts", 1, &[]),
        &["one", "two"],
        ExportKind::Named,
    );
    let test = reading(module("src/a.test.ts", 1, &[]), &["one"]);
    let mut model = model(vec![source, test], &["src/a.test.ts"]);
    model.pairings = vec![Pairing {
        test_file: "src/a.test.ts".to_owned(),
        source_module: "src/a.ts".to_owned(),
        reason: PairingReason::SiblingName,
        ambiguous: false,
    }];

    assert_eq!(weakness_of(&model, "src/a.ts"), "0.50");
}

#[test]
fn findings_run_by_score_descending_then_path_and_a_zero_score_is_no_finding() {
    let mut unparsed = module("src/0-unparsed.ts", 0, &[]);
    unparsed.parse_status = ParseStatus::Failed;
    let model = model(
        vec![
            unparsed,
            module("src/a.ts", 2, &[]),
            module("src/b.ts", 9, &[]),
            module("src/c.ts", 2, &[]),
            module("src/d.ts", 10, &[]),
        ],
        &[],
    );

    let files: Vec<String> = run(&model).into_iter().map(|f| f.location.file).collect();

    assert_eq!(files, ["src/d.ts", "src/b.ts", "src/a.ts", "src/c.ts"]);
}

#[test]
fn with_no_churn_input_one_diagnostic_says_churn_counted_one() {
    let without = model(
        vec![module("src/a.ts", 1, &[]), module("src/b.ts", 1, &[])],
        &[],
    );
    let mut with = without.clone();
    with.churn = Some(Vec::new());

    let codes: Vec<String> = hotspots(&without)
        .diagnostics
        .into_iter()
        .map(|d| d.code)
        .collect();

    assert_eq!(codes, ["untested_hotspot_no_churn"]);
    assert!(hotspots(&with).diagnostics.is_empty());
}

#[test]
fn untested_hotspot_is_blind_to_mutation_data() {
    let source = exporting(
        module("src/a.ts", 3, &[]),
        &["one", "two"],
        ExportKind::Named,
    );
    let test = reading(module("src/a.test.ts", 1, &["src/a.ts"]), &["one"]);
    let without = model(
        vec![source, test, module("src/b.ts", 2, &[])],
        &["src/a.test.ts"],
    );
    let mut with = without.clone();
    // A module every mutant of which survives, and one whose mutants all die: were the
    // ranking reading them, `src/a.ts` would move up and `src/b.ts` down.
    with.mutants = ["Survived", "Killed"]
        .iter()
        .zip(["src/a.ts", "src/b.ts"])
        .enumerate()
        .map(|(id, (status, file))| spirolo_model::Mutants {
            file: file.to_owned(),
            span: Span::default(),
            id: id.to_string(),
            mutator: Some("ConditionalExpression".to_owned()),
            status: if *status == "Killed" {
                spirolo_model::MutantStatus::Killed
            } else {
                spirolo_model::MutantStatus::Survived
            },
            killed_by: vec!["0".to_owned()],
            covered_by: vec!["0".to_owned()],
            report: "reports/mutation/mutation.json".to_owned(),
        })
        .collect();
    with.mutation_tests = vec![spirolo_model::MutationTest {
        report: "reports/mutation/mutation.json".to_owned(),
        id: "0".to_owned(),
        file: "src/a.test.ts".to_owned(),
        name: "t".to_owned(),
        joined: None,
    }];

    assert_eq!(run(&with), run(&without));
    assert_eq!(hotspots(&with), hotspots(&without));
}

/// Whether a lone module at `path`, which no test reaches, gets a finding.
fn ranked(path: &str) -> bool {
    let model = model(vec![module(path, 5, &[])], &[]);
    run(&model).iter().any(|f| f.location.file == path)
}

// Non-product modules (#456): examples, docs, stories and tool rc/config files are not the
// product the suite should pin. One test per real row of round 1's draw.

#[test]
fn an_example_under_examples_is_not_ranked_watermelondb() {
    assert!(!ranked("examples/typescript/ts-example.ts"));
}

#[test]
fn an_example_under_examples_is_not_ranked_instantsearch() {
    assert!(!ranked("examples/react/getting-started/src/index.tsx"));
}

#[test]
fn an_example_under_examples_is_not_ranked_boardgame_io() {
    assert!(!ranked(
        "examples/react-web/src/simulator/example-others.js"
    ));
}

#[test]
fn an_example_nested_in_an_app_is_not_ranked_chakra_ui() {
    assert!(!ranked(
        "apps/compositions/src/examples/pagination-with-sibling-count.tsx"
    ));
}

#[test]
fn an_example_app_is_not_ranked_fumadocs() {
    assert!(!ranked("examples/asyncapi/app/layout.tsx"));
}

#[test]
fn a_docs_script_is_not_ranked_react_native_paper() {
    assert!(!ranked("docs/public/4.0/scripts/snack.js"));
}

#[test]
fn an_examples_dash_version_dir_counts_as_examples_react_day_picker() {
    assert!(!ranked("apps/website/examples-v9/Dropdown.tsx"));
}

#[test]
fn a_stories_file_is_not_ranked_grommet() {
    assert!(!ranked(
        "src/js/components/Tip/stories/typescript/Children.stories.tsx"
    ));
}

#[test]
fn a_story_file_is_not_ranked_mantine() {
    assert!(!ranked(
        "packages/@mantine/dates/src/components/DateInput/DateInput.story.tsx"
    ));
}

#[test]
fn a_tool_rc_file_is_not_ranked_expo() {
    assert!(!ranked("packages/expo-dev-client/.detoxrc.js"));
}

#[test]
fn a_dash_config_file_is_not_ranked_gatsby() {
    assert!(!ranked("deprecated-packages/gatsby-admin/gatsby-config.js"));
}

#[test]
fn a_dash_config_file_in_examples_is_not_ranked_gatsby() {
    assert!(!ranked("examples/using-asciidoc/gatsby-config.js"));
}

#[test]
fn an_example_app_under_example_is_not_ranked_react_native_paper() {
    assert!(!ranked("example/src/Examples/Dialogs/index.tsx"));
}

#[test]
fn an_integration_test_fixture_is_not_ranked_metro() {
    assert!(!ranked(
        "packages/metro/src/integration_tests/basic_bundle/require-context/subdir/c.js"
    ));
}

#[test]
fn a_benchmark_implementation_is_not_ranked_react_native_web() {
    assert!(!ranked(
        "packages/benchmarks/src/implementations/styleq/Box.js"
    ));
}

#[test]
fn a_runtime_tests_entry_is_not_ranked_reanimated() {
    assert!(!ranked(
        "apps/fabric-example/index.runtimeTests.worklets.js"
    ));
}

// Product modules keep their finding.

#[test]
fn a_production_component_no_test_imports_is_still_ranked_nextchat() {
    assert!(ranked("app/components/sidebar.tsx"));
}

#[test]
fn a_production_component_no_test_imports_is_still_ranked_decap_cms() {
    assert!(ranked(
        "packages/decap-cms-core/src/components/MediaLibrary/MediaLibraryModal.js"
    ));
}

#[test]
fn example_inside_a_segment_name_is_still_ranked() {
    assert!(ranked("src/exampleUtils.ts"));
}

// Generated modules (#456): no test could pin them and a hand edit is forbidden.

#[test]
fn a_module_the_model_flags_generated_is_not_ranked() {
    let mut icon = module("src/eva-icons/icons/clipboard-outline.ts", 1, &[]);
    icon.generated = true;
    let model = model(vec![icon, module("src/cart.ts", 1, &[])], &[]);

    let files: Vec<String> = run(&model).into_iter().map(|f| f.location.file).collect();

    assert_eq!(files, ["src/cart.ts"]);
}

#[test]
fn a_umd_or_minified_bundle_is_not_ranked() {
    // mobx's docs bundle, moved out of docs/ so only its name decides.
    assert!(!ranked("assets/javascripts/mobx.umd.js"));
    // A vendored, minified third-party SDK.
    assert!(!ranked("public/static/scandit-engine-sdk.min.js"));
}

// Well-pinned modules (#456): a reached module whose every named value export a reaching
// test references is at the weakness floor, and the rule has no evidence its tests are weak.

#[test]
fn a_reached_module_whose_value_exports_its_test_all_references_is_not_ranked() {
    let source = exporting(
        module("src/validate.ts", 9, &[]),
        &["required", "minLength"],
        ExportKind::Named,
    );
    let source = exporting(source, &["Validator"], ExportKind::Type);
    let test = reading(
        module("src/validate.spec.ts", 1, &["src/validate.ts"]),
        &["required", "minLength"],
    );
    let model = model(vec![source, test], &["src/validate.spec.ts"]);

    assert!(run(&model).is_empty());
}

#[test]
fn a_component_whose_one_named_export_its_dedicated_test_references_is_not_ranked_hyperdx() {
    let source = exporting(
        module(
            "packages/app/src/components/ViewTraceCalloutButton.tsx",
            9,
            &[],
        ),
        &["ViewTraceCalloutButton"],
        ExportKind::Named,
    );
    let test = reading(
        module(
            "packages/app/src/components/__tests__/ViewTraceCalloutButton.test.tsx",
            1,
            &["packages/app/src/components/ViewTraceCalloutButton.tsx"],
        ),
        &["ViewTraceCalloutButton"],
    );
    let model = model(
        vec![source, test],
        &["packages/app/src/components/__tests__/ViewTraceCalloutButton.test.tsx"],
    );

    assert!(run(&model).is_empty());
}

#[test]
fn a_reached_module_whose_test_references_one_of_three_value_exports_is_still_ranked() {
    let source = exporting(
        module("src/a.ts", 3, &[]),
        &["one", "two", "three"],
        ExportKind::Named,
    );
    let test = reading(module("src/a.test.ts", 1, &["src/a.ts"]), &["one"]);
    let model = model(vec![source, test], &["src/a.test.ts"]);

    assert_eq!(weakness_of(&model, "src/a.ts"), "0.67");
}

#[test]
fn the_explain_says_the_tests_touch_little_only_from_half_weak_up() {
    let weak = exporting(
        module("src/weak.ts", 3, &[]),
        &["one", "two", "three"],
        ExportKind::Named,
    );
    let pinned = exporting(
        module("src/pinned.ts", 3, &[]),
        &["four", "five", "six", "seven"],
        ExportKind::Named,
    );
    let test = reading(
        module("src/all.test.ts", 1, &["src/weak.ts", "src/pinned.ts"]),
        &["one", "four", "five", "six"],
    );
    let model = model(vec![weak, pinned, test], &["src/all.test.ts"]);
    let findings = run(&model);
    let explain = |file: &str| {
        findings
            .iter()
            .find(|f| f.location.file == file)
            .unwrap()
            .explain
            .clone()
    };

    assert!(explain("src/weak.ts").contains("its tests touch little of it (weakness 0.67"));
    let pinned = explain("src/pinned.ts");
    assert!(!pinned.contains("touch little"), "{pinned}");
    assert!(
        pinned.contains("its tests reference 75% of its named value exports (weakness 0.25"),
        "{pinned}"
    );
}

#[test]
fn a_reached_module_with_one_of_ten_value_exports_unreferenced_is_still_ranked_at_the_floor() {
    let names: Vec<String> = (1..=10).map(|i| format!("f{i}")).collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let source = exporting(module("src/many.ts", 10, &[]), &names, ExportKind::Named);
    let test = reading(module("src/many.test.ts", 1, &["src/many.ts"]), &names[..9]);
    let model = model(vec![source, test], &["src/many.test.ts"]);

    assert_eq!(weakness_of(&model, "src/many.ts"), "0.10");
}

#[test]
fn the_explain_names_the_exact_share_referenced_at_the_floor() {
    let names: Vec<String> = (1..=20).map(|i| format!("f{i}")).collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let source = exporting(module("src/many.ts", 10, &[]), &names, ExportKind::Named);
    let test = reading(
        module("src/many.test.ts", 1, &["src/many.ts"]),
        &names[..19],
    );
    let model = model(vec![source, test], &["src/many.test.ts"]);
    let explain = run(&model).remove(0).explain;

    assert!(
        explain.contains("its tests reference 95% of its named value exports (weakness 0.10"),
        "{explain}"
    );
}
