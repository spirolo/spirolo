//! The substrate a corpus clone is on: installed at a lockfile, or bare.
//!
//! Decision 3 of the M3 reopen: counts, precision and adjudication read one substrate per
//! entry, recorded in `truth.json`. Whether a finding exists depends on the installed
//! dependencies (the resolver follows `node_modules`), so a number recorded on one substrate
//! is not comparable with a run on another. The substrate is named as a string both the
//! harness and the corpus adjudicators compute the same way: `installed <lockfile>
//! git:<blob id>` when the clone has a root `node_modules`, the lockfile named by the first
//! of [`LOCKFILES`] present and identified by `git hash-object`, else `bare`.

use std::path::Path;
use std::process::Command;

/// The lockfiles a clone may be installed from, in the order they are looked for.
const LOCKFILES: &[&str] = &[
    "pnpm-lock.yaml",
    "package-lock.json",
    "yarn.lock",
    "bun.lockb",
];

/// The substrate `repo` is on, as `truth.json` records it.
///
/// `bare` when there is no root `node_modules`; `installed <lockfile> git:<blob id>` when
/// there is, or `installed (no lockfile)` when no known lockfile is present. A lockfile whose
/// blob id cannot be computed (no `git` on the path) names the error in place of the id, so
/// it never compares equal to a recorded substrate.
#[must_use]
pub fn substrate_of(repo: &Path) -> String {
    if !repo.join("node_modules").is_dir() {
        return "bare".to_owned();
    }
    let Some(lockfile) = LOCKFILES.iter().find(|name| repo.join(name).is_file()) else {
        return "installed (no lockfile)".to_owned();
    };
    let blob = Command::new("git")
        .arg("hash-object")
        .arg(repo.join(lockfile))
        .output()
        .map_err(|error| error.to_string())
        .and_then(|output| {
            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
            }
        });
    match blob {
        Ok(blob) => format!("installed {lockfile} git:{blob}"),
        Err(error) => format!("installed {lockfile} (git hash-object failed: {error})"),
    }
}
