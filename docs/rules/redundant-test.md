# `redundant-test`

Groups tests that kill the same nonempty set of mutants in an existing mutation report.

## What it checks

The rule reads per-test `killedBy` data from Stryker reports. Reports with truncated
results from bail, or without per-test data, produce an undecided result. A group
that kills only emptied-block mutants is also undecided. Without a mutation report,
the rule has no evidence to report.

The explanation distinguishes matching normalized bodies from differing bodies.
It is more cautious when bodies differ or a report test cannot be joined to a
source test.

## What to do

Inspect the report's scope and the tests' assertions. Before removing a test, rerun
the affected suite and mutation analysis with that test removed. Equal kill sets
can help identify overlap, but are not sufficient evidence for deletion.

## Limits

A report covers only the mutants it contains. Different behaviors can kill the
same mutants, and test order or shared state can affect the result. Missing,
stale or incomplete per-test data limits what the rule can decide. It neither
creates mutants nor runs Stryker.

## Status and evidence

The recorded verdict is `DROPPED`. It is hidden by default and cannot fail a gate, even with a severity override.
Run `spirolo rule-status` for the table embedded in your installed binary, or
`spirolo explain redundant-test` for its explanation. Use
`spirolo check --rule redundant-test --all` to inspect findings outside the default view.

See [rule tiers](../tiers.md), [measurement policy](../rule-visits/README.md),
the [status data](../../corpus/rule-status.json), and the
[detector source](../../crates/spirolo-rules/src/redundant_test.rs).
