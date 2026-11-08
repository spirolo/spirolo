//! What the pairing step puts in the `Pairing` table: each test file tied to the source
//! it exercises, the reason recorded, and a test that pairs with nothing left as a
//! diagnostic. Assertions read the tables, never an internal struct.

use std::path::{Path, PathBuf};

use spirolo_model::{BuildOptions, Model, Pairing, PairingReason};

/// The fixture project of that name, as a model root.
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn build(name: &str) -> Model {
    Model::build(&fixture(name), &BuildOptions::default())
}

/// The one pairing whose test file is `test_file`, with the table printed on a miss.
fn pairing<'m>(model: &'m Model, test_file: &str) -> &'m Pairing {
    model
        .pairings
        .iter()
        .find(|p| p.test_file == test_file)
        .unwrap_or_else(|| {
            let seen: Vec<&str> = model
                .pairings
                .iter()
                .map(|p| p.test_file.as_str())
                .collect();
            panic!("no pairing for {test_file}; saw {seen:?}")
        })
}

// -------------------------------------------------------------- import

#[test]
fn a_test_that_imports_its_source_pairs_by_import() {
    let model = build("pair-import");
    let paired = pairing(&model, "src/service.test.ts");

    assert_eq!(paired.source_module, "src/service.ts");
    assert_eq!(paired.reason, PairingReason::DirectImport);
    assert!(!paired.ambiguous, "a single import is not ambiguous");
}

#[test]
fn two_imported_sources_pair_with_the_one_matching_the_test_name_and_record_it() {
    let model = build("pair-ambiguous");
    let paired = pairing(&model, "src/service.test.ts");

    assert_eq!(
        paired.source_module, "src/service.ts",
        "the name-matching import wins over the other non-test import"
    );
    assert_eq!(paired.reason, PairingReason::DirectImport);
    assert!(
        paired.ambiguous,
        "a tie broken by name is recorded as ambiguous"
    );
}

// -------------------------------------------------------------- sibling

#[test]
fn a_test_next_to_its_source_pairs_by_sibling_name() {
    let model = build("pair-sibling");
    let paired = pairing(&model, "src/widget.test.ts");

    assert_eq!(paired.source_module, "src/widget.ts");
    assert_eq!(paired.reason, PairingReason::SiblingName);
    assert!(!paired.ambiguous);
}

// -------------------------------------------------------------- tests-dir

#[test]
fn a_test_under_tests_pairs_with_the_source_one_directory_up() {
    let model = build("pair-tests-dir");
    let paired = pairing(&model, "src/__tests__/widget.test.ts");

    assert_eq!(paired.source_module, "src/widget.ts");
    assert_eq!(paired.reason, PairingReason::TestsDirectory);
    assert!(!paired.ambiguous);
}

// -------------------------------------------------------------- unpaired

#[test]
fn a_test_with_no_source_is_a_diagnostic_not_a_pairing() {
    let model = build("pair-orphan");

    assert!(
        model.pairings.is_empty(),
        "nothing should pair, saw {:?}",
        model.pairings
    );
    let diagnostic = model
        .diagnostics
        .iter()
        .find(|d| d.code == "unpaired_test")
        .unwrap_or_else(|| panic!("no unpaired_test diagnostic; saw {:?}", model.diagnostics));
    assert_eq!(diagnostic.file.as_deref(), Some("orphan.test.ts"));
}

// -------------------------------------------------------------- snapshots

fn snapshot_pairings(name: &str) {
    let model = build(name);
    insta::assert_json_snapshot!(format!("pairings__{name}"), model.pairings);
}

#[test]
fn snapshot_import() {
    snapshot_pairings("pair-import");
}

#[test]
fn snapshot_ambiguous() {
    snapshot_pairings("pair-ambiguous");
}

#[test]
fn snapshot_sibling() {
    snapshot_pairings("pair-sibling");
}

#[test]
fn snapshot_tests_dir() {
    snapshot_pairings("pair-tests-dir");
}
