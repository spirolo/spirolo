//! `duplicate-test`: two or more tests whose normalised bodies hash equal and that run in the
//! same context — a test written twice under two names, one finding a maintainer can act on.
//!
//! The rule reads `TestCase.body_hash` (the model normalises identifiers and excludes the
//! title, so two tests with the same behaviour and different names hash equal) over the
//! `test(…)` / `it(…)` cases only. Equal text is one test only over the same things, so a group
//! also shares `TestCase.context`: every free name the bodies read is bound to the same
//! declaration (one file-level or suite-level binding, or an import of one resolved module and
//! export), every relative module the bodies load is one file, and the same before-hooks run
//! before each. A row defined inside a loop or a `.each` table is one definition the runner
//! instantiates many times, not a clone, and a placeholder (an empty body, an unconditional
//! skip or todo) tests nothing; both are excluded. A group is two or more surviving rows: one
//! finding at the first member in file-and-span order, the others as related locations
//! labelled with their suite path and title. Whole-body equality only — a shared arrange
//! block under a different act-and-assert is not a clone, and a near-clone heuristic is out
//! of scope.

use std::collections::{BTreeMap, HashMap};

use spirolo_model::{ContextRef, Model, Module, SkipKind, TestCase, TestKind};

use crate::finding::{
    number_collisions, sort, Evidence, Finding, Location, Outcome, Proposal, RelatedLocation,
    TestPath,
};
use crate::{Config, Family, Rule, Severity};

/// The `duplicate-test` rule. See the module docs for the semantics.
pub struct DuplicateTest;

impl Rule for DuplicateTest {
    fn name(&self) -> &'static str {
        "duplicate-test"
    }

    fn family(&self) -> Family {
        Family::Hygiene
    }

    fn subject(&self) -> crate::Subject {
        crate::Subject::Tests
    }

    fn summary(&self) -> &'static str {
        "Two tests with different names have bodies that normalise to the same code, so one of them adds nothing."
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn run(&self, model: &Model, config: &Config) -> Vec<Finding> {
        let severity = config.severity(self.name(), self.default_severity());

        // Group the eligible tests by body hash and context, keeping each group in
        // file-and-span order.
        let modules: HashMap<&str, &Module> = model
            .modules
            .iter()
            .map(|module| (module.path.as_str(), module))
            .collect();
        let mut groups: BTreeMap<(&str, Vec<String>), Vec<&TestCase>> = BTreeMap::new();
        for case in &model.test_cases {
            let Some(hash) = eligible_hash(case) else {
                continue;
            };
            groups
                .entry((hash, context_key(case, modules.get(case.file.as_str()))))
                .or_default()
                .push(case);
        }

        let mut findings = Vec::new();
        for ((hash, _), mut members) in groups {
            members.sort_by(|a, b| a.file.cmp(&b.file).then(a.span.cmp(&b.span)));
            // Two recognisers can list one definition (a Playwright spec in a Vitest
            // project): it is one test, not two.
            members.dedup_by(|a, b| a.file == b.file && a.span == b.span);
            if members.len() < 2 {
                continue;
            }
            findings.push(self.finding(hash, &members, model, severity, config.propose));
        }

        sort(&mut findings);
        number_collisions(&mut findings);
        findings
    }
}

impl DuplicateTest {
    /// One finding for a group: located at the first member, the others related and labelled
    /// with their suite path and title, with the group size, whether it spans files, and each
    /// member's paired source as evidence.
    fn finding(
        &self,
        hash: &str,
        members: &[&TestCase],
        model: &Model,
        severity: Severity,
        propose: bool,
    ) -> Finding {
        let first = members[0];
        let related = members[1..]
            .iter()
            .map(|member| RelatedLocation {
                label: label_of(member),
                location: Location::at(member.file.clone(), member.span),
            })
            .collect();
        let cross_file = members.iter().any(|member| member.file != first.file);
        let sources = members
            .iter()
            .map(|member| format!("{}: {}", member.file, paired_source(model, &member.file)))
            .collect::<Vec<_>>()
            .join("; ");
        let others = members.len() - 1;
        Finding {
            rule: self.name(),
            severity,
            outcome: Outcome::Finding,
            location: Location::at(first.file.clone(), first.span),
            related,
            message: format!(
                "{} duplicates {} other test{}",
                label_of(first),
                others,
                if others == 1 { "" } else { "s" }
            ),
            evidence: vec![
                Evidence::new("cross-file", if cross_file { "yes" } else { "no" }),
                Evidence::new("size", members.len().to_string()),
                Evidence::new("sources", sources),
            ],
            explain: explain(members),
            anchor: hash.to_owned(),
            test: Some(TestPath::of(first)),
            ordinal: 0,
            proposal: propose.then(|| {
                let into = richest(members);
                Proposal::Merge {
                    from: members
                        .iter()
                        .filter(|member| !std::ptr::eq(**member, into))
                        .map(|member| identity(member))
                        .collect::<Vec<_>>()
                        .join("; "),
                    into: identity(into),
                }
            }),
        }
    }
}

/// The member a duplicate group merges into: the one with the most assertions, ties broken
/// towards the first in order — the same choice the explain names.
fn richest<'a>(members: &'a [&'a TestCase]) -> &'a TestCase {
    members
        .iter()
        .enumerate()
        .max_by_key(|(index, member)| (member.assertions.len(), usize::MAX - index))
        .map_or(members[0], |(_, member)| *member)
}

/// A member's identity for a proposal: its label and where it lives, as `label (file:line)`.
fn identity(member: &TestCase) -> String {
    format!(
        "{} ({}:{})",
        label_of(member),
        member.file,
        member.span.line
    )
}

/// The repair text: names each member and where it lives, points at the richer test — the
/// member with the most assertions, else the first in order — as the one to keep, and names
/// the copy-paste slip, where the fix is the misnamed body rather than a deletion.
fn explain(members: &[&TestCase]) -> String {
    let richest = richest(members);
    let list = members
        .iter()
        .map(|member| {
            format!(
                "{} ({}:{})",
                label_of(member),
                member.file,
                member.span.line
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "These {} tests have identical bodies once names are normalised: {list}. They check one \
         behaviour {} times under different names. Keep one — {} ({}:{}), which carries the \
         most assertions — and delete the others; or, if a title promises a case its body does \
         not test (a copy-paste slip), change that body to test what its title says.",
        members.len(),
        members.len(),
        label_of(richest),
        richest.file,
        richest.span.line,
    )
}

/// The source module a test file is paired with, or `unpaired` when no strategy paired it —
/// the evidence a maintainer reads to see whether two clones exercise the same source.
fn paired_source(model: &Model, file: &str) -> String {
    model
        .pairings
        .iter()
        .find(|pairing| pairing.test_file == file)
        .map_or_else(
            || "unpaired".to_owned(),
            |pairing| pairing.source_module.clone(),
        )
}

/// The body hash of a case the rule groups on, or `None` when the case is not a candidate: a
/// suite or hook row, a row with no hash, a row the runner instantiates many times from
/// one definition (inside a loop or a `.each` table), or a placeholder (an empty body, or
/// an unconditional skip or todo: it tests nothing, and a skip belongs to `dead-skip`).
fn eligible_hash(case: &TestCase) -> Option<&str> {
    if case.kind != TestKind::Test || case.each.is_some() || !case.loops.is_empty() || case.empty {
        return None;
    }
    if case.skip.as_ref().is_some_and(|skip| {
        skip.condition.is_none()
            && matches!(skip.kind, SkipKind::Skip | SkipKind::Todo | SkipKind::Fixme)
    }) {
        return None;
    }
    case.body_hash.as_deref()
}

/// What a case's body reads outside its own text, each entry written so that two cases'
/// entries are equal exactly when they name the same thing: a binding by its file and place,
/// an import by the module it resolves to (`module`, the case's file, holds the table) and
/// the export it binds, a module the body loads by the module, a before-hook by its file and
/// place.
fn context_key(case: &TestCase, module: Option<&&Module>) -> Vec<String> {
    case.context
        .iter()
        .map(|reference| match reference {
            ContextRef::Binding { name, span } => {
                format!("binding {} {} {name}", case.file, span.start)
            }
            ContextRef::Import {
                specifier, export, ..
            } => format!(
                "import {} {export}",
                module_of(&case.file, module, specifier)
            ),
            ContextRef::Specifier { specifier } => {
                format!("load {}", module_of(&case.file, module, specifier))
            }
            ContextRef::Hook { span } => format!("hook {} {}", case.file, span.start),
        })
        .collect()
}

/// The module `specifier`, written in `file`, names: where the model resolved it, else the
/// specifier joined to the file's directory when it is relative, else the bare specifier (a
/// package is one package from every file).
fn module_of(file: &str, module: Option<&&Module>, specifier: &str) -> String {
    let resolved = module.and_then(|module| {
        module
            .imports
            .iter()
            .find(|import| import.specifier == specifier)
            .and_then(|import| import.resolved.clone())
    });
    if let Some(resolved) = resolved {
        return resolved;
    }
    if !specifier.starts_with('.') {
        return specifier.to_owned();
    }
    let mut parts: Vec<&str> = file.split('/').collect();
    parts.pop();
    for part in specifier.split('/') {
        match part {
            "." | "" => {}
            ".." => {
                parts.pop();
            }
            part => parts.push(part),
        }
    }
    parts.join("/")
}

/// A member's label for a related location: its suite path joined with its title, so a
/// reader sees which test the clone is without opening the file.
fn label_of(case: &TestCase) -> String {
    let mut parts = case.suite_path.clone();
    parts.push(
        case.title
            .clone()
            .unwrap_or_else(|| format!("<dynamic>:{}", case.span.line)),
    );
    parts.join(" › ")
}
