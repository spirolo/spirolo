# Spirolo for VS Code

A thin client over `spirolo lsp`. The server publishes spirolo's findings as diagnostics,
answers hover with each finding's explain text and its rule's measured status, and re-runs the
whole `check` pass on every save. The extension adds six views in an "Spirolo" activity-bar
container, one per question, each folding, refreshing and hiding on its own:

- **Weak tests** (`audit`), **Merge gate** (`gate`), **Unused tests** (`hygiene`) and **Flaky**
  (`flake`), the family views. Each lists, from the `spirolo/rules` summary, one collapsed row
  per rule with findings: the rule's own icon (for example `circle-slash` for `dead-skip`,
  `zap` for `flake-candidate`) coloured by its editor state, the label `<rule> (<findings>)`,
  the description `<files> files`, and the measured status and summary as tooltip.
  Expanding a rule asks `spirolo/rules {rule}` for its files; a file opens its path and expands
  to its findings, which open their line. A `project` entry carries findings on no file and
  opens nothing. Rules with no findings follow as their own rows, so every rule the view runs
  stays visible: the rule's icon in green and the description `✓`, or greyed with `no data`
  when the server has nothing to judge it on. A rule's family is read from the server's answer.
- **Health**: the provisional score from `spirolo/health`. The `Score` row is open and shows
  `<grade> (<score>)`. Then one collapsed row per family
  part, `<title> −<penalty> / <cap>`, expanding to each rule's `<flagged>/<subjects>
  ×<weight>`; a `Tests <n> · <x> expects/test` row; and `Untested hotspots (<shown> of
  <total>)`, whose files (the top 200) open on click.
- **File**: `spirolo/file` for the active editor, one item per row (covers, handlers, each firing
  rule, flake, mutation, hygiene for a test file) with a state icon and the row's detail. A
  greyed row is `none`: its input (coverage, handlers, a mutation report) was never provided.

Every family view's title bar has a **By folder** button: the view then asks `spirolo/rules
{family}` and shows that family's files under their directories, each finding described by
its rule. **By rule** switches back. The choice applies to all four family views and persists
for this workspace; the Command Palette's **Spirolo: Toggle grouping** does the same. Every view
refreshes on `spirolo/checked` and starts collapsed, except the Score row.

## What the server answers

Besides diagnostics and hover, `spirolo lsp` answers these requests over the last good pass (each
is `null` before one) and sends `spirolo/checked` after every pass:

- `spirolo/rules` with no params: the summary, every rule of every family (empty ones included)
  as `{rule, family, tier, status, summary, counts, state, files: null, file_count}`, ordered by
  family, then tier, strongest editor state and registry order. `status` is the bare status
  line, without the measured corpus list.
- `spirolo/rules {family}`: that family's rules with `files` filled in (`{path, counts, state,
  rows}`, each row `{line, id, message, severity}`). `spirolo/rules {rule}`: that one rule, the
  same way. An unknown family or rule, or both at once, is an `InvalidParams` error. The
  family views ask the summary, then `{rule}` on expand, or `{family}` by folder.
- `spirolo/health`: the provisional score (formula v1) as `{score, grade,
  formula_version, provisional, lean_true, proven, parts, facts, hotspots}`.
- `spirolo/tree` and `spirolo/file`: tree and active-file details; the File view reads `spirolo/file`.

Editor severity is capped by each rule's measured status: only a proven rule shows as Warning or
Error. An `info` row of a rule the default view shows (proven or advisory) is Information; an
`info` row of a rule shown only under `spirolo lsp --all` (experimental, FAIL, DROPPED) is a Hint.
See [rule tiers](../../docs/tiers.md) for the visibility and severity policy.

## Install a packaged extension

If you have a packaged VSIX, install it with `code --install-extension /path/to/spirolo.vsix`,
using the path to your artifact. See the
[VS Code instructions](https://code.visualstudio.com/docs/configure/extensions/extension-marketplace#_install-from-a-vsix)
for installing a local VSIX.

## Run it from a checkout

From the repository root:

```sh
cargo build --release -p spirolo          # or put an `spirolo` binary on PATH
(cd editors/vscode && npm ci)           # locked runtime and packaging dependencies
code --extensionDevelopmentPath="$PWD/editors/vscode" <project>
```

The extension activates in a workspace containing `.spirolorc.jsonc`, `vitest.config.*`,
`jest.config.*` or `playwright.config.*`. It first runs the workspace's `node_modules/.bin/spirolo`. If that is absent,
`spirolo.path` names the binary (default: the `spirolo` command on PATH), for example `"spirolo.path": "/path/to/spirolo/target/release/spirolo"`.
The server starts in the first workspace folder; a project with no `.spirolorc.jsonc` is checked
exactly as `spirolo check` would check it. Like `spirolo check`, it shows only the findings of rules
the default view shows (proven and advisory, see [docs/tiers.md](../../docs/tiers.md));
`"spirolo.all": true` shows every rule's (`spirolo lsp --all`).

## Tests

`view.js` maps answers to view items. `view.test.mjs` runs it against
canned `spirolo/rules`, `spirolo/health` and `spirolo/file` answers with plain `node --test`, no VS
Code and no `node_modules`:

```sh
node --test editors/vscode/view.test.mjs
```

The canned answers in `test/*.json` are real. `test/families.json` was captured from the real
server on `test/workspace` by `test/capture-families.mjs`: the summary, every `{family}` and
`{rule}` answer, and `spirolo/health`. `test/capture.mjs` drove `spirolo lsp` on small temp fixture
projects for the `spirolo/file` answers. Each file's `_capture` says how it was taken; re-take
them the same way when the server's answers change, using a built binary.

The view check above is self-contained; it needs neither a packaged extension nor a built
Spirolo binary. The checks below need the extension's npm dependencies or a built binary.

`binary.test.mjs` installs the real local launcher package with npm offline and checks binary
selection and fallback:

```sh
node --test editors/vscode/binary.test.mjs
```

The Electron check starts real VS Code, checks that the five family views appear and that
Health opens on the Score row, expands a rule to list its files, opens a finding at its line,
saves a test to check refresh, toggles every family view to folders and back, then reopens the
same workspace to check persistence. It reads the public providers used by the views; it mocks
neither VS Code nor the LSP. After `npm ci` in this directory, run:

```sh
xvfb-run -a npm --prefix editors/vscode run test:electron   # Linux
npm --prefix editors/vscode run test:electron               # macOS
```

VS Code 1.96.4 is downloaded into `target/vscode-test` by default; `SPIROLO_VSCODE_EXECUTABLE`
can select an already installed executable, and `SPIROLO_TEST_BINARY` can select another spirolo
binary. No test runner is executed in the fixture workspace.
