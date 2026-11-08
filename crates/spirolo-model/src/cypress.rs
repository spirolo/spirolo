//! Which specs a Cypress config collects, read from its `cypress.config.*` by AST.
//!
//! Cypress is not a runner the model scopes test cases by — its specs are recognised by the
//! same `describe`/`it` vocabulary as a Jest suite — but a spec it collects runs, so
//! `orphan-test` must not call it never collected. Each testing type the config declares
//! contributes its `specPattern`, relative to the config's directory, or Cypress's default
//! for that type when it sets none: `cypress/e2e/**/*.cy.{js,jsx,ts,tsx}` for `e2e`,
//! `**/*.cy.{js,jsx,ts,tsx}` for `component`. A config that declares neither runs `e2e`.
//! A config whose export or `specPattern` the reader cannot see into collects an unknown
//! set: [`CypressConfig::spec_patterns`] is `None`, and every file under its directory is
//! left undecided rather than reported.

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;

use crate::config_reader::{export_parts, literal_strings, object_value, prop, ConfigPart};

/// Cypress's default `specPattern` for end-to-end specs.
const E2E_DEFAULT: &str = "cypress/e2e/**/*.cy.{js,jsx,ts,tsx}";

/// Cypress's default `specPattern` for component specs.
const COMPONENT_DEFAULT: &str = "**/*.cy.{js,jsx,ts,tsx}";

/// One `cypress.config.*` and the specs it collects.
pub struct CypressConfig {
    /// Repo-relative directory of the config, which its spec patterns are relative to.
    pub dir: String,
    /// Every testing type's spec patterns, or `None` when the config could not be read.
    pub spec_patterns: Option<Vec<String>>,
}

/// Whether a repo-relative path names a Cypress config.
pub fn is_config(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.strip_prefix("cypress.config.")
        .is_some_and(|ext| matches!(ext, "js" | "cjs" | "mjs" | "ts" | "cts" | "mts"))
}

/// Read the config at `path` (repo-relative) under `root`.
pub fn read(root: &std::path::Path, path: &str) -> CypressConfig {
    let dir = path
        .rfind('/')
        .map_or("", |slash| &path[..slash])
        .to_owned();
    let spec_patterns = std::fs::read_to_string(root.join(path))
        .ok()
        .and_then(|source| spec_patterns(path, &source));
    CypressConfig { dir, spec_patterns }
}

/// The spec patterns a config's source declares, or `None` when it cannot be read whole.
fn spec_patterns(path: &str, source: &str) -> Option<Vec<String>> {
    let source_type = SourceType::from_path(path).ok()?;
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();
    let parts = export_parts(&parsed.program);
    let [ConfigPart::Object(config)] = parts.as_slice() else {
        return None;
    };
    let mut patterns = Vec::new();
    let mut declared = false;
    for (kind, default) in [("e2e", E2E_DEFAULT), ("component", COMPONENT_DEFAULT)] {
        let Some(value) = prop(config, kind) else {
            continue;
        };
        declared = true;
        let block = object_value(value)?;
        match prop(block, "specPattern") {
            Some(spec) => patterns.extend(literal_strings(spec)?),
            None => patterns.push(default.to_owned()),
        }
    }
    if !declared {
        patterns.push(E2E_DEFAULT.to_owned());
    }
    Some(patterns)
}
