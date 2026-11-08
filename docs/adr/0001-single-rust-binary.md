# ADR 0001: One Rust binary that owns its analysis, not an orchestrator

**Date** 2026-09-13 · **Status** accepted

## Context

Three approaches were considered for analyzing test quality: assemble existing tools,
write a TypeScript orchestrator that wraps them, or build a Rust analyzer on oxc that
owns the graph. The chosen approach keeps the project model in one process, shared
by every rule and interface. See the [architecture](../architecture.md).

## Decision

Build a single Rust binary on oxc that computes its own whole-project
model of the test suite and derives every finding from that model.

Concretely:

- **Own the graph.** Parsing, alias resolution, test discovery, test↔source pairing,
  export tables and constant folding all run in-process, with no Node runtime or TypeScript
  compiler.
- **No wrapping.** Nothing that shells out to Stryker, ESLint, Trunk or a runner is in
  the core. Where a shelf tool already answers a question (per-file lint, mutation
  engine, hosted flake analytics), the binary *reads their artifacts* if useful and
  otherwise defers by name in the docs.
- **Static first, runtime later and optional.** Anything that must execute the
  project's tests (assertion negation, handler-match reporting) is a separate,
  later layer. The static binary must be complete and
  valuable alone.
- **Few dependencies, all deliberate.** oxc crates, an ignore-aware walker, a JSON
  serializer, a CLI parser. Every added crate is justified in its PR.
- **Deterministic.** Same tree → same findings, same ids, same exit code. No sampling,
  no ML, no network.

## Consequences

- The `mutate` family (aim Stryker, read it back) leaves the core. Reading a completed
  Stryker report as an *input* for hygiene/verification-gap stays possible; launching
  it does not.
- Runtime probing remains an optional later layer.
- `flake` keeps its static half (candidate patterns) and its ledger half (ingesting
  JUnit / Vitest JSON files the runner wrote, using file parsing without execution);
  it drops exporters to vendors until someone asks.
- LSP, MCP and SARIF are shells over the model, built after precision is proven, from
  the same binary.
- Building the graph takes longer than wrapping existing tools. That cost is accepted
  because the rules need a shared model.

## Rejected

- **TypeScript orchestrator.** Fastest to validate rules, but the graph gets rewritten
  in Rust for the intended editor integration, duplicating the implementation.
- **Multi-language via tree-sitter.** Would trade the precision the whole thesis rests on
  for breadth. Revisit only after a corpus proves precision on JS/TS.
- **Hosting anything.** Ledgers live in the repo. No cloud, no accounts.
