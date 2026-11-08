//! Things the model could not decide, reported rather than guessed at.

use serde::{Deserialize, Serialize};

use crate::Span;

/// How much a diagnostic should worry the reader. No severity ever aborts a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// The model lost information it was supposed to have (a file failed to parse).
    Error,
    /// The model guessed or gave up on one detail (an import it could not resolve).
    Warning,
    /// Context worth printing, nothing lost.
    Info,
}

/// A parse failure, an unresolved import, an unreadable config, an unpaired test.
///
/// Diagnostics are data, not logging: they are part of the JSON output and the
/// `--agent` view, and they never abort the run.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Stable machine-readable identifier, for example `parse_failed`.
    pub code: String,
    /// How much information was lost.
    pub severity: Severity,
    /// Repo-relative, slash-separated path the diagnostic is about, when it has one.
    pub file: Option<String>,
    /// Location inside `file`, when the diagnostic is about a location.
    pub span: Option<Span>,
    /// One sentence for a human, no trailing period.
    pub message: String,
}
