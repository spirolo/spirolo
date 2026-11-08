//! Reading a corpus entry's `truth.json`: the runner's own counts and the wall-clock
//! budget the harness holds `spirolo list` to.
//!
//! Only the fields the harness compares against are modelled; everything else the file
//! carries (metrics, notes, ground-truth findings) is ignored by `serde`, so the shape
//! can grow without touching this crate.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::Deserialize;

/// Whether an entry can be measured on this machine, or only carries copied numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Access {
    /// Cloneable from here; `counts` come from a real run and can be reproduced.
    Local,
    /// Not reachable; `counts` are copied from a document of record, never run.
    Pending,
}

/// One framework's ground-truth counts. A `null` in the file becomes `None` — the source
/// gave no number and the harness never invents one, so it compares only what it was told.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct FrameworkCounts {
    /// Number of test files the runner reported, or `None` when the source gave none.
    pub files: Option<u64>,
    /// Number of test cases the runner reported, or `None` when the source gave none.
    pub tests: Option<u64>,
}

/// A package the runner never listed, so its tests are absent from the truth totals.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ExcludedPackage {
    /// Repo-relative directory of the package.
    pub path: String,
}

/// A test file a [`RunnerCounts`] listing counted but whose tests are generated at load
/// time, left out of both sides of the runner row.
///
/// Its `listed_tests` come off the runner's side and its rows off spirolo's, so the row
/// compares the same set. The count is the runner's own at the pin of the listing, which
/// may differ from the `counts` record's [`ExcludedFile::listed_tests`] when the two were
/// measured at different pins.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RunnerExcludedFile {
    /// Repo-relative path of the file.
    pub path: String,
    /// How many tests the runner listed in it, subtracted from the runner's side.
    pub listed_tests: u64,
}

/// One framework's listing by the runner itself, taken at the pin on an installed clone.
///
/// This is the M1 done-condition's "runner's own counts" (`vitest list --json`,
/// `playwright test --list`), compared against spirolo's tally on the same set within a
/// stated tolerance.
///
/// `counts` is spirolo's self-regression tally (spirolo agrees with spirolo); this record is the
/// independent side. Its `excluded_packages` are the runner's own gaps at this listing
/// (crashers, packages the listing never visited), not the `counts` exclusions, so the
/// runner row tallies spirolo's document with these and only these left out. Fields the
/// harness does not compare (`packages_listed`, `attributed`, `unattributed_tests`,
/// `method`) stay in the file as provenance and are ignored here.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RunnerCounts {
    /// Distinct test files the runner listed, `excluded_files` included (one file each
    /// comes off before the comparison).
    pub files: u64,
    /// Tests the runner listed, `excluded_files`' tests included (their `listed_tests`
    /// come off before the comparison). `null` when the runner lists files only (`jest
    /// --listTests`): the row then compares files alone.
    pub tests: Option<u64>,
    /// Repo-relative package directories the runner never listed at this listing; spirolo's
    /// rows under them are left out of the runner row's tally.
    #[serde(default)]
    pub excluded_packages: Vec<String>,
    /// Files whose tests are generated at load time: `listed_tests` off the runner's
    /// side, the file's rows off spirolo's.
    #[serde(default)]
    pub excluded_files: Vec<RunnerExcludedFile>,
    /// How far spirolo's test count may sit from the runner's and still match: the
    /// attributed static-vs-runtime gap (`.each` tables and loops over bindings spirolo
    /// reports as `test_count_undecidable`), stated in the record's `method`. Defaults to
    /// zero: an exact match.
    #[serde(default)]
    pub tolerance_tests: u64,
    /// How far spirolo's file count may sit from the runner's. Defaults to zero: files must
    /// match exactly.
    #[serde(default)]
    pub tolerance_files: u64,
}

impl RunnerCounts {
    /// The excluded file paths, for the tally to leave out.
    pub fn excluded_file_paths(&self) -> Vec<String> {
        self.excluded_files
            .iter()
            .map(|file| file.path.clone())
            .collect()
    }

    /// The runner's counts net of its `excluded_files`: what spirolo's tally is compared to.
    pub fn compared(&self) -> (u64, Option<u64>) {
        let files = u64::try_from(self.excluded_files.len()).unwrap_or(u64::MAX);
        let tests = self
            .excluded_files
            .iter()
            .fold(0_u64, |sum, file| sum.saturating_add(file.listed_tests));
        (
            self.files.saturating_sub(files),
            self.tests.map(|listed| listed.saturating_sub(tests)),
        )
    }
}

/// A test file the runner listed whose tests are generated at load time, so a static
/// model cannot count them: left out of both sides and reported as an open line.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ExcludedFile {
    /// Repo-relative path of the file.
    pub path: String,
    /// The `counts` framework its tests are subtracted from.
    pub framework: String,
    /// How many tests the runner listed in it at the pinned commit — the amount
    /// subtracted from the expected side, recorded so the subtraction is auditable.
    pub listed_tests: u64,
    /// Why it cannot be counted statically.
    pub reason: String,
}

/// One adjudicator's verdict on a reported finding: the precision side of the harness.
///
/// `true` and `false` are the two decided outcomes; `open` is what an adjudicator that
/// could not decide records — never `true`, so the precision figure stays conservative.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    /// The finding is real: applying the controlled change proved the disagreement.
    True,
    /// The finding is a false positive: the change did not prove any disagreement.
    False,
    /// The adjudicator could not decide; counted apart, never as `true`.
    Open,
}

/// A labelled sample of what spirolo reported: one row per adjudicated finding.
///
/// Joined to a reported finding by its `id`. This is the precision side of `truth.json`,
/// written by the corpus adjudicators (slice #35+); the harness only reads it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Adjudication {
    /// The rule the finding is for, recorded for readability (the join is on `id`).
    pub rule: String,
    /// The reported finding's identity, joined against `check`'s `findings[].id`.
    pub id: String,
    /// The adjudicator's verdict.
    pub verdict: Verdict,
    /// What disagrees with what, and the evidence the adjudicator produced.
    #[serde(default)]
    pub note: String,
    /// The substrate the adjudicator ran on (`installed <lockfile> git:<blob>` or `bare`),
    /// stamped by the adjudicator. The join refuses a row whose substrate is not the
    /// entry's [`Truth::substrate`].
    #[serde(default)]
    pub substrate: Option<String>,
}

/// A rule for which history holds no shipped/fixed replay pair, recorded so the entry's
/// per-rule coverage is explicit rather than a silent gap.
///
/// The mining slice (#46) records one [`Finding`] replay pair per measurable rule where
/// history has one, and one of these `none` markers where it does not — the acceptance
/// criterion is "one pair per rule where history has one, else recorded as none". It is
/// deliberately not a [`Finding`]: a `Finding` with no `fixed_commit` is read as a
/// present-at-pin recall positive, which a `none` marker is not.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ReplayNone {
    /// The rule that has no replay pair in history.
    pub rule: String,
    /// Why no pair was found, and the search that looked for one, so the record is
    /// auditable and repeatable at a later pin.
    #[serde(default)]
    pub note: String,
}

/// Whether a recorded finding is expected to be reported at the pin, or expected to stay
/// silent there.
///
/// The default, `present`, is a recall positive: the run must reproduce it. `absent` marks a
/// true negative the entry records so a rule is measured against a site where it must *not*
/// fire (a real Stryker `break` threshold, a preset that satisfies every invariant); it is
/// documentation of the rule's boundary and is left out of recall's denominator.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Expected {
    /// The finding is present at the pin: a recall positive the run must reproduce.
    #[default]
    Present,
    /// A true negative: the rule must stay silent here. Never a recall positive.
    Absent,
}

/// A recorded ground-truth finding: a recall positive, a true negative, or one half of a
/// replay pair.
///
/// A positive present at the pin carries no commits; a replay pair carries a shipped and a
/// fixed commit; a true negative carries `expected: absent`. Only the fields the precision
/// section reads are modelled; the rest of the schema is ignored by `serde`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Finding {
    /// The spirolo rule this row is ground truth for.
    pub rule: String,
    /// A semantic anchor for the finding, used to label its replay line.
    #[serde(default)]
    pub anchor: Option<String>,
    /// The repo-relative file the finding lives in, used to scope a replay pair's
    /// present-check so a broadly-firing rule is not read as "present" at the fixed commit on
    /// an unrelated finding elsewhere in the tree. `None` scopes to the whole tree, the
    /// pre-existing behaviour for a rule that fires on one site.
    #[serde(default)]
    pub file: Option<String>,
    /// The invariant slugs a replay pair proves, for a rule that reports several findings in
    /// one file — `fail-closed-setup` names one finding per missing invariant (`console-guard`,
    /// `unhandled-request-error`, …), so a pair that proves two of them must not read as
    /// "present" at the fixed commit on a third, unrelated invariant that legally still fires
    /// there. Empty scopes to any finding in the file, the behaviour for a rule whose pair
    /// fires on one finding.
    #[serde(default)]
    pub invariants: Vec<String>,
    /// The commit the bug shipped at — the finding must be present here. `None` for a
    /// present-at-pin positive.
    #[serde(default)]
    pub shipped_commit: Option<String>,
    /// The commit the fix landed at — the finding must be absent here. `None` for a
    /// present-at-pin positive.
    #[serde(default)]
    pub fixed_commit: Option<String>,
    /// The commit a recall positive is measured at, when it is not the pin. A positive that
    /// names a `present_commit` (the recorded incident ref for `redundant-test`) is reproduced
    /// by the run's findings at *that* checkout, not at the pin: the harness stages the
    /// commit's `inputs/` and re-runs `check` there. `None` is the default — a present-at-pin
    /// positive. A positive with a `present_commit` carries no `fixed_commit` (it is not a
    /// replay pair) and is left out of the pin recall it would otherwise fail.
    #[serde(default)]
    pub present_commit: Option<String>,
    /// Whether the run is expected to report this finding at the pin (`present`, the
    /// default and a recall positive) or to stay silent on it (`absent`, a true negative
    /// excluded from recall).
    #[serde(default)]
    pub expected: Expected,
}

/// A replay pair the harness proves by checking the finding out at each commit.
///
/// Derived from a [`Finding`] that carries both a shipped and a fixed commit; the finding
/// must be present at the first and absent at the second.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayPair {
    /// The rule the pair proves.
    pub rule: String,
    /// A short anchor for the pair, from the finding.
    pub anchor: String,
    /// The file the finding lives in, scoping the present-check; empty scopes to the whole
    /// tree (a rule that fires on a single site needs no scope).
    pub file: String,
    /// The invariant slugs this pair proves, narrowing the present-check within the file to
    /// findings whose identity names one of them. Empty scopes to any finding of the rule in
    /// the file. See [`Finding::invariants`].
    pub invariants: Vec<String>,
    /// The commit the bug shipped at: the finding must be present here.
    pub shipped_commit: String,
    /// The commit the fix landed at: the finding must be absent here.
    pub fixed_commit: String,
}

/// An incident recorded as a replayable commit.
///
/// A stranded branch made concrete: a ref built from the entry's pin plus the branch's
/// title-unique tests as sibling `*.stranded.test.ts` files, so the harness can replay the
/// incident rather than describe it. The harness checks the incident commit out and
/// measures the entry's recall positives that name a `present_commit` there, not at the pin.
#[derive(Debug, Clone, Deserialize)]
pub struct Incident {
    /// The ref the incident was built as, on the entry's origin.
    #[serde(rename = "ref")]
    pub reference: String,
    /// The incident commit's sha on the entry's origin: one commit on top of `base`.
    pub sha: String,
    /// The M2 pin the incident is built on.
    pub base: String,
    /// The kept and dropped test counts across the three stranded files.
    #[serde(default)]
    pub counts: IncidentCounts,
    /// One record per stranded file: its sibling on the pin and the tests kept in it.
    #[serde(default)]
    pub files: Vec<IncidentFile>,
    /// The three near-duplicate pairs PR #1480 quoted (a stranded test and its `main` twin).
    #[serde(default)]
    pub pairs: Vec<IncidentPair>,
    /// The kept tests PR #1480 did not individually pair to a `main` test.
    #[serde(default)]
    pub unpaired: Vec<IncidentTest>,
}

/// The incident's kept/dropped tallies: kept are the title-unique stranded tests, dropped
/// the shared-ancestry ones that already exist on the pin under the same title.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct IncidentCounts {
    /// Title-unique stranded tests kept in the `*.stranded.test.ts` files.
    pub kept: usize,
    /// Shared-ancestry stranded tests dropped (present on the pin under the same title).
    pub dropped: usize,
}

/// One stranded file: its path, the sibling file on the pin it was compared against, the
/// `suite > title` identity of every kept test, and how many tests it dropped.
#[derive(Debug, Clone, Deserialize)]
pub struct IncidentFile {
    /// The committed `*.stranded.test.ts` file, relative to the repo root.
    pub stranded: String,
    /// The sibling test file on the pin whose titles decided kept from dropped.
    pub sibling: String,
    /// The runner's `suite > title` identity of each kept test in this file.
    pub kept_titles: Vec<String>,
    /// How many of this file's stranded tests were dropped as shared ancestry.
    pub dropped_count: usize,
}

/// A near-duplicate pair PR #1480 quoted: a kept stranded test and its `main` twin, each
/// named by its runner `suite > title` identity and file.
#[derive(Debug, Clone, Deserialize)]
pub struct IncidentPair {
    /// The kept stranded test's `suite > title` identity.
    pub stranded_title: String,
    /// The stranded file the test lives in.
    pub stranded_file: String,
    /// The `main` test's `suite > title` identity.
    pub main_title: String,
    /// The `main` file the twin lives in.
    pub main_file: String,
    /// The line the `main` twin sits at in the pinned checkout (grep-verified, slice #46).
    pub main_line: u32,
}

/// A kept stranded test with no individually paired `main` twin.
#[derive(Debug, Clone, Deserialize)]
pub struct IncidentTest {
    /// The kept test's `suite > title` identity.
    pub title: String,
    /// The stranded file it lives in.
    pub file: String,
}

/// The parts of a `truth.json` the harness reads.
#[derive(Debug, Clone, Deserialize)]
pub struct Truth {
    /// Whether this entry is measurable here or copied from a record.
    pub access: Access,
    /// Whether the corpus adjudicators run this entry's tests. An entry that does not
    /// execute its tests (for example one whose suite needs services) adjudicates by
    /// inspection — reading the finding, not proving it against the suite — so its `true`
    /// and `false` verdicts are counted apart from the executed ones and never enter a
    /// rule's precision, its Wilson interval or the 20-adjudicated floor. Defaults to
    /// `true` for a file that predates the field.
    #[serde(default = "default_execute")]
    pub execute: bool,
    /// Repo-relative directories whose tests the entry never executes although the rest of
    /// it runs (for example a monorepo's `apps/api`), the same list the adjudicators honour through
    /// `neverExecutedPaths`. A finding under one is not executable: no executed oracle can
    /// cover it, so it never makes a rule `UNADJUDICATED`.
    #[serde(default)]
    pub never_executed: Vec<String>,
    /// Directories never executed for one rule's oracle only, keyed by rule id (#202): the
    /// rest of the entry, and every other rule's findings in the same directory, stay
    /// executable. When an entry's vitest never collects the Playwright specs in
    /// `packages/e2e-kit`, a `flake-candidate` there cannot be fault-injected, while the
    /// same directory's `dead-skip` findings are still adjudicated. Matched as
    /// [`Self::never_executed`] is.
    #[serde(default)]
    pub never_executed_by_rule: BTreeMap<String, Vec<String>>,
    /// The one substrate counts, precision and adjudication read for this entry: the clone
    /// installed at its committed lockfile (`installed pnpm-lock.yaml git:<blob id>`), or
    /// `bare` (no `node_modules`). Decision 3 of the M3 reopen; an adjudication recorded on
    /// another substrate is refused by the join.
    #[serde(default)]
    pub substrate: Option<String>,
    /// Packages the runner never listed (it crashed on them, or the measurement never
    /// visited them): the harness leaves their rows out so both sides count the same set.
    #[serde(default)]
    pub excluded_packages: Vec<ExcludedPackage>,
    /// Files the runner listed but whose tests are generated at load time: subtracted
    /// from the expected side, left out of the actual side, printed as open lines.
    #[serde(default)]
    pub excluded_files: Vec<ExcludedFile>,
    /// Ground-truth counts per framework, keyed by framework name (`vitest`, `playwright`).
    #[serde(default)]
    pub counts: BTreeMap<String, FrameworkCounts>,
    /// The runner's own listing per framework, keyed the same way: the independent side
    /// the runner row compares spirolo's tally to, within the record's tolerance. Absent for
    /// an entry that has no such listing; the entry then prints no runner row.
    #[serde(default)]
    pub runner: BTreeMap<String, RunnerCounts>,
    /// Wall-clock ceiling for `spirolo list` on this entry, in milliseconds (M1's budget).
    pub budget_ms: u64,
    /// The `spirolo list` wall-clock recorded at the M2 close, in milliseconds. When present,
    /// the M3 performance budget is one added second over it: the five hygiene rules and the
    /// mutation-report reader must not slow the model by more than a second on this entry, so
    /// the effective ceiling is `m2_list_ms + 1000` rather than the flat `budget_ms`.
    #[serde(default)]
    pub m2_list_ms: Option<u64>,
    /// The precision side: a labelled sample of what spirolo reported, joined by finding id.
    #[serde(default)]
    pub adjudications: Vec<Adjudication>,
    /// Recorded ground-truth findings: recall positives and replay pairs.
    #[serde(default)]
    pub findings: Vec<Finding>,
    /// Rules for which history holds no replay pair, recorded explicitly (slice #46) so a
    /// rule's absence of a pair is a decision on the record, not an omission.
    #[serde(default)]
    pub replay_none: Vec<ReplayNone>,
    /// The recorded incident ref, when this entry records one.
    #[serde(default)]
    pub incident: Option<Incident>,
    /// The flake ledger the oracle recomputed from the entry's committed runs (`runs/`), when
    /// the entry has runs (PRD 0004, #181).
    #[serde(default)]
    pub ledger: Option<crate::LedgerTruth>,
    /// Rules whose precision sample is stratified rather than strided, keyed by rule id
    /// (#203): `flake-candidate` samples a quota from each of its kinds.
    #[serde(default)]
    pub stratified_samples: BTreeMap<String, StratifiedSample>,
    /// Adjudications taken out of the precision join, kept on the record with the date and
    /// the reason; the harness reads only which rule's ids they were, so a stratified
    /// sample never draws a retired finding again (#203).
    #[serde(default)]
    pub retired_adjudications: Vec<Retirement>,
}

/// One dated retirement of a rule's adjudications, as far as the harness reads it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Retirement {
    /// The rule whose rows were retired.
    pub rule: String,
    /// The retired rows; only their ids are read.
    #[serde(default)]
    pub rows: Vec<RetiredRow>,
}

/// A retired adjudication row, reduced to the finding id it judged.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RetiredRow {
    /// The finding's identity.
    pub id: String,
}

/// How one rule's precision sample is stratified (#203): by an evidence key, with a seeded
/// draw inside each stratum, so every stratum with findings is sampled.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct StratifiedSample {
    /// The evidence key whose value is a finding's stratum (`kind`).
    pub by: String,
    /// The seed of the draw inside each stratum.
    pub seed: u64,
    /// The sample size the quota divides, and the executable rows the sample reaches.
    pub target: usize,
    /// Every stratum the rule can report, so one with no finding is printed as such.
    pub strata: Vec<String>,
}

/// The `execute` an entry that does not write the field gets: an execution oracle.
const fn default_execute() -> bool {
    true
}

impl Truth {
    /// Whether an executed oracle could cover a finding in `file`: the entry executes its
    /// tests and `file` is under no `never_executed` directory. A directory matches itself
    /// and the paths below it, never a bare string prefix (`apps/api-client` is not under
    /// `apps/api`), as the adjudicators' `neverExecutedPrefix` does.
    #[must_use]
    pub fn executes(&self, file: &str) -> bool {
        self.execute && !under_any(&self.never_executed, file)
    }

    /// Whether an executed oracle could cover `rule`'s finding in `file`: [`Self::executes`],
    /// and `file` is under none of the directories [`Self::never_executed_by_rule`] scopes to
    /// `rule`.
    #[must_use]
    pub fn executes_for(&self, rule: &str, file: &str) -> bool {
        self.executes(file)
            && !self
                .never_executed_by_rule
                .get(rule)
                .is_some_and(|dirs| under_any(dirs, file))
    }

    /// The finding ids whose adjudications for `rule` were retired.
    #[must_use]
    pub fn retired_ids(&self, rule: &str) -> BTreeSet<&str> {
        self.retired_adjudications
            .iter()
            .filter(|retirement| retirement.rule == rule)
            .flat_map(|retirement| retirement.rows.iter().map(|row| row.id.as_str()))
            .collect()
    }

    /// The excluded package paths, for the tally to leave out.
    pub fn excluded_paths(&self) -> Vec<String> {
        self.excluded_packages
            .iter()
            .map(|package| package.path.clone())
            .collect()
    }

    /// The excluded file paths, for the tally to leave out.
    pub fn excluded_file_paths(&self) -> Vec<String> {
        self.excluded_files
            .iter()
            .map(|file| file.path.clone())
            .collect()
    }

    /// Parse a `truth.json` from its text, naming the file in any error.
    pub fn parse(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|error| format!("truth.json is not valid: {error}"))
    }

    /// Read and parse the `truth.json` at `path`.
    pub fn read(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        Self::parse(&text)
    }
}

/// Whether `file` is one of `dirs` or below one: a directory matches itself and its paths,
/// never a bare string prefix (`apps/api-client` is not under `apps/api`).
fn under_any(dirs: &[String], file: &str) -> bool {
    dirs.iter().any(|dir| {
        let dir = dir.trim_end_matches('/');
        file == dir
            || file
                .strip_prefix(dir)
                .is_some_and(|rest| rest.starts_with('/'))
    })
}

#[cfg(test)]
mod tests {
    use super::{Access, Expected, Truth};

    /// A committed `never_executed: ["apps/api"]` covers the directory and
    /// the files below it, not a sibling whose name only starts with it; `execute: false`
    /// covers every file.
    #[test]
    fn a_never_executed_directory_covers_its_own_paths_not_a_sibling_prefix() {
        let truth = Truth::parse(
            r#"{ "access": "local", "budget_ms": 5000, "never_executed": ["apps/api"] }"#,
        )
        .unwrap();
        assert!(!truth.executes("apps/api/src/app.controller.spec.ts"));
        assert!(!truth.executes("apps/api"));
        assert!(truth.executes("apps/api-client/src/a.test.ts"));
        assert!(truth.executes("apps/web/src/a.test.ts"));

        let inspected =
            Truth::parse(r#"{ "access": "local", "budget_ms": 2000, "execute": false }"#).unwrap();
        assert!(!inspected.executes("src/features/admin/a.test.tsx"));
    }

    #[test]
    fn a_null_file_count_reads_as_none_not_zero() {
        let truth = Truth::parse(
            r#"{ "access": "pending", "budget_ms": 2000,
                "counts": { "playwright": { "files": null, "tests": 12 } } }"#,
        )
        .unwrap();
        assert_eq!(truth.access, Access::Pending);
        let playwright = &truth.counts["playwright"];
        assert_eq!(playwright.files, None);
        assert_eq!(playwright.tests, Some(12));
    }

    #[test]
    fn fields_the_harness_ignores_do_not_break_parsing() {
        let truth = Truth::parse(
            r#"{ "access": "local", "budget_ms": 5000,
                "measured_by": "a real run", "metrics": { "skips": 7 },
                "findings": [ { "rule": "x", "verified": true } ],
                "counts": { "vitest": { "files": 2, "tests": 3, "expects": 9 } } }"#,
        )
        .unwrap();
        assert_eq!(truth.budget_ms, 5000);
        assert_eq!(truth.counts["vitest"].tests, Some(3));
        assert!(truth.excluded_paths().is_empty());
    }

    #[test]
    fn the_recorded_m2_timing_is_read_when_present_and_none_otherwise() {
        let with = Truth::parse(r#"{ "access": "local", "budget_ms": 5000, "m2_list_ms": 2300 }"#)
            .unwrap();
        assert_eq!(with.m2_list_ms, Some(2300));
        let without = Truth::parse(r#"{ "access": "local", "budget_ms": 5000 }"#).unwrap();
        assert_eq!(without.m2_list_ms, None);
    }

    #[test]
    fn excluded_packages_are_read_by_path() {
        let truth = Truth::parse(
            r#"{ "access": "local", "budget_ms": 5000,
                "excluded_packages": [ { "path": "apps/dashboard", "reason": "crashed" } ] }"#,
        )
        .unwrap();
        assert_eq!(truth.excluded_paths(), ["apps/dashboard"]);
    }

    /// A `findings` row defaults to `expected: present` (a recall positive); an explicit
    /// `"expected": "absent"` marks a true negative the rule must stay silent on.
    #[test]
    fn a_finding_reads_expected_present_by_default_and_absent_when_written() {
        let truth = Truth::parse(
            r#"{ "access": "local", "budget_ms": 5000,
                "findings": [
                  { "rule": "gate-truth", "file": "apps/api/stryker.config.json",
                    "expected": "absent", "note": "TRUE NEGATIVE" },
                  { "rule": "gate-truth", "file": "apps/web/vitest.config.ts" }
                ] }"#,
        )
        .unwrap();
        assert_eq!(truth.findings[0].expected, Expected::Absent);
        assert_eq!(truth.findings[1].expected, Expected::Present);
        assert_eq!(Expected::default(), Expected::Present);
    }

    /// `execute` reads `false` when written so, and defaults to `true` when absent.
    #[test]
    fn execute_defaults_to_true_and_reads_false_when_written() {
        let inspected =
            Truth::parse(r#"{ "access": "local", "budget_ms": 2000, "execute": false }"#).unwrap();
        assert!(!inspected.execute);
        let unset = Truth::parse(r#"{ "access": "local", "budget_ms": 2000 }"#).unwrap();
        assert!(unset.execute);
    }

    /// A `runner` record reads its tolerances (defaulting to exact) and nets its excluded
    /// files off the compared counts; an entry without one has no runner rows.
    #[test]
    fn a_runner_record_reads_its_tolerance_and_nets_its_excluded_files() {
        let truth = Truth::parse(
            r#"{ "access": "local", "budget_ms": 5000,
                "runner": {
                  "vitest": { "files": 1634, "tests": 18412, "packages_listed": 122,
                              "excluded_packages": ["apps/dashboard"],
                              "tolerance_tests": 135, "method": "vitest list --json" },
                  "playwright": { "files": 22, "tests": 382,
                                  "excluded_files": [ { "path": "e2e/smoke.spec.ts",
                                                        "listed_tests": 109 } ] }
                } }"#,
        )
        .unwrap();
        let vitest = &truth.runner["vitest"];
        assert_eq!(vitest.excluded_packages, ["apps/dashboard"]);
        assert_eq!((vitest.tolerance_files, vitest.tolerance_tests), (0, 135));
        assert_eq!(vitest.compared(), (1634, Some(18412)));
        let playwright = &truth.runner["playwright"];
        assert_eq!(playwright.tolerance_tests, 0);
        assert_eq!(playwright.excluded_file_paths(), ["e2e/smoke.spec.ts"]);
        assert_eq!(playwright.compared(), (21, Some(273)));

        let without = Truth::parse(r#"{ "access": "local", "budget_ms": 5000 }"#).unwrap();
        assert!(without.runner.is_empty());
    }

    #[test]
    fn excluded_files_carry_their_listed_count() {
        let truth = Truth::parse(
            r#"{ "access": "local", "budget_ms": 5000,
                "excluded_files": [ { "path": "e2e/smoke.spec.ts", "framework": "playwright",
                                      "listed_tests": 108, "reason": "generated at load time" } ] }"#,
        )
        .unwrap();
        assert_eq!(truth.excluded_file_paths(), ["e2e/smoke.spec.ts"]);
        assert_eq!(truth.excluded_files[0].listed_tests, 108);
    }
}
