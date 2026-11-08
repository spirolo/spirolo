# spirolo

Spirolo reads a JavaScript/TypeScript test suite as a whole-project graph and reports wiring
mistakes, weak assertions, tests that do not gate a merge, duplicate tests and potential
flakes. It is one deterministic native binary: it reads runner artifacts when you provide
them, and it never runs your tests, shells out or touches the network during analysis.

## Install

Node 20 or newer:

```sh
npm install -D spirolo@alpha
npx spirolo --version
npx spirolo check
```

`check` needs no config. Install the project's own dependencies first, so its tsconfig and
module resolution can load.

## Supported platforms

This launcher picks one of these optional native packages; there is no postinstall download.

| Package | OS | CPU |
| --- | --- | --- |
| `spirolo-linux-x64` | Linux (glibc) | x64 |
| `spirolo-linux-arm64` | Linux (glibc) | arm64 |
| `spirolo-darwin-x64` | macOS | x64 |
| `spirolo-darwin-arm64` | macOS | arm64 |

Windows and Alpine/musl are not release targets.

## An advisory alpha

`0.1.0-alpha.1` is an advisory release: **no rule is proven yet.** Every rule runs, and each
finding is a signal for you to judge, labelled with how often the rule has been right on an
open-source corpus.

- `spirolo check` shows by default the rules with at least 15 decided corpus findings and a
  measured precision of at least 0.80. In this release those are **absence-anchor**,
  **dead-skip**, **duplicate-test** and **fail-closed-setup**, each heading labelled
  ` · advisory: <true>/<decided> (<STATUS>)`.
- `spirolo check --all` also shows every other rule's findings, labelled `experimental`, or
  `rule: FAIL` / `rule: DROPPED`.
- Nothing gates by default: unproven rules report at `info` and the exit code stays 0. Set a
  rule's severity to `warn` or `error` in `.spirolorc.jsonc` to make it show and gate:

```jsonc
// .spirolorc.jsonc
{ "rules": { "dead-skip": { "severity": "warn" } } }
```

`spirolo explain <rule>` states what a rule claims and how it was measured. Other output
formats: `spirolo check --format json|sarif|markdown|github-annotations|agent`.

## More

Source, documentation and the measured precision of every rule:
<https://github.com/spirolo/spirolo>. Rule tiers and overrides:
[docs/tiers.md](https://github.com/spirolo/spirolo/blob/main/docs/tiers.md). Licence: Apache 2.0.
