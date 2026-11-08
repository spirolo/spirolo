// xvfb-run -a node editors/vscode/test/electron.mjs [all|default|toggle|persist]
import { cpSync, mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { spawn } from 'node:child_process';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { downloadAndUnzipVSCode, runTests } from '@vscode/test-electron';

const here = fileURLToPath(new URL('.', import.meta.url));
const scratch = mkdtempSync(join(tmpdir(), 'spirolo-rules-electron-'));
const workspace = join(scratch, 'workspace');
const binary = resolve(process.env.SPIROLO_TEST_BINARY ?? join(here, '../../../target/debug/spirolo'));
cpSync(join(here, 'workspace'), workspace, { recursive: true });
mkdirSync(join(workspace, '.vscode'));
writeFileSync(join(workspace, '.vscode/settings.json'), JSON.stringify({ 'spirolo.path': binary }));
const mode = process.argv[2] ?? 'all';
const phases = mode === 'all' ? ['default', 'toggle', 'persist'] : [mode];
const watchdog = setTimeout(() => { console.error('Electron check exceeded five minutes'); process.exit(1); }, 300_000);
try {
  const executable = process.env.SPIROLO_VSCODE_EXECUTABLE ?? await downloadAndUnzipVSCode({
    version: '1.96.4', cachePath: process.env.SPIROLO_VSCODE_CACHE ?? resolve(here, '../../../target/vscode-test'),
  });
  for (const phase of phases) {
    if (phase === 'persist') {
      for (const normalPhase of ['toggle', 'persist']) {
        const out = join(scratch, `${normalPhase}.json`);
        const child = spawn(executable, [workspace,
          `--extensionDevelopmentPath=${resolve(here, '..')}`,
          `--extensionDevelopmentPath=${join(here, 'probe')}`,
          `--user-data-dir=${join(scratch, 'user')}`, `--extensions-dir=${join(scratch, 'extensions')}`,
          '--no-sandbox', '--disable-gpu', '--disable-workspace-trust', '--skip-welcome', '--skip-release-notes'], {
          stdio: 'inherit', env: { ...process.env, SPIROLO_E2E_PHASE: normalPhase, SPIROLO_E2E_OUT: out },
        });
        const timer = setTimeout(() => child.kill('SIGTERM'), 60_000);
        try {
          const code = await new Promise((resolve, reject) => {
            child.once('error', reject);
            child.once('exit', resolve);
          });
          if (code !== 0) throw new Error(`VS Code ${normalPhase} exited ${code}`);
          const result = JSON.parse(readFileSync(out, 'utf8'));
          if (!result.pass) throw new Error(result.error);
          console.log(`Persistence ${normalPhase}: PASS`);
        } finally {
          clearTimeout(timer);
        }
      }
      continue;
    }
    await runTests({
      vscodeExecutablePath: executable,
      extensionDevelopmentPath: resolve(here, '..'),
      extensionTestsPath: join(here, 'electron-suite.js'),
      extensionTestsEnv: { SPIROLO_E2E_PHASE: phase },
      launchArgs: [workspace, '--user-data-dir', join(scratch, 'user'), '--extensions-dir', join(scratch, 'extensions'),
        '--no-sandbox', '--disable-gpu', '--disable-workspace-trust', '--skip-welcome', '--skip-release-notes'],
    });
  }
} finally {
  clearTimeout(watchdog);
  if (process.env.SPIROLO_E2E_KEEP) console.log(`Electron scratch: ${scratch}`);
  else rmSync(scratch, { recursive: true, force: true });
}
