// view.js against canned answers taken from real `spirolo lsp` runs: `spirolo/rules` and
// `spirolo/health` on test/workspace (test/capture-families.mjs), `spirolo/file` on two temp fixture
// projects (test/capture.mjs); provenance in each JSON's `_capture`.
// Runs under `node --test` without vscode or node_modules.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import view from './view.js';

const load = (name) => JSON.parse(readFileSync(new URL(`./test/${name}.json`, import.meta.url)));
const fixture = load('fixture');
const bare = load('bare');
const families = load('families');

test('a family view lists its rules with findings as collapsed rows: count, files and status tooltip', () => {
  const rows = view.familyItems(families.summary, 'gate').filter((item) => item.kind === 'rule');
  assert.deepEqual(rows.map((item) => [item.label, item.description, item.state]), [
    ['fail-closed-setup (2)', '1 file', 'info'],
    ['dead-skip (1)', '1 file', 'info'],
    ['gate-truth (1)', '1 file', 'info'],
  ]);
  for (const item of rows) {
    const rule = families.summary.rules.find((r) => r.rule === item.rule);
    assert.equal(rule.family, 'gate');
    assert.equal(item.tooltip, `${rule.status}\n${rule.summary}`);
    assert.equal(item.lazy, true);
    assert.deepEqual(item.icon, view.ruleIcon(rule.rule, rule.state));
  }
  assert.deepEqual(view.familyItems(null, 'gate'), []);
});

test('rules with no findings follow as leaf rows: their own icon, green, described ✓', () => {
  const audit = view.familyItems(families.summary, 'audit');
  assert.deepEqual(audit.map((item) => [item.label, item.description]), [
    ['weak-assertion (2)', '2 files'],
    ['absence-anchor', '✓'],
    ['handler-drift', '✓'],
    ['generator-walk', '✓'],
    ['data-layer-mock', '✓'],
  ]);
  for (const item of audit.slice(1)) {
    const rule = families.summary.rules.find((r) => r.rule === item.rule);
    assert.equal(item.kind, 'clean');
    assert.equal(item.state, 'ok');
    assert.deepEqual(item.icon, view.ruleIcon(item.rule, 'ok'));
    assert.equal(item.tooltip, `No findings\n${rule.status}\n${rule.summary}`);
    assert.equal(item.lazy, undefined);
    assert.deepEqual(item.children, []);
  }
  const flake = view.familyItems(families.summary, 'flake');
  assert.deepEqual(flake.map((item) => [item.label, item.description]), [['flake-candidate', '✓'], ['flake-observed', '✓']]);
});

test('a rule with no findings and no data is greyed and says so, never ✓', () => {
  const rule = { ...families.summary.rules.find((r) => r.rule === 'flake-observed'), state: 'none' };
  const [item] = view.familyItems({ rules: [rule] }, 'flake');
  assert.equal(item.description, 'no data');
  assert.deepEqual(item.icon, view.ruleIcon('flake-observed', 'none'));
  assert.match(item.tooltip, /^No data\n/);
});

test('an expanded rule lists its files from spirolo/rules {rule}, each file its findings that open their line', () => {
  const answer = families.rule['weak-assertion'];
  assert.deepEqual(outline(view.ruleChildren(answer)), [
    'project',
    '  :0 1 of 2 assertions carry no value (50.0%, ceiling 6.0%)',
    'src/widget.test.ts',
    '  :4 test "renders the widget" asserts only value-less matchers (toBeTruthy)',
  ]);
  const [project, file] = view.ruleChildren(answer);
  assert.equal(project.open, null);
  assert.equal(project.children[0].open, null);
  assert.deepEqual([file.kind, file.open, file.state, file.description], ['file', 'src/widget.test.ts', 'info', '1 info']);
  assert.deepEqual([file.children[0].open, file.children[0].line, file.children[0].ids],
    ['src/widget.test.ts', 4, [answer.rules[0].files[1].rows[0].id]]);
  assert.equal(view.ruleChildren(families.rule['gate-truth'])[0].children[0].state, 'info');
  assert.deepEqual(view.ruleChildren(null), []);
});

test('each of the 16 rules in the summary has its own codicon, coloured by its editor state', () => {
  // PRD 0006, "Amendment 2026-10-04: the fallow shape", per-rule icon table.
  const table = {
    'handler-drift': 'plug', 'absence-anchor': 'eye-closed', 'weak-assertion': 'symbol-boolean',
    'data-layer-mock': 'database', 'generator-walk': 'references', 'gate-truth': 'shield',
    'dead-skip': 'circle-slash', 'fail-closed-setup': 'unlock', 'duplicate-test': 'copy',
    'redundant-test': 'files', 'orphan-test': 'debug-disconnect', 'unused-mock': 'trash',
    'stale-snapshot': 'history', 'flake-candidate': 'zap', 'flake-observed': 'graph-line',
    'untested-hotspot': 'flame',
  };
  assert.equal(families.summary.rules.length, 16);
  for (const rule of families.summary.rules) {
    assert.deepEqual(view.ruleIcon(rule.rule, rule.state), { id: table[rule.rule], color: view.stateIcon(rule.state).color }, rule.rule);
  }
  assert.deepEqual(view.ruleIcon('dead-skip', 'error'), { id: 'circle-slash', color: 'problemsErrorIcon.foreground' });
  assert.deepEqual(view.ruleIcon('dead-skip', 'warn'), { id: 'circle-slash', color: 'problemsWarningIcon.foreground' });
  assert.deepEqual(view.ruleIcon('dead-skip', 'none'), { id: 'circle-slash', color: 'disabledForeground' });
});

test('the Health view opens on the score, then one collapsed row per family part', () => {
  const items = view.healthItems(families.health);
  assert.deepEqual(outline(items), [
    'Score',
    '  D (45)',
    'Weak tests −25.0 / 25',
    '  handler-drift 0/2 ×0.5',
    '  generator-walk 0/2 ×0.5',
    '  absence-anchor 0/2 ×1',
    '  weak-assertion 1/2 ×1',
    '  data-layer-mock 0/2 ×0.5',
    'Merge gate −30.0 / 30',
    '  fail-closed-setup 1/1 ×1',
    '  gate-truth 1/1 ×0.5',
    '  dead-skip 1/2 ×1',
    'Unused tests −0.0 / 10',
    '  duplicate-test 0/2 ×0.5',
    '  redundant-test 0/2 ×0.5',
    '  unused-mock 0/1 ×0.5',
    '  orphan-test 0/1 ×1',
    '  stale-snapshot 0/1 ×0.5',
    'Flaky −0.0 / 20',
    '  flake-candidate 0/2 ×0.5',
    '  flake-observed 0/0 ×0.5',
    'Health −0.0 / 15',
    '  untested-hotspot 0/1 ×0.5',
    'Tests 2 · 1.0 expects/test',
    'Untested hotspots (1 of 1)',
    '  src/widget.ts',
  ]);
  assert.deepEqual(items.map((item) => item.expanded ?? false), [true, false, false, false, false, false, false, false]);
  const gate = items.find((item) => item.label.startsWith('Merge gate'));
  assert.deepEqual(gate.children[1].icon, view.ruleIcon('gate-truth', 'info'));
  assert.deepEqual(items[3].children[0].icon, view.ruleIcon('duplicate-test', 'ok'));
  const hotspot = items.at(-1).children[0];
  assert.deepEqual([hotspot.open, hotspot.description], ['src/widget.ts', 'score 0.2 · complexity 2 · weakness 0.1']);
  assert.deepEqual(hotspot.icon, view.ruleIcon('untested-hotspot', 'info'));
  assert.deepEqual(view.healthItems(null), []);
});

test('by folder builds a family view\'s folder tree from that family\'s file paths', () => {
  const gate = view.folderItems(families.family.gate);
  assert.deepEqual(outline(gate), [
    'src',
    '  widget.test.ts',
    '    :5 skip "later" has no linked issue',
    'vitest.config.ts',
    `  :0 ${families.family.gate.rules[0].files[0].rows[0].message}`,
    `  :0 ${families.family.gate.rules[0].files[0].rows[1].message}`,
    `  :0 ${families.family.gate.rules[2].files[0].rows[0].message}`,
  ]);
  assert.deepEqual(flat(gate).map((item) => [item.kind, item.state, item.description, item.open]), [
    ['dir', 'info', '1 info', null],
    ['file', 'info', '1 info', 'src/widget.test.ts'],
    ['finding', 'info', 'dead-skip', 'src/widget.test.ts'],
    ['file', 'info', '2 info · 1 hint', 'vitest.config.ts'],
    ['finding', 'info', 'fail-closed-setup', 'vitest.config.ts'],
    ['finding', 'info', 'fail-closed-setup', 'vitest.config.ts'],
    ['finding', 'info', 'gate-truth', 'vitest.config.ts'],
  ]);
  assert.deepEqual(gate[1].children[2].icon, view.ruleIcon('gate-truth', 'info'));
  assert.equal(gate[0].children[0].children[0].line, 5);
  assert.deepEqual(outline(view.folderItems(families.family.audit)).filter((line) => !line.includes(':')), ['project', 'src', '  widget.test.ts']);
  assert.equal(view.folderItems(families.family.audit)[0].open, null);
  assert.deepEqual(view.folderItems(families.family.flake), []);
  assert.deepEqual(view.folderItems(null), []);
});

/** A forest's items, depth first. */
const flat = (items) => items.flatMap((item) => [item, ...flat(item.children)]);

/** The labels of a forest, depth first, indented by depth. */
const outline = (items, depth = 0) =>
  items.flatMap((item) => [`${'  '.repeat(depth)}${item.label}`, ...outline(item.children, depth + 1)]);

/** A panel's items as `[label, state, description]`. */
const panel = (file) => view.fileItems(file).map((item) => [item.label, item.state, item.description]);

test('a test file panel is one item per row, detail as the description', () => {
  assert.deepEqual(panel(fixture.file['src/widget.test.ts']), [
    ['covers: src/widget.ts', 'ok', 'no coverage report read'],
    ['handlers: 1 handlers in the project · no drift in this file', 'ok', 'rule: SKIPPED'],
    [
      'weak-assertion: 1 × line 4: test "renders the widget" asserts only value-less matchers (toBeTruthy)',
      'warn',
      'rule: PASS 1.000 [0.942, 1.000]',
    ],
    ['dead-skip: 1 × line 5: skip "later" has no linked issue', 'error', 'rule: PASS 1.000 [0.845, 1.000]'],
    ['flake: no static candidates', 'ok', 'no runs ingested'],
    ['mutation: no mutation report read', 'none', ''],
    ['hygiene: unused-mock ×1', 'info', ''],
  ]);
});

test('rows with no input say none, and an empty panel says why it is empty', () => {
  assert.deepEqual(panel(bare.file['test/math.test.ts']).slice(0, 2), [
    ['covers: no paired source', 'none', 'no coverage report read'],
    ['handlers: no handlers in the project', 'none', 'rule: SKIPPED'],
  ]);
  assert.equal(view.fileMessage(bare.file['test/math.test.ts']), undefined);
  assert.deepEqual(panel(bare.file['README.md']), []);
  assert.equal(view.fileMessage(bare.file['README.md']), 'README.md: no spirolo findings');
  assert.deepEqual(panel(null), []);
  assert.equal(view.fileMessage(null), 'no spirolo answer for this file (before the first pass, or outside the project)');
});

test('every state the answers carry has its own icon', () => {
  const answers = [...Object.values(fixture.file), ...Object.values(bare.file)];
  const states = [...new Set(answers.flatMap((file) => view.fileItems(file).map((item) => item.state)))].sort();
  assert.deepEqual(states, ['error', 'info', 'none', 'ok', 'warn']);
  assert.deepEqual(Object.fromEntries(states.map((state) => [state, view.stateIcon(state)])), {
    error: { id: 'error', color: 'problemsErrorIcon.foreground' },
    info: { id: 'info', color: 'problemsInfoIcon.foreground' },
    none: { id: 'circle-slash', color: 'disabledForeground' },
    ok: { id: 'pass', color: 'testing.iconPassed' },
    warn: { id: 'warning', color: 'problemsWarningIcon.foreground' },
  });
});

test('a diagnostic is proven only on a PASS status line, else advisory at Information and above', () => {
  // Status lines in PRD 0006's forms, as `data.status` carries them.
  assert.equal(view.diagnosticKind(3, 'rule: PASS 0.980 [0.930, 0.990]'), 'proven');
  assert.equal(view.diagnosticKind(1, 'rule: PASS 0.980 [0.930, 0.990]'), 'proven');
  assert.equal(view.diagnosticKind(3, 'rule: UNADJUDICATED'), 'advisory');
  assert.equal(view.diagnosticKind(3, 'rule: FAIL 0.750 [0.551, 0.880], carried to M6'), 'advisory');
  assert.equal(view.diagnosticKind(4, 'rule: INSUFFICIENT, carried to M6'), 'hint');
  assert.equal(view.diagnosticKind(4, undefined), 'hint');
});

test('the status bar counts what Problems shows, the sidebar-only rest in the tooltip', () => {
  const item = view.statusBar({ proven: 3, advisory: 322, hint: 4290 });
  assert.equal(item.text, '$(beaker) spirolo: 325 findings');
  assert.match(item.tooltip, /^325 findings in Problems\n/);
  assert.match(item.tooltip, /\n4,290 more in the Spirolo sidebar\n/);
  assert.match(item.tooltip, /Click to open the Spirolo sidebar$/);
  assert.equal(view.statusBar(null), null);
});
