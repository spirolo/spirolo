//! What constant folding puts in the `Module` table: a value where it could be decided
//! across imports and re-exports, and an opaque row with a reason where it could not.
//! Assertions read the tables, never an internal struct.

use std::path::{Path, PathBuf};

use spirolo_model::{BuildOptions, Constant, Model, Module};

/// The fixture project of that name, as a model root.
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn build(name: &str) -> Model {
    Model::build(&fixture(name), &BuildOptions::default())
}

/// The one module row at `path`, panicking with the table's paths when it is missing.
fn module<'m>(model: &'m Model, path: &str) -> &'m Module {
    model
        .modules
        .iter()
        .find(|m| m.path == path)
        .unwrap_or_else(|| {
            let seen: Vec<&str> = model.modules.iter().map(|m| m.path.as_str()).collect();
            panic!("no module {path}; saw {seen:?}")
        })
}

/// The one constant named `name` in `module`, with the module's constants on a miss.
fn constant<'m>(module: &'m Module, name: &str) -> &'m Constant {
    module
        .constants
        .iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| {
            let seen: Vec<&str> = module.constants.iter().map(|c| c.name.as_str()).collect();
            panic!("no constant {name} in {}; saw {seen:?}", module.path)
        })
}

#[test]
fn a_string_literal_constant_folds_to_its_value() {
    let model = build("constants");
    let base = constant(module(&model, "src/config.ts"), "BASE");

    assert_eq!(base.value.as_deref(), Some("https://api.example.com"));
    assert!(!base.opaque);
    assert_eq!(base.reason, None);
}

#[test]
fn a_concatenation_folds_across_a_reexport_chain() {
    let model = build("constants");
    let url = constant(module(&model, "src/service.ts"), "urlApi");

    assert_eq!(
        url.value.as_deref(),
        Some("https://api.example.com/v1"),
        "BASE flows config -> shared (re-export) -> service (import) and concatenates"
    );
    assert!(!url.opaque);
}

#[test]
fn a_template_literal_folds_the_same_chain() {
    let model = build("constants");
    let url = constant(module(&model, "src/service.ts"), "urlV2");

    assert_eq!(url.value.as_deref(), Some("https://api.example.com/v2"));
    assert!(!url.opaque);
}

#[test]
fn a_constant_that_references_the_same_name_twice_folds_and_is_not_a_cycle() {
    let model = build("constants");
    let twice = constant(module(&model, "src/service.ts"), "twice");

    assert_eq!(
        twice.value.as_deref(),
        Some("https://api.example.com-https://api.example.com"),
        "referencing BASE twice is a DAG, not a cycle"
    );
    assert!(!twice.opaque);
    assert_eq!(twice.reason, None);
}

#[test]
fn an_unfoldable_constant_is_opaque_with_a_reason_and_never_a_guessed_value() {
    let model = build("constants");
    let dynamic = constant(module(&model, "src/service.ts"), "dynamic");

    assert_eq!(dynamic.value, None, "an opaque constant is never guessed");
    assert!(dynamic.opaque);
    assert!(
        dynamic
            .reason
            .as_deref()
            .is_some_and(|reason| !reason.is_empty()),
        "opaque constants carry a reason, got {:?}",
        dynamic.reason
    );
}

#[test]
fn an_env_read_stays_opaque_but_keeps_its_hole_as_an_env_template() {
    let model = build("constants");
    // `dynamic = process.env.API_URL + "/fallback"`: opaque with no folded value (the model
    // never reads the environment), but the `${API_URL}` hole is preserved so a rule holding
    // an assumed value can complete it.
    let dynamic = constant(module(&model, "src/service.ts"), "dynamic");
    assert_eq!(dynamic.value, None, "still never a guessed value");
    assert!(dynamic.opaque, "still opaque");
    assert_eq!(
        dynamic.env_template.as_deref(),
        Some("${API_URL}/fallback"),
        "the env hole is kept as a template"
    );

    // A constant opaque for any *other* reason carries no env template — only env holes do.
    let base = constant(module(&model, "src/service.ts"), "urlApi");
    assert_eq!(
        base.env_template, None,
        "a concrete constant has no env template"
    );
}

#[test]
fn snapshot_constants() {
    let model = build("constants");
    let constants: Vec<(&str, &Vec<Constant>)> = model
        .modules
        .iter()
        .filter(|module| !module.constants.is_empty())
        .map(|module| (module.path.as_str(), &module.constants))
        .collect();
    insta::assert_json_snapshot!("constants__chain", constants);
}
