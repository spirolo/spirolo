// What the site calls the product and how it is installed, in one place for every page.
export const product = {
  name: 'Spirolo',
  cli: 'spirolo',
  npmPackage: 'spirolo',
  distTag: 'alpha',
  version: '0.1.0-alpha.1',
  config: '.spirolorc.jsonc',
  repoUrl: 'https://github.com/spirolo/spirolo',
} as const;

export const installCommand = `npm install -D ${product.npmPackage}@${product.distTag}`;
export const runCommand = `npx ${product.cli} check`;
