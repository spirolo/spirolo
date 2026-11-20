import assert from 'node:assert/strict';
import { test } from 'node:test';
import { alignClaims, precisionTable, speaksOfRelease, tierFloors, tierOf, trustLabel } from './claims.mjs';

const floors = { decided: 15, numerator: 4, denominator: 5 };
const row = (verdict, truths, falses) => ({ verdict, truths, falses, precision: truths / (truths + falses), interval: [0.9, 1], measured_on: ['a'] });
const status = { spirolo_commit: 'abc', entries: ['a', 'b'], rules: { 'dead-skip': row('UNADJUDICATED', 39, 0) } };
const context = { status, floors, version: '0.1.0-alpha.2' };

test('the floors are read from the Rust source', () => {
  const source = 'pub const SHOWN_DECIDED_FLOOR: u32 = 15;\npub const SHOWN_PRECISION_FLOOR: (u32, u32) = (4, 5);';
  assert.deepEqual(tierFloors(source), floors);
  assert.throws(() => tierFloors('nothing'), /no longer declares/);
});

test('a tier follows the verdict first, then the floors', () => {
  assert.equal(tierOf(row('PASS', 40, 0), floors), 'proven');
  assert.equal(tierOf(row('FAIL', 40, 0), floors), 'hidden');
  assert.equal(tierOf(row('UNADJUDICATED', 16, 4), floors), 'advisory');
  assert.equal(tierOf(row('UNADJUDICATED', 11, 4), floors), 'experimental');
  assert.equal(tierOf(row('INSUFFICIENT', 14, 0), floors), 'experimental');
});

test('a sample banner is rewritten to the label the table gives today', () => {
  const stale = '● dead-skip (1 file, 2 findings) · 1 info · advisory: 18/18 (INSUFFICIENT)\n';
  const { aligned, problems } = alignClaims('docs/x.md', stale, context);
  assert.equal(aligned, `● dead-skip (1 file, 2 findings) · 1 info · ${trustLabel(status.rules['dead-skip'], floors)}\n`);
  assert.match(aligned, /advisory: 39\/39 \(UNADJUDICATED\)/);
  assert.deepEqual(problems, []);
});

test('a banner for a rule the table does not measure is named', () => {
  const { problems } = alignClaims('docs/x.md', '● gone-rule (1 file, 1 finding) · advisory: 1/1 (PASS)', context);
  assert.match(problems[0], /gone-rule/);
});

test('"no rule is proven" stands only while no rule has passed', () => {
  const claim = 'intro\n**No rule is proven yet.**\n目前还没有任何规则达到已验证（proven）级别。';
  assert.deepEqual(alignClaims('README.md', claim, context).problems, []);
  const passed = { ...context, status: { ...status, rules: { 'dead-skip': row('PASS', 39, 0) } } };
  const { problems } = alignClaims('README.md', claim, passed);
  assert.equal(problems.length, 2);
  assert.match(problems[0], /README\.md:2 says no rule is proven, but the table has dead-skip at PASS/);
});

test('another release version in the text is named, an example tag is not', () => {
  const { problems } = alignClaims('README.md', 'Version `0.1.0-alpha.1` is out.\nPush a tag such as `v0.1.0-alpha.1`.\nNow 0.1.0-alpha.2.', context);
  assert.deepEqual(problems, ['README.md:1 names version 0.1.0-alpha.1; this release is 0.1.0-alpha.2']);
});

test('both README tables carry the same rows', () => {
  const data = line => line.startsWith('| dead-skip');
  const en = precisionTable(status, floors, 'en').split('\n');
  const zh = precisionTable(status, floors, 'zh').split('\n');
  assert.equal(en.find(data), '| dead-skip | advisory | UNADJUDICATED | 39 / 39 | 1.000 | 0.900–1.000 | 1 |');
  assert.equal(zh.find(data), en.find(data));
  assert.match(zh[0], /覆盖 2 个/);
});

test('history, policy, fixtures and the corpus are not release claims', () => {
  for (const file of ['README.md', 'website/content/docs/trust.mdx', 'website/components/landing/copy/en.ts', 'npm/spirolo/README.md']) {
    assert.equal(speaksOfRelease(file), true, file);
  }
  for (const file of ['CHANGELOG.md', 'SECURITY.md', 'corpus/README.md', 'crates/x/fixtures/a.md', 'npm/release.mjs']) {
    assert.equal(speaksOfRelease(file), false, file);
  }
});
