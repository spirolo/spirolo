// The site is published, so nothing private may reach it: no machine paths, no personal
// addresses, no internal hosts, no business notes. This fails on any of the patterns below in a
// file git tracks under website/. Words that are private in themselves (a client, a private
// corpus entry) cannot be listed here without publishing them, so they go one per line in a
// gitignored `.public-denylist` next to this package, or in PUBLIC_DENYLIST (comma-separated).
import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const site = join(dirname(fileURLToPath(import.meta.url)), '..');

const patterns = [
  [/\/(?:root|Users|home)\/[\w.-]+/, 'an absolute machine path'],
  [/[\w.+-]+@(?!users\.noreply\.github\.com|example\.(?:com|org)\b)[\w-]+(?:\.[\w-]+)*\.[a-z]{2,}\b/i, 'an email address'],
  [/\b(?:ssh|scp)\s+[\w-]+@/, 'a login to a host'],
  [/\b(?:infisical|tailscale|systemd-run)\b/i, 'internal tooling'],
  [/\b(?:revenue|pricing|invoice|affiliate|customer list)\b/i, 'a business note'],
  [/\b(?:sk|pk|rk)_(?:live|test)_\w+|\bghp_\w{20,}|\bAKIA[0-9A-Z]{16}\b/, 'a key'],
];

const extra = [
  ...(existsSync(join(site, '.public-denylist'))
    ? readFileSync(join(site, '.public-denylist'), 'utf8').split('\n')
    : []),
  ...(process.env.PUBLIC_DENYLIST ?? '').split(','),
]
  .map((word) => word.trim())
  .filter((word) => word && !word.startsWith('#'));
for (const word of extra) {
  patterns.push([new RegExp(word.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'i'), 'a word from the private denylist']);
}

const files = execFileSync('git', ['ls-files', '--cached', '--others', '--exclude-standard', '.'], {
  cwd: site,
  encoding: 'utf8',
})
  .split('\n')
  .filter((file) => file && !/\.(?:png|webp|jpe?g|ico)$/.test(file) && file !== 'pnpm-lock.yaml');

let failures = 0;
for (const file of files) {
  if (file === 'scripts/check-public.mjs') continue;
  const lines = readFileSync(join(site, file), 'utf8').split('\n');
  lines.forEach((line, index) => {
    for (const [pattern, what] of patterns) {
      if (pattern.test(line)) {
        failures += 1;
        console.error(`${file}:${index + 1} ${what}: ${line.trim().slice(0, 120)}`);
      }
    }
  });
}

if (failures > 0) {
  console.error(`\n${failures} line(s) look private. The site is public: remove them.`);
  process.exit(1);
}
console.log(`check-public: ${files.length} files, nothing private found (${extra.length} private words checked)`);
