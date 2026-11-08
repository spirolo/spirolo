# Good first issues

These are draft tasks, not filed issues. Each covers a small documentation gap
around behavior already in the code. Before starting, check the repository's open
issues and pull requests for overlapping work. Use `docs` and `good first issue`
when filing a task; the proposed label definitions are in
[labels.yml](../.github/labels.yml).

Follow [CONTRIBUTING.md](../CONTRIBUTING.md). Keep examples public-safe. Record
output from an actual run on a shareable project, and name the version and command
used. Do not invent output or findings to illustrate a claim. Run [`typos`](https://github.com/crate-ci/typos)
and check relative links for each documentation change.

## Document what init writes and when it overwrites

**Gap:** The configuration generator explains its keys in the generated file,
but there is no short user guide for creating or replacing that file.

**Start:** [init.rs](../crates/spirolo/src/init.rs) and its
[command tests](../crates/spirolo/tests/init.rs), especially
`init_refuses_to_overwrite_without_force` and `init_overwrites_with_force`.

**Task:** Add `docs/configuration.md` with a walkthrough of `spirolo init`, the
generated `.spirolorc.jsonc`, and `--force`. Explain which detected suggestions are
commented out and need review.

**Done when:** The guide shows the initial write, the refusal to overwrite, and
an explicit overwrite in a disposable checkout. It links from the README and
makes clear that `--force` replaces local configuration.

## Explain configuration path and parse failures

**Gap:** The config reader distinguishes a missing default, a missing explicit
path, and malformed JSONC. The user docs lack a troubleshooting comparison.

**Start:** [rc.rs](../crates/spirolo/src/rc.rs), particularly
`resolve_config_path` and `load_path`, and the config cases in
[init tests](../crates/spirolo/tests/init.rs).

**Task:** Add a configuration troubleshooting section to
[CI reports](ci-reports.md). Explain where an explicit `--config` path is resolved
and what happens when a config cannot be parsed.

**Done when:** Real command output demonstrates each case. The text distinguishes
an explicit missing-path error from a parse diagnostic that falls back to defaults,
and links to the existing module-resolution troubleshooting section.

## Add a baseline maintenance recipe

**Gap:** CI report documentation mentions baseline filtering but does not show
how to create and maintain the baseline.

**Start:** [baseline.rs](../crates/spirolo/src/baseline.rs), `RatchetArgs` in
[main.rs](../crates/spirolo/src/main.rs), and
[ratchet tests](../crates/spirolo/tests/ratchet.rs).

**Task:** Add a recipe to [CI reports](ci-reports.md) for `spirolo ratchet`,
`--tighten`, `--prune`, and `check --fail-on-stale-baseline`.

**Done when:** The recipe names `.spirolo/baseline.json`, explains which commands
write it, and distinguishes tightening ceilings from removing stale entries.
Examples record the observed output and exit status. They explain that recording
findings does not fix them or prove their rules correct.

## Show impact analysis from a changed-file list

**Gap:** The CLI accepts changed paths on stdin, but there is no user guide for
this workflow and its unresolved results.

**Start:** `ImpactArgs` in [main.rs](../crates/spirolo/src/main.rs), and the
`--changed -` branch that reads paths from stdin.

**Task:** Add `docs/impact.md` showing `spirolo impact --changed - --root <project>`
with a real changed-file list. Explain paths relative to the project root and the
`selected`, `unreached`, and `diagnostics` fields in JSON output.

**Done when:** The example has a matching explicit-path invocation and records
the same selection for both. It explains that Spirolo selects tests without running
them and that an unreached file is not evidence that no tests are needed. Link
the guide from the README.

## Show how to investigate a finding by its id

**Gap:** The README introduces rule explanations, but lacks a worked path from
a JSON finding to an explanation of that specific finding.

**Start:** `render_explain` in [main.rs](../crates/spirolo/src/main.rs) and
[explain tests](../crates/spirolo/tests/explain.rs).

**Task:** Add a short investigation recipe to [CI reports](ci-reports.md): take
an actual finding id from `spirolo check --format json`, then run
`spirolo explain <id> <project> --format json` against the same tree and config.

**Done when:** The recipe distinguishes a rule name from a finding id, shows
the evidence returned for that finding, and records what happens for an unknown
id. Any quoted id and output come from the documented run.
