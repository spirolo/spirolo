//! The rule families as pure queries over the model: model in, findings out, no I/O.
//!
//! This crate is one layer above `spirolo-frameworks` in the workspace chain
//! (`model ← frameworks ← inputs ← rules ← report ← spirolo`). It owns three things:
//! the [`Finding`] a rule produces, the [`Rule`] trait every rule implements, and the
//! [`registry`] that lists them in a fixed order behind their families. The binary wires
//! config, model, rules and the report together; nothing here reads a file or a clock.
//!
//! A rule is a pure function of the model and its [`Config`]: [`Rule::run`] takes both and
//! returns findings sorted by the crate's own key, so two runs over the same tree produce
//! byte-identical output. `weak-assertion` is the first rule, and the shape the others
//! copy.

mod absence_anchor;
mod claims;
mod config;
mod data_layer_mock;
mod dead_skip;
mod duplicate_test;
mod fail_closed_setup;
mod finding;
mod flake_candidate;
mod flake_observed;
mod gate_truth;
mod generator_walk;
mod handler_drift;
mod impact;
mod orphan_test;
mod redundant_test;
mod stale_snapshot;
pub mod status;
mod untested_hotspot;
mod unused_mock;
mod weak_assertion;

use spirolo_model::{Model, TestCase, TestKind};

pub use crate::absence_anchor::AbsenceAnchor;
pub use crate::config::{ClaimDocument, Config, RuleConfig};
pub use crate::data_layer_mock::DataLayerMock;
pub use crate::dead_skip::DeadSkip;
pub use crate::duplicate_test::DuplicateTest;
pub use crate::fail_closed_setup::FailClosedSetup;
pub use crate::finding::{
    gate_proposals, Evidence, Finding, Location, Outcome, Proposal, RelatedLocation, TestPath,
    PROPOSALS_CLEARED,
};
pub use crate::flake_candidate::FlakeCandidate;
pub use crate::flake_observed::FlakeObserved;
pub use crate::gate_truth::GateTruth;
pub use crate::generator_walk::GeneratorWalk;
pub use crate::handler_drift::HandlerDrift;
pub use crate::impact::{impact, Impact, Selected};
pub use crate::orphan_test::OrphanTest;
pub use crate::redundant_test::RedundantTest;
pub use crate::stale_snapshot::StaleSnapshot;
pub use crate::untested_hotspot::{churn_missing, hotspots, Hotspot, Hotspots, UntestedHotspot};
pub use crate::unused_mock::UnusedMock;
pub use crate::weak_assertion::WeakAssertion;

/// The rows a per-test rule judges: the model's `test(…)` / `it(…)` cases only. A suite
/// row carries no body of its own; a hook (`beforeEach`) guards the setup for the tests
/// that follow rather than proving behaviour; a `bench` measures; a Playwright `test.step`
/// is a sub-unit whose assertions its parent test already carries. None of them is a test
/// that could pass against a wrong value, so `weak-assertion` and `absence-anchor` read
/// through this and never flag one.
fn tests(model: &Model) -> impl Iterator<Item = &TestCase> {
    model
        .test_cases
        .iter()
        .filter(|case| case.kind == TestKind::Test)
}

/// The families a rule belongs to, matching the `spirolo <family>` subcommands.
///
/// `check` runs every rule; each other command runs one family. A rule declares its
/// family so the registry can filter without a second table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Family {
    /// Cross-file wiring: handler drift, absence anchors, weak assertions, data-layer
    /// mocks, generator walks — everything `spirolo audit` runs.
    Audit,
    /// What actually blocks a merge: gate truth and dead skips — `spirolo gate`.
    Gate,
    /// Unused tests a maintainer can pay down under a ceiling but that never fails the build:
    /// duplicate and redundant tests and, in later slices, unused, orphan and stale — `spirolo
    /// hygiene`. Registered after `gate`.
    Hygiene,
    /// Tests that can pass on one run and fail on the next: static flake candidates and,
    /// in later slices, flakes observed in the run ledger — `spirolo flake`. Registered after
    /// `hygiene`.
    Flake,
    /// Where test effort pays: `untested-hotspot`, ranking source modules by churn ×
    /// complexity × weakness. Registered after `flake`.
    Impact,
}

impl Family {
    /// The question this family's human and editor section answers (PRD 0006).
    pub const fn title(self) -> &'static str {
        match self {
            Self::Audit => "Weak tests",
            Self::Gate => "Merge gate",
            Self::Hygiene => "Unused tests",
            Self::Flake => "Flaky",
            Self::Impact => "Health",
        }
    }

    /// The lower-case name the CLI and the report spell this family with.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Audit => "audit",
            Self::Gate => "gate",
            Self::Hygiene => "hygiene",
            Self::Flake => "flake",
            Self::Impact => "impact",
        }
    }
}

/// One default severity a rule carries, overridable per project in the [`Config`].
///
/// Distinct from the model's `Severity`, which grades a diagnostic: this grades a finding,
/// and its `Warn` and `Error` levels gate unsuppressed findings in the CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// A finding that fails the build when it is not in the baseline.
    Error,
    /// A finding worth reporting that gates the build when it is not in the baseline.
    Warn,
    /// Context, reported for the count and never a failure.
    Info,
}

impl Severity {
    /// The lower-case token the report and the config spell this severity with.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
        }
    }

    /// Parse a severity from its config spelling, `None` for anything else.
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "error" => Some(Self::Error),
            "warn" => Some(Self::Warn),
            "info" => Some(Self::Info),
            _ => None,
        }
    }
}

/// The population a rule judges for the provisional health score (formula v1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subject {
    /// Model cases whose kind is `TestKind::Test`.
    Tests,
    /// Distinct test files collected or recognised in the model.
    TestFiles,
    /// Runner gate configurations in the model's `gates` table.
    Gates,
    /// Recorded ledger runs.
    Runs,
    /// The 200 source modules with highest hotspot complexity evidence.
    HotspotModules,
}

/// A rule: model plus config in, findings out. Pure — no I/O, no clock, no mutation of the
/// model.
///
/// Every rule declares its identity (name, family, default severity) so the registry can
/// list and filter it, and produces findings in a deterministic order. The report and the
/// harness read only what [`Rule::run`] returns.
pub trait Rule: Sync {
    /// The lower-case name a user selects with `--rule`, for example `weak-assertion`.
    fn name(&self) -> &'static str;

    /// Which family the rule runs under.
    fn family(&self) -> Family;

    /// The subject population this rule judges, fixed by PRD 0006's score table.
    fn subject(&self) -> Subject;

    /// One sentence naming the two things that disagree when the rule fires, printed by
    /// `spirolo explain <rule>`.
    fn summary(&self) -> &'static str;

    /// The severity a finding carries unless the config overrides it.
    fn default_severity(&self) -> Severity;

    /// Run the rule over the model, honouring any overrides in `config`.
    ///
    /// Returns findings sorted by the crate's key (rule, then file, then span), so the
    /// caller can concatenate several rules' output and keep it deterministic.
    fn run(&self, model: &Model, config: &Config) -> Vec<Finding>;
}

/// Every rule, in the fixed order the registry runs them.
///
/// Machine reports retain this order across rules. The human check view orders
/// findings by proof, severity and location and lists empty rules in registry order.
pub fn registry() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(HandlerDrift),
        Box::new(GeneratorWalk),
        Box::new(AbsenceAnchor),
        Box::new(WeakAssertion),
        Box::new(DataLayerMock),
        Box::new(FailClosedSetup),
        Box::new(GateTruth),
        Box::new(DeadSkip),
        Box::new(DuplicateTest),
        Box::new(RedundantTest),
        Box::new(UnusedMock),
        Box::new(OrphanTest),
        Box::new(StaleSnapshot),
        Box::new(FlakeCandidate),
        Box::new(FlakeObserved),
        Box::new(UntestedHotspot),
    ]
}

/// The rule with the given name, or `None` when no rule answers to it — which the binary
/// turns into a "no such rule" error and exit 2.
pub fn rule_named(name: &str) -> Option<Box<dyn Rule>> {
    registry().into_iter().find(|rule| rule.name() == name)
}

/// Every rule in `family`, in registry order — what `spirolo audit` and `spirolo gate` run.
pub fn family_rules(family: Family) -> Vec<Box<dyn Rule>> {
    registry()
        .into_iter()
        .filter(|rule| rule.family() == family)
        .collect()
}
