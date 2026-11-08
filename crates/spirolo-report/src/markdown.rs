//! A deterministic GitHub job summary over the embedded status and check findings.

use std::fmt::Write as _;

use spirolo_rules::{status, Finding};

/// Render a job summary: measured rule status, proven findings, then collapsed info.
///
/// Findings of unproven rules stay in the info section even under a severity override.
/// The caller supplies the same (possibly baselined) findings as the JSON surface.
pub fn render_check_markdown(findings: &[Finding]) -> String {
    let mut out = String::from("# Spirolo\n\n## Rule status\n\n");
    let entries = &status::table().entries;
    let _ = writeln!(
        out,
        "<details>\n<summary>Measured on {} projects</summary>\n\n{}\n\n</details>\n",
        entries.len(),
        cell(&entries.join(", "))
    );
    out.push_str("| Rule | Status | Precision | 95% interval |\n| --- | --- | --- | --- |\n");
    for (rule, row) in &status::table().rules {
        let verdict = row.carried.as_ref().map_or_else(
            || row.verdict.name().to_owned(),
            |milestone| format!("{}, carried to {milestone}", row.verdict.name()),
        );
        let precision = row
            .precision
            .map_or_else(|| "—".into(), |p| format!("{p:.3}"));
        let interval = row.interval.map_or_else(
            || "—".into(),
            |[low, high]| format!("[{low:.3}, {high:.3}]"),
        );
        let _ = writeln!(
            out,
            "| {} | {} | {precision} | {interval} |",
            cell(rule),
            cell(&verdict)
        );
    }

    out.push_str("\n## Proven findings\n\n");
    let (proven, info): (Vec<_>, Vec<_>) = findings.iter().partition(|f| status::is_proven(f.rule));
    write_findings(&mut out, &proven);
    let _ = write!(
        out,
        "\n<details>\n<summary>Info ({})</summary>\n\n",
        info.len()
    );
    write_rule_counts(&mut out, &info);
    let shown = info.len().min(INFO_ROW_CAP);
    write_findings(&mut out, &info[..shown]);
    if info.len() > shown {
        let _ = writeln!(
            out,
            "\n{} more info rows not shown; `spirolo check --format json` lists every one.",
            info.len() - shown
        );
    }
    out.push_str("\n</details>\n");
    out
}

/// The most info rows the summary prints. GitHub refuses a step summary over 1 MiB, and a
/// large monorepo has thousands of advisory rows: the per-rule counts above the table keep
/// the whole picture, the table a readable sample, and the JSON every row.
const INFO_ROW_CAP: usize = 500;

/// One `rule: count` line per rule among `findings`, in rule order with its rule status, so a
/// capped table still says how many rows each rule produced.
fn write_rule_counts(out: &mut String, findings: &[&Finding]) {
    if findings.is_empty() {
        return;
    }
    let mut counts: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for finding in findings {
        *counts.entry(finding.rule).or_default() += 1;
    }
    out.push_str("| Rule | Status | Rows |\n| --- | --- | ---: |\n");
    for (rule, count) in counts {
        let _ = writeln!(
            out,
            "| {} | {} | {count} |",
            cell(rule),
            crate::findings::rule_status(rule)
        );
    }
    out.push('\n');
}

/// The findings table: one row per finding, its rule status beside its rule.
fn write_findings(out: &mut String, findings: &[&Finding]) {
    if findings.is_empty() {
        out.push_str("None.\n");
        return;
    }
    out.push_str(
        "| Rule | Status | Severity | Outcome | Location | Message |\n| --- | --- | --- | --- | --- | --- |\n",
    );
    for f in findings {
        let location = if f.location.file.is_empty() {
            "project".to_owned()
        } else {
            format!("{}:{}", f.location.file, f.location.line)
        };
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} |",
            cell(f.rule),
            crate::findings::rule_status(f.rule),
            f.severity.name(),
            crate::findings::outcome_token(f.outcome),
            cell(&location),
            cell(&f.message),
        );
    }
}

/// Keep source-controlled text literal and within one Markdown table cell.
fn cell(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('|', "&#124;")
        .replace('`', "&#96;")
        .replace('[', "&#91;")
        .replace(']', "&#93;")
        .replace('\\', "&#92;")
        .replace('*', "&#42;")
        .replace('_', "&#95;")
        .replace("\r\n", "<br>")
        .replace(['\r', '\n'], "<br>")
}

#[cfg(test)]
mod tests {
    #[test]
    fn table_cells_keep_literal_text_inside_one_cell() {
        assert_eq!(
            super::cell("50% | `test` [x] \\ *bold* _name_ <details>\r\nnext::line"),
            "50% &#124; &#96;test&#96; &#91;x&#93; &#92; &#42;bold&#42; &#95;name&#95; &lt;details&gt;<br>next::line"
        );
    }
}
