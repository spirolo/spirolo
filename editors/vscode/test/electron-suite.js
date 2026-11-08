// Runs inside real VS Code; reads the same public providers that render the Spirolo views.
'use strict';
const assert = require('node:assert/strict');
const vscode = require('vscode');

/** Polls `check` until it holds or 20 s pass: views fill after the first `spirolo/checked`. */
async function until(check) {
  const deadline = Date.now() + 20_000;
  while (!(await check()) && Date.now() < deadline) await new Promise((resolve) => setTimeout(resolve, 100));
  return check();
}

async function run() {
  const extension = vscode.extensions.getExtension('spirolo.spirolo');
  assert.ok(extension, 'Spirolo development extension is installed');
  const api = await extension.activate();
  const views = api?.views;
  assert.deepEqual(Object.keys(views ?? {}), ['audit', 'gate', 'hygiene', 'flake', 'health'], 'the five family views');
  const labels = async (family) => (await views[family].getChildren()).map((item) => item.label);
  assert.ok(await until(async () => (await labels('health'))[0] === 'Score'), 'Health opens on the Score row');
  if (process.env.SPIROLO_E2E_PHASE === 'persist') {
    assert.ok((await labels('gate')).includes('src'), 'folder choice survived reopening the workspace');
    return;
  }
  for (const family of ['audit', 'gate', 'hygiene', 'flake']) {
    assert.ok((await labels(family)).length > 0, `${family} view has rows`);
  }
  const score = (await views.health.getChildren())[0];
  assert.equal(views.health.getTreeItem(score).collapsibleState, vscode.TreeItemCollapsibleState.Expanded);

  const roots = await views.gate.getChildren();
  const deadSkip = roots.find((item) => item.label === 'dead-skip (1)');
  assert.ok(deadSkip, `gate rows: ${roots.map((item) => item.label)}`);
  const treeItem = views.gate.getTreeItem(deadSkip);
  assert.equal(treeItem.collapsibleState, vscode.TreeItemCollapsibleState.Collapsed);
  assert.equal(treeItem.iconPath.id, 'circle-slash');
  assert.ok(treeItem.tooltip.includes('\n'));
  const files = await views.gate.getChildren(deadSkip);
  assert.deepEqual(files.map((file) => file.label), ['src/widget.test.ts'], 'expanding a rule lists its files');
  const [row] = await views.gate.getChildren(files[0]);
  const command = views.gate.getTreeItem(row).command;
  await vscode.commands.executeCommand(command.command, ...command.arguments);
  assert.equal(vscode.window.activeTextEditor.document.uri.fsPath,
    vscode.Uri.joinPath(vscode.workspace.workspaceFolders[0].uri, files[0].open).fsPath);
  assert.equal(vscode.window.activeTextEditor.selection.active.line, Math.max(row.line - 1, 0));

  if (process.env.SPIROLO_E2E_PHASE === 'default') {
    const uri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders[0].uri, 'src/widget.test.ts');
    const document = await vscode.workspace.openTextDocument(uri);
    const edit = new vscode.WorkspaceEdit();
    edit.replace(uri, new vscode.Range(0, 0, document.lineCount, 0), document.getText().replace('toBeTruthy()', 'toBe(1)'));
    assert.ok(await vscode.workspace.applyEdit(edit));
    assert.ok(await document.save());
    // The fixed rule stays in its view, now as a clean leaf row.
    const clean = async () => (await views.audit.getChildren())
      .some((item) => item.label === 'weak-assertion' && item.description === '✓');
    assert.ok(await until(clean), 'save refreshed the family views');
  }
  if (['toggle', 'persist'].includes(process.env.SPIROLO_E2E_PHASE)) {
    await vscode.commands.executeCommand('spirolo.groupBy');
    assert.ok((await labels('gate')).includes('src'));
    assert.ok((await labels('hygiene')).includes('src'), 'by folder applies to every family view');
    await vscode.commands.executeCommand('spirolo.groupByRule');
    assert.ok((await labels('gate')).includes('dead-skip (1)'));
    await vscode.commands.executeCommand('spirolo.groupByFolder');
    assert.ok((await views.gate.getChildren()).some((item) => item.kind === 'dir'));
  }
}

module.exports = { run };
