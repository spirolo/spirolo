//! The SARIF 2.1 surface: findings as a static-analysis run GitHub code scanning and IDE
//! problem panels consume and deduplicate themselves.
//!
//! One `rules` entry per rule that produced a finding, carrying that rule's explain as its
//! help text and its severity as the descriptor's default level; one `results` entry per
//! finding, with its primary location, its related locations (each label a message), and its
//! severity mapped to a SARIF level. A `could not decide` outcome is honesty, not a code
//! scanning result, so it never appears. We emit no `partialFingerprints`: the upload action
//! computes `primaryLocationLineHash` itself when a run omits it, and a hash of our own would
//! only fight its deduplication. Keys are sorted at every depth and the document carries no
//! clock, so two runs over the same findings are byte-identical.

use std::collections::BTreeMap;

use serde_json::json;
use spirolo_rules::{Finding, Outcome, Severity};

/// Where the tool documents itself, for the driver's `informationUri`: the workspace's
/// `repository`, so the URL is declared once, in the root manifest.
const INFORMATION_URI: &str = env!("CARGO_PKG_REPOSITORY");

/// The SARIF 2.1 schema the document validates against.
const SCHEMA_URI: &str = "https://json.schemastore.org/sarif-2.1.0.json";

/// Serialize findings as a SARIF 2.1 document, with `driver.version` set to `version` (the
/// binary passes its own crate version so this crate stays free of it).
///
/// Only [`Outcome::Finding`] findings become results; a [`Outcome::CouldNotDecide`] or an
/// [`Outcome::Listed`] row is not a code-scanning result. The `rules` array lists the rules those results reference, once
/// each, in id order, each with its explain as help text — so a rule with only undecided
/// findings contributes neither a result nor a descriptor. Keys are sorted at every depth by
/// [`crate::canonicalise`] and nothing depends on the clock, the machine or the working
/// directory, so the guard `two_runs_are_byte_equal` holds.
///
/// # Panics
///
/// Never in practice: the document is a plain tree of owned values, and `serde_json` only
/// fails to serialize a value whose `Serialize` implementation errors, which none here has.
pub fn to_sarif(findings: &[Finding], version: &str) -> String {
    let decided: Vec<&Finding> = findings
        .iter()
        .filter(|f| f.outcome == Outcome::Finding)
        .collect();

    let mut document = json!({
        "$schema": SCHEMA_URI,
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "spirolo",
                    "informationUri": INFORMATION_URI,
                    "version": version,
                    "rules": rules_json(&decided),
                }
            },
            "results": decided.iter().map(|f| result_json(f)).collect::<Vec<_>>(),
        }],
    });
    crate::canonicalise(&mut document);

    let mut json = serde_json::to_string_pretty(&document).expect("a JSON value re-serializes");
    json.push('\n');
    json
}

/// The `rules` array: one descriptor per rule that a result references, in id order, each
/// with its explain as help text and its severity as the default level. The first finding of
/// a rule (findings arrive sorted by rule) supplies both, so the descriptor is deterministic.
fn rules_json(decided: &[&Finding]) -> Vec<serde_json::Value> {
    let mut first: BTreeMap<&str, &Finding> = BTreeMap::new();
    for finding in decided {
        first.entry(finding.rule).or_insert(finding);
    }
    first
        .values()
        .map(|finding| {
            json!({
                "id": finding.rule,
                "name": finding.rule,
                "shortDescription": { "text": finding.rule },
                "help": { "text": finding.explain },
                "helpUri": INFORMATION_URI,
                "defaultConfiguration": { "level": level(finding.severity) },
            })
        })
        .collect()
}

/// One `results` entry: the finding's rule, level and message, its primary location, its
/// related locations, each carrying its label as the message, and its rule status in
/// `properties.ruleStatus`.
///
/// A finding with no file — the project-wide ratio row — carries an empty `locations` array
/// rather than a physical location whose `region` would violate the schema's one-based line
/// (SARIF's `startLine` minimum is 1), so the document validates.
fn result_json(finding: &Finding) -> serde_json::Value {
    let mut result = json!({
        "ruleId": finding.rule,
        "level": level(finding.severity),
        "message": { "text": finding.message },
        "locations": physical_location(&finding.location).into_iter().collect::<Vec<_>>(),
        "relatedLocations": finding.related.iter().filter_map(|related| {
            let mut location = physical_location(&related.location)?;
            location
                .as_object_mut()
                .expect("a physical location is an object")
                .insert("message".to_owned(), json!({ "text": related.label }));
            Some(location)
        }).collect::<Vec<_>>(),
    });
    // A `properties` bag carrying the same values the JSON surface renders, so a
    // code-scanning consumer reads them without a schema of its own: `ruleStatus` always,
    // `test` on a finding anchored at a test, `proposal` under `--propose` only.
    let mut properties = serde_json::Map::new();
    properties.insert(
        "ruleStatus".to_owned(),
        json!(crate::findings::rule_status(finding.rule)),
    );
    if let Some(test) = &finding.test {
        properties.insert("test".to_owned(), crate::findings::test_json(test));
    }
    if let Some(proposal) = &finding.proposal {
        properties.insert(
            "proposal".to_owned(),
            crate::proposal::proposal_json(proposal),
        );
    }
    result
        .as_object_mut()
        .expect("a result is an object")
        .insert("properties".to_owned(), properties.into());
    result
}

/// A SARIF location object — an artifact location (the repo-relative uri) and a one-based
/// region — or `None` for a file-less finding, whose location the caller omits.
fn physical_location(location: &spirolo_rules::Location) -> Option<serde_json::Value> {
    if location.file.is_empty() {
        return None;
    }
    Some(json!({
        "physicalLocation": {
            "artifactLocation": { "uri": location.file },
            "region": {
                "startLine": location.line,
                "startColumn": location.column,
            }
        }
    }))
}

/// The SARIF level a finding's severity maps to: `error`, `warning`, `note`.
const fn level(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warn => "warning",
        Severity::Info => "note",
    }
}
