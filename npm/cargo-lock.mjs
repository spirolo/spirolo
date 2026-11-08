// The release build runs `cargo build --locked`, which refuses to rewrite Cargo.lock. A workspace
// version bump that leaves the lockfile behind therefore fails every tag build, so the release
// check compares the lockfile's record of each workspace crate with the workspace version.

import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

/** The workspace version, from `[workspace.package]` in the root Cargo.toml. */
export function workspaceVersion(root) {
  return readFileSync(join(root, 'Cargo.toml'), 'utf8').split('[workspace.package]')[1].match(/version = "([^"]+)"/)[1];
}

/** The package names of the `crates/*` members that inherit the workspace version. */
export function workspaceCrates(root) {
  return readdirSync(join(root, 'crates')).sort()
    .map(dir => join(root, 'crates', dir, 'Cargo.toml')).filter(existsSync)
    .map(path => readFileSync(path, 'utf8'))
    .filter(manifest => /^version\.workspace = true$/m.test(manifest))
    .map(manifest => manifest.match(/^name = "([^"]+)"$/m)[1]);
}

/** Each Cargo.lock package without a `source`: the workspace's own crates, name to version. */
export function localLockPackages(lock) {
  const local = new Map();
  for (const block of lock.split(/^\[\[package\]\]$/m).slice(1)) {
    const field = key => block.match(new RegExp(`^${key} = "([^"]*)"$`, 'm'))?.[1];
    if (field('source') === undefined && field('name') !== undefined) local.set(field('name'), field('version'));
  }
  return local;
}

/**
 * Every workspace crate in `crates` whose Cargo.lock entry is missing or not at `version`;
 * empty when the lockfile matches what a locked build of `version` needs.
 */
export function staleLockEntries(lock, version, crates) {
  const local = localLockPackages(lock);
  const errors = [];
  for (const name of crates) {
    if (!local.has(name)) errors.push(`Cargo.lock has no entry for workspace crate ${name}`);
    else if (local.get(name) !== version) {
      errors.push(`Cargo.lock records ${name} ${local.get(name)}, not the workspace version ${version}`);
    }
  }
  return errors;
}
