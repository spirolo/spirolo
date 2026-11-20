// What the site calls the product and how it is installed, in one place for every page.
// The version is the Cargo workspace version, copied by scripts/sync.mjs.
import release from './generated/release.json';

export const product = {
  name: 'Spirolo',
  cli: 'spirolo',
  npmPackage: 'spirolo',
  distTag: 'alpha',
  version: release.version,
  config: '.spirolorc.jsonc',
  repoUrl: 'https://github.com/spirolo/spirolo',
} as const;

export const installCommand = `npm install -D ${product.npmPackage}@${product.distTag}`;
export const runCommand = `npx ${product.cli} check`;
