# `untested-hotspot`

Ranks source modules for inspection using churn, complexity and a static estimate of test reach.

## What it checks

The score is `churn × complexity × weakness`. Churn is added and deleted lines plus
`1` from the supplied history; without history it is `1`, with a diagnostic saying
churn is missing. Complexity counts functions and branches in the parsed module.

Weakness is `1.0` when no test reaches a module. For a reached module, it is the
share of named value exports no reaching test references, with a floor of `0.1`.
A reached module with no named exports uses `0.5`. A module whose named value
exports are all referenced is omitted. These values are defined in the rule source.

The ranking excludes recognized tests, declarations, generated files and several
common configuration, example and fixture paths. `spirolo impact --hotspots` exposes
the same ranking.

## What to do

Inspect a high-ranked module's role and its tests. Use coverage or mutation results
to check whether important behavior lacks assertions. Supply churn from the period
you care about if recent changes should affect the order.

## Limits

Referencing an export does not prove that a test asserts its behavior. Import reach
is not execution coverage, and this score does not read mutation results. Unusual
example or generated paths can still be ranked. A high score is a review priority,
not proof of a bug or an untested module.

## Status and evidence

The recorded verdict is `UNADJUDICATED`. Visibility and gating depend on its measured tier and your severity overrides.
Run `spirolo rule-status` for the table embedded in your installed binary, or
`spirolo explain untested-hotspot` for its explanation. Use
`spirolo check --rule untested-hotspot --all` to inspect findings outside the default view.

See [rule tiers](../tiers.md), [measurement policy](../rule-visits/README.md),
the [status data](../../corpus/rule-status.json), and the
[detector source](../../crates/spirolo-rules/src/untested_hotspot.rs).
