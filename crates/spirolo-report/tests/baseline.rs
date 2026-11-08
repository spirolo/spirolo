//! The ratchet baseline on hand-built findings: count per file per rule, the rising-count
//! rule, prune candidates, staleness, tighten and prune, and byte-stable JSON.
//!
//! Seam 3 of the PRD's testing decisions applied to the baseline: the diff is a pure
//! function of findings and a recorded baseline, so it is exercised on findings built in
//! the test rather than through the binary. The binary's own `ratchet`/`check` wiring is
//! proven on a fixture project in the `spirolo` crate.

use spirolo_report::Baseline;
use spirolo_rules::{Evidence, Finding, Location, Outcome, Severity};

/// A per-file finding of `rule` at `file`, with a distinct anchor so several in one file do
/// not collide.
fn finding(rule: &'static str, file: &str, anchor: &str) -> Finding {
    Finding {
        rule,
        severity: Severity::Warn,
        outcome: Outcome::Finding,
        location: Location {
            file: file.to_owned(),
            line: 1,
            column: 1,
        },
        related: Vec::new(),
        message: format!("{rule} in {file}"),
        evidence: Vec::new(),
        explain: "repair, do not delete".to_owned(),
        anchor: anchor.to_owned(),
        test: None,
        ordinal: 0,
        proposal: None,
    }
}

/// The project-wide ratio row a ratio rule emits: no file, a `ratio` evidence entry.
fn ratio_finding(rule: &'static str, ratio: f64) -> Finding {
    Finding {
        rule,
        severity: Severity::Warn,
        outcome: Outcome::Finding,
        location: Location {
            file: String::new(),
            line: 0,
            column: 0,
        },
        related: Vec::new(),
        message: format!("ratio {ratio}"),
        evidence: vec![Evidence::new("ratio", format!("{ratio:.4}"))],
        explain: "ratchet the ratio".to_owned(),
        anchor: "ratio".to_owned(),
        test: None,
        ordinal: 0,
        proposal: None,
    }
}

// ----------------------------------------------------------------- recording and matching

#[test]
fn a_baseline_counts_findings_per_file_per_rule_and_a_matched_run_surfaces_nothing() {
    let findings = vec![
        finding("weak-assertion", "src/a.test.ts", "a1"),
        finding("weak-assertion", "src/a.test.ts", "a2"),
        finding("weak-assertion", "src/b.test.ts", "b1"),
    ];
    let baseline = Baseline::from_findings(&findings);

    assert_eq!(baseline.files["src/a.test.ts"]["weak-assertion"], 2);
    assert_eq!(baseline.files["src/b.test.ts"]["weak-assertion"], 1);

    // The same three findings against that baseline are all suppressed: nothing new.
    let diff = baseline.diff(&findings);
    assert!(diff.surfaced.is_empty(), "a matched run surfaces nothing");
    assert_eq!(diff.unmatched, 0);
}

#[test]
fn undecided_and_listed_rows_are_neither_baselined_nor_surfaced() {
    let mut undecided = finding("handler-drift", "src/a.test.ts", "u");
    undecided.outcome = Outcome::CouldNotDecide;
    let mut listed = finding("dead-skip", "src/a.test.ts", "listed:t");
    listed.outcome = Outcome::Listed;
    let findings = vec![undecided, listed];

    let baseline = Baseline::from_findings(&findings);
    assert!(
        baseline.files.is_empty(),
        "nothing to suppress: {baseline:?}"
    );

    let diff = Baseline::default().diff(&findings);
    assert!(
        diff.surfaced.is_empty(),
        "a row that is not a finding never surfaces as new"
    );
}

#[test]
fn a_rising_count_surfaces_every_finding_of_that_rule_in_the_file() {
    let recorded = vec![finding("weak-assertion", "src/a.test.ts", "a1")];
    let baseline = Baseline::from_findings(&recorded);

    // A second finding of the same rule appears in the same file: the count rose from 1 to 2.
    let now = vec![
        finding("weak-assertion", "src/a.test.ts", "a1"),
        finding("weak-assertion", "src/a.test.ts", "a2"),
    ];
    let diff = baseline.diff(&now);

    // Both findings surface — the tool never guesses which one is new.
    assert_eq!(
        diff.surfaced.len(),
        2,
        "all of the rule's findings in the file"
    );
    assert!(diff.fallen.is_empty());
}

#[test]
fn an_unbaselined_file_surfaces_its_findings() {
    let baseline = Baseline::default();
    let now = vec![finding("weak-assertion", "src/new.test.ts", "n1")];

    // No baseline at all: every finding is above the (zero) count and surfaces.
    let diff = baseline.diff(&now);
    assert_eq!(diff.surfaced.len(), 1);
}

// ----------------------------------------------------------------------------- prune / fall

#[test]
fn a_falling_count_is_a_prune_candidate() {
    let recorded = vec![
        finding("weak-assertion", "src/a.test.ts", "a1"),
        finding("weak-assertion", "src/a.test.ts", "a2"),
    ];
    let baseline = Baseline::from_findings(&recorded);

    // One of the two findings is gone: the count fell from 2 to 1.
    let now = vec![finding("weak-assertion", "src/a.test.ts", "a1")];
    let diff = baseline.diff(&now);

    assert_eq!(diff.fallen.len(), 1);
    assert_eq!(diff.fallen[0].file, "src/a.test.ts");
    assert_eq!(diff.fallen[0].was, 2);
    assert_eq!(diff.fallen[0].now, 1);
    // A fall is not a rise: nothing surfaces.
    assert!(diff.surfaced.is_empty());
}

#[test]
fn prune_drops_entries_the_run_no_longer_matches() {
    let recorded = vec![
        finding("weak-assertion", "src/a.test.ts", "a1"),
        finding("weak-assertion", "src/gone.test.ts", "g1"),
    ];
    let mut baseline = Baseline::from_findings(&recorded);

    // `src/gone.test.ts` no longer has any finding.
    let now = vec![finding("weak-assertion", "src/a.test.ts", "a1")];
    baseline.prune(&now);

    assert!(baseline.files.contains_key("src/a.test.ts"));
    assert!(
        !baseline.files.contains_key("src/gone.test.ts"),
        "the dead entry is dropped"
    );
}

// -------------------------------------------------------------------------------- staleness

#[test]
fn a_baseline_where_a_quarter_or_more_matches_nothing_is_stale() {
    // Four entries across four files.
    let recorded = vec![
        finding("weak-assertion", "src/a.test.ts", "a1"),
        finding("weak-assertion", "src/b.test.ts", "b1"),
        finding("weak-assertion", "src/c.test.ts", "c1"),
        finding("weak-assertion", "src/d.test.ts", "d1"),
    ];
    let baseline = Baseline::from_findings(&recorded);

    // Only one of the four still matches.
    let now = vec![finding("weak-assertion", "src/a.test.ts", "a1")];
    let diff = baseline.diff(&now);

    assert_eq!(diff.total, 4);
    assert_eq!(diff.unmatched, 3);
    assert!(diff.is_stale(), "3 of 4 unmatched is stale");
}

#[test]
fn a_baseline_that_mostly_matches_is_not_stale() {
    let recorded = vec![
        finding("weak-assertion", "src/a.test.ts", "a1"),
        finding("weak-assertion", "src/b.test.ts", "b1"),
        finding("weak-assertion", "src/c.test.ts", "c1"),
        finding("weak-assertion", "src/d.test.ts", "d1"),
    ];
    let baseline = Baseline::from_findings(&recorded);
    let diff = baseline.diff(&recorded);

    assert_eq!(diff.unmatched, 0);
    assert!(!diff.is_stale());
}

// ------------------------------------------------------------------------- the ratio ceiling

#[test]
fn a_ratio_row_surfaces_only_above_the_recorded_ceiling() {
    // Recorded at 10 %.
    let baseline = Baseline::from_findings(&[ratio_finding("weak-assertion", 0.10)]);
    assert!((baseline.ratios["weak-assertion"] - 0.10).abs() < 1e-9);

    // A run at exactly the ceiling does not surface.
    let at = baseline.diff(&[ratio_finding("weak-assertion", 0.10)]);
    assert!(at.surfaced.is_empty(), "at the ceiling is not new");

    // A run above it does.
    let above = baseline.diff(&[ratio_finding("weak-assertion", 0.12)]);
    assert_eq!(above.surfaced.len(), 1, "above the ceiling surfaces");
}

#[test]
fn tighten_lowers_a_ratio_ceiling_and_counts_to_the_current_run() {
    let recorded = vec![
        finding("weak-assertion", "src/a.test.ts", "a1"),
        finding("weak-assertion", "src/a.test.ts", "a2"),
        ratio_finding("weak-assertion", 0.20),
    ];
    let mut baseline = Baseline::from_findings(&recorded);
    assert_eq!(baseline.files["src/a.test.ts"]["weak-assertion"], 2);
    assert!((baseline.ratios["weak-assertion"] - 0.20).abs() < 1e-9);

    // The current run has one fewer finding and a lower ratio.
    let now = vec![
        finding("weak-assertion", "src/a.test.ts", "a1"),
        ratio_finding("weak-assertion", 0.08),
    ];
    baseline.tighten(&now);

    assert_eq!(
        baseline.files["src/a.test.ts"]["weak-assertion"], 1,
        "count lowered to current"
    );
    assert!(
        (baseline.ratios["weak-assertion"] - 0.08).abs() < 1e-9,
        "ceiling lowered to current ratio"
    );
}

#[test]
fn tighten_never_raises_a_ceiling_or_a_count() {
    let mut baseline = Baseline::from_findings(&[
        finding("weak-assertion", "src/a.test.ts", "a1"),
        ratio_finding("weak-assertion", 0.05),
    ]);

    // A worse current run (more findings, higher ratio) must not loosen the baseline.
    let now = vec![
        finding("weak-assertion", "src/a.test.ts", "a1"),
        finding("weak-assertion", "src/a.test.ts", "a2"),
        ratio_finding("weak-assertion", 0.30),
    ];
    baseline.tighten(&now);

    assert_eq!(baseline.files["src/a.test.ts"]["weak-assertion"], 1);
    assert!((baseline.ratios["weak-assertion"] - 0.05).abs() < 1e-9);
}

// ------------------------------------------------------------------------------- json shape

#[test]
fn json_is_sorted_byte_stable_and_carries_no_line_numbers() {
    let findings = vec![
        finding("weak-assertion", "src/z.test.ts", "z1"),
        finding("weak-assertion", "src/a.test.ts", "a1"),
        finding("data-layer-mock", "src/a.test.ts", "d1"),
        ratio_finding("weak-assertion", 0.07),
    ];
    let baseline = Baseline::from_findings(&findings);

    let json = baseline.to_json();

    // Files sorted, rules within a file sorted: `src/a.test.ts` before `src/z.test.ts`,
    // `data-layer-mock` before `weak-assertion`.
    let a = json.find("src/a.test.ts").unwrap();
    let z = json.find("src/z.test.ts").unwrap();
    assert!(a < z, "files are sorted");
    let dlm = json.find("data-layer-mock").unwrap();
    let wa = json.find("weak-assertion").unwrap();
    assert!(dlm < wa, "rules within a file are sorted");

    // No line numbers anywhere: the baseline is line-shift immune by construction.
    assert!(
        !json.contains("\"line\""),
        "the baseline carries no line numbers"
    );

    // Round-trips and re-serializes byte-identically.
    let reparsed = Baseline::from_json(&json).unwrap();
    assert_eq!(reparsed, baseline);
    assert_eq!(reparsed.to_json(), json, "byte-stable across a round trip");
}
