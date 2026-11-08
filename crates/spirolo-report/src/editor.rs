//! How loud a finding may be in an editor: PRD 0006's Editor severity table.
//!
//! The one mapping from a finding to an LSP severity. The CLI applies its release severity policy first;
//! this caps it by the rule's measured status, so only a proven rule (PASS)
//! renders as an Error or a Warning. An unproven rule the default view
//! [shows](status::shown_by_default) (the advisory tier) lists its `info` findings as
//! Information, so they reach the Problems panel; the `info` findings of rules shown only under
//! `--all` (experimental, FAIL, DROPPED) stay Hints.

use spirolo_rules::{status, Finding, Outcome, Severity};

/// An LSP `DiagnosticSeverity`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EditorSeverity {
    /// A red squiggle, reserved for a proven rule's `error` finding.
    Error,
    /// A yellow squiggle, reserved for a proven rule's `warn` finding.
    Warning,
    /// A proven rule's `info` finding, an unproven rule's `error` or `warn` one, or the `info`
    /// finding of an advisory rule (shown by default).
    Information,
    /// The `info` finding of a rule shown only under `--all`, or any finding the rule could
    /// not decide.
    Hint,
}

impl EditorSeverity {
    /// The protocol's number for this severity, `1` (Error) to `4` (Hint).
    pub const fn code(self) -> u8 {
        match self {
            Self::Error => 1,
            Self::Warning => 2,
            Self::Information => 3,
            Self::Hint => 4,
        }
    }
}

/// The editor severity of `finding`, or `None` for a `listed` row, which is never
/// published as a diagnostic.
pub fn editor_severity(finding: &Finding) -> Option<EditorSeverity> {
    match finding.outcome {
        Outcome::Listed => None,
        Outcome::CouldNotDecide => Some(EditorSeverity::Hint),
        Outcome::Finding if status::is_proven(finding.rule) => Some(match finding.severity {
            Severity::Error => EditorSeverity::Error,
            Severity::Warn => EditorSeverity::Warning,
            Severity::Info => EditorSeverity::Information,
        }),
        Outcome::Finding => Some(match finding.severity {
            Severity::Error | Severity::Warn => EditorSeverity::Information,
            Severity::Info if status::shown_by_default(finding.rule) => EditorSeverity::Information,
            Severity::Info => EditorSeverity::Hint,
        }),
    }
}
