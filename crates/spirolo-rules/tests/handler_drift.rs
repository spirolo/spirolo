//! `handler-drift` on hand-built models: one case per branch of the MSW matching semantics.
//!
//! The rule is a pure function of the model and its config, so these tests build the
//! `Handler` rows and the base-URL `Constant`s by hand and read the findings out — never how
//! they were computed. The binary fixture (Seam 2) proves the same rule end to end through
//! `spirolo check`; a corpus replay pair (in `just bench`) proves it on the entry that gave
//! the rule catalogue its classes.

use std::collections::BTreeMap;

use spirolo_model::{Constant, Handler, HandlerLibrary, Model, Module, PatternKind, Span};
use spirolo_rules::{Config, Finding, HandlerDrift, Outcome, Rule};

/// A span at a line, enough to sort and locate by.
const fn span(line: u32) -> Span {
    Span {
        start: line,
        end: line + 1,
        line,
        column: 1,
    }
}

/// An MSW handler with a folded string pattern.
fn handler(file: &str, pattern: &str, line: u32) -> Handler {
    Handler {
        file: file.to_owned(),
        span: span(line),
        library: HandlerLibrary::Msw,
        method: Some("GET".to_owned()),
        pattern: Some(pattern.to_owned()),
        pattern_kind: PatternKind::String,
    }
}

/// A handler whose pattern the model could not fold, tagged with `kind` (regex or opaque).
fn undecidable_handler(file: &str, kind: PatternKind, line: u32) -> Handler {
    Handler {
        file: file.to_owned(),
        span: span(line),
        library: HandlerLibrary::Msw,
        method: Some("GET".to_owned()),
        pattern: None,
        pattern_kind: kind,
    }
}

/// A concrete folded string constant named `name`.
fn constant(name: &str, value: &str, line: u32) -> Constant {
    Constant {
        name: name.to_owned(),
        value: Some(value.to_owned()),
        opaque: false,
        reason: None,
        env_template: None,
        span: span(line),
    }
}

/// An opaque constant carrying an env template (a `${VAR}` hole the model preserved).
fn env_constant(name: &str, template: &str, line: u32) -> Constant {
    Constant {
        name: name.to_owned(),
        value: None,
        opaque: true,
        reason: Some("read from the environment".to_owned()),
        env_template: Some(template.to_owned()),
        span: span(line),
    }
}

/// A model with one config module carrying the base-URL constants, and the given handlers.
fn model(config_file: &str, constants: Vec<Constant>, handlers: Vec<Handler>) -> Model {
    Model {
        modules: vec![Module {
            path: config_file.to_owned(),
            language: spirolo_model::Language::TypeScript,
            parse_status: spirolo_model::ParseStatus::Parsed,
            imports: Vec::new(),
            exports: Vec::new(),
            constants,
            mocks: Vec::new(),
            references: Vec::new(),
            spies: Vec::new(),
            loads: Vec::new(),
            complexity: 1,
            generated: false,
        }],
        handlers,
        ..Model::default()
    }
}

/// A config naming one `urlSources` module and an optional `env` map.
fn config(url_sources: &[&str], env: &[(&str, &str)]) -> Config {
    Config {
        url_sources: url_sources.iter().map(ToString::to_string).collect(),
        env: env
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect::<BTreeMap<_, _>>(),
        ..Config::default()
    }
}

/// Every finding the rule produces.
fn findings(model: &Model, config: &Config) -> Vec<Finding> {
    HandlerDrift.run(model, config)
}

/// The decided-drift findings only.
fn drifts(model: &Model, config: &Config) -> Vec<Finding> {
    findings(model, config)
        .into_iter()
        .filter(|f| f.outcome == Outcome::Finding)
        .collect()
}

/// The could-not-decide rows only.
fn undecided(model: &Model, config: &Config) -> Vec<Finding> {
    findings(model, config)
        .into_iter()
        .filter(|f| f.outcome == Outcome::CouldNotDecide)
        .collect()
}

// ------------------------------------------------------------------ the acceptance fixtures

#[test]
fn a_handler_the_base_path_cannot_prefix_is_a_drift_naming_both() {
    // `*/api/v1/x` against base `http://localhost:5000/v1` (path `/v1`): `/v1` is not a
    // prefix of `/api/v1/x`, so the handler is dead.
    let model = model(
        "src/config/api.ts",
        vec![constant("urlApi", "http://localhost:5000/v1", 1)],
        vec![handler("src/mocks/handlers.ts", "*/api/v1/x", 3)],
    );
    let drifts = drifts(&model, &config(&["src/config/api.ts"], &[]));
    assert_eq!(drifts.len(), 1, "the one dead handler");
    let finding = &drifts[0];
    assert_eq!(finding.location.file, "src/mocks/handlers.ts");
    // The message names the pattern and the base it cannot reach.
    assert!(
        finding.message.contains("*/api/v1/x"),
        "{}",
        finding.message
    );
    assert!(
        finding.message.contains("http://localhost:5000/v1"),
        "{}",
        finding.message
    );
    // A related location points at the base URL declaration.
    assert!(
        finding
            .related
            .iter()
            .any(|r| r.label == "base url" && r.location.file == "src/config/api.ts"),
        "the base URL is a related location"
    );
    // The explain repairs rather than deletes: it tells the reader to fix the base path and
    // explicitly not to remove the handler.
    assert!(finding.explain.contains("do not delete the handler"));
    assert!(finding.explain.to_lowercase().contains("fix"));
}

#[test]
fn a_handler_the_base_path_prefixes_is_ok_not_a_finding() {
    // `*/v1/x` against base path `/v1`: `/v1` prefixes `/v1/x`, so the handler is reachable.
    let model = model(
        "src/config/api.ts",
        vec![constant("urlApi", "http://localhost:5000/v1", 1)],
        vec![handler("src/mocks/handlers.ts", "*/v1/x", 3)],
    );
    assert!(
        findings(&model, &config(&["src/config/api.ts"], &[])).is_empty(),
        "a reachable handler is ok, no row at all"
    );
}

#[test]
fn an_opaque_base_segment_with_no_env_is_could_not_decide() {
    // `urlApi = http://localhost:5000/${API_VERSION}` with no `env`: the version segment is
    // opaque, and `*/v1/x` neither clearly matches nor clearly drifts.
    let model = model(
        "src/config/api.ts",
        vec![env_constant(
            "urlApi",
            "http://localhost:5000/${API_VERSION}",
            1,
        )],
        vec![handler("src/mocks/handlers.ts", "*/v1/x", 3)],
    );
    let undecided = undecided(&model, &config(&["src/config/api.ts"], &[]));
    assert_eq!(undecided.len(), 1, "one could-not-decide row");
    assert!(drifts(&model, &config(&["src/config/api.ts"], &[])).is_empty());
}

#[test]
fn an_env_assumption_folds_the_opaque_segment_and_decides_it() {
    // The same opaque base, now with `env.API_VERSION = v1`: `http://localhost:5000/v1`,
    // path `/v1`, which prefixes `*/v1/x` — decided, and reachable, so no row.
    let model = model(
        "src/config/api.ts",
        vec![env_constant(
            "urlApi",
            "http://localhost:5000/${API_VERSION}",
            1,
        )],
        vec![handler("src/mocks/handlers.ts", "*/v1/x", 3)],
    );
    let config = config(&["src/config/api.ts"], &[("API_VERSION", "v1")]);
    assert!(
        findings(&model, &config).is_empty(),
        "with env the base folds to /v1 and the handler matches"
    );
}

#[test]
fn an_env_assumption_decides_a_drift_when_the_folded_base_still_disagrees() {
    // With `env.API_VERSION = v1` the base folds to `/v1`; the shipped-bug handler `*/api/v1`
    // still disagrees on the `/api/` segment, so the drift is decided, not merely undecided.
    let model = model(
        "src/config/api.ts",
        vec![env_constant(
            "urlApi",
            "http://localhost:5000/${API_VERSION}",
            1,
        )],
        vec![handler("src/mocks/handlers.ts", "*/api/v1/materials", 3)],
    );
    let config = config(&["src/config/api.ts"], &[("API_VERSION", "v1")]);
    let drifts = drifts(&model, &config);
    assert_eq!(drifts.len(), 1, "the /api/ disagreement is a decided drift");
}

// ------------------------------------------------------------------- the `*` semantics table

/// What the rule says about one (pattern, base) pair.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Expected {
    /// No row: the handler is reachable under the base.
    Reachable,
    /// A decided drift.
    Drifted,
    /// A could-not-decide row.
    Undecided,
}

/// The rule's verdict on one handler pattern against one literal `urlSources` base, with
/// the given `env` assumptions.
fn verdict(pattern: &str, base: &str, env: &[(&str, &str)]) -> Expected {
    let model = model(
        "src/config/api.ts",
        Vec::new(),
        vec![handler("src/mocks/handlers.ts", pattern, 3)],
    );
    let findings = findings(&model, &config(&[base], env));
    match findings.as_slice() {
        [] => Expected::Reachable,
        [one] if one.outcome == Outcome::Finding => Expected::Drifted,
        [one] if one.outcome == Outcome::CouldNotDecide => Expected::Undecided,
        other => panic!("one row at most for one handler: {other:?}"),
    }
}

/// The module doc's table: each row is one (pattern, base) pair and what the rule says.
/// MSW rewrites `*` to `(.*)` and matches origin + pathname: a leading `*` swallows the
/// origin and any path prefix, a `*` in the middle crosses segments, `:name` is one
/// segment, a pattern with no scheme is matched on the pathname only. The one refinement
/// (a tail that names the base's last segment behind a different prefix is a drift) keeps
/// the shipped `*/api/v1` class decidable.
const STAR_TABLE: &[(&str, &str, Expected)] = &[
    // a leading `*`
    ("*/v1/x", "http://localhost:5000/v1", Expected::Reachable),
    (
        "*/v1/x",
        "http://localhost:5000/api/v1",
        Expected::Reachable,
    ),
    ("*/x", "http://localhost:5000/v1", Expected::Reachable),
    ("*/api/v1/x", "http://localhost:5000/v1", Expected::Drifted),
    (
        "*/api/v1/x",
        "http://localhost:5000/api/v1",
        Expected::Reachable,
    ),
    (
        "*/v1/api/v1/x",
        "http://localhost:5000/v1",
        Expected::Reachable,
    ),
    (
        "*/:version/x",
        "http://localhost:5000/v1",
        Expected::Reachable,
    ),
    (
        "*/v1/*",
        "http://localhost:5000/v1/a/b",
        Expected::Reachable,
    ),
    ("*", "http://localhost:5000/v1", Expected::Reachable),
    (
        "*/v1/x?q=1",
        "http://localhost:5000/v1",
        Expected::Reachable,
    ),
    ("*/v1/x", "http://localhost:5000", Expected::Reachable),
    ("*/v1/x", "/api/v1", Expected::Reachable),
    ("*/api/v1/x", "/v1", Expected::Drifted),
    // absolute: origin and pathname
    (
        "http://localhost:5000/v1/x",
        "http://localhost:5000/v1",
        Expected::Reachable,
    ),
    (
        "http://localhost:5000/api/v1/x",
        "http://localhost:5000/v1",
        Expected::Drifted,
    ),
    (
        "http://other.test:9000/v1/x",
        "http://localhost:5000/v1",
        Expected::Drifted,
    ),
    (
        "http://localhost:5000/*/x",
        "http://localhost:5000/v1/api",
        Expected::Reachable,
    ),
    (
        "http://localhost:5000/v*",
        "http://localhost:5000/v1/x",
        Expected::Reachable,
    ),
    (
        "http://localhost:5000/w*",
        "http://localhost:5000/v1",
        Expected::Drifted,
    ),
    (
        "http://*.example.test/v1/x",
        "http://api.example.test/v1",
        Expected::Reachable,
    ),
    (
        "http://localhost:3101/api/solutions",
        "http://localhost:3101",
        Expected::Reachable,
    ),
    ("http://localhost:5000/v1/x", "/v1", Expected::Reachable),
    // relative: pathname only
    ("/v1/x", "http://localhost:5000/v1", Expected::Reachable),
    ("/v1/x", "http://localhost:5000/api/v1", Expected::Drifted),
    ("/api/v1/x", "http://localhost:5000/v1", Expected::Drifted),
    ("/v1/:id", "http://localhost:5000/v1", Expected::Reachable),
    // `:name` is exactly one segment: it cannot stand for the base's two, where `*` can.
    ("/:id", "http://localhost:5000/api/v1", Expected::Drifted),
    ("/*", "http://localhost:5000/api/v1", Expected::Reachable),
    ("/:id/x", "http://localhost:5000/api/v1", Expected::Drifted),
    ("/*/x", "http://localhost:5000/api/v1", Expected::Reachable),
    (
        "http://localhost:5000/:id/x",
        "http://localhost:5000/api/v1",
        Expected::Drifted,
    ),
    // opaque bases: never a decided drift
    ("*/api/v1/x", "${API}", Expected::Undecided),
    (
        "http://localhost:3101/api/x",
        "${NEXT_PUBLIC_API_URL}",
        Expected::Undecided,
    ),
    (
        "http://localhost:3101/api/x",
        "http://${API_HOST}/api",
        Expected::Undecided,
    ),
    (
        "http://localhost:5000/v1/x",
        "http://localhost:5000/${VERSION}",
        Expected::Undecided,
    ),
    (
        "http://localhost:5000/v1/x",
        "http://localhost:5000/api/${VERSION}",
        Expected::Undecided,
    ),
];

#[test]
fn the_star_is_read_as_msw_reads_it() {
    for (pattern, base, expected) in STAR_TABLE {
        assert_eq!(
            verdict(pattern, base, &[]),
            *expected,
            "pattern {pattern:?} against base {base:?}"
        );
    }
}

#[test]
fn an_env_assumption_turns_the_opaque_rows_into_decisions() {
    // The same opaque bases with their holes filled decide both ways.
    let env = &[
        ("API", "http://localhost:5000/v1"),
        ("NEXT_PUBLIC_API_URL", "http://localhost:3101"),
        ("API_HOST", "localhost:3101"),
        ("VERSION", "v1"),
    ];
    assert_eq!(verdict("*/api/v1/x", "${API}", env), Expected::Drifted);
    assert_eq!(
        verdict("http://localhost:3101/api/x", "${NEXT_PUBLIC_API_URL}", env),
        Expected::Reachable
    );
    assert_eq!(
        verdict("http://localhost:3101/api/x", "http://${API_HOST}/api", env),
        Expected::Reachable
    );
    assert_eq!(
        verdict("http://localhost:9999/api/x", "http://${API_HOST}/api", env),
        Expected::Drifted
    );
    assert_eq!(
        verdict(
            "http://localhost:5000/v1/x",
            "http://localhost:5000/api/${VERSION}",
            env
        ),
        Expected::Drifted
    );
}

#[test]
fn an_opaque_host_is_could_not_decide_and_names_the_hole_never_a_drift_with_no_base() {
    // Before: an absolute handler against `http://${API_HOST}/api` failed the origin check,
    // fell through `could_reach`, and was reported as a decided drift "under any base ()"
    // with an empty base list. Now: undecided, and the row says which hole is unresolved.
    let model = model(
        "src/config/api.ts",
        Vec::new(),
        vec![handler(
            "src/mocks/handlers.ts",
            "http://localhost:3101/api/x",
            3,
        )],
    );
    let config = config(&["http://${API_HOST}/api"], &[]);
    assert!(
        drifts(&model, &config).is_empty(),
        "never a drift with no concrete base"
    );
    let undecided = undecided(&model, &config);
    assert_eq!(undecided.len(), 1);
    let row = &undecided[0];
    assert!(row.message.contains("${API_HOST}"), "{}", row.message);
    assert!(
        row.evidence
            .iter()
            .any(|e| e.key == "reason" && e.value.contains("${API_HOST}")),
        "{:?}",
        row.evidence
    );
}

#[test]
fn a_concrete_base_beside_an_opaque_one_decides_only_when_the_opaque_one_cannot_reach() {
    // Concrete `/v1` does not reach `*/api/v1/x`; the opaque `http://${HOST}/api/v1` could,
    // so the row stays undecided rather than a drift the assumption could dissolve.
    let any_prefix = model(
        "src/config/api.ts",
        Vec::new(),
        vec![handler("src/mocks/handlers.ts", "*/api/v1/x", 3)],
    );
    let could = config(&["http://localhost:5000/v1", "http://${HOST}/api/v1"], &[]);
    assert_eq!(
        undecided(&any_prefix, &could).len(),
        1,
        "the opaque base could reach"
    );
    assert!(drifts(&any_prefix, &could).is_empty());

    // An absolute handler on another origin: the opaque base's known path `/other` already
    // disagrees, so the concrete base decides the drift and is the one named.
    let absolute = model(
        "src/config/api.ts",
        Vec::new(),
        vec![handler(
            "src/mocks/handlers.ts",
            "http://localhost:5000/api/v1/x",
            3,
        )],
    );
    let cannot = config(
        &[
            "http://localhost:5000/v1",
            "http://${HOST}/other/${VERSION}",
        ],
        &[],
    );
    let drifts = drifts(&absolute, &cannot);
    assert_eq!(drifts.len(), 1, "the concrete base decides");
    assert!(drifts[0].message.contains("http://localhost:5000/v1"));
    assert!(
        !drifts[0].message.contains("${HOST}"),
        "{}",
        drifts[0].message
    );
}

#[test]
fn the_identity_carries_the_method_so_get_and_post_of_one_url_do_not_collide() {
    let mut get = handler("src/mocks/handlers.ts", "*/api/v1/x", 3);
    get.method = Some("GET".to_owned());
    let mut post = handler("src/mocks/handlers.ts", "*/api/v1/x", 4);
    post.method = Some("POST".to_owned());
    let mut all = handler("src/mocks/handlers.ts", "*/api/v1/x", 5);
    all.method = None;
    let model = model(
        "src/config/api.ts",
        vec![constant("urlApi", "http://localhost:5000/v1", 1)],
        vec![get, post, all],
    );
    let drifts = drifts(&model, &config(&["src/config/api.ts"], &[]));
    assert_eq!(drifts.len(), 3);
    assert_eq!(
        drifts[0].id(),
        "handler-drift src/mocks/handlers.ts GET */api/v1/x#0"
    );
    assert_eq!(
        drifts[1].id(),
        "handler-drift src/mocks/handlers.ts POST */api/v1/x#0"
    );
    assert_eq!(
        drifts[2].id(),
        "handler-drift src/mocks/handlers.ts ANY */api/v1/x#0"
    );
}

// ---------------------------------------------------------------- pattern kinds and tokens

#[test]
fn a_regex_literal_pattern_is_never_judged() {
    // A regex pattern (`pattern: None`, kind `Regex`) is MSW's to run at runtime: no drift,
    // no could-not-decide, no row at all.
    let model = model(
        "src/config/api.ts",
        vec![constant("urlApi", "http://localhost:5000/v1", 1)],
        vec![undecidable_handler(
            "src/mocks/handlers.ts",
            PatternKind::Regex,
            3,
        )],
    );
    assert!(
        findings(&model, &config(&["src/config/api.ts"], &[])).is_empty(),
        "a regex is recorded, never judged"
    );
}

#[test]
fn a_computed_opaque_pattern_is_could_not_decide() {
    let model = model(
        "src/config/api.ts",
        vec![constant("urlApi", "http://localhost:5000/v1", 1)],
        vec![undecidable_handler(
            "src/mocks/handlers.ts",
            PatternKind::Opaque,
            3,
        )],
    );
    let undecided = undecided(&model, &config(&["src/config/api.ts"], &[]));
    assert_eq!(
        undecided.len(),
        1,
        "an opaque pattern is honest, not dropped"
    );
}

#[test]
fn a_named_param_matches_one_segment() {
    // Base `/v1`, handler `*/v1/:id`: `:id` is one segment, `/v1` prefixes, so reachable.
    let model = model(
        "src/config/api.ts",
        vec![constant("urlApi", "http://localhost:5000/v1", 1)],
        vec![handler("src/mocks/handlers.ts", "*/v1/:id", 3)],
    );
    assert!(findings(&model, &config(&["src/config/api.ts"], &[])).is_empty());
}

#[test]
fn a_query_string_never_takes_part() {
    // Base `/v1`, handler `*/v1/x?foo=bar`: the query is dropped, the path `/v1/x` matches.
    let model = model(
        "src/config/api.ts",
        vec![constant("urlApi", "http://localhost:5000/v1", 1)],
        vec![handler("src/mocks/handlers.ts", "*/v1/x?foo=bar", 3)],
    );
    assert!(findings(&model, &config(&["src/config/api.ts"], &[])).is_empty());
}

#[test]
fn an_absolute_handler_must_agree_on_scheme_and_host() {
    // An absolute handler on a different host than the base drifts: MSW matches origin too.
    let model = model(
        "src/config/api.ts",
        vec![constant("urlApi", "http://localhost:5000/v1", 1)],
        vec![handler(
            "src/mocks/handlers.ts",
            "http://other.test:9000/v1/x",
            3,
        )],
    );
    let drifts = drifts(&model, &config(&["src/config/api.ts"], &[]));
    assert_eq!(drifts.len(), 1, "a different origin is a drift");
}

#[test]
fn an_absolute_handler_matching_the_base_origin_and_path_is_ok() {
    let model = model(
        "src/config/api.ts",
        vec![constant("urlApi", "http://localhost:5000/v1", 1)],
        vec![handler(
            "src/mocks/handlers.ts",
            "http://localhost:5000/v1/x",
            3,
        )],
    );
    assert!(findings(&model, &config(&["src/config/api.ts"], &[])).is_empty());
}

// ------------------------------------------------------------------------- base resolution

#[test]
fn a_source_key_selects_one_named_constant() {
    // `path#name` reads only that constant; the other base is ignored, so `*/api/v1` drifts
    // against `/v1` alone.
    let model = model(
        "src/config/api.ts",
        vec![
            constant("urlApi", "http://localhost:5000/v1", 1),
            constant("urlOther", "http://localhost:5000/api/v1", 2),
        ],
        vec![handler("src/mocks/handlers.ts", "*/api/v1/x", 3)],
    );
    // Against the whole module (both bases), `*/api/v1/x` is reachable under `urlOther`: ok.
    assert!(findings(&model, &config(&["src/config/api.ts"], &[])).is_empty());
    // Against `#urlApi` only, `/v1` cannot reach `*/api/v1/x`: a drift.
    let drifts = drifts(&model, &config(&["src/config/api.ts#urlApi"], &[]));
    assert_eq!(drifts.len(), 1, "the named key narrows the bases");
}

#[test]
fn a_literal_base_url_source_is_used_directly_and_env_folds_its_hole() {
    // A base the model cannot fold from source (assembled at runtime) is declared directly
    // as a literal `urlSources` entry with a `${VAR}` hole; `env` folds it to `/v1`, and the
    // shipped-bug `*/api/v1` handler drifts against it.
    let model = model(
        "src/config/api.ts",
        Vec::new(),
        vec![handler("src/mocks/handlers.ts", "*/api/v1/materials", 3)],
    );
    let with_env = config(
        &["http://localhost:5000/${API_VERSION}"],
        &[("API_VERSION", "v1")],
    );
    let drifts = drifts(&model, &with_env);
    assert_eq!(drifts.len(), 1, "the literal base decides the drift");
    assert!(drifts[0].message.contains("http://localhost:5000/v1"));

    // Without the env assumption the hole survives, so the same handler is could-not-decide.
    let without_env = config(&["http://localhost:5000/${API_VERSION}"], &[]);
    assert_eq!(
        undecided(&model, &without_env).len(),
        1,
        "an unfilled hole is undecided"
    );
}

#[test]
fn a_root_relative_literal_base_is_honoured() {
    let model = model(
        "src/config/api.ts",
        Vec::new(),
        vec![handler("src/mocks/handlers.ts", "*/v1/users", 3)],
    );
    // Base `/v1` prefixes `*/v1/users`: reachable, no row.
    assert!(findings(&model, &config(&["/v1"], &[])).is_empty());
}

#[test]
fn no_url_sources_leaves_every_handler_undecided() {
    // Nothing to compare against: honest could-not-decide, never a guessed drift.
    let model = model(
        "src/config/api.ts",
        vec![constant("urlApi", "http://localhost:5000/v1", 1)],
        vec![handler("src/mocks/handlers.ts", "*/api/v1/x", 3)],
    );
    let undecided = undecided(&model, &Config::default());
    assert_eq!(undecided.len(), 1);
    assert!(drifts(&model, &Config::default()).is_empty());
}

// ------------------------------------------------------------------------------ determinism

#[test]
fn findings_are_sorted_by_file_then_span() {
    let model = model(
        "src/config/api.ts",
        vec![constant("urlApi", "http://localhost:5000/v1", 1)],
        vec![
            handler("src/mocks/b.ts", "*/api/v1/x", 5),
            handler("src/mocks/a.ts", "*/api/v1/y", 9),
        ],
    );
    let drifts = drifts(&model, &config(&["src/config/api.ts"], &[]));
    assert_eq!(drifts.len(), 2);
    assert_eq!(drifts[0].location.file, "src/mocks/a.ts", "sorted by file");
    assert_eq!(drifts[1].location.file, "src/mocks/b.ts");
}
