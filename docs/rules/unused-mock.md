# `unused-mock`

Looks for module mocks outside the imports a test can reach, and mock or spy bindings that the file never reads.

## What it checks

The rule follows resolved project imports transitively. External and builtin
specifiers are recorded at the edge of that graph. Setup-file mocks are checked
against the combined imports of the tests they govern. Unknown mock targets or
incomplete import paths can produce undecided results.

## What to do

Trace how the tested code loads the mocked module. Remove a mock only after running
the affected tests without it. For an unread spy binding, check whether creating
the spy itself changes behavior even though its return value is unused.

## Limits

Static reachability is not runtime use. Installed package internals, dynamic imports,
custom loaders and unsupported source formats can leave gaps in the graph. A mock
can also have a side effect without a read of its binding. The rule cannot prove
that removing it preserves the test.

## Status and evidence

The recorded verdict is `INSUFFICIENT`. Visibility and gating depend on its measured tier and your severity overrides.
Run `spirolo rule-status` for the table embedded in your installed binary, or
`spirolo explain unused-mock` for its explanation. Use
`spirolo check --rule unused-mock --all` to inspect findings outside the default view.

See [rule tiers](../tiers.md), [measurement policy](../rule-visits/README.md),
the [status data](../../corpus/rule-status.json), and the
[detector source](../../crates/spirolo-rules/src/unused_mock.rs).
