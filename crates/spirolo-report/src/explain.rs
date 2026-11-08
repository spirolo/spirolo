//! `spirolo explain`: one rule, or one finding by its stable id, with the rule's measured
//! status.

use std::fmt::Write as _;

use serde_json::json;
use spirolo_rules::{status, Finding, Location, Rule};

/// A finding as `explain <id>` prints it: the rule and location, the message, one line per
/// related location and per evidence value, the explain text, then the status line.
pub fn explain_human(finding: &Finding) -> String {
    let mut out = format!(
        "{}  {}\n{}\n",
        finding.rule,
        position(&finding.location),
        finding.message
    );
    for related in &finding.related {
        let _ = writeln!(
            out,
            "related  {}  {}",
            related.label,
            position(&related.location)
        );
    }
    for evidence in &finding.evidence {
        let _ = writeln!(out, "evidence  {}  {}", evidence.key, evidence.value);
    }
    let _ = writeln!(out, "explain  {}", finding.explain);
    let _ = writeln!(out, "{}", status::status_line(finding.rule));
    out
}

/// A location as `file:line:column`, or `project` for a file-less project-wide row.
fn position(location: &Location) -> String {
    if location.file.is_empty() {
        "project".to_owned()
    } else {
        format!("{}:{}:{}", location.file, location.line, location.column)
    }
}

/// A rule as `explain <rule> --format json` prints it: `{rule, family, default_severity,
/// summary, status}`.
///
/// `status` is the rule's row in the status table plus the fields of the human Status block: `tier`, `decided`, `measured_on` (always present), the tier
/// `floors`, the `proven_bar`, the `advisory` sentence and the `docs` page (or `null`).
pub fn explain_rule_json(rule: &dyn Rule) -> String {
    let row = status::status(rule.name());
    let mut block = json!(row);
    let object = block
        .as_object_mut()
        .expect("a status row serializes to a JSON object");
    object.insert("measured_on".to_owned(), json!(row.measured_on));
    object.insert("tier".to_owned(), json!(tier_name(row.tier())));
    object.insert("decided".to_owned(), json!(decided(row)));
    object.insert(
        "floors".to_owned(),
        json!({
            "decided": status::SHOWN_DECIDED_FLOOR,
            "precision": precision_floor(),
        }),
    );
    object.insert("proven_bar".to_owned(), json!(PROVEN_BAR));
    object.insert("advisory".to_owned(), json!(ADVISORY));
    object.insert("docs".to_owned(), json!(docs_page(rule.name())));
    let mut document = json!({
        "rule": rule.name(),
        "family": rule.family().name(),
        "default_severity": status::default_severity(rule).name(),
        "summary": rule.summary(),
        "status": block,
    });
    pretty(&mut document)
}

/// A finding as `explain <id> --format json` prints it.
///
/// The object `check --format json` renders for it, plus `status` (the rule's row in the status table) and
/// `editor_severity` (`1`–`4`, or `null` for a `listed` row).
pub fn explain_json(finding: &Finding) -> String {
    let mut document = crate::findings::finding_json(finding);
    let object = document
        .as_object_mut()
        .expect("a finding serializes to a JSON object");
    object.insert("status".to_owned(), json!(status::status(finding.rule)));
    object.insert(
        "editor_severity".to_owned(),
        json!(crate::editor_severity(finding).map(crate::EditorSeverity::code)),
    );
    pretty(&mut document)
}

/// A rule as `explain <rule>` prints it: the name with its family and default severity,
/// the summary, then the [Status block](status_block).
pub fn explain_rule_human(rule: &dyn Rule) -> String {
    format!(
        "{} ({}, default {})\n{}\n{}",
        rule.name(),
        rule.family().name(),
        status::default_severity(rule).name(),
        rule.summary(),
        status_block(rule.name()),
    )
}

/// The bar a rule's pooled sample must meet to be proven, as `docs/rule-visits/README.md`
/// pre-registers it.
const PROVEN_BAR: &str = "Wilson lower bound ≥ 0.9 on the pooled dev sample: \
     ≥35 decided with 0 false, ≥53 with 1, ≥69 with 2 (docs/rule-visits/README.md)";

/// What advisory means, whichever tier the rule is in.
const ADVISORY: &str = "an unproven rule reports at info and never fails the build unless \
     .spirolorc raises its severity; a DROPPED or FAIL rule never gates";

/// The rules with a page under `docs/rules/`, listed by the build script.
const RULE_DOCS: &[&str] = include!(concat!(env!("OUT_DIR"), "/rule_docs.rs"));

/// The repo-relative path of the rule's page, when the repo has one.
fn docs_page(rule: &str) -> Option<String> {
    RULE_DOCS
        .contains(&rule)
        .then(|| format!("docs/rules/{rule}.md"))
}

/// Decided findings: truths plus falses.
const fn decided(row: &status::RuleStatus) -> u32 {
    row.truths + row.falses
}

/// [`status::SHOWN_PRECISION_FLOOR`] as a number: 0.80.
fn precision_floor() -> f64 {
    let (numerator, denominator) = status::SHOWN_PRECISION_FLOOR;
    f64::from(numerator) / f64::from(denominator)
}

/// The tier's lower-case name, as the JSON `tier` field spells it.
const fn tier_name(tier: status::Tier) -> &'static str {
    match tier {
        status::Tier::Proven => "proven",
        status::Tier::Advisory => "advisory",
        status::Tier::Experimental => "experimental",
        status::Tier::Hidden => "hidden",
    }
}

/// The Status block: verdict, tier, shown precision with its Wilson interval, truths and
/// falses, the corpus entries, the proven bar, what advisory means and the rule's page.
fn status_block(rule: &str) -> String {
    let row = status::status(rule);
    let floors = format!(
        "≥{} decided and shown precision ≥{:.2}",
        status::SHOWN_DECIDED_FLOOR,
        precision_floor()
    );
    let mut out = String::from("Status\n");
    let mut verdict = row.verdict.name().to_owned();
    if let Some(carried) = &row.carried {
        let _ = write!(verdict, ", carried to {carried}");
    }
    let _ = writeln!(out, "  verdict      {verdict}");
    let tier = match row.tier() {
        status::Tier::Proven => {
            "proven: PASS, shown by default at its declared severity".to_owned()
        }
        status::Tier::Advisory => format!("advisory: shown by default ({floors})"),
        status::Tier::Experimental => {
            format!("experimental: shown only with --all (the default view needs {floors})")
        }
        status::Tier::Hidden => {
            "hidden: DROPPED or FAIL, shown only with --all, never gates".to_owned()
        }
    };
    let _ = writeln!(out, "  tier         {tier}");
    let decided = decided(row);
    let mut precision = if decided == 0 {
        "none decided".to_owned()
    } else {
        format!(
            "{}/{decided} = {:.3}",
            row.truths,
            f64::from(row.truths) / f64::from(decided)
        )
    };
    if let Some([low, high]) = row.interval {
        let _ = write!(precision, ", Wilson 95% [{low:.3}, {high:.3}]");
    }
    let _ = writeln!(out, "  precision    {precision}");
    let _ = writeln!(
        out,
        "  decided      {} true, {} false",
        row.truths, row.falses
    );
    let measured = if row.measured_on.is_empty() {
        "no corpus entry recorded".to_owned()
    } else {
        row.measured_on.join(", ")
    };
    let _ = writeln!(out, "  measured on  {measured}");
    let _ = writeln!(out, "  proven bar   {PROVEN_BAR}");
    let _ = writeln!(out, "  advisory     {ADVISORY}");
    if let Some(page) = docs_page(rule) {
        let _ = writeln!(out, "  docs         {page}");
    }
    out
}

/// Canonical key order, pretty-printed, one trailing newline: the shape of every JSON
/// document the binary prints.
fn pretty(document: &mut serde_json::Value) -> String {
    crate::canonicalise(document);
    let mut json = serde_json::to_string_pretty(document).expect("a JSON value re-serializes");
    json.push('\n');
    json
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_proven_bar_quotes_the_pre_registered_bars() {
        let readme = include_str!("../../../docs/rule-visits/README.md").replace("\n  ", " ");
        let (bar, source) = PROVEN_BAR.split_once(" (").unwrap();
        assert_eq!(source, "docs/rule-visits/README.md)");
        assert!(readme.contains(&format!("**Backed** = {bar}.")), "{bar}");
    }

    #[test]
    fn every_linked_page_exists_and_every_rule_page_is_linked() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for rule in spirolo_rules::registry() {
            let path = format!("docs/rules/{}.md", rule.name());
            let exists = root.join(&path).is_file();
            assert_eq!(
                docs_page(rule.name()),
                exists.then_some(path),
                "{}",
                rule.name()
            );
        }
        assert_eq!(docs_page("no-such-rule"), None);
    }
}
