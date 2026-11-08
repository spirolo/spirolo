# `weak-assertion`

Looks for tests whose recognized assertions all carry no expected value. Separately,
reports the project-wide share of assertions classified as carrying no value.

## What it checks

Matchers such as `toBeTruthy`, `toBeDefined` and `not.toBeNull` count as checks
without an expected value. A test is reported when it has at least one recognized
assertion and all its recognized assertions have that form.
The classifier mostly reads the matcher, rather than what the subject computes.

The project-wide ratio divides the number of value-less assertions by the total
number of recognized assertions across tests. Value-less assertions in tests that
also contain value-carrying assertions count too. Hooks and benchmarks are excluded.
A separate finding is reported only when this ratio exceeds `maxRatio`, whose
default is `0.06`. No ratio finding is reported when there are no recognized assertions.

## What to do

Read the assertion's subject before changing it. If the test only checks that a
result exists, compare the behavior or value it promises to test. Keep a truthiness
check when the subject already expresses the expected condition.

## Limits

`queryByText(expectedText)`, a boolean predicate, or a lookup by an expected key can
be a meaningful assertion even with a truthiness matcher. Helper assertions and
throwing queries may also be missed. The reported ratio is a syntax-based measure,
not a measure of how many tests are useful.

## Status and evidence

The recorded verdict is `DROPPED`. It is hidden by default and cannot fail a gate, even with a severity override.
Run `spirolo rule-status` for the table embedded in your installed binary, or
`spirolo explain weak-assertion` for its explanation. Use
`spirolo check --rule weak-assertion --all` to inspect findings outside the default view.

See [rule tiers](../tiers.md), [measurement policy](../rule-visits/README.md),
the [status data](../../corpus/rule-status.json), and the
[detector source](../../crates/spirolo-rules/src/weak_assertion.rs).
