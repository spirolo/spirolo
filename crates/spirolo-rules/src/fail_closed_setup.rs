//! `fail-closed-setup`: the four ways a runner's setup lets a test pass for the wrong reason,
//! read from the setup files the model already analysed.
//!
//! A suite fails closed when four invariants hold, each read onto [`Gate::setup`] by the
//! model:
//!
//! - **unhandled requests error**: MSW's `server.listen({ onUnhandledRequest: "error" })`, so
//!   a request no handler matches fails the test instead of passing with a warning.
//! - **console guard**: a `console.error`/`console.warn` spy that throws, or a fail-on-console
//!   package, so a logged error fails the test rather than scrolling past.
//! - **mock reset**: `restoreMocks: true` or an `afterEach(restoreAllMocks)`, so a mock set in
//!   one test cannot leak into the next.
//! - **global server**: the mock server's `listen()` in a `beforeAll`, so every test runs
//!   behind it rather than starting it case by case.
//!
//! Each invariant that is [`Invariant::Absent`] is one finding naming it and the setup file;
//! an invariant that holds is silent. A gate the reader could not see whole (an unresolvable
//! preset, a function export, a spread — [`Gate::opaque`], with the cause in
//! [`Gate::unread`]) has setup files it does not know, so it is one
//! [`Outcome::CouldNotDecide`] row saying what could not be read, never four decided
//! absences. The two MSW invariants are judged only when a mock
//! server is present ([`Invariant::NotApplicable`] otherwise), so a project with no MSW is
//! never told to configure one. A gate that collects no test file and that no other run loads
//! as a project ([`Gate::collects_no_tests`]: a fixture app's `jest` block) protects nothing,
//! so it is not judged at all.
//!
//! **Shared presets.** A monorepo whose packages compose one preset declares the setup once,
//! in the preset, and inherits it everywhere. A gate that composes a preset and lacks an
//! invariant lacks it because the preset sets it nowhere and the gate adds nothing, so the
//! rule reports it once at the preset (the root of the gate's chain) with the count of
//! packages that inherit the gap as evidence — one decision, one finding — and never once per
//! package. An inheritor that adds the invariant itself is simply not counted. The two MSW
//! invariants are lifted only when the preset's own setup starts the mock server; a server
//! the package's own setup starts without the option is the package's gap, at its setup file.
//! A gate with no preset chain is reported at the package itself.

use std::collections::BTreeMap;

use spirolo_model::{Gate, GateTool, Invariant, Model, Span};

use crate::finding::{number_collisions, sort, Evidence, Finding, Location, Outcome};
use crate::{Config, Family, Rule, Severity};

/// The `fail-closed-setup` rule. See the module docs for the semantics.
pub struct FailClosedSetup;

impl Rule for FailClosedSetup {
    fn name(&self) -> &'static str {
        "fail-closed-setup"
    }

    fn family(&self) -> Family {
        Family::Gate
    }

    fn subject(&self) -> crate::Subject {
        crate::Subject::Gates
    }

    fn summary(&self) -> &'static str {
        "The runner's setup misses an invariant (unhandled requests error, a console guard, mock reset, a global server) that makes a broken test fail."
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn run(&self, model: &Model, config: &Config) -> Vec<Finding> {
        let severity = config.severity(self.name(), self.default_severity());
        let runners = model
            .gates
            .iter()
            .filter(|gate| matches!(gate.tool, GateTool::Vitest | GateTool::Jest))
            // A gate that collects no test file protects nothing: its setup is not advice.
            .filter(|gate| !gate.collects_no_tests);
        // A gate the reader could not see whole has setup files it does not know: its
        // invariants are undecided, not absent. One could-not-decide row per such gate; the
        // decided gates are judged invariant by invariant.
        let (unread, gates): (Vec<&Gate>, Vec<&Gate>) = runners.partition(|gate| gate.opaque);

        let mut findings: Vec<Finding> = unread
            .into_iter()
            .map(|gate| self.setup_unread(gate))
            .collect();
        for invariant in INVARIANTS {
            self.judge_invariant(*invariant, &gates, severity, &mut findings);
        }

        sort(&mut findings);
        number_collisions(&mut findings);
        findings
    }
}

impl FailClosedSetup {
    /// Emit the findings for one invariant across every readable runner gate: one
    /// preset-level finding per shared preset, counting the inheritors that lack it, and one
    /// package-level finding per gate whose gap is its own.
    fn judge_invariant(
        &self,
        invariant: InvariantKind,
        gates: &[&Gate],
        severity: Severity,
        findings: &mut Vec<Finding>,
    ) {
        // The inheritors lacking the invariant, by the preset the gap is decided at.
        let mut groups: BTreeMap<&str, usize> = BTreeMap::new();
        for gate in gates {
            if invariant.state(gate) != Invariant::Absent {
                continue;
            }
            match decided_at_preset(invariant, gate) {
                Some(preset) => *groups.entry(preset).or_default() += 1,
                None => findings.push(self.gate_finding(invariant, gate, severity)),
            }
        }
        for (preset, packages) in groups {
            findings.push(self.preset_finding(invariant, preset, packages, severity));
        }
    }

    /// A could-not-decide row at a gate the reader could not see whole: its setup files are
    /// not known, so no invariant can be called absent. The row says what could not be read.
    fn setup_unread(&self, gate: &Gate) -> Finding {
        let unread = gate.unread.join("; ");
        Finding {
            rule: self.name(),
            severity: Severity::Info,
            outcome: Outcome::CouldNotDecide,
            location: Location::at(gate.source.clone(), Span::default()),
            related: Vec::new(),
            message: "runner config could not be read statically, so its setup invariants are \
                      undecided"
                .to_owned(),
            evidence: vec![Evidence::new("unread", unread.clone())],
            explain: format!(
                "The reader could not decide this config statically: {unread}. Its setup files \
                 are therefore not known, and none of the four fail-closed invariants can be \
                 called present or absent — this row is not a finding and never fails the \
                 build. Make the config readable by AST (an object literal, or a preset the \
                 resolver can follow) so the setup can be judged, or check the setup by hand."
            ),
            anchor: "setup-unread".to_owned(),
            test: None,
            ordinal: 0,
            proposal: None,
        }
    }

    /// A finding located at one gate's setup file (or its config when it has no setup file).
    fn gate_finding(&self, invariant: InvariantKind, gate: &Gate, severity: Severity) -> Finding {
        let file = gate
            .setup
            .setup_file
            .clone()
            .unwrap_or_else(|| gate.source.clone());
        Finding {
            rule: self.name(),
            severity,
            outcome: Outcome::Finding,
            location: Location::at(file, Span::default()),
            related: Vec::new(),
            message: format!("setup does not {}", invariant.gap()),
            evidence: vec![Evidence::new("invariant", invariant.slug())],
            explain: invariant.explain().to_owned(),
            anchor: invariant.slug().to_owned(),
            test: None,
            ordinal: 0,
            proposal: None,
        }
    }

    /// A finding lifted to a shared preset, carrying the count of packages that inherit the
    /// gap. Its identity is the preset path and the invariant slug, no line.
    fn preset_finding(
        &self,
        invariant: InvariantKind,
        preset: &str,
        packages: usize,
        severity: Severity,
    ) -> Finding {
        let plural = if packages == 1 { "" } else { "s" };
        Finding {
            rule: self.name(),
            severity,
            outcome: Outcome::Finding,
            location: Location::at(preset.to_owned(), Span::default()),
            related: Vec::new(),
            message: format!(
                "shared preset does not {} for {packages} inheriting package{plural}",
                invariant.gap()
            ),
            evidence: vec![
                Evidence::new("invariant", invariant.slug()),
                Evidence::new("packages", packages.to_string()),
            ],
            explain: format!(
                "{packages} package{plural} compose this preset and none of them adds this \
                 invariant itself, so the gap is one decision here, not one per package. {}",
                invariant.explain()
            ),
            anchor: invariant.slug().to_owned(),
            test: None,
            ordinal: 0,
            proposal: None,
        }
    }
}

/// The preset a gate's missing invariant is decided at, or `None` when the gap is the
/// gate's own.
///
/// A gate that composes a preset and lacks the console guard or the mock reset lacks it
/// because the preset sets neither and the gate adds nothing — the preset's decision. The
/// two MSW invariants are the preset's only when the preset's own setup starts the mock
/// server ([`spirolo_model::SetupInvariants::msw_inherited`]); a server the gate's own setup
/// starts without the option is the gate's own gap, at its own setup file.
fn decided_at_preset(invariant: InvariantKind, gate: &Gate) -> Option<&str> {
    let preset = gate.preset_chain.last()?.as_str();
    if invariant.needs_msw() && !gate.setup.msw_inherited {
        return None;
    }
    Some(preset)
}

/// One of the four fail-closed invariants, so the rule can read, name and explain each without
/// four near-identical methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InvariantKind {
    /// MSW `onUnhandledRequest: "error"`.
    UnhandledRequestError,
    /// A throwing console guard or fail-on-console package.
    ConsoleGuard,
    /// `restoreMocks` / `afterEach(restoreAllMocks)`.
    RestoreMocks,
    /// The mock server's `listen()` in a `beforeAll`.
    GlobalServer,
}

/// The four invariants, in the order the rule checks and the report groups them.
const INVARIANTS: &[InvariantKind] = &[
    InvariantKind::UnhandledRequestError,
    InvariantKind::ConsoleGuard,
    InvariantKind::RestoreMocks,
    InvariantKind::GlobalServer,
];

impl InvariantKind {
    /// This invariant's state on a gate.
    const fn state(self, gate: &Gate) -> Invariant {
        match self {
            Self::UnhandledRequestError => gate.setup.unhandled_request_error,
            Self::ConsoleGuard => gate.setup.console_guard,
            Self::RestoreMocks => gate.setup.restore_mocks,
            Self::GlobalServer => gate.setup.global_server,
        }
    }

    /// Whether this invariant is one of the two judged only when a mock server is present.
    const fn needs_msw(self) -> bool {
        matches!(self, Self::UnhandledRequestError | Self::GlobalServer)
    }

    /// The stable slug used in evidence and the finding anchor.
    const fn slug(self) -> &'static str {
        match self {
            Self::UnhandledRequestError => "unhandled-request-error",
            Self::ConsoleGuard => "console-guard",
            Self::RestoreMocks => "restore-mocks",
            Self::GlobalServer => "global-server",
        }
    }

    /// The gap phrase the message reads (`setup does not …`).
    const fn gap(self) -> &'static str {
        match self {
            Self::UnhandledRequestError => {
                "error on an unhandled request (no onUnhandledRequest: \"error\")"
            }
            Self::ConsoleGuard => {
                "guard the console (no throwing console.error/warn spy or fail-on-console package)"
            }
            Self::RestoreMocks => {
                "restore mocks between tests (no restoreMocks or afterEach restoreAllMocks)"
            }
            Self::GlobalServer => {
                "start the mock server globally (no server.listen in a beforeAll)"
            }
        }
    }

    /// The repair-oriented explain text, never "remove the assertion".
    const fn explain(self) -> &'static str {
        match self {
            Self::UnhandledRequestError => {
                "The mock server does not error on a request no handler matches, so a test that \
                 hits an unmocked URL passes with a warning instead of failing. Call \
                 `server.listen({ onUnhandledRequest: \"error\" })` in the setup file, so a \
                 missing handler fails the test that depends on it."
            }
            Self::ConsoleGuard => {
                "No console guard fails the test on a logged error, so a React warning or a \
                 caught exception logged to `console.error` scrolls past under a green run. Add \
                 a `console.error`/`console.warn` spy whose implementation throws, or a \
                 fail-on-console package, in the setup file, so a logged error is a failing test."
            }
            Self::RestoreMocks => {
                "Mocks are not restored between tests, so a mock set in one test leaks into the \
                 next and a later test can pass on the earlier one's stub. Set `restoreMocks: \
                 true` in the runner config, or call `restoreAllMocks()` in an `afterEach` in \
                 the setup file, so each test starts from the real implementations."
            }
            Self::GlobalServer => {
                "The mock server is not started once for every test, so a test that forgot to \
                 start it runs against the real network or none at all. Call the server's \
                 `listen()` in a `beforeAll` in the setup file (with `close()` in `afterAll` \
                 and `resetHandlers()` in `afterEach`), so every test runs behind the server."
            }
        }
    }
}
