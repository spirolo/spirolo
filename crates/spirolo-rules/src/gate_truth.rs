//! `gate-truth`: what actually fails the build, read from the `Gate` table and the CI reader.
//!
//! A maintainer believes the suite is gated — thresholds are set, mutation ratchets, CI runs
//! the tests — but belief is not a fact. This rule reads the configs the model already parsed
//! (runner, mutation, Playwright) and the CI workflows the model read as YAML, and reports
//! each place the gate is weaker than it looks.
//!
//! It has two halves. The **config half** reads the model's `Gate` table for the weaknesses
//! below. The **claims half** reads the documents named by the config's `claims` key — their
//! contents handed in by the binary as `claim_documents`, since the rules crate reads no file —
//! and diffs each line's gating claim against those same config facts with a narrow grammar (see
//! [`crate::claims`]): a line that says a coverage or mutation gate is enforced while the config
//! does not back it is an `info` finding at the document line, naming the config field it
//! disagrees with. The grammar is deliberate — the kill criterion is precision, and reading
//! intent from prose is where it is easiest to lose.
//!
//! What it reports, each a distinct finding kind:
//!
//! - **thresholds absent** (`thresholds-absent`): a runner gate (Vitest/Jest) declares no
//!   coverage thresholds, so a coverage drop never fails the build. A gate that composes a
//!   preset and neither inherits nor adds thresholds lacks them because the preset does: those
//!   are reported once at the preset (the root of the gate's chain) with the count of
//!   inheriting configs as `packages` evidence, never once per package; a config that composes
//!   no preset keeps its own finding.
//! - **config unread** (`config-unread`, could not decide): a runner gate the reader could
//!   not see whole (an unresolvable preset, a function export, a spread) and that declares no
//!   thresholds the reader *did* see. Whether it gates coverage is undecided, so the row is
//!   [`Outcome::CouldNotDecide`] — never a decided "absent" — and its explain says what could
//!   not be read. Counted apart from findings; never fails the build.
//! - **mutation break zero** (`mutation-break-zero`): a Stryker gate sets `break: 0`, the
//!   explicit "mutation score never fails the build".
//! - **mutation break null** (`mutation-break-null`): a Stryker gate whose `break` is `null`
//!   or absent (Stryker's own default is `null`), so the score is computed and reported but
//!   never gates — story 14 names `0` or `null` alike. Judged only on a gate the reader saw
//!   whole: an opaque Stryker config may set `break` in a part it could not read.
//! - **forbidOnly** (`forbid-only`): a Playwright gate leaves `forbidOnly` false or unset while
//!   a CI invocation selects it, so a stray `.only` committed to a branch passes CI as a run of one
//!   test. A different suite running in CI does not activate this config. Unread workflows,
//!   unresolved `CircleCI` selection and a `playwright test` the reader cannot place (behind
//!   an unknown runner or after a `cd` into a computed directory) remain conservative rather
//!   than hiding a finding.
//!   The scaffold's own `forbidOnly: !!process.env.CI` (and its `Boolean(…)`, `=== "true"`,
//!   `||` spellings) is read by the model as CI-conditional and counts as set: it is true in
//!   the build the rule judges.
//! - **CI runs nothing** (`ci-runs-nothing`): no workflow the project runs has a run step that
//!   runs the tests, coverage or mutation, so nothing in CI executes the suite. One finding
//!   for the project, located at the workflows directory with every workflow it read as
//!   evidence — not one per workflow that happens not to, since a lint workflow beside a test
//!   workflow is fine. Silent when any workflow runs them. The reader includes root
//!   `.circleci/config.yml` and files directly under root `.github/workflows`; a nested one
//!   (`packages/x/.github/workflows/ci.yml`) is read into the model but GitHub never runs it
//!   for this repository, so it neither counts as CI nor as running the suite. When a
//!   workflow could not be read (invalid YAML) and no readable one runs the suite, the row is
//!   [`Outcome::CouldNotDecide`] — the rule does not claim a workflow it could not read runs
//!   nothing.
//! - **retries** (`retries`, info): a gate configures retries, which hide flakiness rather than
//!   fail on it. A Playwright config that readable GitHub CI never loads is silent.
//!   Reported for the count, never a build failure.
//! - **claim unbacked** (`claim-unbacked`, info): a `claims` document line asserts a coverage or
//!   mutation gate is enforced that the config does not back. Located at the document line, with
//!   the config field as its related location. Info: a document's belief is worth surfacing but
//!   never fails the build. Its identity is the document path and the claim's normalised
//!   words, never its line number, so an edit above the line does not re-key it.
//!
//! Every finding is located at the config (or workflow) that owns the fact, with a `kind`
//! evidence entry the report and the threshold adjudicator read. The rule is a pure query over
//! the model; it reads no file and no clock.

use std::collections::BTreeMap;

use spirolo_model::{command_words, Gate, GateFlag, GateTool, Model};

use crate::claims::{classify, Claim, ClaimKind};
use crate::finding::{
    number_collisions, sort, Evidence, Finding, Location, Outcome, RelatedLocation,
};
use crate::{Config, Family, Rule, Severity};

/// The `gate-truth` rule. See the module docs for the semantics.
pub struct GateTruth;

/// The config field a claims finding points at as the other side of the disagreement.
struct ClaimField {
    /// The config file that owns the fact — the related location's file.
    file: String,
    /// What the field is, for example `coverage thresholds` — the related location's label.
    label: String,
}

/// The config's own value for a claimed gate: whether it is present, and the field a finding
/// names when the claim disagrees with it.
///
/// A coverage claim's value is present when a runner gate declares thresholds; a mutation
/// claim's when a Stryker `break` is above zero. The field points at the config file that owns
/// the fact (or a bare marker when the project has no such gate), so a finding names both sides
/// of the disagreement whichever way it falls.
struct ConfigValue {
    /// Whether the config sets an enforcing value for this gate.
    present: bool,
    /// The field a finding names as the config side of the disagreement.
    field: ClaimField,
}

impl ConfigValue {
    /// The config field this value names — the related location of a finding.
    const fn field(&self) -> &ClaimField {
        &self.field
    }
}

impl Rule for GateTruth {
    fn name(&self) -> &'static str {
        "gate-truth"
    }

    fn family(&self) -> Family {
        Family::Gate
    }

    fn subject(&self) -> crate::Subject {
        crate::Subject::Gates
    }

    fn summary(&self) -> &'static str {
        "What the project believes gates a merge (thresholds, mutation ratchets, CI running the tests) disagrees with what its configs and workflows actually enforce."
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn run(&self, model: &Model, config: &Config) -> Vec<Finding> {
        let severity = config.severity(self.name(), self.default_severity());
        let workflows = project_workflows(model);
        // CI presence gates the `forbidOnly` finding: a stray `.only` only matters if a build
        // actually runs the suite. A single readable CI gate is enough to count as "CI exists".
        let ci_present = workflows.iter().any(|gate| !gate.opaque);

        let mut findings = Vec::new();
        // Inheritors of one preset that declare no thresholds (and inherit none) lack them
        // because the preset does: counted by preset, one finding each.
        let mut lifted: BTreeMap<&str, usize> = BTreeMap::new();
        for gate in &model.gates {
            // A different suite running in CI says nothing about this Playwright config.
            if gate.tool == GateTool::Playwright && ci_present && !ci_loads_config(gate, &workflows)
            {
                continue;
            }
            self.judge_gate(gate, ci_present, severity, &mut findings, &mut lifted);
        }
        for (preset, configs) in lifted {
            findings.push(self.thresholds_absent_at_preset(preset, configs, severity));
        }
        if let Some(finding) = self.ci_runs_nothing(&workflows, severity) {
            findings.push(finding);
        }

        self.diff_claims(model, config, &mut findings);

        sort(&mut findings);
        number_collisions(&mut findings);
        findings
    }
}

impl GateTruth {
    /// Emit every finding one gate contributes; a thresholds gap decided at a preset is
    /// counted in `lifted` instead, for one finding per preset.
    fn judge_gate<'a>(
        &self,
        gate: &'a Gate,
        ci_present: bool,
        severity: Severity,
        findings: &mut Vec<Finding>,
        lifted: &mut BTreeMap<&'a str, usize>,
    ) {
        match gate.tool {
            GateTool::Vitest | GateTool::Jest => {
                if !gate.thresholds.is_empty() {
                    // Thresholds read are thresholds present, whatever else was unread.
                } else if gate.opaque {
                    // The reader could not see the whole config: whether it declares
                    // thresholds is undecided, never a decided "absent".
                    findings.push(self.config_unread(gate));
                } else if let Some(preset) = gate.preset_chain.last() {
                    *lifted.entry(preset.as_str()).or_default() += 1;
                } else {
                    findings.push(self.thresholds_absent(gate, severity));
                }
            }
            GateTool::Stryker => {
                if gate.mutation_break == Some(0.0) {
                    findings.push(self.mutation_break_zero(gate, severity));
                } else if gate.mutation_break.is_none() && !gate.opaque {
                    // Absent or `null`: Stryker's default, and the score never gates. An
                    // opaque gate may set it in a part the reader could not see, so it is
                    // not judged.
                    findings.push(self.mutation_break_null(gate, severity));
                }
            }
            GateTool::Playwright => {
                // A flag that holds in CI — the literal `true`, or the scaffold's own
                // `!!process.env.CI` — is set where it matters.
                let set_in_ci = gate.forbid_only.is_some_and(GateFlag::holds_in_ci);
                if ci_present && !set_in_ci && !gate.opaque {
                    findings.push(self.forbid_only(gate, severity));
                }
            }
            // Workflows are judged together, project-wide, after the loop.
            GateTool::Ci | GateTool::Unknown => {}
        }
        // Retries are read from any gate that carries them, at info severity.
        if gate.retries.is_some_and(|n| n > 0) {
            findings.push(self.retries(gate));
        }
    }

    /// Diff each `claims` document line against the config and emit an info finding per line
    /// whose gating claim the config does not back.
    ///
    /// A line is read by the narrow grammar (see [`crate::claims`]). A coverage claim's config
    /// value is a runner gate that declares coverage thresholds; a mutation claim's is a Stryker
    /// `break` above zero. A plain claim is backed by the config value alone. A *CI-scoped* claim
    /// ("enforced in CI", "the pipeline gates on coverage") asks for more — that a CI step
    /// actually run the gate — so it is backed only when the config value is present *and* a CI
    /// gate runs the suite. A backed claim is silent; an unbacked claim is an info finding at the
    /// document line, with the config field it disagrees with as its related location.
    fn diff_claims(&self, model: &Model, config: &Config, findings: &mut Vec<Finding>) {
        let coverage = coverage_backing(model);
        let mutation = mutation_backing(model);
        let ci_runs_suite = ci_runs_the_tests(model);
        for document in &config.claim_documents {
            for (index, line) in document.lines.iter().enumerate() {
                let Some(claim) = classify(line) else {
                    continue;
                };
                let value = match claim.kind {
                    ClaimKind::Coverage => &coverage,
                    ClaimKind::Mutation => &mutation,
                };
                if claim_is_backed(value, claim, ci_runs_suite) {
                    continue;
                }
                let one_based = u32::try_from(index + 1).unwrap_or(u32::MAX);
                findings.push(self.claim_unbacked(
                    &document.path,
                    one_based,
                    line,
                    claim.kind,
                    value.field(),
                ));
            }
        }
    }

    /// An info finding at a document line whose gating claim the config does not back.
    fn claim_unbacked(
        &self,
        path: &str,
        line: u32,
        text: &str,
        kind: ClaimKind,
        field: &ClaimField,
    ) -> Finding {
        let (metric, config_key) = match kind {
            ClaimKind::Coverage => (
                "coverage thresholds",
                "`coverage.thresholds` (Vitest) or `coverageThreshold` (Jest)",
            ),
            ClaimKind::Mutation => ("a mutation score gate", "Stryker's `thresholds.break`"),
        };
        Finding {
            rule: self.name(),
            severity: Severity::Info,
            outcome: Outcome::Finding,
            location: Location {
                file: path.to_owned(),
                line,
                column: 1,
            },
            related: vec![RelatedLocation {
                label: field.label.clone(),
                location: Location {
                    file: field.file.clone(),
                    line: 0,
                    column: 0,
                },
            }],
            message: format!(
                "document claims {metric} are enforced, but the config does not back it"
            ),
            evidence: vec![
                Evidence::new("kind", "claim-unbacked"),
                Evidence::new("claim", text.trim().to_owned()),
                Evidence::new("config", field.file.clone()),
            ],
            explain: format!(
                "This document line claims {metric} are enforced, but {config_key} does not set \
                 an enforcing value, so the claim is a belief the build does not hold anyone to. \
                 Either set the config to enforce what the document promises, or edit the \
                 document to say what is actually true — do not delete the claim without deciding \
                 which of the two is right. Reported for information only; it never fails the \
                 build."
            ),
            // The identity is the claim's own words, not its line: an edit above the line
            // must not re-key the finding, and two identical lines take ordinals.
            anchor: format!("claim-unbacked:{}", normalised_claim(text)),
            test: None,
            ordinal: 0,
            proposal: None,
        }
    }

    /// A could-not-decide row at a runner gate the reader could not see whole: whether it
    /// declares coverage thresholds is unknown, and the row says what could not be read.
    fn config_unread(&self, gate: &Gate) -> Finding {
        let unread = gate.unread.join("; ");
        Finding {
            rule: self.name(),
            severity: Severity::Info,
            outcome: Outcome::CouldNotDecide,
            location: Location::at(gate.source.clone(), spirolo_model::Span::default()),
            related: Vec::new(),
            message: "runner config could not be read statically, so whether it declares \
                      coverage thresholds is undecided"
                .to_owned(),
            evidence: vec![
                Evidence::new("kind", "config-unread"),
                Evidence::new("unread", unread.clone()),
            ],
            explain: format!(
                "The reader could not decide this config statically: {unread}. Nothing was \
                 executed to find out, so whether the runner enforces coverage thresholds is \
                 not known — this row is not a finding and never fails the build. Make the \
                 config readable by AST (an object literal, or a preset the resolver can \
                 follow) so the gate can be judged, or check the thresholds by hand."
            ),
            anchor: "config-unread".to_owned(),
            test: None,
            ordinal: 0,
            proposal: None,
        }
    }

    /// A finding at a runner gate with no coverage thresholds.
    fn thresholds_absent(&self, gate: &Gate, severity: Severity) -> Finding {
        Finding {
            rule: self.name(),
            severity,
            outcome: Outcome::Finding,
            location: Location::at(gate.source.clone(), spirolo_model::Span::default()),
            related: Vec::new(),
            message: "runner config declares no coverage thresholds, so a coverage drop never \
                      fails the build"
                .to_owned(),
            evidence: vec![Evidence::new("kind", "thresholds-absent")],
            explain: "The runner config sets no coverage thresholds, so coverage can fall to \
                      zero under a green build. Set `coverage.thresholds` (Vitest) or \
                      `coverageThreshold` (Jest) to the suite's current numbers and ratchet them \
                      up, so a drop below today's coverage fails the run rather than passing \
                      unnoticed."
                .to_owned(),
            anchor: "thresholds-absent".to_owned(),
            test: None,
            ordinal: 0,
            proposal: None,
        }
    }

    /// One finding at a shared preset that declares no coverage thresholds, counting the
    /// configs that inherit the gap. Its identity is the preset path and the slug, no line.
    fn thresholds_absent_at_preset(
        &self,
        preset: &str,
        configs: usize,
        severity: Severity,
    ) -> Finding {
        let plural = if configs == 1 { "" } else { "s" };
        Finding {
            rule: self.name(),
            severity,
            outcome: Outcome::Finding,
            location: Location::at(preset.to_owned(), spirolo_model::Span::default()),
            related: Vec::new(),
            message: format!(
                "shared preset declares no coverage thresholds for {configs} inheriting \
                 config{plural}, so a coverage drop never fails their build"
            ),
            evidence: vec![
                Evidence::new("kind", "thresholds-absent"),
                Evidence::new("packages", configs.to_string()),
            ],
            explain: format!(
                "This preset sets no coverage thresholds and none of the {configs} config{plural} \
                 that compose it adds any, so coverage can fall to zero under a green build in \
                 every one of them. Set `coverage.thresholds` (Vitest) or `coverageThreshold` \
                 (Jest) in the preset to the suite's current numbers and ratchet them up — one \
                 decision gates every inheriting package — or in a package's own config where \
                 it needs a different bar."
            ),
            anchor: "thresholds-absent".to_owned(),
            test: None,
            ordinal: 0,
            proposal: None,
        }
    }

    /// A finding at a Stryker gate whose `break` is `0`.
    fn mutation_break_zero(&self, gate: &Gate, severity: Severity) -> Finding {
        Finding {
            rule: self.name(),
            severity,
            outcome: Outcome::Finding,
            location: Location::at(gate.source.clone(), spirolo_model::Span::default()),
            related: Vec::new(),
            message: "mutation break is 0, so the mutation score never fails the build".to_owned(),
            evidence: vec![
                Evidence::new("kind", "mutation-break-zero"),
                Evidence::new("break", "0"),
            ],
            explain: "Stryker's `thresholds.break` is 0, so no mutation score, however low, ever \
                      fails the build — the mutation gate is off. Raise `break` to the suite's \
                      current mutation score and ratchet it up, so a regression in test strength \
                      fails the run instead of being recorded and ignored."
                .to_owned(),
            anchor: "mutation-break-zero".to_owned(),
            test: None,
            ordinal: 0,
            proposal: None,
        }
    }

    /// A finding at a Stryker gate whose `break` is `null` or absent: the score is computed
    /// but never gates.
    fn mutation_break_null(&self, gate: &Gate, severity: Severity) -> Finding {
        Finding {
            rule: self.name(),
            severity,
            outcome: Outcome::Finding,
            location: Location::at(gate.source.clone(), spirolo_model::Span::default()),
            related: Vec::new(),
            message: "mutation break is null, so the mutation score is computed but never fails \
                      the build"
                .to_owned(),
            evidence: vec![
                Evidence::new("kind", "mutation-break-null"),
                Evidence::new("break", "null"),
            ],
            explain: "Stryker's `thresholds.break` is `null` (its default when unset), so every \
                      run computes and reports a mutation score and no score, however low, ever \
                      fails the build — the mutation run is a report, not a gate. Set `break` to \
                      the suite's current mutation score and ratchet it up, so a regression in \
                      test strength fails the run instead of being recorded and ignored; if \
                      report-only is the intent, say so where the score is read."
                .to_owned(),
            anchor: "mutation-break-null".to_owned(),
            test: None,
            ordinal: 0,
            proposal: None,
        }
    }

    /// A finding at a Playwright gate that does not forbid `.only` while CI runs.
    fn forbid_only(&self, gate: &Gate, severity: Severity) -> Finding {
        Finding {
            rule: self.name(),
            severity,
            outcome: Outcome::Finding,
            location: Location::at(gate.source.clone(), spirolo_model::Span::default()),
            related: Vec::new(),
            message: "Playwright forbidOnly is not set while CI runs, so a stray .only passes CI \
                      as a run of one test"
                .to_owned(),
            evidence: vec![Evidence::new("kind", "forbid-only")],
            explain: "`forbidOnly` is not enabled, so a `test.only` committed by mistake makes CI \
                      run just that one test and still go green. Set `forbidOnly: true` in the \
                      Playwright config (it defaults to `!!process.env.CI` in the generated \
                      config for exactly this reason), so a committed `.only` fails CI instead of \
                      silently skipping the rest of the suite."
                .to_owned(),
            anchor: "forbid-only".to_owned(),
            test: None,
            ordinal: 0,
            proposal: None,
        }
    }

    /// The one project-wide row when no workflow the project runs executes the suite, or
    /// `None` when there is no workflow or one of them does.
    ///
    /// Located at the workflows directory, never at one workflow, so adding a lint workflow
    /// beside the offending one does not re-key it; every workflow read is named in the
    /// `workflows` evidence. With a workflow the reader could not parse and no readable one
    /// running the suite, the row is could-not-decide and names the unreadable file.
    fn ci_runs_nothing(&self, workflows: &[&Gate], severity: Severity) -> Option<Finding> {
        if workflows.is_empty() || workflows.iter().any(|gate| ci_runs_the_suite(gate)) {
            return None;
        }
        let directory = workflows[0]
            .source
            .rsplit_once('/')
            .map_or(".github/workflows", |(dir, _)| dir)
            .to_owned();
        let names: Vec<&str> = workflows.iter().map(|gate| gate.source.as_str()).collect();
        let steps: usize = workflows.iter().map(|gate| gate.steps.len()).sum();
        let unreadable: Vec<&str> = workflows
            .iter()
            .filter(|gate| gate.opaque)
            .map(|gate| gate.source.as_str())
            .collect();
        let evidence = vec![
            Evidence::new("kind", "ci-runs-nothing"),
            Evidence::new("steps", steps.to_string()),
            Evidence::new("workflows", names.join(", ")),
        ];
        let plural = if names.len() == 1 { "" } else { "s" };
        Some(if unreadable.is_empty() {
            Finding {
                rule: self.name(),
                severity,
                outcome: Outcome::Finding,
                location: Location::at(directory, spirolo_model::Span::default()),
                related: Vec::new(),
                message: format!(
                    "no CI step runs the tests, coverage or mutation in {} workflow{plural}, so \
                     the suite never runs on a pull request",
                    names.len()
                ),
                evidence,
                explain: format!(
                    "None of the project's workflows ({}) has a run step that invokes the test \
                     runner, coverage or Stryker, so whatever the runner config gates, nothing \
                     in CI executes it — a pull request goes green without the suite running. \
                     Add a step that runs the tests (for example `pnpm vitest run --coverage`) \
                     to a job that gates the merge, so the thresholds and skips the rest of \
                     these rules read are actually enforced.",
                    names.join(", ")
                ),
                anchor: "ci-runs-nothing".to_owned(),
                test: None,
                ordinal: 0,
                proposal: None,
            }
        } else {
            Finding {
                rule: self.name(),
                severity: Severity::Info,
                outcome: Outcome::CouldNotDecide,
                location: Location::at(directory, spirolo_model::Span::default()),
                related: Vec::new(),
                message: "whether any CI step runs the tests is undecided: a workflow could not \
                          be read"
                    .to_owned(),
                evidence,
                explain: format!(
                    "No readable workflow has a run step that invokes the test runner, \
                     coverage or Stryker, but {} could not be parsed, so whether CI runs the \
                     suite is not known — this row is not a finding and never fails the build. \
                     Make the workflow valid YAML so it can be judged, or check its steps by \
                     hand.",
                    unreadable.join(", ")
                ),
                anchor: "ci-runs-nothing".to_owned(),
                test: None,
                ordinal: 0,
                proposal: None,
            }
        })
    }

    /// An info finding at any gate that configures retries.
    fn retries(&self, gate: &Gate) -> Finding {
        let count = gate.retries.unwrap_or(0);
        Finding {
            rule: self.name(),
            severity: Severity::Info,
            outcome: Outcome::Finding,
            location: Location::at(gate.source.clone(), spirolo_model::Span::default()),
            related: Vec::new(),
            message: format!(
                "config retries a failing test up to {count} times, which hides flakiness rather \
                 than failing on it"
            ),
            evidence: vec![
                Evidence::new("kind", "retries"),
                Evidence::new("retries", count.to_string()),
            ],
            explain: "The config retries a failing test, so a flaky test that passes on a retry \
                      is recorded as green and its flakiness never surfaces. Retries trade a \
                      failing build for a hidden defect; prefer fixing the flake (seed the clock, \
                      await the state) and dropping the retry, so a flake is a finding rather \
                      than a silent pass. Reported for information only — it never fails the \
                      build."
                .to_owned(),
            anchor: "retries".to_owned(),
            test: None,
            ordinal: 0,
            proposal: None,
        }
    }
}

/// A claim line reduced to the words that make it one, for the finding's identity: trimmed,
/// lower-cased, its whitespace runs collapsed to one space, and a leading Markdown list
/// marker or heading mark dropped — so re-indenting, re-wrapping or re-numbering the
/// document leaves the finding's id where it was.
fn normalised_claim(line: &str) -> String {
    let trimmed = line.trim();
    let body = trimmed
        .trim_start_matches(['-', '*', '+', '#', '>'])
        .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ')')
        .trim();
    let words: Vec<String> = body.split_whitespace().map(str::to_lowercase).collect();
    words.join(" ")
}

/// Whether a claim is backed by the config: the config value must be present, and a CI-scoped
/// claim needs a CI step that runs the suite too.
///
/// A plain claim ("thresholds enforced") asks only that the config value exist. A CI-scoped
/// claim ("thresholds enforced in CI", "the pipeline gates on coverage") asks that CI actually
/// run the gate, so a project whose runner declares thresholds but whose CI never runs the tests
/// does not back it — for example a playbook that says "enforced in CI" while
/// no workflow step runs coverage.
const fn claim_is_backed(value: &ConfigValue, claim: Claim, ci_runs_suite: bool) -> bool {
    value.present && (!claim.ci_scoped || ci_runs_suite)
}

/// The config's coverage value: present when *any* runner gate declares coverage thresholds.
///
/// A claims document speaks for the whole project, so a monorepo where any package enforces
/// thresholds backs a coverage claim — otherwise a per-package ratchet (a common monorepo shape,
/// where the shared base carries none but the packages do) would read as unbacked and every
/// coverage line would be a false positive. The field points at a runner gate that declares
/// thresholds when one exists, else at the first runner gate, else at a bare marker.
fn coverage_backing(model: &Model) -> ConfigValue {
    let runners = || {
        model
            .gates
            .iter()
            .filter(|g| matches!(g.tool, GateTool::Vitest | GateTool::Jest))
    };
    let with_thresholds = runners().find(|g| !g.thresholds.is_empty());
    let present = with_thresholds.is_some();
    let file = with_thresholds
        .or_else(|| runners().next())
        .map_or_else(|| "runner config".to_owned(), |g| g.source.clone());
    ConfigValue {
        present,
        field: ClaimField {
            file,
            label: "coverage thresholds".to_owned(),
        },
    }
}

/// The config's mutation value: present when *any* Stryker gate's `break` is above zero.
///
/// `break: 0` and `break: null` (unset) both leave the score toothless. As with coverage, a
/// monorepo where any package sets a real `break` backs a project-wide mutation claim. The field
/// points at a Stryker gate with a real `break` when one exists, else at the first Stryker gate,
/// else at a bare marker.
fn mutation_backing(model: &Model) -> ConfigValue {
    let strykers = || model.gates.iter().filter(|g| g.tool == GateTool::Stryker);
    let with_break = strykers().find(|g| g.mutation_break.is_some_and(|b| b > 0.0));
    let present = with_break.is_some();
    let file = with_break
        .or_else(|| strykers().next())
        .map_or_else(|| "mutation config".to_owned(), |g| g.source.clone());
    ConfigValue {
        present,
        field: ClaimField {
            file,
            label: "mutation break".to_owned(),
        },
    }
}

/// Whether a CI command selects this browser-test config. `CircleCI` reusable commands and
/// unread workflows are still uncertain; retain their rows until the reader can resolve them.
/// So is a `playwright test` the reader cannot place: behind a runner it does not know
/// (`pnpm --filter web exec`, `bunx`), quoted as another program's argument, or after a `cd`
/// into a computed directory.
fn ci_loads_config(config: &Gate, workflows: &[&Gate]) -> bool {
    workflows.iter().any(|workflow| {
        if workflow.opaque || workflow.source == ".circleci/config.yml" {
            return true;
        }
        workflow.steps.iter().any(|step| {
            let mut directory = Some(String::new());
            for command in step.split(['\n', ';', '&', '|']) {
                let words = command_words(command);
                if words.first() == Some(&"cd") {
                    if let Some(path) = words.get(1) {
                        directory = directory
                            .filter(|_| !path.contains('$'))
                            .map(|directory| joined_path(&directory, path));
                    }
                    continue;
                }
                // A quoted run is another program's argument (`sh -c 'playwright test'`): the
                // config may be the one it loads, so the run is not placed.
                let Some(at) = words.windows(2).position(|pair| {
                    let program = unquoted(pair[0]).split('@').next().unwrap_or_default();
                    (program == "playwright" || program.ends_with("/playwright"))
                        && unquoted(pair[1]) == "test"
                }) else {
                    continue;
                };
                let placed = words[..at].iter().all(|word| {
                    matches!(
                        *word,
                        "npm" | "pnpm" | "yarn" | "npx" | "exec" | "run" | "dlx" | "-y" | "--yes"
                    )
                }) && words[at..=at + 1]
                    .iter()
                    .all(|word| unquoted(word) == *word);
                let Some(directory) = directory.as_deref().filter(|_| placed) else {
                    return true;
                };
                let args = &words[at..];
                // Playwright keeps the last `-c`/`--config` it is given.
                let explicit = args.iter().enumerate().rev().find_map(|(index, arg)| {
                    if matches!(*arg, "--config" | "-c") {
                        args.get(index + 1).copied()
                    } else {
                        arg.strip_prefix("--config=")
                    }
                });
                let (config_dir, config_file) = config
                    .source
                    .rsplit_once('/')
                    .unwrap_or(("", &config.source));
                if let Some(path) = explicit {
                    // A directory loads the `playwright.config.*` inside it.
                    let path = joined_path(directory, path);
                    if path.contains('$')
                        || path == config.source
                        || (path == config_dir && config_file.starts_with("playwright.config."))
                    {
                        return true;
                    }
                } else if config_dir == directory {
                    return true;
                }
            }
            false
        })
    })
}

fn unquoted(word: &str) -> &str {
    word.trim_matches(['"', '\'', '`', '(', ')'])
}

fn joined_path(base: &str, path: &str) -> String {
    let mut parts = Vec::new();
    for part in base
        .split('/')
        .chain(path.trim_matches(['\'', '"']).split('/'))
    {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    parts.join("/")
}

/// Whether any workflow the project runs has a run step that runs the tests, coverage or
/// mutation — a CI-scoped claim needs this to be true. An opaque workflow has no steps and so
/// never counts (we do not claim an unreadable workflow runs the suite), and a project with no
/// workflow at all leaves this false.
fn ci_runs_the_tests(model: &Model) -> bool {
    project_workflows(model).into_iter().any(ci_runs_the_suite)
}

/// The root CI gates: `.circleci/config.yml` and workflow files directly under
/// `.github/workflows`, in source order. A workflow nested in a package
/// (`packages/x/.github/workflows/ci.yml`) is read into the model but never run, so it is
/// neither CI nor evidence that the suite runs.
fn project_workflows(model: &Model) -> Vec<&Gate> {
    model
        .gates
        .iter()
        .filter(|gate| gate.tool == GateTool::Ci && is_project_workflow(&gate.source))
        .collect()
}

/// Whether a workflow belongs to root GitHub Actions or `CircleCI`.
fn is_project_workflow(source: &str) -> bool {
    source == ".circleci/config.yml"
        || source
            .strip_prefix(".github/workflows/")
            .is_some_and(|name| !name.is_empty() && !name.contains('/'))
}

/// Whether a CI gate's run steps run the tests, coverage or mutation at all.
///
/// A step counts when a word of it names a known test runner (`vitest`, `jest`, `playwright`,
/// `cypress`), a mutation run (`stryker`), or a package script that conventionally runs the
/// tests (`test`, `test:`, `test-`). Words match whole, so `pnpm vitest run`, `npm test` and
/// `yarn test:ci` all count while a `pretest` script, a `config-vitest` directory or an
/// `eslint-plugin-jest` does not.
fn ci_runs_the_suite(gate: &Gate) -> bool {
    gate.steps.iter().any(|step| step_runs_the_suite(step))
}

/// The tokens whose presence in a run step means the suite runs.
const RUNNER_TOKENS: [&str; 5] = ["vitest", "jest", "playwright", "stryker", "cypress"];

/// Whether one run-step command runs the tests, coverage or mutation.
fn step_runs_the_suite(step: &str) -> bool {
    let lower = step.to_ascii_lowercase();
    lower
        .split(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '`' | '(' | ')'))
        .any(|word| {
            // A runner by path (`node_modules/.bin/jest`), version (`vitest@3`) or scoped
            // package (`@stryker-mutator/core`), or a script variant (`jest:ci`).
            let program = word.rsplit('/').next().unwrap_or_default();
            let program = program.split(['@', ':']).next().unwrap_or_default();
            RUNNER_TOKENS.contains(&program)
                || word.starts_with("@stryker-mutator/")
                // A package script run: `npm test`, `pnpm test`, `yarn test:ci`, `pnpm run
                // test`. `test` stands as its own token (optionally followed by `:` or `-` for
                // a script variant) so `pretest` or `latest` never counts.
                || word == "test"
                || word.starts_with("test:")
                || word.starts_with("test-")
                || word.contains(":test")
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_vitest_run_with_coverage_counts() {
        assert!(step_runs_the_suite("pnpm vitest run --coverage"));
    }

    #[test]
    fn npm_test_counts() {
        assert!(step_runs_the_suite("npm test"));
        assert!(step_runs_the_suite("yarn test:ci"));
        assert!(step_runs_the_suite("pnpm run test"));
    }

    #[test]
    fn a_stryker_run_counts_as_running_the_tests() {
        assert!(step_runs_the_suite("pnpm exec stryker run"));
    }

    #[test]
    fn lint_and_typecheck_do_not_count() {
        assert!(!step_runs_the_suite("pnpm lint"));
        assert!(!step_runs_the_suite("pnpm tsc --noEmit"));
        assert!(!step_runs_the_suite("pnpm install"));
        // `pretest` and `latest` are false friends of the `test` token.
        assert!(!step_runs_the_suite("pnpm pretest"));
        assert!(!step_runs_the_suite("use the latest node"));
    }
}
