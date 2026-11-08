//! The `.snap` reader: snapshot text in, entries out.
//!
//! Vitest and Jest write an external snapshot file as a `CommonJS` module of
//! `exports[`key`] = `…`;` statements. The key is the runner's own test name — the
//! `describe` titles and the test title, ` > `-joined by Vitest (`suite > … > title N`),
//! space-joined by Jest (`suite … title N`) — then a space and a one-based ordinal that
//! counts a test's several snapshots. This reader turns each such line into a
//! [`SnapshotEntry`]: the key as the runner named it (the template literal's escapes undone:
//! a backslash before a backtick, a backslash or a dollar sign), its ordinal and its line. The key is never split on
//! ` > `, since a snapshot hint or a title may hold one. A key that does not end in an
//! ordinal is one diagnostic, never a silently dropped entry.
//!
//! It reads text and returns rows — no I/O, no siblings, no staleness. The binary walks the
//! `.snap` files and hands the bytes over; `stale-snapshot` decides which entries are stale.

use spirolo_model::{Diagnostic, Severity, SnapshotEntry};

/// What the reader lifted from one `.snap` file: the entries it could parse and one
/// diagnostic per line it could not.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SnapshotFacts {
    /// The entries parsed from the file, in file order.
    pub entries: Vec<SnapshotEntry>,
    /// One `snapshot_key_malformed` diagnostic per `exports[…]` line whose key is not the
    /// runner's `suite > title N` form.
    pub diagnostics: Vec<Diagnostic>,
}

/// Read a `.snap` file's `text`, attributing every diagnostic to `path`.
///
/// Each `exports[`…`]` statement contributes one entry when its key ends in a one-based
/// ordinal, or one diagnostic when it does not. Every other line — the `// Vitest Snapshot`
/// banner, the snapshot bodies, blank lines — is ignored.
#[must_use]
pub fn read_snapshot(path: &str, text: &str) -> SnapshotFacts {
    let mut facts = SnapshotFacts::default();
    for (index, line) in text.lines().enumerate() {
        let Some(key) = export_key(line) else {
            continue;
        };
        let line = u32::try_from(index + 1).unwrap_or(u32::MAX);
        match parse_key(&key, line) {
            Some(entry) => facts.entries.push(entry),
            None => facts.diagnostics.push(Diagnostic {
                code: "snapshot_key_malformed".to_owned(),
                severity: Severity::Warning,
                file: Some(path.to_owned()),
                span: None,
                message: format!("snapshot key {key:?} is not the runner's `suite > title N` form"),
            }),
        }
    }
    facts
}

/// The key of an `exports[`…`] = …` line as the runner named it — the text between the
/// backticks, with the template-literal escapes the runner wrote undone (a backslash before a
/// backtick, a backslash or a dollar sign) — or `None` when the line declares no export. The key ends at the first backtick that is
/// not escaped.
fn export_key(line: &str) -> Option<String> {
    let rest = line.trim_start().strip_prefix("exports[`")?;
    let mut key = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => key.push(chars.next()?),
            '`' => return chars.next().is_some_and(|c| c == ']').then_some(key),
            c => key.push(c),
        }
    }
    None
}

/// The entry for a snapshot key written at `line`, or `None` when the key does not end in
/// ` <ordinal>`.
fn parse_key(key: &str, line: u32) -> Option<SnapshotEntry> {
    let (_name, ordinal) = key.rsplit_once(' ')?;
    Some(SnapshotEntry {
        key: key.to_owned(),
        ordinal: ordinal.parse().ok()?,
        line,
    })
}
