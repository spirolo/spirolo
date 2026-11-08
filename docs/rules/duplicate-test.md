# `duplicate-test`

Groups tests whose complete bodies normalize to the same code and whose recorded execution context matches.

## What it checks

A group needs matching body hashes, matching bindings and resolved imports, and the
same before-hooks. Empty tests, unconditional skips, tests defined in loops and
parameterized `.each` cases are excluded. The report points to the other group
members. Shared setup alone is not enough to make a group.

## What to do

Compare the titles, inputs, assertions, hooks and runner projects. A title may
promise a case that a copied body never exercises. Correct that body, or remove a
confirmed duplicate after running the affected tests.

## Limits

Matching code and bindings do not establish matching runtime state. Earlier tests
can change suite-level state, and platform-specific runner projects can load
different implementations. The rule does not execute either test. Do not delete a
test solely because it appears in a group.

## Status and evidence

The recorded verdict is `UNADJUDICATED`. Visibility and gating depend on its measured tier and your severity overrides.
Run `spirolo rule-status` for the table embedded in your installed binary, or
`spirolo explain duplicate-test` for its explanation. Use
`spirolo check --rule duplicate-test --all` to inspect findings outside the default view.

See [rule tiers](../tiers.md), [measurement policy](../rule-visits/README.md),
the [status data](../../corpus/rule-status.json), and the
[detector source](../../crates/spirolo-rules/src/duplicate_test.rs).
