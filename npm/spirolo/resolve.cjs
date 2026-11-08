'use strict';

function packageName(platform, arch) {
  if (['linux', 'darwin'].includes(platform) && ['x64', 'arm64'].includes(arch)) {
    return `spirolo-${platform}-${arch}`;
  }
  throw new Error(`Spirolo has no release candidate for ${platform}-${arch}`);
}

function resolveBinary(platform = process.platform, arch = process.arch) {
  const name = packageName(platform, arch);
  try {
    return require.resolve(`${name}/bin/spirolo`);
  } catch (error) {
    throw new Error(`Missing ${name}. Reinstall spirolo with optional dependencies enabled.`, { cause: error });
  }
}

module.exports = { packageName, resolveBinary };
