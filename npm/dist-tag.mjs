// The npm dist-tag a version publishes under: a prerelease's identifier (`0.1.0-alpha.1` → `alpha`),
// or `latest` for a release. A prerelease must never take `latest`, which plain `npm install` resolves.
export function distTag(version) {
  const match = /^\d+\.\d+\.\d+(?:-([0-9A-Za-z.-]+))?$/.exec(version);
  if (!match) throw new Error(`Not a semver version: ${version}`);
  if (!match[1]) return 'latest';
  const id = /^[a-z][a-z-]*/i.exec(match[1]);
  if (!id) throw new Error(`Prerelease ${version} has no alphabetic identifier to use as a dist-tag`);
  return id[0].toLowerCase();
}
