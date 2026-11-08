import assert from 'node:assert/strict';
import { test } from 'node:test';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync, readdirSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { createRequire } from 'node:module';
import { packageName } from './spirolo/resolve.cjs';

test('Linux x64 resolves the linux-x64 optional package', () => {
  assert.equal(packageName('linux', 'x64'), 'spirolo-linux-x64');
});

for (const [platform, arch] of [['linux', 'arm64'], ['darwin', 'x64'], ['darwin', 'arm64']]) {
  test(`${platform} ${arch} resolves its optional platform package`, () => {
    assert.equal(packageName(platform, arch), `spirolo-${platform}-${arch}`);
  });
}

test('unsupported targets report which platform is unavailable', () => {
  assert.throws(() => packageName('win32', 'x64'), /win32-x64/);
});

test('a missing optional binary explains how to reinstall it', () => {
  const workspace = mkdtempSync(join(tmpdir(), 'spirolo-missing-'));
  try {
    // Real npm output isolates this case even if the developer has installed a native package.
    const pack = spawnSync('npm', ['pack', '--ignore-scripts', '--pack-destination', workspace],
      { cwd: resolve('npm/spirolo'), encoding: 'utf8' });
    assert.equal(pack.status, 0, pack.stderr);
    const tarball = join(workspace, readdirSync(workspace).find(name => name.endsWith('.tgz')));
    const install = spawnSync('npm', ['install', '--prefix', workspace, '--offline', '--omit=optional',
      '--ignore-scripts', '--no-audit', '--no-fund', tarball], { encoding: 'utf8' });
    assert.equal(install.status, 0, install.stderr);
    const require = createRequire(join(workspace, 'package.json'));
    const { resolveBinary } = require('spirolo/resolve.cjs');
    assert.throws(() => resolveBinary(), /optional dependencies/);
    const launch = spawnSync(process.execPath,
      [join(workspace, 'node_modules/spirolo/bin/spirolo.cjs'), '--version'], { encoding: 'utf8' });
    assert.equal(launch.status, 2);
    assert.match(launch.stderr, /optional dependencies/);
  } finally {
    rmSync(workspace, { recursive: true, force: true });
  }
});
