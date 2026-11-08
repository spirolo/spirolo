# Changelog

## 0.1.0-alpha.1 (2026-11-08, advisory alpha)

The first public release. Spirolo analyzes JavaScript and TypeScript test suites
without running them. No rule is proven yet, and findings do not fail a command
unless you opt a supported rule into gating.

```sh
npm install -D spirolo@alpha
npx spirolo check
```

### What ships

- A native CLI that reads tests, source code and runner configuration together.
  Reports are available as terminal text, JSON, SARIF, Markdown and GitHub
  annotations. Findings carry the rule's measured status.
- `spirolo explain <rule>` describes the rule and its evidence. `spirolo rule-status`
  prints the measurement table embedded in the binary.
- An LSP server and `spirolo mcp` for tools that use the Model Context Protocol.
- The npm launcher `spirolo` and native packages `spirolo-<platform>` for Linux
  with glibc and macOS, on x64 and arm64. The launcher selects the native package
  without a postinstall download. The packages are published with npm provenance,
  under the `alpha` tag: plain `npm install spirolo` does not install the alpha.
- The same binaries as archives on the GitHub release, with `SHA256SUMS`.

Windows and Alpine/musl are not release targets. The VS Code extension is built
from this repository but not yet published to the Marketplace.

### Default rules

The default report shows advisory findings from these rules:

| Rule | What to inspect |
| --- | --- |
| `absence-anchor` | An assertion checks that something is absent without checking that expected content rendered. |
| `dead-skip` | A skipped test lacks a linked issue, or a focused test prevents other tests from running. |
| `duplicate-test` | Test bodies and their modeled setup look equivalent. Review their purpose before removing either. |
| `fail-closed-setup` | Runner setup lacks guards for mock cleanup, console errors or unhandled requests. |

Advisory rules have at least 15 decided findings and measured precision of at
least 0.80. That evidence does not prove a rule or predict its accuracy on your
project. Findings default to `info`. A `warn` or `error` override in
`.spirolorc.jsonc` makes an advisory or experimental rule visible and lets its
findings fail the command, subject to the rule's allowed severities.

### Experimental and hidden rules

`flake-candidate`, `gate-truth`, `untested-hotspot` and `unused-mock` are
experimental. Their evidence is too limited or their measured precision is below
the advisory threshold. Their findings appear with `spirolo check --all`, or when
an allowed severity override opts them in.

`flake-observed` has status `FAIL`: it can combine reports from different runner
configurations and mistake a load failure in one for a flaky test in another.
`data-layer-mock`, `generator-walk`, `handler-drift`, `orphan-test`,
`redundant-test`, `stale-snapshot` and `weak-assertion` have status `DROPPED`.
These rules are hidden by default. `--all` reveals their findings, but they
remain informational and never gate, even with a severity override.

The same default filtering applies across report formats, family commands, MCP
and LSP. `--all` changes visibility, not severity. See [rule tiers](docs/tiers.md)
for the policy and override examples.

### Other limitations

- The health grade is provisional and includes experimental and hidden findings.
  It is not a count of confirmed bugs.
- Missing dependencies or an unreadable TypeScript configuration can prevent
  module resolution. The report warns about the failure and marks affected
  findings unreliable; that warning does not change the exit code.
- `explain` links to rule write-ups by repository-relative path.

## 0.1.0-rc.1 (unpublished release candidate)

- Staged Linux and macOS native packages with an npm launcher that uses optional
  platform dependencies and no postinstall download.
- Packaged the VS Code extension as a VSIX, preferring the workspace's npm binary
  before falling back to `spirolo.path` or PATH.
- Added `spirolo mcp`, Markdown summaries and capped GitHub annotations.
- Embedded rule measurements and generated the README status table from them.
- Made unproven rules informational by default, with explicit severity overrides
  for projects that choose to gate on their findings.

This candidate was rehearsed locally and never published.
