// Persistence probe in normal VS Code: extensionTestsPath forces in-memory storage.
'use strict';
const vscode = require('vscode');
const { writeFileSync } = require('node:fs');
const { run } = require('../electron-suite.js');

async function activate() {
  let result;
  try {
    await run();
    result = { pass: true };
  } catch (error) {
    result = { pass: false, error: String(error.stack ?? error) };
  }
  writeFileSync(process.env.SPIROLO_E2E_OUT, JSON.stringify(result));
  // Normal shutdown flushes the real workspaceState to disk before the second launch.
  await vscode.commands.executeCommand('workbench.action.quit');
}

module.exports = { activate };
