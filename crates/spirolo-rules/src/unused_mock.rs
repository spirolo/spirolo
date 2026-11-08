//! `unused-mock`: a mock left behind by a refactor — a module mock nothing the test file
//! imports resolves to, or a spy binding the file never reads.
//!
//! Two kinds, both severity `warn`, both a fact about the file:
//!
//! - **module mock.** A `vi.mock` / `jest.mock` (hoisted or in-body) whose target, resolved
//!   the way imports are resolved (or taken as the specifier for an external package), is
//!   reached by nothing in the test file's import closure — its own imports and runner-registry
//!   loads (`jest.requireActual`, `jest.requireMock`, `vi.importActual`, `vi.importMock`,
//!   except inside the factory of the mock of the same module), then theirs, transitively
//!   through project modules, each with its React Native platform variants (`X.ios.tsx`,
//!   `X.android.tsx`, `X.native.tsx`, `X.web.tsx` beside `X.tsx`: some platform's runner
//!   loads each). An external package is reached when any module in
//!   the closure imports it or a subpath of it, or imports the package the target is a
//!   subpath of; a target that does not resolve from the test
//!   file is reached when a module in the closure writes the same specifier. The mock call
//!   itself is not an import.
//!   When the target is not reached but the closure is incomplete — some import in it is
//!   neither a walked project module, an installed external package nor a Node builtin
//!   (`unknown_package`, `missing`, `unknown`, `outside_root`, `computed`: an alias, a
//!   workspace package the model did not walk into, an uninstalled dependency, an
//!   `import(specifier)`), a module did not parse, a file the model tables no module for
//!   (a `.vue` component; a data or asset file is a leaf) was imported, or a runtime import resolved to a
//!   declaration file with no sibling script (a declaration file *with* one is walked through
//!   it, the module the runner loads) — the row is could-not-decide, kept under `--propose`:
//!   what the unseen module imports is unknown. So is an unreached Node-builtin or package
//!   target when the closure loads any installed package at run time (the runner's own API
//!   aside): the walk stops at packages, and what they load themselves is unseen. So is a mock whose own target did not resolve for
//!   such a reason: which module it replaces is unknown.
//! - **spy binding.** A `vi.fn()` / `jest.fn()` / `vi.spyOn` / `jest.spyOn` result bound to a
//!   name that no other identifier reference in the file reads.
//!
//! A mock inside a `setupFiles` entry is judged against the union of the import closures of
//! every file the setup applies to, and reported once at the setup file. Identity is the file
//! plus the specifier (module mock) or the binding name (spy).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use spirolo_model::{
    GateTool, ImportKind, Load, Mock, MockKind, Model, Module, ParseStatus, PLATFORMS,
};

use crate::finding::{number_collisions, sort, Evidence, Finding, Location, Outcome, Proposal};
use crate::{Config, Family, Rule, Severity};

/// The `unused-mock` rule. See the module docs for the two kinds.
pub struct UnusedMock;

impl Rule for UnusedMock {
    fn name(&self) -> &'static str {
        "unused-mock"
    }

    fn family(&self) -> Family {
        Family::Hygiene
    }

    fn subject(&self) -> crate::Subject {
        crate::Subject::TestFiles
    }

    fn summary(&self) -> &'static str {
        "A module mock or spy in a test file disagrees with what the file imports or reads, so nothing uses it."
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn run(&self, model: &Model, config: &Config) -> Vec<Finding> {
        let severity = config.severity(self.name(), self.default_severity());
        let by_path: HashMap<&str, &Module> =
            model.modules.iter().map(|m| (m.path.as_str(), m)).collect();
        let test_files: HashSet<&str> = model.test_cases.iter().map(|c| c.file.as_str()).collect();

        let mut findings = Vec::new();
        for path in &test_files {
            let Some(module) = by_path.get(path) else {
                continue;
            };
            let closure = Closure::of(&[path], &by_path);
            for mock in module.mocks.iter().filter(|m| m.kind == MockKind::Module) {
                if let Some(finding) = self.module_mock_finding(module, mock, &closure, severity) {
                    findings.push(finding);
                }
            }
            let reads: HashSet<&str> = module.references.iter().map(|r| r.name.as_str()).collect();
            for spy in &module.spies {
                if !reads.contains(spy.binding.as_str()) {
                    findings.push(self.spy_finding(module, spy, severity));
                }
            }
        }

        findings.extend(self.setup_findings(model, &test_files, &by_path, severity));

        // Every unused mock, module or spy, is dead configuration a refactor left behind:
        // the proposal is uniformly to delete it, so it is set here rather than in each
        // builder. A mock the rule could not decide may be reached, so it is kept.
        if config.propose {
            for finding in &mut findings {
                finding.proposal = Some(match finding.outcome {
                    Outcome::CouldNotDecide => Proposal::Keep {
                        reason: "the import closure is incomplete; install and re-check".to_owned(),
                    },
                    _ => Proposal::Delete { what: None },
                });
            }
        }

        sort(&mut findings);
        number_collisions(&mut findings);
        findings
    }
}

impl UnusedMock {
    /// A finding for a module mock whose target the closure does not reach, or `None` when the
    /// target is reachable or cannot be decided (no target to search for).
    fn module_mock_finding(
        &self,
        module: &Module,
        mock: &Mock,
        closure: &Closure,
        severity: Severity,
    ) -> Option<Finding> {
        let target = mock.target.as_deref()?;
        if closure.reaches(mock) {
            return None;
        }
        if !closure.unresolved.is_empty() || target_unknown(mock) || closure.hides(mock) {
            return Some(self.undecided(module, mock, target, closure, severity));
        }
        let searched = closure.describe();
        Some(Finding {
            rule: self.name(),
            severity,
            outcome: Outcome::Finding,
            location: Location::at(module.path.clone(), mock.span),
            related: Vec::new(),
            message: format!("module mock of {target:?} is reached by nothing the test imports"),
            evidence: vec![
                Evidence::new("target", target.to_owned()),
                Evidence::new("kind", "module"),
                Evidence::new("searched", searched),
            ],
            explain: format!(
                "The test file mocks {target:?}, but nothing it imports — directly or \
                 transitively through the project's modules — resolves to that module. The \
                 mock replaces a module the code under test never loads, so it is dead \
                 configuration a refactor left behind. Delete the mock."
            ),
            anchor: target.to_owned(),
            test: None,
            ordinal: 0,
            proposal: None,
        })
    }

    /// The could-not-decide row for a module mock the closure does not reach but might: some
    /// import on the way is unresolved (an alias, a workspace package the model did not walk
    /// into, an uninstalled dependency, a missing file), and what that module imports is
    /// unknown.
    fn undecided(
        &self,
        module: &Module,
        mock: &Mock,
        target: &str,
        closure: &Closure,
        severity: Severity,
    ) -> Finding {
        let mut unresolved: BTreeSet<&str> =
            closure.unresolved.iter().map(String::as_str).collect();
        if target_unknown(mock) {
            unresolved.insert(target);
        }
        let packages: Vec<String> = if closure.hides(mock) {
            closure
                .packages
                .iter()
                .map(|package| format!("{package} (installed package, not walked)"))
                .collect()
        } else {
            Vec::new()
        };
        unresolved.extend(packages.iter().map(String::as_str));
        let unresolved = unresolved.into_iter().collect::<Vec<_>>().join(", ");
        let gap = if target_unknown(mock) {
            "the mock target itself did not resolve"
        } else {
            "the import closure is incomplete"
        };
        Finding {
            rule: self.name(),
            severity,
            outcome: Outcome::CouldNotDecide,
            location: Location::at(module.path.clone(), mock.span),
            related: Vec::new(),
            message: format!(
                "module mock of {target:?} is reached by nothing the model could follow, but \
                 {gap}"
            ),
            evidence: vec![
                Evidence::new("target", target.to_owned()),
                Evidence::new("kind", "module"),
                Evidence::new("searched", closure.describe()),
                Evidence::new("unresolved", unresolved.clone()),
            ],
            explain: format!(
                "The test file mocks {target:?}, and nothing the model followed from its \
                 imports resolves to that module, but the closure stops at modules whose \
                 imports the model could not see ({unresolved}). Whatever those modules import \
                 is unknown, so the mock may well be reached. Where the gap is a specifier that \
                 did not resolve, install the dependencies (or configure the alias) and check \
                 again."
            ),
            anchor: target.to_owned(),
            test: None,
            ordinal: 0,
            proposal: None,
        }
    }

    /// A finding for a spy binding no reference in the file reads.
    fn spy_finding(
        &self,
        module: &Module,
        spy: &spirolo_model::Spy,
        severity: Severity,
    ) -> Finding {
        let binding = &spy.binding;
        Finding {
            rule: self.name(),
            severity,
            outcome: Outcome::Finding,
            location: Location::at(module.path.clone(), spy.span),
            related: Vec::new(),
            message: format!("spy {binding:?} is bound but never read"),
            evidence: vec![
                Evidence::new("binding", binding.clone()),
                Evidence::new("kind", "spy"),
                Evidence::new("searched", "every identifier reference in the file"),
            ],
            explain: format!(
                "The spy {binding:?} is created with `vi.fn()` / `vi.spyOn` but no later \
                 reference in the file reads it — nothing asserts on its calls or wires it \
                 into the code under test. A spy nobody reads proves nothing. Delete the \
                 binding, or assert on it if the behaviour was meant to be checked."
            ),
            anchor: binding.clone(),
            test: None,
            ordinal: 0,
            proposal: None,
        }
    }

    /// Module-mock findings for the mocks a `setupFiles` entry declares. A setup file runs
    /// before every test its runner applies it to, so its mock is judged against the union of
    /// those files' import closures and reported once at the setup file — never once per test.
    /// The applicable files are the test files in the gate's own package (at or below its
    /// config's directory), the scope a per-package runner config governs.
    fn setup_findings(
        &self,
        model: &Model,
        test_files: &HashSet<&str>,
        by_path: &HashMap<&str, &Module>,
        severity: Severity,
    ) -> Vec<Finding> {
        // Keyed by (setup file, specifier) so a setup file loaded by two gates is reported once.
        let mut reported: BTreeMap<(String, String), Finding> = BTreeMap::new();
        for gate in &model.gates {
            if !matches!(gate.tool, GateTool::Vitest | GateTool::Jest)
                || gate.setup_files.is_empty()
            {
                continue;
            }
            let gate_dir = parent_dir(&gate.source);
            let applicable: Vec<&str> = test_files
                .iter()
                .copied()
                .filter(|file| under(&gate_dir, file))
                .collect();
            if applicable.is_empty() {
                continue;
            }
            let closure = Closure::of(&applicable, by_path);
            for spelling in &gate.setup_files {
                let Some(path) = resolve_setup(&gate_dir, spelling, by_path) else {
                    continue;
                };
                let Some(module) = by_path.get(path.as_str()) else {
                    continue;
                };
                for mock in module.mocks.iter().filter(|m| m.kind == MockKind::Module) {
                    if let Some(finding) =
                        self.module_mock_finding(module, mock, &closure, severity)
                    {
                        reported
                            .entry((path.clone(), finding.anchor.clone()))
                            .or_insert(finding);
                    }
                }
            }
        }
        reported.into_values().collect()
    }
}

/// The directory part of a repo-relative slash path — everything before the last `/`, or the
/// empty string for a path at the root.
pub fn parent_dir(path: &str) -> String {
    path.rsplit_once('/')
        .map_or_else(String::new, |(dir, _)| dir.to_owned())
}

/// Whether `file` lies at or below `dir` (the empty `dir` is the repo root — every file).
pub fn under(dir: &str, file: &str) -> bool {
    dir.is_empty() || file == dir || file.starts_with(&format!("{dir}/"))
}

/// The repo-relative module path a `setupFiles` spelling names, resolved against the gate's
/// directory the way an inherited-from-root or config-relative spelling is written, or `None`
/// when no module answers to it. Tries the spelling as a repo-relative path first, then joined
/// with the gate directory.
pub fn resolve_setup(
    gate_dir: &str,
    spelling: &str,
    by_path: &HashMap<&str, &Module>,
) -> Option<String> {
    let bare = spelling.trim_start_matches("./");
    if by_path.contains_key(bare) {
        return Some(bare.to_owned());
    }
    let joined = normalise_join(gate_dir, spelling);
    by_path.contains_key(joined.as_str()).then_some(joined)
}

/// Join `dir` and a possibly-`./`/`../` relative `spelling` into a normalised repo-relative
/// slash path.
pub fn normalise_join(dir: &str, spelling: &str) -> String {
    let mut parts: Vec<&str> = if dir.is_empty() {
        Vec::new()
    } else {
        dir.split('/').collect()
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
    parts.join("/")
}

/// Whether a module mock's target did not resolve for a reason that leaves it unknown (an
/// alias no config applies, an uninstalled package, a missing file), rather than because it
/// names an installed package or a Node builtin: which module it replaces cannot be told.
fn target_unknown(mock: &Mock) -> bool {
    mock.resolved.is_none()
        && !matches!(mock.reason.as_deref(), None | Some("external" | "builtin"))
}

/// The set of project module paths and external specifiers a test file reaches through its
/// import closure: its own imports, then theirs, transitively through resolved project
/// modules. A module mock is reachable when its resolved project path is in `project`, or —
/// for an external target — when some `external` specifier equals it or is a subpath of it.
struct Closure {
    /// Repo-relative paths of every project module reached (the seeds included).
    project: BTreeSet<String>,
    /// Every external specifier imported anywhere in the closure, as written.
    external: BTreeSet<String>,
    /// Every specifier written anywhere in the closure, resolved or not: a mock target the
    /// test file cannot resolve is still reached when a module writes the same specifier.
    written: BTreeSet<String>,
    /// The specifiers in the closure that are neither a walked project module, an installed
    /// external package nor a Node builtin (`unknown_package`, `missing`, `unknown`,
    /// `outside_root`, `computed`): modules whose own imports the walk could not see.
    /// Non-empty means the closure is incomplete, so an unreached target is undecidable.
    unresolved: BTreeSet<String>,
    /// The installed packages the closure loads at run time (any import but `import type`,
    /// the test runner's own API aside). The walk stops at them, so a Node builtin or another
    /// package they load themselves is reached unseen.
    packages: BTreeSet<String>,
}

impl Closure {
    /// Walk the import graph from `seeds` (repo-relative test-file paths), following resolved
    /// project imports and collecting external specifiers.
    fn of(seeds: &[&str], by_path: &HashMap<&str, &Module>) -> Self {
        let mut project = BTreeSet::new();
        let mut external = BTreeSet::new();
        let mut unresolved = BTreeSet::new();
        let mut written = BTreeSet::new();
        let mut packages = BTreeSet::new();
        let mut queue: Vec<String> = seeds.iter().map(|s| (*s).to_owned()).collect();
        while let Some(path) = queue.pop() {
            let Some(module) = by_path.get(path.as_str()) else {
                // A file the model tables no module for: a data or asset file imports nothing,
                // but anything else (a `.vue` / `.svelte` / `.astro` component) runs code
                // whose imports the walk cannot see.
                if !is_inert_asset(&path) {
                    unresolved.insert(format!("{path} (not read)"));
                }
                continue;
            };
            // A module the parser refused (or only recovered) lists none, or only some, of
            // its imports: whatever it loads is unknown.
            if module.parse_status != ParseStatus::Parsed {
                unresolved.insert(format!("{path} (not parsed)"));
            }
            // A runner-registry load (`jest.requireActual("x")`) loads `x` like a `require`,
            // except inside the factory of the mock of `x` itself, which only builds the mock.
            let loads = module
                .loads
                .iter()
                .filter(|load| !inside_own_factory(module, load))
                .map(|load| (&load.specifier, &load.resolved, &load.reason, true));
            let imports = module.imports.iter().map(|import| {
                (
                    &import.specifier,
                    &import.resolved,
                    &import.reason,
                    import.kind != ImportKind::Type,
                )
            });
            // A declaration file's own imports are types: they load nothing at run time.
            let runtime_importer = declaration_stem(&path).is_none();
            for (specifier, resolved, reason, runtime) in imports.chain(loads) {
                written.insert(specifier.clone());
                if let Some(resolved) = resolved {
                    if project.insert(resolved.clone()) {
                        queue.push(resolved.clone());
                        // React Native's runner loads `X.ios.tsx` / `X.android.tsx` where the
                        // resolver answered `X.tsx`: walk every platform's variant, since a
                        // module any platform loads may be the one the mock replaces.
                        for variant in platform_variants(resolved) {
                            if by_path.contains_key(variant.as_str())
                                && project.insert(variant.clone())
                            {
                                queue.push(variant);
                            }
                        }
                    }
                    // A runtime load the resolver answered with a declaration file runs the
                    // sibling script the declaration describes; with none, it runs something
                    // the model cannot see.
                    if let Some(stem) =
                        declaration_stem(resolved).filter(|_| runtime && runtime_importer)
                    {
                        match ["js", "mjs", "cjs"]
                            .iter()
                            .map(|extension| format!("{stem}.{extension}"))
                            .find(|sibling| by_path.contains_key(sibling.as_str()))
                        {
                            Some(sibling) => {
                                if project.insert(sibling.clone()) {
                                    queue.push(sibling);
                                }
                            }
                            None => {
                                unresolved.insert(format!("{resolved} (declaration file)"));
                            }
                        }
                    }
                } else {
                    if runtime && reason.as_deref() == Some("external") && !is_runner_api(specifier)
                    {
                        packages.insert(specifier.clone());
                    }
                    if !matches!(reason.as_deref(), Some("external" | "builtin")) {
                        unresolved.insert(specifier.clone());
                    }
                    external.insert(specifier.clone());
                }
            }
        }
        Self {
            project,
            external,
            written,
            unresolved,
            packages,
        }
    }

    /// Whether an unreached `mock` may be reached inside an installed package: its target is
    /// a Node builtin or a package (no project path), and the closure loads some package at
    /// run time whose own imports the walk did not follow.
    fn hides(&self, mock: &Mock) -> bool {
        mock.resolved.is_none() && !self.packages.is_empty()
    }

    /// Whether the closure reaches `mock`'s target: a resolved project path present in the
    /// project set, or — for a target with no project path — a specifier the closure writes
    /// (exactly or as a subpath).
    fn reaches(&self, mock: &Mock) -> bool {
        if let Some(resolved) = &mock.resolved {
            return self.project.contains(resolved);
        }
        let Some(target) = &mock.target else {
            return false;
        };
        // A module inside an installed package the closure loads (`pkg/lib/x` under
        // `import 'pkg'`) is one the package itself may load. A builtin is no package:
        // `fs` does not load `fs/promises`.
        let inside_imported = self
            .packages
            .iter()
            .any(|package| target.starts_with(&format!("{package}/")));
        inside_imported
            || self
                .written
                .iter()
                .any(|spec| spec == target || spec.starts_with(&format!("{target}/")))
    }

    /// A one-line description of what was searched, for the finding's evidence.
    fn describe(&self) -> String {
        format!(
            "the import closure — {}, {}",
            count(self.project.len(), "project module"),
            count(self.external.len(), "external specifier"),
        )
    }
}

/// Whether `specifier` is the test runner's own API (`vitest`, `@jest/globals`), which the
/// runner loads outside the module registry a mock replaces modules in.
fn is_runner_api(specifier: &str) -> bool {
    specifier == "vitest"
        || specifier == "@jest/globals"
        || specifier.starts_with("vitest/")
        || specifier.starts_with("@vitest/")
}

/// Whether `path` is a file a runner loads as data or a stub — JSON, a stylesheet, an image,
/// a font, text — which imports no other module.
fn is_inert_asset(path: &str) -> bool {
    const INERT: &[&str] = &[
        "json", "css", "scss", "sass", "less", "svg", "png", "jpg", "jpeg", "gif", "webp", "avif",
        "ico", "bmp", "woff", "woff2", "ttf", "otf", "eot", "txt", "md", "html", "yaml", "yml",
        "graphql", "gql", "wasm",
    ];
    path.rsplit_once('.')
        .is_some_and(|(_, extension)| INERT.contains(&extension.to_ascii_lowercase().as_str()))
}

/// The React Native platform variants of `path` (`X.tsx` or `X.ios.tsx` → `X.ios.tsx`,
/// `X.android.tsx`, …), whether or not they exist.
fn platform_variants(path: &str) -> Vec<String> {
    let Some((stem, extension)) = path.rsplit_once('.').filter(|(_, ext)| !ext.contains('/'))
    else {
        return Vec::new();
    };
    let stem = PLATFORMS
        .iter()
        .find_map(|platform| stem.strip_suffix(&format!(".{platform}")))
        .unwrap_or(stem);
    PLATFORMS
        .iter()
        .map(|platform| format!("{stem}.{platform}.{extension}"))
        .collect()
}

/// The path before a declaration-file extension (`.d.ts`, `.d.mts`, `.d.cts`), or `None` for
/// any other file.
fn declaration_stem(path: &str) -> Option<&str> {
    [".d.ts", ".d.mts", ".d.cts"]
        .iter()
        .find_map(|extension| path.strip_suffix(extension))
}

/// Whether `load` sits inside the factory of a mock in `module` whose target it names: the
/// factory's `requireActual` builds the mock from the real module and loads nothing else.
fn inside_own_factory(module: &Module, load: &Load) -> bool {
    module.mocks.iter().any(|mock| {
        mock.factory
            && mock.target.as_deref() == Some(load.specifier.as_str())
            && mock.span.start <= load.span.start
            && load.span.end <= mock.span.end
    })
}

/// `n noun` with the noun pluralised by a trailing `s` for anything but one.
fn count(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}
