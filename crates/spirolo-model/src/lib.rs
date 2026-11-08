//! The whole-project model of a JavaScript test suite: the tables every rule family
//! queries, and the one function that builds them from a directory on disk.
//!
//! This crate is the bottom of the workspace's dependency chain
//! (`model ← frameworks ← inputs ← rules ← report ← spirolo`): it knows nothing about a
//! CLI, an editor or an output format.
//!
//! [`Model::build`] walks the tree once, parses every JS/TS file it is allowed to see
//! into its own arena on a work-stealing pool, keeps the facts and drops the AST, then
//! merges and sorts. Later slices fill more of each row; the schema is already complete.
//!
//! # What the model reads from disk
//!
//! Nothing is executed and nothing leaves the project root, but the walk is not the only
//! read. Every filesystem contact, so a consumer knows what a build depends on:
//!
//! - **the walk** (`ignore`-aware, honouring `.gitignore` when asked): every JS/TS file,
//!   parsed as a module; every `package.json`, read as data for its name and directory —
//!   the one source of the package set the aliases and the package boundary are drawn
//!   from — and for a `jest` key; and the JSON runner/mutation configs
//!   (`jest.config.json`, `stryker.conf.json`, `stryker.config.json`), read as data into
//!   the `Gate` table;
//! - **the resolver** (`oxc_resolver`): for each import, the nearest `tsconfig.json` /
//!   `jsconfig.json` upward from the importing file (auto-discovered, `extends` followed),
//!   the `package.json` of any package a specifier lands in (`exports`, `main`,
//!   `module`), directory listings and file probes for extensions, and a Yarn `PnP`
//!   manifest (`.pnp.cjs` / `.pnp.js`) at the root when one exists — `.gitignore` is not
//!   consulted here, as the runner's own resolution does not consult it either;
//! - **the preset reader**: a Vitest/Jest config that inherits its globs through
//!   `mergeConfig` is re-read as source, and each imported preset it names is resolved
//!   and read the same way, up to a fixed depth;
//! - **the command reader**: the root package's Node/yargs test wrapper and the config
//!   selected by its literal defaults are re-read from files already found by the walk;
//! - **the CI reader**: GitHub workflows and root `.circleci/config.yml`, plus the root
//!   `Makefile` recipes invoked by CI and checked-in Node wrappers with static spawn
//!   commands; package scripts come from the manifests above;
//! - **the workspace reader**: `pnpm-workspace.yaml` and the root `package.json`
//!   (`workspaces`) at the root, once.
//!
//! Environment variables, the network, the clock and anything outside the root are never
//! touched.
//!
//! ```
//! use spirolo_model::{BuildOptions, Model};
//!
//! let model = Model::build(std::path::Path::new("."), &BuildOptions::default());
//! assert!(model.test_cases.is_empty());
//! ```

mod ci;
mod commands;
pub mod config_reader;
mod constants;
mod cypress;
mod diagnostic;
mod extract;
mod flake;
mod holders;
mod manifest;
mod modules;
mod pairing;
mod paths;
mod presets;
mod reach;
mod recognise;
mod resolve;
mod scope;
mod setup;
mod sizes;
mod span;
mod state;
mod tables;
mod typecheck;
mod value_less;
mod walk;
mod workspace;

use std::path::{Path, PathBuf};

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

pub use crate::ci::command_words;
pub use crate::diagnostic::{Diagnostic, Severity};
pub use crate::paths::relative_slash;
pub use crate::reach::{Importers, TestReach};
pub use crate::recognise::{FileContext, Recogniser};
pub use crate::resolve::PLATFORMS;
pub use crate::span::{LineIndex, Span};
pub use crate::tables::{
    Assertion, Bail, Churn, CollectionProject, Constant, ContextRef, Coverage, Each, Enclosing,
    Export, ExportKind, FlakeGuard, FlakeKind, FlakeNames, FlakeSite, FlakeSource, Gate, GateFlag,
    GateTool, Handler, HandlerLibrary, Hook, IdentRef, Import, ImportKind, Invariant, JoinedCase,
    Language, LineCoverage, Load, Mock, MockKind, Module, MutantStatus, Mutants, MutationReport,
    MutationTest, Pairing, PairingReason, ParseStatus, PatternKind, PresetImport, Run, RunOutcome,
    SagaStepKind, SagaWalk, SetupInvariants, Skip, SkipKind, Snapshot, SnapshotEntry, Spy,
    StateAccess, TestCase, TestFile, TestKind, Threshold, ThrowingQuery, Uncollected,
};
pub use crate::value_less::ValueLess;

use crate::extract::{extract_file, FileFacts};

/// How to build a model. Every knob has a default that matches what a runner does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildOptions {
    /// Honour `.gitignore` and friends while walking. On by default, as the runner does.
    pub respect_gitignore: bool,
    /// Globs naming the files handlers are read from. Empty keeps each recogniser's own
    /// default (MSW/nock read only files under a `mocks` or `msw` directory); a non-empty
    /// list, as `.spirolorc.jsonc` supplies, replaces that default with the user's globs.
    pub handler_globs: Vec<String>,
    /// The file `handler_globs` were read from, named as the `file` of a `glob_invalid`
    /// diagnostic so a typo points at the line to fix. `None` when the globs came from
    /// nowhere the model can point at (a library caller's own list).
    pub handler_globs_source: Option<String>,
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            respect_gitignore: true,
            handler_globs: Vec::new(),
            handler_globs_source: None,
        }
    }
}

/// Everything the model knows about one project, built in one pass and then queried.
///
/// Tables are sorted by path then span before they leave this crate, so two builds of
/// the same tree produce byte-identical output.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Model {
    /// The directory the model was built from. Absolute, machine-specific, and
    /// therefore never serialized: every path in the tables is relative to it.
    #[serde(skip)]
    pub root: PathBuf,
    /// One row per source file read.
    pub modules: Vec<Module>,
    /// One row per test, suite, hook or benchmark recognised.
    pub test_cases: Vec<TestCase>,
    /// Files selected by static runner collection, including files whose cases
    /// are generated by helpers or whose bodies cannot be parsed. Case recognition
    /// remains in `test_cases`; collection never fabricates cases or assertions.
    #[serde(default)]
    pub test_files: Vec<TestFile>,
    /// One row per test file paired with the source it exercises.
    pub pairings: Vec<Pairing>,
    /// One row per network request interceptor declared in the tree.
    pub handlers: Vec<Handler>,
    /// One row per saga step-walk site: a `gen.next().value` assertion against an effect
    /// creator, or a `testSaga` chain step.
    pub saga_walks: Vec<SagaWalk>,
    /// One row per flake mechanism written in any file (a clock read that reaches an
    /// assertion, an unseeded random, a fixed wait, a `networkidle` wait), read by
    /// `flake-candidate`. Not serialized — an in-memory fact, like [`Model::snapshots`].
    #[serde(skip)]
    pub flake_sites: Vec<FlakeSite>,
    /// One row per call that neutralises a flake mechanism in scope (fake timers, a seed, a
    /// mock of a randomness source), read by `flake-candidate`. Not serialized.
    #[serde(skip)]
    pub flake_guards: Vec<FlakeGuard>,
    /// One row per access to module-scope state from inside a function, read by
    /// `flake-candidate` for `shared-state` and `order-dependence`. Not serialized.
    #[serde(skip)]
    pub state_accesses: Vec<StateAccess>,
    /// One row per file: the names its code references, by enclosing function, read by
    /// `flake-candidate` to follow a test into a test-only module. Not serialized.
    #[serde(skip)]
    pub flake_names: Vec<FlakeNames>,
    /// Every package's entry modules, repo-relative: what its `package.json` `main`,
    /// `module`, `types`, `source` and `exports` name, where a module exists at that path.
    /// Code a package publishes is production code even when nothing in the repository
    /// imports the package, so `flake-candidate` never calls it test-only. Not serialized.
    #[serde(skip)]
    pub package_entries: Vec<String>,
    /// One row per config file that can fail the build.
    pub gates: Vec<Gate>,
    /// One row per test outcome in a recorded run, read from the normalized run files
    /// under `.spirolo/runs/` that `spirolo flake ingest` writes. `check` and `spirolo flake`
    /// fill it; `list` leaves it empty.
    pub runs: Vec<Run>,
    /// One row per file covered by an ingested coverage report. Empty until M4.
    pub coverage: Vec<Coverage>,
    /// One row per mutant ingested from a mutation report. Filled by the binary from the
    /// `inputs.mutation` reports through `spirolo-inputs`; empty when the project has none.
    pub mutants: Vec<Mutants>,
    /// One row per test a mutation report names, joined to a `TestCase` where it can be.
    pub mutation_tests: Vec<MutationTest>,
    /// One row per mutation report read, with its provenance and whether it is usable.
    pub mutation_reports: Vec<MutationReport>,
    /// One row per file the model recognised as a test file that its runner's include and
    /// exclude would never collect: a test that exists and never runs, read by
    /// `orphan-test`. Not serialized — an in-memory fact, filled by [`scope`](crate::scope).
    #[serde(skip)]
    pub uncollected: Vec<Uncollected>,
    /// One row per `__snapshots__/<name>.snap` file, read by `stale-snapshot`. Filled by the
    /// binary after the build, the way the mutation tables are; empty when the project keeps
    /// no external snapshots. Not serialized — a `.snap` entry is a finding, not a `list`
    /// table.
    #[serde(skip)]
    pub snapshots: Vec<Snapshot>,
    /// One row per path the supplied churn input (`inputs.churn` or `--churn`) names, read
    /// by `untested-hotspot`. `None` when no churn input was supplied, which the rule reports
    /// rather than guesses at; filled by the binary after the build. Not serialized.
    #[serde(skip)]
    pub churn: Option<Vec<Churn>>,
    /// What the model could not decide. Never aborts a run.
    pub diagnostics: Vec<Diagnostic>,
}

/// The repo-relative modules the manifests name as entries, sorted and deduplicated: each
/// spelling joined to its package's directory, kept only where the walk read a module.
fn package_entries(model: &Model, manifests: &[manifest::Manifest]) -> Vec<String> {
    let modules: std::collections::HashSet<&str> =
        model.modules.iter().map(|m| m.path.as_str()).collect();
    let mut entries: Vec<String> = manifests
        .iter()
        .flat_map(|manifest| {
            manifest.entries.iter().filter_map(|spelling| {
                let mut parts: Vec<&str> = if manifest.dir.is_empty() {
                    Vec::new()
                } else {
                    manifest.dir.split('/').collect()
                };
                for part in spelling.split('/') {
                    match part {
                        "" | "." => {}
                        ".." => {
                            parts.pop();
                        }
                        other => parts.push(other),
                    }
                }
                let path = parts.join("/");
                modules.contains(path.as_str()).then_some(path)
            })
        })
        .collect();
    entries.sort();
    entries.dedup();
    entries
}

impl Model {
    /// Build the model of the project rooted at `root`.
    ///
    /// Reads the tree and nothing else: no test is executed, no config evaluated, no
    /// network touched. Anything undecidable lands in [`Model::diagnostics`] rather
    /// than failing the build.
    ///
    /// The walk is sequential and sorted; extraction is one rayon task per file, each
    /// with its own arena; the merge is sequential again. Nothing about the result
    /// depends on how the pool scheduled the middle phase.
    ///
    /// This form builds the model with no recognisers, so the `TestCase` table is
    /// empty. It exists for the crates below `spirolo-frameworks` in the dependency chain,
    /// which cannot name a recogniser; the binary uses [`Model::build_with`].
    pub fn build(root: &Path, options: &BuildOptions) -> Self {
        Self::build_with(root, options, &[])
    }

    /// Build the model, running every recogniser in `recognisers` over each parsed file.
    ///
    /// The model crate is the leaf of the workspace and cannot depend on the recognisers
    /// in `spirolo-frameworks`; a caller that can (the binary) passes them in here. Each
    /// recogniser sees a file inside the parallel phase, while its arena is still alive,
    /// and returns owned rows — see [`Recogniser`].
    pub fn build_with(
        root: &Path,
        options: &BuildOptions,
        recognisers: &[&dyn Recogniser],
    ) -> Self {
        // The resolver, the workspace aliases and the preset chain all work on absolute
        // paths; a relative root (`spirolo list ./repo`) would make every one of them fail
        // silently. Paths in the tables stay relative to this root either way.
        let canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let root = canonical.as_path();
        let files = walk::discover(root, options.respect_gitignore);
        // Compile the handler globs once, before the parallel phase, so every file matches
        // against the same shared matcher rather than rebuilding it per file.
        let (handler_globs, glob_diagnostics) = compile_globs(
            &options.handler_globs,
            options.handler_globs_source.as_deref(),
        );

        let facts: Vec<FileFacts> = files
            .par_iter()
            .map(|(path, language)| {
                read_file(root, path, *language, recognisers, handler_globs.as_ref())
            })
            .collect();

        let mut model = Self {
            root: root.to_path_buf(),
            diagnostics: glob_diagnostics,
            ..Self::default()
        };
        // Constant facts are keyed by module path, so the whole-project fold can follow an
        // import to the file that exports the name once resolution has run.
        let mut fold_facts = std::collections::HashMap::new();
        // The packages the walk found, in path order: the alias table and the package
        // boundary come from here and nowhere else.
        let mut manifests = Vec::new();
        for FileFacts {
            module,
            test_cases,
            handlers,
            saga_walks,
            flake,
            gates,
            fold,
            manifest,
            diagnostics,
        } in facts
        {
            if let Some(module) = &module {
                fold_facts.insert(module.path.clone(), fold);
            }
            model.modules.extend(module);
            let test_cases_empty = test_cases.is_empty();
            model.test_cases.extend(test_cases);
            model.handlers.extend(handlers);
            model.saga_walks.extend(saga_walks);
            model.flake_sites.extend(flake.sites);
            model.flake_guards.extend(flake.guards);
            model.state_accesses.extend(flake.state);
            let mut names = flake.names;
            if test_cases_empty {
                names.uses = Vec::new();
            }
            model.flake_names.push(names);
            model.gates.extend(gates);
            manifests.extend(manifest);
            model.diagnostics.extend(diagnostics);
        }
        // CI workflows are YAML, not modules: read them on their own after the JS/TS gates are
        // in, and append one CI gate per workflow. A workflow the model cannot read as UTF-8 is
        // skipped silently, the same way an unreadable directory is — an unread workflow is not
        // a fact about the suite's gates.
        for path in walk::discover_workflows(root, options.respect_gitignore) {
            let Some(spelling) = relative_slash(root, &path) else {
                continue;
            };
            if let Ok(source) = std::fs::read_to_string(&path) {
                model.gates.push(ci::read_workflow(&spelling, &source));
            }
        }

        // CircleCI is a root CI entry point too; GitHub may host only housekeeping jobs.
        if let Ok(source) = std::fs::read_to_string(root.join(".circleci/config.yml")) {
            model
                .gates
                .push(ci::read_workflow(".circleci/config.yml", &source));
        }
        ci::expand_steps(root, &manifests, &mut model.gates);

        model.package_entries = package_entries(&model, &manifests);
        let package_dirs: std::collections::HashSet<String> = manifests
            .iter()
            .map(|manifest| manifest.dir.clone())
            .collect();
        // Resolution is a whole-project step: it needs every module's path and every
        // workspace package before it can turn one file's specifiers into targets.
        let resolver = resolve::resolve(&mut model, &manifests);
        // Folding follows resolved imports across modules, so it runs after resolution.
        constants::fold(&mut model, &fold_facts);
        // A `.each` table or a loop read from a binding in another module is counted the
        // same way, once imports resolve.
        sizes::resolve(&mut model, &fold_facts);
        // A gate whose config inherits from an imported preset (`mergeConfig(variantConfig,
        // …)`, `export default baseConfig`) has empty fields until the preset is followed;
        // do that through the resolver before scoping, or the whole package would look
        // test-less and drop, and before the setup reader, which needs the preset's setup
        // files.
        let selected_configs = commands::discover(&mut model, &manifests);
        presets::resolve(&mut model, resolver.oxc());
        // The four fail-closed invariants live in the setup files' bodies, not the config
        // object; read them now the gates, their preset chains and the handler table (for MSW
        // presence) are all in place.
        setup::analyze(&mut model, &resolver);
        // The runner's include/exclude decides which recognised cases really are tests,
        // so a file the runner would never collect is a module but not a test. Runs once
        // the gate rows are in, and drops only from the test-case table.
        let scopes = scope::apply(&mut model, &package_dirs, &manifests, &selected_configs);
        // A module mock's target resolves like an import from the test file; done once
        // scoping has settled which cases the runner collects.
        resolve::resolve_mocks(&mut model, &resolver);
        // Pairing reads the resolved imports and, to leave Playwright's specs out, the
        // runner scopes, so it runs after both.
        pairing::pair(&mut model, &package_dirs, &|file| {
            scopes.playwright_collects(file)
        });
        model.sort();
        model
    }

    /// Total number of assertions across every test case.
    pub fn assertion_count(&self) -> usize {
        self.test_cases
            .iter()
            .map(|case| case.assertions.len())
            .sum()
    }

    /// Sort every table by its documented key so output is byte-stable across runs.
    fn sort(&mut self) {
        self.modules.sort();
        self.test_cases.sort();
        self.test_files.sort();
        self.pairings.sort();
        self.handlers.sort();
        self.saga_walks.sort();
        self.flake_sites.sort();
        self.flake_guards.sort();
        self.state_accesses.sort();
        self.flake_names.sort();
        self.gates.sort_by(|a, b| a.source.cmp(&b.source));
        self.runs.sort();
        self.coverage.sort();
        self.mutants.sort();
        self.mutation_tests.sort();
        self.mutation_reports.sort();
        self.snapshots.sort();
        self.diagnostics.sort();
    }
}

/// Compile handler globs into a shared matcher, or `None` when there are none, plus one
/// `glob_invalid` diagnostic per glob that does not compile, located on `source` when
/// the caller named the file the globs came from.
///
/// A leading `./` is noise once the glob is matched against repo-relative paths, so it is
/// stripped, mirroring the `scope` step. A glob that does not compile is skipped rather
/// than aborting the build, and reported rather than dropped silently; an all-skipped
/// list still returns an empty matcher, so "the user asked for globs but none compiled"
/// reads as "no file is a handler file" rather than falling back to the default
/// directories.
fn compile_globs(
    globs: &[String],
    source: Option<&str>,
) -> (Option<globset::GlobSet>, Vec<Diagnostic>) {
    if globs.is_empty() {
        return (None, Vec::new());
    }
    let mut builder = globset::GlobSetBuilder::new();
    let mut diagnostics = Vec::new();
    for glob in globs {
        let candidate = glob.strip_prefix("./").unwrap_or(glob);
        match globset::Glob::new(candidate) {
            Ok(compiled) => {
                builder.add(compiled);
            }
            Err(error) => diagnostics.push(Diagnostic {
                code: "glob_invalid".to_owned(),
                severity: Severity::Warning,
                file: source.map(str::to_owned),
                span: None,
                message: format!("ignoring handler glob {glob:?}: {error}"),
            }),
        }
    }
    (builder.build().ok(), diagnostics)
}

/// Read one discovered file from disk and hand it to the extractor, or to the data
/// reader when the walk marked it as data.
///
/// The three ways a file can fail before the parser sees it — no model spelling for its
/// path, unreadable, not UTF-8 — each become a diagnostic. The last two keep a
/// [`ParseStatus::Failed`] row, because a file the runner would try to load is a fact
/// about the suite even when its contents are not; the first has no path to key a row on.
fn read_file(
    root: &Path,
    path: &Path,
    language: Language,
    recognisers: &[&dyn Recogniser],
    handler_globs: Option<&globset::GlobSet>,
) -> FileFacts {
    let Some(spelling) = relative_slash(root, path) else {
        return FileFacts {
            diagnostics: vec![Diagnostic {
                code: "path_not_utf8".to_owned(),
                severity: Severity::Error,
                file: None,
                span: None,
                message: format!(
                    "{} has no UTF-8 spelling relative to the root",
                    path.display()
                ),
            }],
            ..FileFacts::empty()
        };
    };

    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            return FileFacts::failed(spelling, language, "unreadable_file", error.to_string())
        }
    };
    match String::from_utf8(bytes) {
        // A data file is read for its facts and never parsed as a module.
        Ok(source) if language == Language::Json => {
            FileFacts::data(&spelling, &source, recognisers)
        }
        Ok(source) => extract_file(&spelling, language, &source, recognisers, handler_globs),
        Err(error) => FileFacts::failed(
            spelling,
            language,
            "not_utf8",
            format!(
                "not valid UTF-8 at byte {}",
                error.utf8_error().valid_up_to()
            ),
        ),
    }
}
