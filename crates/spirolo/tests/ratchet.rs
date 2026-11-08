//! `spirolo ratchet` and the baseline half of `spirolo check`, end to end on fixture projects.
//!
//! Seam 2 of the PRD's testing decisions applied to the ratchet: a small project with a
//! known number of weak-assertion findings, driven through the real binary with
//! `assert_cmd`. `ratchet` writes `.spirolo/baseline.json`; a following `check` suppresses
//! what the baseline recorded and surfaces only what rose. The diff's own branches are
//! covered on hand-built findings in `spirolo-report`; here we prove the wiring — model,
//! rules, baseline read/write, exit codes — is whole.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;

#[path = "support/policy.rs"]
mod policy_support;

fn spirolo() -> Command {
    Command::cargo_bin("spirolo").unwrap()
}

/// A fresh project directory under the target tree, named after the test that asked for it.
fn project(name: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(path.join("src")).unwrap();
    policy_support::info(&path);
    path
}

/// Write `contents` to `path` under `root`, creating parents.
fn write(root: &Path, path: &str, contents: &str) {
    let full = root.join(path);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(full, contents).unwrap();
}

/// A weak test in `file` with the given `title`, asserting one value-less matcher.
fn weak_test(title: &str) -> String {
    format!(
        "import {{ it, expect }} from 'vitest'\n\
         it('{title}', () => {{ expect(widget()).toBeTruthy() }})\n"
    )
}

/// A fixture with three weak tests across two files — three findings, all `warn`.
fn three_findings(name: &str) -> PathBuf {
    let root = project(name);
    write(
        &root,
        "src/a.test.ts",
        "import { it, expect } from 'vitest'\n\
         it('a one', () => { expect(widget()).toBeTruthy() })\n\
         it('a two', () => { expect(widget()).toBeDefined() })\n",
    );
    write(&root, "src/b.test.ts", &weak_test("b one"));
    root
}

/// The written baseline file's contents.
fn baseline_of(root: &Path) -> String {
    std::fs::read_to_string(root.join(".spirolo/baseline.json")).unwrap()
}

// ----------------------------------------------------------- ratchet writes, check is clean

#[test]
fn ratchet_writes_counts_and_check_then_reports_zero_new() {
    let root = three_findings("ratchet-clean");

    // Ratchet records today's three findings.
    spirolo().arg("ratchet").arg(&root).assert().success();
    let baseline = baseline_of(&root);
    assert!(
        baseline.contains("src/a.test.ts") && baseline.contains("src/b.test.ts"),
        "both files recorded"
    );
    assert!(
        baseline.contains("weak-assertion"),
        "the rule is recorded per file"
    );

    // A following check surfaces none of them and says so.
    spirolo()
        .args(["check", "--all"])
        .arg(&root)
        .assert()
        .success()
        .stdout(predicate::str::contains("0 new"));
}

// ------------------------------------------------- a rising count surfaces, error exits one

#[test]
fn adding_a_finding_surfaces_all_of_that_rule_in_the_file_and_error_exits_one() {
    // The rise must be able to gate, and a DROPPED rule never gates whatever the config asks;
    // `dead-skip`'s visit is closed and it is not DROPPED, so the error half does not ride on a
    // verdict a visit may still change (weak-assertion's did, #501).
    const GATING_RULE: &str = "dead-skip";
    assert_ne!(
        spirolo_rules::status::status(GATING_RULE).verdict,
        spirolo_rules::status::Verdict::Dropped,
        "{GATING_RULE} is DROPPED, so no override can gate it: pick another rule for this test"
    );
    let skips = |titles: &[&str]| {
        titles.iter().fold(
            "import { it, expect } from 'vitest'\n".to_owned(),
            |file, title| {
                file + &format!("it.skip('{title}', () => {{ expect(sum(1, 2)).toBe(3) }})\n")
            },
        )
    };
    let root = project("ratchet-rise");
    write(&root, "src/a.test.ts", &skips(&["a one", "a two"]));
    write(&root, "src/b.test.ts", &skips(&["b one"]));
    spirolo().arg("ratchet").arg(&root).assert().success();

    // Add a third untracked skip to `src/a.test.ts`: its count rises from 2 to 3.
    write(
        &root,
        "src/a.test.ts",
        &skips(&["a one", "a two", "a three"]),
    );

    // At info, the new findings surface but the run stays green.
    spirolo()
        .args(["check", "--all"])
        .arg(&root)
        .assert()
        .success()
        .stdout(
            // All of that rule's findings in the risen file surface — not just the new one.
            predicate::str::contains("a one")
                .and(predicate::str::contains("a two"))
                .and(predicate::str::contains("a three"))
                // The unchanged file stays suppressed.
                .and(predicate::str::contains("b one").not())
                .and(predicate::str::contains("3 new")),
        );

    // The same rise at `error` severity fails the build.
    write(
        &root,
        ".spirolorc.jsonc",
        &format!("{{ \"rules\": {{ \"{GATING_RULE}\": \"error\" }} }}\n"),
    );
    spirolo()
        .args(["check", "--all"])
        .arg(&root)
        .assert()
        .code(1);
}

// ------------------------------------------------------------------- a falling count prunes

#[test]
fn removing_a_finding_warns_and_prune_drops_the_entry() {
    let root = three_findings("ratchet-prune");
    spirolo().arg("ratchet").arg(&root).assert().success();

    // Remove `src/b.test.ts` entirely: its baselined finding no longer matches.
    std::fs::remove_file(root.join("src/b.test.ts")).unwrap();

    // A plain check warns that the baseline has a stale entry to prune.
    spirolo()
        .args(["check", "--all"])
        .arg(&root)
        .assert()
        .success()
        .stdout(predicate::str::contains("prune"));

    // `ratchet --prune` drops the dead entry.
    spirolo()
        .args(["ratchet", "--prune"])
        .arg(&root)
        .assert()
        .success();
    assert!(
        !baseline_of(&root).contains("src/b.test.ts"),
        "the removed file's entry is pruned"
    );
}

// ------------------------------------------------------------------------------- staleness

#[test]
fn a_mostly_unmatched_baseline_warns_and_fail_on_stale_exits_one() {
    let root = project("ratchet-stale");
    // Four weak tests across four files → four baseline entries.
    for file in ["a", "b", "c", "d"] {
        write(&root, &format!("src/{file}.test.ts"), &weak_test(file));
    }
    spirolo().arg("ratchet").arg(&root).assert().success();

    // Remove three of the four: only one entry still matches — 3 of 4 unmatched, stale.
    for file in ["b", "c", "d"] {
        std::fs::remove_file(root.join(format!("src/{file}.test.ts"))).unwrap();
    }

    // A plain check warns about the stale baseline but still exits by severity (green here).
    spirolo()
        .args(["check", "--all"])
        .arg(&root)
        .assert()
        .success()
        .stdout(predicate::str::contains("stale"));

    // `--fail-on-stale-baseline` turns the warning into a failure.
    spirolo()
        .args(["check", "--all", "--fail-on-stale-baseline"])
        .arg(&root)
        .assert()
        .code(1);
}

// ------------------------------------------------------------------------------- tighten

#[test]
fn tighten_lowers_the_ratio_ceiling_to_the_measured_ratio() {
    let root = three_findings("ratchet-tighten");
    // All three assertions are value-less: the ratio is 100 %. Record a loose ceiling in the
    // baseline first via a config ceiling that would otherwise silence the row is not the
    // point here — we record whatever the run measures.
    spirolo().arg("ratchet").arg(&root).assert().success();

    let recorded = baseline_of(&root);
    // The recorded ceiling is the measured ratio: 3 of 3 value-less is 1.0.
    assert!(
        recorded.contains("\"weak-assertion\": 1.0") || recorded.contains("\"weak-assertion\": 1"),
        "the ratio ceiling is recorded: {recorded}"
    );

    // Now lower one file to a strong test so the measured ratio drops, then tighten.
    write(
        &root,
        "src/b.test.ts",
        "import { it, expect } from 'vitest'\n\
         it('b one', () => { expect(sum(1, 2)).toBe(3) })\n",
    );
    spirolo()
        .args(["ratchet", "--tighten"])
        .arg(&root)
        .assert()
        .success();

    let tightened = baseline_of(&root);
    // The ceiling fell below 1.0 (two of three assertions value-less → ~0.67).
    assert!(
        !tightened.contains("\"weak-assertion\": 1.0"),
        "the ratio ceiling was lowered to the measured ratio: {tightened}"
    );
}

// ----------------------------------------------------------------- duplicate-test is counted

/// A fixture with a within-file clone pair in each of two files — one `duplicate-test`
/// finding per file, all `warn`. The two files carry distinct bodies (different callee and
/// literal) so each is its own group rather than one cross-file group.
fn clone_pairs(name: &str) -> PathBuf {
    let root = project(name);
    write(
        &root,
        "src/a.test.ts",
        "import { it, expect } from 'vitest'\n\
         it('a one', () => { const x = alpha(); expect(x).toBe(1) })\n\
         it('a two', () => { const y = alpha(); expect(y).toBe(1) })\n",
    );
    write(
        &root,
        "src/b.test.ts",
        "import { it, expect } from 'vitest'\n\
         it('b one', () => { const x = beta(); expect(x).toBe(2) })\n\
         it('b two', () => { const y = beta(); expect(y).toBe(2) })\n",
    );
    root
}

#[test]
fn ratchet_counts_duplicate_test_per_file_and_prune_drops_a_healed_file() {
    let root = clone_pairs("ratchet-duplicate");

    // The ratchet records the duplicate-test finding per file, like any other rule.
    spirolo().arg("ratchet").arg(&root).assert().success();
    let baseline = baseline_of(&root);
    assert!(
        baseline.contains("duplicate-test"),
        "the hygiene rule is baselined: {baseline}"
    );
    assert!(
        baseline.contains("src/a.test.ts") && baseline.contains("src/b.test.ts"),
        "both files' groups are recorded: {baseline}"
    );

    // A following check surfaces none of the baselined clones.
    spirolo()
        .args(["check", "--all"])
        .arg(&root)
        .assert()
        .success()
        .stdout(predicate::str::contains("0 new"));

    // Heal `src/b.test.ts` — its second test now has a distinct body, so the group is gone.
    write(
        &root,
        "src/b.test.ts",
        "import { it, expect } from 'vitest'\n\
         it('b one', () => { const x = beta(); expect(x).toBe(2) })\n\
         it('b two', () => { const y = beta(); expect(y).toBe(9) })\n",
    );

    // A plain check warns the baseline has a stale entry, and `--prune` drops it.
    spirolo()
        .args(["check", "--all"])
        .arg(&root)
        .assert()
        .success()
        .stdout(predicate::str::contains("prune"));
    spirolo()
        .args(["ratchet", "--prune"])
        .arg(&root)
        .assert()
        .success();
    let pruned = baseline_of(&root);
    assert!(
        !pruned.contains("src/b.test.ts"),
        "the healed file's duplicate-test entry is pruned: {pruned}"
    );
    assert!(
        pruned.contains("src/a.test.ts"),
        "the still-cloned file keeps its entry: {pruned}"
    );
}

#[test]
fn tighten_lowers_a_duplicate_test_count_to_the_current_run() {
    let root = clone_pairs("ratchet-duplicate-tighten");

    // Seed `src/a.test.ts` with two clone groups: one finding per group, count 2 in the file.
    write(
        &root,
        "src/a.test.ts",
        "import { it, expect } from 'vitest'\n\
         it('a one', () => { const x = alpha(); expect(x).toBe(1) })\n\
         it('a two', () => { const y = alpha(); expect(y).toBe(1) })\n\
         it('a three', () => { const p = gamma(); expect(p).toBe(3) })\n\
         it('a four', () => { const q = gamma(); expect(q).toBe(3) })\n",
    );
    spirolo().arg("ratchet").arg(&root).assert().success();
    assert!(
        baseline_of(&root).contains("\"duplicate-test\": 2"),
        "two duplicate-test groups are recorded for the file: {}",
        baseline_of(&root)
    );

    // Heal one group; tighten lowers the recorded count to the one that remains.
    write(
        &root,
        "src/a.test.ts",
        "import { it, expect } from 'vitest'\n\
         it('a one', () => { const x = alpha(); expect(x).toBe(1) })\n\
         it('a two', () => { const y = alpha(); expect(y).toBe(1) })\n\
         it('a three', () => { const p = gamma(); expect(p).toBe(3) })\n\
         it('a four', () => { const q = gamma(); expect(q).toBe(7) })\n",
    );
    spirolo()
        .args(["ratchet", "--tighten"])
        .arg(&root)
        .assert()
        .success();
    assert!(
        baseline_of(&root).contains("\"duplicate-test\": 1"),
        "the count was tightened to the surviving group: {}",
        baseline_of(&root)
    );
}

// -------------------------------------------------------------------------- byte-stable json

#[test]
fn the_written_baseline_is_byte_stable_and_has_no_line_numbers() {
    let root = three_findings("ratchet-stable");

    spirolo().arg("ratchet").arg(&root).assert().success();
    let first = baseline_of(&root);

    spirolo().arg("ratchet").arg(&root).assert().success();
    let second = baseline_of(&root);

    assert_eq!(
        first, second,
        "two ratchets over the same tree are byte-equal"
    );
    assert!(
        !first.contains("\"line\""),
        "the baseline carries no line numbers"
    );
}
