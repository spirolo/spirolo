// The GitHub release text for one version: its CHANGELOG.md section, without the heading.
// `node npm/release-notes.mjs <version>` prints it, and fails when the section is missing or
// empty, so a tag without release notes stops at the build job instead of the publish job.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

export function releaseNotes(changelog, version) {
  const lines = changelog.split('\n');
  const start = lines.findIndex((line) => line === `## ${version}` || line.startsWith(`## ${version} `));
  if (start < 0) throw new Error(`CHANGELOG.md has no "## ${version}" section`);
  const next = lines.findIndex((line, index) => index > start && line.startsWith('## '));
  const body = lines.slice(start + 1, next < 0 ? undefined : next).join('\n').trim();
  if (!body) throw new Error(`CHANGELOG.md section ${version} is empty`);
  return `${body}\n`;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  try {
    const [version] = process.argv.slice(2);
    if (!version) throw new Error('usage: node npm/release-notes.mjs <version>');
    process.stdout.write(releaseNotes(readFileSync(new URL('../CHANGELOG.md', import.meta.url), 'utf8'), version));
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
