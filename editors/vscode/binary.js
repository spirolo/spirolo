'use strict';
const { accessSync, constants } = require('node:fs');
const { join } = require('node:path');

// Prefer the same binary a project's npx spirolo uses. With no npm install, retain spirolo.path
// (a command name is resolved by the child process on PATH).
function resolveCommand(workspace, configured = 'spirolo') {
  if (workspace) {
    const local = join(workspace, 'node_modules', '.bin', process.platform === 'win32' ? 'spirolo.cmd' : 'spirolo');
    try {
      accessSync(local, constants.X_OK);
      return local;
    } catch {
      // No executable project install; use the configured command.
    }
  }
  return configured;
}

// `spirolo lsp` publishes only PASS rules' findings; spirolo.all asks for every rule's.
function serverArgs(all) {
  return all ? ['lsp', '--all'] : ['lsp'];
}
module.exports = { resolveCommand, serverArgs };
