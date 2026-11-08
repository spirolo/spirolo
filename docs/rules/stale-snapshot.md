# `stale-snapshot`

Looks for snapshot entries without a matching test name, and snapshot files without a recognized sibling test file.

## What it checks

The rule reads external snapshot files and compares their keys with recognized
test titles. Computed titles can produce an undecided result. A custom snapshot
resolver also limits what the rule can decide.

## What to do

Run the owning test file with its normal runner and configuration. Review the
runner's obsolete-snapshot report before removing an entry. Check that the sibling
file was parsed and was not excluded by ignore rules.

## Limits

Aliases of `it` or `test`, imported suite factories and dynamically generated titles
can hide the test that owns a snapshot. An ignored sibling can look missing even
when it exists. A static finding does not establish that a snapshot is unused, and
updating snapshots without reviewing their changes can discard useful assertions.

## Status and evidence

The recorded verdict is `DROPPED`. It is hidden by default and cannot fail a gate, even with a severity override.
Run `spirolo rule-status` for the table embedded in your installed binary, or
`spirolo explain stale-snapshot` for its explanation. Use
`spirolo check --rule stale-snapshot --all` to inspect findings outside the default view.

See [rule tiers](../tiers.md), [measurement policy](../rule-visits/README.md),
the [status data](../../corpus/rule-status.json), and the
[detector source](../../crates/spirolo-rules/src/stale_snapshot.rs).
