//! GitHub workflow-command annotations over the same decided findings as SARIF.

use std::fmt::Write as _;

use spirolo_rules::{Finding, Outcome, Severity};

// Workflow commands allow ten annotations of each level (notice, warning, error) per step:
// https://github.com/orgs/community/discussions/26680
const LEVEL_LIMIT: usize = 10;

/// Render findings as GitHub annotations, followed by a plain log summary.
///
/// The command follows the finding's severity: `::notice` for info (an advisory row never
/// prints a yellow warning on a run that exits 0), `::warning` for warn, `::error` for error.
/// Each title is the rule and its rule status, as `dead-skip (UNADJUDICATED)`.
/// Project-wide findings omit the optional file and line properties; file-level findings
/// (line 0) name the file and omit the line.
/// Listing and undecided outcomes do not claim an annotation.
/// At most ten annotations of each level are emitted in the input's deterministic order; the
/// summary counts every decided finding, including those omitted to respect the per-step limit.
pub fn render_github_annotations(findings: &[Finding]) -> String {
    let mut out = String::new();
    let mut emitted = 0;
    let mut per_level = [0_usize; 3];
    let decided: Vec<_> = findings
        .iter()
        .filter(|f| f.outcome == Outcome::Finding)
        .collect();
    for finding in &decided {
        let (level, command) = match finding.severity {
            Severity::Info => (0, "notice"),
            Severity::Warn => (1, "warning"),
            Severity::Error => (2, "error"),
        };
        if per_level[level] == LEVEL_LIMIT {
            continue;
        }
        per_level[level] += 1;
        let _ = write!(out, "::{command} ");
        if !finding.location.file.is_empty() {
            let _ = write!(out, "file={},", escape_property(&finding.location.file));
            // Line 0 is a file-level anchor; GitHub's lines are 1-based, so leave it out.
            if finding.location.line > 0 {
                let _ = write!(out, "line={},", finding.location.line);
            }
        }
        let _ = writeln!(
            out,
            "title={}::{}",
            escape_property(&format!(
                "{} ({})",
                finding.rule,
                crate::findings::rule_status(finding.rule)
            )),
            escape_data(&finding.message)
        );
        emitted += 1;
    }
    let _ = writeln!(
        out,
        "spirolo: {} findings · {emitted} annotations · {} omitted",
        decided.len(),
        decided.len() - emitted
    );
    out
}

/// GitHub's toolkit escapes percent first, so literal encoded text round-trips once.
/// After the command delimiter, `::` is ordinary message text.
fn escape_data(text: &str) -> String {
    text.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

/// Property commas separate fields and colons can terminate the command header.
fn escape_property(text: &str) -> String {
    escape_data(text).replace(':', "%3A").replace(',', "%2C")
}

#[cfg(test)]
mod tests {
    #[test]
    fn messages_escape_percent_and_newlines_without_creating_commands() {
        assert_eq!(
            super::escape_data("50%\r\n::warning, next%0A"),
            "50%25%0D%0A::warning, next%250A"
        );
    }

    #[test]
    fn properties_escape_command_delimiters_and_preserve_literal_encodings() {
        assert_eq!(
            super::escape_property("src/50%,::name\r\n%3A.test.ts"),
            "src/50%25%2C%3A%3Aname%0D%0A%253A.test.ts"
        );
    }
}
