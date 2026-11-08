//! `generator-walk`: the places a saga is tested by stepping its generator, counted as a
//! number a maintainer can bring down.
//!
//! Two idioms yield a site (see the saga recogniser in `spirolo-frameworks`): a
//! `expect(gen.next().value).toEqual(call(api, 1))` assertion against an effect creator
//! imported from `redux-saga/effects`, and each `.next(…)` step of a
//! `testSaga(saga).next().put(a).next()…` chain from `redux-saga-test-plan`. Both assert a
//! saga's yielded effects in order, which couples the test to the saga's internal step
//! sequence: any reorder or refactor breaks the test without a behaviour changing, and the
//! test proves nothing about what the saga does, only how it is written.
//!
//! The rule reports one `info` finding per site — never an error, never a could-not-decide —
//! so the count is visible and ratchetable while the explain steers towards `expectSaga(…)
//! .run()` and branch testing rather than deleting the assertion. Beside the sites, one
//! [`Outcome::Listed`] count row per file (anchor `count`, evidence `sites` and `site_list`)
//! carries the number the PRD asks for, "a count with the sites": a listing for the tally,
//! never a finding for precision, never failing the build.
//!
//! A site's identity is anchored on the test it sits in (`suite › title`), the idiom and
//! the effect creator, with the collision ordinal counting the yields of that effect within
//! that test: inserting a site in another test, or a test above, does not re-key it. A site
//! outside any test (a helper at module scope) anchors on the idiom and the effect alone,
//! numbered within the file.

use spirolo_model::{Model, SagaStepKind, SagaWalk, TestCase, TestKind};

use std::collections::BTreeMap;

use crate::finding::{number_collisions, sort, Evidence, Finding, Location, Outcome};
use crate::{Config, Family, Rule, Severity};

/// The `generator-walk` rule. See the module docs for the semantics.
pub struct GeneratorWalk;

impl Rule for GeneratorWalk {
    fn name(&self) -> &'static str {
        "generator-walk"
    }

    fn family(&self) -> Family {
        Family::Audit
    }

    fn subject(&self) -> crate::Subject {
        crate::Subject::Tests
    }

    fn summary(&self) -> &'static str {
        "A saga test steps the generator effect by effect instead of running the saga against its outcome, so the test pins the implementation rather than the behaviour."
    }

    fn default_severity(&self) -> Severity {
        // A step-walk is a design smell, not a defect: reported as a count, never failing a
        // build. Overridable per project like every other rule.
        Severity::Info
    }

    fn run(&self, model: &Model, config: &Config) -> Vec<Finding> {
        let severity = config.severity(self.name(), self.default_severity());
        let mut findings: Vec<Finding> = model
            .saga_walks
            .iter()
            .map(|walk| self.finding(walk, enclosing_case(model, walk), severity))
            .collect();
        findings.extend(self.counts(model));

        sort(&mut findings);
        number_collisions(&mut findings);
        findings
    }
}

impl GeneratorWalk {
    /// The finding for one step-walk site: located at the site, naming the idiom and the
    /// effect, with an explain that points to running the saga rather than deleting the
    /// assertion. `case` is the test the site sits in, when one encloses it.
    fn finding(&self, walk: &SagaWalk, case: Option<&TestCase>, severity: Severity) -> Finding {
        let idiom = idiom_name(walk.kind);
        let effect = walk.effect.clone().unwrap_or_else(|| "<none>".to_owned());
        let site = format!("{idiom}:{effect}");
        let anchor = case.map_or_else(
            || site.clone(),
            |case| format!("{} › {site}", anchor_of(case)),
        );
        Finding {
            rule: self.name(),
            severity,
            outcome: Outcome::Finding,
            location: Location::at(walk.file.clone(), walk.span),
            related: Vec::new(),
            message: format!(
                "saga step-walk ({idiom}): asserts the yielded {effect} effect in order"
            ),
            evidence: vec![
                Evidence::new("effect", effect.clone()),
                Evidence::new("idiom", idiom.to_owned()),
            ],
            explain: format!(
                "This site steps a saga's generator and asserts its yielded {effect} effect \
                 in order ({idiom} idiom): it breaks on any reorder and proves how the saga \
                 is written, not what it does. Prefer `expectSaga(saga).run()` with branch \
                 testing, which asserts the effects the saga dispatches without pinning their \
                 order — rewrite the test to run the saga, do not remove its assertions.",
            ),
            anchor,
            test: None,
            ordinal: 0,
            proposal: None,
        }
    }

    /// One count row per file that carries a site: located at the file's first site, the
    /// number of sites and their list (`line:idiom:effect`) in the evidence.
    fn counts(&self, model: &Model) -> Vec<Finding> {
        let mut by_file: BTreeMap<&str, Vec<&SagaWalk>> = BTreeMap::new();
        for walk in &model.saga_walks {
            by_file.entry(walk.file.as_str()).or_default().push(walk);
        }
        by_file
            .into_iter()
            .map(|(file, mut walks)| {
                walks.sort_by_key(|walk| walk.span);
                let sites = walks.len();
                let site_list = walks
                    .iter()
                    .map(|walk| {
                        format!(
                            "{}:{}:{}",
                            walk.span.line,
                            idiom_name(walk.kind),
                            walk.effect.as_deref().unwrap_or("<none>")
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                let first = walks[0];
                Finding {
                    rule: self.name(),
                    severity: Severity::Info,
                    outcome: Outcome::Listed,
                    location: Location::at(file.to_owned(), first.span),
                    related: Vec::new(),
                    message: format!("{sites} saga step-walk sites in this file"),
                    evidence: vec![
                        Evidence::new("kind", "walk-count"),
                        Evidence::new("site_list", site_list),
                        Evidence::new("sites", sites.to_string()),
                    ],
                    explain: format!(
                        "Listed for the count: this file steps a saga's generator at {sites} \
                         sites, each reported beside this row. A count row is not a finding \
                         and never fails the build; bring the number down by rewriting the \
                         tests to `expectSaga(saga).run()`, not by removing assertions."
                    ),
                    anchor: "count".to_owned(),
                    test: None,
                    ordinal: 0,
                    proposal: None,
                }
            })
            .collect()
    }
}

/// The innermost test, hook, benchmark or step whose span encloses the site, in the same
/// file; `None` for a site at module scope or inside a helper outside any case.
fn enclosing_case<'a>(model: &'a Model, walk: &SagaWalk) -> Option<&'a TestCase> {
    model
        .test_cases
        .iter()
        .filter(|case| {
            case.file == walk.file
                && case.kind != TestKind::Suite
                && case.span.start <= walk.span.start
                && walk.span.end <= case.span.end
        })
        .min_by_key(|case| case.span.end - case.span.start)
}

/// The semantic anchor for a case: its suite path joined with its title, the shape the other
/// rules use, so the identity is stable across runs and unrelated edits.
fn anchor_of(case: &TestCase) -> String {
    let mut parts = case.suite_path.clone();
    parts.push(
        case.title
            .clone()
            .unwrap_or_else(|| format!("<dynamic>:{}", case.span.line)),
    );
    parts.join(" › ")
}

/// The lower-case name of a step-walk idiom, for the message, the evidence and the anchor.
const fn idiom_name(kind: SagaStepKind) -> &'static str {
    match kind {
        SagaStepKind::GenNext => "gen.next",
        SagaStepKind::TestSaga => "testSaga",
    }
}
