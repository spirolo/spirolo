//! Rendering a [`Proposal`] the same way in every surface: a small object for the JSON
//! `proposal` key and the SARIF `properties.proposal` bag, and a one-line string for the
//! human and agent views. Kept in one place so the four surfaces never drift.

use serde_json::json;
use spirolo_rules::Proposal;

/// The proposal as a JSON object: an `action` token, plus the detail the action carries —
/// `target` for a merge (the member to merge into, with `from` naming the members merged
/// away) or a delete that names what to remove, and `reason` for a keep. A bare delete is just its action. This object is both the JSON
/// `proposal` value and the SARIF `properties.proposal` bag, so the two never disagree.
pub fn proposal_json(proposal: &Proposal) -> serde_json::Value {
    match proposal {
        Proposal::Merge { from, into } => {
            json!({ "action": "merge", "from": from, "target": into })
        }
        Proposal::Delete { what: Some(target) } => {
            json!({ "action": "delete", "target": target })
        }
        Proposal::Delete { what: None } => json!({ "action": "delete" }),
        Proposal::Keep { reason } => json!({ "action": "keep", "reason": reason }),
    }
}

/// The proposal as one related-location-free line for the human and agent views: the action
/// and what it points at, no second location and no decoration.
pub fn proposal_line(proposal: &Proposal) -> String {
    match proposal {
        Proposal::Merge { from, into } => format!("merge {from} into {into}"),
        Proposal::Delete { what: Some(target) } => format!("delete {target}"),
        Proposal::Delete { what: None } => "delete".to_owned(),
        Proposal::Keep { reason } => format!("keep — {reason}"),
    }
}
