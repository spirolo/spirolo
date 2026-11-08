//! Making sure a corpus clone has the commit the bench is about to check out (#202, F6).
//!
//! The clone under `corpus/<entry>/repo` is fetched only when someone remembers to, so a row
//! that names a newer commit (the known-flaky #1915 row's `b34e2a6`) errored with `invalid
//! reference`. The bench now fetches a missing commit from the clone's origin once, says so,
//! and never moves the clone's HEAD: a fetch only writes objects and remote-tracking refs.

use std::path::Path;
use std::process::Command;

/// Make `commit` (a full or abbreviated id, or a ref) resolvable in the clone at `repo`.
///
/// `Ok(None)` when the clone already has it. Otherwise the clone's `origin` is fetched once
/// (every branch and tag: an abbreviated id cannot be asked for by name) and the result is
/// `Ok(Some(message))`, the line the bench prints to say it fetched. `Err` when the fetch
/// fails or the commit is still missing after it.
pub fn ensure_commit(repo: &Path, commit: &str) -> Result<Option<String>, String> {
    if has_commit(repo, commit) {
        return Ok(None);
    }
    let fetched = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["fetch", "--quiet", "--tags", "origin"])
        .output()
        .map_err(|error| format!("git fetch origin: {error}"))?;
    if !fetched.status.success() {
        return Err(format!(
            "git fetch origin for {commit}: {}",
            String::from_utf8_lossy(&fetched.stderr).trim()
        ));
    }
    if !has_commit(repo, commit) {
        return Err(format!(
            "{commit} is not in {} nor in its origin",
            repo.display()
        ));
    }
    Ok(Some(format!(
        "fetched {commit} from the origin of {} (the clone did not have it)",
        repo.display()
    )))
}

/// Whether `commit` names a commit object in the clone.
fn has_commit(repo: &Path, commit: &str) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["cat-file", "-e", &format!("{commit}^{{commit}}")])
        .output()
        .is_ok_and(|output| output.status.success())
}
