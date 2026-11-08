//! What the walk and the parse put in the `Module` table, fixture project by fixture
//! project. Every assertion reads the table, never an internal struct.

use std::path::{Path, PathBuf};

use spirolo_model::{BuildOptions, Language, Model, ParseStatus, Severity};

/// The fixture project of that name, as a model root.
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn build(name: &str) -> Model {
    Model::build(&fixture(name), &BuildOptions::default())
}

/// A fresh, empty project directory under the target tree, named after its test.
fn scratch_project(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    dir
}

fn paths(model: &Model) -> Vec<&str> {
    model.modules.iter().map(|m| m.path.as_str()).collect()
}

/// The `Module` table of one fixture project, as the JSON surface will carry it.
fn snapshot_modules(name: &str) {
    let model = build(name);
    insta::assert_json_snapshot!(format!("modules__{name}"), model.modules);
}

// ------------------------------------------------------------ path, language, status

#[test]
fn a_vitest_project_lists_every_typescript_file_it_owns() {
    let model = build("vitest-basic");

    assert_eq!(
        paths(&model),
        ["src/sum.test.ts", "src/sum.ts", "vitest.config.ts"]
    );
    assert!(model
        .modules
        .iter()
        .all(|m| m.language == Language::TypeScript && m.parse_status == ParseStatus::Parsed));
    assert!(model.diagnostics.is_empty(), "{:?}", model.diagnostics);
}

#[test]
fn a_jest_project_lists_every_javascript_file_it_owns() {
    let model = build("jest-basic");

    assert_eq!(
        paths(&model),
        [
            "jest.config.js",
            "src/__tests__/greet.test.js",
            "src/greet.js"
        ]
    );
    assert!(model
        .modules
        .iter()
        .all(|m| m.language == Language::JavaScript && m.parse_status == ParseStatus::Parsed));
    assert!(model.diagnostics.is_empty(), "{:?}", model.diagnostics);
}

#[test]
fn every_dialect_extension_is_read_as_its_own_language() {
    let model = build("esm-cjs-mix");

    let seen: Vec<(&str, Language, ParseStatus)> = model
        .modules
        .iter()
        .map(|m| (m.path.as_str(), m.language, m.parse_status))
        .collect();
    assert_eq!(
        seen,
        [
            ("lib/config.cts", Language::TypeScript, ParseStatus::Parsed),
            ("lib/legacy.cjs", Language::JavaScript, ParseStatus::Parsed),
            ("lib/modern.mjs", Language::JavaScript, ParseStatus::Parsed),
            ("lib/plain.js", Language::JavaScript, ParseStatus::Parsed),
            ("lib/types.mts", Language::TypeScript, ParseStatus::Parsed),
            ("ui/widget.jsx", Language::Jsx, ParseStatus::Parsed),
        ]
    );
    assert!(model.diagnostics.is_empty(), "{:?}", model.diagnostics);
}

#[test]
fn tsx_files_are_read_as_tsx_and_parse_with_their_markup() {
    let model = build("tsx");

    assert_eq!(paths(&model), ["src/Button.test.tsx", "src/Button.tsx"]);
    assert!(model
        .modules
        .iter()
        .all(|m| m.language == Language::Tsx && m.parse_status == ParseStatus::Parsed));
    assert!(model.diagnostics.is_empty(), "{:?}", model.diagnostics);
}

// ------------------------------------------------------------------- failures

#[test]
fn a_broken_file_yields_one_diagnostic_and_the_rest_parse() {
    let model = build("broken-file");

    assert_eq!(paths(&model), ["src/broken.ts", "src/ok.ts"]);
    let statuses: Vec<ParseStatus> = model.modules.iter().map(|m| m.parse_status).collect();
    assert_ne!(
        statuses[0],
        ParseStatus::Parsed,
        "the broken file is not trusted"
    );
    assert_eq!(statuses[1], ParseStatus::Parsed, "N-1 files still parse");

    assert_eq!(model.diagnostics.len(), 1, "{:?}", model.diagnostics);
    let diagnostic = &model.diagnostics[0];
    assert_eq!(diagnostic.code, "parse_failed");
    assert_eq!(diagnostic.severity, Severity::Error);
    assert_eq!(diagnostic.file.as_deref(), Some("src/broken.ts"));
    let span = diagnostic.span.expect("the parser says where");
    assert!(span.line >= 1 && span.column >= 1, "{span:?}");
    assert!(!diagnostic.message.ends_with('.'), "{}", diagnostic.message);
}

#[test]
fn a_file_that_is_not_utf8_gets_a_failed_row_and_a_diagnostic_not_a_panic() {
    let project = scratch_project("not-utf8");
    std::fs::write(project.join("src/ok.ts"), "export const ok = 1;\n").unwrap();
    std::fs::write(
        project.join("src/latin1.ts"),
        b"export const value = \"\xe9\xff\";\n",
    )
    .unwrap();

    let model = Model::build(&project, &BuildOptions::default());

    assert_eq!(paths(&model), ["src/latin1.ts", "src/ok.ts"]);
    assert_eq!(model.modules[0].parse_status, ParseStatus::Failed);
    assert_eq!(model.modules[1].parse_status, ParseStatus::Parsed);
    assert_eq!(model.diagnostics.len(), 1, "{:?}", model.diagnostics);
    assert_eq!(model.diagnostics[0].code, "not_utf8");
    assert_eq!(model.diagnostics[0].file.as_deref(), Some("src/latin1.ts"));
    assert!(
        model.diagnostics[0].message.contains("byte 22"),
        "{}",
        model.diagnostics[0].message
    );
}

// ----------------------------------------------------------------- what is skipped

#[test]
fn node_modules_dist_coverage_and_gitignored_paths_never_appear() {
    let model = build("ignored");

    assert_eq!(paths(&model), ["src/app.ts"]);
    assert!(model.diagnostics.is_empty(), "{:?}", model.diagnostics);
}

#[test]
fn turning_gitignore_off_reveals_the_ignored_file_but_not_the_skipped_directories() {
    let model = Model::build(
        &fixture("ignored"),
        &BuildOptions {
            respect_gitignore: false,
            ..BuildOptions::default()
        },
    );

    assert_eq!(paths(&model), ["src/app.ts", "src/generated.ts"]);
}

// ---------------------------------------------------------------------- CI workflows

#[test]
fn a_github_workflow_becomes_a_ci_gate_carrying_its_run_steps() {
    use spirolo_model::GateTool;

    let project = scratch_project("ci-workflow");
    std::fs::create_dir_all(project.join(".github/workflows")).unwrap();
    std::fs::write(
        project.join(".github/workflows/ci.yml"),
        "name: CI\n\
         on: [push]\n\
         jobs:\n\
         \x20 test:\n\
         \x20   runs-on: ubuntu-latest\n\
         \x20   steps:\n\
         \x20     - uses: actions/checkout@v4\n\
         \x20     - run: pnpm vitest run --coverage\n",
    )
    .unwrap();
    // The workflow YAML is read as a gate, never as a module.
    std::fs::write(project.join("src/app.ts"), "export const x = 1;\n").unwrap();

    let model = Model::build(&project, &BuildOptions::default());

    let ci: Vec<&spirolo_model::Gate> = model
        .gates
        .iter()
        .filter(|g| g.tool == GateTool::Ci)
        .collect();
    assert_eq!(ci.len(), 1, "one workflow, one CI gate: {:?}", model.gates);
    assert_eq!(ci[0].source, ".github/workflows/ci.yml");
    assert_eq!(ci[0].steps, ["pnpm vitest run --coverage"]);
    // The workflow file is not a module.
    assert!(
        !paths(&model).contains(&".github/workflows/ci.yml"),
        "the workflow is read as a gate, not parsed as a module: {:?}",
        paths(&model)
    );
}

// ---------------------------------------------------------------------- parallel

#[test]
fn two_hundred_generated_files_come_back_complete_and_in_path_order() {
    let project = scratch_project("two-hundred");
    for i in 0..200 {
        let dir = project.join("src").join(format!("mod{:02}", i % 20));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(format!("file{i:03}.ts")),
            format!("export const value{i} = {i};\n"),
        )
        .unwrap();
    }

    let model = Model::build(&project, &BuildOptions::default());

    assert_eq!(model.modules.len(), 200);
    assert!(model.diagnostics.is_empty(), "{:?}", model.diagnostics);
    assert!(model
        .modules
        .iter()
        .all(|m| m.parse_status == ParseStatus::Parsed));
    let mut sorted = paths(&model);
    sorted.sort_unstable();
    assert_eq!(
        paths(&model),
        sorted,
        "merge order is path order, not pool order"
    );
    assert_eq!(model.modules[0].path, "src/mod00/file000.ts");
    assert_eq!(model.modules[199].path, "src/mod19/file199.ts");
}

// --------------------------------------------------------------------- snapshots

#[test]
fn snapshot_vitest_basic() {
    snapshot_modules("vitest-basic");
}

#[test]
fn snapshot_jest_basic() {
    snapshot_modules("jest-basic");
}

#[test]
fn snapshot_tsx() {
    snapshot_modules("tsx");
}

#[test]
fn snapshot_esm_cjs_mix() {
    snapshot_modules("esm-cjs-mix");
}

#[test]
fn snapshot_broken_file() {
    snapshot_modules("broken-file");
}

#[test]
fn snapshot_ignored() {
    snapshot_modules("ignored");
}
