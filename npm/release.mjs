// Cargo's workspace version is the only editable version. Generated metadata is checked in CI.
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { resolve, dirname } from 'node:path';
import { staleLockEntries, workspaceCrates, workspaceVersion } from './cargo-lock.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const read = path => readFileSync(resolve(root, path), 'utf8');
const version = workspaceVersion(root);
const mode = process.argv[2];
if (!['sync', 'check'].includes(mode)) throw new Error('Usage: node npm/release.mjs sync|check');
// Cargo.lock is Cargo's to write, never this generator's: check names it, sync reminds.
const stale = staleLockEntries(read('Cargo.lock'), version, workspaceCrates(root));
if (stale.length > 0) {
  const fix = `${stale.join('\n')}\nRun cargo update --workspace and commit Cargo.lock with the release metadata.`;
  if (mode === 'check') throw new Error(fix);
  console.warn(fix);
}

function generated(path, contents) {
  if (mode === 'check') {
    if (read(path) !== contents) throw new Error(`${path} is stale; run node npm/release.mjs sync`);
  } else {
    mkdirSync(dirname(resolve(root, path)), { recursive: true });
    writeFileSync(resolve(root, path), contents);
  }
}
const json = data => `${JSON.stringify(data, null, 2)}\n`;
const optionalDependencies = {};
for (const os of ['linux', 'darwin']) {
  for (const cpu of ['x64', 'arm64']) {
    const name = `spirolo-${os}-${cpu}`;
    optionalDependencies[name] = version;
    generated(`npm/${os}-${cpu}/package.json`, json({
      name, version, description: `Spirolo native binary for ${os} ${cpu}`,
      license: 'Apache-2.0', homepage: 'https://spirolo.com', repository: 'https://github.com/spirolo/spirolo',
      os: [os], cpu: [cpu], files: ['bin/spirolo'],
    }));
  }
}
generated('npm/spirolo/package.json', json({
  name: 'spirolo', version,
  description: 'Whole-project analyzer for JavaScript test suites',
  license: 'Apache-2.0', homepage: 'https://spirolo.com', repository: 'https://github.com/spirolo/spirolo',
  engines: { node: '>=20' }, bin: { spirolo: 'bin/spirolo.cjs' },
  files: ['bin/spirolo.cjs', 'resolve.cjs'], optionalDependencies,
}));
const editor = JSON.parse(read('editors/vscode/package.json'));
editor.version = version;
generated('editors/vscode/package.json', json(editor));
const lock = JSON.parse(read('editors/vscode/package-lock.json'));
lock.version = version;
lock.packages[''].version = version;
generated('editors/vscode/package-lock.json', json(lock));
const status = JSON.parse(read('corpus/rule-status.json'));
// The tier floors are read from the Rust source the binary is built from, so the README's tier
// column cannot drift from what `spirolo check` shows by default.
const statusSource = read('crates/spirolo-rules/src/status.rs');
const constant = name => {
  const match = statusSource.match(new RegExp(`pub const ${name}: [^=]+= ([^;]+);`));
  if (!match) throw new Error(`crates/spirolo-rules/src/status.rs no longer declares ${name}`);
  return match[1];
};
const decidedFloor = Number(constant('SHOWN_DECIDED_FLOOR'));
const [numerator, denominator] = constant('SHOWN_PRECISION_FLOOR').match(/^\((\d+), (\d+)\)$/).slice(1).map(Number);
const tiers = ['proven', 'advisory', 'experimental', 'hidden'];
const tier = row => {
  if (row.verdict === 'PASS') return 'proven';
  if (['FAIL', 'DROPPED'].includes(row.verdict)) return 'hidden';
  const decided = row.truths + row.falses;
  return decided >= decidedFloor && row.truths * denominator >= decided * numerator ? 'advisory' : 'experimental';
};
const number = value => value === null ? 'n/a' : value.toFixed(3);
const rows = Object.entries(status.rules)
  .sort(([a, x], [b, y]) => tiers.indexOf(tier(x)) - tiers.indexOf(tier(y)) || a.localeCompare(b))
  .map(([name, row]) =>
    `| ${name} | ${tier(row)} | ${row.verdict} | ${row.truths} / ${row.truths + row.falses} | ${number(row.precision)} | ${row.interval ? row.interval.map(number).join('–') : 'n/a'} | ${row.measured_on?.length ?? 0} |`);
const table = [
  `Measured at spirolo commit \`${status.spirolo_commit}\` across ${status.entries.length} OSS dev entries.`,
  `Advisory: at least ${decidedFloor} decided and shown precision at least ${(numerator / denominator).toFixed(2)}.`,
  '', '| Rule | Tier | Status | True / decided | Precision | 95% Wilson interval | Measured entries |',
  '| --- | --- | --- | ---: | ---: | --- | ---: |', ...rows,
].join('\n');
const readme = read('README.md').replace(/<!-- precision:start -->[\s\S]*?<!-- precision:end -->/,
  `<!-- precision:start -->\n${table}\n<!-- precision:end -->`);
generated('README.md', readme);
generated('editors/vscode/LICENSE', read('LICENSE'));
for (const dir of ['spirolo', 'linux-x64', 'linux-arm64', 'darwin-x64', 'darwin-arm64']) {
  generated(`npm/${dir}/LICENSE`, read('LICENSE'));
}
console.log(`release metadata ${version}: ${mode}`);
