# `flake-candidate`

Looks for assertions whose result can depend on elapsed wall-clock time or a calendar value read again from the clock.

## What it checks

The rule follows clock reads into elapsed-time budgets and calendar comparisons.
Recognized fake timers or a fixed system time can guard a read. Hooks, setup files
and reachable test helpers can contribute evidence. Calls the reader cannot follow
can produce an undecided result.

The current detector reports clock-related candidates. Other patterns recorded
by the model, such as random draws, fixed waits and shared state, are not reported
by this rule. It does not read run history.

## What to do

Check whether the assertion is about behavior or machine speed. For deterministic
behavior tests, control time and wait for the expected state. Keep intentional
performance checks separate and make their environment and budget explicit.

## Limits

A candidate has not been observed failing. Intentional timing tests and unfamiliar
helper control flow can produce misleading findings. A lower-bound assertion or
a deadline that only caps a polling wait is not the same as an upper time budget;
the reader recognizes supported forms of that distinction. A clean report does
not establish that a test is reliable. Use [run history](flake-observed.md) to
inspect observed pass/fail changes.

Supported polling forms include `do…while` and `while (true)` bodies that break or
return when the awaited state is ready. Their deadline comparisons can use clock
bindings declared or assigned inside the body. A loop whose only exit depends on
elapsed time still reports clock dependence; asserting an elapsed-time upper
budget inside a poll also remains a candidate.

## Status and evidence

The recorded verdict is `INSUFFICIENT`. Visibility and gating depend on its measured tier and your severity overrides.
Run `spirolo rule-status` for the table embedded in your installed binary, or
`spirolo explain flake-candidate` for its explanation. Use
`spirolo check --rule flake-candidate --all` to inspect findings outside the default view.

See [rule tiers](../tiers.md), [measurement policy](../rule-visits/README.md),
the [status data](../../corpus/rule-status.json), and the
[detector source](../../crates/spirolo-rules/src/flake_candidate.rs).
