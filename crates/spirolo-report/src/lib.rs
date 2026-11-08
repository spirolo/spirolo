//! Output surfaces over the model: the versioned JSON schema, and the human and agent
//! views, which are views *over that JSON* rather than second renderings of the model.
//!
//! Two promises this crate keeps, because consumers (and snapshot tests) depend on them:
//! the JSON has sorted keys at every depth, and it carries no clock — the timing of a
//! run belongs to the human line only.

mod annotations;
mod baseline;
mod editor;
mod explain;
mod findings;
mod human;
mod impact;
mod markdown;
mod proposal;
mod sarif;
pub mod score;
mod sidebar;

use spirolo_model::Model;

pub use crate::annotations::render_github_annotations;
pub use crate::baseline::{Baseline, Diff, Fallen};
pub use crate::editor::{editor_severity, EditorSeverity};
pub use crate::explain::{explain_human, explain_json, explain_rule_human, explain_rule_json};
pub use crate::findings::{
    baselined_exit_code, render_check_agent, render_check_human, render_check_human_baselined,
    render_check_human_baselined_with_options, render_check_human_with_options, tally, tier_label,
    to_check_json, HumanOptions, Tally,
};
pub use crate::human::{render_agent, render_human, summary_line};
pub use crate::impact::{
    hotspots_to_json, impact_to_json, render_hotspots_human, render_impact_human,
    render_impact_print,
};
pub use crate::markdown::render_check_markdown;
pub use crate::sarif::to_sarif;
pub use crate::sidebar::{
    sidebar_file, sidebar_rules, sidebar_tree, Counts, Handlers, Node, Project, Row, RuleFile,
    RuleNode, RuleRow, RulesScope, SidebarFile, SidebarRules, SidebarTree,
};

/// Version of the JSON document [`to_json`] emits.
///
/// Bumped whenever a key is removed or changes meaning; adding a key does not bump it,
/// so consumers should ignore keys they do not know. Version 2: a gate's `forbid_only` is
/// a `GateFlag` token (`"true"`, `"false"`, `"ci_conditional"`) rather than a boolean.
pub const SCHEMA_VERSION: u32 = 2;

/// Serialize a model as the canonical JSON document.
///
/// Keys are sorted at every depth by [`canonicalise`], not by luck of the feature set.
/// `serde_json`'s default map is a `BTreeMap`, but any crate in the dependency graph
/// enabling `serde_json/preserve_order` (the resolver does) unifies the feature
/// workspace-wide and swaps in an insertion-order `IndexMap`; sorting here holds the
/// promise either way. Tables are in the order the model sorted them, and nothing in the
/// output depends on the clock, the machine or the working directory — two runs over the
/// same tree are byte-equal. `json_keys_are_sorted_at_every_depth` is the guard.
///
/// # Panics
///
/// Never in practice: the model is a plain tree of owned values, and the only way
/// `serde_json` fails on one is a `Serialize` implementation that errors, which none of
/// the model's types has.
pub fn to_json(model: &Model) -> String {
    let mut document = serde_json::to_value(model).expect("the model serializes infallibly");
    document
        .as_object_mut()
        .expect("a model serializes to a JSON object")
        .insert(
            "schema_version".to_owned(),
            serde_json::json!(SCHEMA_VERSION),
        );
    canonicalise(&mut document);

    let mut json = serde_json::to_string_pretty(&document).expect("a JSON value re-serializes");
    json.push('\n');
    json
}

/// Sort every object's keys, at every depth, so the document is byte-identical whatever
/// map `serde_json` was compiled with. Rebuilding an already-sorted `BTreeMap` is a
/// no-op; rebuilding an `IndexMap` from sorted entries makes it iterate in sorted order.
pub(crate) fn canonicalise(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for (_, child) in map.iter_mut() {
                canonicalise(child);
            }
            let mut entries: Vec<_> = std::mem::take(map).into_iter().collect();
            entries.sort_by(|(a, _), (b, _)| a.cmp(b));
            *map = entries.into_iter().collect();
        }
        serde_json::Value::Array(items) => {
            for item in items {
                canonicalise(item);
            }
        }
        _ => {}
    }
}
