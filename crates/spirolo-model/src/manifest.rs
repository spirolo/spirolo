//! `package.json` read as data: the package's directory, name and entry paths.
//!
//! The walk hands every `package.json` it is allowed to see to this reader, so the set
//! of packages the model knows follows the same `.gitignore` and skipped directories as
//! every other file — no second scan of the tree. The name feeds the workspace alias
//! table in [`crate::resolve`]; the directory is the package boundary [`crate::pairing`]
//! and [`crate::workspace`] draw. A manifest is never a `Module` row.

use crate::{Diagnostic, GateTool, Severity};

/// One `package.json` the walk found.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Manifest {
    /// Repo-relative, slash-separated directory the manifest sits in; `""` at the root.
    pub dir: String,
    /// The `name` field, when the manifest declares one.
    pub name: Option<String>,
    /// Every path the package publishes as an entry, as written relative to `dir`: the
    /// `main`, `module`, `types` and `source` fields and every string in `exports`.
    pub entries: Vec<String>,
    /// The runner directly declared in dependencies, when exactly one family is
    /// present. Configs and explicit imports take precedence over this fallback.
    pub runner: Option<GateTool>,
    /// Jest's declared major when a literal version or caret/tilde range fixes it.
    pub jest_major: Option<u32>,
    /// The commands of the `scripts` block, in source order.
    pub scripts: Vec<String>,
    /// Script names and commands, used to follow CI invocations and the primary `test`
    /// command (whose explicit config takes precedence over defaults) without executing them.
    pub named_scripts: std::collections::BTreeMap<String, String>,
}

/// Read one manifest. A file that is not valid JSON is a `manifest_invalid` diagnostic
/// and no manifest: the package cannot be named, so nothing aliases to it, but the
/// directory is still not silently guessed to be a package.
pub fn read(path: &str, source: &str) -> Result<Manifest, Diagnostic> {
    match serde_json::from_str::<serde_json::Value>(source) {
        Ok(value) => Ok(Manifest {
            dir: path
                .rfind('/')
                .map_or("", |slash| &path[..slash])
                .to_owned(),
            name: value
                .get("name")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
            entries: entries(&value),
            runner: runner(&value),
            jest_major: ["devDependencies", "dependencies"]
                .iter()
                .find_map(|field| {
                    let version = value.get(field)?.get("jest")?.as_str()?;
                    let version = version.trim_start_matches(['^', '~', '=']);
                    version.split('.').next()?.parse().ok()
                }),
            named_scripts: value
                .get("scripts")
                .and_then(serde_json::Value::as_object)
                .map(|scripts| {
                    scripts
                        .iter()
                        .filter_map(|(name, command)| {
                            Some((name.clone(), command.as_str()?.to_owned()))
                        })
                        .collect()
                })
                .unwrap_or_default(),
            scripts: value
                .get("scripts")
                .and_then(serde_json::Value::as_object)
                .map(|scripts| {
                    scripts
                        .values()
                        .filter_map(serde_json::Value::as_str)
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default(),
        }),
        Err(error) => Err(Diagnostic {
            code: "manifest_invalid".to_owned(),
            severity: Severity::Warning,
            file: Some(path.to_owned()),
            span: None,
            message: format!("package.json is not valid JSON: {error}"),
        }),
    }
}

fn runner(manifest: &serde_json::Value) -> Option<GateTool> {
    let declares = |name| {
        ["dependencies", "devDependencies"].iter().any(|field| {
            manifest
                .get(field)
                .and_then(|deps| deps.get(name))
                .is_some()
        })
    };
    match (declares("jest"), declares("vitest")) {
        (true, false) => Some(GateTool::Jest),
        (false, true) => Some(GateTool::Vitest),
        _ => None,
    }
}

/// The entry paths a manifest names: `main`, `module`, `types`, `source`, and every string
/// leaf of `exports` (a string, a subpath map, or nested conditions), in that order.
fn entries(manifest: &serde_json::Value) -> Vec<String> {
    let mut entries: Vec<String> = ["main", "module", "types", "source"]
        .iter()
        .filter_map(|field| manifest.get(field)?.as_str().map(str::to_owned))
        .collect();
    let mut stack: Vec<&serde_json::Value> = manifest.get("exports").into_iter().collect();
    while let Some(value) = stack.pop() {
        match value {
            serde_json::Value::String(path) => entries.push(path.clone()),
            serde_json::Value::Object(map) => stack.extend(map.values()),
            serde_json::Value::Array(items) => stack.extend(items),
            _ => {}
        }
    }
    entries
}
