// Seam 5: install REAL staged packages, never counterfeit binaries or runner answers.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { spawnSync } from 'node:child_process';
import { readFileSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { createRequire } from 'node:module';

const rc = resolve(process.env.SPIROLO_RC_DIR ?? 'target/rc');
const { platforms } = JSON.parse(readFileSync(join(rc, 'artifacts.json')));
const manifests = ['spirolo', ...platforms];
const tarballs = manifests.map(dir => {
  const { name, version } = JSON.parse(readFileSync(`npm/${dir}/package.json`));
  return join(rc, `${name.slice(1).replace('/', '-')}-${version}.tgz`);
});

test('each installed optional platform package resolves its own real native binary', () => {
  const workspace = mkdtempSync(join(tmpdir(), 'spirolo-rc-'));
  try {
    const install = spawnSync('npm', ['install', '--prefix', workspace, '--offline', '--force',
      '--ignore-scripts', '--no-audit', '--no-fund', ...tarballs], { encoding: 'utf8' });
    assert.equal(install.status, 0, install.stderr);
    const require = createRequire(join(workspace, 'package.json'));
    const { resolveBinary } = require('spirolo/resolve.cjs');
    for (const [platform, arch, machine] of [
      ['linux', 'x64', 62], ['linux', 'arm64', 183],
      ['darwin', 'x64', 0x01000007], ['darwin', 'arm64', 0x0100000c],
    ].filter(([platform, arch]) => platforms.includes(`${platform}-${arch}`))) {
      const binary = resolveBinary(platform, arch);
      const bytes = readFileSync(binary);
      if (platform === 'linux') {
        assert.equal(bytes.subarray(0, 4).toString('hex'), '7f454c46');
        assert.equal(bytes.readUInt16LE(18), machine);
      } else {
        assert.equal(bytes.readUInt32LE(0), 0xfeedfacf);
        assert.equal(bytes.readUInt32LE(4), machine);
      }
      assert.ok(binary.includes(`spirolo-${platform}-${arch}`), binary);
    }
    const launch = spawnSync(process.execPath,
      [join(workspace, 'node_modules/spirolo/bin/spirolo.cjs'), '--version'], { encoding: 'utf8' });
    const { version } = require('spirolo/package.json');
    assert.equal(launch.status, 0, launch.stderr);
    assert.equal(launch.stdout.trim(), `spirolo ${version}`);
  } finally {
    rmSync(workspace, { recursive: true, force: true });
  }
});
