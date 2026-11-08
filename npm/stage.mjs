// Stage native payloads from cargo-dist's real archives, then pack for offline installation.
import { readFileSync, writeFileSync, copyFileSync, mkdirSync, chmodSync, readdirSync, rmSync, existsSync } from 'node:fs';
import { resolve, join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const out = resolve(process.argv[2] ?? join(root, 'target/rc'));
mkdirSync(out, { recursive: true });
function run(command, args, cwd = root) {
  const result = spawnSync(command, args, { cwd, encoding: 'utf8' });
  if (result.status !== 0) throw new Error(`${command} failed: ${result.stderr}`);
  return result.stdout;
}
run(process.execPath, ['npm/release.mjs', 'check']);
const packages = [['linux-x64', 'x86_64-unknown-linux-gnu'], ['linux-arm64', 'aarch64-unknown-linux-gnu'],
  ['darwin-x64', 'x86_64-apple-darwin'], ['darwin-arm64', 'aarch64-apple-darwin']]
  .filter(([name]) => !process.argv.includes('--linux-only') || name.startsWith('linux'));
for (const [platform, target] of packages) {
  const archive = `spirolo-${target}.tar.xz`;
  let source = join(root, 'target/distrib', archive);
  const unpack = join(out, `.unpack-${platform}`);
  mkdirSync(unpack, { recursive: true });
  try {
    const packageDir = join(unpack, `spirolo-${target}`);
    const binary = join(packageDir, 'spirolo');
    if (existsSync(source)) {
      run('tar', ['xf', source, '-C', unpack]);
    } else if (target.endsWith('apple-darwin')) {
      // cargo-dist refuses Linux→Darwin. A real cargo zigbuild payload can still be archived.
      mkdirSync(packageDir, { recursive: true });
      copyFileSync(join(root, 'target', target, 'release/spirolo'), binary);
      for (const file of ['README.md', 'LICENSE', 'CHANGELOG.md']) copyFileSync(join(root, file), join(packageDir, file));
      source = join(out, archive);
      run('tar', ['cJf', source, '-C', unpack, `spirolo-${target}`]);
    } else {
      throw new Error(`Missing dist build artifact: ${source}`);
    }
    const payload = join(root, 'npm', platform, 'bin/spirolo');
    mkdirSync(dirname(payload), { recursive: true });
    copyFileSync(binary, payload);
    chmodSync(payload, 0o755);
    copyFileSync(binary, join(out, `spirolo-${target}`));
    chmodSync(join(out, `spirolo-${target}`), 0o755);
    if (source !== join(out, archive)) copyFileSync(source, join(out, archive));
  } finally {
    rmSync(unpack, { recursive: true, force: true });
  }
}
for (const platform of ['spirolo', ...packages.map(([name]) => name)]) {
  run('npm', ['pack', '--ignore-scripts', '--pack-destination', out], join(root, 'npm', platform));
}
writeFileSync(join(out, 'artifacts.json'), `${JSON.stringify({ platforms: packages.map(([name]) => name) }, null, 2)}\n`);
const sums = readdirSync(out).filter(name => /\.tgz$|\.tar\.xz$|\.vsix$|^spirolo-(?:aarch64|x86_64)-|^artifacts\.json$/.test(name))
  .sort().map(name => `${createHash('sha256').update(readFileSync(join(out, name))).digest('hex')}  ${name}`);
writeFileSync(join(out, 'SHA256SUMS'), `${sums.join('\n')}\n`);
console.log(`Staged ${sums.length} checksummed artifacts in ${out}`);
