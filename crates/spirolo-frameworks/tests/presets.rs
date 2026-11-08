//! What the model says about a config that inherits from a preset without `mergeConfig`:
//! the bare re-exports a monorepo writes (`export default baseConfig`,
//! `export { default } from`, `defineConfig(preset)`, a named import, a relative path) are
//! followed to the preset and read whole, and the shapes the reader cannot follow (a factory
//! call, an unresolvable specifier, a function body) leave the gate opaque *and say why*.
//!
//! One fixture project (`preset-reexports`), one package per shape; every assertion reads
//! the `Gate` table with the recognisers wired in as the binary wires them.

use std::path::Path;

use spirolo_frameworks::recognisers;
use spirolo_model::{BuildOptions, Gate, Invariant, Model};

fn build() -> Model {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/preset-reexports");
    Model::build_with(&root, &BuildOptions::default(), &recognisers())
}

fn gate<'a>(model: &'a Model, package: &str) -> &'a Gate {
    let source = format!("packages/{package}/vitest.config.mts");
    model
        .gates
        .iter()
        .find(|gate| gate.source == source)
        .unwrap_or_else(|| panic!("a gate at {source}"))
}

const BASE: &str = "packages/config/base.mjs";
const REACT: &str = "packages/config/react.mjs";

// ---------------------------------------------------------------- shapes the reader follows

#[test]
fn a_bare_default_re_export_is_read_from_the_preset() {
    let model = build();
    let bare = gate(&model, "bare");
    assert_eq!(bare.preset_chain, vec![BASE.to_owned()]);
    assert_eq!(bare.include, vec!["**/*.unit.{ts,tsx}".to_owned()]);
    assert!(
        !bare.opaque,
        "read whole through the preset: {:?}",
        bare.unread
    );
    assert!(bare.unread.is_empty());
}

#[test]
fn a_named_import_is_read_from_that_export_of_the_preset() {
    let model = build();
    let named = gate(&model, "named");
    assert_eq!(
        named.preset_chain,
        vec!["packages/config/named.mjs".to_owned()]
    );
    assert_eq!(named.include, vec!["**/*.strict.ts".to_owned()]);
    assert_eq!(
        named.restore_mocks,
        Some(true),
        "a scalar the preset sets is inherited"
    );
    assert!(!named.opaque, "{:?}", named.unread);
}

#[test]
fn an_export_default_from_follows_the_whole_chain() {
    let model = build();
    let reexport = gate(&model, "reexport");
    assert_eq!(
        reexport.preset_chain,
        vec![REACT.to_owned(), BASE.to_owned()],
        "the variant, then the base it merges"
    );
    assert_eq!(reexport.include, vec!["**/*.unit.{ts,tsx}".to_owned()]);
    assert_eq!(
        reexport.thresholds.len(),
        1,
        "the threshold the variant declares is the inheritor's: {:?}",
        reexport.thresholds
    );
    assert!(!reexport.opaque, "{:?}", reexport.unread);
}

#[test]
fn define_config_of_a_preset_is_the_preset() {
    let model = build();
    let wrapped = gate(&model, "wrapped");
    assert_eq!(
        wrapped.preset_chain,
        vec![REACT.to_owned(), BASE.to_owned()]
    );
    assert!(!wrapped.opaque, "{:?}", wrapped.unread);
}

#[test]
fn a_relative_path_resolves_like_a_package_specifier() {
    let model = build();
    let relative = gate(&model, "relative");
    assert_eq!(relative.preset_chain, vec![BASE.to_owned()]);
    assert!(!relative.opaque, "{:?}", relative.unread);
}

#[test]
fn a_local_const_export_is_followed_to_its_initialiser() {
    let model = build();
    let local = gate(&model, "local");
    assert!(local.preset_chain.is_empty(), "no preset is involved");
    assert_eq!(local.include, vec!["**/*.local.ts".to_owned()]);
    assert!(!local.opaque, "{:?}", local.unread);
}

#[test]
fn a_presets_setup_files_are_the_inheritors_spelled_from_the_root() {
    // base.mjs declares `setupFiles: ['./setup.ts']`, relative to the preset. The inheritor
    // loads that file (the runner does), so the gate carries it — spelled from the root, not
    // as `./setup.ts` beside the kit where no such file exists — and the invariant the
    // preset's setup declares (an `afterEach(restoreAllMocks)`) counts for the inheritor.
    let model = build();
    let bare = gate(&model, "bare");
    assert_eq!(
        bare.setup_files,
        vec!["packages/config/setup.ts".to_owned()]
    );
    assert_eq!(bare.setup.restore_mocks, Invariant::Present);
    assert_eq!(
        bare.setup.setup_file.as_deref(),
        Some("packages/config/setup.ts")
    );
}

#[test]
fn a_kits_own_setup_files_follow_the_presets_and_are_told_apart() {
    // own-msw merges base (which declares `./setup.ts` beside the preset) and adds its own
    // `./setup.ts`. The runner loads the preset's first; the gate keeps the inherited subset
    // apart so the setup reader knows which file started the mock server.
    let model = build();
    let own = gate(&model, "own-msw");
    assert_eq!(
        own.setup_files,
        vec![
            "packages/config/setup.ts".to_owned(),
            "./setup.ts".to_owned()
        ]
    );
    assert_eq!(
        own.inherited_setup_files,
        vec!["packages/config/setup.ts".to_owned()]
    );
    assert!(own.setup.msw_present, "the kit's own setup starts a server");
    assert!(
        !own.setup.msw_inherited,
        "the server is the kit's, not the preset's"
    );
    assert_eq!(
        own.setup.setup_file.as_deref(),
        Some("packages/own-msw/setup.ts"),
        "a finding on this gate points at the kit's own setup file"
    );
    assert_eq!(
        own.setup.restore_mocks,
        Invariant::Present,
        "the preset's afterEach(restoreAllMocks) still counts"
    );
    assert_eq!(
        own.setup.unhandled_request_error,
        Invariant::Absent,
        "the kit's listen() passes no onUnhandledRequest"
    );
}

#[test]
fn a_server_the_presets_own_setup_starts_is_inherited() {
    // inherits-msw bare-re-exports msw.mjs, whose `./msw-setup.ts` starts the mock server
    // with no `onUnhandledRequest`. The inheritor has no setup of its own, so the server —
    // and the gap — are the preset's: `msw_inherited` is set, and a rule can lift the MSW
    // invariant to the preset rather than report it at every inheritor.
    let model = build();
    let inherits = gate(&model, "inherits-msw");
    assert_eq!(
        inherits.preset_chain,
        vec!["packages/config/msw.mjs".to_owned()]
    );
    assert_eq!(
        inherits.inherited_setup_files,
        vec!["packages/config/msw-setup.ts".to_owned()],
        "the preset's setup file, spelled from the root"
    );
    assert!(
        inherits.setup.msw_present,
        "the preset's setup starts a server"
    );
    assert!(
        inherits.setup.msw_inherited,
        "the server is the preset's, not the kit's"
    );
    assert_eq!(inherits.setup.global_server, Invariant::Present);
    assert_eq!(
        inherits.setup.unhandled_request_error,
        Invariant::Absent,
        "the preset's listen() passes no onUnhandledRequest — a gap the preset owns"
    );
    assert_eq!(
        inherits.setup.setup_file.as_deref(),
        Some("packages/config/msw-setup.ts"),
        "with no setup of its own, a finding points at the inherited file"
    );
}

#[test]
fn the_import_as_written_is_recorded_beside_the_resolved_chain() {
    let model = build();
    let bare = gate(&model, "bare");
    assert_eq!(bare.preset_imports.len(), 1);
    assert_eq!(bare.preset_imports[0].specifier, "@fx/config-vitest/base");
    assert_eq!(bare.preset_imports[0].name, None, "the default export");
    let named = gate(&model, "named");
    assert_eq!(named.preset_imports[0].name.as_deref(), Some("strict"));
}

// ---------------------------------------------------------------- shapes it cannot follow

#[test]
fn a_preset_factory_call_is_opaque_and_names_the_preset() {
    let model = build();
    let factory = gate(&model, "factory");
    assert_eq!(
        factory.preset_chain,
        vec!["packages/config/factory.mjs".to_owned()],
        "the preset is resolved even though its export cannot be read"
    );
    assert!(factory.opaque);
    assert_eq!(
        factory.unread,
        vec![
            "preset '@fx/config-vitest/factory' (packages/config/factory.mjs): `export default` \
             declares a function or class the reader does not evaluate"
                .to_owned()
        ]
    );
    assert!(
        factory.include.is_empty(),
        "nothing is guessed from a function body"
    );
}

#[test]
fn an_unresolvable_specifier_is_opaque_and_names_the_specifier() {
    let model = build();
    let missing = gate(&model, "missing");
    assert!(missing.preset_chain.is_empty());
    assert!(missing.opaque);
    assert_eq!(
        missing.unread,
        vec!["preset '@fx/nowhere/base' could not be resolved from \
              packages/missing/vitest.config.mts"
            .to_owned()]
    );
}

#[test]
fn a_function_config_stays_opaque_with_no_chain() {
    let model = build();
    let computed = gate(&model, "computed");
    assert!(computed.preset_chain.is_empty());
    assert!(computed.preset_imports.is_empty());
    assert!(computed.opaque);
    assert_eq!(
        computed.unread,
        vec!["`defineConfig(…)` is passed a function the reader does not evaluate".to_owned()]
    );
}

#[test]
fn every_gate_is_either_read_whole_or_says_why_not() {
    let model = build();
    for gate in &model.gates {
        assert_eq!(
            gate.opaque,
            !gate.unread.is_empty(),
            "{}: opaque and unread agree: {:?}",
            gate.source,
            gate.unread
        );
    }
}

#[test]
fn preset_reexports_snapshot() {
    let model = build();
    insta::assert_json_snapshot!("preset_reexports_gates", model.gates);
}
