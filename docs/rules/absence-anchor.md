# `absence-anchor`

Looks for render tests that check only absence, without a recognized assertion that the render produced anything.

## What it checks

The rule recognizes absence checks such as `queryBy*(...).toBeNull()`,
`not.toBeInTheDocument()` and `toBeEmptyDOMElement()`. A positive DOM assertion,
a throwing Testing Library query, or a recognized nonzero spy-call assertion can
anchor the test. Throwing queries in enclosing hooks and supported helpers count.
Playwright tests and tests with assertions the rule cannot classify are left alone.

Most negated jest-dom matchers still require an element: `not.toHaveClass()` or
`not.toHaveAttribute()` throws on a missing subject. Calls, optional call chains,
nullable child properties such as `container.firstChild`, and locals bound to those
reads count as anchors for these matchers. `not.toBeInTheDocument()` accepts a
missing element and remains an absence check.

## What to do

Check whether an empty render would satisfy the test. If the component should render
content, assert that expected content is present before checking what is absent.
Keep the absence assertion: it may still test useful behavior.
If an empty render is intentional, inspect the render setup and error handling
instead; there may be no content to assert as present.

## Limits

An empty render can be the intended result. The rule cannot infer that intent from
props. It can also miss anchors expressed through unsupported helpers or property
accesses. A finding is a reason to inspect the test, not proof that a broken component
would pass. Assertions the rule leaves unread can hide a real gap.

## Status and evidence

The recorded verdict is `UNADJUDICATED`. Visibility and gating depend on its measured tier and your severity overrides.
Run `spirolo rule-status` for the table embedded in your installed binary, or
`spirolo explain absence-anchor` for its explanation. Use
`spirolo check --rule absence-anchor --all` to inspect findings outside the default view.

See [rule tiers](../tiers.md), [measurement policy](../rule-visits/README.md),
the [status data](../../corpus/rule-status.json), and the
[detector source](../../crates/spirolo-rules/src/absence_anchor.rs).
