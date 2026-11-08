import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { resolveCommand, serverArgs } from './binary.js';

test('the workspace npm binary takes priority over a configured binary', () => {
  const workspace = mkdtempSync(join(tmpdir(), 'spirolo-editor-'));
  try {
    // npm creates the real project tree and .bin entry from our actual launcher package.
    const install = spawnSync('npm', ['install', '--prefix', workspace, '--offline',
      '--ignore-scripts', '--omit=optional', '--no-audit', '--no-fund', resolve('npm/spirolo')],
      { encoding: 'utf8' });
    assert.equal(install.status, 0, install.stderr);
    const bin = join(workspace, 'node_modules', '.bin');
    assert.equal(resolveCommand(workspace, '/configured/spirolo'), join(bin, 'spirolo'));
  } finally {
    rmSync(workspace, { recursive: true, force: true });
  }
});

test('an absent npm installation retains the configured path and PATH default', () => {
  assert.equal(resolveCommand(undefined, '/configured/spirolo'), '/configured/spirolo');
  assert.equal(resolveCommand(undefined), 'spirolo');
});

test('the server runs as spirolo lsp, with --all only when spirolo.all is on', () => {
  assert.deepEqual(serverArgs(false), ['lsp']);
  assert.deepEqual(serverArgs(true), ['lsp', '--all']);
});
