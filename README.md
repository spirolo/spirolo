# Spirolo

[![CI](https://github.com/spirolo/spirolo/actions/workflows/ci.yml/badge.svg)](https://github.com/spirolo/spirolo/actions/workflows/ci.yml)
[![Licence](https://img.shields.io/github/license/spirolo/spirolo)](LICENSE)
[![npm](https://img.shields.io/npm/v/spirolo)](https://www.npmjs.com/package/spirolo)

Spirolo is a static analyzer for JavaScript and TypeScript test suites.

It reads tests, source code and runner configuration together to find missing setup
guards, untracked skips, duplicate tests and assertions that may pass without
testing anything. Analysis runs locally in a Rust binary, without running your
tests or sending your code anywhere.

## What Spirolo will not do

- It does not run your tests or execute your code.
- It does not fix or rewrite your source files.
- It does not fail CI by default: findings are advisory until you raise a rule's severity.
- It does not send your code anywhere: analysis needs no network access.
- It covers JavaScript and TypeScript test suites only.

## Install

Spirolo is an unpublished alpha. For now, build from a source checkout with the
current stable Rust toolchain, from the repository root:

```sh
cargo build --release --locked -p spirolo
./target/release/spirolo --version
./target/release/spirolo check /path/to/your/project
```

No Spirolo config is needed. Install your project's dependencies first so its
TypeScript configuration and imports can resolve.

The npm launcher is `spirolo`. It requires Node 20 or newer and selects a
native platform package without a postinstall download. It is not on the registry
yet; [release staging](docs/release.md) describes how to build and install local
package tarballs.

## One real run

Captured from the release binary built at Spirolo commit `4a8d80cd`, on
[Kaneo](https://github.com/usekaneo/kaneo) at commit
[`afe97be2473e0cd97fe4a63e526fef9ac4df85b0`](https://github.com/usekaneo/kaneo/tree/afe97be2473e0cd97fe4a63e526fef9ac4df85b0),
with its dependencies already installed. Set `KANEO` to the path of that checkout,
then run from Spirolo's source directory:

```sh
./target/release/spirolo check "$KANEO"
```

Excerpt below: `[...]` marks omitted lines. The timing is from this run, not a
benchmark.

```text
1754 files · 351 test files · 1797 tests (1797 vitest) · 4309 expects · 0 handlers · 35 configs · 253ms
Health C (60)
[...]
● fail-closed-setup (10 files, 19 findings) · advisory: 19/19 (UNADJUDICATED)
  apps/api/vitest.config.ts (2)
    :0 setup does not guard the console (no throwing console.error/warn spy or fail-on-console package)
    :0 setup does not restore mocks between tests (no restoreMocks or afterEach restoreAllMocks)
[...]
34 findings in 3 of 16 rules · 0 warn · 0 error · 34 info · exit 0
 hidden 1226 findings: 1211 experimental (gate-truth 11, unused-mock 3, flake-candidate 1, untested-hotspot 1196), 15 from DROPPED or FAIL rules (weak-assertion 15); spirolo check --all shows them
 note   no churn input: untested-hotspot ranks by complexity × weakness alone. Save `git log --numstat --format= --since=180.days` to a file and pass it with `--churn <file>` or name it in `inputs.churn`
```

This run reported missing setup guards, absence-only assertions and a possible
duplicate. It did not execute Kaneo's tests or verify each finding. The command
exited successfully because these findings are advisory. The hidden count includes
experimental hotspot rankings without churn data; it is not a count of confirmed
bugs.

## What it finds

The default report shows these advisory rules:

| Rule | What to inspect |
| --- | --- |
| `absence-anchor` | A test checks that something is absent without checking that the expected content rendered. |
| `dead-skip` | A test is skipped without a linked issue, or focused so other tests will not run. |
| `duplicate-test` | Test bodies and their modeled setup look equivalent. Check whether they cover different behavior before deleting either. |
| `fail-closed-setup` | Runner setup lacks guards for mock cleanup, console errors or unhandled requests. |

`check --all` also shows experimental rules and rules whose measurements failed or
were dropped. These include merge-gate checks, static flake candidates, unused
mocks and source modules ranked by churn, complexity and test strength. Use
`spirolo explain <rule>` to read a rule's claim and measured status.

### Why another test tool?

Test lint plugins check patterns in test code. Coverage reports tell you what ran.
Mutation tools such as Stryker check whether tests detect changes to production
code. Spirolo adds a view across tests, imports, setup and runner configuration,
and can read existing coverage, mutation and run reports. Keep running your tests
and using those tools: a static finding answers a different question.

The [tooling comparison](docs/landscape.md) explains that division of work.

## How far to trust it

**No rule is proven yet.** The measurements below record how often sampled findings
were judged correct, not how many bugs Spirolo can find in your project.

- **Advisory:** shown by default once a rule has at least 15 decided findings and
  measured precision of at least 0.80. These findings report at `info`.
- **Experimental:** findings appear only with `--all` unless you opt the rule in.
  The evidence is too limited or the measured precision is below the advisory floor.
- **FAIL or DROPPED:** findings appear only with `--all` and never gate, even with
  a severity override.
- **Proven:** reserved for a rule that passes the measurement bar and a fresh
  held-out check. None has done so.

Today, findings do not fail a command by default. To make a rule gate, set its
severity to `warn` or `error` in `.spirolorc.jsonc`, subject to the rule's allowed
severities. For example:

```jsonc
{ "rules": { "dead-skip": { "severity": "warn" } } }
```

[Rule tiers](docs/tiers.md) explains the thresholds, labels, exit codes and override
limits. `spirolo rule-status` prints the measurements embedded in your binary.

### Measured precision

This table is generated from the [embedded measurement data](corpus/rule-status.json),
using public repositories only. A perfect result on a small sample is not proof;
the confidence intervals show the uncertainty. These are measurements at the
recorded commit, not a new evaluation of every later change.
`n/a` means no findings were decided, so there is no precision estimate.

<!-- precision:start -->
Measured at spirolo commit `939f0af519fd50318e092700b74094b71cfeca23` across 40 OSS dev entries.
Advisory: at least 15 decided and shown precision at least 0.80.

| Rule | Tier | Status | True / decided | Precision | 95% Wilson interval | Measured entries |
| --- | --- | --- | ---: | ---: | --- | ---: |
| absence-anchor | advisory | UNADJUDICATED | 16 / 20 | 0.800 | 0.584–0.919 | 12 |
| dead-skip | advisory | INSUFFICIENT | 18 / 18 | 1.000 | 0.824–1.000 | 7 |
| duplicate-test | advisory | UNADJUDICATED | 15 / 18 | 0.833 | 0.608–0.942 | 19 |
| fail-closed-setup | advisory | UNADJUDICATED | 19 / 19 | 1.000 | 0.832–1.000 | 10 |
| flake-candidate | experimental | INSUFFICIENT | 3 / 6 | 0.500 | 0.188–0.812 | 7 |
| gate-truth | experimental | UNADJUDICATED | 12 / 18 | 0.667 | 0.437–0.837 | 8 |
| untested-hotspot | experimental | UNADJUDICATED | 5 / 16 | 0.313 | 0.142–0.556 | 40 |
| unused-mock | experimental | INSUFFICIENT | 0 / 0 | n/a | n/a | 1 |
| data-layer-mock | hidden | DROPPED | 0 / 0 | n/a | n/a | 0 |
| flake-observed | hidden | FAIL | 39 / 52 | 0.750 | 0.618–0.848 | 4 |
| generator-walk | hidden | DROPPED | 0 / 0 | n/a | n/a | 0 |
| handler-drift | hidden | DROPPED | 0 / 0 | n/a | n/a | 0 |
| orphan-test | hidden | DROPPED | 4 / 14 | 0.286 | 0.117–0.546 | 12 |
| redundant-test | hidden | DROPPED | 0 / 12 | 0.000 | 0.000–0.243 | 5 |
| stale-snapshot | hidden | DROPPED | 0 / 0 | n/a | n/a | 3 |
| weak-assertion | hidden | DROPPED | 12 / 34 | 0.353 | 0.215–0.521 | 17 |
<!-- precision:end -->

## Where it runs

The native package targets are Linux with glibc and macOS, on x64 and arm64.
Windows and Alpine/musl are outside the alpha's targets. The native binary needs
neither Node nor network access for analysis.

The measured scope is JavaScript and TypeScript React projects using Vitest or
Jest. Spirolo also reads Playwright configuration and test patterns; this does not
make every framework or configuration a measured target.

Use the CLI locally, [Markdown summaries or GitHub annotations in CI](docs/ci-reports.md),
or [MCP](docs/mcp.md) in an agent client. JSON and SARIF reports are available with
`--format json` and `--format sarif`. A VS Code extension using the same binary can
be [packaged and installed locally](docs/release.md).

## Scope

Spirolo does not run tests, launch mutation engines, collect coverage, generate tests
or host run history. It reads artifacts you already have. Missing artifacts are
reported as unverified context, not as evidence that the suite works.

A static flake candidate is not an observed flaky test. A duplicate finding is a
reason to inspect the tests, not permission to delete them. The provisional health
score is a summary of signals, not a guarantee of correctness.

Vue, Cypress, other languages and arbitrary non-React frameworks are outside the
measured scope.

## Status and limits

**Version `0.1.0-alpha.1`: advisory alpha, not yet published.** The npm packages and
VS Code extension have not been published either. See the [changelog](CHANGELOG.md)
for changes.

Static analysis can miss dynamic configuration and patterns it does not model.
If a TypeScript configuration cannot load, Spirolo reports module-resolution problems
and marks affected findings as unreliable. Install missing dependencies and resolve
the configuration warning before relying on those rows.

The human report caps displayed files and findings. `spirolo check --rule <rule> --all`
shows the full report for that rule, including experimental or dropped findings.
Review the evidence before opting a rule into CI.

Contributions are welcome: see [CONTRIBUTING.md](CONTRIBUTING.md).
For vulnerability reports, see [SECURITY.md](SECURITY.md).

## Licence

[Apache 2.0](LICENSE).
