import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { localLockPackages, staleLockEntries, workspaceCrates, workspaceVersion } from './cargo-lock.mjs';
import { distTag } from './dist-tag.mjs';

test('Cargo, npm and VSIX metadata agree on the canonical version', () => {
  const result = spawnSync(process.execPath, ['npm/release.mjs', 'check'], { encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr);
});

test('the README precision table is current with the embedded OSS status', () => {
  const result = spawnSync(process.execPath, ['npm/release.mjs', 'check'], { encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr);
  // The public document must actually contain the generated table.
  return import('node:fs').then(({ readFileSync }) => {
    assert.match(readFileSync('README.md', 'utf8'), /<!-- precision:start -->/);
  });
});

test('a prerelease publishes under its identifier, never latest', () => {
  assert.equal(distTag('0.1.0-alpha.1'), 'alpha');
  assert.equal(distTag('0.2.0-rc.3'), 'rc');
  assert.equal(distTag('1.0.0-beta'), 'beta');
  assert.equal(distTag('1.0.0'), 'latest');
  assert.throws(() => distTag('1.0.0-1'), /no alphabetic identifier/);
  assert.throws(() => distTag('v1.0'), /Not a semver/);
});

test('Cargo.lock records every workspace crate at the workspace version, so --locked builds it', () => {
  const lock = readFileSync('Cargo.lock', 'utf8');
  const version = workspaceVersion('.');
  const crates = workspaceCrates('.');
  assert.ok(crates.includes('spirolo') && crates.length === localLockPackages(lock).size, crates.join(' '));
  assert.deepEqual(staleLockEntries(lock, version, crates), []);
});

test('a version bump without cargo update --workspace is named crate by crate', () => {
  const lock = readFileSync('Cargo.lock', 'utf8');
  const version = workspaceVersion('.');
  const crates = workspaceCrates('.');
  // The workspace moved on; the lockfile still records the old version for every crate.
  assert.deepEqual(staleLockEntries(lock, '9.9.9-alpha.1', crates),
    crates.map(name => `Cargo.lock records ${name} ${version}, not the workspace version 9.9.9-alpha.1`));
  // One crate left behind, and one missing from the lockfile.
  const planted = lock.replace(`name = "spirolo-model"\nversion = "${version}"`, 'name = "spirolo-model"\nversion = "0.0.1"');
  assert.notEqual(planted, lock);
  assert.deepEqual(staleLockEntries(planted, version, crates), [`Cargo.lock records spirolo-model 0.0.1, not the workspace version ${version}`]);
  assert.deepEqual(staleLockEntries(lock, version, [...crates, 'spirolo-new']), ['Cargo.lock has no entry for workspace crate spirolo-new']);
  // A registry package that shares a version string is not a workspace crate.
  assert.ok(![...localLockPackages(lock).keys()].some(name => !crates.includes(name)));
});
