// Tag releases are rehearsals unless the public Actions job explicitly enables publishing.
import { readFileSync, realpathSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { distTag } from './dist-tag.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const readPackage = platform => JSON.parse(readFileSync(resolve(root, 'npm', platform, 'package.json'), 'utf8'));
const launcher = readPackage('spirolo');

export function publishPlan({ ref, directory = 'target/rc', publish = false, actions = false, repositoryPrivate = true } = {}) {
  if (ref !== `refs/tags/v${launcher.version}`) {
    throw new Error(`Release tag must be refs/tags/v${launcher.version}; got ${ref}`);
  }
  if (publish && (!actions || repositoryPrivate)) {
    throw new Error('Publishing requires an explicitly enabled public GitHub Actions tag release');
  }
  const packages = ['linux-x64', 'linux-arm64', 'darwin-x64', 'darwin-arm64'].map(readPackage);
  for (const pkg of packages) {
    if (pkg.version !== launcher.version || launcher.optionalDependencies[pkg.name] !== pkg.version) {
      throw new Error(`Release metadata mismatch: ${pkg.name}`);
    }
  }
  return [...packages, launcher].map(pkg => {
    const filename = `${pkg.name.replace(/^@/, '').replace('/', '-')}-${pkg.version}.tgz`;
    return {
      name: pkg.name, filename,
      args: ['publish', resolve(directory, filename), '--registry', 'https://registry.npmjs.org',
        '--access', 'public', '--tag', distTag(pkg.version), '--ignore-scripts', '--provenance',
        ...(publish ? [] : ['--dry-run'])],
    };
  });
}

export function verifyChecksums(directory, requiredFiles) {
  const manifest = readFileSync(resolve(directory, 'SHA256SUMS'), 'utf8').trim();
  const seen = new Set();
  for (const line of manifest.split('\n')) {
    const match = /^([a-f0-9]{64})  ([A-Za-z0-9][A-Za-z0-9._-]*)$/.exec(line);
    if (!match) throw new Error(`Invalid checksum line: ${line}`);
    const [, expected, filename] = match;
    if (seen.has(filename)) throw new Error(`Duplicate checksum: ${filename}`);
    seen.add(filename);
    const actual = createHash('sha256').update(readFileSync(resolve(directory, filename))).digest('hex');
    if (actual !== expected) throw new Error(`Checksum mismatch: ${filename}`);
  }
  for (const filename of requiredFiles) {
    if (!seen.has(filename)) throw new Error(`Missing checksum: ${filename}`);
  }
}

// Node names this module by its real path; resolve a symlinked launch path the same way.
if (process.argv[1] && realpathSync(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const checkTag = process.argv[2] === '--check-tag';
  const directory = resolve(checkTag ? 'target/rc' : process.argv[2] ?? 'target/rc');
  const plan = publishPlan({
    ref: process.env.GITHUB_REF,
    directory,
    publish: process.env.PUBLISH_NPM === 'true',
    actions: process.env.GITHUB_ACTIONS === 'true',
    repositoryPrivate: process.env.REPOSITORY_PRIVATE !== 'false',
  });
  if (checkTag) {
    console.log(`Release tag matches ${launcher.version}`);
  } else {
    // Validate everything before publishing the first package.
    verifyChecksums(directory, plan.map(item => item.filename));
    for (const { name, args } of plan) {
      console.log(`${args.includes('--dry-run') ? 'Rehearse' : 'Publish'} ${name}`);
      const result = spawnSync('npm', args, { stdio: 'inherit' });
      if (result.status !== 0) throw new Error(`npm publish failed: ${name} (${result.error ?? result.status})`);
    }
  }
}
