//! Explicit severities keep rule-focused fixtures independent of corpus adjudications.

use std::path::Path;

/// These tests exercise findings and reports; the release defaults are tested separately
/// in `severity_policy`. Opt their fixtures into info so a future PASS cannot change them.
pub fn info(root: &Path) {
    let rules: serde_json::Map<String, serde_json::Value> = spirolo_rules::registry()
        .iter()
        .map(|rule| (rule.name().into(), serde_json::Value::from("info")))
        .collect();
    std::fs::write(
        root.join(".spirolorc.jsonc"),
        serde_json::json!({"rules": rules}).to_string(),
    )
    .unwrap();
}
