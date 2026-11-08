//! The quarantine file's format on snippets (Seam 4 of PRD 0004): `.spirolo/quarantine.json`
//! parses and writes back to the same bytes, and a merge with the ledger's flakes adds, keeps
//! and updates entries the way PRD 0004's "Quarantine file" decides.

use std::collections::BTreeMap;

use spirolo_inputs::{Added, QuarantineEntry, QuarantineFile, TreeEvidence};

/// A file as `spirolo flake quarantine` writes it: one ledger entry and one a human added.
const WRITTEN: &str = r#"{
  "version": 1,
  "entries": [
    {
      "file": "src/ledger.test.ts",
      "test": [
        "ledger",
        "flips between runs"
      ],
      "tree_hash_first_seen": "b34e2a6",
      "added": "ledger",
      "evidence": [
        {
          "tree_hash": "b34e2a6",
          "passed": 1,
          "failed": 1
        }
      ]
    },
    {
      "file": "src/ledger.test.ts",
      "test": [
        "ledger",
        "times out on CI"
      ],
      "added": "human",
      "reason": "times out under load on CI; tracked in #12"
    }
  ]
}
"#;

#[test]
fn an_unchanged_file_parses_and_writes_back_to_identical_bytes() {
    let file = QuarantineFile::parse(WRITTEN.as_bytes()).unwrap();
    assert_eq!(file.to_json(), WRITTEN);
}

/// A ledger flake of `test` in `src/ledger.test.ts`, seen on `tree_hash`.
fn ledger_flake(test: &str, tree_hash: &str, passed: u32, failed: u32) -> QuarantineEntry {
    QuarantineEntry {
        file: "src/ledger.test.ts".to_owned(),
        test: vec!["ledger".to_owned(), test.to_owned()],
        tree_hash_first_seen: Some(tree_hash.to_owned()),
        added: Added::Ledger,
        reason: None,
        evidence: vec![TreeEvidence {
            tree_hash: tree_hash.to_owned(),
            passed,
            failed,
        }],
        extra: BTreeMap::new(),
    }
}

#[test]
fn a_human_entry_with_a_reason_survives_a_merge_untouched() {
    let mut file = QuarantineFile::parse(WRITTEN.as_bytes()).unwrap();
    let human = file.entries[1].clone();

    // The ledger now sees the human's test flip too.
    file.merge(&[ledger_flake("times out on CI", "c0ffee1", 3, 2)]);

    assert_eq!(file.entries.len(), 2);
    assert_eq!(file.entries[1], human);
}

#[test]
fn a_new_ledger_flake_is_added_with_its_evidence_in_file_then_test_order() {
    let mut file = QuarantineFile::parse(WRITTEN.as_bytes()).unwrap();
    let new = ledger_flake("is red or skipped", "c0ffee1", 4, 1);

    file.merge(std::slice::from_ref(&new));

    let tests: Vec<&str> = file.entries.iter().map(|e| e.test[1].as_str()).collect();
    assert_eq!(
        tests,
        ["flips between runs", "is red or skipped", "times out on CI"]
    );
    assert_eq!(file.entries[1], new);
}

#[test]
fn a_ledger_entry_gets_the_ledgers_current_counts_and_keeps_its_first_tree() {
    let mut file = QuarantineFile::parse(WRITTEN.as_bytes()).unwrap();
    let mut seen_again = ledger_flake("flips between runs", "b34e2a6", 5, 3);
    seen_again.evidence.push(TreeEvidence {
        tree_hash: "c0ffee1".to_owned(),
        passed: 2,
        failed: 1,
    });
    // The ledger's first sight in this merge is not the entry's first sight.
    seen_again.tree_hash_first_seen = Some("c0ffee1".to_owned());

    file.merge(std::slice::from_ref(&seen_again));

    let entry = &file.entries[0];
    assert_eq!(entry.tree_hash_first_seen.as_deref(), Some("b34e2a6"));
    assert_eq!(entry.evidence, seen_again.evidence);
}

#[test]
fn an_entry_the_ledger_no_longer_sees_is_kept() {
    let mut file = QuarantineFile::parse(WRITTEN.as_bytes()).unwrap();

    file.merge(&[]);

    assert_eq!(file.to_json(), WRITTEN);
}

#[test]
fn a_key_a_human_added_to_an_entry_is_kept() {
    let written = WRITTEN.replace(
        r#""reason": "times out under load on CI; tracked in #12""#,
        r#""reason": "times out under load on CI; tracked in #12",
      "owner": "@payments""#,
    );
    let mut file = QuarantineFile::parse(written.as_bytes()).unwrap();

    file.merge(&[]);

    assert_eq!(file.to_json(), written);
}
