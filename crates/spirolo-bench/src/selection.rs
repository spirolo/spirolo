//! Frozen corpus membership, read before touching entry truth or clones.
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Release-policy membership of one manifest entry.
#[derive(Debug, Clone, Default)]
pub struct Membership {
    /// `oss` or `private`; absent means a legacy regression entry.
    pub source: String,
    /// `dev` or `held-out`; only explicit `dev` is pooled.
    pub set: String,
}

/// Read the manifest's basic-string membership fields, ignoring notes and other fields.
/// The repository manifest uses one field per line; multiline notes are skipped entirely.
///
/// # Errors
/// Returns unreadable metadata or malformed membership strings as errors.
pub fn memberships(root: &Path) -> Result<BTreeMap<String, Membership>, String> {
    let path = metadata_path(root, "manifest.toml");
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut entries = BTreeMap::new();
    let mut name = None;
    let mut member = Membership::default();
    let mut in_notes = false;
    for line in text.lines() {
        let line = line.trim();
        if in_notes {
            if line == "\"\"\"" {
                in_notes = false;
            }
            continue;
        }
        if line == "[[entry]]" {
            if let Some(name) = name.take() {
                entries.insert(name, member.clone());
            }
            member = Membership::default();
        } else if line.starts_with("notes = \"\"\"") {
            in_notes = true;
        } else if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            if !["name", "source", "set"].contains(&key) {
                continue;
            }
            let value: String = serde_json::from_str(value.trim())
                .map_err(|e| format!("{}: {key}: {e}", path.display()))?;
            match key {
                "name" => name = Some(value),
                "source" => member.source = value,
                "set" => member.set = value,
                _ => {}
            }
        }
    }
    if let Some(name) = name {
        entries.insert(name, member);
    }
    Ok(entries)
}

/// Held-out names from both the frozen manifest and its held-out registry.
///
/// # Errors
/// Refuses unreadable or malformed metadata so a corrupt guard cannot expose an entry.
pub fn held_out_names(root: &Path) -> Result<BTreeSet<String>, String> {
    let mut names: BTreeSet<_> = memberships(root)?
        .into_iter()
        .filter(|(_, e)| e.set == "held-out")
        .map(|(name, _)| name)
        .collect();
    let path = metadata_path(root, "select/held-out.json");
    if path.exists() {
        let doc: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        let entries = doc["held_out"]
            .as_array()
            .ok_or("held-out registry has no held_out array")?;
        for entry in entries {
            names.insert(
                entry["name"]
                    .as_str()
                    .ok_or("held-out entry has no name")?
                    .to_owned(),
            );
        }
    }
    Ok(names)
}

fn metadata_path(root: &Path, file: &str) -> std::path::PathBuf {
    let local = root.join(file);
    if local.exists() || root.join("manifest.toml").exists() {
        local
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../corpus")
            .join(file)
    }
}
