//! `redundant-test`: within one mutation report, two or more tests whose kill sets — the
//! mutants each killed — are equal and non-empty. A test rewritten under a new name that
//! kills exactly the mutants an existing test already kills adds no discrimination, whatever
//! its title or literals, and is a finding a maintainer can act on.
//!
//! The rule reads the `Mutants` table (each mutant's `killed_by`) grouped by report. A test
//! with an empty kill set is never reported — a test that kills nothing is `impact`'s
//! concern, not redundancy. A group whose shared kill set holds only emptied blocks
//! (`BlockStatement` mutants) is "could not decide" at `info`: any test that runs a block
//! kills the mutant that empties it, so such a kill set cannot tell two behaviours apart.
//! It says "could not decide" for a test file whose paired source
//! no report covers, and emits one diagnostic per report that carries no per-test data or was
//! produced with bail (its kill sets are truncated), whose explain says to re-run with
//! `disableBail`. It is silent when the project has no mutation report.

use std::collections::BTreeMap;

use spirolo_model::{Bail, Model, Mutants, MutationReport, MutationTest, Span, TestCase};

use crate::finding::{
    number_collisions, sort, Evidence, Finding, Location, Outcome, Proposal, RelatedLocation,
    TestPath,
};
use crate::{Config, Family, Rule, Severity};

/// The `redundant-test` rule. See the module docs for the semantics.
pub struct RedundantTest;

impl Rule for RedundantTest {
    fn name(&self) -> &'static str {
        "redundant-test"
    }

    fn family(&self) -> Family {
        Family::Hygiene
    }

    fn subject(&self) -> crate::Subject {
        crate::Subject::Tests
    }

    fn summary(&self) -> &'static str {
        "Two tests in one mutation report kill exactly the same mutants, so the second adds no discrimination."
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn run(&self, model: &Model, config: &Config) -> Vec<Finding> {
        // Silent when the project has no mutation report: the rule never guesses redundancy
        // from the absence of data.
        if model.mutation_reports.is_empty() {
            return Vec::new();
        }
        let severity = config.severity(self.name(), self.default_severity());

        let mut findings = Vec::new();
        for report in &model.mutation_reports {
            if usable(report) {
                self.group_report(model, report, severity, config.propose, &mut findings);
            } else {
                findings.push(self.report_diagnostic(report));
            }
        }
        self.undecided(model, &mut findings);

        sort(&mut findings);
        number_collisions(&mut findings);
        findings
    }
}

impl RedundantTest {
    /// Group the tests of one usable report by their kill set and push a finding for every
    /// group of two or more with a non-empty kill set.
    fn group_report(
        &self,
        model: &Model,
        report: &MutationReport,
        severity: Severity,
        propose: bool,
        findings: &mut Vec<Finding>,
    ) {
        // Each test's kill set: the sorted mutant ids it killed, within this report.
        let mut kill_sets: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for mutant in &report_mutants(model, &report.path) {
            for killer in &mutant.killed_by {
                kill_sets.entry(killer).or_default().push(&mutant.id);
            }
        }

        // Group test ids by their (non-empty) kill set. The key is the joined mutant ids, so
        // two tests that killed exactly the same mutants land in one group.
        let mut groups: BTreeMap<String, Vec<&str>> = BTreeMap::new();
        for (test_id, mutants) in &kill_sets {
            if mutants.is_empty() {
                continue;
            }
            groups.entry(mutants.join(",")).or_default().push(test_id);
        }

        let mutants = report_mutants(model, &report.path);
        for (key, test_ids) in groups {
            if test_ids.len() < 2 {
                continue;
            }
            let ids: Vec<&str> = key.split(',').collect();
            let killed = ids.len();
            let group = Group {
                test_ids: &test_ids,
                killed,
                equivalent: only_emptied_blocks(&mutants, &ids),
            };
            findings.push(self.finding(model, report, &group, severity, propose));
        }
    }

    /// The one could-not-decide row an unusable report earns: a report with no per-test data,
    /// or one produced with bail (its `killedBy` truncated to the first killer, so every test
    /// looks redundant with the first), cannot be trusted for redundancy. The explain says to
    /// re-run with `disableBail` so every killing test is recorded.
    fn report_diagnostic(&self, report: &MutationReport) -> Finding {
        let (reason, anchor) = if report.per_test_data {
            (
                "was produced with bail, so its kill sets are truncated",
                "bailed",
            )
        } else {
            (
                "carries no per-test data, so no kill set can be built",
                "no-per-test",
            )
        };
        Finding {
            rule: self.name(),
            severity: Severity::Info,
            outcome: Outcome::CouldNotDecide,
            location: Location::at(report.path.clone(), Span::default()),
            related: Vec::new(),
            message: format!("mutation report {} {reason}", report.path),
            evidence: vec![Evidence::new("report", report.path.clone())],
            explain: format!(
                "The mutation report {} {reason}, so redundant-test cannot compare the tests' \
                 kill sets. Re-run Stryker with `disableBail: true` and `coverageAnalysis: \
                 \"perTest\"` so every killing test is recorded, then this rule can decide.",
                report.path
            ),
            anchor: format!("report-{anchor}"),
            test: None,
            ordinal: 0,
            proposal: None,
        }
    }

    /// A could-not-decide row for every test file whose paired source no report covers: the
    /// rule saw no mutation data for that source, so it cannot say whether the test is
    /// redundant. Never a finding for precision — honesty, not a guess.
    fn undecided(&self, model: &Model, findings: &mut Vec<Finding>) {
        for pairing in &model.pairings {
            if model
                .mutants
                .iter()
                .any(|mutant| mutant.file == pairing.source_module)
            {
                continue;
            }
            findings.push(Finding {
                rule: self.name(),
                severity: Severity::Info,
                outcome: Outcome::CouldNotDecide,
                location: Location::at(pairing.test_file.clone(), Span::default()),
                related: Vec::new(),
                message: format!(
                    "no mutation report covers {}, the source {} is paired to",
                    pairing.source_module, pairing.test_file
                ),
                evidence: vec![Evidence::new("source", pairing.source_module.clone())],
                explain: format!(
                    "No mutation report holds a mutant in {}, the source {} is paired to, so \
                     redundant-test cannot say whether any of its tests kill the same mutants as \
                     another. Run mutation testing over {} with per-test coverage to let the rule \
                     decide.",
                    pairing.source_module, pairing.test_file, pairing.source_module
                ),
                anchor: format!("uncovered:{}", pairing.source_module),
                test: None,
                ordinal: 0,
                proposal: None,
            });
        }
    }

    /// One finding for a redundant group: located at the first member, the others related,
    /// with the kill-set size, the members' paired sources and the report path as evidence.
    fn finding(
        &self,
        model: &Model,
        report: &MutationReport,
        group: &Group,
        severity: Severity,
        propose: bool,
    ) -> Finding {
        let killed = group.killed;
        let mut members: Vec<&MutationTest> = group
            .test_ids
            .iter()
            .filter_map(|id| named_test(model, &report.path, id))
            .collect();
        members.sort_by(|a, b| {
            location_of(a)
                .cmp(&location_of(b))
                .then(a.name.cmp(&b.name))
        });

        let first = members[0];
        let (first_file, first_span) = location_of(first);
        let related = members[1..]
            .iter()
            .map(|member| {
                let (file, span) = location_of(member);
                RelatedLocation {
                    label: member.name.clone(),
                    location: Location::at(file, span),
                }
            })
            .collect();
        let sources = members
            .iter()
            .map(|member| paired_source(model, &member.file))
            .collect::<Vec<_>>();
        let others = members.len() - 1;
        let (severity, outcome, explain) = if group.equivalent {
            (
                Severity::Info,
                Outcome::CouldNotDecide,
                equivalent_explain(&members, killed, &report.path),
            )
        } else {
            (
                severity,
                Outcome::Finding,
                explain(model, &members, killed, &report.path),
            )
        };
        Finding {
            rule: self.name(),
            severity,
            outcome,
            location: Location::at(first_file.clone(), first_span),
            related,
            message: format!(
                "{} kills the same {} mutant{} as {} other test{}",
                first.name,
                killed,
                if killed == 1 { "" } else { "s" },
                others,
                if others == 1 { "" } else { "s" }
            ),
            evidence: vec![
                Evidence::new("kill-set", killed.to_string()),
                Evidence::new("sources", join_unique(&sources)),
                Evidence::new("report", report.path.clone()),
            ],
            explain,
            anchor: anchor(&members, &first_file),
            test: joined_case(model, first).map(TestPath::of),
            ordinal: 0,
            proposal: (propose && !group.equivalent).then(|| proposal(model, &members)),
        }
    }
}

/// Whether every mutant of a kill set empties a block (`BlockStatement`): such a mutant is
/// killed by any test that runs the block, whatever it asserts, so a kill set made only of
/// them cannot tell two behaviours apart.
fn only_emptied_blocks(mutants: &[&Mutants], ids: &[&str]) -> bool {
    ids.iter().all(|id| {
        mutants
            .iter()
            .any(|mutant| mutant.id == *id && mutant.mutator.as_deref() == Some("BlockStatement"))
    })
}

/// One group of tests with an identical kill set, as the rule found it in a report.
struct Group<'a> {
    /// The report ids of the members.
    test_ids: &'a [&'a str],
    /// How many mutants the members jointly kill.
    killed: usize,
    /// Whether every one of those mutants only empties a block, so the members are
    /// mutation-equivalent rather than shown redundant.
    equivalent: bool,
}

/// The proposal for a redundant group. The deletion oracle keeps the first member and
/// deletes the others, so the evidence covers deleting only the later members, and only when
/// each has the first's body once names are normalised; otherwise every member is kept.
fn proposal(model: &Model, members: &[&MutationTest]) -> Proposal {
    let differ = differing(model, members);
    if differ.is_empty() {
        Proposal::Delete {
            what: Some(
                members[1..]
                    .iter()
                    .map(|member| identity(member))
                    .collect::<Vec<_>>()
                    .join("; "),
            ),
        }
    } else {
        Proposal::Keep {
            reason: format!(
                "{} differ{} from {} beyond names, so may check what it does not",
                names(&differ),
                if differ.len() == 1 { "s" } else { "" },
                members[0].name
            ),
        }
    }
}

/// The later members that may check what the first does not: any whose body differs from the
/// first's once names are normalised — other inputs, set up in a literal or a local, or other
/// assertions. Only a body equal to the first's shows a member adds nothing, and a member
/// whose body spirolo could not read counts as differing, since nothing shows otherwise.
fn differing<'a>(model: &Model, members: &[&'a MutationTest]) -> Vec<&'a MutationTest> {
    let first = case_of(model, members[0]).and_then(|case| case.body_hash.as_deref());
    members[1..]
        .iter()
        .copied()
        .filter(|member| {
            let body = case_of(model, member).and_then(|case| case.body_hash.as_deref());
            first.is_none() || body != first
        })
        .collect()
}

/// The test case a report test joined, when it joined one.
fn case_of<'a>(model: &'a Model, test: &MutationTest) -> Option<&'a TestCase> {
    let joined = test.joined.as_ref()?;
    model
        .test_cases
        .iter()
        .find(|case| case.file == joined.file && case.span == joined.span)
}

/// The members' runner names, comma-separated.
fn names(members: &[&MutationTest]) -> String {
    members
        .iter()
        .map(|member| member.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// A member's identity for a proposal: its runner name and where it lives, as
/// `name (file:line)`.
fn identity(member: &MutationTest) -> String {
    let (file, span) = location_of(member);
    format!("{} ({}:{})", member.name, file, span.line)
}

/// Whether a report's kill sets can be trusted for redundancy: it carries per-test data and
/// its bail was off, so every killing test is recorded. Any other report earns a diagnostic
/// instead of contributing groups.
fn usable(report: &MutationReport) -> bool {
    report.per_test_data && report.bail == Bail::Disabled
}

/// The mutants of one report.
fn report_mutants<'a>(model: &'a Model, report: &str) -> Vec<&'a Mutants> {
    model
        .mutants
        .iter()
        .filter(|mutant| mutant.report == report)
        .collect()
}

/// The `MutationTest` row for a test id in a report, or `None` when the report named no such
/// test.
fn named_test<'a>(model: &'a Model, report: &str, id: &str) -> Option<&'a MutationTest> {
    model
        .mutation_tests
        .iter()
        .find(|test| test.report == report && test.id == id)
}

/// A member's display location: the joined `TestCase`'s file and span when the test joined
/// the model, else the report's test file with an unknown span.
fn location_of(test: &MutationTest) -> (String, Span) {
    test.joined.as_ref().map_or_else(
        || (test.file.clone(), Span::default()),
        |joined| (joined.file.clone(), joined.span),
    )
}

/// The model's `TestCase` a report test joined to, or `None` when it did not join: a group
/// located at an unjoined member names no test the model can see.
fn joined_case<'a>(model: &'a Model, test: &MutationTest) -> Option<&'a TestCase> {
    let joined = test.joined.as_ref()?;
    model
        .test_cases
        .iter()
        .find(|case| case.file == joined.file && case.span == joined.span)
}

/// The source module a test file is paired with, or `unpaired` when no strategy paired it.
fn paired_source(model: &Model, test_file: &str) -> String {
    model
        .pairings
        .iter()
        .find(|pairing| pairing.test_file == test_file)
        .map_or_else(
            || "unpaired".to_owned(),
            |pairing| pairing.source_module.clone(),
        )
}

/// Join the values, dropping adjacent duplicates so a shared paired source is named once.
fn join_unique(values: &[String]) -> String {
    let mut seen: Vec<&String> = Vec::new();
    for value in values {
        if !seen.contains(&value) {
            seen.push(value);
        }
    }
    seen.iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join("; ")
}

/// The group's identity: a hash of its members' sorted names. Combined with the first
/// member's file it names the finding stably across runs, whatever the report ids were.
fn anchor(members: &[&MutationTest], _first_file: &str) -> String {
    let mut names: Vec<&str> = members.iter().map(|member| member.name.as_str()).collect();
    names.sort_unstable();
    let joined = names.join("\n");
    format!("{:016x}", xxhash_rust::xxh3::xxh3_64(joined.as_bytes()))
}

/// The repair text: names each test and what they jointly kill, then either keeps the first
/// and deletes the others — each has the first's body under another name — or names the
/// members whose body differs and deletes none, the same choice the proposal makes.
fn explain(model: &Model, members: &[&MutationTest], killed: usize, report: &str) -> String {
    let differ = differing(model, members);
    let head = format!(
        "These {} tests kill exactly the same {} mutant{} in {report}: {}.",
        members.len(),
        killed,
        if killed == 1 { "" } else { "s" },
        names(members),
    );
    if differ.is_empty() {
        format!(
            "{head} They test the same behaviour under different names — a change to the code \
             is caught by each, and the later ones have {}'s body under other names. Keep {} and \
             delete the others.",
            members[0].name, members[0].name
        )
    } else {
        format!(
            "{head} But {} differ{} from {} beyond names — other inputs or other assertions — \
             and may check a behaviour the report's mutants do not reach, so delete none of \
             them unless a deletion re-run shows the kill set holds without it.",
            names(&differ),
            if differ.len() == 1 { "s" } else { "" },
            members[0].name
        )
    }
}

/// The explain of a group whose kill set holds only emptied blocks: the members are
/// mutation-equivalent, which says nothing about what each asserts, so the reader checks by
/// hand before merging anything.
fn equivalent_explain(members: &[&MutationTest], killed: usize, report: &str) -> String {
    let names = members
        .iter()
        .map(|member| member.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let (mutants, each) = if killed == 1 {
        ("mutant", "it empties a whole block")
    } else {
        ("mutants", "each empties a whole block")
    };
    format!(
        "These {} tests kill exactly the same {killed} {mutants} in {report}: {names}. But \
         {each}, and any test that runs a block kills the mutant that empties it, whatever the \
         test asserts. The tests are mutation-equivalent, not shown to test the same behaviour: \
         check by hand whether they do before merging them.",
        members.len(),
    )
}
