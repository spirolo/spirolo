import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { test } from 'node:test';
import { parseDocument } from 'yaml';

const workflows = new URL('./workflows/', import.meta.url);
const documents = readdirSync(workflows).filter(name => /\.ya?ml$/.test(name)).map(name => {
  const source = readFileSync(new URL(name, workflows), 'utf8');
  const document = parseDocument(source, { uniqueKeys: true });
  assert.deepEqual(document.errors, [], name);
  return { name, source, workflow: document.toJS() };
});

test('every workflow pins actions, limits token permissions and discards checkout credentials', () => {
  assert.ok(documents.length > 0);
  for (const { name, source, workflow } of documents) {
    assert.deepEqual(workflow.permissions, { contents: 'read' }, name);
    for (const [jobName, job] of Object.entries(workflow.jobs)) {
      for (const step of job.steps ?? []) {
        if (!step.uses) continue;
        assert.match(step.uses, /^[\w.-]+\/[\w./-]+@[a-f0-9]{40}$/, `${name}/${jobName}: ${step.uses}`);
        const line = source.split('\n').find(line => line.includes(`uses: ${step.uses}`));
        assert.match(line, / # (?:v\d|stable|1\.90)/, `${name}: version comment`);
        if (step.uses.startsWith('actions/checkout@')) {
          assert.equal(step.with?.['persist-credentials'], false, `${name}/${jobName}`);
        }
      }
      const permissions = job.permissions ?? workflow.permissions;
      const expected = name === 'release.yml' && jobName === 'publish'
        ? { contents: 'write', 'id-token': 'write' }
        : name === 'release.yml' && jobName === 'registry' ? { contents: 'read', 'id-token': 'write' }
        : name === 'codeql.yml' ? { contents: 'read', 'security-events': 'write' } : { contents: 'read' };
      assert.deepEqual(permissions, expected, `${name}/${jobName}: token permissions`);
    }
  }
});

test('a v tag builds all npm platforms and rehearses before explicitly enabled publication', () => {
  const release = documents.find(({ name }) => name === 'release.yml').workflow;
  assert.deepEqual(release.on, { push: { tags: ['v*'] } });
  assert.deepEqual(release.jobs.build.strategy.matrix.include.map(row => row.target).sort(), [
    'aarch64-apple-darwin', 'aarch64-unknown-linux-gnu',
    'x86_64-apple-darwin', 'x86_64-unknown-linux-gnu',
  ]);
  assert.equal(release.jobs.package.needs, 'build');
  assert.ok(release.jobs.package.steps.some(step => step.run === 'node npm/stage.mjs target/rc'));
  assert.ok(release.jobs.package.steps.some(step => step.run === 'sha256sum --check SHA256SUMS'));
  assert.ok(release.jobs.package.steps.some(step => step.run === 'node npm/publish.mjs target/rc'));
  const publish = release.jobs.publish;
  assert.equal(publish.needs, 'package');
  assert.equal(publish.if, "vars.RELEASE_PUBLISH == 'true' && github.event.repository.private == false");
  assert.deepEqual(publish.permissions, { contents: 'write', 'id-token': 'write' });
  assert.equal(publish.env.PUBLISH_NPM, 'true');
  assert.equal(publish.env.REPOSITORY_PRIVATE, '${{ github.event.repository.private }}');
  assert.ok(publish.steps.some(step => step.run === 'node npm/publish.mjs target/rc'));
  assert.ok(publish.steps.some(step => step.run?.includes('gh release create')));
  // Release text comes from the version's CHANGELOG section, never from generated PR titles.
  assert.ok(publish.steps.some(step => step.run?.includes('--notes-file') && step.run.includes('npm/release-notes.mjs')));
  assert.ok(!publish.steps.some(step => step.run?.includes('--generate-notes')));
  assert.ok(release.jobs.package.steps.some(step =>
    step.uses?.startsWith('actions/upload-artifact@') && step.with.path === 'target/rc/*'));
});

test('the MCP Registry listing follows the npm publication, from a verified pinned publisher', () => {
  const { registry, publish } = documents.find(({ name }) => name === 'release.yml').workflow.jobs;
  assert.equal(registry.needs, 'publish');
  assert.equal(registry.if, publish.if);
  assert.match(registry.env.MCP_PUBLISHER_VERSION, /^v\d+\.\d+\.\d+$/);
  const verify = registry.steps.findIndex(step => step.run?.includes('cosign verify-blob'));
  const submit = registry.steps.findIndex(step => step.run?.includes('mcp-publisher" publish'));
  assert.ok(verify >= 0 && submit > verify, 'the publisher is verified before it runs');
  assert.ok(!registry.steps.some(step => step.run?.includes('releases/latest')), 'never the mutable latest URL');
  assert.ok(registry.steps[submit].run.includes('login github-oidc'), 'no stored token');
  // The registry reads `mcpName` from the npm version server.json names, so the three must agree.
  const server = JSON.parse(readFileSync(new URL('../server.json', import.meta.url), 'utf8'));
  const launcher = JSON.parse(readFileSync(new URL('../npm/spirolo/package.json', import.meta.url), 'utf8'));
  assert.equal(server.name, launcher.mcpName);
  assert.deepEqual([server.version, server.packages.map(pkg => `${pkg.identifier}@${pkg.version}`)],
    [launcher.version, [`${launcher.name}@${launcher.version}`]]);
  assert.ok(server.description.length <= 100, 'the registry refuses a description over 100 characters');
});

test('native artifacts come from locked release builds for the selected matrix target', () => {
  const { jobs } = documents.find(({ name }) => name === 'release.yml').workflow;
  assert.equal(jobs.build['runs-on'], '${{ matrix.os }}');
  const steps = jobs.build.steps;
  const buildIndex = steps.findIndex(step => /^cargo build\b/.test(step.run ?? ''));
  assert.ok(buildIndex >= 0, 'native compilation must execute cargo build');
  const build = steps[buildIndex];
  assert.equal(build.env.TARGET, '${{ matrix.target }}');
  for (const argument of ['--release', '--locked', '-p spirolo', '--target "$TARGET"']) {
    assert.ok(build.run.includes(argument), `cargo build requires ${argument}`);
  }
  const archiveIndex = steps.findIndex(step => /^\s*tar -cJf\b/m.test(step.run ?? ''));
  assert.ok(archiveIndex > buildIndex, 'archive creation must execute tar after compilation');
  const archive = steps[archiveIndex];
  assert.equal(archive.env.TARGET, '${{ matrix.target }}');
  assert.match(archive.run, /^\s*cp "target\/\$TARGET\/release\/spirolo" "target\/distrib\/spirolo-\$TARGET\/"$/m);
  assert.match(archive.run, /^\s*tar -cJf "target\/distrib\/spirolo-\$TARGET\.tar\.xz" -C target\/distrib "spirolo-\$TARGET"$/m);
  const uploadIndex = steps.findIndex(step => step.uses?.startsWith('actions/upload-artifact@'));
  assert.ok(uploadIndex > archiveIndex, 'upload must follow archive creation');
  assert.equal(steps[uploadIndex].with.path, 'target/distrib/*.tar.xz');
  assert.equal(steps[uploadIndex].with['if-no-files-found'], 'error');
});

test('CodeQL scans Actions, JavaScript/TypeScript and Rust on main, pull requests and weekly', () => {
  const codeql = documents.find(({ name }) => name === 'codeql.yml').workflow;
  assert.deepEqual(Object.keys(codeql.on).sort(), ['pull_request', 'push', 'schedule']);
  assert.deepEqual(codeql.on.push, { branches: ['main'] });
  assert.equal(codeql.on.schedule.length, 1);
  const { analyze } = codeql.jobs;
  assert.deepEqual(analyze.strategy.matrix.language, ['actions', 'javascript-typescript', 'rust']);
  const init = analyze.steps.find(step => step.uses?.startsWith('github/codeql-action/init@'));
  const run = analyze.steps.find(step => step.uses?.startsWith('github/codeql-action/analyze@'));
  assert.equal(init.with.languages, '${{ matrix.language }}');
  assert.equal(init.uses.split('@')[1], run.uses.split('@')[1], 'init and analyze share one CodeQL release');
});

test('Dependabot maintains Cargo, npm and Actions in weekly groups', () => {
  const document = parseDocument(readFileSync(new URL('./dependabot.yml', import.meta.url), 'utf8'));
  assert.deepEqual(document.errors, []);
  const config = document.toJS();
  assert.equal(config.version, 2);
  assert.deepEqual(config.updates.map(entry => entry['package-ecosystem']).sort(), ['cargo', 'github-actions', 'npm']);
  for (const entry of config.updates) {
    assert.equal(entry.schedule.interval, 'weekly');
    assert.ok(Object.keys(entry.groups ?? {}).length > 0, `${entry['package-ecosystem']} groups`);
  }
  const npm = config.updates.find(entry => entry['package-ecosystem'] === 'npm');
  for (const directory of ['/.github', '/editors/vscode', '/website', '/npm/*']) {
    assert.ok(npm.directories.includes(directory), `npm coverage: ${directory}`);
  }
  const launcher = JSON.parse(readFileSync(new URL('../npm/spirolo/package.json', import.meta.url), 'utf8'));
  assert.ok(npm.ignore.some(entry => entry['dependency-name'] === `${launcher.name}*`), 'native versions follow Cargo');
});
