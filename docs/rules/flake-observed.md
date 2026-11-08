# `flake-observed`

Reports tests with both a pass and a failure recorded for the same tree hash.

## What it checks

The rule reads normalized reports from `.spirolo/runs/`, grouped by file, test title
path and tree hash. A pass and failure must occur within one runner home
(runner configuration). A pass on retry counts as both a pass and a failure. A test
that always fails, a skipped result or a result without a tree hash is not evidence
of a pass/fail change.

Vitest and supported JUnit file-level failures can count against tests named in
other runs of the same home on the same tree when the failed run has no result
for those tests. Findings list counts, run identifiers and conditions from homes
where that test actually changed outcome.

## What to do

Ingest reports from your runner with `spirolo flake ingest` and the correct tree hash.
Use `--runner-home <identity>` to distinguish independent configurations collecting
the same test. Keep the identity stable across runs, including when report filenames
change. A `runner_home` string in the report's `run.json` sidecar supplies the same
identity; the flag takes precedence.

A sidecar with a nonempty `homes` list (`{"home":".","runner":"vitest"}` entries)
marks separate report streams. Without an explicit identity, ingest uses each
report's filename as its home, so use stable filenames across run directories.
The normalized ledger preserves this identity. Re-ingest older multi-home reports
to add it to existing ledger files.

Compare the failing and passing runs' configurations, environment and logs.
Reproduce the failure before deciding whether to change the test or application.
Quarantine records do not change your runner's behavior automatically.

## Limits

The tree hash identifies source, not a complete execution environment. Unlabelled
reports and older ledgers share one context for compatibility; distinguish their
runner homes when they contain independent configurations. A labelled home never
borrows a pass or failure from an unlabelled one.
Infrastructure failures can also look like test failures. The rule establishes
what the supplied reports say, not the cause of the change. No history means no
finding, not evidence that the suite is stable.

## Status and evidence

The recorded verdict is `FAIL`. It is hidden by default and cannot fail a gate, even with a severity override.
Run `spirolo rule-status` for the table embedded in your installed binary, or
`spirolo explain flake-observed` for its explanation. Use
`spirolo check --rule flake-observed --all` to inspect findings outside the default view.

See [rule tiers](../tiers.md), [measurement policy](../rule-visits/README.md),
the [status data](../../corpus/rule-status.json), and the
[detector source](../../crates/spirolo-rules/src/flake_observed.rs).
