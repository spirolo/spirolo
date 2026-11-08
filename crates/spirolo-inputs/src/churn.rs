//! The churn reader: `git log --numstat --format=` output, the churn input `untested-hotspot`
//! reads (PRD 0005, "Churn input").
//!
//! Each numstat line is `added<TAB>deleted<TAB>path`. Lines are summed per path into one
//! [`Churn`] row. The binary never runs git; the user supplies this file.

use std::collections::BTreeMap;

use spirolo_model::{Churn, Diagnostic, Severity};

/// What one churn input yields: one row per path, and what could not be read.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChurnTable {
    /// One row per path, sorted by path.
    pub churn: Vec<Churn>,
    /// Lines the reader could not use.
    pub diagnostics: Vec<Diagnostic>,
}

/// Read `git log --numstat` output. `path` names the input in diagnostics.
///
/// Commit headers and messages (`git log --numstat` without `--format=`) and blank lines are
/// skipped. Any other line that is not `added<TAB>deleted<TAB>path` is a
/// `churn_line_unreadable` diagnostic, and the lines around it still count.
#[must_use]
pub fn read_numstat(path: &str, bytes: &[u8]) -> ChurnTable {
    let text = String::from_utf8_lossy(bytes);
    let mut sums: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    let mut diagnostics = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if is_header(line) {
            continue;
        }
        let Some((added, deleted, file)) = numstat_line(line) else {
            diagnostics.push(Diagnostic {
                code: "churn_line_unreadable".to_owned(),
                severity: Severity::Warning,
                file: Some(path.to_owned()),
                span: None,
                message: format!(
                    "line {} is not `added<TAB>deleted<TAB>path`: `{line}`",
                    index + 1
                ),
            });
            continue;
        };
        let sum = sums.entry(new_path(file)).or_default();
        sum.0 += added;
        sum.1 += deleted;
    }
    ChurnTable {
        churn: sums
            .into_iter()
            .map(|(path, (added, deleted))| Churn {
                path,
                added,
                deleted,
            })
            .collect(),
        diagnostics,
    }
}

/// A line `git log --numstat` writes around the numstat lines: blank, a commit header, or an
/// indented message line.
fn is_header(line: &str) -> bool {
    line.trim().is_empty()
        || line.starts_with("    ")
        || ["commit ", "Merge:", "Author:", "Date:"]
            .iter()
            .any(|header| line.starts_with(header))
}

/// The counts and the path of one numstat line, or `None` when it is not one.
fn numstat_line(line: &str) -> Option<(u64, u64, &str)> {
    let mut fields = line.splitn(3, '\t');
    let added = count(fields.next()?)?;
    let deleted = count(fields.next()?)?;
    let file = fields.next().filter(|file| !file.is_empty())?;
    Some((added, deleted, file))
}

/// A numstat count: a number, or `-` for a binary file, which adds no lines.
fn count(field: &str) -> Option<u64> {
    if field == "-" {
        Some(0)
    } else {
        field.parse().ok()
    }
}

/// The path a numstat line counts toward: the new side of a rename. Git writes a rename as
/// `prefix{old => new}suffix` when the paths share a prefix or suffix, and as `old => new`
/// otherwise; an empty side (`a/{b => }/c`) leaves a doubled slash, which is collapsed.
fn new_path(file: &str) -> String {
    if let (Some(open), Some(close)) = (file.find('{'), file.rfind('}')) {
        if let Some((_, new)) = file[open + 1..close].split_once(" => ") {
            let joined = format!("{}{new}{}", &file[..open], &file[close + 1..]);
            return joined.replace("//", "/");
        }
    }
    match file.split_once(" => ") {
        Some((_, new)) => new.to_owned(),
        None => file.to_owned(),
    }
}
