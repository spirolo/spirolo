//! `orphan-test` on the shapes round 2 of its visit read as false (#372), each a fixture cut
//! from the corpus file it came from (provenance in `fixtures/orphan-test-shapes.json`), and
//! the one real orphan the round found, which must stay a finding through every change.
//!
//! Each fixture is copied to its own directory under the target tree and run through the
//! real binary, so the walk, the model's scoping and the rule are all on the path.

use std::path::{Path, PathBuf};

use assert_cmd::Command;

/// A fresh copy of the fixture `name` under the target tree, prefixed so it never shares a
/// directory with another rule's fixture.
fn fixture(name: &str) -> PathBuf {
    fixture_as(name, name)
}

/// A fresh copy of the fixture `name` in a directory of its own named after `dir`, for a
/// test that changes its copy.
fn fixture_as(name: &str, dir: &str) -> PathBuf {
    let from = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let to = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("orphan-shapes-{dir}"));
    let _ = std::fs::remove_dir_all(&to);
    copy(&from, &to);
    to
}

fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// The `orphan-test` findings of `check --all`, as `(kind, file, anchor, outcome)`.
fn findings(root: &Path) -> Vec<(String, String, String, String)> {
    let output = Command::cargo_bin("spirolo")
        .unwrap()
        .args([
            "check",
            "--all",
            "--format",
            "json",
            "--rule",
            "orphan-test",
        ])
        .arg(root)
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    json["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            let kind = f["evidence"]
                .as_array()
                .unwrap()
                .iter()
                .find(|e| e["key"] == "kind")
                .map_or("", |e| e["value"].as_str().unwrap());
            let anchor = f["id"].as_str().unwrap().rsplit(' ').next().unwrap();
            (
                kind.to_owned(),
                f["file"].as_str().unwrap().to_owned(),
                anchor.to_owned(),
                f["outcome"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

/// The files of the never-collected findings.
fn never_collected(root: &Path) -> Vec<String> {
    findings(root)
        .into_iter()
        .filter(|(kind, ..)| kind == "never-collected")
        .map(|(_, file, ..)| file)
        .collect()
}

#[test]
fn an_example_test_the_root_jest_config_ignores_stays_a_never_collected_finding() {
    // boardgame.io row 15, the real orphan: `examples/` is in the root jest
    // `testPathIgnorePatterns`, the example package runs no test runner, nothing imports it.
    let root = fixture("orphan-real-boardgame");
    assert_eq!(
        never_collected(&root),
        ["examples/react-web/src/app.test.js"]
    );
}

#[test]
fn a_spec_the_cypress_e2e_spec_pattern_collects_is_not_never_collected() {
    // decap-cms: `e2e.specPattern: 'cypress/e2e/*spec*.js'` in the root cypress.config.ts.
    let root = fixture("orphan-shape-cypress-decap");
    assert_eq!(never_collected(&root), Vec::<String>::new());
}

#[test]
fn a_spec_a_cypress_config_in_a_subdirectory_collects_relative_to_it_is_not_never_collected() {
    // react-admin: cypress/cypress.config.js, `e2e.specPattern: 'e2e/**/*.cy.{js,jsx,ts,tsx}'`.
    let root = fixture("orphan-shape-cypress-subdir");
    assert_eq!(never_collected(&root), Vec::<String>::new());
    // gatsby: e2e-tests/development-runtime/cypress.config.ts, `cypress/integration/**/*.{js,ts}`.
    let root = fixture("orphan-shape-cypress-gatsby");
    assert_eq!(never_collected(&root), Vec::<String>::new());
}

#[test]
fn a_spec_the_cypress_component_default_collects_is_not_never_collected() {
    // nivo: cypress/cypress.config.ts declares `component` with no specPattern, so the
    // default `**/*.cy.{js,jsx,ts,tsx}` collects cypress/src/components/boxplot/BoxPlot.cy.tsx.
    let root = fixture("orphan-shape-cypress-component");
    assert_eq!(never_collected(&root), Vec::<String>::new());
}

#[test]
fn a_spec_a_playwright_regexp_test_match_collects_is_not_never_collected() {
    // carbon: playwright.config.js, `testDir: path.join(__dirname, 'e2e')` and
    // `testMatch: /.*-test(.avt|.vrt)?.e2e\.m?js$/`.
    let root = fixture("orphan-shape-playwright-regex");
    assert_eq!(never_collected(&root), Vec::<String>::new());
}

#[test]
fn a_test_a_second_jest_config_a_package_script_selects_collects_is_not_never_collected() {
    // hyperdx packages/api: `ci:int` runs `jest --config jest.int.config.js`, whose testMatch
    // `**/__tests__/**/*.int.test.ts?(x)` collects what the default jest.config.js (rootDir
    // `./src`) ignores with `.*\.int\.test\.tsx?$`.
    let root = fixture("orphan-shape-jest-script-config");
    assert_eq!(never_collected(&root), Vec::<String>::new());
}

#[test]
fn a_test_a_package_local_jest_config_outside_the_workspace_collects_is_not_never_collected() {
    // expo docs/: no workspace member, but its own package.json runs jest from its `test`
    // script and docs/jest.config.js sets testMatch `**/*.test.(js|ts|tsx)`.
    let root = fixture("orphan-shape-jest-outside-workspace");
    assert_eq!(never_collected(&root), Vec::<String>::new());
}

#[test]
fn a_type_test_tsc_checks_from_a_type_check_script_is_not_never_collected() {
    // orpc: the root `type:check` script ends in `tsc`, and the root tsconfig.json includes
    // `*/*/src/**/*.test-d.ts`, though packages/server's own tsconfig excludes it.
    let root = fixture("orphan-shape-typecheck-tsc");
    assert_eq!(never_collected(&root), Vec::<String>::new());
}

#[test]
fn a_type_test_vitest_typecheck_collects_is_not_never_collected() {
    // nextra packages/nextra: `"test": "vitest --typecheck"`.
    let root = fixture("orphan-shape-typecheck-vitest");
    assert_eq!(never_collected(&root), Vec::<String>::new());
}

#[test]
fn a_spec_under_a_cypress_config_whose_spec_pattern_is_computed_is_not_reported() {
    // No corpus config computes its specPattern, so this one is planted: the reader cannot
    // say what it collects, and the rule says nothing about the files under it.
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("orphan-shapes-cypress-computed");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("cypress/e2e")).unwrap();
    std::fs::write(
        root.join("cypress.config.ts"),
        "import { defineConfig } from 'cypress'\nconst specs = ['cypress/e2e/**/*_spec.js']\n\
         export default defineConfig({ e2e: { specPattern: specs } })\n",
    )
    .unwrap();
    std::fs::write(
        root.join("cypress/e2e/login_spec.js"),
        "describe('login', () => { it('logs in', () => { cy.visit('/') }) })\n",
    )
    .unwrap();
    assert_eq!(never_collected(&root), Vec::<String>::new());
}

#[test]
fn a_shared_test_module_a_collected_test_reaches_through_an_index_is_not_never_collected() {
    // rjsf packages/utils: test/schema/findSelectedOptionInXxxOfTest.ts registers its tests
    // inside an exported function; test/schema/index.ts re-exports it and the collected
    // test/schema.test.ts calls it.
    let root = fixture("orphan-shape-shared-module");
    assert_eq!(never_collected(&root), Vec::<String>::new());
}

#[test]
fn a_shared_test_module_a_collected_test_reaches_through_a_jest_module_name_mapper_is_not_never_collected(
) {
    // mantine: @mantine/schedule's YearView.test.tsx imports `@mantine-tests/core`, whose
    // entries name only unbuilt cjs/ and esm/; the root jest.config.ts maps
    // `@mantine-tests/(.*)` to `<rootDir>/packages/@mantine-tests/$1/src`, whose index.ts
    // re-exports it-supports-system-props.tsx, which calls itSupportsFontsProps.
    let root = fixture("orphan-shape-module-name-mapper");
    assert_eq!(never_collected(&root), Vec::<String>::new());
}

#[test]
fn a_shared_test_module_only_an_uncollected_file_reaches_stays_never_collected() {
    // The same rjsf files with the importer renamed out of every include: nothing collected
    // loads the shared module, so its tests never run.
    let root = fixture_as(
        "orphan-shape-shared-module",
        "shared-module-uncollected-importer",
    );
    std::fs::rename(
        root.join("packages/utils/test/schema.test.ts"),
        root.join("packages/utils/test/schema.ts"),
    )
    .unwrap();
    assert_eq!(
        never_collected(&root),
        ["packages/utils/test/schema/findSelectedOptionInXxxOfTest.ts"]
    );
}

#[test]
fn a_vitest_reporter_that_iterates_test_objects_is_not_a_test_file() {
    // mui test/regressions/a11y/a11yReporter.ts: `for (const test of walkTests(mod))` then
    // `test.meta()` reads a reporter's TestCase object; it registers no test.
    let root = fixture("orphan-shape-reporter");
    assert_eq!(never_collected(&root), Vec::<String>::new());
}

#[test]
fn a_minified_bundle_that_declares_its_own_it_and_test_is_not_a_test_file() {
    // The shape of boardgame.io's docs/documentation/snippets/example-3.1fa4f5db.js, a
    // parcel bundle (436 KB, too large to carry) whose minifier named locals `it` and `test`:
    // `function it(e){…}` declared in a function body and at the top level, `var test=`
    // bound to a function, each then called.
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("orphan-shapes-minified-bundle");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("docs/snippets")).unwrap();
    std::fs::write(
        root.join("docs/snippets/example-3.1fa4f5db.js"),
        "parcelRequire=function(e){function it(e){var t=e.alternate;return t}function at(e){return it(e)}return at(e)}({});\
         function it(e,t,n){const r=e.slice();return r[4]=t[n],r}var test=function(e){return it(e,[],0)};test([1]);it([],[],0);\n",
    )
    .unwrap();
    assert_eq!(never_collected(&root), Vec::<String>::new());
}

#[test]
fn a_codemod_test_fixture_is_not_never_collected() {
    // react-admin packages/ra-core/codemods: __tests__/replace-Admin-history.spec.ts calls
    // jscodeshift's defineTest, which reads __testfixtures__/*.input.tsx as text; the input
    // holds describe/it but is never meant to run.
    let root = fixture("orphan-shape-testfixtures");
    assert_eq!(never_collected(&root), Vec::<String>::new());
}

/// The source-gone findings, as `(file, specifier, outcome)`.
fn source_gone(root: &Path) -> Vec<(String, String, String)> {
    findings(root)
        .into_iter()
        .filter(|(kind, ..)| kind == "source-gone")
        .map(|(_, file, anchor, outcome)| (file, anchor, outcome))
        .collect()
}

#[test]
fn an_import_of_the_build_output_package_json_main_names_is_not_source_gone() {
    // carbon packages/react: `main: lib/index.js`, and __tests__/ssr-test.js requires `../lib`.
    let root = fixture("orphan-shape-build-main");
    assert_eq!(
        source_gone(&root),
        [(
            "packages/react/__tests__/ssr-test.js".to_owned(),
            "../lib#0".to_owned(),
            "could_not_decide".to_owned()
        )]
    );
}

#[test]
fn an_import_of_a_build_scripts_out_dir_is_undecided_not_source_gone() {
    // gatsby packages/gatsby-source-wordpress: `build` runs `babel src --out-dir dist`, and
    // __tests__/process-node.test.js imports `../dist/store`.
    let root = fixture("orphan-shape-build-out-dir");
    let gone = source_gone(&root);
    assert!(
        gone.iter().any(
            |(_, anchor, outcome)| anchor == "../dist/store#0" && outcome == "could_not_decide"
        ),
        "{gone:?}"
    );
    assert!(
        gone.iter().all(|(_, _, outcome)| outcome != "finding"),
        "{gone:?}"
    );
}

#[test]
fn an_import_a_react_native_platform_suffix_resolves_is_not_source_gone() {
    // expo packages/jest-expo: tests/__tests__/jest-test.js requires `../default-extension`,
    // and only default-extension.ios.js / .android.js / .web.js exist.
    let root = fixture("orphan-shape-platform-extension");
    assert_eq!(source_gone(&root), []);
}

#[test]
fn an_import_of_a_deleted_file_in_a_source_directory_a_package_json_entry_names_stays_source_gone()
{
    // Planted on the carbon cut: `main` points into `src/`, which the checkout has, and a
    // test requires `../src/gone`, which it does not: the file was deleted, not unbuilt.
    let root = fixture_as("orphan-shape-build-main", "build-main-src-entry");
    let package = root.join("packages/react");
    let manifest = std::fs::read_to_string(package.join("package.json")).unwrap();
    std::fs::write(
        package.join("package.json"),
        manifest.replace(r#""main": "lib/index.js""#, r#""main": "src/index.js""#),
    )
    .unwrap();
    std::fs::create_dir_all(package.join("src")).unwrap();
    std::fs::write(package.join("src/index.js"), "module.exports = {};\n").unwrap();
    std::fs::write(
        package.join("__tests__/gone-test.js"),
        "const gone = require('../src/gone');\n\ntest('gone', () => {\n  expect(gone).toBeDefined();\n});\n",
    )
    .unwrap();
    let gone = source_gone(&root);
    assert!(
        gone.iter().any(
            |(file, anchor, outcome)| file == "packages/react/__tests__/gone-test.js"
                && anchor == "../src/gone#0"
                && outcome == "finding"
        ),
        "{gone:?}"
    );
}

#[test]
fn an_import_of_a_deleted_file_in_a_committed_lib_directory_stays_source_gone() {
    // Planted on the carbon cut: `lib/` is checked in, not gitignored, so it is source, and a
    // test requiring `../lib/gone`, which it does not have, imports a deleted file.
    let root = fixture_as("orphan-shape-build-main", "build-main-committed-lib");
    let package = root.join("packages/react");
    std::fs::create_dir_all(package.join("lib")).unwrap();
    std::fs::write(package.join("lib/index.js"), "module.exports = {};\n").unwrap();
    std::fs::write(
        package.join("__tests__/gone-test.js"),
        "const gone = require('../lib/gone');\n\ntest('gone', () => {\n  expect(gone).toBeDefined();\n});\n",
    )
    .unwrap();
    assert_eq!(
        source_gone(&root),
        [(
            "packages/react/__tests__/gone-test.js".to_owned(),
            "../lib/gone#0".to_owned(),
            "finding".to_owned()
        )]
    );
}

#[test]
fn an_import_of_a_gitignored_dist_directory_is_undecided_not_source_gone() {
    // Planted on the carbon cut: no entry or script names `dist/`, but the root .gitignore
    // ignores it with nivo's `/packages/*/dist`, so `../dist/gone` is unbuilt output.
    let root = fixture_as("orphan-shape-build-main", "build-main-gitignored-dist");
    std::fs::write(
        root.join(".gitignore"),
        "node_modules\n\n/packages/*/dist\n",
    )
    .unwrap();
    let package = root.join("packages/react");
    std::fs::create_dir_all(package.join("lib")).unwrap();
    std::fs::write(package.join("lib/index.js"), "module.exports = {};\n").unwrap();
    std::fs::write(
        package.join("__tests__/dist-test.js"),
        "const built = require('../dist/gone');\n\ntest('built', () => {\n  expect(built).toBeDefined();\n});\n",
    )
    .unwrap();
    assert_eq!(
        source_gone(&root),
        [(
            "packages/react/__tests__/dist-test.js".to_owned(),
            "../dist/gone#0".to_owned(),
            "could_not_decide".to_owned()
        )]
    );
}
