//! `untested-hotspot`: source modules ranked by churn × complexity × weakness (PRD 0005),
//! so test effort (a Stryker run, a hardening pass) is aimed where it pays.
//!
//! - `churn` = lines added + deleted for the module over the supplied `git log --numstat`
//!   input, + 1; with no churn input, 1 for every module.
//! - `complexity` = the model's per-module count (`Module::complexity`).
//! - `weakness` = 1.0 when no test file reaches the module, by import closure or pairing;
//!   otherwise the share of its named value exports (type exports count on neither side)
//!   no reaching test file references by identifier, floored at 0.1; 0.5 for a reached
//!   module with no named exports.
//!
//! One `info` finding per candidate with a score above zero, except a reached module whose
//! every named value export its tests reference (#456),
//! ordered by score descending then path: the finding order *is* the ranking, and
//! `impact --hotspots` prints the same one. Blind to mutation data by construction: it reads the `Module`, `TestCase`,
//! `Pairing` and `Churn` tables and nothing else.

use std::collections::{BTreeSet, HashMap, HashSet};

use spirolo_model::{Diagnostic, ExportKind, Language, Model, Module, Severity as Level};

use crate::finding::{Evidence, Finding, Location, Outcome};
use crate::{Config, Family, Rule, Severity};

/// The least a reached module's weakness can be while one of its named value exports goes
/// unreferenced. A module whose tests reference all of them is not ranked: the rule has no
/// evidence its tests are weak there (#456).
const FLOOR: f64 = 0.1;

/// The weakness of a reached module with no named exports, where no reference can be
/// counted either way.
const NO_NAMED_EXPORTS: f64 = 0.5;

/// The `untested-hotspot` rule. See the module docs for the semantics.
pub struct UntestedHotspot;

/// One ranked source module and the three factors its score multiplies.
#[derive(Debug, Clone, PartialEq)]
pub struct Hotspot {
    /// Repo-relative path of the module.
    pub file: String,
    /// Lines added + deleted over the churn input, + 1.
    pub churn: u64,
    /// Whether a churn input was read. `false` means `churn` is the placeholder 1, and the
    /// finding says so rather than presenting it as a measured factor.
    pub churn_known: bool,
    /// The module's complexity.
    pub complexity: u32,
    /// How little of the module its tests touch, from 0.1 to 1.0.
    pub weakness: f64,
    /// The share of its named value exports the reaching tests reference, below 1.0;
    /// `None` when no test reaches it or it has no named exports.
    pub referenced: Option<f64>,
    /// `churn × complexity × weakness`.
    pub score: f64,
}

/// What the hotspot ranking says about a model.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Hotspots {
    /// Every candidate with a score above zero and a named value export its tests leave
    /// unreferenced, by score descending then path.
    pub hotspots: Vec<Hotspot>,
    /// `untested_hotspot_no_churn` when the model has no churn input.
    pub diagnostics: Vec<Diagnostic>,
}

/// Rank the model's source modules by churn × complexity × weakness.
pub fn hotspots(model: &Model) -> Hotspots {
    let churns: HashMap<&str, u64> = model
        .churn
        .iter()
        .flatten()
        .map(|row| (row.path.as_str(), row.added + row.deleted))
        .collect();
    let churn_known = model.churn.is_some();
    let tests: HashSet<&str> = model.test_cases.iter().map(|c| c.file.as_str()).collect();
    let test_reach = model.test_reach();
    let mut referenced: HashMap<&str, BTreeSet<&str>> = HashMap::new();
    for row in &test_reach {
        referenced
            .entry(row.module.as_str())
            .or_default()
            .extend(row.referenced.iter().map(String::as_str));
    }
    let mut hotspots: Vec<Hotspot> = model
        .modules
        .iter()
        .filter(|module| is_candidate(module, &tests))
        .filter_map(|module| {
            let churn = churns.get(module.path.as_str()).copied().unwrap_or(0) + 1;
            let reach = reach(module, referenced.get(module.path.as_str()))?;
            let weakness = weakness(reach);
            Some(Hotspot {
                file: module.path.clone(),
                churn,
                churn_known,
                complexity: module.complexity,
                weakness,
                referenced: match reach {
                    Reach::Share(share) => Some(share),
                    Reach::Unreached | Reach::NoNamedExports => None,
                },
                // A line count far below 2^52: the cast is exact.
                #[allow(clippy::cast_precision_loss)]
                score: churn as f64 * f64::from(module.complexity) * weakness,
            })
        })
        .filter(|hotspot| hotspot.score > 0.0)
        .collect();
    hotspots.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.file.cmp(&b.file)));
    Hotspots {
        hotspots,
        diagnostics: churn_missing(model).into_iter().collect(),
    }
}

/// The one diagnostic a model with no churn input earns: every module's churn was counted
/// as 1, so the ranking is complexity × weakness alone. `None` when a churn input was read,
/// even an empty one.
pub fn churn_missing(model: &Model) -> Option<Diagnostic> {
    model.churn.is_none().then(|| Diagnostic {
        code: "untested_hotspot_no_churn".to_owned(),
        severity: Level::Info,
        file: None,
        span: None,
        message: "no churn input: untested-hotspot ranks by complexity × weakness alone. Save \
                  `git log --numstat --format= --since=180.days` to a file and pass it with \
                  `--churn <file>` or name it in `inputs.churn`"
            .to_owned(),
    })
}

impl Rule for UntestedHotspot {
    fn name(&self) -> &'static str {
        "untested-hotspot"
    }

    fn family(&self) -> Family {
        Family::Impact
    }

    fn subject(&self) -> crate::Subject {
        crate::Subject::HotspotModules
    }

    fn summary(&self) -> &'static str {
        "A source module's churn and complexity disagree with how weakly its tests pin it, ranking it where test effort pays."
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    /// Findings in ranking order (score descending, then path), not the crate's file order:
    /// the order is what the rule says.
    fn run(&self, model: &Model, config: &Config) -> Vec<Finding> {
        let severity = config.severity(self.name(), self.default_severity());
        hotspots(model)
            .hotspots
            .into_iter()
            .map(|hotspot| self.finding(hotspot, severity))
            .collect()
    }
}

impl UntestedHotspot {
    /// The finding for one ranked module, at its head, carrying the three factors and the
    /// score.
    fn finding(&self, hotspot: Hotspot, severity: Severity) -> Finding {
        let Hotspot {
            file,
            churn,
            churn_known,
            complexity,
            weakness,
            referenced,
            score,
        } = hotspot;
        let (message, changes) = if churn_known {
            (
                format!(
                    "{file} scores {score:.2}: churn {churn} × complexity {complexity} × \
                     weakness {weakness:.2}"
                ),
                format!("changes often (churn {churn}), "),
            )
        } else {
            (
                format!(
                    "{file} scores {score:.2}: complexity {complexity} × weakness {weakness:.2} \
                     (no churn input)"
                ),
                "has no churn input, so it ranks on complexity and weakness alone; it ".to_owned(),
            )
        };
        // Below half weak, the tests reference most of the module: say how much rather than
        // "little", from the unfloored share.
        let touched = match referenced {
            Some(share) if weakness < 0.5 => format!(
                "its tests reference {:.0}% of its named value exports",
                share * 100.0
            ),
            _ => "its tests touch little of it".to_owned(),
        };
        Finding {
            rule: self.name(),
            severity,
            outcome: Outcome::Finding,
            location: Location {
                file: file.clone(),
                line: 1,
                column: 1,
            },
            related: Vec::new(),
            message,
            evidence: vec![
                Evidence::new("churn", churn.to_string()),
                Evidence::new("complexity", complexity.to_string()),
                Evidence::new("weakness", format!("{weakness:.2}")),
                Evidence::new("score", format!("{score:.2}")),
            ],
            explain: format!(
                "{file} {changes}branches a lot (complexity \
                 {complexity}) and {touched} (weakness {weakness:.2}: 1.00 \
                 when no test reaches it, otherwise the share of its named exports no test \
                 references). It ranks high among the places where a test, or a mutation run, \
                 pays. Advisory: it never fails the build."
            ),
            anchor: "untested-hotspot".to_owned(),
            test: None,
            ordinal: 0,
            proposal: None,
        }
    }
}

/// Whether `module` is a product source module the rule ranks: JavaScript or TypeScript,
/// not a test file (one the `TestCase` table has a row in, or one named like a test the way
/// pairing reads names), not a declaration file, not a tool's rc or config file, not an
/// example, doc, story, fixture or benchmark, and not generated (a generator's header or a
/// `.umd.js` / `.min.js` bundle) (#456): no test is owed to those.
fn is_candidate(module: &Module, tests: &HashSet<&str>) -> bool {
    let path = module.path.as_str();
    let name = path.rsplit('/').next().unwrap_or(path);
    matches!(
        module.language,
        Language::TypeScript | Language::Tsx | Language::JavaScript | Language::Jsx
    ) && !module.generated
        && !tests.contains(path)
        && !name.contains(".test.")
        && !name.contains(".spec.")
        && !path.split('/').any(|segment| segment == "__tests__")
        && ![".d.ts", ".d.mts", ".d.cts"]
            .iter()
            .any(|suffix| name.ends_with(suffix))
        && !is_tool_config(name)
        && !path.split('/').any(is_non_product_dir)
        && ![".stories.", ".story.", ".runtimeTests."]
            .iter()
            .any(|infix| name.contains(infix))
        && ![".umd.js", ".min.js"]
            .iter()
            .any(|suffix| name.ends_with(suffix))
}

/// Whether a file name is a tool's rc or config file: `vitest.config.ts`, a leading-dot
/// name (`.detoxrc.js`, `.eslintrc.js`) or a `-config` script (`gatsby-config.js`).
fn is_tool_config(name: &str) -> bool {
    name.contains(".config.")
        || name.starts_with('.')
        || [".js", ".cjs", ".mjs"].iter().any(|ext| {
            name.strip_suffix(ext)
                .is_some_and(|stem| stem.ends_with("-config"))
        })
}

/// Whether a directory segment holds code that is not the product: examples, docs,
/// stories, fixtures, benchmarks and end-to-end or integration test apps.
fn is_non_product_dir(segment: &str) -> bool {
    matches!(
        segment,
        "examples"
            | "example"
            | "docs"
            | "stories"
            | "__stories__"
            | "__fixtures__"
            | "fixtures"
            | "integration_tests"
            | "benchmarks"
            | "e2e"
    ) || segment.starts_with("examples-")
}

/// What the reaching tests reference of a module that is ranked.
#[derive(Debug, Clone, Copy)]
enum Reach {
    /// No test file reaches it.
    Unreached,
    /// Test files reach it, but it has no named exports to reference.
    NoNamedExports,
    /// The share of its named value exports the reaching tests reference, below 1.0.
    Share(f64),
}

/// What the reaching tests reference of `module`; `None` when they reference every named
/// value export, so the module is not ranked. Decided on the count, not the floored
/// weakness, so 9 of 10 referenced stays ranked at the floor. `referenced` is the union of
/// the named exports the reaching test files reference, `None` when no test file reaches
/// the module.
fn reach(module: &Module, referenced: Option<&BTreeSet<&str>>) -> Option<Reach> {
    let Some(referenced) = referenced else {
        return Some(Reach::Unreached);
    };
    let named: BTreeSet<&str> = module
        .exports
        .iter()
        .filter(|export| export.kind == ExportKind::Named)
        .map(|export| export.name.as_str())
        .collect();
    if named.is_empty() {
        return Some(Reach::NoNamedExports);
    }
    let touched = named.intersection(referenced).count();
    #[allow(clippy::cast_precision_loss)]
    (touched < named.len()).then(|| Reach::Share(touched as f64 / named.len() as f64))
}

/// How little of a module its tests touch: 1.0 when no test file reaches it,
/// [`NO_NAMED_EXPORTS`] when it has no named exports, otherwise the share of its named
/// value exports no reaching test references, floored at [`FLOOR`].
fn weakness(reach: Reach) -> f64 {
    match reach {
        Reach::Unreached => 1.0,
        Reach::NoNamedExports => NO_NAMED_EXPORTS,
        Reach::Share(share) => (1.0 - share).max(FLOOR),
    }
}
