//! Snippet-to-rows tests for the MSW/nock recogniser: one library idiom, one snapshot.
//!
//! Every case reads the `Handler` rows the recogniser returns — the same table the JSON
//! surface carries. Snippets live at a `mocks/` path, because a handler is recognised
//! only in a handler file; a `src/` path proves the gate.

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use spirolo_frameworks::MswNock;
use spirolo_model::{FileContext, Handler, HandlerLibrary, Language, PatternKind, Recogniser};

/// Run the recogniser over one snippet at a given path.
fn handlers_at(path: &str, source: &str) -> Vec<Handler> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::from_path(path).unwrap()).parse();
    let file = FileContext {
        path,
        language: Language::TypeScript,
        source,
        program: &parsed.program,
        handler_globs: None,
    };
    MswNock.handlers(&file)
}

/// Run the recogniser over a snippet as if it were a handler file under `mocks/`.
fn handlers(source: &str) -> Vec<Handler> {
    handlers_at("src/mocks/handlers.ts", source)
}

// ---------------------------------------------------------------- one snapshot per idiom

#[test]
fn msw_v2_http() {
    insta::assert_json_snapshot!(handlers(
        "import { http, HttpResponse } from 'msw'\n\
         export const handlers = [\n\
         \x20 http.get('/api/user', () => HttpResponse.json({})),\n\
         \x20 http.post('/api/user', () => HttpResponse.json({})),\n\
         \x20 http.all('/api/*', () => HttpResponse.json({})),\n\
         ]\n"
    ));
}

#[test]
fn msw_v1_rest() {
    insta::assert_json_snapshot!(handlers(
        "import { rest } from 'msw'\n\
         export const handlers = [\n\
         \x20 rest.get('/api/user', (req, res, ctx) => res(ctx.json({}))),\n\
         \x20 rest.delete('/api/user/:id', (req, res, ctx) => res(ctx.status(204))),\n\
         ]\n"
    ));
}

#[test]
fn msw_graphql() {
    insta::assert_json_snapshot!(handlers(
        "import { graphql, HttpResponse } from 'msw'\n\
         export const handlers = [\n\
         \x20 graphql.query('GetUser', () => HttpResponse.json({ data: {} })),\n\
         \x20 graphql.mutation('AddTodo', () => HttpResponse.json({ data: {} })),\n\
         ]\n"
    ));
}

#[test]
fn nock_chain() {
    insta::assert_json_snapshot!(handlers(
        "import nock from 'nock'\n\
         nock('https://api.example.com')\n\
         \x20 .get('/users')\n\
         \x20 .reply(200, [])\n\
         \x20 .post('/users')\n\
         \x20 .reply(201, {})\n"
    ));
}

#[test]
fn a_regex_and_template_pattern_are_distinguished_in_json() {
    // The JSON carries `pattern_kind`, so a consumer tells a regex (never judged) from a
    // template (folded) from a plain string.
    insta::assert_json_snapshot!(handlers(
        "import { http, HttpResponse } from 'msw'\n\
         export const handlers = [\n\
         \x20 http.get('/api/user', () => HttpResponse.json({})),\n\
         \x20 http.get(`/api/version`, () => HttpResponse.json({})),\n\
         \x20 http.get(/status-message-templates/, () => HttpResponse.json({})),\n\
         ]\n"
    ));
}

#[test]
fn an_opaque_pattern_is_kept_not_dropped() {
    // A pattern built from a runtime call folds to nothing: kept as opaque, not dropped.
    insta::assert_json_snapshot!(handlers(
        "import { http, HttpResponse } from 'msw'\n\
         export const handlers = [\n\
         \x20 http.get(makeUrl('/user'), () => HttpResponse.json({})),\n\
         ]\n"
    ));
}

// ------------------------------------------------------------ what the snapshots must hold

#[test]
fn a_literal_pattern_is_folded_and_the_method_uppercased() {
    let rows = handlers("import { http } from 'msw'\nhttp.get('/api/user', () => {})\n");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].library, HandlerLibrary::Msw);
    assert_eq!(rows[0].method.as_deref(), Some("GET"));
    assert_eq!(rows[0].pattern.as_deref(), Some("/api/user"));
}

#[test]
fn a_computed_call_pattern_is_opaque() {
    // A pattern the fold cannot decide (a function call) stays opaque, not a guess.
    let rows = handlers("import { http } from 'msw'\nhttp.get(buildUrl(), () => {})\n");
    assert_eq!(rows.len(), 1, "recorded, not dropped");
    assert_eq!(rows[0].pattern, None, "opaque, not a guess");
    assert_eq!(rows[0].pattern_kind, PatternKind::Opaque);
    assert_eq!(rows[0].method.as_deref(), Some("GET"));
}

#[test]
fn a_string_literal_pattern_is_recorded_as_a_string_kind() {
    let rows = handlers("import { http } from 'msw'\nhttp.get('/api/user', () => {})\n");
    assert_eq!(rows[0].pattern_kind, PatternKind::String);
    assert_eq!(rows[0].pattern.as_deref(), Some("/api/user"));
}

#[test]
fn a_substitution_free_template_folds_and_is_a_template_kind() {
    let rows = handlers("import { http } from 'msw'\nhttp.get(`/api/user`, () => {})\n");
    assert_eq!(rows[0].pattern_kind, PatternKind::Template);
    assert_eq!(rows[0].pattern.as_deref(), Some("/api/user"));
}

#[test]
fn a_template_over_a_local_base_constant_folds() {
    // `const BASE = '*/api/v1'; http.get(`${BASE}/materials`)` — the common MSW idiom — folds
    // against the file's own constants to a comparable path.
    let rows = handlers(
        "import { http } from 'msw'\nconst BASE = '*/api/v1'\nhttp.get(`${BASE}/materials`, () => {})\n",
    );
    assert_eq!(rows[0].pattern_kind, PatternKind::Template);
    assert_eq!(rows[0].pattern.as_deref(), Some("*/api/v1/materials"));
}

#[test]
fn a_base_constant_from_an_env_read_folds_to_its_default() {
    // `const BASE_URL = process.env["X"] ?? "http://localhost:3101"` — the modern MSW idiom
    // — folds to the default (the value that runs when the variable is unset), so a handler
    // built from it decides.
    let rows = handlers(
        "import { http } from 'msw'\n\
         const BASE_URL = process.env[\"NEXT_PUBLIC_API_URL\"] ?? 'http://localhost:3101'\n\
         http.get(`${BASE_URL}/api/users`, () => {})\n",
    );
    assert_eq!(
        rows[0].pattern.as_deref(),
        Some("http://localhost:3101/api/users")
    );
}

#[test]
fn a_template_over_an_unknown_binding_is_opaque_not_a_guess() {
    // No local constant named `base`: the pattern cannot fold, so it stays opaque.
    let rows = handlers("import { http } from 'msw'\nhttp.get(`${base}/user`, () => {})\n");
    assert_eq!(
        rows[0].pattern_kind,
        PatternKind::Opaque,
        "an unresolved binding is opaque"
    );
    assert_eq!(rows[0].pattern, None);
}

#[test]
fn a_bare_identifier_pattern_folds_to_a_local_constant() {
    let rows =
        handlers("import { http } from 'msw'\nconst URL = '*/v1/users'\nhttp.get(URL, () => {})\n");
    assert_eq!(rows[0].pattern.as_deref(), Some("*/v1/users"));
}

#[test]
fn a_regex_literal_pattern_is_recorded_and_never_folded() {
    let rows = handlers("import { http } from 'msw'\nhttp.get(/status-message/, () => {})\n");
    assert_eq!(rows.len(), 1, "recorded, not dropped");
    assert_eq!(rows[0].pattern_kind, PatternKind::Regex);
    assert_eq!(
        rows[0].pattern, None,
        "a regex is never judged, so never folded"
    );
}

#[test]
fn msw_all_has_no_method() {
    let rows = handlers("import { http } from 'msw'\nhttp.all('/api/*', () => {})\n");
    assert_eq!(rows[0].method, None, "any method");
    assert_eq!(rows[0].pattern.as_deref(), Some("/api/*"));
}

#[test]
fn a_graphql_operation_keeps_its_name_as_the_pattern() {
    let rows = handlers("import { graphql } from 'msw'\ngraphql.query('GetUser', () => {})\n");
    assert_eq!(rows[0].library, HandlerLibrary::Msw);
    assert_eq!(rows[0].method, None, "graphql has no HTTP method");
    assert_eq!(rows[0].pattern.as_deref(), Some("GetUser"));
}

#[test]
fn a_nock_verb_records_its_path_and_library() {
    let rows =
        handlers("import nock from 'nock'\nnock('https://x.test').get('/users').reply(200)\n");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].library, HandlerLibrary::Nock);
    assert_eq!(rows[0].method.as_deref(), Some("GET"));
    assert_eq!(rows[0].pattern.as_deref(), Some("/users"));
}

#[test]
fn handlers_outside_a_handler_file_are_not_collected() {
    let rows = handlers_at(
        "src/feature.ts",
        "import { http } from 'msw'\nhttp.get('/api/user', () => {})\n",
    );
    assert!(rows.is_empty(), "only handler files are read");
}

/// Run the recogniser over a snippet at `path` with an explicit set of handler globs, the
/// way `.spirolorc.jsonc` supplies them.
fn handlers_globbed(path: &str, globs: &[&str], source: &str) -> Vec<Handler> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::from_path(path).unwrap()).parse();
    let mut builder = globset::GlobSet::builder();
    for glob in globs {
        builder.add(globset::Glob::new(glob).unwrap());
    }
    let set = builder.build().unwrap();
    let file = FileContext {
        path,
        language: Language::TypeScript,
        source,
        program: &parsed.program,
        handler_globs: Some(&set),
    };
    MswNock.handlers(&file)
}

#[test]
fn a_configured_glob_makes_a_non_mocks_file_a_handler_file() {
    let rows = handlers_globbed(
        "src/testing/server.ts",
        &["src/testing/**"],
        "import { http } from 'msw'\nhttp.get('/api/user', () => {})\n",
    );
    assert_eq!(rows.len(), 1, "the glob names this file a handler file");
    assert_eq!(rows[0].pattern.as_deref(), Some("/api/user"));
}

#[test]
fn a_configured_glob_excludes_a_mocks_file_it_does_not_name() {
    // With globs configured, the built-in `mocks` default no longer applies: only the
    // glob decides, so a `mocks` file the glob misses is not a handler file.
    let rows = handlers_globbed(
        "src/mocks/handlers.ts",
        &["src/testing/**"],
        "import { http } from 'msw'\nhttp.get('/api/user', () => {})\n",
    );
    assert!(rows.is_empty(), "the configured glob replaces the default");
}

#[test]
fn the_node_http_module_is_not_mistaken_for_msw() {
    let rows = handlers_at(
        "src/mocks/server.ts",
        "import http from 'http'\nhttp.get('http://localhost/x', () => {})\n",
    );
    assert!(rows.is_empty(), "no msw import, so http is Node's");
}
