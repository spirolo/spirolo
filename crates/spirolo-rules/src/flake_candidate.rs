//! `flake-candidate`: a test whose code, as written, can pass on one run and fail on the next
//! — the static half of the `flake` family, decided from source alone before any run.
//!
//! The rule reads the model's flake sites and guards (see `spirolo_model::FlakeSite`) and the
//! test cases, and never the `Run` table: "flagged before any run" is a fact about the code.
//!
//! **Claim** (narrowed in round 2 of its visit, #449): the test asserts on elapsed wall-clock
//! time, or on a calendar value both the code under test and the assertion read from the
//! clock, so it can fail on a slower machine or across a date boundary. Only `wall-clock`
//! sites are reported, with no fake timers or system-time set in scope: two clock reads
//! whose values meet in an assertion or a comparison (an elapsed-time budget; an assertion
//! that the elapsed time is at least a bound is none, since a slower machine passes it), or a calendar
//! field (`getFullYear()`, `toDateString()`) of a read an assertion expects and no other code
//! receives (a date boundary). One read typed in and expected back, or compared with a
//! constant, holds under any clock and is no site. A value the reader followed through a
//! call it cannot see into is could-not-decide. A deadline in the condition or body exit
//! of a loop that polls the awaited state caps a wait, and is no such comparison.
//! The model still records the other kinds (PRD 0004, "Kind semantics": unseeded random,
//! fixed timeouts, `networkidle`, dropped promises, real I/O, missing restores, shared state,
//! order dependence); two rounds of reading found them false in 33 of 35 rows, so the rule
//! does not report them.
//!
//! **Scope.** A mechanism counts for a test when it sits in the test's own body or in a hook
//! in the test's scope (`before*`/`after*` of the file or of an enclosing `describe`). A guard
//! counts there too, and at module scope or in the body of an enclosing `describe`.
//! Everything in a runner **setup file** counts for every test its config governs.
//!
//! A guard anywhere in a **test-only module** — a project module that only test and setup
//! files import (PRD 0004 story 10) — counts for every test of every file that imports it. A
//! mechanism there counts for a test only when the test's code reaches it, by name, as the
//! model records where each is written (`spirolo_model::Enclosing`):
//!
//! - at the module's top level, which runs on import: for every test of every importer;
//! - in a named function, method or class: when that name is in the closure, through the
//!   module's own call graph (each named code to the names it references), of the names the
//!   module's top level references and the names that call into the module for the test —
//!   those its own code uses when its file imports the module (its body, the hooks in its
//!   scope, and module scope and the `describe` bodies around it), every name of a setup file
//!   that imports the module, and every name of the production modules the test's file
//!   imports, directly or through one another, which may call back into a helper object the
//!   test hands them. A mock provider whose `search()` asserts an elapsed budget is no clock
//!   read for a test that only calls `getById()`, and is one for a test that calls
//!   `getRandom()`, which calls `this.search()`;
//! - in code the reader cannot tie to a name a test could use (a function assigned to
//!   `module.exports`, a callback at module scope passed to a call other than a hook or a
//!   module mock), or in a named function only such code calls: could-not-decide for every
//!   test of every importer. So is one in a hook a function of the module registers (in
//!   whichever suite calls it, which the rule cannot see), for a test that reaches the
//!   function; and every named mechanism of the module the test does not otherwise reach,
//!   when code it reaches may call anything (`this[k]()`, `Object.values(mock)`).
//!
//! An import is followed under the name its module gives it (`makeId` for
//! `import { makeId as id }`, `default` for a default import, which links to the name the
//! default export declares). Names are matched as written, never resolved to a binding, so
//! the closure over-approximates — reports a mechanism a test cannot run — where two pieces
//! of code share a name, where a name reached on one path reaches everything its code
//! references on every path, and where every name of the other modules the test's file
//! imports seeds the closure whether or not the test reaches that code. It misses, as a
//! known limit, a call through a value read from a computed member and called later
//! (`const f = p[k]; f()`), and a test-only module the test's file reaches only through
//! another module (a re-export from an `index`, a helper's own helper), which is not among
//! the files whose code runs for it at all. The finding keeps the mechanism's own location
//! as the related location. A module any non-test file imports is not the test's, and neither is a
//! package's entry (`main`, `exports`), even when only tests import it here: its
//! `Date.now()` is what the test exercises, not how the test is written. Nor is the module a
//! test file is paired with in its subject's place (next to it, or above its `__tests__`):
//! for that file it is the module under test. An unambiguous source-place pairing also
//! identifies a production subject for the other specs importing it. Modules explicitly
//! under `test-utils`, `test-helpers`, `test`, `tests`, `__tests__` or `__fixtures__` remain
//! test infrastructure for their other importers even when they have their own spec.
//!
//! **Skip gates.** A test behind an environment gate that is closed by default
//! (`skipIf(!process.env.PERF)`, on the test or an enclosing `describe`) is parked, not a live
//! risk: its candidates are listed at `info` with the gate as evidence and are never decided.
//!
//! **Severity.** `info` by default: advisory until M6 meets the recall and precision floors
//! M4 measured it below. A `.spirolorc.jsonc` can still raise it.
//!
//! One finding per test, anchored at the test, with every clock read in scope as a related
//! location.

use std::collections::{BTreeSet, HashMap};

use spirolo_model::{
    Enclosing, FlakeGuard, FlakeKind, FlakeNames, FlakeSite, GateTool, ImportKind, Model, Module,
    SkipKind, Span, TestCase, TestKind,
};

use crate::dead_skip::{env_variable, modifier_word};
use crate::finding::{number_collisions, sort, Evidence, Finding, Location, Outcome, TestPath};
use crate::unused_mock::{normalise_join, parent_dir, resolve_setup, under};
use crate::{tests, Config, Family, Rule, Severity};

/// The `flake-candidate` rule. See the module docs for the semantics.
pub struct FlakeCandidate;

impl Rule for FlakeCandidate {
    fn name(&self) -> &'static str {
        "flake-candidate"
    }

    fn family(&self) -> Family {
        Family::Flake
    }

    /// `info`: advisory. M4 measured recall 5 of 11 and precision 0 of 20 against the
    /// floors, so the rule ships listed, not blocking; its floors carry to M6.
    fn subject(&self) -> crate::Subject {
        crate::Subject::Tests
    }

    fn summary(&self) -> &'static str {
        "A test asserts on elapsed wall-clock time or on a calendar value it re-reads from the clock, so a slower machine or a date boundary can fail it."
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn run(&self, model: &Model, config: &Config) -> Vec<Finding> {
        let severity = config.severity(self.name(), self.default_severity());
        let cases = by_file(&model.test_cases, |c| &c.file);
        // The claim is narrowed to the wall clock (#449): the model still records every
        // other kind, which the rule no longer reports.
        let mut sites: HashMap<&str, Vec<&FlakeSite>> = HashMap::new();
        for site in model
            .flake_sites
            .iter()
            .filter(|s| s.kind == FlakeKind::WallClock)
        {
            sites.entry(site.file.as_str()).or_default().push(site);
        }
        let guards = by_file(&model.flake_guards, |g| &g.file);
        let reach = reached_files(model);
        let names: HashMap<&str, &FlakeNames> = model
            .flake_names
            .iter()
            .map(|n| (n.file.as_str(), n))
            .collect();
        let mut scopes: HashMap<&str, Scopes<'_>> = HashMap::new();
        let mut graphs = HashMap::new();
        let mut findings = Vec::new();
        for case in tests(model) {
            let reached = reach
                .files
                .get(case.file.as_str())
                .map_or(&[][..], Vec::as_slice);
            let file_cases = cases.get(case.file.as_str()).map_or(&[][..], Vec::as_slice);
            let mut follow = Follow {
                case,
                reach: &reach,
                names: &names,
                own: None,
                file_cases,
                scopes: &mut scopes,
                graphs: &mut graphs,
            };
            let (called, undecided) = follow.sites(reached, &sites);
            let ranges = scope_of(case, file_cases);
            let in_scope = |span: Span| ranges.iter().any(|r| contains(*r, span));
            let guards: Vec<&FlakeGuard> = guards
                .get(case.file.as_str())
                .into_iter()
                .flatten()
                .copied()
                .filter(|g| in_scope(g.span) || encloses(case, file_cases, g.span))
                .chain(
                    reached
                        .iter()
                        .flat_map(|h| guards.get(h).into_iter().flatten().copied()),
                )
                .collect();
            let found: Vec<&FlakeSite> = sites
                .get(case.file.as_str())
                .into_iter()
                .flatten()
                .copied()
                .filter(|s| in_scope(s.span))
                .chain(called)
                .chain(&undecided)
                .filter(|s| !guards.iter().any(|g| covers(g, s)))
                .collect();
            if found.is_empty() {
                continue;
            }
            let mut finding = self.finding(case, &found, severity);
            if let Some(gate) = off_by_default_gate(case, file_cases) {
                finding.outcome = Outcome::Listed;
                finding.severity = Severity::Info;
                finding.evidence.push(Evidence::new("gate", gate.clone()));
                finding.message = format!("{}, parked behind {gate}", finding.message);
            }
            findings.push(finding);
        }
        sort(&mut findings);
        number_collisions(&mut findings);
        findings
    }
}

/// A test file's setup files: those of every Vitest/Jest config that governs it.
type Setups<'m> = HashMap<&'m str, BTreeSet<&'m str>>;

/// The files whose code runs for each test file's tests, and what the rule needs to follow
/// a test into them.
struct Reached<'m> {
    /// For each test file, its setup files and the test-only modules it or they import.
    files: HashMap<&'m str, Vec<&'m str>>,
    /// For each test file, its setup files.
    setups: Setups<'m>,
    /// Each module's imports, resolved.
    imports: HashMap<&'m str, Vec<&'m str>>,
    /// Each module's imports that run code, resolved: all but `import type`.
    runtime: HashMap<&'m str, Vec<&'m str>>,
    /// Test and setup files.
    runner: BTreeSet<&'m str>,
}

/// For each test file, the other files whose code runs for its tests, sorted: the setup
/// files of every Vitest/Jest config that governs it, and the test-only modules it or those
/// setup files import — modules that only test and setup files import (PRD 0004 story 10),
/// and that no package publishes as its entry. A module any other file imports is production
/// code the test merely exercises, and its mechanisms are not the test's; so is the module
/// any test file is unambiguously paired with in its subject's place, though only tests
/// import it, unless its path explicitly places it in test infrastructure. The
/// setup files are also returned on their own, with every module's resolved imports and the
/// files that are not production, for the rule to follow a test into a test-only module.
fn reached_files(model: &Model) -> Reached<'_> {
    let test_files: BTreeSet<&str> = model.test_cases.iter().map(|c| c.file.as_str()).collect();
    let by_path: HashMap<&str, &Module> =
        model.modules.iter().map(|m| (m.path.as_str(), m)).collect();
    let setups = setup_files(model, &test_files, &by_path);
    let test_side: BTreeSet<&str> = test_files
        .iter()
        .copied()
        .chain(setups.values().flatten().copied())
        .collect();
    let imports = resolved_imports(model, &by_path, |_| true);
    let runtime = resolved_imports(model, &by_path, |kind| kind != ImportKind::Type);
    let mut importers: HashMap<&str, BTreeSet<&str>> = HashMap::new();
    for (importer, targets) in &imports {
        for target in targets {
            importers.entry(target).or_default().insert(importer);
        }
    }
    // A module every importer of which is a test or setup file, and that no package
    // publishes as its entry: what its consumers import is production, whoever imports it
    // here.
    let entries: BTreeSet<&str> = model.package_entries.iter().map(String::as_str).collect();
    let subjects: BTreeSet<&str> = model
        .pairings
        .iter()
        .filter(|pair| !pair.ambiguous && subject_place(&pair.test_file, &pair.source_module))
        .filter(|pair| {
            !pair.source_module.split('/').any(|part| {
                matches!(
                    part,
                    "test-utils" | "test-helpers" | "test" | "tests" | "__tests__" | "__fixtures__"
                )
            })
        })
        .map(|pair| pair.source_module.as_str())
        .collect();
    let test_only: BTreeSet<&str> = importers
        .iter()
        .filter(|(module, by)| {
            !test_side.contains(*module)
                && !entries.contains(*module)
                && !subjects.contains(*module)
                && by.iter().all(|i| test_side.contains(i))
        })
        .map(|(module, _)| *module)
        .collect();
    let under_test: HashMap<&str, &str> = model
        .pairings
        .iter()
        .filter(|p| subject_place(&p.test_file, &p.source_module))
        .map(|p| (p.test_file.as_str(), p.source_module.as_str()))
        .collect();
    let reached = test_files
        .iter()
        .map(|file| {
            let mut reached: BTreeSet<&str> =
                setups.get(file).into_iter().flatten().copied().collect();
            let roots: Vec<&str> = reached.iter().copied().chain([*file]).collect();
            for root in roots {
                reached.extend(
                    imports
                        .get(root)
                        .into_iter()
                        .flatten()
                        .copied()
                        .filter(|target| test_only.contains(target))
                        .filter(|target| under_test.get(file) != Some(target)),
                );
            }
            (*file, reached.into_iter().collect())
        })
        .collect();
    Reached {
        files: reached,
        setups,
        imports,
        runtime,
        runner: test_side,
    }
}

/// Whether a test reaches a mechanism of a module whose code runs for it.
enum Reach {
    /// It does: the mechanism is the test's.
    Yes,
    /// The reader cannot tell: the mechanism is the test's, undecided.
    Maybe,
    /// The test's code never calls into it.
    No,
}

/// One test file's name uses placed against its tests: every use with its offset, in
/// offset order, and the uses outside every test, step and benchmark, grouped by the
/// innermost `describe` holding them (`None` at module scope).
struct Scopes<'m> {
    uses: Vec<(u32, &'m str)>,
    outside: Vec<(Option<Span>, BTreeSet<&'m str>)>,
}

impl<'m> Scopes<'m> {
    /// The uses of `names`, the rows of a file whose cases are `file_cases`.
    fn of(names: Option<&'m FlakeNames>, file_cases: &[&TestCase]) -> Self {
        let mut uses: Vec<(u32, &str)> = names
            .into_iter()
            .flat_map(|n| n.uses.iter().map(|(offset, name)| (*offset, name.as_str())))
            .collect();
        uses.sort_unstable();
        let mut outside: Vec<(Option<Span>, BTreeSet<&str>)> = Vec::new();
        for (offset, name) in &uses {
            let at = Span::new(*offset, *offset);
            let in_a_test = file_cases.iter().any(|c| {
                matches!(
                    c.kind,
                    TestKind::Test | TestKind::Step | TestKind::Benchmark
                ) && contains(c.span, at)
            });
            if in_a_test {
                continue;
            }
            let suite = file_cases
                .iter()
                .filter(|c| c.kind == TestKind::Suite && contains(c.span, at))
                .max_by_key(|c| c.span.start)
                .map(|c| c.span);
            match outside.iter_mut().find(|(s, _)| *s == suite) {
                Some((_, group)) => {
                    group.insert(name);
                }
                None => outside.push((suite, BTreeSet::from([*name]))),
            }
        }
        Self { uses, outside }
    }

    /// The names `case`'s code uses: in its body, and around it — at module scope and in the
    /// body of every `describe` holding it, hooks included, which is the code that runs
    /// for it.
    fn of_case(&self, case: &TestCase) -> BTreeSet<&'m str> {
        let from = self
            .uses
            .partition_point(|(offset, _)| *offset < case.span.start);
        let mut names: BTreeSet<&str> = self.uses[from..]
            .iter()
            .take_while(|(offset, _)| *offset < case.span.end)
            .map(|(_, name)| *name)
            .collect();
        for (suite, used) in &self.outside {
            if suite.is_none_or(|suite| contains(suite, case.span)) {
                names.extend(used);
            }
        }
        names
    }
}

/// A test-only module's intra-module call graph (each named code to the names it
/// references), with the names one test file's tests reach in it whatever their own code:
/// for sure, from the names the module's top level references, which run on import, and the
/// names that call into it from the file's setup files and production imports; maybe, from
/// the names its anonymous code references.
struct Graph<'m> {
    by_name: HashMap<&'m str, &'m BTreeSet<String>>,
    sure: BTreeSet<&'m str>,
    maybe: BTreeSet<&'m str>,
}

impl<'m> Graph<'m> {
    /// The graph of `names`, reached for sure from `seeds` and its top level.
    fn of(names: &'m FlakeNames, seeds: impl IntoIterator<Item = &'m str>) -> Self {
        let by_name = names
            .calls
            .iter()
            .filter_map(|(enclosing, names)| match enclosing {
                Enclosing::Named(name) => Some((name.as_str(), names)),
                Enclosing::Module | Enclosing::Anonymous => None,
            })
            .collect();
        let of = |enclosing: &Enclosing| {
            names
                .calls
                .get(enclosing)
                .into_iter()
                .flatten()
                .map(String::as_str)
        };
        let mut graph = Self {
            by_name,
            sure: BTreeSet::new(),
            maybe: BTreeSet::new(),
        };
        graph.sure = graph.walk(seeds.into_iter().chain(of(&Enclosing::Module)));
        graph.maybe = graph.walk(of(&Enclosing::Anonymous));
        graph
    }

    /// Every name reached from `seeds`, the seeds included.
    fn walk(&self, seeds: impl IntoIterator<Item = &'m str>) -> BTreeSet<&'m str> {
        let mut seen = BTreeSet::new();
        let mut stack: Vec<&str> = seeds.into_iter().collect();
        while let Some(name) = stack.pop() {
            if seen.insert(name) {
                stack.extend(
                    self.by_name
                        .get(name)
                        .into_iter()
                        .copied()
                        .flatten()
                        .map(String::as_str),
                );
            }
        }
        seen
    }
}

/// Following one test into the modules whose code runs for it.
struct Follow<'r, 'm> {
    case: &'m TestCase,
    reach: &'r Reached<'m>,
    names: &'r HashMap<&'m str, &'m FlakeNames>,
    file_cases: &'r [&'m TestCase],
    scopes: &'r mut HashMap<&'m str, Scopes<'m>>,
    /// For each test file and test-only module, the module's graph and what the file
    /// reaches in it.
    graphs: &'r mut HashMap<(&'m str, &'m str), Graph<'m>>,
    /// The names the test's own code uses, once asked for.
    own: Option<BTreeSet<&'m str>>,
}

impl<'r, 'm> Follow<'r, 'm> {
    /// The mechanisms of `reached`, the files whose code runs for the test, that it reaches:
    /// as they are where it follows the call by name, as undecided copies where it cannot.
    fn sites(
        &mut self,
        reached: &[&'m str],
        sites: &HashMap<&str, Vec<&'m FlakeSite>>,
    ) -> (Vec<&'m FlakeSite>, Vec<FlakeSite>) {
        let mut called = Vec::new();
        let mut undecided = Vec::new();
        for module in reached {
            let mut own = None;
            for site in sites.get(module).into_iter().flatten().copied() {
                match self.reaches(module, site, &mut own) {
                    Reach::Yes => called.push(site),
                    Reach::Maybe => undecided.push(FlakeSite {
                        decided: false,
                        ..site.clone()
                    }),
                    Reach::No => {}
                }
            }
        }
        (called, undecided)
    }

    /// Whether the test reaches `site`, a mechanism of `module` (one of the files whose
    /// code runs for it), with `own` the names of the module the test's own code reaches,
    /// filled on first use. Everything in a setup file runs for every test; in a test-only
    /// module, top-level code runs on import, a named function when the test reaches its
    /// name, and an anonymous one undecided.
    fn reaches(
        &mut self,
        module: &'m str,
        site: &FlakeSite,
        own: &mut Option<BTreeSet<&'m str>>,
    ) -> Reach {
        let file: &'m str = self.case.file.as_str();
        if self
            .reach
            .setups
            .get(file)
            .is_some_and(|setups| setups.contains(module))
        {
            return Reach::Yes;
        }
        let name = match &site.enclosing {
            Enclosing::Module => return Reach::Yes,
            Enclosing::Anonymous => return Reach::Maybe,
            Enclosing::Named(name) => name.as_str(),
        };
        let Some(names) = self.names.get(module).copied() else {
            return Reach::Yes;
        };
        if !self.graphs.contains_key(&(file, module)) {
            let seeds = self.callers(module);
            self.graphs.insert((file, module), Graph::of(names, seeds));
        }
        if own.is_none() {
            let used = if self.imports(file).contains(&module) {
                self.own()
            } else {
                BTreeSet::new()
            };
            *own = Some(self.graphs[&(file, module)].walk(used));
        }
        let graph = &self.graphs[&(file, module)];
        let own = own.as_ref().map_or(&BTreeSet::new(), |own| own).clone();
        let dynamic = FlakeNames::DYNAMIC;
        if graph.sure.contains(name) || own.contains(name) {
            Reach::Yes
        } else if graph.maybe.contains(name)
            || graph.sure.contains(dynamic)
            || graph.maybe.contains(dynamic)
            || own.contains(dynamic)
        {
            // Code the test reaches may call anything in the module.
            Reach::Maybe
        } else {
            Reach::No
        }
    }

    /// The resolved imports of `module`.
    fn imports(&self, module: &str) -> &'r [&'m str] {
        self.reach
            .imports
            .get(module)
            .map_or(&[][..], Vec::as_slice)
    }

    /// The names that call into `module` for every test of the test's file: every name of
    /// each of its setup files that imports the module, and every name of the other modules
    /// the file imports, directly or through one another — production code and test-only
    /// modules alike, short of test and setup files, of `import type`, and of `module`
    /// itself, whose own imports are code it calls, not code that calls it — which may call
    /// back into what a test hands them (a mock provider's `search`, through a harness's
    /// `runScenario`).
    fn callers(&self, module: &str) -> Vec<&'m str> {
        let file: &'m str = self.case.file.as_str();
        let runtime = |of: &str| self.reach.runtime.get(of).map_or(&[][..], Vec::as_slice);
        let mut seeds: Vec<&'m str> = Vec::new();
        for setup in self.reach.setups.get(file).into_iter().flatten() {
            if self.imports(setup).contains(&module) {
                seeds.extend(names_of(self.names, setup));
            }
        }
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        let mut stack: Vec<&str> = runtime(file).to_vec();
        while let Some(import) = stack.pop() {
            if import != module && !self.reach.runner.contains(import) && seen.insert(import) {
                stack.extend(runtime(import));
            }
        }
        for import in seen {
            seeds.extend(names_of(self.names, import));
        }
        seeds
    }

    /// The names the test's own code uses (see [`Scopes::of_case`]).
    fn own(&mut self) -> BTreeSet<&'m str> {
        if let Some(own) = &self.own {
            return own.clone();
        }
        let file: &'m str = self.case.file.as_str();
        let names = self.names.get(file).copied();
        let file_cases = self.file_cases;
        let own = self
            .scopes
            .entry(file)
            .or_insert_with(|| Scopes::of(names, file_cases))
            .of_case(self.case);
        self.own = Some(own.clone());
        own
    }
}

/// Every name `file`'s code references.
fn names_of<'m>(
    names: &HashMap<&'m str, &'m FlakeNames>,
    file: &str,
) -> impl Iterator<Item = &'m str> {
    names
        .get(file)
        .copied()
        .into_iter()
        .flat_map(|n| n.calls.values().flatten().map(String::as_str))
}

/// Each module's imports of a kind `keep` accepts, resolved: by the resolver, or by the
/// relative specifier as written.
fn resolved_imports<'m>(
    model: &'m Model,
    by_path: &HashMap<&str, &'m Module>,
    keep: impl Fn(ImportKind) -> bool,
) -> HashMap<&'m str, Vec<&'m str>> {
    model
        .modules
        .iter()
        .map(|m| {
            let resolved = m
                .imports
                .iter()
                .filter(|i| keep(i.kind))
                .filter_map(|i| {
                    i.resolved
                        .as_deref()
                        .or_else(|| relative_target(&m.path, &i.specifier, by_path))
                })
                .collect();
            (m.path.as_str(), resolved)
        })
        .collect()
}

/// Whether `source` sits where a spec's subject does: next to `test`, or one directory
/// above the `__tests__` directory holding it. The module a spec is paired with there is the
/// module it exercises — production code, whatever else imports it (PRD 0004 story 11) — and
/// not a helper of the spec's; one paired elsewhere (a spec whose only import is a helper
/// under `test-utils/`) may be either, and stays a helper when only tests import it.
fn subject_place(test: &str, source: &str) -> bool {
    let test_dir = test.rsplit_once('/').map_or("", |(dir, _)| dir);
    let source_dir = source.rsplit_once('/').map_or("", |(dir, _)| dir);
    source_dir == test_dir
        || (test_dir == "__tests__" && source_dir.is_empty())
        || test_dir
            .strip_suffix("/__tests__")
            .is_some_and(|parent| parent == source_dir)
}

/// Whether `guard` neutralises `site`: it covers the site's source, and, when it puts back
/// one global, that global is the one the site stubs.
fn covers(guard: &FlakeGuard, site: &FlakeSite) -> bool {
    guard.covers.contains(&site.source)
        && guard
            .target
            .as_ref()
            .is_none_or(|global| site.target.as_ref() == Some(global))
}

/// The extensions a relative specifier is tried with, in the runner's order.
const EXTENSIONS: [&str; 6] = [".ts", ".tsx", ".mts", ".js", ".jsx", ".mjs"];

/// The module a relative specifier names as written (`./wait`, `../utils/index.js`), tried
/// with the extensions and `index` files the runner would, when the resolver left it
/// unresolved. A tsconfig that extends an uninstalled preset fails every relative import on
/// a bare worktree; the specifier as written does not depend on `node_modules`.
fn relative_target<'m>(
    importer: &str,
    specifier: &str,
    by_path: &HashMap<&str, &'m Module>,
) -> Option<&'m str> {
    if !specifier.starts_with("./") && !specifier.starts_with("../") {
        return None;
    }
    let joined = normalise_join(&parent_dir(importer), specifier);
    let stem = [".js", ".jsx", ".mjs", ".cjs"]
        .iter()
        .find_map(|ext| joined.strip_suffix(ext))
        .unwrap_or(&joined);
    std::iter::once(joined.clone())
        .chain(EXTENSIONS.iter().map(|ext| format!("{stem}{ext}")))
        .chain(EXTENSIONS.iter().map(|ext| format!("{joined}/index{ext}")))
        .find_map(|path| by_path.get(path.as_str()).map(|m| m.path.as_str()))
}

/// For each test file, the setup files of the Vitest/Jest configs that govern it: those in
/// its package, at or below the config's directory — the scope `unused-mock` uses.
fn setup_files<'m>(
    model: &'m Model,
    test_files: &BTreeSet<&'m str>,
    by_path: &HashMap<&str, &'m Module>,
) -> HashMap<&'m str, BTreeSet<&'m str>> {
    let mut setups: HashMap<&str, BTreeSet<&str>> = HashMap::new();
    for gate in &model.gates {
        if !matches!(gate.tool, GateTool::Vitest | GateTool::Jest) {
            continue;
        }
        let gate_dir = parent_dir(&gate.source);
        let resolved: Vec<&str> = gate
            .setup_files
            .iter()
            .filter_map(|spelling| resolve_setup(&gate_dir, spelling, by_path))
            .filter_map(|path| by_path.get(path.as_str()).map(|m| m.path.as_str()))
            .collect();
        if resolved.is_empty() {
            continue;
        }
        for file in test_files.iter().filter(|f| under(&gate_dir, f)) {
            setups
                .entry(file)
                .or_default()
                .extend(resolved.iter().copied());
        }
    }
    setups
}

/// `rows` grouped by the file `file_of` names, so a test reads only its own file's rows.
fn by_file<T>(rows: &[T], file_of: impl Fn(&T) -> &String) -> HashMap<&str, Vec<&T>> {
    let mut map: HashMap<&str, Vec<&T>> = HashMap::new();
    for row in rows {
        map.entry(file_of(row).as_str()).or_default().push(row);
    }
    map
}

impl FlakeCandidate {
    /// The finding for one test: decided when any of its clock reads was, could-not-decide
    /// otherwise.
    fn finding(&self, case: &TestCase, sites: &[&FlakeSite], severity: Severity) -> Finding {
        let kind = FlakeKind::WallClock;
        let decided = sites.iter().any(|s| s.decided);
        let calendar = sites.iter().all(|s| s.calendar);
        let title = case.title.clone().unwrap_or_else(|| "<dynamic>".to_owned());
        let first = sites[0];
        let mechanisms: Vec<&str> = sites.iter().map(|s| s.what.as_str()).collect();
        Finding {
            rule: self.name(),
            severity,
            outcome: if decided {
                Outcome::Finding
            } else {
                Outcome::CouldNotDecide
            },
            location: Location::at(case.file.clone(), case.span),
            related: sites
                .iter()
                .map(|s| crate::RelatedLocation {
                    label: kind.name().to_owned(),
                    location: Location::at(s.file.clone(), s.span),
                })
                .collect(),
            message: if calendar {
                format!(
                    "test {title:?} re-reads a calendar value from the clock in its \
                     expectation ({}), so a run that crosses a date boundary fails it",
                    first.what
                )
            } else {
                format!(
                    "test {title:?} asserts an elapsed-time budget ({}), so a slower or \
                     busier machine can fail it",
                    first.what
                )
            },
            evidence: vec![
                Evidence::new("kind", kind.name()),
                Evidence::new("mechanism", mechanisms.join("; ")),
            ],
            explain: explain(&title, calendar),
            anchor: format!("{} [{}]", anchor_of(case), kind.name()),
            test: Some(TestPath::of(case)),
            ordinal: 0,
            proposal: None,
        }
    }
}

/// How to repair a test that asserts an elapsed-time budget, or re-reads a calendar value
/// from the clock (`calendar`).
fn explain(title: &str, calendar: bool) -> String {
    if calendar {
        format!(
            "The test {title:?} expects a calendar value (a year, a date) it reads from the \
             clock itself, beside the code under test's own read, so a run that crosses \
             midnight or the new year between the two fails it. Set the system time \
             (`vi.setSystemTime(…)`) or install fake timers, or assert on a date the test \
             passes in."
        )
    } else {
        format!(
            "The test {title:?} measures the time between two clock reads and asserts a \
             bound on it, so a slower or busier machine fails it. Install fake timers \
             (`vi.useFakeTimers()`) or set the system time, or assert the behaviour without \
             timing it."
        )
    }
}

/// The skip gate that keeps `case` from running by default, spelled `skipIf(<condition>)`,
/// when it or an enclosing `describe` carries one: a gate on an environment variable (the
/// gate `dead-skip` calls CI-scoped) that is closed while the variable is unset —
/// `skipIf(!process.env.PERF)`, `runIf(process.env.PERF)`. A gate that skips only where a
/// variable is set (`skipIf(process.env.CI)`) runs by default and is not one.
fn off_by_default_gate(case: &TestCase, file_cases: &[&TestCase]) -> Option<String> {
    std::iter::once(case)
        .chain(
            file_cases
                .iter()
                .copied()
                .filter(|c| c.kind == TestKind::Suite && contains(c.span, case.span)),
        )
        .filter_map(|c| c.skip.as_ref())
        .find_map(|skip| {
            let condition = skip.condition.as_deref()?;
            env_variable(condition)?;
            let closed_when_unset = match skip.kind {
                SkipKind::Skip | SkipKind::SkipIf | SkipKind::Fixme => negated(condition),
                SkipKind::RunIf => !negated(condition),
                _ => return None,
            };
            closed_when_unset.then(|| format!("{}({condition})", modifier_word(skip.kind)))
        })
}

/// Whether an environment condition holds while the variable is unset: a single negation
/// (`!process.env.X`, not `!!process.env.X`), an inequality, or a test for `undefined`.
fn negated(condition: &str) -> bool {
    let condition = condition.trim();
    (condition.starts_with('!') && !condition.starts_with("!!"))
        || condition.contains("!=")
        || condition.contains("undefined")
}

/// Whether `outer` contains `inner`, byte range for byte range.
const fn contains(outer: Span, inner: Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

/// The byte ranges of `case`'s own file whose code runs for it: its body, and every hook
/// whose enclosing suites all enclose the test (a file-level hook has none, so it reaches
/// every test of the file).
fn scope_of(case: &TestCase, file_cases: &[&TestCase]) -> Vec<Span> {
    let mut ranges = vec![case.span];
    ranges.extend(
        file_cases
            .iter()
            .filter(|c| c.kind == TestKind::Hook && encloses(case, file_cases, c.span))
            .map(|c| c.span),
    );
    ranges
}

/// Whether code at `span` of `case`'s file, outside every test, step and benchmark, runs
/// around `case`: every `describe` that encloses the span also encloses the test. True for
/// module scope, and for a `describe` body the test sits in.
fn encloses(case: &TestCase, file_cases: &[&TestCase], span: Span) -> bool {
    file_cases.iter().all(|c| match c.kind {
        TestKind::Suite => !contains(c.span, span) || contains(c.span, case.span),
        TestKind::Hook => true,
        TestKind::Test | TestKind::Step | TestKind::Benchmark => !contains(c.span, span),
    })
}

/// The semantic anchor for a test: its suite path joined with its title.
fn anchor_of(case: &TestCase) -> String {
    let mut parts = case.suite_path.clone();
    parts.push(
        case.title
            .clone()
            .unwrap_or_else(|| format!("<dynamic>:{}", case.span.line)),
    );
    parts.join(" › ")
}
