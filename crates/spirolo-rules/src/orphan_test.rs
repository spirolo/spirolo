//! `orphan-test`: a test that exists and never runs.
//!
//! Two kinds, each read straight off the model. *Never collected*: a file a recogniser
//! claimed a test in that no runner gate's include and exclude would collect — the model's
//! [`Uncollected`](spirolo_model::Uncollected) table, filled by scoping with the same
//! file-collection logic the M1 counts match against the runners. Its explain names what
//! left the file out — the workspace boundary (a file outside every member that no config
//! the workspace runs reaches) or the governing config's globs — and says the suite is
//! smaller than it looks. *Source gone*: a test file whose project import does not
//! resolve — an import the model left `missing` — reported at the specifier, so a test left
//! behind by a deleted or moved module is found before the runner errors on it. Both carry
//! severity `warn`. A relative import the model left `unknown` — the resolver could not load
//! the tsconfig that applies to it, typically an `extends` preset only an install provides —
//! is a source-gone the rule could not decide, never a finding; so is a relative import
//! into the package's build output (`../lib`, `../dist/store`), which a clean checkout lacks
//! and the build makes before the suite runs.

use std::collections::BTreeSet;

use spirolo_model::{Import, Model, TestKind};

use crate::finding::{Evidence, Finding, Location, Outcome, Proposal};
use crate::{Config, Family, Rule, Severity};

/// The `orphan-test` rule. See the module docs for the semantics.
pub struct OrphanTest;

impl Rule for OrphanTest {
    fn name(&self) -> &'static str {
        "orphan-test"
    }

    fn family(&self) -> Family {
        Family::Hygiene
    }

    fn subject(&self) -> crate::Subject {
        crate::Subject::TestFiles
    }

    fn summary(&self) -> &'static str {
        "A test file disagrees with the suite that runs it: no runner gate collects it, or a project import it reads no longer resolves."
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn run(&self, model: &Model, config: &Config) -> Vec<Finding> {
        let severity = config.severity(self.name(), self.default_severity());
        let mut findings = Vec::new();
        findings.extend(self.never_collected(model, severity, config.propose));
        findings.extend(self.source_gone(model, severity, config.propose));
        crate::finding::sort(&mut findings);
        crate::finding::number_collisions(&mut findings);
        findings
    }
}

impl OrphanTest {
    /// One finding per recognised test file the runner would not collect, at the head of
    /// the file, naming the config and the globs it was judged against.
    fn never_collected(&self, model: &Model, severity: Severity, propose: bool) -> Vec<Finding> {
        model
            .uncollected
            .iter()
            .map(|orphan| {
                let config = orphan.config.as_deref().unwrap_or("the runner defaults");
                Finding {
                    rule: self.name(),
                    severity,
                    outcome: Outcome::Finding,
                    location: Location {
                        file: orphan.file.clone(),
                        line: 1,
                        column: 1,
                    },
                    related: Vec::new(),
                    message: format!(
                        "{} is a test file no runner collects, so it never runs",
                        orphan.file
                    ),
                    evidence: vec![
                        Evidence::new("kind", "never-collected"),
                        Evidence::new(
                            "reason",
                            if orphan.outside_workspace {
                                "outside-workspace"
                            } else {
                                "globs"
                            },
                        ),
                        Evidence::new("config", config),
                        Evidence::new("include", orphan.include.join(", ")),
                        Evidence::new("exclude", orphan.exclude.join(", ")),
                    ],
                    explain: if orphan.outside_workspace {
                        format!(
                            "{} sits outside every member of the workspace, and no runner config \
                             the workspace runs reaches it, so no test run collects it and its \
                             tests never run, whatever its own globs say. The suite is smaller \
                             than it looks. Add its directory to the workspace, move it into a \
                             member, or delete it if it is dead.",
                            orphan.file,
                        )
                    } else {
                        format!(
                            "{} matches none of the include globs {} (or is removed by {}) of \
                             {}, so the runner never collects it and its tests never run. The \
                             suite is smaller than it looks. Add an include glob that covers it, \
                             move it under one that does, or delete it if it is dead.",
                            orphan.file,
                            globs_or_defaults(&orphan.include),
                            globs_or_none(&orphan.exclude),
                            config,
                        )
                    },
                    anchor: "never-collected".to_owned(),
                    test: None,
                    ordinal: 0,
                    proposal: propose.then(|| Proposal::Keep {
                        reason: "add the include glob".to_owned(),
                    }),
                }
            })
            .collect()
    }

    /// One finding per project import a test file leaves unresolved, at the specifier.
    fn source_gone(&self, model: &Model, severity: Severity, propose: bool) -> Vec<Finding> {
        let test_files: BTreeSet<&str> = model
            .test_cases
            .iter()
            .filter(|case| case.kind == TestKind::Test)
            .map(|case| case.file.as_str())
            .collect();
        let mut findings = Vec::new();
        for module in &model.modules {
            if !test_files.contains(module.path.as_str()) {
                continue;
            }
            for import in &module.imports {
                if is_undecided(import) {
                    findings.push(self.undecided(&module.path, import, severity, propose));
                    continue;
                }
                if is_build_output(import) {
                    findings.push(self.unbuilt(&module.path, import, severity, propose));
                    continue;
                }
                if !is_missing(import) {
                    continue;
                }
                findings.push(Finding {
                    rule: self.name(),
                    severity,
                    outcome: Outcome::Finding,
                    location: Location::at(module.path.clone(), import.span),
                    related: Vec::new(),
                    message: format!(
                        "the test imports {}, which no longer resolves",
                        import.specifier
                    ),
                    evidence: vec![
                        Evidence::new("kind", "source-gone"),
                        Evidence::new("specifier", import.specifier.clone()),
                    ],
                    explain: format!(
                        "{} imports {}, a project module that does not resolve — deleted or \
                         moved. The runner errors on the file before it runs a single test. \
                         Restore the module, fix the path, or delete the test if the code it \
                         covered is gone.",
                        module.path, import.specifier,
                    ),
                    anchor: import.specifier.clone(),
                    test: None,
                    ordinal: 0,
                    // The specifier names what is missing, not what to delete, and the rule
                    // cannot tell a moved module from a removed one: offer all three repairs.
                    proposal: propose.then(|| Proposal::Keep {
                        reason: "restore the module, fix the path, or delete the test — spirolo \
                                 cannot tell a moved module from a removed one"
                            .to_owned(),
                    }),
                });
            }
        }
        findings
    }

    /// A relative import into the package's build output (`../lib`, `../dist/store`): absent
    /// from a clean checkout, made by the build before the suite runs, so whether the module
    /// is gone is not known.
    fn unbuilt(&self, file: &str, import: &Import, severity: Severity, propose: bool) -> Finding {
        Finding {
            message: format!(
                "the test imports {}, the package's build output, which is not built here",
                import.specifier
            ),
            explain: format!(
                "{file} imports {}, which lands in a directory the package builds into (named \
                 by its package.json entries or a build script's output directory, or a lib/ \
                 or dist/ beside it). A clean checkout has no build output, so whether the \
                 module exists after the build cannot be told. Build the package and run again.",
                import.specifier,
            ),
            proposal: propose.then(|| Proposal::Keep {
                reason: "build the package and re-check".to_owned(),
            }),
            ..self.undecided(file, import, severity, propose)
        }
    }

    /// A relative import the resolver failed on without looking for the file — its package's
    /// tsconfig could not be loaded — so whether the module is gone is not known.
    fn undecided(&self, file: &str, import: &Import, severity: Severity, propose: bool) -> Finding {
        Finding {
            rule: self.name(),
            severity,
            outcome: Outcome::CouldNotDecide,
            location: Location::at(file.to_owned(), import.span),
            related: Vec::new(),
            message: format!(
                "the test imports {}, which could not be resolved to decide",
                import.specifier
            ),
            evidence: vec![
                Evidence::new("kind", "source-gone"),
                Evidence::new("specifier", import.specifier.clone()),
            ],
            explain: format!(
                "{file} imports {}, but the resolver could not load the tsconfig that applies \
                 to it (see the tsconfig_unloadable diagnostic), so it never looked for the \
                 module. Install the dependencies the config extends and run again.",
                import.specifier,
            ),
            anchor: import.specifier.clone(),
            test: None,
            ordinal: 0,
            proposal: propose.then(|| Proposal::Keep {
                reason: "install the tsconfig's extends and re-check".to_owned(),
            }),
        }
    }
}

/// Whether an import is a relative one into its package's build directory, which only a
/// build fills.
fn is_build_output(import: &Import) -> bool {
    import.reason.as_deref() == Some("build_output")
}

/// Whether an import is a relative one the model left `unknown`: the resolver failed on its
/// config, not on the specifier.
fn is_undecided(import: &Import) -> bool {
    import.reason.as_deref() == Some("unknown")
}

/// Whether an import is a project import the model left unresolved — a dead relative or a
/// dead project alias, both reason `missing`. An external or unknown-package specifier is
/// not a source-gone: it may simply be uninstalled.
fn is_missing(import: &Import) -> bool {
    import.reason.as_deref() == Some("missing")
}

/// The include globs joined for prose, or a phrase for the empty set.
fn globs_or_defaults(globs: &[String]) -> String {
    if globs.is_empty() {
        "(none)".to_owned()
    } else {
        globs.join(", ")
    }
}

/// The exclude globs joined for prose, or `nothing` for the empty set.
fn globs_or_none(globs: &[String]) -> String {
    if globs.is_empty() {
        "nothing".to_owned()
    } else {
        globs.join(", ")
    }
}
