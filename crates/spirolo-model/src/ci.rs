//! Read GitHub Actions and root `CircleCI` jobs as static CI commands.
//!
//! GitHub `run` strings and `CircleCI` `run.command` mappings are retained, including block
//! scalars and anchored job templates. A Cypress action contributes a test command unless
//! `runTests: false` makes it install-only. Named package scripts and Makefile recipes are
//! followed after all manifests are available, past process wrappers (`xvfb-run`, `timeout`)
//! and through workspace fan-outs (`--filter`, `-r`, `turbo`, `lerna`, `nx`); a selector the
//! reader cannot match follows every workspace with the script. Nothing is executed.
//!
//! Invalid YAML or a non-mapping `jobs` value leaves an opaque gate, so rules can report
//! uncertainty instead of claiming an unreadable workflow runs nothing.

mod commands;
mod node;

pub use commands::{command_words, expand_steps};

use yaml_rust2::{Yaml, YamlLoader};

use crate::tables::{Gate, GateTool};

/// The [`Gate`] a workflow file declares: one row with [`GateTool::Ci`], its `steps` the run
/// commands of every job's steps in document order.
///
/// A workflow that does not parse, or whose `jobs` is not a mapping, is an opaque gate with no
/// steps — the tool is known, its steps are not, and nothing was run to find out.
#[must_use]
pub fn read_workflow(path: &str, source: &str) -> Gate {
    let mut gate = blank_ci_gate(path);
    let Ok(docs) = YamlLoader::load_from_str(source) else {
        gate.could_not_read("the workflow is not valid YAML");
        return gate;
    };
    let Some(doc) = docs.first() else {
        // An empty workflow file declares no jobs; it gates nothing, and that is decided, not
        // opaque — there is simply nothing to run.
        return gate;
    };
    let Yaml::Hash(top) = doc else {
        gate.could_not_read("the workflow is not a mapping");
        return gate;
    };
    let Some(Yaml::Hash(jobs)) = top.get(&Yaml::String("jobs".to_owned())) else {
        // No `jobs` mapping: either the key is absent (a workflow with no jobs runs nothing) or
        // it is a shape the reader cannot decide. Absent is decided; a non-mapping is opaque.
        if top.contains_key(&Yaml::String("jobs".to_owned())) {
            gate.could_not_read("`jobs` is not a mapping");
        }
        return gate;
    };

    let workflow_directory = doc["defaults"]["run"]["working-directory"].as_str();
    for (_, job) in jobs {
        let Yaml::Hash(job) = job else {
            gate.could_not_read("a job under `jobs` is not a mapping");
            continue;
        };
        let Some(Yaml::Array(steps)) = field(job, "steps") else {
            continue;
        };
        let job_directory = field(job, "defaults")
            .and_then(|defaults| defaults["run"]["working-directory"].as_str())
            .or(workflow_directory);
        for step in steps {
            let Yaml::Hash(step) = step else { continue };
            // The Cypress action runs tests by default, but its install-only invocation
            // explicitly opts out. Checkout, setup and artifact actions run no suite.
            if field(step, "uses")
                .and_then(Yaml::as_str)
                .is_some_and(|action| action.starts_with("cypress-io/github-action@"))
            {
                let disabled = field(step, "with")
                    .and_then(Yaml::as_hash)
                    .and_then(|with| field(with, "runTests"))
                    .is_some_and(|value| {
                        value.as_bool() == Some(false) || value.as_str() == Some("false")
                    });
                if !disabled {
                    gate.steps.push("cypress run".to_owned());
                }
            }
            match field(step, "run") {
                Some(Yaml::String(run)) => {
                    let directory = field(step, "working-directory")
                        .and_then(Yaml::as_str)
                        .or(job_directory);
                    gate.steps.push(
                        directory.map_or_else(|| run.clone(), |dir| format!("cd '{dir}' && {run}")),
                    );
                }
                Some(Yaml::Hash(run)) if path == ".circleci/config.yml" => {
                    if let Some(Yaml::String(command)) = field(run, "command") {
                        gate.steps.push(command.clone());
                    }
                }
                _ => {}
            }
        }
    }
    gate
}

/// Read a YAML field including the merge keys `CircleCI` uses for anchored job templates.
fn field<'a>(map: &'a yaml_rust2::yaml::Hash, key: &str) -> Option<&'a Yaml> {
    if let Some(value) = map.get(&Yaml::String(key.to_owned())) {
        return Some(value);
    }
    match map.get(&Yaml::String("<<".to_owned()))? {
        Yaml::Hash(inherited) => field(inherited, key),
        Yaml::Array(inherited) => inherited.iter().find_map(|value| {
            let Yaml::Hash(inherited) = value else {
                return None;
            };
            field(inherited, key)
        }),
        _ => None,
    }
}

/// A blank CI gate at `path`: the source and tool decided, every runner/mutation field empty.
fn blank_ci_gate(path: &str) -> Gate {
    Gate::blank(path, GateTool::Ci)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_run_steps_across_jobs_in_order() {
        let source = "\
name: CI
on: [push]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: pnpm install
      - run: pnpm build
  test:
    runs-on: ubuntu-latest
    steps:
      - run: pnpm vitest run --coverage
";
        let gate = read_workflow(".github/workflows/ci.yml", source);
        assert_eq!(gate.tool, GateTool::Ci);
        assert_eq!(gate.source, ".github/workflows/ci.yml");
        assert!(!gate.opaque);
        assert_eq!(
            gate.steps,
            ["pnpm install", "pnpm build", "pnpm vitest run --coverage",]
        );
    }

    #[test]
    fn a_uses_only_workflow_has_no_run_steps() {
        let source = "\
jobs:
  lint:
    steps:
      - uses: actions/checkout@v4
      - uses: some/action@v1
";
        let gate = read_workflow(".github/workflows/lint.yml", source);
        assert!(gate.steps.is_empty());
        assert!(
            !gate.opaque,
            "a workflow with no run steps is decided, not opaque"
        );
    }

    #[test]
    fn a_block_scalar_run_is_kept_whole() {
        let source = "\
jobs:
  test:
    steps:
      - run: |
          pnpm install
          pnpm vitest run
";
        let gate = read_workflow(".github/workflows/ci.yml", source);
        assert_eq!(gate.steps.len(), 1);
        assert!(gate.steps[0].contains("pnpm install"));
        assert!(gate.steps[0].contains("pnpm vitest run"));
    }

    #[test]
    fn invalid_yaml_is_opaque() {
        let gate = read_workflow(".github/workflows/ci.yml", "jobs: [unterminated");
        assert!(gate.opaque);
        assert!(gate.steps.is_empty());
        assert_eq!(
            gate.unread,
            vec!["the workflow is not valid YAML".to_owned()]
        );
    }

    #[test]
    fn a_workflow_with_no_jobs_key_runs_nothing_and_is_decided() {
        let gate = read_workflow(".github/workflows/ci.yml", "name: CI\non: [push]\n");
        assert!(gate.steps.is_empty());
        assert!(!gate.opaque);
    }

    #[test]
    fn a_jobs_value_that_is_not_a_mapping_is_opaque() {
        let gate = read_workflow(".github/workflows/ci.yml", "jobs: not-a-map\n");
        assert!(gate.opaque);
        assert_eq!(gate.unread, vec!["`jobs` is not a mapping".to_owned()]);
    }
}
