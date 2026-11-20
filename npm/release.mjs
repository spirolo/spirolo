// Cargo's workspace version is the only editable version. Generated metadata is checked in CI.
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { resolve, dirname } from 'node:path';
import { execFileSync } from 'node:child_process';
import { alignClaims, precisionTable, speaksOfRelease, tierFloors } from './claims.mjs';
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
// The MCP Registry lists the launcher as a server. It accepts a version only when that npm
// version's `mcpName` equals the name in server.json, so both come from this one constant.
const mcpName = 'io.github.spirolo/spirolo';
const description = 'Whole-project analyzer for JavaScript test suites';
generated('npm/spirolo/package.json', json({
  name: 'spirolo', version, description, mcpName,
  license: 'Apache-2.0', homepage: 'https://spirolo.com', repository: 'https://github.com/spirolo/spirolo',
  engines: { node: '>=20' }, bin: { spirolo: 'bin/spirolo.cjs' },
  files: ['bin/spirolo.cjs', 'resolve.cjs'], optionalDependencies,
}));
generated('server.json', json({
  $schema: 'https://static.modelcontextprotocol.io/schemas/2025-12-11/server.schema.json',
  name: mcpName, title: 'Spirolo', description,
  repository: { url: 'https://github.com/spirolo/spirolo', source: 'github' },
  websiteUrl: 'https://spirolo.com', version,
  // `spirolo mcp` starts the stdio server: the subcommand is the one positional argument.
  packages: [{
    registryType: 'npm', registryBaseUrl: 'https://registry.npmjs.org', identifier: 'spirolo', version,
    transport: { type: 'stdio' }, packageArguments: [{ type: 'positional', value: 'mcp' }],
  }],
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
const floors = tierFloors(read('crates/spirolo-rules/src/status.rs'));
const withTable = (path, language) => read(path).replace(/<!-- precision:start -->[\s\S]*?<!-- precision:end -->/,
  `<!-- precision:start -->\n${precisionTable(status, floors, language)}\n<!-- precision:end -->`);
const tables = { 'README.md': withTable('README.md', 'en'), 'README.zh-CN.md': withTable('README.zh-CN.md', 'zh') };
// Hand-written words about this release (sample output, "no rule is proven", the version) are
// held to the same table: sync rewrites what it can, and what needs a person is named and fails.
const problems = [];
const published = execFileSync('git', ['ls-files', '-z'], { cwd: root, encoding: 'utf8' })
  .split('\0').filter(speaksOfRelease);
for (const path of published) {
  const { aligned, problems: found } = alignClaims(path, tables[path] ?? read(path), { status, floors, version });
  problems.push(...found);
  generated(path, aligned);
}
if (problems.length > 0) {
  throw new Error(`${problems.join('\n')}\nThese words are written by hand: rewrite them to match corpus/rule-status.json and Cargo.toml.`);
}
generated('editors/vscode/LICENSE', read('LICENSE'));
for (const dir of ['spirolo', 'linux-x64', 'linux-arm64', 'darwin-x64', 'darwin-arm64']) {
  generated(`npm/${dir}/LICENSE`, read('LICENSE'));
}
console.log(`release metadata ${version}: ${mode}`);
