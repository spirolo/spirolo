# Architecture

Spirolo is a Rust binary that builds a model of a JavaScript or TypeScript project.
Rules query that model. The CLI, language server and MCP server expose the same
analysis through different interfaces.

## Read source and reports

The source walk uses the oxc parser and resolver to read modules, imports, tests,
configuration and setup. Framework readers add facts such as assertions, mock
handlers, runner gates and snapshot ownership. Rules then combine those facts
across files.

Execution evidence comes from existing mutation and test-run reports.
Spirolo reads these files; it does not launch a test runner, Stryker or Git. If a rule
needs an input that is missing or unreadable, it may report a diagnostic or an
undecided result. Some rules have nothing to report without their required input.

Spirolo does not import coverage reports. Inspect coverage JSON or LCOV with your
coverage tool when you need to know which source lines ran.

## Main model tables

| Data | Source | Use |
| --- | --- | --- |
| Modules and imports | Source files, package metadata, resolver configuration | Import reach and test-to-source relationships |
| Tests | Framework call readers | Assertions, mocks, skips, normalized bodies and context |
| Pairings | Imports, names and directory layout | Associate tests with source |
| Handlers | Recognized MSW and nock calls | Compare patterns with configured bases |
| Gates and setup | Runner configuration, inherited presets, setup files | Check thresholds and safeguards |
| Run history | Normalized Vitest JSON and JUnit reports | Recorded outcomes, retries and duration |
| Mutants and report tests | Stryker reports | Per-test kill sets and report diagnostics |
| Churn | Supplied `git log --numstat` output | Change-weighted hotspot ranking |

The model also records clock reads, guards and module-state accesses. A fact being
recorded does not mean a rule reports it. See the [rule guides](categories.md) for
the current detectors.

## File reads and resolution

The source walk normally honors ignore rules. Module resolution also reads nearby
`tsconfig.json` or `jsconfig.json`, inherited configuration, package metadata and
filesystem paths. Resolver reads do not use the walk's ignore filtering. Supported
runner presets are followed through imports with a bounded depth.

Workspace information comes from package manifests and `pnpm-workspace.yaml`.
Install dependencies before analysis when configuration extends installed packages.
An unresolved base configuration can leave imports unknown; affected findings carry
[resolution diagnostics](ci-reports.md#when-module-resolution-cannot-load).

Dynamic imports, custom loaders, computed configuration and unsupported syntax can
leave gaps. Test-to-source pairing and import reach are static relationships, not
proof that an assertion exercises a source line.

## Local state

- `.spirolorc.jsonc` holds configuration and rule overrides.
- `.spirolo/baseline.json` records accepted finding identifiers for the ratchet.
- `.spirolo/runs/` holds normalized run reports imported by `spirolo flake ingest`.
- `.spirolo/quarantine.json` records quarantine entries for a runner integration to use.

Ingest requires a tree hash from its arguments or report sidecar. A content-derived
run identifier makes repeated ingestion of the same report a no-op. Analysis reads
the ledger back and joins report tests to the current model. A report test that
cannot be joined can still carry evidence at its file.

Quarantine updates preserve human entries and do not remove entries automatically.
Writing a quarantine file does not skip a test in the runner.

## Crates and interfaces

| Path | Responsibility |
| --- | --- |
| `crates/spirolo-model` | Walk, resolve and hold project facts |
| `crates/spirolo-frameworks` | Recognize framework syntax and configuration |
| `crates/spirolo-inputs` | Parse reports and other supplied artifacts |
| `crates/spirolo-rules` | Evaluate rules and attach evidence |
| `crates/spirolo-report` | Render findings, explanations and baseline results |
| `crates/spirolo` | CLI, LSP and MCP entry points |
| `crates/spirolo-bench` | Measurement harness |
| `npm/` | Launcher and platform packages containing native binaries |
| `editors/vscode/` | Editor client |

The npm launcher resolves the installed platform package. Editor clients use
`spirolo lsp`; MCP clients use `spirolo mcp`. These are subcommands of the binary.

## Status is part of the result

The build embeds [rule-status.json](../corpus/rule-status.json). A rule's verdict
and measured precision determine its [tier](tiers.md), default visibility and
whether a severity override can make it gate. The status table is measurement
data, not a new measurement of the project currently being analyzed.

The health score is provisional. Hidden findings can still contribute to it,
so it is not a count of visible findings or a guarantee of test quality.
