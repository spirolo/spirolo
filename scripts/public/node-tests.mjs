import { execFileSync, spawnSync } from 'node:child_process';

// Restrict discovery to committed Node suites. Runner fixtures contain Jest/Vitest
// tests and must never be picked up by Node's default recursive discovery.
// npm/staged.test.mjs installs the staged release tarballs, so it runs after a
// release build (`just rc-test`), not here.
const notASuite = (file) => /(?:^|\/)fixtures\//.test(file) || file === 'npm/staged.test.mjs';
try {
  const env = Object.fromEntries(Object.entries(process.env).filter(([key]) => !key.startsWith('GIT_') && key !== 'NODE_TEST_CONTEXT'));
  const files = execFileSync('git', ['ls-files', '-z'], { env, encoding: 'utf8' })
    .split('\0').filter((file) => /\.test\.[mc]js$/.test(file) && !notASuite(file)).sort();
  if (!files.length) throw new Error('snapshot contains no tracked Node test suites');
  console.log(`Node suites: ${files.join(', ')}`);
  const result = spawnSync(process.execPath, ['--test', '--test-concurrency=1', ...files], {
    env, stdio: 'inherit',
  });
  if (result.error) throw result.error;
  process.exitCode = result.status ?? 1;
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
