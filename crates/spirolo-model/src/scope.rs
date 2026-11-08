//! Letting the runner decide which files are tests.
//!
//! A test file the runner would never collect is still a module — the model walked it,
//! parsed it, and tabled its imports and exports — but it is not a test, so its recognised
//! cases are dropped. This runs after extraction, once the [`Gate`] rows carry the globs,
//! and before pairing, which asks the same scopes which files Playwright collects.
//!
//! The decision is made the way the runner makes it, per framework and per package:
//!
//! - A row is judged only by its own framework's configs. A Vitest/Jest row answers to
//!   `vitest.config.*` / `jest.config.*`, a Playwright row to `playwright.config.*`; one
//!   family's globs never drop the other's rows.
//! - The nearest config scopes a file, including explicit globs that reach outside its
//!   directory. Globs use the config directory or its explicit rootDir. Inline projects
//!   at the same root are alternatives; their includes and excludes are never intersected.
//! - A config whose `include` could not be read (opaque, or simply absent) still applies —
//!   with the runner's own defaults, which is what the runner falls back to. No package is
//!   ever dropped because its config was not statically readable.
//! - With no governing config, an explicit import or the closest unambiguous package
//!   declaration selects the runner's defaults. Without either, the fallback is Vitest.
//! - In a workspace (`pnpm-workspace.yaml`, `package.json` `workspaces`), a file outside
//!   every member package is not a test and a config outside every member package is not
//!   a runner opinion: the workspace's runner never enters an archive or a prototype
//!   directory, whatever configs it carries. A root config is the one exception — it
//!   scopes the whole tree, as a root `vitest run` would. An explicit collection glob in
//!   a member's config can also reach files outside the workspace members.
//!
//! File inventory follows these same selectors independently of case recognition. It
//! includes generated suites and unreadable bodies without fabricating any test cases.

use std::collections::HashSet;

use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::manifest::Manifest;
use crate::workspace::Workspace;
use crate::{Diagnostic, Gate, GateTool, Model, Severity, TestFile, TestKind, Uncollected};

/// Vitest's default `include`, as Vitest documents it
/// (`**/*.{test,spec}.?(c|m)[jt]s?(x)`), spelled for [`globset`].
pub const VITEST_DEFAULT_INCLUDE: [&str; 1] = ["**/*.{test,spec}.?(c|m)[jt]s?(x)"];

/// Vitest's default `exclude`, as Vitest documents it.
pub const VITEST_DEFAULT_EXCLUDE: [&str; 5] = [
    "**/node_modules/**",
    "**/dist/**",
    "**/cypress/**",
    "**/.{idea,git,cache,output,temp}/**",
    "**/{karma,rollup,webpack,vite,vitest,jest,ava,babel,nyc,cypress,tsup,build,eslint,prettier}.config.*",
];

/// Jest 30's default `testMatch`, including CommonJS/ES-module extensions.
pub const JEST_DEFAULT_TEST_MATCH: [&str; 2] = [
    "**/__tests__/**/*.?([mc])[jt]s?(x)",
    "**/?(*.)+(spec|test).?([mc])[jt]s?(x)",
];

/// Before Jest 30, the defaults exclude .mjs/.cjs/.mts/.cts.
const JEST_LEGACY_TEST_MATCH: [&str; 2] = [
    "**/__tests__/**/*.[jt]s?(x)",
    "**/?(*.)+(spec|test).[tj]s?(x)",
];

fn jest_default_match(manifests: &[Manifest], dir: &str) -> &'static [&'static str] {
    let major = manifests
        .iter()
        .filter(|manifest| manifest.dir == dir || contains(&manifest.dir, dir))
        .filter_map(|manifest| manifest.jest_major.map(|major| (&manifest.dir, major)))
        .max_by_key(|(dir, _)| dir.len())
        .map(|(_, major)| major);
    if major.is_some_and(|major| major < 30) {
        &JEST_LEGACY_TEST_MATCH
    } else {
        &JEST_DEFAULT_TEST_MATCH
    }
}

/// Jest's default `testPathIgnorePatterns` — regular expressions, not globs.
pub const JEST_DEFAULT_IGNORE: [&str; 1] = ["/node_modules/"];

/// Playwright's default `testMatch`, as Playwright documents it.
pub const PLAYWRIGHT_DEFAULT_TEST_MATCH: [&str; 1] = ["**/*.@(spec|test).?(c|m)[jt]s?(x)"];

/// Drop every recognised case in a file its framework's runner would not collect, and
/// hand back the compiled scopes so a later step can ask the same question. `package_dirs`
/// are the directories holding a `package.json` the walk found, which the workspace
/// boundary needs to tell a member package from a bare directory. A config glob
/// that does not compile is skipped and reported as a `glob_invalid` (or, for a Jest
/// ignore pattern, `regex_invalid`) diagnostic on the config that declared it.
pub fn apply(
    model: &mut Model,
    package_dirs: &HashSet<String>,
    manifests: &[Manifest],
    selected: &[crate::commands::Selection],
) -> Scopes {
    let mut diagnostics = Vec::new();
    let mut scopes = Scopes::build(
        &model.gates,
        Workspace::read(&model.root, package_dirs),
        manifests,
        selected,
        &mut diagnostics,
    );
    scopes.cypress = model
        .modules
        .iter()
        .filter(|module| crate::cypress::is_config(&module.path))
        .map(|module| Cypress::compile(crate::cypress::read(&model.root, &module.path)))
        .collect();
    scopes.typecheck = crate::typecheck::checks(&model.root, manifests)
        .into_iter()
        .map(|check| {
            let include: Vec<&str> = check.include.iter().map(String::as_str).collect();
            let mut exclude: Vec<&str> = check.exclude.iter().map(String::as_str).collect();
            exclude.push("**/node_modules/**");
            matcher(&check.dir, &include, &exclude, &[]).0
        })
        .collect();
    // Explicit runner imports label their own vocabulary. Injected globals take
    // their label from the governing config, once the whole project's gates exist.
    let explicit: HashSet<&str> = model
        .modules
        .iter()
        .filter(|module| {
            module.imports.iter().any(|import| {
                matches!(
                    import.specifier.as_str(),
                    "vitest" | "vite-plus/test" | "@jest/globals"
                )
            })
        })
        .map(|module| module.path.as_str())
        .collect();
    for case in &mut model.test_cases {
        if case.framework.as_deref() != Some("playwright") && !explicit.contains(case.file.as_str())
        {
            if let Some(framework) = scopes.framework(&case.file) {
                case.framework = Some(framework.to_owned());
            }
        }
    }
    let playwright_files: HashSet<&str> = model
        .test_cases
        .iter()
        .filter(|case| case.framework.as_deref() == Some("playwright"))
        .map(|case| case.file.as_str())
        .collect();
    for module in &model.modules {
        let framework = if playwright_files.contains(module.path.as_str())
            || module
                .imports
                .iter()
                .any(|import| import.specifier == "@playwright/test")
        {
            "playwright"
        } else {
            scopes
                .runner
                .nearest(&module.path)
                .map(|entry| entry.framework)
                .or_else(|| {
                    module
                        .imports
                        .iter()
                        .find_map(|import| match import.specifier.as_str() {
                            "@jest/globals" => Some("jest"),
                            "vitest" | "vite-plus/test" => Some("vitest"),
                            _ => None,
                        })
                })
                .or_else(|| scopes.framework(&module.path))
                .unwrap_or("vitest")
        };
        if scopes.keeps(&module.path, Some(framework)) {
            model.test_files.push(TestFile {
                file: module.path.clone(),
                framework: framework.to_owned(),
            });
        }
    }
    // A file the runner would not collect is not a test, but a recogniser did claim a test
    // in it: record it as uncollected before its cases are dropped, so `orphan-test` can
    // report the test that exists and never runs. Keep the recognised files in first-seen
    // order, which is path order since the table is sorted, so the rows are deterministic.
    // A module a collected test imports, directly or through others, is loaded by that
    // test's run: the tests it registers — in an exported function the importer calls, or
    // at its top level — run there.
    let reached = imported_by_collected(model);
    let mut seen = HashSet::new();
    for case in &model.test_cases {
        if case.kind != TestKind::Test || !seen.insert(case.file.clone()) {
            continue;
        }
        if !scopes.keeps(&case.file, case.framework.as_deref())
            && !scopes.runs_elsewhere(&case.file)
            && !reached.contains(&case.file)
            && !is_codemod_fixture(&case.file)
        {
            model
                .uncollected
                .push(scopes.uncollected(&case.file, case.framework.as_deref()));
        }
    }
    model.uncollected.sort();
    model
        .test_cases
        .retain(|case| scopes.keeps(&case.file, case.framework.as_deref()));
    model.diagnostics.extend(diagnostics);
    mark_gates_that_collect_nothing(model, manifests);
    scopes
}

/// Mark every Vitest/Jest gate whose own selection matches no walked file, judged by its
/// globs alone — not by which config is nearest — so two configs sharing a directory are
/// each judged on what they would collect. A config another run loads as one of its
/// `projects` is part of that run whether or not it holds a test yet, so it is never marked;
/// nor is an opaque gate, whose selection may hang on what the reader could not read.
fn mark_gates_that_collect_nothing(model: &mut Model, manifests: &[Manifest]) {
    let Model { gates, modules, .. } = model;
    let loaded = loaded_projects(gates);
    for gate in gates.iter_mut() {
        if !matches!(gate.tool, GateTool::Vitest | GateTool::Jest)
            || gate.opaque
            || gate.selection_unapplied
        {
            continue;
        }
        let dir = config_dir(&gate.source);
        if loaded.is_match(&gate.source) || (!dir.is_empty() && loaded.is_match(dir)) {
            continue;
        }
        let dir = gate
            .collection_root
            .clone()
            .unwrap_or_else(|| config_dir(&gate.source).to_owned());
        let matchers: Vec<Matcher> = selections(gate)
            .iter()
            .map(|selection| {
                let dir = selection.collection_root.as_deref().unwrap_or(&dir);
                runner_matcher(selection, dir, manifests).matcher
            })
            .collect();
        gate.collects_no_tests = !modules.iter().any(|module| {
            matchers
                .iter()
                .any(|matcher| matcher.is_match(&module.path))
        });
    }
}

/// The directories and configs the runs load through their string `projects` entries, each
/// spelled from the root (joined to the declaring config's directory).
fn loaded_projects(gates: &[Gate]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for gate in gates {
        let dir = config_dir(&gate.source);
        for glob in &gate.project_globs {
            let glob = glob
                .trim_start_matches("<rootDir>/")
                .trim_start_matches("./");
            let _ = add(&mut builder, dir, glob.trim_end_matches('/'));
        }
    }
    builder.build().unwrap_or_else(|_| GlobSet::empty())
}

/// The selections a gate collects with: one per inline project, each inheriting the
/// gate's root when it sets none, or the gate itself when it declares no projects.
fn selections(gate: &Gate) -> Vec<Gate> {
    if gate.collection_projects.is_empty() {
        return vec![gate.clone()];
    }
    gate.collection_projects
        .iter()
        .map(|project| Gate {
            include: project.include.clone(),
            exclude: project.exclude.clone(),
            test_regex: project.test_regex.clone(),
            collection_root: project
                .root
                .clone()
                .or_else(|| gate.collection_root.clone()),
            collection_roots: project.roots.clone(),
            ..gate.clone()
        })
        .collect()
}

/// Whether a file sits under a `__testfixtures__` directory: jscodeshift's `defineTest`
/// reads its inputs and outputs there as text, so a `describe`/`it` in one is the code a
/// transform rewrites, not a test meant to run.
fn is_codemod_fixture(file: &str) -> bool {
    file.split('/').any(|segment| segment == "__testfixtures__")
}

/// Every module a collected test file imports, transitively, by resolved path.
fn imported_by_collected(model: &Model) -> HashSet<String> {
    let imports: std::collections::HashMap<&str, Vec<&str>> = model
        .modules
        .iter()
        .map(|module| {
            (
                module.path.as_str(),
                module
                    .imports
                    .iter()
                    .filter_map(|import| import.resolved.as_deref())
                    .collect(),
            )
        })
        .collect();
    // The runner loads every file it collects, whether or not a recogniser claimed a case
    // in it: a `schema.test.ts` that only calls imported suite functions is one.
    let mut stack: Vec<&str> = model
        .test_files
        .iter()
        .map(|test_file| test_file.file.as_str())
        .collect();
    let mut reached = HashSet::new();
    while let Some(file) = stack.pop() {
        for &next in imports.get(file).into_iter().flatten() {
            if reached.insert(next) {
                stack.push(next);
            }
        }
    }
    reached.into_iter().map(str::to_owned).collect()
}

/// The per-framework config scopes of one project, compiled once.
pub struct Scopes {
    /// Vitest and Jest configs, one entry each, plus the defaults for a file with none.
    runner: Family,
    jest_defaults: Matcher,
    jest_package_defaults: Vec<(String, Matcher)>,
    /// With a root script that runs Jest and no root config, that invocation collects
    /// with defaults. Nested configs belong to separate invocations and cannot veto it.
    root_jest_defaults: Option<Matcher>,
    runner_hints: Vec<(String, &'static str)>,
    /// Playwright configs, one entry each, plus the defaults for a file with none.
    playwright: Family,
    /// The workspace boundary, when the project declares one.
    workspace: Option<Workspace>,
    /// Configs outside every workspace member that their own package runs from a script.
    outside: Vec<Entry>,
    /// The type checks package scripts run, which collect type tests.
    typecheck: Vec<Matcher>,
    /// The Cypress configs, which collect specs no Vitest/Jest/Playwright scope decides.
    cypress: Vec<Cypress>,
}

/// One Cypress config's collection: the directory it governs and its compiled spec
/// patterns, or `None` when the config could not be read and what it collects is unknown.
struct Cypress {
    dir: String,
    matcher: Option<Matcher>,
}

impl Cypress {
    fn compile(config: crate::cypress::CypressConfig) -> Self {
        let matcher = config.spec_patterns.map(|patterns| {
            let patterns: Vec<&str> = patterns.iter().map(String::as_str).collect();
            matcher(&config.dir, &patterns, &["**/node_modules/**"], &[]).0
        });
        Self {
            dir: config.dir,
            matcher,
        }
    }
}

/// One framework family's scopes: every applicable config's compiled globs, and the
/// runner's defaults for a file no config governs.
struct Family {
    entries: Vec<Entry>,
    defaults: Matcher,
    /// The runner's default include/exclude in readable form, for the evidence of a file
    /// no config governs.
    default_include: Vec<String>,
    default_exclude: Vec<String>,
}

/// One config's scope: the directory it governs, what it collects there, and — for the
/// evidence of a file it omits — the config path and the globs it was judged against.
struct Entry {
    dir: String,
    config: String,
    framework: &'static str,
    include: Vec<String>,
    exclude: Vec<String>,
    matcher: Matcher,
}

/// A compiled include/exclude pair. `ignore` holds Jest's `testPathIgnorePatterns`, which
/// are regular expressions matched against the path rather than globs.
struct Matcher {
    include: GlobSet,
    test_regex: Vec<regex::Regex>,
    roots: Vec<String>,
    exclude: GlobSet,
    ignore: Vec<regex::Regex>,
}

impl Matcher {
    fn is_match(&self, file: &str) -> bool {
        (self.roots.is_empty() || self.roots.iter().any(|root| contains(root, file)))
            && (self.include.is_match(file)
                || self
                    .test_regex
                    .iter()
                    .any(|pattern| pattern.is_match(&format!("/{file}"))))
            && !self.exclude.is_match(file)
            && !self
                .ignore
                .iter()
                .any(|pattern| pattern.is_match(&format!("/{file}")))
    }
}

impl Scopes {
    fn build(
        gates: &[Gate],
        workspace: Option<Workspace>,
        manifests: &[Manifest],
        selected: &[crate::commands::Selection],
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Self {
        let mut runner = Vec::new();
        let mut playwright = Vec::new();
        let mut outside = Vec::new();
        for gate in gates {
            // An explicit --config in the package's test command replaces the default
            // config at that package root, including a package.json Jest guard.
            if gate.tool == GateTool::Jest
                && selected
                    .iter()
                    .any(|(dir, config)| config_dir(&gate.source) == dir && gate.source != *config)
            {
                continue;
            }
            let dir = gate
                .collection_root
                .as_deref()
                .unwrap_or_else(|| config_dir(&gate.source));
            // A config outside every workspace member is never run by the workspace's
            // runner; a root config is, over the whole tree. One whose own package runs
            // the runner from a script is run there, on its own: it is kept aside, to
            // speak only for the files it collects.
            let standalone = !dir.is_empty()
                && workspace
                    .as_ref()
                    .is_some_and(|workspace| !workspace.contains(dir));
            if standalone && !runs_runner(manifests, dir, gate.tool) {
                continue;
            }
            for selection in &selections(gate) {
                let dir = selection.collection_root.as_deref().unwrap_or(dir);
                let compiled = match gate.tool {
                    GateTool::Vitest | GateTool::Jest => runner_matcher(selection, dir, manifests),
                    GateTool::Playwright => playwright_matcher(gate, dir),
                    GateTool::Stryker | GateTool::Ci | GateTool::Unknown => continue,
                };
                diagnostics.extend(
                    compiled
                        .invalid
                        .into_iter()
                        .map(|pattern| pattern.diagnostic(&gate.source)),
                );
                let family = if standalone {
                    &mut outside
                } else if gate.tool == GateTool::Playwright {
                    &mut playwright
                } else {
                    &mut runner
                };
                family.push(Entry {
                    dir: dir.to_owned(),
                    config: gate.source.clone(),
                    framework: match gate.tool {
                        GateTool::Jest => "jest",
                        GateTool::Playwright => "playwright",
                        _ => "vitest",
                    },
                    include: compiled.include,
                    exclude: compiled.exclude,
                    matcher: compiled.matcher,
                });
            }
        }
        // The runners' own defaults are constants that compile; nothing to report.
        let (runner_defaults, _) =
            matcher("", &VITEST_DEFAULT_INCLUDE, &VITEST_DEFAULT_EXCLUDE, &[]);
        let (playwright_defaults, _) = matcher(
            "",
            &PLAYWRIGHT_DEFAULT_TEST_MATCH,
            &["**/node_modules/**"],
            &[],
        );
        Self {
            jest_defaults: matcher("", &JEST_DEFAULT_TEST_MATCH, &[], &JEST_DEFAULT_IGNORE).0,
            jest_package_defaults: manifests
                .iter()
                .filter(|manifest| manifest.jest_major.is_some())
                .map(|manifest| {
                    (
                        manifest.dir.clone(),
                        matcher(
                            "",
                            jest_default_match(manifests, &manifest.dir),
                            &[],
                            &JEST_DEFAULT_IGNORE,
                        )
                        .0,
                    )
                })
                .collect(),
            // Only a root script that runs `jest` collects with root defaults; a root that
            // only delegates (`lerna run test`) leaves each package to its own config.
            root_jest_defaults: (runs_runner(manifests, "", GateTool::Jest)
                && !gates.iter().any(|gate| {
                    gate.tool == GateTool::Jest
                        && gate
                            .collection_root
                            .as_deref()
                            .unwrap_or_else(|| config_dir(&gate.source))
                            .is_empty()
                }))
            .then(|| {
                matcher(
                    "",
                    jest_default_match(manifests, ""),
                    &[],
                    &JEST_DEFAULT_IGNORE,
                )
                .0
            }),
            runner_hints: manifests
                .iter()
                .filter_map(|manifest| {
                    let framework = match manifest.runner? {
                        GateTool::Jest => "jest",
                        GateTool::Vitest => "vitest",
                        _ => return None,
                    };
                    Some((manifest.dir.clone(), framework))
                })
                .collect(),
            runner: Family {
                entries: runner,
                defaults: runner_defaults,
                default_include: strings(&VITEST_DEFAULT_INCLUDE),
                default_exclude: strings(&VITEST_DEFAULT_EXCLUDE),
            },
            playwright: Family {
                entries: playwright,
                defaults: playwright_defaults,
                default_include: strings(&PLAYWRIGHT_DEFAULT_TEST_MATCH),
                default_exclude: strings(&["**/node_modules/**"]),
            },
            workspace,
            outside,
            typecheck: Vec::new(),
            cypress: Vec::new(),
        }
    }

    fn framework(&self, file: &str) -> Option<&'static str> {
        self.runner
            .nearest(file)
            .map(|entry| entry.framework)
            .or_else(|| {
                self.runner_hints
                    .iter()
                    .filter(|(dir, _)| contains(dir, file))
                    .max_by_key(|(dir, _)| dir.len())
                    .map(|(_, framework)| *framework)
            })
    }

    /// The evidence for a file its runner would not collect.
    ///
    /// The config it was judged against (the nearest one upward, or `None` when the
    /// defaults decided), that config's include and exclude globs, and whether the
    /// workspace boundary rather than the globs left it out.
    fn uncollected(&self, file: &str, framework: Option<&str>) -> Uncollected {
        let family = if framework == Some("playwright") {
            &self.playwright
        } else {
            &self.runner
        };
        let nearest = family.nearest(file);
        let outside_workspace = self.outside_reach(file, nearest);
        nearest.map_or_else(
            || Uncollected {
                file: file.to_owned(),
                framework: framework.map(str::to_owned),
                config: None,
                include: if framework == Some("jest") {
                    strings(&JEST_DEFAULT_TEST_MATCH)
                } else {
                    family.default_include.clone()
                },
                exclude: if framework == Some("jest") {
                    strings(&JEST_DEFAULT_IGNORE)
                } else {
                    family.default_exclude.clone()
                },
                outside_workspace,
            },
            |entry| Uncollected {
                file: file.to_owned(),
                framework: framework.map(str::to_owned),
                config: Some(entry.config.clone()),
                include: entry.include.clone(),
                exclude: entry.exclude.clone(),
                outside_workspace,
            },
        )
    }

    /// Whether the runner of `framework` would collect `file`.
    fn keeps(&self, file: &str, framework: Option<&str>) -> bool {
        if framework == Some("jest")
            && self
                .root_jest_defaults
                .as_ref()
                .is_some_and(|matcher| matcher.is_match(file))
        {
            return true;
        }
        let family = if framework == Some("playwright") {
            &self.playwright
        } else {
            &self.runner
        };
        let nearest = family.nearest(file);
        if self.outside_reach(file, nearest) {
            return false;
        }
        nearest.map_or_else(
            || {
                if framework == Some("jest") {
                    self.jest_package_defaults
                        .iter()
                        .filter(|(dir, _)| contains(dir, file))
                        .max_by_key(|(dir, _)| dir.len())
                        .map_or(&self.jest_defaults, |(_, matcher)| matcher)
                        .is_match(file)
                } else {
                    family.defaults.is_match(file)
                }
            },
            |entry| entry.matcher.is_match(file),
        )
    }

    /// Whether a Playwright config in the tree collects `file`: a spec under some
    /// `playwright.config.*`'s `testDir` that its `testMatch` names. A project with no
    /// Playwright config collects nothing here — the defaults are not consulted, or every
    /// `*.spec.ts` in a Vitest project would read as an end-to-end test.
    pub fn playwright_collects(&self, file: &str) -> bool {
        let nearest = self.playwright.nearest(file);
        !self.outside_reach(file, nearest)
            && nearest.is_some_and(|entry| entry.matcher.is_match(file))
    }

    /// Whether a runner other than the file's own scope runs it, or might: another Vitest
    /// or Jest config whose globs collect it (a second config a script selects with
    /// `--config`, beside the one nearest the file, or one outside the workspace that its own
    /// package's script runs), a type check a script runs over a type test, a Playwright config whose `testMatch` under its `testDir` names it (a spec whose cases were
    /// attributed to Jest because it destructures `test` from a `require`), a Cypress config
    /// whose spec patterns match it, or one under whose directory it sits that could not be
    /// read. Consulted only for the never-collected record, so a file Cypress runs
    /// is not reported as a test that never runs.
    fn runs_elsewhere(&self, file: &str) -> bool {
        let any = |family: &Family| {
            !self.outside_reach(file, family.nearest(file))
                && family
                    .entries
                    .iter()
                    .any(|entry| entry.matcher.is_match(file))
        };
        any(&self.runner)
            || any(&self.playwright)
            || self
                .outside
                .iter()
                .any(|entry| entry.matcher.is_match(file))
            || (crate::typecheck::is_type_test(file)
                && self.typecheck.iter().any(|check| check.is_match(file)))
            || self.cypress.iter().any(|cypress| {
                cypress.matcher.as_ref().map_or_else(
                    || contains(&cypress.dir, file),
                    |matcher| matcher.is_match(file),
                )
            })
    }

    /// Outside every workspace member only a root config can reach the file.
    fn outside_reach(&self, file: &str, nearest: Option<&Entry>) -> bool {
        self.workspace
            .as_ref()
            .is_some_and(|workspace| !workspace.contains(file))
            && nearest.is_none_or(|entry| !entry.dir.is_empty() && !entry.matcher.is_match(file))
    }
}

impl Family {
    /// The config governing `file`: the one in the deepest directory that contains it.
    fn nearest(&self, file: &str) -> Option<&Entry> {
        self.entries
            .iter()
            .filter(|entry| contains(&entry.dir, file) || entry.matcher.is_match(file))
            // Projects at the same root are a union: prefer any matching scope
            // at the nearest depth, rather than whichever project was read last.
            .max_by_key(|entry| (entry.dir.len(), entry.matcher.is_match(file)))
    }
}

/// Whether the manifest in `dir` has a script that invokes the config's runner.
fn runs_runner(manifests: &[Manifest], dir: &str, tool: GateTool) -> bool {
    let name = match tool {
        GateTool::Jest => "jest",
        GateTool::Vitest => "vitest",
        GateTool::Playwright => "playwright",
        GateTool::Stryker | GateTool::Ci | GateTool::Unknown => return false,
    };
    manifests
        .iter()
        .filter(|manifest| manifest.dir == dir)
        .flat_map(|manifest| &manifest.scripts)
        .any(|script| invokes(script, name))
}

/// Whether a script command runs the binary `name`: as a word, or through a path ending
/// in it (`./node_modules/jest/bin/jest.js`).
pub fn invokes(script: &str, name: &str) -> bool {
    script
        .split(|c: char| c.is_whitespace() || c == '&' || c == ';' || c == '|')
        .any(|word| {
            word == name
                || word
                    .rsplit('/')
                    .next()
                    .is_some_and(|last| last == name || last == format!("{name}.js"))
        })
}

fn contains(dir: &str, file: &str) -> bool {
    dir.is_empty()
        || file
            .strip_prefix(dir)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// A pattern a config declared that the model could not compile, so a rule can say which
/// glob was ignored rather than guess why a file was or was not collected.
struct InvalidPattern {
    /// The config field it was read from (`include`, `exclude`, `testPathIgnorePatterns`).
    field: &'static str,
    /// The pattern as the config spelled it.
    pattern: String,
    /// The compiler's complaint.
    error: String,
    /// True for a Jest ignore pattern, which is a regular expression rather than a glob.
    regex: bool,
}

impl InvalidPattern {
    fn diagnostic(self, source: &str) -> Diagnostic {
        Diagnostic {
            code: if self.regex {
                "regex_invalid"
            } else {
                "glob_invalid"
            }
            .to_owned(),
            severity: Severity::Warning,
            file: Some(source.to_owned()),
            span: None,
            message: format!(
                "ignoring {} {} {:?}: {}",
                self.field,
                if self.regex { "pattern" } else { "glob" },
                self.pattern,
                self.error
            ),
        }
    }
}

/// A compiled config scope plus the readable globs it was built from, so a file the scope
/// omits can name what it was judged against.
struct Compiled {
    matcher: Matcher,
    invalid: Vec<InvalidPattern>,
    include: Vec<String>,
    exclude: Vec<String>,
}

/// Owned copies of a glob slice, for the evidence a config or the defaults carry.
fn strings(globs: &[&str]) -> Vec<String> {
    globs.iter().map(|glob| (*glob).to_owned()).collect()
}

/// The matcher for a Vitest/Jest gate: its own globs where it declared them, the
/// runner's defaults where it did not (or where the reader could not see them).
fn runner_matcher(gate: &Gate, dir: &str, manifests: &[Manifest]) -> Compiled {
    let jest = gate.tool == GateTool::Jest;
    let default_include: &[&str] = if jest {
        jest_default_match(manifests, dir)
    } else {
        &VITEST_DEFAULT_INCLUDE
    };
    let include: Vec<&str> = if jest && !gate.test_regex.is_empty() {
        Vec::new()
    } else if gate.include.is_empty() {
        default_include.to_vec()
    } else {
        gate.include.iter().map(String::as_str).collect()
    };
    // Jest's ignore list is regular expressions; Vitest's exclude is globs. Jest
    // has no glob exclude, and Vitest's defaults apply only when it declared none.
    let (exclude, ignore): (Vec<&str>, Vec<&str>) = if jest {
        let ignore = if gate.exclude.is_empty() {
            JEST_DEFAULT_IGNORE.to_vec()
        } else {
            gate.exclude.iter().map(String::as_str).collect()
        };
        (Vec::new(), ignore)
    } else if gate.exclude.is_empty() {
        (VITEST_DEFAULT_EXCLUDE.to_vec(), Vec::new())
    } else {
        (
            gate.exclude.iter().map(String::as_str).collect(),
            Vec::new(),
        )
    };
    // Jest states its filter as `testPathIgnorePatterns`, Vitest as `exclude`; the readable
    // evidence carries whichever the config used.
    let evidence_exclude = if jest { &ignore } else { &exclude };
    // <rootDir> denotes the config's root, not a literal directory name. Globs
    // are rooted by matcher(); regexes see a slash-prefixed project path. Anchor
    // the substituted root so <rootDir>/docs cannot match src/__tests__/docs.spec.ts.
    let include: Vec<String> = include
        .iter()
        .map(|pattern| pattern.replace("<rootDir>/", ""))
        .collect();
    let regex_root = if dir.is_empty() {
        "^".to_owned()
    } else {
        format!("^/{}", regex::escape(dir))
    };
    let ignore: Vec<String> = ignore
        .iter()
        .map(|pattern| pattern.replace("<rootDir>", &regex_root))
        .collect();
    let (mut compiled, mut invalid) = matcher(
        dir,
        &include.iter().map(String::as_str).collect::<Vec<_>>(),
        &exclude,
        &ignore.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    for pattern in &gate.test_regex {
        match regex::Regex::new(&pattern.replace("<rootDir>", &regex_root)) {
            Ok(pattern) => compiled.test_regex.push(pattern),
            Err(error) => invalid.push(InvalidPattern {
                field: "testRegex",
                pattern: pattern.clone(),
                error: error.to_string(),
                regex: true,
            }),
        }
    }
    compiled.roots = gate
        .collection_roots
        .iter()
        .filter_map(|root| crate::paths::join_relative(dir, &root.replace("<rootDir>/", "")))
        .collect();
    if compiled.roots.is_empty() && !gate.test_regex.is_empty() && !dir.is_empty() {
        compiled.roots.push(dir.to_owned());
    }
    Compiled {
        matcher: compiled,
        invalid,
        include,
        exclude: strings(evidence_exclude),
    }
}

/// The matcher for a Playwright gate: `testMatch` (or its default) under `testDir` (or
/// the config's own directory), minus `testIgnore` and `node_modules`.
fn playwright_matcher(gate: &Gate, dir: &str) -> Compiled {
    let test_dir = gate
        .test_dir
        .as_deref()
        .map_or("", |test_dir| {
            test_dir.strip_prefix("./").unwrap_or(test_dir)
        })
        .trim_end_matches('/');
    let root = match (dir, test_dir) {
        (_, "" | ".") => dir.to_owned(),
        ("", test_dir) => test_dir.to_owned(),
        (dir, test_dir) => format!("{dir}/{test_dir}"),
    };
    let include: Vec<&str> = if gate.include.is_empty() {
        PLAYWRIGHT_DEFAULT_TEST_MATCH.to_vec()
    } else {
        gate.include.iter().map(String::as_str).collect()
    };
    let mut exclude: Vec<&str> = gate.exclude.iter().map(String::as_str).collect();
    exclude.push("**/node_modules/**");
    let (mut compiled, mut invalid) = matcher(&root, &include, &exclude, &[]);
    // A RegExp `testMatch` replaces the default globs and is matched against the path.
    if !gate.test_regex.is_empty() {
        if gate.include.is_empty() {
            compiled.include = GlobSet::empty();
        }
        if !root.is_empty() {
            compiled.roots.push(root);
        }
    }
    for pattern in &gate.test_regex {
        match regex::Regex::new(pattern) {
            Ok(pattern) => compiled.test_regex.push(pattern),
            Err(error) => invalid.push(InvalidPattern {
                field: "testMatch",
                pattern: pattern.clone(),
                error: error.to_string(),
                regex: true,
            }),
        }
    }
    Compiled {
        matcher: compiled,
        invalid,
        include: strings(&include),
        exclude: strings(&exclude),
    }
}

/// Compile one include/exclude/ignore triple, every glob rooted at `dir`, returning the
/// patterns that did not compile alongside so the caller can report them.
fn matcher(
    dir: &str,
    include: &[&str],
    exclude: &[&str],
    ignore: &[&str],
) -> (Matcher, Vec<InvalidPattern>) {
    let mut invalid = Vec::new();
    let mut include_builder = GlobSetBuilder::new();
    let mut exclude_builder = GlobSetBuilder::new();
    for glob in include {
        if let Err(error) = add(&mut include_builder, dir, glob) {
            invalid.push(InvalidPattern {
                field: "include",
                pattern: (*glob).to_owned(),
                error: error.to_string(),
                regex: false,
            });
        }
    }
    for glob in exclude {
        if let Err(error) = add(&mut exclude_builder, dir, glob) {
            invalid.push(InvalidPattern {
                field: "exclude",
                pattern: (*glob).to_owned(),
                error: error.to_string(),
                regex: false,
            });
        }
    }
    let mut compiled_ignore = Vec::new();
    for pattern in ignore {
        match regex::Regex::new(pattern) {
            Ok(compiled) => compiled_ignore.push(compiled),
            Err(error) => invalid.push(InvalidPattern {
                field: "testPathIgnorePatterns",
                pattern: (*pattern).to_owned(),
                error: error.to_string(),
                regex: true,
            }),
        }
    }
    let matcher = Matcher {
        include: include_builder.build().unwrap_or_else(|_| GlobSet::empty()),
        test_regex: Vec::new(),
        roots: Vec::new(),
        exclude: exclude_builder.build().unwrap_or_else(|_| GlobSet::empty()),
        ignore: compiled_ignore,
    };
    (matcher, invalid)
}

/// Add one glob, resolved relative to `dir`, to a matcher. A glob that does not compile
/// is skipped rather than aborting the build; the error comes back so the caller can
/// report it. An extglob fans out into several `globset` globs, which either all compile
/// or all fail on the same spelling, so the first error is the glob's error.
fn add(builder: &mut GlobSetBuilder, dir: &str, glob: &str) -> Result<(), globset::Error> {
    // A leading `./` is noise once the glob is rooted at the config's directory.
    let glob = glob.strip_prefix("./").unwrap_or(glob);
    for variant in extglob_variants(glob) {
        let rooted = crate::paths::join_relative(dir, &variant)
            .unwrap_or_else(|| format!("{dir}/{variant}"));
        builder.add(Glob::new(&rooted)?);
    }
    Ok(())
}

/// Spell a runner glob in [`globset`]'s dialect, as one glob per extglob choice.
///
/// Vitest, Jest and Playwright use `micromatch`/`picomatch`, whose extglobs
/// (`?(c|m)`, `@(spec|test)`, `+(spec|test)`, `*(x)`) `globset` does not parse, and a
/// `globset` brace alternation cannot be empty, so an optional group cannot be spelled
/// in one glob. Each `?(a|b)` / `*(a|b)` therefore fans out into the globs with nothing,
/// `a` and `b` in its place; `@(a|b)` and `+(a|b)` become the brace alternation `{a,b}`
/// (`+` is read as exactly one occurrence — the only spelling runners' defaults use). A
/// glob with no extglob comes back as itself.
pub fn extglob_variants(glob: &str) -> Vec<String> {
    let mut variants = vec![String::with_capacity(glob.len())];
    let bytes = glob.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'(' {
            if let Some(close) = glob[index + 1..].find(')') {
                let inner = &glob[index + 1..index + 1 + close];
                if inner.contains('|') {
                    let alternation = format!("{{{}}}", inner.replace('|', ","));
                    for variant in &mut variants {
                        variant.push_str(&alternation);
                    }
                    index += 1 + close + 1;
                    continue;
                }
            }
        }
        if matches!(byte, b'?' | b'@' | b'+' | b'*') && bytes.get(index + 1) == Some(&b'(') {
            if let Some(close) = glob[index + 2..].find(')') {
                let inner = &glob[index + 2..index + 2 + close];
                match byte {
                    b'?' | b'*' => {
                        let choices: Vec<&str> =
                            std::iter::once("").chain(inner.split('|')).collect();
                        variants = variants
                            .iter()
                            .flat_map(|prefix| {
                                choices
                                    .iter()
                                    .map(move |choice| format!("{prefix}{choice}"))
                            })
                            .collect();
                    }
                    _ => {
                        let alternation = format!("{{{}}}", inner.replace('|', ","));
                        for variant in &mut variants {
                            variant.push_str(&alternation);
                        }
                    }
                }
                index += 2 + close + 1;
                continue;
            }
        }
        for variant in &mut variants {
            variant.push(byte as char);
        }
        index += 1;
    }
    variants
}

/// The repo-relative directory a config path lives in; the empty string at the root.
fn config_dir(source: &str) -> &str {
    source.rfind('/').map_or("", |index| &source[..index])
}

#[cfg(test)]
mod tests {
    use super::{apply, extglob_variants};
    use crate::{Gate, GateTool, Model, Span, TestCase, TestKind};

    /// A gate carrying the given include/exclude globs at `source`.
    fn gate(source: &str, tool: GateTool, include: &[&str], exclude: &[&str]) -> Gate {
        Gate {
            include: include.iter().map(|g| (*g).to_owned()).collect(),
            exclude: exclude.iter().map(|g| (*g).to_owned()).collect(),
            ..Gate::blank(source, tool)
        }
    }

    /// A minimal test-case row in `file`, attributed to `framework`.
    fn case_of(file: &str, framework: &str) -> TestCase {
        TestCase {
            file: file.to_owned(),
            span: Span::default(),
            framework: Some(framework.to_owned()),
            title: Some("t".to_owned()),
            kind: TestKind::Test,
            suite_path: Vec::new(),
            assertions: Vec::new(),
            throwing_queries: Vec::new(),
            mocks: Vec::new(),
            skip: None,
            each: None,
            loops: Vec::new(),
            modifiers: Vec::new(),
            tags: Vec::new(),
            body_hash: None,
            empty: false,
            context: Vec::new(),
        }
    }

    fn case(file: &str) -> TestCase {
        case_of(file, "vitest")
    }

    fn files(model: &Model) -> Vec<&str> {
        model.test_cases.iter().map(|c| c.file.as_str()).collect()
    }

    fn uncollected(model: &Model) -> Vec<&str> {
        model.uncollected.iter().map(|u| u.file.as_str()).collect()
    }

    #[test]
    fn a_recognised_file_the_include_omits_is_recorded_as_uncollected() {
        let mut model = Model {
            gates: vec![gate(
                "vitest.config.ts",
                GateTool::Vitest,
                &["**/*.unit.ts"],
                &[],
            )],
            test_cases: vec![case("src/math.unit.ts"), case("src/orphan.test.ts")],
            ..Model::default()
        };
        apply(&mut model, &std::collections::HashSet::new(), &[], &[]);
        assert_eq!(
            files(&model),
            ["src/math.unit.ts"],
            "the collected file stays"
        );
        assert_eq!(
            uncollected(&model),
            ["src/orphan.test.ts"],
            "the file the include omits is recorded"
        );
        let row = &model.uncollected[0];
        assert_eq!(row.config.as_deref(), Some("vitest.config.ts"));
        assert_eq!(row.framework.as_deref(), Some("vitest"));
        assert_eq!(
            row.include,
            ["**/*.unit.ts"],
            "the include it was judged against"
        );
    }

    #[test]
    fn a_file_an_exclude_removes_is_uncollected_and_a_file_no_config_governs_names_the_defaults() {
        let mut model = Model {
            gates: vec![gate(
                "vitest.config.ts",
                GateTool::Vitest,
                &["**/*.test.ts"],
                &["**/generated/**"],
            )],
            test_cases: vec![
                case("src/a.test.ts"),
                case("src/generated/b.test.ts"),
                case("src/c.helper.ts"),
            ],
            ..Model::default()
        };
        apply(&mut model, &std::collections::HashSet::new(), &[], &[]);
        assert_eq!(files(&model), ["src/a.test.ts"]);
        assert_eq!(
            uncollected(&model),
            ["src/c.helper.ts", "src/generated/b.test.ts"],
            "both the excluded file and the one the include never matched are uncollected"
        );
        let excluded = model
            .uncollected
            .iter()
            .find(|u| u.file == "src/generated/b.test.ts")
            .unwrap();
        assert_eq!(excluded.exclude, ["**/generated/**"]);
    }

    #[test]
    fn a_recognised_file_no_config_governs_is_judged_against_the_runner_defaults() {
        let mut model = Model {
            // No gate at all: every file is judged by the runner's own defaults.
            test_cases: vec![case("src/helper.ts")],
            ..Model::default()
        };
        apply(&mut model, &std::collections::HashSet::new(), &[], &[]);
        assert_eq!(uncollected(&model), ["src/helper.ts"]);
        let row = &model.uncollected[0];
        assert_eq!(row.config, None, "no config governed it, the defaults did");
        assert_eq!(row.include, super::VITEST_DEFAULT_INCLUDE);
    }

    #[test]
    fn an_include_keeps_only_matching_files() {
        let mut model = Model {
            gates: vec![gate(
                "vitest.config.ts",
                GateTool::Vitest,
                &["**/*.unit.ts"],
                &[],
            )],
            test_cases: vec![case("src/math.unit.ts"), case("src/math.test.ts")],
            ..Model::default()
        };
        apply(&mut model, &std::collections::HashSet::new(), &[], &[]);
        assert_eq!(
            files(&model),
            ["src/math.unit.ts"],
            "the non-include file drops its cases"
        );
    }

    #[test]
    fn no_include_falls_back_to_the_runner_defaults() {
        let mut model = Model {
            gates: vec![gate("vitest.config.ts", GateTool::Vitest, &[], &[])],
            test_cases: vec![
                case("src/a.test.ts"),
                case("src/b.spec.mjs"),
                case("src/helpers/c.ts"),
            ],
            ..Model::default()
        };
        apply(&mut model, &std::collections::HashSet::new(), &[], &[]);
        assert_eq!(
            files(&model),
            ["src/a.test.ts", "src/b.spec.mjs"],
            "Vitest's default include decides, so the helper is not a test"
        );
    }

    #[test]
    fn no_config_at_all_keeps_what_the_defaults_collect() {
        let mut model = Model {
            test_cases: vec![case("src/a.test.ts"), case("src/b.test.ts")],
            ..Model::default()
        };
        apply(&mut model, &std::collections::HashSet::new(), &[], &[]);
        assert_eq!(files(&model).len(), 2, "with no config, the defaults apply");
    }

    #[test]
    fn an_exclude_removes_a_file_the_include_matched() {
        let mut model = Model {
            gates: vec![gate(
                "vitest.config.ts",
                GateTool::Vitest,
                &["**/*.test.ts"],
                &["**/generated/**"],
            )],
            test_cases: vec![case("src/a.test.ts"), case("src/generated/b.test.ts")],
            ..Model::default()
        };
        apply(&mut model, &std::collections::HashSet::new(), &[], &[]);
        assert_eq!(
            files(&model),
            ["src/a.test.ts"],
            "the excluded file drops its cases"
        );
    }

    #[test]
    fn a_package_local_config_scopes_only_its_own_package() {
        let mut model = Model {
            gates: vec![gate(
                "packages/a/vitest.config.ts",
                GateTool::Vitest,
                &["**/*.unit.ts"],
                &[],
            )],
            test_cases: vec![
                case("packages/a/src/x.unit.ts"),
                case("packages/a/src/y.test.ts"),
                case("packages/b/src/y.test.ts"),
            ],
            ..Model::default()
        };
        apply(&mut model, &std::collections::HashSet::new(), &[], &[]);
        assert_eq!(
            files(&model),
            ["packages/a/src/x.unit.ts", "packages/b/src/y.test.ts"],
            "a's include governs a; b, with no config, keeps what the defaults collect"
        );
    }

    #[test]
    fn the_nearest_config_upward_wins() {
        let mut model = Model {
            gates: vec![
                gate("vitest.config.ts", GateTool::Vitest, &["**/*.unit.ts"], &[]),
                gate(
                    "packages/a/vitest.config.ts",
                    GateTool::Vitest,
                    &["**/*.test.ts"],
                    &[],
                ),
            ],
            test_cases: vec![
                case("packages/a/src/x.test.ts"),
                case("packages/a/src/x.unit.ts"),
                case("src/y.unit.ts"),
            ],
            ..Model::default()
        };
        apply(&mut model, &std::collections::HashSet::new(), &[], &[]);
        assert_eq!(
            files(&model),
            ["packages/a/src/x.test.ts", "src/y.unit.ts"],
            "a's config governs a; the root config governs the rest"
        );
    }

    #[test]
    fn a_vitest_include_never_drops_a_playwright_row() {
        let mut model = Model {
            gates: vec![
                gate(
                    "packages/a/vitest.config.ts",
                    GateTool::Vitest,
                    &["**/*.unit.ts"],
                    &[],
                ),
                Gate {
                    test_dir: Some("./tests".to_owned()),
                    ..gate(
                        "packages/e2e/playwright.config.ts",
                        GateTool::Playwright,
                        &[],
                        &[],
                    )
                },
            ],
            test_cases: vec![
                case("packages/a/src/x.unit.ts"),
                case_of("packages/e2e/tests/home.spec.ts", "playwright"),
                case_of("packages/e2e/src/helper.spec.ts", "playwright"),
            ],
            ..Model::default()
        };
        apply(&mut model, &std::collections::HashSet::new(), &[], &[]);
        assert_eq!(
            files(&model),
            [
                "packages/a/src/x.unit.ts",
                "packages/e2e/tests/home.spec.ts"
            ],
            "playwright rows answer to testDir, not to the vitest include"
        );
    }

    #[test]
    fn jest_ignore_patterns_are_regular_expressions() {
        let mut model = Model {
            gates: vec![gate(
                "jest.config.js",
                GateTool::Jest,
                &["**/*.test.js"],
                &["/fixtures/", "\\.slow\\.test\\.js$"],
            )],
            test_cases: vec![
                case_of("src/a.test.js", "jest"),
                case_of("src/fixtures/b.test.js", "jest"),
                case_of("src/c.slow.test.js", "jest"),
            ],
            ..Model::default()
        };
        apply(&mut model, &std::collections::HashSet::new(), &[], &[]);
        assert_eq!(
            files(&model),
            ["src/a.test.js"],
            "each ignore pattern is matched as a regex against the path"
        );
    }

    #[test]
    fn jest_default_test_match_reads_extglobs() {
        let mut model = Model {
            gates: vec![gate("jest.config.js", GateTool::Jest, &[], &[])],
            test_cases: vec![
                case_of("src/__tests__/a.js", "jest"),
                case_of("src/b.spec.tsx", "jest"),
                case_of("src/c.ts", "jest"),
                case_of("node_modules/d/e.test.js", "jest"),
            ],
            ..Model::default()
        };
        apply(&mut model, &std::collections::HashSet::new(), &[], &[]);
        assert_eq!(files(&model), ["src/__tests__/a.js", "src/b.spec.tsx"]);
    }

    #[test]
    fn only_runner_gates_scope_the_walk() {
        // A Stryker gate carries globs that are not the test include, so it never
        // filters the test-case table.
        let mut model = Model {
            gates: vec![gate(
                "stryker.conf.js",
                GateTool::Stryker,
                &["src/**/*.ts"],
                &[],
            )],
            test_cases: vec![case("src/a.test.ts")],
            ..Model::default()
        };
        apply(&mut model, &std::collections::HashSet::new(), &[], &[]);
        assert_eq!(
            files(&model).len(),
            1,
            "a mutation config does not scope tests"
        );
    }

    #[test]
    fn a_glob_that_does_not_compile_is_a_diagnostic_on_its_config() {
        let mut model = Model {
            gates: vec![gate(
                "packages/a/vitest.config.ts",
                GateTool::Vitest,
                &["src/[bad", "**/*.test.ts"],
                &[],
            )],
            test_cases: vec![case("packages/a/src/a.test.ts")],
            ..Model::default()
        };
        apply(&mut model, &std::collections::HashSet::new(), &[], &[]);
        let invalid: Vec<&crate::Diagnostic> = model
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "glob_invalid")
            .collect();
        assert_eq!(
            invalid.len(),
            1,
            "one bad glob, one diagnostic: {invalid:?}"
        );
        assert_eq!(
            invalid[0].file.as_deref(),
            Some("packages/a/vitest.config.ts"),
            "located on the config that declared it"
        );
        assert!(
            invalid[0].message.contains("src/[bad") && invalid[0].message.contains("include"),
            "names the glob and the field: {}",
            invalid[0].message
        );
        assert_eq!(
            files(&model),
            ["packages/a/src/a.test.ts"],
            "the globs that did compile still apply"
        );
    }

    #[test]
    fn a_jest_ignore_pattern_that_does_not_compile_is_a_diagnostic_on_its_config() {
        let mut model = Model {
            gates: vec![gate(
                "jest.config.js",
                GateTool::Jest,
                &[],
                &["/fixtures/", "(unclosed"],
            )],
            ..Model::default()
        };
        apply(&mut model, &std::collections::HashSet::new(), &[], &[]);
        let invalid: Vec<&crate::Diagnostic> = model
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "regex_invalid")
            .collect();
        assert_eq!(invalid.len(), 1, "{invalid:?}");
        assert_eq!(invalid[0].file.as_deref(), Some("jest.config.js"));
        assert!(
            invalid[0].message.contains("(unclosed"),
            "{}",
            invalid[0].message
        );
    }

    #[test]
    fn extglobs_fan_out_into_plain_globs() {
        assert_eq!(
            extglob_variants("**/*.@(spec|test).?(c|m)ts"),
            [
                "**/*.{spec,test}.ts",
                "**/*.{spec,test}.cts",
                "**/*.{spec,test}.mts"
            ]
        );
        assert_eq!(
            extglob_variants("**/?(*.)+(spec|test).[jt]s?(x)"),
            [
                "**/{spec,test}.[jt]s",
                "**/{spec,test}.[jt]sx",
                "**/*.{spec,test}.[jt]s",
                "**/*.{spec,test}.[jt]sx"
            ]
        );
        assert_eq!(extglob_variants("src/**/*.ts"), ["src/**/*.ts"]);
    }
}
