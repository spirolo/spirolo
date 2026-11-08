//! The [`Finding`] a rule produces: the two things that disagree, where each lives, an
//! evidence map, a repair-oriented `explain`, and an identity the harness and `explain`
//! join on.
//!
//! A finding is data, not text: the report crate turns it into the human, JSON, SARIF and
//! agent views. Its identity is `rule` plus `file` plus a semantic anchor (a test's suite
//! path and title, a config key), with an ordinal on collision — stable across runs and
//! across unrelated edits, so `explain` and the precision harness name the same finding
//! the same way. The baseline does not use this identity; it counts per file per rule.

use crate::Severity;

/// What `--propose` suggests a maintainer do with a hygiene finding.
///
/// Set only under `--propose` (see [`Finding::proposal`]); absent everywhere else, so a
/// consumer that has not asked for proposals never sees one. Each hygiene rule picks the
/// action its evidence supports: `duplicate-test` merges the others into the richest member,
/// `redundant-test` deletes the later members only when each has the first's body,
/// `unused-mock` and `stale-snapshot` delete, and a could-not-decide row, an `orphan-test`
/// row (a missing glob, or a dead import spirolo cannot tell moved from removed) or a
/// redundant group whose bodies differ keeps, with the reason. Nothing is ever written to the tree, and the binary turns a merge or delete
/// into a keep unless the rule is cleared (see [`gate_proposals`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Proposal {
    /// Merge the finding's other tests into the named target, the richer test. `into` names
    /// that member and `from` the others.
    Merge {
        /// The other members, merged into `into` and then removed, as `label (file:line)`
        /// joined by `; ` — never the target itself, so a merge never names one test twice.
        from: String,
        /// The member to merge into, as `label (file:line)`.
        into: String,
    },
    /// Delete. `what` names what to delete when it is not simply the finding's own location
    /// — the later members of a redundant group, a stale entry's key — and is `None` when the finding's own location is the thing to remove.
    Delete {
        /// What to delete, or `None` for the finding's own location.
        what: Option<String>,
    },
    /// Keep, with the reason the finding is not removable as it stands (a never-collected
    /// file just needs an include glob).
    Keep {
        /// Why the finding is kept rather than removed.
        reason: String,
    },
}

/// The hygiene rules whose `merge` and `delete` proposals reach the user.
///
/// Those whose corpus verdict is PASS on an independent oracle. Empty until the attended M3
/// close records one (PRD 0003), so every proposal is `keep` for now.
pub const PROPOSALS_CLEARED: &[&str] = &[];

/// Turn every `merge` or `delete` proposal of a rule not in `cleared` into a `keep`.
///
/// The keep says why. A rule proposes what its semantics would do; only a rule whose precision an
/// independent oracle has confirmed may tell an agent to remove a test, a mock or a snapshot.
/// The binary applies this with [`PROPOSALS_CLEARED`] to everything the rules return.
pub fn gate_proposals(findings: &mut [Finding], cleared: &[&str]) {
    for finding in findings {
        let removes = matches!(
            finding.proposal,
            Some(Proposal::Merge { .. } | Proposal::Delete { .. })
        );
        if removes && !cleared.contains(&finding.rule) {
            finding.proposal = Some(Proposal::Keep {
                reason: format!(
                    "{} has not passed its corpus precision check on an independent oracle, \
                     so spirolo proposes no merge or delete",
                    finding.rule
                ),
            });
        }
    }
}

/// A location in a source file: where one of the two disagreeing things lives.
///
/// The span is the model's byte range; the report renders it as `file:line`. A related
/// location adds a label naming what it is (see [`RelatedLocation`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    /// Repo-relative, slash-separated path.
    pub file: String,
    /// One-based line of the location's start.
    pub line: u32,
    /// One-based column of the location's start.
    pub column: u32,
}

impl Location {
    /// A location at `file`, taking the line and column from a model span.
    pub fn at(file: impl Into<String>, span: spirolo_model::Span) -> Self {
        Self {
            file: file.into(),
            line: span.line,
            column: span.column,
        }
    }
}

/// A second location a finding points at, with a label saying what it is — the other side
/// of the disagreement, so a reader sees both without opening the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatedLocation {
    /// What this location is, for example `handler` or `production url`.
    pub label: String,
    /// Where it lives.
    pub location: Location,
}

/// The test a finding is anchored at, named the way `spirolo list` names a test case: its
/// enclosing `describe` titles, outermost first, then its own title.
///
/// This is test identity, not a display string: a consumer (the bench matching a finding to
/// a known-flaky row) compares it field for field, so a title that differs only by suite path
/// is a different test. A title that is not a literal is `None`, as in the model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestPath {
    /// Enclosing `describe` titles, outermost first.
    pub suite_path: Vec<String>,
    /// The title as written; `None` when it is not a literal.
    pub title: Option<String>,
}

impl TestPath {
    /// The title path of a test case.
    pub fn of(case: &spirolo_model::TestCase) -> Self {
        Self {
            suite_path: case.suite_path.clone(),
            title: case.title.clone(),
        }
    }
}

/// One named value in a finding's evidence.
///
/// The pattern, the folded URL, the count, the ratio — whatever the rule measured, kept as
/// a `(name, value)` pair rather than baked into the message so a consumer can read it
/// structurally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    /// The name of the measured thing, for example `matchers` or `ratio`.
    pub key: String,
    /// Its value as text.
    pub value: String,
}

impl Evidence {
    /// An evidence entry `(key, value)`.
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

/// Whether a rule decided, could not, or is only listing.
///
/// A conservative rule that cannot fold a URL or resolve a target records
/// [`Outcome::CouldNotDecide`] rather than guess; the harness counts those separately, so
/// honesty is not scored as a false positive. A rule that inventories what it saw (every
/// skip and its condition) records [`Outcome::Listed`]: a fact for the count, not a claim
/// that anything disagrees, so it is never a finding for precision, the baseline or SARIF.
/// `weak-assertion` always decides, but the distinction lives on the finding from the first
/// rule so every later rule inherits it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// A real finding: the two things named do disagree.
    Finding,
    /// The rule saw something worth reporting but could not decide it (an opaque URL, an
    /// unresolvable target). Never fails the build; counted apart from findings.
    CouldNotDecide,
    /// A listing row: something the rule inventories with its facts (a skip, its kind and
    /// its condition) so a count is visible, with no claim that it is wrong. Never fails
    /// the build; counted apart from findings and never sampled for precision.
    Listed,
}

/// One thing a rule found: the two things that disagree, where each lives, the evidence,
/// the repair-oriented `explain`, and a stable identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The rule that produced it, for example `weak-assertion`.
    pub rule: &'static str,
    /// The severity it carries, after config overrides.
    pub severity: Severity,
    /// Whether the rule decided or could not.
    pub outcome: Outcome,
    /// The primary location: the first of the two disagreeing things.
    pub location: Location,
    /// The other locations that disagree with the primary one, each labelled.
    pub related: Vec<RelatedLocation>,
    /// One sentence naming the disagreement, no trailing period.
    pub message: String,
    /// Named measured values, sorted by key so output is deterministic.
    pub evidence: Vec<Evidence>,
    /// How to repair the test — never "remove the assertion". Read by the agent view and
    /// `explain`.
    pub explain: String,
    /// The semantic anchor the identity is built from: a test's suite path and title, a
    /// config key, a handler pattern. Combined with `rule`, `file` and an ordinal it names
    /// the finding stably across runs.
    pub anchor: String,
    /// The test the finding is anchored at, when it is anchored at one; `None` for a finding
    /// anchored at a file, a config key, a mock or a handler, so the field is absent from
    /// every format there.
    pub test: Option<TestPath>,
    /// Ordinal on identity collision: `0` unless two findings of the same rule share a
    /// file and anchor, then `1`, `2`, … in the order they were produced.
    pub ordinal: u32,
    /// What to do about the finding, set only under `--propose` and only on a hygiene
    /// finding. `None` everywhere else, so the field is absent from every format unless the
    /// flag asked for it.
    pub proposal: Option<Proposal>,
}

impl Finding {
    /// The finding's stable identity: `rule` `file` `anchor` `#ordinal`, the string the
    /// harness joins adjudications on and `explain` addresses a finding by.
    pub fn id(&self) -> String {
        format!(
            "{} {} {}#{}",
            self.rule, self.location.file, self.anchor, self.ordinal
        )
    }
}

/// Assign the collision ordinals across a rule's findings: findings that share a file and
/// anchor take `0`, `1`, `2`, … in order; every other finding keeps `0`.
///
/// A rule builds its findings with `ordinal: 0`, sorts them, then calls this once so the
/// ordinal reflects the sorted order and is stable across runs.
pub fn number_collisions(findings: &mut [Finding]) {
    use std::collections::HashMap;
    let mut seen: HashMap<(String, String), u32> = HashMap::new();
    for finding in findings.iter_mut() {
        let key = (finding.location.file.clone(), finding.anchor.clone());
        let next = seen.entry(key).or_insert(0);
        finding.ordinal = *next;
        *next += 1;
    }
}

/// Sort findings by the crate's key — file, then line, then column, then anchor — so a
/// rule's output is deterministic before ordinals are assigned.
pub fn sort(findings: &mut [Finding]) {
    findings.sort_by(|a, b| {
        a.location
            .file
            .cmp(&b.location.file)
            .then(a.location.line.cmp(&b.location.line))
            .then(a.location.column.cmp(&b.location.column))
            .then(a.anchor.cmp(&b.anchor))
    });
}
