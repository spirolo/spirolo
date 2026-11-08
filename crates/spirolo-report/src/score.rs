//! Provisional health on human and editor surfaces only (PRD 0006, formula v1).
//! This module never changes machine reports, findings, or exit codes.
use std::collections::BTreeSet;

use serde::Serialize;
use spirolo_model::{Model, Span, TestKind};
use spirolo_rules::{status, Family, Finding, Outcome, Rule, Subject};

/// The provisional suite health and the penalties behind it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Health {
    /// Unrounded score; rounding belongs to the display surface.
    pub score: f64,
    /// A, B, C, D or F, selected from the unrounded score.
    pub grade: char,
    /// Version of the frozen caps, saturations, subjects and weights.
    pub formula_version: u32,
    /// Whether a rule contributing a nonzero ratio is not proven.
    pub provisional: bool,
    /// Registered rules satisfying `status::leans_true` in the supplied table.
    pub lean_true: usize,
    /// Registered rules satisfying `status::is_proven` in the supplied table.
    pub proven: usize,
    /// Family penalties in registry order.
    pub parts: Vec<Part>,
    /// Model inventory, independent of findings or the baseline.
    pub facts: Facts,
    /// At most 200 hotspots, ranked independently from the penalty population.
    pub hotspots: Hotspots,
}

/// The score-ranked hotspot display, after the baseline.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Hotspots {
    /// Distinct modules with decided hotspot findings and numeric evidence.
    pub total: usize,
    /// Top 200 by descending score, with path breaking ties.
    pub shown: Vec<Hotspot>,
}

/// One source module's recorded hotspot evidence.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Hotspot {
    /// Repo-relative module path.
    pub path: String,
    /// Recorded score evidence, without recomputation or rounding.
    pub score: f64,
    /// Recorded complexity evidence.
    pub complexity: u32,
    /// Recorded weakness evidence.
    pub weakness: f64,
}

/// Suite inventory shown beside the provisional score.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Facts {
    /// Cases whose kind is `TestKind::Test`.
    pub tests: usize,
    /// Distinct paths in the model's test-file table.
    pub test_files: usize,
    /// Rows in the model's gates table.
    pub gates: usize,
    /// Assertions across all cases, as in the existing summary line.
    pub expects: usize,
    /// Assertions per test; zero when there are no tests.
    pub expects_per_test: f64,
}

/// One family's capped penalty.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Part {
    /// Lowercase family name.
    pub family: &'static str,
    /// Human section title from `Family::title`.
    pub title: &'static str,
    /// Points deducted from 100 by this family.
    pub penalty: f64,
    /// Maximum deduction for this family.
    pub cap: f64,
    /// Sum of the rules' proof-weighted flagged shares.
    pub ratio: f64,
    /// Every rule in this family, in registry order, including zero contributions.
    pub rules: Vec<RulePart>,
}

/// One rule's contribution to its family's ratio.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RulePart {
    /// Registry rule name.
    pub rule: &'static str,
    /// One for proven/advisory rules, one half otherwise.
    pub weight: f64,
    /// Distinct subjects with at least one decided finding.
    pub flagged: usize,
    /// Size of the declared subject population.
    pub subjects: usize,
}

/// Compute formula-v1 health from findings after the baseline and an explicit status table.
/// Pure: no I/O, global status replacement, mutation or rule execution.
pub fn health(findings: &[Finding], model: &Model, status: &status::Table) -> Health {
    let subjects = Subjects::new(findings, model);
    let parts: Vec<_> = [
        Family::Audit,
        Family::Gate,
        Family::Hygiene,
        Family::Flake,
        Family::Impact,
    ]
    .into_iter()
    .map(|family| {
        let rules = spirolo_rules::family_rules(family)
            .into_iter()
            .map(|rule| {
                let (proven, lean_true) = proof(rule.name(), status);
                RulePart {
                    rule: rule.name(),
                    subjects: subjects.count(rule.subject()),
                    flagged: subjects.flagged(rule.as_ref(), findings, model),
                    weight: if proven || lean_true { 1.0 } else { 0.5 },
                }
            })
            .collect();
        family_part(family, rules)
    })
    .collect();
    let score = 100.0 - parts.iter().map(|part| part.penalty).sum::<f64>();
    let provisional = parts
        .iter()
        .flat_map(|part| &part.rules)
        .any(|rule| rule.subjects > 0 && !proof(rule.rule, status).0);
    let registered = spirolo_rules::registry();
    let lean_true = registered
        .iter()
        .filter(|rule| proof(rule.name(), status).1)
        .count();
    let proven = registered
        .iter()
        .filter(|rule| proof(rule.name(), status).0)
        .count();
    let grade = if score >= 85.0 {
        'A'
    } else if score >= 70.0 {
        'B'
    } else if score >= 55.0 {
        'C'
    } else if score >= 40.0 {
        'D'
    } else {
        'F'
    };
    Health {
        score,
        grade,
        facts: subjects.facts,
        hotspots: shown_hotspots(findings),
        formula_version: 1,
        provisional,
        lean_true,
        proven,
        parts,
    }
}

fn family_part(family: Family, rules: Vec<RulePart>) -> Part {
    let ratio = rules
        .iter()
        .map(|rule| rule.weight * share(rule.flagged, rule.subjects))
        .sum::<f64>();
    let (cap, saturation) = match family {
        Family::Audit => (25.0, 0.10),
        Family::Gate => (30.0, 0.20),
        Family::Hygiene => (10.0, 0.10),
        Family::Flake => (20.0, 0.10),
        Family::Impact => (15.0, 1.00),
    };
    Part {
        family: family.name(),
        title: family.title(),
        cap,
        ratio,
        rules,
        penalty: cap * (ratio / saturation).min(1.0),
    }
}

// Populations are model facts, not counts inferred from how many rows a rule emitted.
// Hotspots are the exception specified by v1: their population uses complexity evidence.
struct Subjects<'a> {
    facts: Facts,
    runs: BTreeSet<&'a str>,
    hotspots: Vec<&'a Finding>,
    flagged_hotspots: BTreeSet<&'a str>,
}

impl<'a> Subjects<'a> {
    fn new(findings: &'a [Finding], model: &'a Model) -> Self {
        let tests = model
            .test_cases
            .iter()
            .filter(|case| case.kind == TestKind::Test)
            .count();
        let expects = model.assertion_count();
        Self {
            facts: Facts {
                tests,
                expects,
                expects_per_test: share(expects, tests),
                gates: model.gates.len(),
                test_files: model
                    .test_files
                    .iter()
                    .map(|file| &file.file)
                    .collect::<BTreeSet<_>>()
                    .len(),
            },
            runs: model.runs.iter().map(|run| run.run_id.as_str()).collect(),
            hotspots: complex_modules(findings),
            flagged_hotspots: findings
                .iter()
                .filter(|f| {
                    f.rule == "untested-hotspot"
                        && f.outcome == Outcome::Finding
                        && numeric(f, "weakness").is_some_and(|weakness| weakness >= 0.5)
                })
                .map(|f| f.location.file.as_str())
                .collect(),
        }
    }

    fn count(&self, subject: Subject) -> usize {
        match subject {
            Subject::Tests => self.facts.tests,
            Subject::TestFiles => self.facts.test_files,
            Subject::Gates => self.facts.gates,
            Subject::Runs => self.runs.len(),
            Subject::HotspotModules => self.hotspots.len(),
        }
    }

    fn flagged(&self, rule: &dyn Rule, findings: &[Finding], model: &Model) -> usize {
        let mine = findings
            .iter()
            .filter(|f| f.rule == rule.name() && f.outcome == Outcome::Finding);
        match rule.subject() {
            Subject::Tests => mine
                .flat_map(|f| test_subjects(f, model))
                .collect::<BTreeSet<_>>()
                .len(),
            Subject::TestFiles | Subject::Gates => mine
                .map(|f| f.location.file.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            Subject::HotspotModules => self
                .hotspots
                .iter()
                .filter(|f| self.flagged_hotspots.contains(f.location.file.as_str()))
                .count(),
            Subject::Runs => mine
                .flat_map(|f| {
                    f.evidence
                        .iter()
                        .filter(|e| matches!(e.key.as_str(), "passed_runs" | "failed_runs"))
                })
                .flat_map(|e| e.value.split(", "))
                .filter(|run| self.runs.contains(run))
                .collect::<BTreeSet<_>>()
                .len(),
        }
    }
}

// Use the supplied table with exactly status::is_proven / status::leans_true's policy.
fn proof(rule: &str, table: &status::Table) -> (bool, bool) {
    table.rules.get(rule).map_or((false, false), |row| {
        (
            row.verdict == status::Verdict::Pass,
            row.verdict != status::Verdict::Fail
                && row
                    .interval
                    .is_some_and(|[low, _]| low >= status::LEANS_TRUE),
        )
    })
}

#[allow(clippy::cast_precision_loss)] // Repository populations are far below f64's exact integer limit.
fn share(flagged: usize, subjects: usize) -> f64 {
    if subjects == 0 {
        0.0
    } else {
        flagged as f64 / subjects as f64
    }
}

fn numeric(finding: &Finding, key: &str) -> Option<f64> {
    finding
        .evidence
        .iter()
        .find(|e| e.key == key)?
        .value
        .parse::<f64>()
        .ok()
        .filter(|n| n.is_finite())
}

fn complex_modules(findings: &[Finding]) -> Vec<&Finding> {
    let mut modules: Vec<_> = findings
        .iter()
        .filter(|f| {
            f.rule == "untested-hotspot"
                && numeric(f, "complexity").is_some()
                && numeric(f, "weakness").is_some()
        })
        .collect();
    modules.sort_by(|a, b| {
        a.location.file.cmp(&b.location.file).then_with(|| {
            numeric(b, "complexity")
                .unwrap()
                .total_cmp(&numeric(a, "complexity").unwrap())
        })
    });
    modules.dedup_by(|a, b| a.location.file == b.location.file);
    modules.sort_by(|a, b| {
        numeric(b, "complexity")
            .unwrap()
            .total_cmp(&numeric(a, "complexity").unwrap())
            .then(a.location.file.cmp(&b.location.file))
    });
    modules.truncate(200);
    modules
}

fn shown_hotspots(findings: &[Finding]) -> Hotspots {
    let mut shown: Vec<_> = findings
        .iter()
        .filter(|f| f.rule == "untested-hotspot" && f.outcome == Outcome::Finding)
        .filter_map(|f| {
            Some(Hotspot {
                path: f.location.file.clone(),
                score: numeric(f, "score")?,
                weakness: numeric(f, "weakness")?,
                complexity: f
                    .evidence
                    .iter()
                    .find(|e| e.key == "complexity")?
                    .value
                    .parse()
                    .ok()?,
            })
        })
        .collect();
    shown.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then_with(|| b.score.total_cmp(&a.score))
    });
    shown.dedup_by(|a, b| a.path == b.path);
    shown.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.path.cmp(&b.path)));
    let total = shown.len();
    shown.truncate(200);
    Hotspots { total, shown }
}

type TestSubject<'a> = (&'a str, &'a [String], Option<&'a str>);

fn test_subjects<'a>(finding: &'a Finding, model: &'a Model) -> Vec<TestSubject<'a>> {
    if let Some(test) = &finding.test {
        return vec![(
            &finding.location.file,
            &test.suite_path,
            test.title.as_deref(),
        )];
    }
    let at = &finding.location;
    match finding.rule {
        // generator-walk's frozen finding carries a site rather than a TestPath.
        // Join the recorded site to its enclosing case using model spans, without parsing anchors.
        "generator-walk" => model
            .saga_walks
            .iter()
            .find(|walk| {
                walk.file == at.file && walk.span.line == at.line && walk.span.column == at.column
            })
            .and_then(|walk| enclosing_test(model, &walk.file, walk.span))
            .into_iter()
            .collect(),
        // handler-drift is located at the handler. One written inside a case flags that case;
        // one in a module flags every test of each test file whose resolved imports reach it.
        // A handler loaded only through a runner setup file reaches no test this way.
        "handler-drift" => {
            let Some(handler) = model.handlers.iter().find(|handler| {
                handler.file == at.file
                    && handler.span.line == at.line
                    && handler.span.column == at.column
            }) else {
                return Vec::new();
            };
            if let Some(case) = enclosing_test(model, &handler.file, handler.span) {
                return vec![case];
            }
            let files = model.test_distances(&[handler.file.as_str()]);
            model
                .test_cases
                .iter()
                .filter(|case| case.kind == TestKind::Test && files.contains_key(&case.file))
                .map(|case| {
                    (
                        case.file.as_str(),
                        case.suite_path.as_slice(),
                        case.title.as_deref(),
                    )
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

/// The innermost non-suite case around `span` in `file`, when that case is a test.
fn enclosing_test<'a>(model: &'a Model, file: &str, span: Span) -> Option<TestSubject<'a>> {
    let case = model
        .test_cases
        .iter()
        .filter(|case| {
            case.file == file
                && case.kind != TestKind::Suite
                && case.span.start <= span.start
                && span.end <= case.span.end
        })
        .min_by_key(|case| case.span.end - case.span.start)?;
    (case.kind == TestKind::Test).then_some((
        case.file.as_str(),
        case.suite_path.as_slice(),
        case.title.as_deref(),
    ))
}
