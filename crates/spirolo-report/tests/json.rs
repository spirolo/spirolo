//! What the JSON surface promises consumers: a version, sorted keys, no clock.

use std::time::Duration;

use spirolo_model::{BuildOptions, Model};

/// A model of an empty directory — a fresh one under the target tree, never the crate
/// directory, whose `tests/fixtures/` is populated.
fn empty_model() -> Model {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("empty-model");
    std::fs::create_dir_all(&dir).unwrap();
    Model::build(&dir, &BuildOptions::default())
}

#[test]
fn json_carries_a_schema_version() {
    let json: serde_json::Value =
        serde_json::from_str(&spirolo_report::to_json(&empty_model())).unwrap();

    assert_eq!(
        json["schema_version"],
        serde_json::json!(spirolo_report::SCHEMA_VERSION)
    );
}

#[test]
fn json_declares_every_table_even_when_empty() {
    let json: serde_json::Value =
        serde_json::from_str(&spirolo_report::to_json(&empty_model())).unwrap();
    let keys: Vec<&str> = json
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();

    assert_eq!(
        keys,
        [
            "coverage",
            "diagnostics",
            "gates",
            "handlers",
            "modules",
            "mutants",
            "mutation_reports",
            "mutation_tests",
            "pairings",
            "runs",
            "saga_walks",
            "schema_version",
            "test_cases",
            "test_files",
        ]
    );
}

#[test]
fn json_keys_are_sorted_at_every_depth() {
    let json: serde_json::Value =
        serde_json::from_str(&spirolo_report::to_json(&empty_model())).unwrap();

    assert_sorted(&json);
}

fn assert_sorted(value: &serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            let keys: Vec<&String> = map.keys().collect();
            let mut sorted = keys.clone();
            sorted.sort();
            assert_eq!(keys, sorted, "object keys out of order: {keys:?}");
            for child in map.values() {
                assert_sorted(child);
            }
        }
        serde_json::Value::Array(items) => items.iter().for_each(assert_sorted),
        _ => {}
    }
}

#[test]
fn json_holds_no_timestamp_and_no_absolute_path() {
    let json = spirolo_report::to_json(&empty_model());

    for banned in ["timestamp", "generated_at", "elapsed", "duration", "root"] {
        assert!(
            !json.contains(banned),
            "{banned} leaked into the JSON: {json}"
        );
    }
}

#[test]
fn two_builds_of_the_same_tree_produce_byte_equal_json() {
    // A populated project, with the recognisers the binary wires in: several files go
    // through the parallel phase, a test pairs by import, one is an orphan and one file
    // does not parse, so every table and the diagnostics carry rows whose order the pool
    // could have scrambled.
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/populated");
    let recognisers = spirolo_frameworks::recognisers();
    let build = || Model::build_with(&fixture, &BuildOptions::default(), &recognisers);

    let first = build();
    assert!(!first.modules.is_empty(), "modules");
    assert!(!first.test_cases.is_empty(), "test cases");
    assert!(!first.pairings.is_empty(), "pairings");
    assert!(!first.gates.is_empty(), "gates");
    assert!(!first.diagnostics.is_empty(), "diagnostics");

    let first = spirolo_report::to_json(&first);
    let second = spirolo_report::to_json(&build());

    assert_eq!(first.as_bytes(), second.as_bytes());
}

#[test]
fn the_summary_line_counts_every_table_and_ends_with_the_timing() {
    let line = spirolo_report::summary_line(&empty_model(), Duration::from_millis(12));

    assert_eq!(
        line,
        "0 files · 0 test files · 0 tests · 0 expects · 0 handlers · 0 configs · 12ms"
    );
}

#[test]
fn the_agent_view_of_an_empty_model_is_empty() {
    assert_eq!(spirolo_report::render_agent(&empty_model()), "");
}

#[test]
fn the_human_view_of_an_empty_model_is_the_summary_line() {
    let human = spirolo_report::render_human(&empty_model(), Duration::from_millis(12));

    assert_eq!(
        human,
        "0 files · 0 test files · 0 tests · 0 expects · 0 handlers · 0 configs · 12ms\n"
    );
}

#[test]
fn the_text_views_spell_enums_exactly_as_the_json_does() {
    let mut model = empty_model();
    model.modules.push(spirolo_model::Module {
        path: "src/app.ts".to_owned(),
        language: spirolo_model::Language::TypeScript,
        parse_status: spirolo_model::ParseStatus::Parsed,
        imports: Vec::new(),
        exports: Vec::new(),
        constants: Vec::new(),
        mocks: Vec::new(),
        references: Vec::new(),
        spies: Vec::new(),
        loads: Vec::new(),
        complexity: 1,
        generated: false,
    });
    model.diagnostics.push(spirolo_model::Diagnostic {
        code: "parse_failed".to_owned(),
        severity: spirolo_model::Severity::Warning,
        file: Some("src/app.ts".to_owned()),
        span: None,
        message: "could not decide".to_owned(),
    });

    let json = spirolo_report::to_json(&model);
    assert!(json.contains("\"typescript\""), "{json}");

    let agent = spirolo_report::render_agent(&model);
    assert_eq!(
        agent,
        "modules\tsrc/app.ts\ttypescript\t0 imports\t0 exports\n\
         diagnostic\twarning\tparse_failed\tsrc/app.ts\tcould not decide\n"
    );
}
