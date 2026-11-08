# What Spirolo checks

Spirolo reads a JavaScript or TypeScript project's tests, imports and configuration.
It builds a project model and reports patterns worth inspecting. It can also read
mutation and test-run reports produced by other tools. It does not import coverage
reports, run your tests or generate mutations.

Start with `spirolo check` for the default view and `spirolo explain <rule>` for a rule's
explanation. Findings have different levels of evidence. Read [rule tiers](tiers.md)
before using them to fail CI.

## Assertions and test boundaries

`spirolo audit` checks what tests assert and which boundaries they replace with mocks.

- [absence-anchor](rules/absence-anchor.md): absence checks without a recognized render anchor.
- [weak-assertion](rules/weak-assertion.md): assertions classified as existence or truthiness checks.
- [data-layer-mock](rules/data-layer-mock.md): mocks of a declared data layer in an integration scope.
- [handler-drift](rules/handler-drift.md): handler patterns that disagree with configured API bases.
- [generator-walk](rules/generator-walk.md): assertions on individual saga steps.

## Setup and build gates

`spirolo gate` inspects runner configuration, setup and skipped or focused tests.

- [fail-closed-setup](rules/fail-closed-setup.md): missing recognized setup safeguards.
- [gate-truth](rules/gate-truth.md): settings that may not enforce the intended build policy.
- [dead-skip](rules/dead-skip.md): skip inventory, missing issue references and focused tests.

## Test maintenance

`spirolo hygiene` looks for tests or artifacts that may overlap or no longer be used.
Review the evidence before deleting anything.

- [duplicate-test](rules/duplicate-test.md): matching test bodies and recorded context.
- [redundant-test](rules/redundant-test.md): matching kill sets in mutation reports.
- [orphan-test](rules/orphan-test.md): apparent collection gaps or missing source imports.
- [stale-snapshot](rules/stale-snapshot.md): snapshot entries without a recognized owner.
- [unused-mock](rules/unused-mock.md): mocks outside the recognized import graph or unread bindings.

## Reliability

`spirolo flake` separates static candidates from recorded pass/fail changes.

- [flake-candidate](rules/flake-candidate.md): clock-sensitive assertions.
- [flake-observed](rules/flake-observed.md): pass and failure reports on the same tree hash.

Use `spirolo flake ingest` to import existing runner reports. Quarantine records are
local data; your runner must decide how to use them.

## Where to inspect next

`spirolo impact` uses changed paths and the import graph to identify related tests.
[untested-hotspot](rules/untested-hotspot.md) ranks source modules by churn,
complexity and static test reach. Neither is a substitute for execution coverage.

## Interfaces and scope

The same binary provides the CLI, `spirolo lsp` for editors, and [MCP tools](mcp.md).
[CI reports](ci-reports.md) explain Markdown and GitHub annotation output.
[Architecture](architecture.md) describes the model and its inputs.

Spirolo analyzes JavaScript and TypeScript. Unsupported syntax, dynamic configuration
and unresolved dependencies can limit the model. Read diagnostics alongside findings;
a clean report does not establish that a suite catches every regression.
