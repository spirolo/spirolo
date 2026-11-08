//! Status fixtures captured from the real CLI, independent of the policy oracle.

use std::sync::OnceLock;

/// The document printed by `spirolo rule-status --format json` before this fix at 5b9be0a.
pub fn table() -> &'static spirolo_rules::status::Table {
    static TABLE: OnceLock<spirolo_rules::status::Table> = OnceLock::new();
    TABLE.get_or_init(|| {
        spirolo_rules::status::parse(include_str!("../fixtures/rule-status.json")).unwrap()
    })
}

/// Render against the captured table, leaving the embedding and other tests alone.
pub fn with<R>(run: impl FnOnce() -> R) -> R {
    spirolo_rules::status::with_table(table(), run)
}
