# CI reports

`spirolo check`, `audit`, `gate`, `hygiene` and `flake` accept two CI formats:

```sh
spirolo check --format markdown >> "$GITHUB_STEP_SUMMARY"
spirolo check --format github-annotations
```

Both formats, like every `check` format, carry by default the findings of proven rules
(status PASS) and of advisory rules (unproven, with at least 15 decided rows and shown
precision of at least 0.80 in the embedded status table). Experimental findings also
appear when an applicable `warn` or `error` override enables them. Otherwise they
need `--all`, as do FAIL and DROPPED findings. See
[rule tiers](tiers.md) for overrides and their effect on the exit code.

Every finding carries its rule's status, the rule's verdict in the embedded status table:
`PASS`, `UNADJUDICATED`, `INSUFFICIENT`, `SKIPPED`, `UNMEASURED`, `FAIL` or `DROPPED`.
`--format json` gives it as `rule_status` on each finding, SARIF as `properties.ruleStatus`
on each result, and `--agent` as the second column of each finding row (`rule`, status,
severity, location, message). SARIF maps the finding's severity to `note`, `warning`
or `error`. An applicable severity override changes that level without changing the
rule's status.

Markdown prints the embedded rule-status table, including measured precision and its
95% interval, followed by findings from proven rules. Findings from unproven rules appear in
a collapsed info section even when the user overrides their severity. Each findings table
and the info section's per-rule counts give the rule's status beside the rule. Each row retains
its actual severity and outcome; a listing or an undecided row remains labelled that way.
Project-wide rows say `project`. Source text is escaped so paths and messages cannot break
the table or insert HTML.

GitHub annotations print one workflow command per decided finding, in the same order as
JSON, at the finding's severity: `::notice` for info, `::warning` for warn and `::error`
for error, including applicable severity overrides. Each title is the rule and its
recorded status, such as `dead-skip (INSUFFICIENT)`. At most ten annotations of each
level are printed, as capped by the report renderer. A final plain log line counts decided
findings, emitted annotations and omitted findings. Listing and undecided rows do not become
annotations. Project-wide annotations omit the optional file and line properties. Percent
signs and newlines are escaped in messages; colons and commas are also escaped in properties.

Both formats use the same baseline-filtered findings and exit code as the other check
formats. Choosing an output format does not change which rules run or gate the command.
`spirolo list` retains its graph formats: human, JSON, SARIF and agent.

## When module resolution cannot load

A checkout before its install often has a `tsconfig.json` whose `extends` names a package
that is not in `node_modules` yet (`astro/tsconfigs/strict`). Every import under that config
is then left unresolved, and the rules that read imports would report the holes as findings.
Every `check` format says so:

- when the root config failed or the dependencies are not installed, the human report prints
  a `! module resolution failed` warning under the health line, with the config that failed,
  the likely cause and the fix (install dependencies, e.g. `npm ci`, then rerun);
- when only nested configs failed in an installed project (a stale `archive/` app extending a
  removed preset), it prints one closing ` note` line instead, naming the directories, the
  imports left unresolved and the rows marked;
- JSON carries a `module_resolution_failed` diagnostic beside the `tsconfig_unloadable` ones:
  `warning` in the first case, `info` in the second;
- every row of orphan-test, untested-hotspot, unused-mock and absence-anchor under a config
  that failed (the whole project when it is the root `tsconfig.json`) gets an `unreliable`
  evidence entry and ends its message with `(unreliable: module resolution failed)`, which
  SARIF, Markdown, annotations and the agent view all show. These are the rules a hole in
  the import graph turns into rows; the others that read imports only miss rows, or resolve
  a relative import by its path.

These findings keep their severity and id. Marking them unreliable does not exempt
them from gating: a new, decided finding at `warn` or `error` outside the baseline
still fails the build. Findings at `info` do not fail it, so a successful command
does not establish that module resolution worked. A project whose configs all load
gets no module-resolution warning or marked finding.

The annotation cap and escaping rules are implemented in
[`spirolo-report`](../crates/spirolo-report/src/). Reports describe what Spirolo found;
selecting a format does not run the tests or validate a rule's claim.
