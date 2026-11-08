//! Reading and writing `.spirolo/baseline.json`: where the ratchet lives on disk.
//!
//! The baseline logic — counting, diffing, tightening, pruning — is `spirolo-report`'s; this
//! module is only the file's location and the I/O around it, so the binary owns the path
//! convention and the report crate stays pure. A missing file is simply no baseline, which
//! is how a first run with no `.spirolo/` directory reads as "everything is new".

use std::path::{Path, PathBuf};

use spirolo_report::Baseline;

/// The directory `spirolo` keeps its per-repo state in, relative to the project root.
const STATE_DIR: &str = ".spirolo";
/// The baseline file within [`STATE_DIR`].
const BASELINE_FILE: &str = "baseline.json";

/// The path `.spirolo/baseline.json` under `root`.
pub fn path(root: &Path) -> PathBuf {
    root.join(STATE_DIR).join(BASELINE_FILE)
}

/// Load the baseline under `root`, or `None` when there is no file (a first run).
///
/// A file that does not parse is an error, so a corrupt baseline is caught rather than
/// silently treated as absent — that would turn every finding new and mask the corruption.
pub fn load(root: &Path) -> Result<Option<Baseline>, String> {
    let file = path(root);
    let source = match std::fs::read_to_string(&file) {
        Ok(source) => source,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("cannot read {}: {error}", file.display())),
    };
    Baseline::from_json(&source)
        .map(Some)
        .map_err(|error| format!("{} is not a valid baseline: {error}", file.display()))
}

/// Write `baseline` to `.spirolo/baseline.json` under `root`, creating `.spirolo/` if needed.
pub fn store(root: &Path, baseline: &Baseline) -> Result<(), String> {
    let file = path(root);
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    std::fs::write(&file, baseline.to_json())
        .map_err(|error| format!("cannot write {}: {error}", file.display()))
}
