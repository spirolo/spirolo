//! The quarantine file's format: `.spirolo/quarantine.json`, which `spirolo flake quarantine`
//! writes from the ledger and a human commits.
//!
//! It is a record, not a runner setting: each entry names a test by file and title path,
//! says who added it (`ledger` or `human`), and carries the human's reason or the ledger's
//! evidence. Entries are sorted by file then test, and the bytes are pretty-printed JSON, so
//! reading a file and writing it again gives the same bytes and a change reads as a diff.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The quarantine file format's version, written into every file.
const VERSION: u32 = 1;

/// The whole quarantine file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuarantineFile {
    /// The format's version.
    pub version: u32,
    /// One entry per quarantined test, sorted by file then test.
    pub entries: Vec<QuarantineEntry>,
}

/// One quarantined test.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuarantineEntry {
    /// Repo-relative, slash-separated path of the test file.
    pub file: String,
    /// The enclosing `describe` titles, then the test's own title. The ledger always names a
    /// test, a file-level flip included (one entry per test it named); an empty path is a
    /// human's entry for the whole file.
    pub test: Vec<String>,
    /// The tree the ledger first saw the test flip on; a human entry may leave it out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tree_hash_first_seen: Option<String>,
    /// Who added the entry.
    pub added: Added,
    /// Why the test is quarantined, in a human's words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// The ledger's pass and fail counts, one per tree the test flipped on.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<TreeEvidence>,
    /// Keys a human wrote that the format does not name, kept as they were.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

/// Who added a quarantine entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Added {
    /// `spirolo flake quarantine`, from a flake the ledger observed.
    Ledger,
    /// A human, by hand.
    Human,
}

/// What the ledger saw of one test on one tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeEvidence {
    /// The tree the runs were made on.
    pub tree_hash: String,
    /// Rows that passed, a pass on retry included.
    pub passed: u32,
    /// Rows that failed, a pass on retry included.
    pub failed: u32,
}

impl Default for QuarantineFile {
    /// An empty file at the current version, for a project with none yet.
    fn default() -> Self {
        Self {
            version: VERSION,
            entries: Vec::new(),
        }
    }
}

impl QuarantineFile {
    /// Read a quarantine file.
    ///
    /// # Errors
    ///
    /// The parser's message when the bytes are not a quarantine file.
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        serde_json::from_slice(bytes).map_err(|error| error.to_string())
    }

    /// The file's bytes: pretty-printed JSON with a trailing newline.
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).unwrap_or_default();
        text.push('\n');
        text
    }

    /// Merge the flakes the ledger observes into the file: a flake with no entry for its
    /// file and test is added, a ledger entry takes the ledger's current evidence and keeps
    /// the tree it was first seen on, and an entry a human added is never rewritten. No
    /// entry is removed. The entries stay sorted by file then test.
    pub fn merge(&mut self, ledger: &[QuarantineEntry]) {
        for flake in ledger {
            let present = self
                .entries
                .iter_mut()
                .find(|entry| entry.file == flake.file && entry.test == flake.test);
            match present {
                Some(entry) if entry.added == Added::Ledger => {
                    entry.evidence.clone_from(&flake.evidence);
                }
                Some(_) => {}
                None => self.entries.push(flake.clone()),
            }
        }
        self.entries
            .sort_by(|a, b| (&a.file, &a.test).cmp(&(&b.file, &b.test)));
    }
}
