// The extension's only logic: `spirolo/rules`, `spirolo/health`, `spirolo/file` → view items (PRD 0006,
// "The VS Code extension" and "Amendment 2026-10-04: the fallow shape"). Pure: no vscode, no I/O,
// tested by view.test.mjs.
'use strict';

/**
 * @typedef {'error' | 'warn' | 'info' | 'ok' | 'none'} State
 * @typedef {{error: number, warn: number, info: number, hint: number}} Counts
 * @typedef {{key: string, state: State, text: string, detail: string | null, ids: string[]}} Row
 * @typedef {{file: string, kind: 'test' | 'source' | 'other', rows: Row[]}} FileAnswer
 * @typedef {{label: string, state: State, description: string, ids: string[]}} RowItem
 */

/**
 * `counts` as `1 error · 2 warn · 2 hint`, zeros left out; `''` when all are zero.
 * @param {Counts} counts
 */
function countsText(counts) {
  return /** @type {(keyof Counts)[]} */ (['error', 'warn', 'info', 'hint'])
    .filter((severity) => counts[severity] > 0)
    .map((severity) => `${counts[severity]} ${severity}`)
    .join(' · ');
}

/**
 * One rule file and its findings: the file opens its path, each finding its line; a project
 * entry (`path: ""`) opens nothing.
 * @param {RuleFile} file
 */
function fileItem(file) {
  return {
    kind: 'file',
    label: file.path || 'project',
    state: file.state,
    description: countsText(file.counts),
    open: file.path || null,
    children: file.rows.map((row) => findingItem(row, file.path)),
  };
}

/**
 * A finding row, `:<line> <message>`, opening its line; by folder it is described by its rule
 * and carries the rule's icon.
 * @param {RuleFile['rows'][number]} row
 * @param {string} path
 * @param {string} [rule]
 */
function findingItem(row, path, rule) {
  const state = /** @type {State} */ (['error', 'warn', 'info', 'info'][row.severity - 1]);
  return {
    kind: 'finding',
    label: `:${row.line} ${row.message}`,
    state,
    description: rule ?? '',
    ...(rule ? { icon: ruleIcon(rule, state) } : {}),
    ids: [row.id],
    open: path || null,
    line: row.line,
    children: [],
  };
}

/**
 * An expanded rule's children from `spirolo/rules {rule}`: its files, then their findings.
 * @param {{rules: RuleAnswer[]} | null} answer
 */
function ruleChildren(answer) {
  if (!answer) return [];
  return answer.rules.flatMap((rule) => (rule.files ?? []).map(fileItem));
}

/**
 * @typedef {{rule: string, family: string, tier: 'proven' | 'advisory' | 'unproven', status: string,
 *   summary: string, counts: Counts, state: State, files: RuleFile[] | null, file_count: number}} RuleAnswer
 * @typedef {{path: string, counts: Counts, state: State,
 *   rows: Array<{line: number, id: string, message: string, severity: number}>}} RuleFile
 */

/** @param {Counts} counts */
const total = (counts) => counts.error + counts.warn + counts.info + counts.hint;

/**
 * A family view's rows from the `spirolo/rules` summary, in the server's order: one row per rule,
 * the rule's own icon coloured by its state. A rule with findings is collapsed, `<rule> (<n>)`,
 * `lazy` because its files come from `spirolo/rules {rule}` on expand; a rule with none follows
 * as a leaf described `✓` (green icon), or `no data` (grey) when the server says `none`, so
 * every rule the family runs stays visible. The family is the answer's `family` field.
 * @param {{rules: RuleAnswer[]} | null} summary
 * @param {string} family
 */
function familyItems(summary, family) {
  if (!summary) return [];
  const rules = summary.rules.filter((rule) => rule.family === family);
  const flagged = rules
    .filter((rule) => total(rule.counts) > 0)
    .map((rule) => ({
      kind: 'rule',
      rule: rule.rule,
      label: `${rule.rule} (${total(rule.counts)})`,
      state: rule.state,
      description: rule.file_count === 1 ? '1 file' : `${rule.file_count} files`,
      tooltip: `${rule.status}\n${rule.summary}`,
      icon: ruleIcon(rule.rule, rule.state),
      lazy: true,
      open: null,
      children: [],
    }));
  const clean = rules
    .filter((rule) => total(rule.counts) === 0)
    .map((rule) => ({
      kind: 'clean',
      rule: rule.rule,
      label: rule.rule,
      state: rule.state,
      description: rule.state === 'none' ? 'no data' : '✓',
      tooltip: `${rule.state === 'none' ? 'No data' : 'No findings'}\n${rule.status}\n${rule.summary}`,
      icon: ruleIcon(rule.rule, rule.state),
      open: null,
      children: [],
    }));
  return [...flagged, ...clean];
}

/**
 * @typedef {{score: number, grade: string, formula_version: number, provisional: boolean,
 *   lean_true: number, proven: number,
 *   parts: Array<{family: string, title: string, penalty: number, cap: number, ratio: number,
 *     rules: Array<{rule: string, weight: number, flagged: number, subjects: number}>}>,
 *   facts: {tests: number, test_files: number, gates: number, expects: number, expects_per_test: number},
 *   hotspots: {total: number, shown: Array<{path: string, score: number, complexity: number, weakness: number}>}}} Health
 */

/**
 * A Health view row; `icon` null for rows that name no rule.
 * @param {string} label
 * @param {{icon?: {id: string, color: string} | null, description?: string, open?: string | null,
 *   expanded?: boolean, children?: any[]}} [rest]
 */
const healthRow = (label, rest = {}) => ({
  kind: 'health', label, state: /** @type {State} */ ('none'), description: '', icon: null, open: null, children: [], ...rest,
});

/**
 * The Health view from `spirolo/health` (ADR 0003): the Score row (the only one expanded) with
 * the grade; one row per family part with its rules'
 * `flagged/subjects ×weight`; the facts; the untested hotspots, which open their file.
 * @param {Health | null} health
 */
function healthItems(health) {
  if (!health) return [];
  return [
    healthRow('Score', {
      expanded: true,
      children: [
        healthRow(`${health.grade} (${Math.round(health.score)})`),
      ],
    }),
    ...health.parts.map((part) => healthRow(`${part.title} −${part.penalty.toFixed(1)} / ${part.cap}`, {
      children: part.rules.map((rule) => healthRow(`${rule.rule} ${rule.flagged}/${rule.subjects} ×${rule.weight}`, {
        icon: ruleIcon(rule.rule, rule.flagged > 0 ? 'info' : 'ok'),
      })),
    })),
    healthRow(`Tests ${health.facts.tests} · ${health.facts.expects_per_test.toFixed(1)} expects/test`),
    healthRow(`Untested hotspots (${health.hotspots.shown.length} of ${health.hotspots.total})`, {
      children: health.hotspots.shown.map((hotspot) => healthRow(hotspot.path, {
        icon: ruleIcon('untested-hotspot', 'info'),
        description: `score ${hotspot.score} · complexity ${hotspot.complexity} · weakness ${hotspot.weakness}`,
        open: hotspot.path,
      })),
    }),
  ];
}

/** @type {(keyof Counts)[]} */
const SEVERITIES = ['error', 'warn', 'info', 'hint'];

/**
 * The state of a sum of counts: the strongest severity, Information and Hint both `info`.
 * @param {Counts} counts
 * @returns {State}
 */
function countsState(counts) {
  if (counts.error > 0) return 'error';
  if (counts.warn > 0) return 'warn';
  return counts.info + counts.hint > 0 ? 'info' : 'ok';
}

/**
 * A family view by folder, from `spirolo/rules {family}`: every file of every rule in the family
 * once, under its directories, sorted by path, the project entry first. A file's findings
 * are its rows from all the family's rules, by line then the server's rule order, each
 * described by its rule. Directories and files carry their summed counts.
 * @param {{rules: RuleAnswer[]} | null} answer
 */
function folderItems(answer) {
  if (!answer) return [];
  /** @type {Map<string, {counts: Counts, rows: any[]}>} */
  const files = new Map();
  for (const rule of answer.rules) {
    for (const file of rule.files ?? []) {
      const entry = files.get(file.path) ?? { counts: { error: 0, warn: 0, info: 0, hint: 0 }, rows: [] };
      for (const severity of SEVERITIES) entry.counts[severity] += file.counts[severity];
      entry.rows.push(...file.rows.map((row) => findingItem(row, file.path, rule.rule)));
      files.set(file.path, entry);
    }
  }
  /** @type {Map<string, any>} */
  const dirs = new Map();
  /** @type {any[]} */
  const top = [];
  /** The directory item for `path` (`''` is the top level), made with its ancestors on first use. */
  const dir = (path) => {
    if (path === '') return null;
    let item = dirs.get(path);
    if (item) return item;
    const slash = path.lastIndexOf('/');
    item = {
      kind: 'dir', label: path.slice(slash + 1), state: 'ok', description: '',
      counts: { error: 0, warn: 0, info: 0, hint: 0 }, open: null, children: [],
    };
    const parent = dir(path.slice(0, Math.max(slash, 0)));
    (parent ? parent.children : top).push(item);
    dirs.set(path, item);
    return item;
  };
  for (const path of [...files.keys()].sort()) {
    const { counts, rows } = files.get(path);
    rows.sort((a, b) => a.line - b.line);
    const slash = path.lastIndexOf('/');
    const item = {
      kind: path ? 'file' : 'project',
      label: path ? path.slice(slash + 1) : 'project',
      state: countsState(counts),
      description: countsText(counts),
      open: path || null,
      children: rows,
    };
    const parent = path ? dir(path.slice(0, Math.max(slash, 0))) : null;
    (parent ? parent.children : top).push(item);
    for (let up = path.slice(0, Math.max(slash, 0)); up; up = up.slice(0, Math.max(up.lastIndexOf('/'), 0))) {
      const ancestor = dirs.get(up);
      for (const severity of SEVERITIES) ancestor.counts[severity] += counts[severity];
      ancestor.state = countsState(ancestor.counts);
      ancestor.description = countsText(ancestor.counts);
    }
  }
  return top;
}

/**
 * The file panel: one item per row, `<key>: <text>` with the detail as the description.
 * @param {FileAnswer | null} file
 * @returns {RowItem[]}
 */
function fileItems(file) {
  if (!file) return [];
  return file.rows.map((row) => ({
    label: `${row.key}: ${row.text}`,
    state: row.state,
    description: row.detail ?? '',
    ids: row.ids,
  }));
}

/**
 * The panel's message when it has no items, else `undefined`.
 * @param {FileAnswer | null} file
 * @returns {string | undefined}
 */
function fileMessage(file) {
  if (!file) return 'no spirolo answer for this file (before the first pass, or outside the project)';
  if (file.rows.length === 0) return `${file.file}: no spirolo findings`;
  return undefined;
}

/**
 * @typedef {'proven' | 'advisory' | 'hint'} Kind
 * @typedef {Record<Kind, number>} KindCounts
 */

/**
 * How one published diagnostic reads: `proven` when its status line is a PASS (the rule's own
 * severity), else `advisory` at Information (listed in Problems), else `hint` (sidebar only).
 * @param {number} severity the LSP code, 1 (Error) to 4 (Hint)
 * @param {string | undefined} status the diagnostic's `data.status` line
 * @returns {Kind}
 */
function diagnosticKind(severity, status) {
  if (status?.startsWith('rule: PASS')) return 'proven';
  return severity <= 3 ? 'advisory' : 'hint';
}

/**
 * The status-bar item: `spirolo: 325 findings` (what Problems shows), the hints in the tooltip; null
 * before the first pass.
 * @param {KindCounts | null} counts
 * @returns {{text: string, tooltip: string} | null}
 */
function statusBar(counts) {
  if (!counts) return null;
  const n = (count) => count.toLocaleString('en-US');
  return {
    text: `$(beaker) spirolo: ${n(counts.proven + counts.advisory)} findings`,
    tooltip: [
      `${n(counts.proven + counts.advisory)} findings in Problems`,
      `${n(counts.hint)} more in the Spirolo sidebar`,
      'Click to open the Spirolo sidebar',
    ].join('\n'),
  };
}

/** @type {Record<State, {id: string, color: string}>} */
const ICONS = {
  error: { id: 'error', color: 'problemsErrorIcon.foreground' },
  warn: { id: 'warning', color: 'problemsWarningIcon.foreground' },
  info: { id: 'info', color: 'problemsInfoIcon.foreground' },
  ok: { id: 'pass', color: 'testing.iconPassed' },
  none: { id: 'circle-slash', color: 'disabledForeground' },
};

/**
 * A state's codicon and theme colour (`none` is greyed: no data is never shown as ok).
 * @param {State} state
 */
function stateIcon(state) {
  return ICONS[state];
}

/**
 * The codicon per rule (PRD 0006, "Amendment 2026-10-04: the fallow shape"): shape names the
 * finding type.
 * @type {Record<string, string>}
 */
const RULE_ICONS = {
  'handler-drift': 'plug',
  'absence-anchor': 'eye-closed',
  'weak-assertion': 'symbol-boolean',
  'data-layer-mock': 'database',
  'generator-walk': 'references',
  'gate-truth': 'shield',
  'dead-skip': 'circle-slash',
  'fail-closed-setup': 'unlock',
  'duplicate-test': 'copy',
  'redundant-test': 'files',
  'orphan-test': 'debug-disconnect',
  'unused-mock': 'trash',
  'stale-snapshot': 'history',
  'flake-candidate': 'zap',
  'flake-observed': 'graph-line',
  'untested-hotspot': 'flame',
};

/**
 * A rule row's icon: the rule's codicon, coloured by its editor state (`none` greyed). A rule
 * the table does not know yet falls back to the state's own icon.
 * @param {string} rule
 * @param {State} state
 */
function ruleIcon(rule, state) {
  return { id: RULE_ICONS[rule] ?? ICONS[state].id, color: ICONS[state].color };
}

module.exports = { folderItems, healthItems, ruleIcon, familyItems, ruleChildren, fileItems, fileMessage, stateIcon, diagnosticKind, statusBar };
