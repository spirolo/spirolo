# `gate-truth`

Reports test and CI settings that may allow a build to pass without enforcing the checks you expect.

## What it checks

- Missing coverage thresholds in readable Jest or Vitest configurations.
- A missing or zero Stryker mutation-score `break` threshold.
- Playwright configurations that do not forbid focused tests in CI.
- Root GitHub workflows with no recognized test command in their `run` steps.
- Retries, as information, and configured documentation claims the settings do not back.

Opaque configuration produces an undecided result. Recognized inherited presets can
be reported once rather than once per consumer.

## What to do

Follow your CI job through its scripts to the configuration it actually runs.
Check whether coverage collection is enabled and whether a failing check changes
the job's exit code. Set thresholds and focused-test guards to match your project's
policy; a threshold's presence alone does not establish useful coverage.

## Limits

The rule does not prove that CI loads a configuration. Its workflow check does not
follow shell scripts, make targets or `uses:` actions, and it reads root GitHub
workflows rather than every CI service. A separate linter may already reject focused
tests. Child project configurations can be reported even when a root configuration
owns coverage. Treat missing-setting findings as questions about your build.

## Status and evidence

The recorded verdict is `UNADJUDICATED`. Visibility and gating depend on its measured tier and your severity overrides.
Run `spirolo rule-status` for the table embedded in your installed binary, or
`spirolo explain gate-truth` for its explanation. Use
`spirolo check --rule gate-truth --all` to inspect findings outside the default view.

See [rule tiers](../tiers.md), [measurement policy](../rule-visits/README.md),
the [status data](../../corpus/rule-status.json), and the
[detector source](../../crates/spirolo-rules/src/gate_truth.rs).
