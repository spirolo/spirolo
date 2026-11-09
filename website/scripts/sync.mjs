// Copies the two things the site reads from the rest of the repo, so neither is duplicated in git:
// the brand pictures (../branding) and the measured rule table the binary embeds
// (../corpus/rule-status.json). Runs before every dev and build.
import { cpSync, mkdirSync, rmSync, copyFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

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
