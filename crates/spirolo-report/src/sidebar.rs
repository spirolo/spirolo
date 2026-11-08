//! The sidebar's data: PRD 0006's `spirolo/tree`, `spirolo/rules` and `spirolo/file` row spec.
//!
//! Pure functions over the model and the published findings (after the baseline, exactly the
//! set `check --format json` prints). The findings are counted by their editor severity, so the
//! sidebar never shows a rule louder than its diagnostics.

use std::collections::{BTreeMap, BTreeSet};

use spirolo_model::{Model, MutantStatus, Mutants};
use spirolo_rules::{status, Family, Finding};

use crate::editor::{editor_severity, EditorSeverity};
use serde::Serialize;

/// `spirolo/rules`: published findings grouped by family, then measured rule tier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SidebarRules {
    /// The asked rules, in family, tier, editor state and registry order.
    pub rules: Vec<RuleNode>,
}

/// Which rules `spirolo/rules` answers, and whether their files come with them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RulesScope {
    /// Every rule of every family, empty ones included, without files.
    Summary,
    /// That family's rules, empty ones included, with their files.
    Family(Family),
    /// That one rule, by registry name, with its files.
    Rule(&'static str),
}

impl RulesScope {
    /// The scope `spirolo/rules` params `{family?, rule?}` name; an unknown name is an error.
    pub fn parse(family: Option<&str>, rule: Option<&str>) -> Result<Self, String> {
        match (family, rule) {
            (None, None) => Ok(Self::Summary),
            (Some(family), None) => spirolo_rules::registry()
                .iter()
                .map(|rule| rule.family())
                .find(|candidate| candidate.name() == family)
                .map(Self::Family)
                .ok_or_else(|| format!("unknown family: {family}")),
            (None, Some(rule)) => spirolo_rules::rule_named(rule)
                .map(|rule| Self::Rule(rule.name()))
                .ok_or_else(|| format!("unknown rule: {rule}")),
            (Some(_), Some(_)) => {
                Err("spirolo/rules takes a family or a rule, not both".to_owned())
            }
        }
    }
}

/// One rule and its published findings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuleNode {
    /// Registry rule name.
    pub rule: &'static str,
    /// The rule's `Family::name()`.
    pub family: &'static str,
    /// `proven`, `advisory` or `unproven` (an experimental, DROPPED or FAIL rule), by
    /// [`spirolo_rules::status::tier`].
    pub tier: &'static str,
    /// The bare status line: no summary and no "measured on" corpus list.
    pub status: String,
    /// The rule's repair-oriented summary.
    pub summary: &'static str,
    /// Published rows by editor severity, including impact and project findings.
    pub counts: Counts,
    /// Strongest editor state.
    pub state: &'static str,
    /// Files in path order, or descending score for untested-hotspot; `None` in the summary.
    pub files: Option<Vec<RuleFile>>,
    /// How many files (the project entry included) carry published rows.
    pub file_count: usize,
}

/// One file within a rule, or the project entry with an empty path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuleFile {
    /// Repo-relative path; empty for project findings.
    pub path: String,
    /// Published rows by editor severity.
    pub counts: Counts,
    /// Strongest editor state.
    pub state: &'static str,
    /// Findings in line and identity order.
    pub rows: Vec<RuleRow>,
}

/// One published finding, with its one-based source line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuleRow {
    /// One-based line (zero for a project row).
    pub line: u32,
    /// Stable finding identity.
    pub id: String,
    /// The finding's message.
    pub message: String,
    /// LSP editor severity, 1 through 4.
    pub severity: u8,
}

/// Group the last good check's published findings without changing their severity.
pub fn sidebar_rules(_model: &Model, findings: &[Finding], scope: RulesScope) -> SidebarRules {
    let mut rules: Vec<_> = spirolo_rules::registry()
        .into_iter()
        .filter(|rule| match scope {
            RulesScope::Summary => true,
            RulesScope::Family(family) => rule.family() == family,
            RulesScope::Rule(name) => rule.name() == name,
        })
        .map(|rule| {
            let mine: Vec<_> = findings
                .iter()
                .filter(|f| f.rule == rule.name() && editor_severity(f).is_some())
                .collect();
            let mut counts = Counts::default();
            let files = crate::findings::ordered_files(&mine, rule.name() == "untested-hotspot")
                .into_iter()
                .map(|(path, findings)| {
                    let mut file_counts = Counts::default();
                    let mut rows: Vec<_> = findings
                        .into_iter()
                        .filter_map(|finding| {
                            let severity = editor_severity(finding)?;
                            file_counts.count(severity);
                            Some(RuleRow {
                                line: finding.location.line,
                                id: finding.id(),
                                message: finding.message.clone(),
                                severity: severity.code(),
                            })
                        })
                        .collect();
                    rows.sort_by(|a, b| (a.line, &a.id).cmp(&(b.line, &b.id)));
                    counts.add(file_counts);
                    RuleFile {
                        path: path.clone(),
                        counts: file_counts,
                        state: file_counts.state(),
                        rows,
                    }
                })
                .collect::<Vec<_>>();
            (
                rule.family(),
                RuleNode {
                    rule: rule.name(),
                    family: rule.family().name(),
                    tier: match sidebar_tier(rule.name()) {
                        0 => "proven",
                        1 => "advisory",
                        _ => "unproven",
                    },
                    status: crate::findings::header_status(rule.name()),
                    summary: rule.summary(),
                    counts,
                    state: counts.state(),
                    file_count: files.len(),
                    files: match scope {
                        RulesScope::Summary => None,
                        RulesScope::Family(_) | RulesScope::Rule(_) => Some(files),
                    },
                },
            )
        })
        .collect();
    rules.sort_by_key(|(family, rule)| {
        (
            *family,
            sidebar_tier(rule.rule),
            match rule.state {
                "error" => 0,
                "warn" => 1,
                "info" => 2,
                _ => 3,
            },
        )
    });
    SidebarRules {
        rules: rules.into_iter().map(|(_, rule)| rule).collect(),
    }
}

/// The rule's rank by its published `tier` word: the human view's [`crate::findings::tier`],
/// with experimental, DROPPED and FAIL rules sharing `unproven`, so they sort by state together.
fn sidebar_tier(rule: &str) -> u8 {
    crate::findings::tier(rule).min(2)
}

/// `spirolo/tree`: the header and the tree of test files and directories.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SidebarTree {
    /// The absolute project root.
    pub root: String,
    /// Distinct test files per framework; `vitest`, `jest` and `playwright` are always present.
    pub tests: BTreeMap<String, usize>,
    /// Entries in `.spirolo/baseline.json`, or `None` when there is none.
    pub baseline: Option<usize>,
    /// The project's handlers and its publishable `handler-drift` findings.
    pub handlers: Handlers,
    /// The project-level findings (`file: ""`), counted like a node, or `None` when there are
    /// none. They belong to no file, so they are no node and no diagnostic.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<Project>,
    /// Test files, files with a non-`impact` finding, and their directories, sorted by path.
    pub nodes: Vec<Node>,
}

/// `spirolo/tree`'s project row: the findings on no file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Project {
    /// Publishable project-level diagnostics by editor severity.
    pub counts: Counts,
    /// `error`, `warn` or `info`, from the counts.
    pub state: &'static str,
}

/// One entry of the tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Node {
    /// The repo-relative path.
    pub path: String,
    /// `file` or `dir`.
    pub kind: &'static str,
    /// Publishable diagnostics by editor severity, `impact` excluded; a directory sums its files.
    pub counts: Counts,
    /// `error`, `warn`, `info` or `ok`, from the counts.
    pub state: &'static str,
}

/// A node's diagnostics by editor severity.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Counts {
    /// Errors.
    pub error: u32,
    /// Warnings.
    pub warn: u32,
    /// Information.
    pub info: u32,
    /// Hints.
    pub hint: u32,
}

impl Counts {
    const fn count(&mut self, severity: EditorSeverity) {
        match severity {
            EditorSeverity::Error => self.error += 1,
            EditorSeverity::Warning => self.warn += 1,
            EditorSeverity::Information => self.info += 1,
            EditorSeverity::Hint => self.hint += 1,
        }
    }

    const fn add(&mut self, other: Self) {
        self.error += other.error;
        self.warn += other.warn;
        self.info += other.info;
        self.hint += other.hint;
    }

    const fn state(self) -> &'static str {
        if self.error > 0 {
            "error"
        } else if self.warn > 0 {
            "warn"
        } else if self.info + self.hint > 0 {
            "info"
        } else {
            "ok"
        }
    }
}

/// `spirolo/tree`'s handler header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Handlers {
    /// The size of the `handlers` table.
    pub count: usize,
    /// The number of publishable `handler-drift` findings.
    pub drift: usize,
}

/// `spirolo/tree` over the model and the published findings, with the baseline's size.
pub fn sidebar_tree(model: &Model, findings: &[Finding], baseline: Option<usize>) -> SidebarTree {
    let mut tests: BTreeMap<String, usize> = ["vitest", "jest", "playwright"]
        .into_iter()
        .map(|framework| (framework.to_owned(), 0))
        .collect();
    let files: BTreeSet<(&str, &str)> = model
        .test_cases
        .iter()
        .filter_map(|case| Some((case.framework.as_deref()?, case.file.as_str())))
        .collect();
    for (framework, _) in files {
        *tests.entry(framework.to_owned()).or_default() += 1;
    }
    let drift = findings
        .iter()
        .filter(|finding| finding.rule == "handler-drift" && editor_severity(finding).is_some())
        .count();
    let (project, nodes) = nodes(model, findings);
    SidebarTree {
        root: model.root.display().to_string(),
        tests,
        baseline,
        handlers: Handlers {
            count: model.handlers.len(),
            drift,
        },
        project,
        nodes,
    }
}

/// The tree's project row, then its nodes: a file node per test file and per file with a
/// publishable non-`impact` finding, a dir node per ancestor directory below the root.
fn nodes(model: &Model, findings: &[Finding]) -> (Option<Project>, Vec<Node>) {
    let impact: BTreeSet<&str> = spirolo_rules::family_rules(Family::Impact)
        .iter()
        .map(|rule| rule.name())
        .collect();
    let mut files: BTreeMap<&str, Counts> = model
        .test_cases
        .iter()
        .map(|case| (case.file.as_str(), Counts::default()))
        .collect();
    let mut project: Option<Counts> = None;
    for finding in findings
        .iter()
        .filter(|finding| !impact.contains(finding.rule))
    {
        let Some(severity) = editor_severity(finding) else {
            continue;
        };
        let counts = if finding.location.file.is_empty() {
            project.get_or_insert_with(Counts::default)
        } else {
            files.entry(finding.location.file.as_str()).or_default()
        };
        match severity {
            EditorSeverity::Error => counts.error += 1,
            EditorSeverity::Warning => counts.warn += 1,
            EditorSeverity::Information => counts.info += 1,
            EditorSeverity::Hint => counts.hint += 1,
        }
    }
    let mut dirs: BTreeMap<&str, Counts> = BTreeMap::new();
    for (file, counts) in &files {
        let mut path = *file;
        while let Some((parent, _)) = path.rsplit_once('/') {
            dirs.entry(parent).or_default().add(*counts);
            path = parent;
        }
    }
    let mut nodes: Vec<Node> = files
        .into_iter()
        .map(|(path, counts)| (path, "file", counts))
        .chain(dirs.into_iter().map(|(path, counts)| (path, "dir", counts)))
        .map(|(path, kind, counts)| Node {
            path: path.to_owned(),
            kind,
            counts,
            state: counts.state(),
        })
        .collect();
    nodes.sort_by(|a, b| a.path.cmp(&b.path));
    let project = project.map(|counts| Project {
        counts,
        state: counts.state(),
    });
    (project, nodes)
}

/// `spirolo/file`: what the panel shows for one file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SidebarFile {
    /// The repo-relative path asked about.
    pub file: String,
    /// `test` if the file has test cases, else `source` if it is a model module, else `other`.
    pub kind: &'static str,
    /// The panel's rows, in the PRD's fixed order.
    pub rows: Vec<Row>,
}

/// One panel row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Row {
    /// What the row is about: `covers`, `handlers`, a rule name, `flake`, `mutation`, `hygiene`.
    pub key: String,
    /// `error`, `warn`, `info`, `ok`, or `none` when the row's input was never provided.
    pub state: &'static str,
    /// The row's one line.
    pub text: String,
    /// A second line, or `None`.
    pub detail: Option<String>,
    /// The finding ids the row stands for, sorted.
    pub ids: Vec<String>,
}

/// `spirolo/file` for `file`, a repo-relative path.
pub fn sidebar_file(model: &Model, findings: &[Finding], file: &str) -> SidebarFile {
    // The file's publishable findings, ordered by line, then id.
    let mut here: Vec<(&Finding, EditorSeverity, String)> = findings
        .iter()
        .filter(|finding| finding.location.file == file)
        .filter_map(|finding| Some((finding, editor_severity(finding)?, finding.id())))
        .collect();
    here.sort_by(|a, b| (a.0.location.line, &a.2).cmp(&(b.0.location.line, &b.2)));
    let here: Vec<Published> = here
        .into_iter()
        .map(|(finding, severity, id)| Published {
            finding,
            severity,
            id,
        })
        .collect();

    let (kind, rows) = if model.test_cases.iter().any(|case| case.file == file) {
        let mut rows = vec![covers(model, file), handlers(model, &here)];
        rows.extend(rule_rows(&here, |rule| {
            matches!(rule.family(), Family::Audit | Family::Gate) && rule.name() != "handler-drift"
        }));
        rows.push(flake(model, &here));
        rows.push(mutation(model, file));
        rows.push(hygiene(&here));
        ("test", rows)
    } else if model.modules.iter().any(|module| module.path == file) {
        ("source", rule_rows(&here, |_| true))
    } else {
        ("other", rule_rows(&here, |_| true))
    };
    SidebarFile {
        file: file.to_owned(),
        kind,
        rows,
    }
}

/// A publishable finding in the asked-about file.
struct Published<'a> {
    finding: &'a Finding,
    severity: EditorSeverity,
    id: String,
}

/// The strongest editor severity among `findings` as a row state: Error is `error`, Warning
/// is `warn`, Information and Hint are `info`.
fn strongest(findings: &[&Published]) -> &'static str {
    match findings.iter().map(|published| published.severity).min() {
        Some(EditorSeverity::Error) => "error",
        Some(EditorSeverity::Warning) => "warn",
        Some(EditorSeverity::Information | EditorSeverity::Hint) => "info",
        None => "ok",
    }
}

/// The ids of `findings`, sorted.
fn ids(findings: &[&Published]) -> Vec<String> {
    let mut ids: Vec<String> = findings
        .iter()
        .map(|published| published.id.clone())
        .collect();
    ids.sort();
    ids
}

/// The sources `file` is paired with, `ambiguous` or not, sorted.
fn paired<'a>(model: &'a Model, file: &str, ambiguous: bool) -> Vec<&'a str> {
    let mut sources: Vec<&str> = model
        .pairings
        .iter()
        .filter(|pairing| pairing.test_file == file && pairing.ambiguous == ambiguous)
        .map(|pairing| pairing.source_module.as_str())
        .collect();
    sources.sort_unstable();
    sources.dedup();
    sources
}

/// Row 1: the paired sources, unambiguous first, then ambiguous ones, each group sorted.
fn covers(model: &Model, file: &str) -> Row {
    let unambiguous = paired(model, file, false);
    let ambiguous = paired(model, file, true);
    let detail = coverage_detail(model, unambiguous.iter().chain(&ambiguous).copied());
    let sources: Vec<String> = unambiguous
        .into_iter()
        .map(str::to_owned)
        .chain(
            ambiguous
                .into_iter()
                .map(|source| format!("{source} (ambiguous)")),
        )
        .collect();
    let (state, text) = if sources.is_empty() {
        ("none", "no paired source".to_owned())
    } else {
        ("ok", sources.join(" · "))
    };
    Row {
        key: "covers".to_owned(),
        state,
        text,
        detail: Some(detail),
        ids: Vec::new(),
    }
}

/// `covers`' detail: covered over total lines, summed over the paired sources' coverage rows.
fn coverage_detail<'a>(model: &Model, sources: impl Iterator<Item = &'a str>) -> String {
    if model.coverage.is_empty() {
        return "no coverage report read".to_owned();
    }
    let sources: Vec<&str> = sources.collect();
    let rows: Vec<_> = model
        .coverage
        .iter()
        .filter(|coverage| sources.contains(&coverage.file.as_str()))
        .collect();
    if rows.is_empty() {
        return "no coverage for the paired source".to_owned();
    }
    let lines = rows.iter().flat_map(|coverage| &coverage.lines);
    let total = lines.clone().count();
    let covered = lines.filter(|line| line.hits > 0).count();
    format!("{covered}/{total} lines")
}

/// Row 2: this file's `handler-drift` findings against the project's handlers.
fn handlers(model: &Model, here: &[Published]) -> Row {
    let detail = Some(status::status_line("handler-drift"));
    if model.handlers.is_empty() {
        return Row {
            key: "handlers".to_owned(),
            state: "none",
            text: "no handlers in the project".to_owned(),
            detail,
            ids: Vec::new(),
        };
    }
    let drift: Vec<&Published> = here
        .iter()
        .filter(|published| published.finding.rule == "handler-drift")
        .collect();
    let text = drift.first().map_or_else(
        || {
            format!(
                "{} handlers in the project · no drift in this file",
                model.handlers.len()
            )
        },
        |first| first.finding.message.clone(),
    );
    Row {
        key: "handlers".to_owned(),
        state: strongest(&drift),
        text,
        detail,
        ids: ids(&drift),
    }
}

/// One row per rule `keep` admits that has a publishable finding here, in registry order:
/// `<n> × line <first line>: <first message>`.
fn rule_rows(here: &[Published], keep: impl Fn(&dyn spirolo_rules::Rule) -> bool) -> Vec<Row> {
    spirolo_rules::registry()
        .iter()
        .filter(|rule| keep(rule.as_ref()))
        .filter_map(|rule| {
            let fired: Vec<&Published> = here
                .iter()
                .filter(|published| published.finding.rule == rule.name())
                .collect();
            let first = fired.first()?.finding;
            Some(Row {
                key: rule.name().to_owned(),
                state: strongest(&fired),
                text: format!(
                    "{} × line {}: {}",
                    fired.len(),
                    first.location.line,
                    first.message
                ),
                detail: Some(status::status_line(rule.name())),
                ids: ids(&fired),
            })
        })
        .collect()
}

/// Row 4: this file's static flake candidates and observed flips, and the runs ingested.
fn flake(model: &Model, here: &[Published]) -> Row {
    let of = |rule: &str| -> Vec<&Published> {
        here.iter()
            .filter(|published| published.finding.rule == rule)
            .collect()
    };
    let candidates = of("flake-candidate");
    let observed = of("flake-observed");
    let all: Vec<&Published> = candidates.iter().chain(&observed).copied().collect();
    let text = if all.is_empty() {
        "no static candidates".to_owned()
    } else {
        format!(
            "{} static candidates · {} observed flips",
            candidates.len(),
            observed.len()
        )
    };
    let detail = if model.runs.is_empty() {
        "no runs ingested".to_owned()
    } else {
        format!("{} runs ingested", model.runs.len())
    };
    Row {
        key: "flake".to_owned(),
        state: strongest(&all),
        text,
        detail: Some(detail),
        ids: ids(&all),
    }
}

/// Row 5: how many of the paired sources' mutants survived (`Survived` or `NoCoverage`).
fn mutation(model: &Model, file: &str) -> Row {
    let row = |state, text: String| Row {
        key: "mutation".to_owned(),
        state,
        text,
        detail: None,
        ids: Vec::new(),
    };
    if model.mutants.is_empty() {
        return row("none", "no mutation report read".to_owned());
    }
    let mut sources = paired(model, file, false);
    sources.extend(paired(model, file, true));
    let mutants: Vec<&Mutants> = model
        .mutants
        .iter()
        .filter(|mutant| sources.contains(&mutant.file.as_str()))
        .collect();
    if mutants.is_empty() {
        return row("none", "no mutants for the paired source".to_owned());
    }
    let survived = mutants
        .iter()
        .filter(|mutant| {
            matches!(
                mutant.status,
                MutantStatus::Survived | MutantStatus::NoCoverage
            )
        })
        .count();
    row(
        if survived > 0 { "info" } else { "ok" },
        format!(
            "paired source: {survived} of {} mutants survived",
            mutants.len()
        ),
    )
}

/// Row 6: this file's `hygiene`-family findings, `<rule> ×<n>` per rule in registry order.
fn hygiene(here: &[Published]) -> Row {
    let mut all = Vec::new();
    let mut counts = Vec::new();
    for rule in spirolo_rules::family_rules(Family::Hygiene) {
        let fired: Vec<&Published> = here
            .iter()
            .filter(|published| published.finding.rule == rule.name())
            .collect();
        if !fired.is_empty() {
            counts.push(format!("{} ×{}", rule.name(), fired.len()));
            all.extend(fired);
        }
    }
    let text = if counts.is_empty() {
        "no hygiene findings".to_owned()
    } else {
        counts.join(" · ")
    };
    Row {
        key: "hygiene".to_owned(),
        state: strongest(&all),
        text,
        detail: None,
        ids: ids(&all),
    }
}
