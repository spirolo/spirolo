# `generator-walk`

Identifies saga tests that assert individual generator steps rather than the behavior of a completed run.

## What it checks

Recognition depends on imports from `redux-saga/effects` or `testSaga` from
`redux-saga-test-plan`. The rule recognizes assertions on `.next().value` against
an imported effect creator, and fluent `testSaga(...).next(...)` chains. It reports
individual sites and a per-file inventory.

## What to do

Read what the test promises. If it promises an observable result, consider testing
that result through a saga run. Keep step assertions when the sequence of effects
is itself the contract you intend to protect.

## Limits

A generator-step assertion is not inherently wrong. The rule does not run the saga
or prove that a behavior-level replacement covers the same contract. Other iterator
libraries and unrecognized helper wrappers are outside its recognition scope.

## Status and evidence

The recorded verdict is `DROPPED`. It is hidden by default and cannot fail a gate, even with a severity override.
Run `spirolo rule-status` for the table embedded in your installed binary, or
`spirolo explain generator-walk` for its explanation. Use
`spirolo check --rule generator-walk --all` to inspect findings outside the default view.

See [rule tiers](../tiers.md), [measurement policy](../rule-visits/README.md),
the [status data](../../corpus/rule-status.json), and the
[detector source](../../crates/spirolo-rules/src/generator_walk.rs).
