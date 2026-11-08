//! The rule-status table: each rule's measured corpus verdict, embedded in the binary.
//!
//! The bench writes `corpus/rule-status.json` from its own verdicts (the measurement role);
//! this module embeds it with `include_str!` and answers what a surface needs: a rule's
//! [`RuleStatus`], whether it is [proven](is_proven), which [`Tier`] the default view puts it
//! in, and the one-line [`status_line`] that hover, `explain` and the sidebar print. Nothing
//! here re-measures a rule.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// The embedded file, exactly as the bench wrote it.
const RULE_STATUS_JSON: &str = include_str!("../../../corpus/rule-status.json");

/// The complete embedded measurement document, without dropping unparsed metadata.
pub const fn json() -> &'static str {
    RULE_STATUS_JSON
}

/// The parsed file: the bench's commit, the corpus entries and one status per rule.
#[derive(Debug, Clone, Deserialize)]
pub struct Table {
    /// The file's schema number.
    pub schema: u32,
    /// The spirolo commit the bench ran.
    pub spirolo_commit: String,
    /// The corpus entries the verdicts pool over, as `name@sha`.
    pub entries: Vec<String>,
    /// Entry names on which the policy was measured; absent in pre-M6 documents.
    #[serde(default)]
    pub measured_on: Vec<String>,
    /// One status per rule, by rule name.
    pub rules: BTreeMap<String, RuleStatus>,
}

/// One rule's row in the table.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RuleStatus {
    /// Entry names on which this rule was measured; absent in pre-M6 documents.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub measured_on: Vec<String>,
    /// The bench's pooled verdict for the rule.
    pub verdict: Verdict,
    /// The milestone the rule's floor is carried to (`M6`), or `None` when it is not carried.
    pub carried: Option<String>,
    /// The pooled precision, `None` when no finding was decided.
    pub precision: Option<f64>,
    /// The precision's interval, `None` when no finding was decided.
    pub interval: Option<[f64; 2]>,
    /// Decided findings adjudicated true.
    pub truths: u32,
    /// Decided findings adjudicated false.
    pub falses: u32,
}

/// A bench verdict, spelled as the file spells it. Any other spelling fails the parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Verdict {
    /// Precision measured at or above the floor.
    Pass,
    /// Precision measured below the floor.
    Fail,
    /// Too few decided findings in the corpus to call it.
    Insufficient,
    /// Findings exist but no adjudicator judged them.
    Unadjudicated,
    /// The bench skipped the rule's row.
    Skipped,
    /// No bench row measures the rule.
    Unmeasured,
    /// The rule's visit dropped it: it never shows by default and never gates.
    Dropped,
}

/// Parse a rule-status document; an unknown verdict or a missing field is an error.
pub fn parse(text: &str) -> Result<Table, String> {
    serde_json::from_str(text).map_err(|error| format!("rule-status.json: {error}"))
}

/// The supplied table in a [`with_table`] scope, otherwise the embedded table parsed once.
pub fn table() -> &'static Table {
    static TABLE: OnceLock<Table> = OnceLock::new();
    SUPPLIED_TABLE.with(Cell::get).unwrap_or_else(|| {
        TABLE.get_or_init(|| parse(RULE_STATUS_JSON).expect("the embedded rule-status.json parses"))
    })
}

thread_local! {
    static SUPPLIED_TABLE: Cell<Option<&'static Table>> = const { Cell::new(None) };
}

/// Run a surface with an explicitly supplied status table on the current thread.
///
/// The caller owns the table's lifetime. Nested calls restore the previous table, including
/// on panic; other threads and the embedded measurement document are unaffected. This lets
/// consumers test status behavior independently of the measurements shipped in the binary.
pub fn with_table<R>(table: &'static Table, run: impl FnOnce() -> R) -> R {
    struct Restore(Option<&'static Table>);
    impl Drop for Restore {
        fn drop(&mut self) {
            SUPPLIED_TABLE.with(|supplied| supplied.set(self.0));
        }
    }
    let _restore = Restore(SUPPLIED_TABLE.with(|supplied| supplied.replace(Some(table))));
    run()
}

/// The row a rule absent from the table stands for: no bench row measures it.
static UNMEASURED: RuleStatus = RuleStatus {
    measured_on: Vec::new(),
    verdict: Verdict::Unmeasured,
    carried: None,
    precision: None,
    interval: None,
    truths: 0,
    falses: 0,
};

impl Verdict {
    /// The upper-case token the file and the status line spell this verdict with.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Fail => "FAIL",
            Self::Insufficient => "INSUFFICIENT",
            Self::Unadjudicated => "UNADJUDICATED",
            Self::Skipped => "SKIPPED",
            Self::Unmeasured => "UNMEASURED",
            Self::Dropped => "DROPPED",
        }
    }
}

impl RuleStatus {
    /// The status line: `rule: <VERDICT>`, then ` <precision> [<low>, <high>]` when the
    /// verdict is PASS or FAIL, then `, carried to <milestone>` when the rule is carried,
    /// and `, measured on <entries>` when the measurement names its corpus.
    pub fn line(&self) -> String {
        let mut line = format!("rule: {}", self.verdict.name());
        if matches!(self.verdict, Verdict::Pass | Verdict::Fail) {
            if let (Some(precision), Some([low, high])) = (self.precision, self.interval) {
                let _ = write!(line, " {precision:.3} [{low:.3}, {high:.3}]");
            }
        }
        if let Some(carried) = &self.carried {
            let _ = write!(line, ", carried to {carried}");
        }
        if !self.measured_on.is_empty() {
            let _ = write!(line, ", measured on {}", self.measured_on.join(", "));
        }
        line
    }
}

/// The rule's row in the embedded table, or an UNMEASURED row when the table has none.
pub fn status(rule: &str) -> &'static RuleStatus {
    table().rules.get(rule).unwrap_or(&UNMEASURED)
}

/// The rule's status line, as hover, `explain` and the sidebar print it.
pub fn status_line(rule: &str) -> String {
    status(rule).line()
}

/// Whether the rule is proven: its verdict is PASS. A carried-floor note does not
/// override the M6 release policy; only a proven rule keeps its default severity.
pub fn is_proven(rule: &str) -> bool {
    status(rule).verdict == Verdict::Pass
}

/// The fewest decided findings (truths + falses) an unproven rule needs to show by default.
pub const SHOWN_DECIDED_FLOOR: u32 = 15;

/// The lowest shown precision (truths / decided) at which an unproven rule shows by default,
/// as a `(numerator, denominator)` fraction so the comparison is exact: 4/5 is 0.80.
pub const SHOWN_PRECISION_FLOOR: (u32, u32) = (4, 5);

/// Where the default view puts a rule, computed only from its row in the status table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// Verdict PASS: shown by default at the rule's declared severity.
    Proven,
    /// Not PASS, FAIL or DROPPED, with at least [`SHOWN_DECIDED_FLOOR`] decided findings and
    /// shown precision at or above [`SHOWN_PRECISION_FLOOR`]: shown by default at `info`.
    Advisory,
    /// Every other rule that is not FAIL or DROPPED, measured or not: shown only by `--all`.
    Experimental,
    /// Verdict FAIL or DROPPED: shown only by `--all`.
    Hidden,
}

impl Tier {
    /// Whether the default view shows this tier's rows; `--all` shows every tier.
    pub const fn shown_by_default(self) -> bool {
        matches!(self, Self::Proven | Self::Advisory)
    }
}

impl RuleStatus {
    /// The row's [`Tier`]: its verdict, then the decided-count and shown-precision floors.
    pub fn tier(&self) -> Tier {
        match self.verdict {
            Verdict::Pass => Tier::Proven,
            Verdict::Fail | Verdict::Dropped => Tier::Hidden,
            _ => {
                let truths = u64::from(self.truths);
                let decided = truths + u64::from(self.falses);
                let (numerator, denominator) = SHOWN_PRECISION_FLOOR;
                if decided >= u64::from(SHOWN_DECIDED_FLOOR)
                    && truths * u64::from(denominator) >= decided * u64::from(numerator)
                {
                    Tier::Advisory
                } else {
                    Tier::Experimental
                }
            }
        }
    }
}

/// The rule's [`Tier`] from its row in the table; a rule with no row is experimental.
pub fn tier(rule: &str) -> Tier {
    status(rule).tier()
}

/// Whether the default view shows the rule's rows: it is proven or advisory.
pub fn shown_by_default(rule: &str) -> bool {
    tier(rule).shown_by_default()
}

/// The lowest precision interval bound at which an unproven rule's findings lean true.
pub const LEANS_TRUE: f64 = 0.5;

/// Whether an unproven rule's findings lean true, so a finding is more likely right than wrong.
///
/// Its verdict is not FAIL and its measured precision interval starts at or above
/// [`LEANS_TRUE`]. The editor lists such a rule's `info` findings as advisory, not as hints.
pub fn leans_true(rule: &str) -> bool {
    let status = status(rule);
    status.verdict != Verdict::Fail && status.interval.is_some_and(|[low, _]| low >= LEANS_TRUE)
}

/// The release-policy default: PASS keeps the rule's declared severity; every other
/// verdict, including a missing status row, reports at info. User overrides take priority.
pub fn default_severity(rule: &dyn crate::Rule) -> crate::Severity {
    if is_proven(rule.name()) {
        rule.default_severity()
    } else {
        crate::Severity::Info
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Captured from `spirolo rule-status --format json` before this fix at 5b9be0a.
    fn fixture() -> &'static Table {
        static TABLE: OnceLock<Table> = OnceLock::new();
        TABLE.get_or_init(|| parse(include_str!("../tests/fixtures/rule-status.json")).unwrap())
    }

    #[test]
    fn every_registered_rule_is_in_the_embedded_table() {
        let table = table();
        for rule in crate::registry() {
            assert!(
                table.rules.contains_key(rule.name()),
                "{} is missing from corpus/rule-status.json",
                rule.name()
            );
        }
    }

    #[test]
    fn measured_on_entry_names_survive_parsing() {
        let mut document: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/rule-status.json")).unwrap();
        let names = serde_json::json!(["sample-monorepo", "example-console", "example-platform"]);
        document["measured_on"] = names.clone();
        document["rules"]["weak-assertion"]["measured_on"] = names.clone();
        let table = parse(&document.to_string()).unwrap();
        assert_eq!(serde_json::json!(table.measured_on), names);
        assert_eq!(
            serde_json::to_value(&table.rules["weak-assertion"]).unwrap()["measured_on"],
            names
        );
    }

    #[test]
    fn status_lines_name_the_actual_measured_corpus() {
        // Captured from `spirolo rule-status --format json` at 80d7763, after #260.
        let table = Box::leak(Box::new(
            parse(include_str!("../tests/fixtures/m6-rule-status.json")).unwrap(),
        ));
        with_table(table, || {
            let row = status("weak-assertion");
            assert!(!row.measured_on.is_empty());
            assert_eq!(
                status_line("weak-assertion"),
                format!(
                    "rule: {}, measured on {}",
                    row.verdict.name(),
                    row.measured_on.join(", ")
                )
            );
            assert_eq!(status_line("no-such-rule"), "rule: UNMEASURED");
        });
    }

    #[test]
    fn an_unknown_verdict_is_rejected() {
        let text = include_str!("../tests/fixtures/rule-status.json").replacen(
            "\"PASS\"",
            "\"PASSED\"",
            1,
        );
        let error = parse(&text).unwrap_err();
        assert!(error.contains("PASSED"), "{error}");
    }

    #[test]
    fn a_dropped_verdict_parses_and_is_never_proven() {
        let text = include_str!("../tests/fixtures/rule-status.json").replacen(
            "\"INSUFFICIENT\"",
            "\"DROPPED\"",
            1,
        );
        let table = parse(&text).unwrap();
        let (rule, row) = table
            .rules
            .iter()
            .find(|(_, row)| row.verdict == Verdict::Dropped)
            .unwrap();
        assert!(row.line().starts_with("rule: DROPPED"));
        let table = Box::leak(Box::new(table.clone()));
        with_table(table, || assert!(!is_proven(rule)));
    }

    #[test]
    fn status_lines_take_the_prds_four_forms() {
        with_table(fixture(), || {
            assert_eq!(
                status_line("weak-assertion"),
                "rule: PASS 1.000 [0.942, 1.000]"
            );
            assert_eq!(
                status_line("duplicate-test"),
                "rule: FAIL 0.750 [0.551, 0.880], carried to M6"
            );
            assert_eq!(
                status_line("unused-mock"),
                "rule: INSUFFICIENT, carried to M6"
            );
            assert_eq!(status_line("no-such-rule"), "rule: UNMEASURED");
        });
    }

    #[test]
    fn precision_prints_only_for_pass_and_fail() {
        with_table(fixture(), || {
            // `fail-closed-setup` has a precision in the table, but its verdict is INSUFFICIENT.
            assert!(table().rules["fail-closed-setup"].precision.is_some());
            assert_eq!(status_line("fail-closed-setup"), "rule: INSUFFICIENT");
            assert_eq!(status_line("handler-drift"), "rule: SKIPPED");
        });
    }

    #[test]
    fn every_pass_is_proven_even_with_a_carried_floor() {
        with_table(fixture(), || {
            let proven: Vec<_> = crate::registry()
                .iter()
                .map(|rule| rule.name())
                .filter(|name| is_proven(name))
                .collect();
            assert_eq!(
                proven,
                [
                    "absence-anchor",
                    "weak-assertion",
                    "dead-skip",
                    "flake-observed"
                ]
            );
            assert!(!is_proven("duplicate-test"), "FAIL, carried");
            let mut carried = fixture().clone();
            carried.rules.get_mut("weak-assertion").unwrap().carried = Some("M6".into());
            with_table(Box::leak(Box::new(carried)), || {
                assert!(is_proven("weak-assertion"), "M6 keeps every PASS severity");
            });
            assert!(!is_proven("no-such-rule"), "UNMEASURED");
        });
    }

    fn row(verdict: Verdict, truths: u32, falses: u32) -> RuleStatus {
        RuleStatus {
            verdict,
            truths,
            falses,
            ..UNMEASURED.clone()
        }
    }

    #[test]
    fn an_unproven_rule_is_advisory_only_at_both_floors() {
        for verdict in [
            Verdict::Insufficient,
            Verdict::Unadjudicated,
            Verdict::Skipped,
            Verdict::Unmeasured,
        ] {
            let tier = |truths, falses| row(verdict, truths, falses).tier();
            // The decided floor: 14 is under it, 15 meets it, at any precision.
            assert_eq!(tier(14, 0), Tier::Experimental, "{verdict:?}");
            assert_eq!(tier(15, 0), Tier::Advisory, "{verdict:?}");
            // The precision floor, compared exactly: 12/15 and 16/20 are 0.80, so they are in.
            assert_eq!(tier(12, 3), Tier::Advisory, "{verdict:?}");
            assert_eq!(tier(16, 4), Tier::Advisory, "{verdict:?}");
            // Just below 0.80: 11/15 and 79/99.
            assert_eq!(tier(11, 4), Tier::Experimental, "{verdict:?}");
            assert_eq!(tier(79, 20), Tier::Experimental, "{verdict:?}");
            // Nothing decided is experimental.
            assert_eq!(tier(0, 0), Tier::Experimental, "{verdict:?}");
        }
    }

    #[test]
    fn the_verdict_decides_proven_and_hidden_whatever_the_counts() {
        for (truths, falses) in [(0, 0), (14, 0), (20, 0), (0, 20)] {
            assert_eq!(row(Verdict::Pass, truths, falses).tier(), Tier::Proven);
            assert_eq!(row(Verdict::Fail, truths, falses).tier(), Tier::Hidden);
            assert_eq!(row(Verdict::Dropped, truths, falses).tier(), Tier::Hidden);
        }
        assert!(Tier::Proven.shown_by_default() && Tier::Advisory.shown_by_default());
        assert!(!Tier::Experimental.shown_by_default() && !Tier::Hidden.shown_by_default());
    }

    #[test]
    fn a_rules_tier_comes_from_the_supplied_table() {
        let mut table = fixture().clone();
        table.rules = [
            ("dead-skip".into(), row(Verdict::Insufficient, 18, 0)),
            ("gate-truth".into(), row(Verdict::Unadjudicated, 12, 6)),
            ("orphan-test".into(), row(Verdict::Dropped, 20, 0)),
        ]
        .into();
        with_table(Box::leak(Box::new(table)), || {
            assert_eq!(tier("dead-skip"), Tier::Advisory);
            assert!(shown_by_default("dead-skip"));
            assert_eq!(tier("gate-truth"), Tier::Experimental);
            assert!(!shown_by_default("gate-truth"));
            assert_eq!(tier("orphan-test"), Tier::Hidden);
            assert_eq!(tier("no-such-rule"), Tier::Experimental, "UNMEASURED");
        });
    }
}
