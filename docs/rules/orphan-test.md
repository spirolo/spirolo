# `orphan-test`

Reports test files that no recognized runner configuration collects, or whose project imports appear to point to missing source.

## What it checks

`never-collected` findings name the collection globs or workspace boundary that
excluded a file. `source-gone` findings point to a missing project import. An import
whose resolution is unknown can produce an undecided result instead.

## What to do

Use your runner's own test listing with the same arguments and configuration as CI.
Check whether the file registers tests through another file, belongs to a fixture,
or imports generated output that must be built first. Fix collection or imports
only after confirming the runner has the same problem.

## Limits

Wrapper CLIs, runtime configuration, alternate configuration files and unsupported
runners can collect tests the model misses. Shared test factories and fixture apps
can look like uncollected tests. Build output and platform resolution can look like
missing source. Install dependencies and read module-resolution diagnostics before
acting on a finding.

## Status and evidence

The recorded verdict is `DROPPED`. It is hidden by default and cannot fail a gate, even with a severity override.
Run `spirolo rule-status` for the table embedded in your installed binary, or
`spirolo explain orphan-test` for its explanation. Use
`spirolo check --rule orphan-test --all` to inspect findings outside the default view.

See [rule tiers](../tiers.md), [measurement policy](../rule-visits/README.md),
the [status data](../../corpus/rule-status.json), and the
[detector source](../../crates/spirolo-rules/src/orphan_test.rs).
