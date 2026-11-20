// Copies what the site reads from the rest of the repo, so none of it is typed twice: the brand
// pictures (../branding), the measured rule table the binary embeds (../corpus/rule-status.json),
// and the release version and tier floors (../Cargo.toml, the Rust status source). Runs before
// every dev and build.
import { cpSync, mkdirSync, rmSync, copyFileSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { workspaceVersion } from '../../npm/cargo-lock.mjs';
import { tierFloors } from '../../npm/claims.mjs';

const site = join(dirname(fileURLToPath(import.meta.url)), '..');
const repo = join(site, '..');
const brand = join(site, 'public/brand');

rmSync(brand, { recursive: true, force: true });
// branding/ holds only what the site ships (the rest is in the brand library), so all of it is copied.
for (const dir of ['scenes', 'stickers', 'elements', 'logo']) {
  cpSync(join(repo, 'branding', dir), join(brand, dir), { recursive: true });
}
copyFileSync(join(repo, 'branding/logo/favicon.svg'), join(site, 'app/icon.svg'));

mkdirSync(join(site, 'lib/generated'), { recursive: true });
copyFileSync(join(repo, 'corpus/rule-status.json'), join(site, 'lib/generated/rule-status.json'));
writeFileSync(join(site, 'lib/generated/release.json'), `${JSON.stringify({
  version: workspaceVersion(repo),
  floors: tierFloors(readFileSync(join(repo, 'crates/spirolo-rules/src/status.rs'), 'utf8')),
}, null, 2)}\n`);
