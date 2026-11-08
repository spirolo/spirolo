//! Saying so when the project's module resolution could not load, and marking the rows that
//! read it.
//!
//! The model leaves every import under a `tsconfig` it cannot load `unknown` (one
//! `tsconfig_unloadable` diagnostic per config). The usual cause is a checkout before its
//! install: `"extends": "astro/tsconfigs/strict"` names a package `node_modules` does not
//! have yet. A rule that reads imports then sees a graph with holes in it and reports the
//! holes: the dogfood run on a repo before `npm ci` (PR #505) printed 49 orphan-test rows
//! and a dozen untested-hotspot rows that were gone after the install.
//!
//! The run gets one `module_resolution_failed` diagnostic, which JSON carries, and every
//! row of a rule in [`DEPENDENT`] whose file lies under a config that failed is marked: an
//! `unreliable` evidence entry and a suffix on its message, so the flag reaches the formats
//! that carry no diagnostics (SARIF, Markdown, annotations, the agent view). A root config
//! that fails marks the whole project; one stale package's config (an `archive/` app
//! extending a removed preset) marks only the rows under it.
//!
//! How loudly the human report says so follows how much of the project is affected. The
//! diagnostic is a `warning`, printed under the health line, when the root config failed or
//! the dependencies are not installed: the project's own graph has holes. When only nested
//! configs failed in an installed project, it is `info`, one closing ` note` line: the
//! project loaded, and a few stale packages did not.
//!
//! Rows are marked, not dropped: the exit code, the finding ids and a baseline's counts stay
//! what they would be, so a CI job without an install neither passes silently nor sees
//! rows vanish and come back.

use std::path::Path;

use spirolo_model::{Diagnostic, Model, Severity};
use spirolo_rules::{Evidence, Finding};

/// The rules a hole in the import graph turns into rows that should not be there:
/// - `orphan-test`: a source-gone row is a relative import that did not resolve; under an
///   unloadable config every one is a `could not decide`;
/// - `untested-hotspot`: weakness is whether a test reaches the module through imports, so
///   an unreached module scores 1.0;
/// - `unused-mock`: a mock whose test's import closure has an unresolved import is a
///   `could not decide`;
/// - `absence-anchor`: a throwing helper exempts a test only when it lives in a module the
///   test's imports resolve to.
///
/// The other rules that read imports only miss rows on a hole, or fall back to the
/// specifier joined to the file's directory: `flake-candidate` and `duplicate-test` resolve
/// a relative import that way (a bare worktree gives `flake-candidate` the findings an
/// installed one does), and `data-layer-mock` also matches a mock by its specifier.
pub const DEPENDENT: [&str; 4] = [
    "orphan-test",
    "untested-hotspot",
    "unused-mock",
    "absence-anchor",
];

/// The diagnostic code of a run whose module resolution could not load.
pub const CODE: &str = "module_resolution_failed";

/// How many failed directories the diagnostic names before it counts the rest.
const SCOPES_NAMED: usize = 3;

/// A run whose module resolution could not load: which configs failed, under which
/// directories, and how much it left unresolved.
pub struct Failure {
    /// The repo-relative directories of the configs that failed, sorted; `""` is the root.
    scopes: Vec<String>,
    /// How many configs failed.
    configs: usize,
    /// How many imports under the failed directories were left unresolved.
    imports: usize,
    /// Whether the root's dependencies are declared and not installed.
    uninstalled: bool,
}

impl Failure {
    /// The failure of `model`, built from `root`, or `None` when every tsconfig loaded.
    pub fn of(model: &Model, root: &Path) -> Option<Self> {
        let mut scopes: Vec<String> = model
            .diagnostics
            .iter()
            .filter(|d| d.code == "tsconfig_unloadable")
            .filter_map(|d| d.file.as_deref())
            .map(|file| file.rsplit_once('/').map_or("", |(dir, _)| dir).to_owned())
            .collect();
        if scopes.is_empty() {
            return None;
        }
        let configs = scopes.len();
        scopes.sort();
        scopes.dedup();
        let mut failure = Self {
            scopes,
            configs,
            imports: 0,
            uninstalled: uninstalled(root),
        };
        failure.imports = model
            .modules
            .iter()
            .filter(|module| failure.covers(&module.path))
            .flat_map(|module| &module.imports)
            .filter(|import| import.reason.as_deref() == Some("unknown"))
            .count();
        Some(failure)
    }

    /// Whether the project's own graph has holes — the root config failed, or nothing is
    /// installed — so the human report says so under the health line, not in a closing note.
    fn loud(&self) -> bool {
        self.uninstalled || self.scopes.iter().any(String::is_empty)
    }

    /// The `module_resolution_failed` diagnostic, once `marked` rows are marked. Loud, a
    /// `warning` whose `; `-separated clauses (what was left unresolved, the cause, the fix,
    /// what is marked) the human report prints one a line; else an `info` the report prints
    /// as one closing ` note`.
    pub fn diagnostic(&self, marked: usize) -> Diagnostic {
        let imports = format!(
            "{} {}",
            self.imports,
            plural(self.imports, "import was", "imports were")
        );
        let configs = format!(
            "{} {}",
            self.configs,
            plural(self.configs, "tsconfig", "tsconfigs")
        );
        let rows = format!("{marked} {}", plural(marked, "row", "rows"));
        let (severity, message) = if self.loud() {
            let (cause, fix) = if self.uninstalled {
                (
                    "dependencies are not installed (package.json declares them, there is no \
                     node_modules)",
                    "install dependencies, e.g. `npm ci`, then rerun",
                )
            } else {
                (
                    "a tsconfig extends a package that is not installed or a file that does \
                     not exist",
                    "install the package the tsconfig extends or correct the path, then rerun",
                )
            };
            (
                Severity::Warning,
                format!(
                    "{imports} left unresolved under {configs} that could not be loaded; likely \
                     cause: {cause}; fix: {fix}; marked unreliable until then: {rows} of {}{}",
                    DEPENDENT.join(", "),
                    if self.scopes.iter().any(String::is_empty) {
                        String::new()
                    } else {
                        format!(" under {}", named(&self.scopes))
                    },
                ),
            )
        } else {
            (
                Severity::Info,
                format!(
                    "module resolution failed for {configs} in nested directories ({}): \
                     {imports} left unresolved, {rows} under them marked unreliable; spirolo list \
                     shows the configs",
                    named(&self.scopes),
                ),
            )
        };
        Diagnostic {
            code: CODE.to_owned(),
            severity,
            file: None,
            span: None,
            message,
        }
    }

    /// Mark every row of a [`DEPENDENT`] rule under a failed config as unreliable: an
    /// `unreliable` evidence entry, kept in key order, and a suffix on the message. Returns
    /// how many rows it marked.
    pub fn mark(&self, findings: &mut [Finding]) -> usize {
        let mut marked = 0;
        for finding in findings.iter_mut().filter(|finding| {
            DEPENDENT.contains(&finding.rule) && self.covers(&finding.location.file)
        }) {
            finding
                .evidence
                .push(Evidence::new("unreliable", "module resolution failed"));
            finding.evidence.sort_by(|a, b| a.key.cmp(&b.key));
            finding
                .message
                .push_str(" (unreliable: module resolution failed)");
            marked += 1;
        }
        marked
    }

    /// Whether `file` lies under a config that failed.
    fn covers(&self, file: &str) -> bool {
        self.scopes.iter().any(|scope| {
            scope.is_empty()
                || file
                    .strip_prefix(scope.as_str())
                    .is_some_and(|rest| rest.starts_with('/'))
        })
    }
}

/// The failed directories `scopes` for the message: the first few, then a count of the
/// rest.
fn named(scopes: &[String]) -> String {
    let named: Vec<String> = scopes
        .iter()
        .take(SCOPES_NAMED)
        .map(|scope| format!("{scope}/"))
        .collect();
    let more = scopes.len().saturating_sub(SCOPES_NAMED);
    if more > 0 {
        format!(
            "{} and {more} more {}",
            named.join(", "),
            plural(more, "directory", "directories")
        )
    } else {
        named.join(", ")
    }
}

/// Whether the root's `package.json` declares dependencies and no `node_modules` sits
/// beside it.
fn uninstalled(root: &Path) -> bool {
    if root.join("node_modules").is_dir() {
        return false;
    }
    let Ok(text) = std::fs::read_to_string(root.join("package.json")) else {
        return false;
    };
    let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&text) else {
        return false;
    };
    ["dependencies", "devDependencies"].iter().any(|key| {
        manifest[key]
            .as_object()
            .is_some_and(|deps| !deps.is_empty())
    })
}

/// `one` for a count of one, `many` otherwise.
const fn plural(count: usize, one: &'static str, many: &'static str) -> &'static str {
    if count == 1 {
        one
    } else {
        many
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use spirolo_rules::status::Verdict;
    use spirolo_rules::{Evidence, Finding, Location, Outcome, Severity};

    use super::{named, Failure};

    /// A failure of the configs in `scopes`.
    fn failure(scopes: &[&str]) -> Failure {
        Failure {
            scopes: scopes.iter().map(|s| (*s).to_owned()).collect(),
            configs: scopes.len(),
            imports: 0,
            uninstalled: false,
        }
    }

    fn row(rule: &'static str, file: &str) -> Finding {
        Finding {
            rule,
            severity: Severity::Warn,
            outcome: Outcome::Finding,
            location: Location {
                file: file.to_owned(),
                line: 1,
                column: 1,
            },
            related: Vec::new(),
            message: "scores 10.00".to_owned(),
            evidence: vec![
                Evidence::new("churn", "1"),
                Evidence::new("weakness", "1.00"),
            ],
            explain: String::new(),
            anchor: file.to_owned(),
            test: None,
            ordinal: 0,
            proposal: None,
        }
    }

    #[test]
    fn a_dependent_row_is_marked_and_keeps_its_identity_severity_and_outcome() {
        let mut findings = vec![row("untested-hotspot", "src/lib/glossary.ts")];
        let before = findings[0].clone();
        failure(&[""]).mark(&mut findings);
        let after = &findings[0];
        assert_eq!(after.id(), before.id());
        assert_eq!(after.severity, before.severity);
        assert_eq!(after.outcome, before.outcome);
        assert_eq!(
            after.message,
            "scores 10.00 (unreliable: module resolution failed)"
        );
        let keys: Vec<&str> = after.evidence.iter().map(|e| e.key.as_str()).collect();
        assert_eq!(keys, ["churn", "unreliable", "weakness"]);
    }

    #[test]
    fn a_row_of_a_rule_that_reads_no_import_is_left_alone() {
        let mut findings = vec![row("fail-closed-setup", "vitest.config.ts")];
        let before = findings.clone();
        failure(&[""]).mark(&mut findings);
        assert_eq!(findings, before);
    }

    #[test]
    fn a_nested_config_marks_only_the_rows_under_it() {
        let mut findings = vec![
            row("untested-hotspot", "archive/app/src/a.ts"),
            row("untested-hotspot", "archive/app-two/src/b.ts"),
            row("untested-hotspot", "packages/ui/src/c.ts"),
        ];
        failure(&["archive/app"]).mark(&mut findings);
        let marked: Vec<bool> = findings
            .iter()
            .map(|f| f.evidence.iter().any(|e| e.key == "unreliable"))
            .collect();
        assert_eq!(marked, [true, false, false]);
    }

    #[test]
    fn the_message_names_the_first_failed_directories_and_counts_the_rest() {
        let scopes =
            |scopes: &[&str]| -> Vec<String> { scopes.iter().map(|s| (*s).to_owned()).collect() };
        assert_eq!(named(&scopes(&["packages/ui"])), "packages/ui/");
        assert_eq!(
            named(&scopes(&["a", "b", "c", "d", "e"])),
            "a/, b/, c/ and 2 more directories"
        );
    }

    #[test]
    fn the_project_failing_is_a_warning_and_nested_configs_alone_an_info() {
        use spirolo_model::Severity::{Info, Warning};
        let severity = |scopes: &[&str], uninstalled: bool| {
            Failure {
                uninstalled,
                ..failure(scopes)
            }
            .diagnostic(1)
            .severity
        };
        assert_eq!(severity(&["", "packages/ui"], false), Warning);
        assert_eq!(severity(&["prototypes/dental"], true), Warning);
        assert_eq!(severity(&["prototypes/dental"], false), Info);
    }

    /// `check --all` over a copy of the Astro site fixture without its install, with
    /// untested-hotspot opted into `warn` and every other rule at `info`, under a table where
    /// untested-hotspot's verdict is `verdict`: the run's exit code and the hotspot row.
    fn gated_run(name: &str, verdict: Verdict) -> (u8, serde_json::Value) {
        use clap::Parser as _;
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target")
            .join(format!("resolution-{name}"));
        let _ = std::fs::remove_dir_all(&root);
        copy(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/resolution-astro-site/project"),
            &root,
        );
        let mut rules: serde_json::Map<String, serde_json::Value> = spirolo_rules::registry()
            .iter()
            .map(|rule| (rule.name().into(), "info".into()))
            .collect();
        rules.insert(
            "untested-hotspot".into(),
            serde_json::json!({ "severity": "warn" }),
        );
        std::fs::write(
            root.join(".spirolorc.jsonc"),
            serde_json::json!({ "rules": rules }).to_string(),
        )
        .unwrap();
        let mut table = spirolo_rules::status::parse(include_str!(
            "../../spirolo-rules/tests/fixtures/rule-status-939f0af.json"
        ))
        .unwrap();
        table.rules.get_mut("untested-hotspot").unwrap().verdict = verdict;
        spirolo_rules::status::with_table(Box::leak(Box::new(table)), || {
            let args = [
                "spirolo",
                "check",
                root.to_str().unwrap(),
                "--all",
                "--format",
                "json",
            ];
            let crate::Command::Check(check) = crate::Cli::try_parse_from(args).unwrap().command
            else {
                unreachable!()
            };
            let (json, code) = crate::render_check(&check.common, None, false).unwrap();
            let json: serde_json::Value = serde_json::from_str(&json).unwrap();
            let row = json["findings"]
                .as_array()
                .unwrap()
                .iter()
                .find(|finding| finding["rule"] == "untested-hotspot")
                .unwrap()
                .clone();
            (code, row)
        })
    }

    #[test]
    fn a_marked_row_a_project_opted_into_gating_still_gates() {
        // An advisory verdict lets the override through: the marked row stays `warn` and the
        // run exits 1, so a CI job without an install fails rather than passes.
        let (code, row) = gated_run("gates", Verdict::Unadjudicated);
        assert!(row["message"]
            .as_str()
            .unwrap()
            .ends_with("(unreliable: module resolution failed)"));
        assert_eq!(row["severity"], "warn");
        assert_eq!(code, 1);
    }

    #[test]
    fn a_marked_row_of_a_hidden_rule_never_gates() {
        // A FAIL or DROPPED verdict hides the rule and refuses the override: marking changes
        // nothing about that, the row is `info` and the run exits 0.
        for (name, verdict) in [("fail", Verdict::Fail), ("dropped", Verdict::Dropped)] {
            let (code, row) = gated_run(name, verdict);
            assert!(row["evidence"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["key"] == "unreliable"));
            assert_eq!(row["severity"], "info", "{name}");
            assert_eq!(code, 0, "{name}");
        }
    }

    fn copy(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for entry in std::fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let target = to.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy(&entry.path(), &target);
            } else {
                std::fs::copy(entry.path(), target).unwrap();
            }
        }
    }
}
