# `fail-closed-setup`

Looks for test setup that may let console errors, unhandled mock requests or leftover mocks pass unnoticed.

## What it checks

For Jest and Vitest configurations, the rule checks for a console guard and mock
restoration. When a mock server is present, it also checks for a shared server and
an error on unhandled requests. Shared presets can produce a single finding for
the packages that inherit them.

Recognized console guards throw on console errors or warnings, use a fail-on-console
package, or record console calls and throw from a hook. Mock restoration can come
from `restoreMocks: true` or `restoreAllMocks()` in `afterEach`. Restoring only in
`afterAll` does not count. A configuration the reader cannot resolve produces an
undecided result instead of a list of missing safeguards. Configurations that
collect no tests are skipped.

## What to do

Inspect the named setup file and its inherited configuration. Add the missing guard
if it fits your suite. Tests that intentionally log errors should make that expectation
explicit and restore their mocks afterward.

## Limits

Custom wrappers and dynamically built setup can escape recognition. A finding does
not prove that a test passed incorrectly, and the absence of findings does not prove
that every setup path fails on errors. Check which configuration CI actually loads.

## Status and evidence

The recorded verdict is `UNADJUDICATED`. Visibility and gating depend on its measured tier and your severity overrides.
Run `spirolo rule-status` for the table embedded in your installed binary, or
`spirolo explain fail-closed-setup` for its explanation. Use
`spirolo check --rule fail-closed-setup --all` to inspect findings outside the default view.

See [rule tiers](../tiers.md), [measurement policy](../rule-visits/README.md),
the [status data](../../corpus/rule-status.json), and the
[detector source](../../crates/spirolo-rules/src/fail_closed_setup.rs).
