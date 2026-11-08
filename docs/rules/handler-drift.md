# `handler-drift`

Compares recognized mock-handler patterns with the API base URLs you configure.

## What it checks

The reader recognizes MSW and nock handlers in configured handler files. Without
handler globs, it looks for paths with a `mocks` or `msw` segment. `urlSources`
supplies literal bases or module exports; configured `env` values can fill holes.

A finding means no concrete base can match the handler's origin and path under the
rule's pattern model. Computed patterns or missing concrete bases can produce
undecided results. Regular-expression patterns are not judged.

## What to do

Compare the handler with the URL the application actually requests, including its
base path and environment. Correct the handler or the configured base when they
disagree. Use the test runner to confirm that the intended request reaches it.

## Limits

Without `urlSources`, the rule cannot establish which production URL a handler
should match. Pattern matching is static and cannot prove which handlers execute,
which request wins, or whether a handler is intentionally unused in a given test.

## Status and evidence

The recorded verdict is `DROPPED`. It is hidden by default and cannot fail a gate, even with a severity override.
Run `spirolo rule-status` for the table embedded in your installed binary, or
`spirolo explain handler-drift` for its explanation. Use
`spirolo check --rule handler-drift --all` to inspect findings outside the default view.

See [rule tiers](../tiers.md), [measurement policy](../rule-visits/README.md),
the [status data](../../corpus/rule-status.json), and the
[detector source](../../crates/spirolo-rules/src/handler_drift.rs).
