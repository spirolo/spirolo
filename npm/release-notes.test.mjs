import assert from 'node:assert/strict';
import test from 'node:test';
import { releaseNotes } from './release-notes.mjs';

const changelog = `# Changelog

## 0.2.0 (2027-01-10)

Second.

### Fixed

- A thing.

## 0.1.0-alpha.1 (2026-11-08, advisory alpha)

First.

## 0.1.0-alpha.10

Not the same version.
`;

test('the notes are the version section without its heading, up to the next version', () => {
  assert.equal(releaseNotes(changelog, '0.2.0'), 'Second.\n\n### Fixed\n\n- A thing.\n');
  assert.equal(releaseNotes(changelog, '0.1.0-alpha.1'), 'First.\n');
});

test('a version prefix never matches a longer version', () => {
  assert.equal(releaseNotes(changelog, '0.1.0-alpha.10'), 'Not the same version.\n');
  assert.throws(() => releaseNotes(changelog, '0.1.0-alpha'), /no "## 0.1.0-alpha" section/);
});

test('a missing or empty section fails', () => {
  assert.throws(() => releaseNotes(changelog, '9.9.9'), /no "## 9.9.9" section/);
  assert.throws(() => releaseNotes('## 1.0.0\n\n## 0.9.0\n\nOld.\n', '1.0.0'), /section 1.0.0 is empty/);
});

test('the real changelog has notes for the workspace version', async () => {
  const { readFileSync } = await import('node:fs');
  const version = /^\[workspace\.package\][^[]*?^version = "([^"]+)"/ms
    .exec(readFileSync(new URL('../Cargo.toml', import.meta.url), 'utf8'))?.[1];
  assert.ok(version, 'workspace version');
  assert.ok(releaseNotes(readFileSync(new URL('../CHANGELOG.md', import.meta.url), 'utf8'), version).length > 0);
});
