# `data-layer-mock`

Reports tests in your declared integration scope that mock a module you have declared as part of the data layer.

## What it checks

Both configuration entries are needed: the test path must match `dataLayerScope`,
and the mock target must match `dataLayer`. The default scope includes
`**/*.integration.*`, `**/*.msw.*` and `**/integration/**`. Module paths are compared
without their extension; package names also match their subpaths.

With no `dataLayer` declaration, the rule reports nothing. `spirolo init` suggests
frequently mocked modules in comments, but does not declare them for you.

## What to do

Decide which boundary the test is meant to exercise. If it is an integration test
of the data layer, move the mock to the external boundary. If it is intentionally
a unit test, check whether your configured scope describes it correctly.

## Limits

The rule trusts your declarations. It does not infer an integration contract or
prove that a mock bypasses useful behavior. A mock outside the declared scope is
not judged, and a clean report without configuration says nothing about this issue.

## Status and evidence

The recorded verdict is `DROPPED`. It is hidden by default and cannot fail a gate, even with a severity override.
Run `spirolo rule-status` for the table embedded in your installed binary, or
`spirolo explain data-layer-mock` for its explanation. Use
`spirolo check --rule data-layer-mock --all` to inspect findings outside the default view.

See [rule tiers](../tiers.md), [measurement policy](../rule-visits/README.md),
the [status data](../../corpus/rule-status.json), and the
[detector source](../../crates/spirolo-rules/src/data_layer_mock.rs).
