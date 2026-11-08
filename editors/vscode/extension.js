// A thin client over `spirolo lsp` (PRD 0006, "The VS Code extension" and "Amendment 2026-10-04:
// the fallow shape"): the server publishes the diagnostics and hovers; this file starts it,
// forwards file creations and deletions, renders one view per family from `spirolo/rules` (by
// rule, or by folder), "Health" from `spirolo/health` and "File" from `spirolo/file` through
// view.js, and keeps a status-bar count of proven and advisory diagnostics that opens the sidebar.
'use strict';

const vscode = require('vscode');
const { LanguageClient } = require('vscode-languageclient/node');
const view = require('./view.js');
const { resolveCommand, serverArgs } = require('./binary.js');
const { resolve } = require('node:path');

/** @type {LanguageClient | undefined} */
let client;

/**
 * A view over view.js items. A `lazy` item loads its children through `load` on first expand;
 * an item with `icon: null` shows none; `expanded` items start open, every other one collapsed.
 * @template {{label: string, state: import('./view.js').State, description: string,
 *   children?: any[], open?: string | null, ids?: string[], line?: number,
 *   tooltip?: string, kind?: string, lazy?: boolean, expanded?: boolean,
 *   icon?: {id: string, color: string} | null}} T
 * @implements {vscode.TreeDataProvider<T>}
 */
class Items {
  /**
   * @param {string} root
   * @param {(item: T) => Promise<T[]>} [load]
   */
  constructor(root = '', load = async () => []) {
    this.root = root;
    this.load = load;
    /** @type {T[]} */
    this.items = [];
    this.changed = new vscode.EventEmitter();
    this.onDidChangeTreeData = this.changed.event;
  }

  /** @param {T[]} items */
  set(items) {
    this.items = items;
    this.changed.fire(undefined);
  }

  /** @param {T | undefined} item */
  async getChildren(item) {
    if (!item) return this.items;
    if (item.lazy) {
      item.children = await this.load(item);
      item.lazy = false;
    }
    return item.children ?? [];
  }

  /** @param {T} item */
  getTreeItem(item) {
    const collapsible = item.expanded
      ? vscode.TreeItemCollapsibleState.Expanded
      : item.lazy || item.children?.length
        ? vscode.TreeItemCollapsibleState.Collapsed
        : vscode.TreeItemCollapsibleState.None;
    const tree = new vscode.TreeItem(item.label, collapsible);
    const icon = item.icon === undefined ? view.stateIcon(item.state) : item.icon;
    if (icon) tree.iconPath = new vscode.ThemeIcon(icon.id, new vscode.ThemeColor(icon.color));
    tree.description = item.description;
    tree.tooltip = item.tooltip ?? [item.label, item.description, ...(item.ids ?? [])].filter(Boolean).join('\n');
    if (item.open) {
      const uri = vscode.Uri.file(resolve(this.root, item.open));
      const line = Math.max((item.line ?? 1) - 1, 0);
      const args = item.line === undefined ? [uri] : [uri, { selection: new vscode.Range(line, 0, line, 0) }];
      tree.command = { command: 'vscode.open', title: 'Open', arguments: args };
    }
    return tree;
  }
}

/** @param {vscode.ExtensionContext} context */
async function activate(context) {
  const folder = vscode.workspace.workspaceFolders?.[0];
  let groupBy = context.workspaceState.get('spirolo.groupBy') === 'folder' ? 'folder' : 'rule';
  await vscode.commands.executeCommand('setContext', 'spirolo.groupBy', groupBy);
  /** Each published file's diagnostics by kind, keyed by URI. */
  /** @type {Map<string, import('./view.js').KindCounts>} */
  const kinds = new Map();
  let passed = false;
  const settings = vscode.workspace.getConfiguration('spirolo');
  const command = resolveCommand(folder?.uri.fsPath, settings.get('path', 'spirolo'));
  client = new LanguageClient(
    'spirolo',
    'Spirolo',
    { command, args: serverArgs(settings.get('all', false)), options: { cwd: folder?.uri.fsPath } },
    {
      // Every file: a save of a handler, a config or a source module re-runs the pass too.
      documentSelector: [{ scheme: 'file' }],
      // Created and deleted files only; changes to open files arrive as saves.
      synchronize: { fileEvents: vscode.workspace.createFileSystemWatcher('**/*', false, true, false) },
      middleware: {
        handleDiagnostics(uri, diagnostics, next) {
          const counts = { proven: 0, advisory: 0, hint: 0 };
          for (const diagnostic of diagnostics) {
            // vscode's severity is the LSP code minus one.
            const kind = view.diagnosticKind(diagnostic.severity + 1, /** @type {any} */ (diagnostic).data?.status);
            counts[kind] += 1;
          }
          kinds.set(uri.toString(), counts);
          next(uri, diagnostics);
        },
      },
    },
  );

  // The family views are the manifest's `spirolo.<family>` views; Health and File are their own.
  const families = context.extension.packageJSON.contributes.views.spirolo
    .map((/** @type {{id: string}} */ v) => v.id.slice('spirolo.'.length))
    .filter((/** @type {string} */ family) => family !== 'health' && family !== 'file');
  const loadRule = async (/** @type {any} */ item) => view.ruleChildren(await client?.sendRequest('spirolo/rules', { rule: item.rule }));
  /** @type {Record<string, Items<any>>} */
  const views = {};
  /** @type {Record<string, vscode.TreeView<any>>} */
  const treeViews = {};
  for (const family of [...families, 'health']) {
    views[family] = new Items(folder?.uri.fsPath, loadRule);
    treeViews[family] = vscode.window.createTreeView(`spirolo.${family}`, { treeDataProvider: views[family] });
  }
  const file = new Items();
  const fileView = vscode.window.createTreeView('spirolo.file', { treeDataProvider: file });
  const bar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left);
  bar.command = 'workbench.view.extension.spirolo';
  context.subscriptions.push(...Object.values(treeViews), fileView, bar);

  const refreshBar = () => {
    const counts = { proven: 0, advisory: 0, hint: 0 };
    for (const file of kinds.values()) {
      for (const kind of /** @type {const} */ (['proven', 'advisory', 'hint'])) counts[kind] += file[kind];
    }
    const item = view.statusBar(passed ? counts : null);
    if (!item) return bar.hide();
    bar.text = item.text;
    bar.tooltip = item.tooltip;
    bar.show();
  };

  const refreshFamilies = async () => {
    const mode = groupBy;
    const summary = await client?.sendRequest('spirolo/rules');
    const items = await Promise.all(families.map(async (family) =>
      mode === 'rule' ? view.familyItems(summary, family) : view.folderItems(summary ? await client?.sendRequest('spirolo/rules', { family }) : null)));
    if (mode !== groupBy) return;
    families.forEach((family, index) => {
      views[family].set(items[index]);
    });
  };
  const refreshHealth = async () => {
    views.health.set(view.healthItems(await client?.sendRequest('spirolo/health')));
  };
  const setGrouping = async (mode) => {
    groupBy = mode;
    await context.workspaceState.update('spirolo.groupBy', mode);
    await vscode.commands.executeCommand('setContext', 'spirolo.groupBy', mode);
    await refreshFamilies();
  };
  const refreshFile = async () => {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.uri.scheme !== 'file') {
      file.set([]);
      fileView.message = 'no file open';
      return;
    }
    const answer = await client?.sendRequest('spirolo/file', { uri: editor.document.uri.toString() });
    file.set(view.fileItems(answer));
    fileView.message = view.fileMessage(answer);
  };

  const refresh = async () => {
    refreshBar();
    await Promise.all([refreshFamilies(), refreshHealth(), refreshFile()]);
  };

  await client.start();
  context.subscriptions.push(
    vscode.commands.registerCommand('spirolo.groupBy', () => setGrouping(groupBy === 'rule' ? 'folder' : 'rule')),
    vscode.commands.registerCommand('spirolo.groupByRule', () => setGrouping('rule')),
    vscode.commands.registerCommand('spirolo.groupByFolder', () => setGrouping('folder')),
    client.onNotification('spirolo/checked', () => {
      passed = true;
      return refresh();
    }),
    vscode.window.onDidChangeActiveTextEditor(refreshFile),
  );
  await refresh();
  // Public providers also let the Electron integration check read the rendered views.
  return { views };
}

function deactivate() {
  return client?.stop();
}

module.exports = { activate, deactivate };
