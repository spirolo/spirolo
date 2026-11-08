//! `Model::build` against directories on disk: what the model *says about a project*.

use spirolo_model::{BuildOptions, Model};

#[test]
fn an_empty_directory_yields_empty_tables_and_no_diagnostics() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("empty-dir");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let model = Model::build(&dir, &BuildOptions::default());

    assert!(model.modules.is_empty(), "modules");
    assert!(model.test_cases.is_empty(), "test cases");
    assert!(model.pairings.is_empty(), "pairings");
    assert!(model.handlers.is_empty(), "handlers");
    assert!(model.gates.is_empty(), "gates");
    assert!(model.diagnostics.is_empty(), "diagnostics");
}

#[test]
fn a_handler_glob_that_does_not_compile_is_a_diagnostic_naming_it() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("bad-handler-glob");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let model = Model::build(
        &dir,
        &BuildOptions {
            handler_globs: vec!["src/ok/**".to_owned(), "src/[bad".to_owned()],
            handler_globs_source: Some(".spirolorc.jsonc".to_owned()),
            ..BuildOptions::default()
        },
    );

    let invalid: Vec<&spirolo_model::Diagnostic> = model
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "glob_invalid")
        .collect();
    assert_eq!(
        invalid.len(),
        1,
        "only the glob that does not compile: {invalid:?}"
    );
    assert_eq!(invalid[0].severity, spirolo_model::Severity::Warning);
    assert_eq!(
        invalid[0].file.as_deref(),
        Some(".spirolorc.jsonc"),
        "located on the file the caller named"
    );
    assert!(
        invalid[0].message.contains("src/[bad") && invalid[0].message.contains("handler"),
        "names the glob and where it came from: {}",
        invalid[0].message
    );
}

#[test]
fn a_packages_entry_modules_are_what_its_manifest_names_where_a_module_exists() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("package-entries");
    let _ = std::fs::remove_dir_all(&dir);
    for (path, contents) in [
        (
            "packages/kit/package.json",
            r#"{ "name": "@lab/kit", "main": "./src/index.ts", "types": "./src/index.ts",
                 "exports": { ".": { "import": "./src/index.ts" }, "./server": "./src/server.ts",
                              "./dist": "./dist/index.js" } }"#,
        ),
        ("packages/kit/src/index.ts", "export const a = 1\n"),
        ("packages/kit/src/server.ts", "export const b = 2\n"),
        (
            "packages/tool/package.json",
            r#"{ "name": "tool", "main": "index.js" }"#,
        ),
        ("packages/tool/index.js", "module.exports = {}\n"),
    ] {
        let full = dir.join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, contents).unwrap();
    }

    let model = Model::build(&dir, &BuildOptions::default());

    // `./dist/index.js` names no module the walk read, so it is not an entry.
    assert_eq!(
        model.package_entries,
        [
            "packages/kit/src/index.ts",
            "packages/kit/src/server.ts",
            "packages/tool/index.js",
        ]
    );
}

#[test]
fn every_module_carries_its_cyclomatic_complexity_and_a_file_that_fails_to_parse_carries_none() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("complexity-modules");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(
        dir.join("src/price.ts"),
        "export function price(qty: number, member?: boolean) {\n  \
           if (qty > 10 && member) return qty * 0.9\n  \
           return qty\n}\n",
    )
    .unwrap();
    std::fs::write(dir.join("src/broken.ts"), "export function (\n").unwrap();

    let model = Model::build(&dir, &BuildOptions::default());
    let complexity: Vec<(&str, u32)> = model
        .modules
        .iter()
        .map(|module| (module.path.as_str(), module.complexity))
        .collect();

    // price.ts: the top level, the function, the `if` and the `&&`.
    assert_eq!(complexity, [("src/broken.ts", 0), ("src/price.ts", 4)]);
}

#[test]
fn a_module_with_a_generated_marker_in_its_first_20_lines_is_flagged_generated() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("generated-modules");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    // The real src/eva-icons/icons/clipboard-outline.ts of akveo/react-native-ui-kitten.
    std::fs::write(
        dir.join("src/clipboard-outline.ts"),
        include_str!("fixtures/generated/clipboard-outline.ts"),
    )
    .unwrap();
    // Hand-written: the warning sits past line 20, so it is no generated header.
    let mut cart = "export const total = (items) => items.length > 0 && items[0]\n".to_owned();
    cart.push_str(&"// callers sort first\n".repeat(19));
    cart.push_str("// Do not edit lightly: callers rely on the order.\n");
    std::fs::write(dir.join("src/cart.ts"), cart).unwrap();

    let model = Model::build(&dir, &BuildOptions::default());
    let generated: Vec<(&str, bool)> = model
        .modules
        .iter()
        .map(|module| (module.path.as_str(), module.generated))
        .collect();

    assert_eq!(
        generated,
        [("src/cart.ts", false), ("src/clipboard-outline.ts", true)]
    );
}
