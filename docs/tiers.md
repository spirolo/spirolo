# Rule tiers, `--all` and overrides

No rule in the current status table is proven. Every rule still runs on every `check`,
but the default view shows only rules that meet the visibility threshold below,
plus rules explicitly enabled by a severity override. This page explains how a rule
is placed, what each tier looks like, and how to change what you see or what gates.

## Where the numbers come from

Findings are sampled from pinned open-source projects, then checked against source
and execution evidence. Each sampled finding is true, false or undecided. Precision
counts only decided findings; it does not measure recall or the chance that a new
finding is correct in your project. The results are recorded in
[`corpus/rule-status.json`](../corpus/rule-status.json),
which is embedded in the binary at build time. `spirolo rule-status` prints the table the binary
carries, and `spirolo explain <rule>` prints one rule's row as a Status block.

A row has a verdict (`PASS`, `UNADJUDICATED`, `INSUFFICIENT`, `SKIPPED`, `FAIL` or `DROPPED`;
a rule with no row is `UNMEASURED`), the true and false counts, and the shown precision
(true / decided).

## The four tiers

The tier is computed from the rule's row alone, without special cases for rule names.

| Tier | Which rules | View without an override | Severity without an override | Heading label |
| --- | --- | --- | --- | --- |
| proven | verdict `PASS` | shown | the rule's own severity | none |
| advisory | any other verdict except FAIL and DROPPED, with **≥15 decided** and **shown precision ≥0.80** | shown | `info` | ` · advisory: <true>/<decided> (<STATUS>)` |
| experimental | every other rule except FAIL and DROPPED, including rules with nothing decided and `UNMEASURED` | `--all` only | `info` | ` · experimental: <true>/<decided> (<STATUS>)` or ` · experimental: not yet measured` |
| hidden | verdict `FAIL` or `DROPPED` | `--all` only | `info`, always | ` · rule: FAIL` or ` · rule: DROPPED` |

The precision floor is compared exactly (`true × 5 ≥ decided × 4`), so 16/20 is advisory and
11/15 is not. The floors are `SHOWN_DECIDED_FLOOR` and `SHOWN_PRECISION_FLOOR` in
`crates/spirolo-rules/src/status.rs`. A rule is proven only after it clears the pre-registered
bar in [rule-visits/README.md](rule-visits/README.md) (a Wilson 95% lower bound of at least
0.9, so at least 35 decided with none false, plus a fresh held-out check). No rule has reached it.

## What the default view shows

`spirolo check` (and `audit`, `gate`, `hygiene`, `flake`, MCP `check` and `spirolo lsp`) shows
the findings of proven and advisory rules, plus rules enabled by an applicable
`warn` or `error` override. Findings of experimental and hidden rules are
still counted in the provisional health score. The closing line counts hidden
findings by tier and rule and points to `spirolo check --all`.

Machine formats (`--format json`, `sarif`, `markdown`, `github-annotations`, `--agent`) omit
the same rows; every finding they do carry has its rule's status (`rule_status` in JSON,
`properties.ruleStatus` in SARIF). In the editor, an `info` row of a rule the default view
shows is an Information diagnostic, and an `info` row of any other rule is a Hint.

## `--all`

`--all` shows every rule's findings in every format; each human section is labelled with its tier.
It does not change severities or the exit code. Without an applicable severity
override, experimental and hidden findings remain `info`.
With `--rule <rule>`, `--all` also lifts the file and row caps of the human report for that
rule. The VS Code setting `"spirolo.all": true` runs `spirolo lsp --all`.

## Overrides: what gates

Without a config, no unproven rule changes the exit code. To make a rule gate, give it a
`warn` or `error` severity in `.spirolorc.jsonc`:

```jsonc
{ "rules": { "dead-skip": { "severity": "warn" } } }
```

- On an **advisory or experimental** rule, a `warn` or `error` override applies: the rule's
  findings show in the default view at that severity. A new, decided `warn` or `error`
  finding outside the baseline makes the command exit 1.
  Use this only after reviewing the rule on your project.
- On a **FAIL or DROPPED** rule, the override is not applied. Its rows stay at `info`, stay
  out of the default view and never gate. The human report says so:
  ` note   config severity for orphan-test not applied: rule is DROPPED`.
- An `info` override is accepted but opts nothing in: it does not show an experimental rule
  by default.
- Hygiene rules (duplicate-test, redundant-test, orphan-test, stale-snapshot, unused-mock)
  accept `warn` at most; `error` is rejected as a config error.

## When a rule moves

The tiers follow the table, so a rule moves when its numbers change: a measurement round adds
decided rows, an evaluation records a new verdict, or a rule is dropped. The next build embeds the
new table, and the rule's tier, label, default visibility and editor severity
move with it, with no code change. Run `spirolo rule-status` after upgrading to read
the table embedded in that binary.

Two consequences to keep in mind when upgrading:

- A rule that falls below a floor leaves the default view; its findings are then counted on
  the closing line. An override you set on it still applies, so it keeps gating.
- A rule that becomes FAIL or DROPPED stops gating even with an override, and the report
  prints the `not applied` note. A CI job that relied on it no longer fails on its rows.

Tier and override behavior is defined in
[`status.rs`](../crates/spirolo-rules/src/status.rs). The threshold examples above
illustrate that policy; they are not measurements of a particular project.
