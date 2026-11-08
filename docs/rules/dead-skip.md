# `dead-skip`

Lists skipped and focused tests, and reports unconditional skips without a nearby issue reference.

## What it checks

Unconditional `skip`, `todo` and `fixme` definitions need an issue reference in the
title, reason or adjacent comments. The reader checks a trailing comment and up to
three comment lines above the call, stopping at a blank or code line. Issue numbers,
issue and pull-request URLs, merge-request URLs and ticket keys are recognized.

Environment-dependent skips are reported separately. Focused tests using `only`
are also reported. Every skip site has an informational inventory row.

## What to do

For an unfinished test, link the work needed to restore it. Remove accidental focus
before committing. Keep intentional environment gates, but check that the intended
CI job enables them.

## Limits

A linked issue is evidence of intent, not proof that a skip is still justified or
that the issue remains open. Comments inside a test body are not read. Conditional
in-body skips can be left alone when their guard cannot be understood. The rule
does not run skipped tests or determine whether they would pass.

## Status and evidence

The recorded verdict is `INSUFFICIENT`. Visibility and gating depend on its measured tier and your severity overrides.
Run `spirolo rule-status` for the table embedded in your installed binary, or
`spirolo explain dead-skip` for its explanation. Use
`spirolo check --rule dead-skip --all` to inspect findings outside the default view.

See [rule tiers](../tiers.md), [measurement policy](../rule-visits/README.md),
the [status data](../../corpus/rule-status.json), and the
[detector source](../../crates/spirolo-rules/src/dead_skip.rs).
