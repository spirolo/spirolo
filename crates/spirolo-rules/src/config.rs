//! The rules' view of `.spirolorc.jsonc`: the keys they read, as plain data.
//!
//! The binary parses the JSONC file (see the binary's `rc` module) and fills this struct;
//! the rules crate never touches a file. Every key `spirolo init` writes has a home here so
//! the config is not partly decorative: `handlers` feeds the model, `dataLayer`,
//! `dataLayerScope`, `urlSources`, `claims` and `env` feed the rules that read them, and
//! `rules` overrides each rule's severity and options.

use std::collections::BTreeMap;

use crate::Severity;

/// One rule's configuration: a severity override and any options it reads.
///
/// A `rules` entry in the config is either a bare severity string (`"warn"`) or an object
/// with a `severity` key and rule options; both land here, with unset fields left `None`
/// so the rule keeps its default.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RuleConfig {
    /// The severity override, or `None` to keep the rule's default.
    pub severity: Option<Severity>,
    /// The `weak-assertion` ceiling: the ratio of value-less assertions above which the
    /// ratio row is a finding. `None` keeps the rule's built-in default.
    pub max_ratio: Option<f64>,
}

/// One claims document the binary read for `gate-truth`, as lines of text.
///
/// The `claims` key names markdown documents by path, but the rules crate reads no file:
/// the binary reads each named document off disk and hands its lines here, so the claims
/// diff stays a pure query over data. `path` is the repo-relative, slash-separated path a
/// finding points at; `lines` are the document's lines in order, one-based by index, with
/// no trailing newline. A document the binary could not read is simply absent from the
/// list, so the rule never claims a disagreement in a file it could not open.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClaimDocument {
    /// Repo-relative, slash-separated path of the document — the location a finding names.
    pub path: String,
    /// The document's lines in source order, each without its trailing newline. The
    /// one-based line number of `lines[i]` is `i + 1`.
    pub lines: Vec<String>,
}

/// Everything the rules read from the config, filled by the binary from `.spirolorc.jsonc`.
///
/// Absent keys are empty collections and `None`s, which every rule reads as "use my
/// default", so a project with no config runs every rule on its built-ins.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Config {
    /// Modules declared as the data layer, resolved specifiers or package names.
    pub data_layer: Vec<String>,
    /// Globs scoping the files `data-layer-mock` applies to.
    pub data_layer_scope: Vec<String>,
    /// Modules or JSON keys naming the base URLs `handler-drift` folds against.
    pub url_sources: Vec<String>,
    /// Documents `gate-truth` diffs its claims against, named by path — the `claims` key.
    pub claims: Vec<String>,
    /// The claims documents' contents, read by the binary from the `claims` paths. The
    /// rules crate reads no file, so the binary fills this; the claims diff reads it.
    pub claim_documents: Vec<ClaimDocument>,
    /// Assumed values for named environment variables, read from the file only so a URL
    /// built from `process.env.X` folds without the tool reading the real environment.
    pub env: BTreeMap<String, String>,
    /// Per-rule severity and option overrides, keyed by rule name.
    pub rules: BTreeMap<String, RuleConfig>,
    /// Whether `--propose` asked for a proposal on every hygiene finding. Off by default;
    /// set only by the binary when the flag is present, and read only by the hygiene rules,
    /// which attach a [`crate::Proposal`] when it is on.
    pub propose: bool,
}

impl Config {
    /// The configured severity for `rule`, or `default` when the config does not override
    /// it.
    pub fn severity(&self, rule: &str, default: Severity) -> Severity {
        self.rules
            .get(rule)
            .and_then(|entry| entry.severity)
            .unwrap_or(default)
    }

    /// The `max_ratio` option for `rule`, or `default` when the config does not set it.
    pub fn max_ratio(&self, rule: &str, default: f64) -> f64 {
        self.rules
            .get(rule)
            .and_then(|entry| entry.max_ratio)
            .unwrap_or(default)
    }
}
