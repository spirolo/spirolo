//! Reading `.spirolorc.jsonc`: the project-local config the binary honours.
//!
//! The file is optional. Absent, the model builds with its defaults and every rule runs on
//! its built-ins; present, its `handlers` globs replace the recogniser's built-in
//! handler-file set, its `inputs.mutation` globs name the mutation reports the binary reads,
//! its `inputs.churn` names the `git log --numstat` file it reads, and the rest of its keys
//! feed the rules. Every key `spirolo init` writes — `handlers`, `inputs`, `dataLayer`,
//! `dataLayerScope`, `urlSources`, `claims`, `env`, `rules` — is read here, so the config is
//! not partly decorative. A file that does not parse never aborts the run: it becomes one
//! diagnostic carrying the parser's line and column, and the run falls back to defaults.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use jsonc_parser::errors::ParseError;
use jsonc_parser::{parse_to_value, JsonArray, JsonObject, JsonValue, ParseOptions};
use spirolo_model::{Diagnostic, Severity, Span};
use spirolo_rules::{ClaimDocument, Config, RuleConfig, Severity as RuleSeverity};

/// The name `spirolo` looks for in a project root.
pub const FILE_NAME: &str = ".spirolorc.jsonc";

/// What `.spirolorc.jsonc` asked for: the handler globs the model reads and the [`Config`]
/// every rule reads.
#[derive(Debug, Default)]
pub struct Rc {
    /// The `handlers` globs, naming the files handlers are read from.
    pub handler_globs: Vec<String>,
    /// The `inputs.mutation` globs, naming the mutation reports the binary reads and hands
    /// to `spirolo-inputs`. Empty when the key is absent, so the caller falls back to the
    /// default report path.
    pub mutation_globs: Vec<String>,
    /// The `inputs.churn` file, relative to the project root: `git log --numstat` output the
    /// binary reads into the model's churn table. `None` when the key is absent; `--churn`
    /// overrides it.
    pub churn_file: Option<String>,
    /// The keys the rules read: data layer, url sources, claims, env, per-rule overrides.
    pub config: Config,
}

/// The default `inputs.mutation` glob: Stryker's own report path under any package, so a
/// first run reads a committed report with no configuration.
pub const DEFAULT_MUTATION_GLOB: &str = "**/reports/mutation/mutation.json";

/// Load the rc from a project root: reads `root/.spirolorc.jsonc`.
///
/// A missing (or unreadable) file is the empty config and no diagnostic; a malformed file
/// is the empty config plus one `spirolorc_parse_failed` diagnostic located at the parse
/// error, so the run continues on defaults rather than aborting.
pub fn load(root: &Path) -> (Rc, Vec<Diagnostic>) {
    load_path(&root.join(FILE_NAME))
}

/// Load the rc from an explicit path, as `--config <path>` supplies.
///
/// The diagnostics name the given path rather than the default file name, so a malformed
/// external config points at the file the user pointed the flag at. A path that does not
/// exist is treated as no config (empty, no diagnostic): the caller has already checked
/// the path when the flag was given, so a missing default beside a repo is not an error.
pub fn load_path(path: &Path) -> (Rc, Vec<Diagnostic>) {
    let name = display_name(path);
    let Ok(source) = std::fs::read_to_string(path) else {
        return (Rc::default(), Vec::new());
    };
    match parse_to_value(&source, &ParseOptions::default()) {
        // A `handlers` glob that does not compile is the model's to report: it emits one
        // `glob_invalid` diagnostic per such glob, naming this file, while building its
        // matcher (see `BuildOptions::handler_globs_source`).
        Ok(value) => {
            let mut rc = from_value(value);
            let diagnostics = reject_hygiene_error(&mut rc.config, &name);
            (rc, diagnostics)
        }
        Err(error) => (
            Rc::default(),
            vec![parse_diagnostic(&source, &error, &name)],
        ),
    }
}

/// The name a diagnostic about a config file carries: its file name, or the default
/// name when the path has none the model can spell.
pub fn display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(FILE_NAME)
        .to_owned()
}

/// Resolve the config path a run should read: the `--config` value when given, else the
/// default file in `root`. Returns an error only when an explicit `--config` path does not
/// exist, so a mistyped flag is a diagnostic-worthy failure rather than a silent fallback.
pub fn resolve_config_path(root: &Path, config: Option<&Path>) -> Result<PathBuf, String> {
    let Some(path) = config else {
        return Ok(root.join(FILE_NAME));
    };
    if path.is_file() {
        Ok(path.to_path_buf())
    } else {
        Err(format!("config file {} does not exist", path.display()))
    }
}

/// Read the claims documents the config names, so `gate-truth`'s claims half can diff them.
///
/// Each `claims` entry is a repo-relative path under `root`; this reads the file's lines into a
/// [`ClaimDocument`] the rules crate reads (the crate itself never touches a file). A path that
/// does not exist or cannot be read is skipped silently — the rule then reports no disagreement
/// in a document it could not open, which is the honest, conservative default. Paths are
/// normalised to slash-separated form so a finding names the same path on every platform, and
/// the result is sorted by path so two runs produce byte-identical findings.
pub fn read_claim_documents(root: &Path, claims: &[String]) -> Vec<ClaimDocument> {
    let mut documents: Vec<ClaimDocument> = claims
        .iter()
        .filter_map(|claim| {
            let path = root.join(claim);
            let source = std::fs::read_to_string(&path).ok()?;
            Some(ClaimDocument {
                path: claim.replace('\\', "/"),
                lines: source.lines().map(str::to_owned).collect(),
            })
        })
        .collect();
    documents.sort_by(|a, b| a.path.cmp(&b.path));
    documents
}

/// Read the config out of a parsed value: `handlers` feeds the model, the rest feeds the
/// rules. A non-array where a list is expected, or a non-string element, is ignored
/// rather than guessed at. Takes the value by value so nested objects (`env`, `rules`) can
/// be moved out and iterated.
fn from_value(value: Option<JsonValue<'_>>) -> Rc {
    let mut rc = Rc::default();
    let Some(JsonValue::Object(mut object)) = value else {
        return rc;
    };
    rc.handler_globs = string_array(object.take_array("handlers"));
    if let Some(mut inputs) = object.take_object("inputs") {
        rc.mutation_globs = string_array(inputs.take_array("mutation"));
        rc.churn_file = inputs
            .take_string("churn")
            .map(std::borrow::Cow::into_owned);
    }
    rc.config.data_layer = string_array(object.take_array("dataLayer"));
    rc.config.data_layer_scope = string_array(object.take_array("dataLayerScope"));
    rc.config.url_sources = string_array(object.take_array("urlSources"));
    rc.config.claims = string_array(object.take_array("claims"));
    rc.config.env = string_map(object.take_object("env"));
    rc.config.rules = rules_map(object.take_object("rules"));
    rc
}

/// The string elements of an optional JSON array, dropping non-strings.
fn string_array(array: Option<JsonArray<'_>>) -> Vec<String> {
    let Some(array) = array else {
        return Vec::new();
    };
    array
        .into_iter()
        .filter_map(|element| match element {
            JsonValue::String(text) => Some(text.into_owned()),
            _ => None,
        })
        .collect()
}

/// A `{ name: value }` object as a sorted map, keeping only string values (`env` assumes a
/// string per variable).
fn string_map(object: Option<JsonObject<'_>>) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let Some(object) = object else {
        return map;
    };
    for (name, value) in object {
        if let JsonValue::String(value) = value {
            map.insert(name.into_owned(), value.into_owned());
        }
    }
    map
}

/// The `rules` block: each entry a bare severity string or an object with `severity` and
/// options (`maxRatio`). An entry that is neither is dropped.
fn rules_map(object: Option<JsonObject<'_>>) -> BTreeMap<String, RuleConfig> {
    let mut map = BTreeMap::new();
    let Some(object) = object else {
        return map;
    };
    for (name, value) in object {
        let name = name.into_owned();
        match value {
            JsonValue::String(text) => {
                if let Some(severity) = RuleSeverity::parse(text.as_ref()) {
                    map.insert(
                        name,
                        RuleConfig {
                            severity: Some(severity),
                            max_ratio: None,
                        },
                    );
                }
            }
            JsonValue::Object(mut entry) => {
                let severity = entry
                    .take_string("severity")
                    .and_then(|text| RuleSeverity::parse(text.as_ref()));
                let max_ratio = entry.take_number("maxRatio").and_then(|n| n.parse().ok());
                map.insert(
                    name,
                    RuleConfig {
                        severity,
                        max_ratio,
                    },
                );
            }
            _ => {}
        }
    }
    map
}

/// Reject any hygiene rule raised to `error`, dropping the override and returning one
/// diagnostic per refused rule.
///
/// Hygiene findings are `warn` or `info` and never fail the build (PRD 0003, story 15): a
/// config that tries to raise one to `error` is a mistake the tool refuses rather than
/// silently honours or silently ignores. The override is dropped so the rule keeps its
/// default severity, and the diagnostic — `error`-level, so `check` treats it as a config
/// failure and exits 2 — names the rule so the fix is obvious. A rule in any other family is
/// left untouched: only hygiene is forbidden `error`.
fn reject_hygiene_error(config: &mut Config, file: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for (name, entry) in &mut config.rules {
        if entry.severity == Some(RuleSeverity::Error) && is_hygiene_rule(name) {
            entry.severity = None;
            diagnostics.push(Diagnostic {
                code: "hygiene_severity_rejected".to_owned(),
                severity: Severity::Error,
                file: Some(file.to_owned()),
                span: None,
                message: format!(
                    "rule {name:?} is a hygiene rule and cannot be set to error; hygiene \
                     findings are warn or info and never fail the build"
                ),
            });
        }
    }
    diagnostics
}

/// Whether `name` is a rule in the `hygiene` family, so its `error` override is refused. An
/// unknown rule name is not hygiene, so a typo is left for the "no such rule" path to catch.
fn is_hygiene_rule(name: &str) -> bool {
    spirolo_rules::rule_named(name)
        .is_some_and(|rule| rule.family() == spirolo_rules::Family::Hygiene)
}

/// One diagnostic for a `.spirolorc.jsonc` that did not parse, located at the parse error.
fn parse_diagnostic(source: &str, error: &ParseError, file: &str) -> Diagnostic {
    let start = u32::try_from(error.range().start).unwrap_or(u32::MAX);
    Diagnostic {
        code: "spirolorc_parse_failed".to_owned(),
        severity: Severity::Error,
        file: Some(file.to_owned()),
        span: Some(Span::locate(source, start, start)),
        message: format!("{file} is not valid JSONC: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{from_value, Rc};
    use jsonc_parser::{parse_to_value, ParseOptions};
    use spirolo_rules::Severity as RuleSeverity;

    fn rc(source: &str) -> Rc {
        let value = parse_to_value(source, &ParseOptions::default()).unwrap();
        from_value(value)
    }

    fn globs(source: &str) -> Vec<String> {
        rc(source).handler_globs
    }

    #[test]
    fn handlers_is_read_as_a_list_of_globs() {
        assert_eq!(
            globs("{ \"handlers\": [\"src/mocks/**\", \"test/msw/**\"] }"),
            ["src/mocks/**", "test/msw/**"]
        );
    }

    #[test]
    fn a_missing_handlers_key_is_no_globs() {
        assert!(globs("{ \"dataLayer\": [] }").is_empty());
    }

    #[test]
    fn a_non_string_glob_is_skipped_not_a_guess() {
        assert_eq!(globs("{ \"handlers\": [\"ok/**\", 7, true] }"), ["ok/**"]);
    }

    #[test]
    fn comments_and_trailing_commas_are_accepted() {
        assert_eq!(
            globs("{\n  // where handlers live\n  \"handlers\": [\"a/**\",],\n}"),
            ["a/**"]
        );
    }

    #[test]
    fn an_empty_document_is_the_empty_config() {
        assert!(Rc::default().handler_globs.is_empty());
        assert!(globs("").is_empty());
    }

    #[test]
    fn inputs_mutation_is_read_as_a_list_of_globs() {
        let mutation =
            rc(r#"{ "inputs": { "mutation": ["a/**/mutation.json", "b.json"] } }"#).mutation_globs;
        assert_eq!(mutation, ["a/**/mutation.json", "b.json"]);
    }

    #[test]
    fn inputs_churn_is_read_as_one_file() {
        let churn = rc(r#"{ "inputs": { "churn": "churn.numstat" } }"#).churn_file;
        assert_eq!(churn.as_deref(), Some("churn.numstat"));
    }

    #[test]
    fn a_missing_inputs_key_is_no_mutation_globs() {
        assert!(rc("{ \"handlers\": [] }").mutation_globs.is_empty());
    }

    #[test]
    fn every_reserved_key_is_read_into_the_config() {
        let config = rc(r#"{
                "handlers": ["src/mocks/**"],
                "dataLayer": ["@/supabaseClient"],
                "dataLayerScope": ["**/*.integration.*"],
                "urlSources": ["src/config/api.ts"],
                "claims": ["docs/PLAYBOOK.md"],
                "env": { "API_VERSION": "v1" }
            }"#)
        .config;
        assert_eq!(config.data_layer, ["@/supabaseClient"]);
        assert_eq!(config.data_layer_scope, ["**/*.integration.*"]);
        assert_eq!(config.url_sources, ["src/config/api.ts"]);
        assert_eq!(config.claims, ["docs/PLAYBOOK.md"]);
        assert_eq!(
            config.env.get("API_VERSION").map(String::as_str),
            Some("v1")
        );
    }

    /// A fresh empty directory under the OS temp dir, unique to `name`, for a file-reading test.
    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("spirolo-rc-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn claims_documents_are_read_from_the_project_root() {
        let root = scratch("read");
        std::fs::create_dir_all(root.join("docs")).unwrap();
        std::fs::write(
            root.join("docs/PLAYBOOK.md"),
            "# Testing\ncoverage thresholds are enforced\n",
        )
        .unwrap();
        let documents = super::read_claim_documents(&root, &["docs/PLAYBOOK.md".to_owned()]);
        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0].path, "docs/PLAYBOOK.md");
        assert_eq!(
            documents[0].lines,
            vec![
                "# Testing".to_owned(),
                "coverage thresholds are enforced".to_owned()
            ]
        );
    }

    #[test]
    fn a_missing_claims_document_is_skipped_not_an_error() {
        let root = scratch("missing");
        let documents = super::read_claim_documents(&root, &["docs/GONE.md".to_owned()]);
        assert!(
            documents.is_empty(),
            "a path that does not exist yields no document"
        );
    }

    #[test]
    fn a_hygiene_rule_set_to_error_is_rejected_with_a_diagnostic_naming_it() {
        let mut rc = rc(r#"{ "rules": {
                "duplicate-test": "error",
                "weak-assertion": "error"
            } }"#);
        let diagnostics = super::reject_hygiene_error(&mut rc.config, ".spirolorc.jsonc");

        // The hygiene rule's error override is dropped, leaving it on its default severity.
        assert_eq!(
            rc.config.rules.get("duplicate-test").unwrap().severity,
            None
        );
        // An audit rule can still be raised to error: only hygiene is refused.
        assert_eq!(
            rc.config.rules.get("weak-assertion").unwrap().severity,
            Some(RuleSeverity::Error)
        );

        assert_eq!(diagnostics.len(), 1, "one diagnostic, for the hygiene rule");
        assert_eq!(diagnostics[0].code, "hygiene_severity_rejected");
        assert_eq!(diagnostics[0].severity, spirolo_model::Severity::Error);
        assert!(
            diagnostics[0].message.contains("duplicate-test"),
            "the diagnostic names the rule: {}",
            diagnostics[0].message
        );
    }

    #[test]
    fn a_rule_entry_is_a_bare_severity_or_an_object_with_options() {
        let config = rc(r#"{ "rules": {
                "weak-assertion": { "severity": "error", "maxRatio": 0.08 },
                "dead-skip": "info"
            } }"#)
        .config;
        let weak = config.rules.get("weak-assertion").unwrap();
        assert_eq!(weak.severity, Some(RuleSeverity::Error));
        assert_eq!(weak.max_ratio, Some(0.08));
        let skip = config.rules.get("dead-skip").unwrap();
        assert_eq!(skip.severity, Some(RuleSeverity::Info));
    }
}
