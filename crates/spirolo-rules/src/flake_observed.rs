//! `flake-observed`: a test that both passed and failed on one tree — the ledger half of the
//! `flake` family (`docs/categories.md`: "flake = pass and fail on the same tree hash").
//!
//! The rule reads only the model's `Run` table, which the binary fills from the normalized
//! run files under `.spirolo/runs/`, never from raw reports. Rows are grouped by file, title
//! path and tree hash. Passes and failures must occur in the same runner home: two
//! configurations that each produce a consistent, different result do not establish a flake.
//! Unlabelled legacy runs share one context. A group with a pass or a pass on retry,
//! and a fail or a pass on retry, is one finding: a pass on retry is both, so a runner's
//! retry does not hide a flake. A test
//! that failed in every run is a deterministic red, not a flake, and a skipped row says
//! nothing either way.
//!
//! A file-level row (empty title path) records a file that failed outside any test in one
//! run: a hook threw, or the module did not load. It is not a finding of its own. Each test
//! other runs in the same runner home on the same tree name in that file, and that run
//! does not, counts one fail from that run, so a file that loads in one run and not in
//! another names its tests.
//!
//! The finding is anchored at the test case the rows joined to, or at the file when they
//! joined none (the report names a test the model cannot see). Its evidence is each side's
//! count, run ids and conditions, and the tree hash. Severity `warn`, so it never fails the
//! build.

use std::collections::{BTreeMap, BTreeSet};

use spirolo_model::{Model, Run, RunOutcome};

use crate::finding::{number_collisions, sort, Evidence, Finding, Location, Outcome, TestPath};
use crate::{Config, Family, Rule, Severity};

/// The `flake-observed` rule. See the module docs for the semantics.
pub struct FlakeObserved;

impl Rule for FlakeObserved {
    fn name(&self) -> &'static str {
        "flake-observed"
    }

    fn family(&self) -> Family {
        Family::Flake
    }

    fn subject(&self) -> crate::Subject {
        crate::Subject::Runs
    }

    fn summary(&self) -> &'static str {
        "A test both passed and failed on the same tree in the run ledger."
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn run(&self, model: &Model, config: &Config) -> Vec<Finding> {
        let severity = config.severity(self.name(), self.default_severity());
        let mut groups: BTreeMap<(&str, &[String], &str), Vec<&Run>> = BTreeMap::new();
        let mut file_failures: BTreeMap<(&str, &str, Option<&str>), Vec<&Run>> = BTreeMap::new();
        for run in &model.runs {
            let Some(tree_hash) = run.tree_hash.as_deref() else {
                continue;
            };
            if run.title_path.is_empty() {
                if run.outcome == RunOutcome::Failed {
                    file_failures
                        .entry((&run.file, tree_hash, run.runner_home.as_deref()))
                        .or_default()
                        .push(run);
                }
                continue;
            }
            groups
                .entry((&run.file, &run.title_path, tree_hash))
                .or_default()
                .push(run);
        }
        let mut findings: Vec<Finding> = groups
            .into_iter()
            .filter_map(|((file, title_path, tree_hash), mut rows)| {
                // A run in which the test's file failed outside any test, and which has no
                // row of its own for the test, did not let the test pass: it counts as a fail.
                let homes: BTreeSet<_> =
                    rows.iter().map(|row| row.runner_home.as_deref()).collect();
                let unreported: Vec<&Run> = homes
                    .into_iter()
                    .filter_map(|home| file_failures.get(&(file, tree_hash, home)))
                    .flatten()
                    .filter(|failure| rows.iter().all(|row| row.run_id != failure.run_id))
                    .copied()
                    .collect();
                rows.extend(unreported);
                self.flake(title_path, tree_hash, &rows, severity)
            })
            .collect();
        sort(&mut findings);
        number_collisions(&mut findings);
        findings
    }
}

/// One side of a flake: how many rows, from which runs, under which conditions.
#[derive(Default)]
struct Side<'a> {
    count: usize,
    runs: BTreeSet<&'a str>,
    conditions: BTreeSet<&'a str>,
}

impl<'a> Side<'a> {
    fn add(&mut self, run: &'a Run) {
        self.count += 1;
        self.runs.insert(&run.run_id);
        self.conditions
            .insert(run.condition.as_deref().unwrap_or("unlabelled"));
    }
}

impl FlakeObserved {
    /// The finding for one test's rows on one tree, when they hold a pass and a fail.
    fn flake(
        &self,
        title_path: &[String],
        tree_hash: &str,
        rows: &[&Run],
        severity: Severity,
    ) -> Option<Finding> {
        let mut homes: BTreeMap<Option<&str>, (bool, bool)> = BTreeMap::new();
        for run in rows {
            let sides = homes.entry(run.runner_home.as_deref()).or_default();
            sides.0 |= matches!(run.outcome, RunOutcome::Passed | RunOutcome::PassedOnRetry);
            sides.1 |= matches!(run.outcome, RunOutcome::Failed | RunOutcome::PassedOnRetry);
        }
        let comparable: Vec<_> = rows
            .iter()
            .copied()
            .filter(|run| homes[&run.runner_home.as_deref()] == (true, true))
            .collect();
        let first = *comparable.first()?;
        let mut passed = Side::default();
        let mut failed = Side::default();
        for run in comparable {
            if matches!(run.outcome, RunOutcome::Passed | RunOutcome::PassedOnRetry) {
                passed.add(run);
            }
            if matches!(run.outcome, RunOutcome::Failed | RunOutcome::PassedOnRetry) {
                failed.add(run);
            }
        }
        if passed.count == 0 || failed.count == 0 {
            return None;
        }
        let location = first.joined.as_ref().map_or_else(
            || Location {
                file: first.file.clone(),
                line: 1,
                column: 1,
            },
            |joined| Location::at(joined.file.clone(), joined.span),
        );
        let (title, suite_path) = title_path.split_last()?;
        let name = title_path.join(" › ");
        let join = |set: &BTreeSet<&str>| set.iter().copied().collect::<Vec<_>>().join(", ");
        Some(Finding {
            rule: self.name(),
            severity,
            outcome: Outcome::Finding,
            location,
            related: Vec::new(),
            message: format!(
                "test {title:?} has {} passes and {} fails on tree {tree_hash}",
                passed.count, failed.count
            ),
            evidence: vec![
                Evidence::new("failed", failed.count.to_string()),
                Evidence::new("failed_conditions", join(&failed.conditions)),
                Evidence::new("failed_runs", join(&failed.runs)),
                Evidence::new("passed", passed.count.to_string()),
                Evidence::new("passed_conditions", join(&passed.conditions)),
                Evidence::new("passed_runs", join(&passed.runs)),
                Evidence::new("tree_hash", tree_hash),
            ],
            explain: format!(
                "Test {title:?} both passed and failed on one tree, so its outcome depends on \
                 something other than the code: compare the failing runs' conditions with the \
                 passing ones', find what the test reads that varies between runs (the clock, \
                 a random draw, a fixed wait, shared state, the order it runs in), and make \
                 the test control it."
            ),
            anchor: format!("{name} [{tree_hash}]"),
            test: Some(TestPath {
                suite_path: suite_path.to_vec(),
                title: Some(title.clone()),
            }),
            ordinal: 0,
            proposal: None,
        })
    }
}
