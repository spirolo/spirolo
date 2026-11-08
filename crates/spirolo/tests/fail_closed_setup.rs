//! `spirolo check` end to end on `fail-closed-setup` fixtures: what a maintainer and an agent
//! get from the binary when a runner's setup does not fail closed.
//!
//! Seam 2 of the PRD's testing decisions. Four fixtures, each driven through the real binary
//! with `assert_cmd`, its findings JSON read back:
//!
//! - `all_present`: a Vitest config whose setup file fails closed on all four classes →
//!   nothing, the rule is `ok`.
//! - `each_removed`: the same setup with one invariant taken out at a time → one finding
//!   naming that invariant and the setup file.
//! - `no_msw`: a project with no mock server → only the two runner invariants are judged.
//! - `monorepo_preset`: three packages composing one preset that lacks an invariant → one
//!   finding at the preset with `packages: 3`.
//!
//! The rule's branches are covered on hand-built models in `spirolo-rules`; here we prove the
//! wiring — the setup-file analysis, the preset chain, the rule and the report — is whole.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

#[path = "support/policy.rs"]
mod policy_support;

#[allow(dead_code)]
#[path = "support/status.rs"]
mod status_support;

fn spirolo() -> Command {
    Command::cargo_bin("spirolo").unwrap()
}

/// A fresh project directory under the target tree, named after the test that asked for it.
fn project(name: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    policy_support::info(&path);
    path
}

/// Write `contents` to `path` under `root`, creating parents.
fn write(root: &Path, path: &str, contents: &str) {
    let full = root.join(path);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(full, contents).unwrap();
}

/// The `findings` array of `spirolo check --format json --rule fail-closed-setup` on `root`.
fn findings(root: &Path) -> Vec<Value> {
    let output = spirolo()
        .timeout(std::time::Duration::from_secs(10))
        .args([
            "check",
            "--all",
            "--format",
            "json",
            "--rule",
            "fail-closed-setup",
        ])
        .arg(root)
        .output()
        .unwrap();
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    json["findings"].as_array().cloned().unwrap_or_default()
}

/// A Vitest project whose setup file's four lines can each be dropped: `onUnhandledRequest`,
/// the console guard, the `afterEach(restoreAllMocks)`, and the `beforeAll(server.listen)`.
/// `restoreMocks` is left to the `afterEach` rather than the config so all four live in the
/// setup file, one per line. `drop` names the line to leave out, or `""` to keep them all.
fn strong_project(name: &str, drop: &str) -> PathBuf {
    let root = project(name);
    write(
        &root,
        "vitest.config.ts",
        "import { defineConfig } from 'vitest/config'\n\
         export default defineConfig({\n\
         \x20 test: { include: ['src/**/*.test.ts'], setupFiles: ['./src/setup.ts'] },\n\
         })\n",
    );
    // Build the setup file, leaving out the dropped invariant.
    let listen_options = if drop == "unhandled" {
        "server.listen()"
    } else {
        "server.listen({ onUnhandledRequest: 'error' })"
    };
    let mut setup = String::from("import { beforeAll, afterEach, vi } from 'vitest'\n");
    setup.push_str("import { setupServer } from 'msw/node'\n");
    setup.push_str("export const server = setupServer()\n");
    // The global-server invariant is the `listen()` sitting in a `beforeAll`; dropping it calls
    // `listen()` outside a hook so MSW is still present (the request invariants stay decidable)
    // while the global-server invariant is gone.
    let listen_line = if drop == "server" {
        format!("{listen_options}\n")
    } else {
        format!("beforeAll(() => {{ {listen_options} }})\n")
    };
    setup.push_str(&listen_line);
    if drop != "console" {
        setup.push_str(
            "vi.spyOn(console, 'error').mockImplementation(() => { throw new Error('no logs') })\n",
        );
    }
    if drop != "restore" {
        setup.push_str("afterEach(() => { vi.restoreAllMocks() })\n");
    }
    write(&root, "src/setup.ts", &setup);
    write(
        &root,
        "src/example.test.ts",
        "import { it, expect } from 'vitest'\n\
         it('adds', () => { expect(1 + 1).toBe(2) })\n",
    );
    root
}

#[test]
fn a_setup_with_all_four_invariants_is_silent() {
    let root = strong_project("fcs-all", "");
    assert!(
        findings(&root).is_empty(),
        "a setup that fails closed on all four classes is not a finding: {:#?}",
        findings(&root)
    );
}

/// Assert that dropping `drop` yields exactly one finding whose invariant evidence is `slug`
/// and whose location is the setup file.
fn one_finding_for(drop: &str, slug: &str) {
    let root = strong_project(&format!("fcs-drop-{drop}"), drop);
    let found = findings(&root);
    assert_eq!(
        found.len(),
        1,
        "dropping {drop} leaves exactly one finding: {found:#?}"
    );
    let invariant = found[0]["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["key"] == "invariant")
        .expect("an invariant evidence entry");
    assert_eq!(invariant["value"], slug, "{found:#?}");
    assert_eq!(
        found[0]["file"], "src/setup.ts",
        "the finding names the setup file: {found:#?}"
    );
    assert_eq!(found[0]["rule"], "fail-closed-setup");
}

#[test]
fn dropping_the_unhandled_request_error_reports_it() {
    one_finding_for("unhandled", "unhandled-request-error");
}

#[test]
fn dropping_the_console_guard_reports_it() {
    one_finding_for("console", "console-guard");
}

#[test]
fn dropping_the_restore_mocks_reports_it() {
    one_finding_for("restore", "restore-mocks");
}

#[test]
fn dropping_the_global_server_reports_it() {
    one_finding_for("server", "global-server");
}

#[test]
fn a_project_without_msw_reports_only_the_two_runner_invariants() {
    // No mock server anywhere: the setup file guards nothing. Only the console guard and the
    // mock reset are judged; the two MSW invariants are not applicable.
    let root = project("fcs-no-msw");
    write(
        &root,
        "vitest.config.ts",
        "import { defineConfig } from 'vitest/config'\n\
         export default defineConfig({\n\
         \x20 test: { include: ['src/**/*.test.ts'], setupFiles: ['./src/setup.ts'] },\n\
         })\n",
    );
    write(&root, "src/setup.ts", "export const nothing = true\n");
    write(
        &root,
        "src/example.test.ts",
        "import { it, expect } from 'vitest'\n\
         it('adds', () => { expect(1 + 1).toBe(2) })\n",
    );
    let found = findings(&root);
    assert_eq!(
        found.len(),
        2,
        "without MSW only the console guard and mock reset are judged: {found:#?}"
    );
    let slugs: Vec<String> = found
        .iter()
        .map(|f| {
            f["evidence"]
                .as_array()
                .unwrap()
                .iter()
                .find(|e| e["key"] == "invariant")
                .unwrap()["value"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert!(slugs.contains(&"console-guard".to_owned()), "{slugs:?}");
    assert!(slugs.contains(&"restore-mocks".to_owned()), "{slugs:?}");
    assert!(
        !slugs.contains(&"unhandled-request-error".to_owned())
            && !slugs.contains(&"global-server".to_owned()),
        "the MSW invariants are not reported without a mock server: {slugs:?}"
    );
}

#[cfg(unix)]
#[test]
fn three_packages_composing_one_preset_report_once_at_the_preset() {
    // A monorepo where three packages each `mergeConfig` the same preset. The preset declares
    // the setup files (so the whole chain fails closed on the MSW invariants), but none of the
    // three configs sets `restoreMocks` and the shared setup has no `afterEach(restoreAllMocks)`
    // — so the mock-reset invariant is absent for all three, and the rule lifts it to the
    // preset with `packages: 3`.
    let root = project("fcs-monorepo");
    write(
        &root,
        "pnpm-workspace.yaml",
        "packages:\n  - 'packages/*'\n",
    );
    write(
        &root,
        "package.json",
        "{ \"name\": \"root\", \"private\": true }\n",
    );
    // The shared preset: setup files that fail closed on the MSW invariants and the console,
    // but no restoreMocks and no afterEach(restoreAllMocks).
    write(
        &root,
        "packages/config/package.json",
        "{ \"name\": \"@fixture/config\", \"version\": \"1.0.0\", \"main\": \"vitest.mjs\" }\n",
    );
    write(
        &root,
        "packages/config/vitest.mjs",
        "import { defineConfig } from 'vitest/config'\n\
         export default defineConfig({\n\
         \x20 test: { include: ['src/**/*.test.ts'], setupFiles: ['./setup.ts'] },\n\
         })\n",
    );
    write(
        &root,
        "packages/config/setup.ts",
        "import { beforeAll, vi } from 'vitest'\n\
         import { setupServer } from 'msw/node'\n\
         export const server = setupServer()\n\
         beforeAll(() => { server.listen({ onUnhandledRequest: 'error' }) })\n\
         vi.spyOn(console, 'error').mockImplementation(() => { throw new Error('no logs') })\n",
    );
    for pkg in ["a", "b", "c"] {
        write(
            &root,
            &format!("packages/{pkg}/package.json"),
            &format!(
                "{{ \"name\": \"@fixture/{pkg}\", \"version\": \"1.0.0\", \"dependencies\": {{ \"@fixture/config\": \"1.0.0\" }} }}\n"
            ),
        );
        write(
            &root,
            &format!("packages/{pkg}/vitest.config.mjs"),
            "import { defineConfig, mergeConfig } from 'vitest/config'\n\
             import preset from '@fixture/config'\n\
             export default mergeConfig(preset, defineConfig({ test: {} }))\n",
        );
        write(
            &root,
            &format!("packages/{pkg}/src/example.test.ts"),
            "import { it, expect } from 'vitest'\n\
             it('adds', () => { expect(1 + 1).toBe(2) })\n",
        );
    }
    // Symlink the preset into each package's node_modules so the resolver follows `@fixture/config`.
    for pkg in ["a", "b", "c"] {
        let link_dir = root.join(format!("packages/{pkg}/node_modules/@fixture"));
        std::fs::create_dir_all(&link_dir).unwrap();
        let target = root.join("packages/config");
        std::os::unix::fs::symlink(&target, link_dir.join("config")).unwrap();
    }

    let found = findings(&root);
    let restore: Vec<&Value> = found
        .iter()
        .filter(|f| {
            f["evidence"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["key"] == "invariant" && e["value"] == "restore-mocks")
        })
        .collect();
    assert_eq!(
        restore.len(),
        1,
        "one decision at the preset, not one per package: {found:#?}"
    );
    assert_eq!(
        restore[0]["file"], "packages/config/vitest.mjs",
        "the finding is located at the preset: {:#?}",
        restore[0]
    );
    let packages = restore[0]["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["key"] == "packages")
        .expect("a packages evidence entry");
    assert_eq!(packages["value"], "3", "{:#?}", restore[0]);
}

#[test]
fn an_unreadable_config_is_one_could_not_decide_row_not_two_absences() {
    // A config whose preset does not resolve: the setup cannot be read, so the rule says so
    // once rather than reporting the console guard and mock reset as absent.
    let root = project("fcs-unread");
    write(
        &root,
        "vitest.config.ts",
        "import baseConfig from '@nowhere/config-vitest/base'\nexport default baseConfig\n",
    );
    write(
        &root,
        "src/example.test.ts",
        "import { it, expect } from 'vitest'\nit('adds', () => { expect(1 + 1).toBe(2) })\n",
    );
    let found = findings(&root);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0]["outcome"], "could_not_decide");
    assert_eq!(found[0]["file"], "vitest.config.ts");
    assert!(
        found[0]["explain"]
            .as_str()
            .unwrap()
            .contains("preset '@nowhere/config-vitest/base' could not be resolved"),
        "{:#?}",
        found[0]
    );
}

#[test]
fn msw_presence_is_decided_per_gate_not_tree_wide() {
    // Two packages: `a` uses MSW (handlers under src/mocks, a setup that starts the server
    // without `onUnhandledRequest: 'error'`), `b` does not. The MSW invariants are judged for
    // `a` only; `b` is never told to configure a mock server another package uses.
    let root = project("fcs-per-gate-msw");
    write(
        &root,
        "pnpm-workspace.yaml",
        "packages:\n  - 'packages/*'\n",
    );
    write(
        &root,
        "package.json",
        "{ \"name\": \"root\", \"private\": true }\n",
    );
    for pkg in ["a", "b"] {
        write(
            &root,
            &format!("packages/{pkg}/package.json"),
            &format!("{{ \"name\": \"@fixture/{pkg}\", \"version\": \"1.0.0\" }}\n"),
        );
        write(
            &root,
            &format!("packages/{pkg}/vitest.config.ts"),
            "import { defineConfig } from 'vitest/config'\n\
             export default defineConfig({\n\
             \x20 test: { include: ['src/**/*.test.ts'], setupFiles: ['./src/setup.ts'], restoreMocks: true },\n\
             })\n",
        );
        write(
            &root,
            &format!("packages/{pkg}/src/example.test.ts"),
            "import { it, expect } from 'vitest'\n\
             it('adds', () => { expect(1 + 1).toBe(2) })\n",
        );
    }
    write(
        &root,
        "packages/a/src/mocks/handlers.ts",
        "import { http, HttpResponse } from 'msw'\n\
         export const handlers = [http.get('*/v1/users', () => HttpResponse.json({}))]\n",
    );
    write(
        &root,
        "packages/a/src/setup.ts",
        "import { beforeAll, vi } from 'vitest'\n\
         import { setupServer } from 'msw/node'\n\
         export const server = setupServer()\n\
         beforeAll(() => { server.listen() })\n\
         vi.spyOn(console, 'error').mockImplementation(() => { throw new Error('no logs') })\n",
    );
    write(
        &root,
        "packages/b/src/setup.ts",
        "import { vi } from 'vitest'\n\
         vi.spyOn(console, 'error').mockImplementation(() => { throw new Error('no logs') })\n",
    );

    let found = findings(&root);
    let files: Vec<(&str, &str)> = found
        .iter()
        .map(|f| {
            let invariant = f["evidence"]
                .as_array()
                .unwrap()
                .iter()
                .find(|e| e["key"] == "invariant")
                .map_or("", |e| e["value"].as_str().unwrap());
            (f["file"].as_str().unwrap(), invariant)
        })
        .collect();
    assert_eq!(
        files,
        vec![("packages/a/src/setup.ts", "unhandled-request-error")],
        "only a's own MSW gap, nothing for b: {found:#?}"
    );
}

#[test]
fn the_findings_json_is_stable() {
    let root = strong_project("fcs-snapshot", "console");
    insta::assert_json_snapshot!(
        "check_json_fail_closed_setup",
        status_support::redact_status(&serde_json::Value::Array(findings(&root)))
    );
}

#[test]
fn fail_closed_setup_runs_under_the_gate_family() {
    let root = strong_project("fcs-gate-family", "restore");
    // `spirolo gate` runs the gate family, which fail-closed-setup is in.
    spirolo()
        .args(["gate", "--all"])
        .arg(&root)
        .assert()
        .stdout(predicate::str::contains("fail-closed-setup"));
    // `spirolo audit` runs the audit family, which fail-closed-setup is not in.
    spirolo()
        .args(["audit", "--all"])
        .arg(&root)
        .assert()
        .stdout(predicate::str::contains("fail-closed-setup").not());
}

/// The invariant slugs of every finding on `root`, in report order.
fn invariants(root: &Path) -> Vec<String> {
    findings(root)
        .iter()
        .filter_map(|finding| {
            finding["evidence"]
                .as_array()?
                .iter()
                .find(|e| e["key"] == "invariant")
                .and_then(|e| e["value"].as_str())
                .map(str::to_owned)
        })
        .collect()
}

/// A Jest project whose `package.json` loads `src/setupTests.js` after the environment, with
/// that setup's source as given and one test beside it.
fn jest_setup_project(name: &str, setup: &str) -> PathBuf {
    let root = project(name);
    write(
        &root,
        "package.json",
        r#"{ "name": "x", "jest": { "setupFilesAfterEnv": ["<rootDir>/src/setupTests.js"] } }"#,
    );
    write(&root, "src/setupTests.js", setup);
    write(
        &root,
        "src/a.test.js",
        "it('adds', () => { expect(1 + 1).toBe(2) })\n",
    );
    root
}

#[test]
fn a_console_method_assigned_a_throwing_function_is_a_guard() {
    // grommet's src/setupTests.js, verbatim (round 1 false row).
    let root = jest_setup_project(
        "fcs-assign-throw",
        "global.console.warn = (message) => {\n  throw message;\n};\n\
         global.console.error = (message) => {\n  throw message;\n};\n",
    );
    let found = invariants(&root);
    assert!(
        !found.contains(&"console-guard".to_owned()),
        "a console.error that throws is a guard: {found:?}"
    );
    assert!(found.contains(&"restore-mocks".to_owned()), "{found:?}");
}

#[test]
fn a_console_wrapper_that_forwards_or_silences_is_not_a_guard() {
    // mantine's jsdom.mocks.ts filter-and-forward, instantsearch's tests/utils/setupTests.ts
    // silencing assignment and ui-kitten's jest.setup.js silencing spies: none fails a test.
    let sources = [
        (
            "fcs-assign-forward",
            "const originalConsoleError = console.error;\n\
             console.error = (...data) => {\n\
             \x20 if (\n\
             \x20   typeof data[0]?.toString === 'function' &&\n\
             \x20   data[0].toString().includes('Error: Could not parse CSS stylesheet')\n\
             \x20 ) {\n\
             \x20   return;\n\
             \x20 }\n\
             \x20 originalConsoleError(...data);\n\
             };\n",
        ),
        (
            // mantine's shape with a filter string that names a throw: the word is no throw.
            "fcs-assign-forward-thrown",
            "const originalConsoleError = console.error;\n\
             console.error = (...data) => {\n\
             \x20 if (String(data[0]).includes('Error: Uncaught [Error: expected to be thrown]')) {\n\
             \x20   return;\n\
             \x20 }\n\
             \x20 originalConsoleError(...data);\n\
             };\n",
        ),
        (
            // react-devtools-shared's setupTests.js console.error, verbatim: it rethrows one
            // message and forwards every other to the original.
            "fcs-assign-forward-rethrow-one",
            "const originalConsoleError = console.error;\nconst consoleErrorMock = jest.fn();\nconst shouldIgnoreConsoleErrorOrWarn = () => false;\nconsole.error = (...args) => {\n  let firstArg = args[0];\n  if (typeof firstArg === 'string' && firstArg.startsWith('Warning: ')) {\n    // Older React versions might use the Warning: prefix. I'm not sure\n    // if they use this code path.\n    firstArg = firstArg.slice(9);\n  }\n  if (firstArg === 'React instrumentation encountered an error: %o') {\n    // Rethrow errors from React.\n    throw args[1];\n  } else if (\n    typeof firstArg === 'string' &&\n    (firstArg.startsWith(\"It looks like you're using the wrong act()\") ||\n      firstArg.startsWith(\n        'The current testing environment is not configured to support act',\n      ) ||\n      firstArg.startsWith('You seem to have overlapping act() calls') ||\n      firstArg.startsWith(\n        'ReactDOM.render is no longer supported in React 18.',\n      ))\n  ) {\n    // DevTools intentionally wraps updates with acts from both DOM and test-renderer,\n    // since test updates are expected to impact both renderers.\n    return;\n  } else if (shouldIgnoreConsoleErrorOrWarn(args)) {\n    // Allows testing how DevTools behaves when it encounters console.error without cluttering the test output.\n    // Errors can be ignored by running in a special context provided by utils.js#withErrorsOrWarningsIgnored\n    return;\n  }\n\n  consoleErrorMock(...args);\n  originalConsoleError.apply(console, args);\n};\n",
        ),
        (
            "fcs-assign-jest-fn",
            "global.console.info = jest.fn();\nglobal.console.warn = jest.fn();\n",
        ),
        (
            "fcs-spy-silence",
            "beforeAll(() => {\n\
             \x20 jest.spyOn(console, 'error').mockImplementation((msg) => {\n\
             \x20   if (typeof msg === 'string' && msg.includes('act(...)')) {\n\
             \x20     return;\n\
             \x20   }\n\
             \x20 });\n\
             });\n\
             afterAll(() => {\n  jest.restoreAllMocks();\n});\n",
        ),
    ];
    for (name, setup) in sources {
        let found = invariants(&jest_setup_project(name, setup));
        assert!(
            found.contains(&"console-guard".to_owned()),
            "{name}: a wrapper that forwards or silences is no guard: {found:?}"
        );
    }
}

/// carbon's config/jest-config-carbon/setup/setupAfterEnv.js console section, verbatim but for
/// the copyright header, the matcher setup and the chalk colouring: each console method is
/// replaced by a recorder, and a `global.beforeEach`/`global.afterEach` throws through
/// `formatConsoleCallStack` when a call was recorded. `restore_check` keeps or drops the
/// afterEach's own direct `throw` (the mock-restore check), so the helper alone is tested.
fn carbon_setup(restore_check: bool) -> String {
    let check = if restore_check {
        "    // eslint-disable-next-line no-console\n\
         \x20   if (console[methodName] !== patchedConsoleMethod) {\n\
         \x20     throw new Error(`Test did not restore a mock for console.${methodName}`);\n\
         \x20   }\n\n"
    } else {
        ""
    };
    format!(
        "import util from 'util';\n\n\
         const consoleMethods = ['error', 'warn', process.env.CI && 'log'].filter(\n  Boolean\n);\n\n\
         for (const methodName of consoleMethods) {{\n\
         \x20 const unexpectedConsoleCallStacks = [];\n\
         \x20 const patchedConsoleMethod = function (format, ...args) {{\n\
         \x20   const stack = new Error().stack;\n\
         \x20   unexpectedConsoleCallStacks.push([\n\
         \x20     stack.substr(stack.indexOf('\\n') + 1),\n\
         \x20     util.format(format, ...args),\n\
         \x20   ]);\n\
         \x20 }};\n\n\
         \x20 // eslint-disable-next-line no-console\n\
         \x20 console[methodName] = patchedConsoleMethod;\n\n\
         \x20 global.beforeEach(() => {{\n\
         \x20   if (unexpectedConsoleCallStacks.length > 0) {{\n\
         \x20     formatConsoleCallStack(unexpectedConsoleCallStacks, methodName);\n\
         \x20   }}\n\
         \x20   unexpectedConsoleCallStacks.length = 0;\n\
         \x20 }});\n\n\
         \x20 global.afterEach(() => {{\n\
         {check}\
         \x20   if (unexpectedConsoleCallStacks.length > 0) {{\n\
         \x20     formatConsoleCallStack(unexpectedConsoleCallStacks, methodName);\n\
         \x20   }}\n\
         \x20 }});\n\
         }}\n\n\
         function formatConsoleCallStack(unexpectedConsoleCallStacks, methodName) {{\n\
         \x20 const messages = unexpectedConsoleCallStacks.map(\n\
         \x20   ([stack, message]) => `${{message}}\\n${{stack}}`\n\
         \x20 );\n\
         \x20 const message = `Expected test not to call console.${{methodName}}()`;\n\n\
         \x20 throw new Error(`${{message}}\\n\\n${{messages.join('\\n\\n')}}`);\n\
         }}\n"
    )
}

#[test]
fn a_recording_console_checked_in_a_throwing_hook_is_a_guard() {
    for (name, restore_check) in [("fcs-carbon-full", true), ("fcs-carbon-helper", false)] {
        let found = invariants(&jest_setup_project(name, &carbon_setup(restore_check)));
        assert!(
            !found.contains(&"console-guard".to_owned()),
            "{name}: carbon's recorder checked in a throwing hook is a guard: {found:?}"
        );
    }
}

/// carbon's layout: root `jest.config.js` names `preset: 'jest-config-carbon'`, a workspace
/// package linked into `node_modules` whose `jest-preset.js` re-exports `index.js`, which loads
/// its setup through `resolve(__dirname, './setup/setupAfterEnv.js')`.
#[cfg(unix)]
fn carbon_preset_project(name: &str) -> PathBuf {
    let root = project(name);
    write(
        &root,
        "jest.config.js",
        "export default {\n\
         \x20 preset: 'jest-config-carbon',\n\
         \x20 testEnvironment: 'jsdom',\n\
         \x20 testMatch: [\n\
         \x20   '<rootDir>/**/__tests__/**/*.@(js|jsx|ts|tsx)',\n\
         \x20   '<rootDir>/**/*.(spec|test).@(js|jsx|ts|tsx)',\n\
         \x20 ],\n\
         };\n",
    );
    write(
        &root,
        "config/jest-config-carbon/package.json",
        r#"{ "name": "jest-config-carbon", "private": true, "type": "module", "main": "index.js" }"#,
    );
    write(
        &root,
        "config/jest-config-carbon/jest-preset.js",
        "import config from './index.js';\n\nexport default config;\n",
    );
    write(
        &root,
        "config/jest-config-carbon/index.js",
        "import { fileURLToPath } from 'url';\n\
         import { dirname, resolve } from 'path';\n\n\
         const __filename = fileURLToPath(import.meta.url);\n\
         const __dirname = dirname(__filename);\n\n\
         export default {\n\
         \x20 moduleFileExtensions: ['tsx', 'ts', 'js', 'json', 'node'],\n\
         \x20 setupFiles: [resolve(__dirname, './setup/setup.js')],\n\
         \x20 setupFilesAfterEnv: [resolve(__dirname, './setup/setupAfterEnv.js')],\n\
         \x20 testEnvironment: 'jsdom',\n\
         };\n",
    );
    write(
        &root,
        "config/jest-config-carbon/setup/setup.js",
        "global.__DEV__ = true;\n",
    );
    write(
        &root,
        "config/jest-config-carbon/setup/setupAfterEnv.js",
        &carbon_setup(true),
    );
    std::fs::create_dir_all(root.join("node_modules")).unwrap();
    std::os::unix::fs::symlink(
        "../config/jest-config-carbon",
        root.join("node_modules/jest-config-carbon"),
    )
    .unwrap();
    write(
        &root,
        "packages/react/src/__tests__/a-test.js",
        "it('adds', () => { expect(1 + 1).toBe(2) })\n",
    );
    root
}

#[cfg(unix)]
#[test]
fn a_guard_reached_through_a_jest_preset_is_read() {
    let root = carbon_preset_project("fcs-carbon-preset");
    let found = findings(&root);
    let slugs = invariants(&root);
    assert!(
        !slugs.contains(&"console-guard".to_owned()),
        "the preset's setupFilesAfterEnv guards the console: {found:#?}"
    );
    assert_eq!(
        slugs,
        ["restore-mocks"],
        "the preset restores no mocks, decided not undecided: {found:#?}"
    );
}

/// ui-kitten's src/codemod/__e2e__/v5-app: a fixture app whose package.json carries a jest
/// block, with only app sources beside it — or, with `test`, one `__tests__/a.test.js` too.
fn fixture_app_project(name: &str, test: bool) -> PathBuf {
    let root = project(name);
    write(
        &root,
        "package.json",
        r#"{
  "name": "uikitten-v5-app",
  "version": "0.0.1",
  "private": true,
  "scripts": { "test": "jest" },
  "jest": {
    "preset": "react-native",
    "transformIgnorePatterns": [
      "node_modules/(?!((jest-)?react-native|@react-native(-community)?)/)"
    ]
  }
}"#,
    );
    write(
        &root,
        "App.tsx",
        "import React from 'react';\nexport default function App() { return null; }\n",
    );
    if test {
        write(
            &root,
            "__tests__/a.test.js",
            "it('adds', () => { expect(1 + 1).toBe(2) })\n",
        );
    }
    root
}

#[test]
fn a_gate_that_collects_no_test_file_is_silent() {
    let found = findings(&fixture_app_project("fcs-no-tests", false));
    assert!(
        found.is_empty(),
        "a gate that owns no test protects nothing: {found:#?}"
    );
    let found = invariants(&fixture_app_project("fcs-one-test", true));
    assert_eq!(found, ["console-guard", "restore-mocks"], "{found:?}");
}

#[test]
fn an_installed_jest_preset_is_not_followed() {
    // midway's packages: `preset: 'ts-jest'`, whose installed jest-preset.js is a require the
    // reader cannot see through. The gate stays decided on its own setup, as before.
    let root = jest_setup_project("fcs-ts-jest", "export {};\n");
    write(
        &root,
        "package.json",
        r#"{ "name": "x", "jest": { "preset": "ts-jest", "setupFilesAfterEnv": ["<rootDir>/src/setupTests.js"] } }"#,
    );
    write(
        &root,
        "node_modules/ts-jest/package.json",
        r#"{ "name": "ts-jest", "main": "dist/index.js" }"#,
    );
    write(
        &root,
        "node_modules/ts-jest/jest-preset.js",
        "module.exports = require('./presets').defaults\n",
    );
    let found = invariants(&root);
    assert_eq!(
        found,
        ["console-guard", "restore-mocks"],
        "{:#?}",
        findings(&root)
    );
}

#[cfg(unix)]
#[test]
fn a_workspace_jest_preset_the_reader_cannot_read_whole_is_not_followed() {
    // expo's packages: `preset: 'expo-module-scripts'`, a workspace package whose
    // jest-preset.cjs (verbatim below) is a call over spreads and requires. Following it would
    // leave the gate undecided; the gate stays decided on its own setup, as before.
    let root = project("fcs-expo-preset");
    write(
        &root,
        "packages/expo-camera/package.json",
        r#"{ "name": "expo-camera", "jest": { "preset": "expo-module-scripts" } }"#,
    );
    write(
        &root,
        "packages/expo-camera/src/__tests__/Camera-test.ts",
        "it('adds', () => { expect(1 + 1).toBe(2) })\n",
    );
    write(
        &root,
        "packages/expo-module-scripts/package.json",
        r#"{ "name": "expo-module-scripts" }"#,
    );
    write(
        &root,
        "packages/expo-module-scripts/jest-preset.cjs",
        "const { withWatchPlugins } = require('jest-expo/config');\n\n\
         const createJestPreset = require('./createJestPreset.cjs');\n\n\
         module.exports = withWatchPlugins({\n\
         \x20 // Don't fail a package that ships the preset but has no test files yet.\n\
         \x20 passWithNoTests: true,\n\
         \x20 ...require('jest-expo/config/maxWorkers'),\n\
         \x20 projects: [\n\
         \x20   createJestPreset(require('jest-expo/ios/jest-preset')),\n\
         \x20   createJestPreset(require('jest-expo/android/jest-preset')),\n\
         \x20   createJestPreset(require('jest-expo/web/jest-preset')),\n\
         \x20   createJestPreset(require('jest-expo/node/jest-preset')),\n\
         \x20   // Remove sub-watch-plugins from the preset when using multi-project runner.\n\
         \x20 ].map(({ watchPlugins, ...config }) => config),\n\n\
         \x20 // See: https://jestjs.io/docs/configuration#prettierpath-string\n\
         \x20 prettierPath: require.resolve('jest-snapshot-prettier'),\n\
         });\n",
    );
    std::fs::create_dir_all(root.join("node_modules")).unwrap();
    std::os::unix::fs::symlink(
        "../packages/expo-module-scripts",
        root.join("node_modules/expo-module-scripts"),
    )
    .unwrap();
    let found = invariants(&root);
    assert_eq!(
        found,
        ["console-guard", "restore-mocks"],
        "{:#?}",
        findings(&root)
    );
}

#[test]
fn a_project_another_run_loads_is_judged_even_with_no_test_yet() {
    // fumadocs: the root config runs `projects: ['packages/*']`; packages/create-app's own
    // config (verbatim) holds no test yet, but the root run loads it, so its setup governs
    // the next test written there (round 2 row 9, true).
    let root = project("fcs-project-of-run");
    write(
        &root,
        "vitest.config.ts",
        "import { defineConfig } from 'vitest/config';\n\n\
         export default defineConfig({\n\
         \x20 test: {\n\
         \x20   projects: ['packages/*'],\n\
         \x20 },\n\
         });\n",
    );
    let project_config = "import { defineProject } from 'vitest/config';\n\n\
         export default defineProject({\n\
         \x20 resolve: {\n\
         \x20   alias: {\n\
         \x20     '@/': new URL('./src/', import.meta.url).pathname,\n\
         \x20   },\n\
         \x20 },\n\
         });\n";
    write(
        &root,
        "packages/create-app/vitest.config.ts",
        project_config,
    );
    write(
        &root,
        "packages/create-app/src/index.ts",
        "export const x = 1;\n",
    );
    write(&root, "packages/core/vitest.config.ts", project_config);
    write(
        &root,
        "packages/core/src/a.test.ts",
        "import { it, expect } from 'vitest'\nit('adds', () => { expect(1 + 1).toBe(2) })\n",
    );
    let found = findings(&root);
    let at_create_app = found
        .iter()
        .filter(|finding| finding["file"] == "packages/create-app/vitest.config.ts")
        .count();
    assert_eq!(at_create_app, 2, "{found:#?}");
}

#[test]
fn clear_mocks_alone_or_an_after_all_restore_still_leaks() {
    // tududi backend/jest.upgrade.config.js and react-day-picker: clearMocks: true without
    // restoreMocks; ui-kitten's jest.setup.js: restoreAllMocks in an afterAll only.
    let root = jest_setup_project("fcs-clear-only", "export {};\n");
    write(
        &root,
        "package.json",
        r#"{ "name": "x", "jest": { "clearMocks": true, "setupFilesAfterEnv": ["<rootDir>/src/setupTests.js"] } }"#,
    );
    assert_eq!(invariants(&root), ["console-guard", "restore-mocks"]);
    let root = jest_setup_project(
        "fcs-after-all-restore",
        "afterAll(() => {\n  jest.restoreAllMocks();\n});\n",
    );
    assert_eq!(invariants(&root), ["console-guard", "restore-mocks"]);
}

#[test]
fn a_gate_whose_selection_the_reader_cannot_read_is_never_judged_empty() {
    // reanimated's plugin-oxc config (verbatim): a spread, a computed rootDir and a computed
    // setup entry. Its testMatch points under ../plugin, where two of its tests live; the
    // reader cannot follow that root, so it cannot say the gate collects nothing.
    let root = project("fcs-oxc-unread");
    write(&root, "plugin-oxc/jest.config.js", "const path = require('path');\n\nconst pluginDir = path.join(__dirname, '..', 'plugin');\n\nmodule.exports = {\n  ...require('../plugin/jest.config.js'),\n  rootDir: pluginDir,\n  testMatch: [\n    '<rootDir>/__tests__/plugin-bundleMode.test.ts',\n    '<rootDir>/__tests__/plugin-shared.test.ts',\n  ],\n  testNamePattern: '^(?!.*bundleless)',\n  setupFilesAfterEnv: [\n    ...(require('../plugin/jest.config.js').setupFilesAfterEnv ?? []),\n    path.join(__dirname, 'jest', 'captureEmittedFiles.js'),\n  ],\n  moduleNameMapper: {\n    '^\\\\.\\\\./index$': path.join(__dirname, 'jest', 'mockPluginVersion.js'),\n  },\n};\n");
    write(&root, "plugin/jest.config.js", "/** @type {import('jest').Config} */\nmodule.exports = {\n  testEnvironment: 'node',\n  setupFilesAfterEnv: ['<rootDir>/jest/setupSnapshots.js'],\n  testMatch: ['<rootDir>/__tests__/**/*.test.ts'],\n  transform: {\n    '^.+\\\\.(js|ts)$': [\n      'babel-jest',\n      {\n        configFile: false,\n        presets: [\n          ['@babel/preset-env', { targets: { node: 'current' } }],\n          '@babel/preset-typescript',\n        ],\n      },\n    ],\n  },\n};\n");
    write(
        &root,
        "plugin/jest/setupSnapshots.js",
        "export {};
",
    );
    for name in ["plugin-bundleMode", "plugin-shared"] {
        write(
            &root,
            &format!("plugin/__tests__/{name}.test.ts"),
            "it('adds', () => { expect(1 + 1).toBe(2) })
",
        );
    }
    let found = findings(&root);
    let oxc: Vec<&Value> = found
        .iter()
        .filter(|f| f["file"] == "plugin-oxc/jest.config.js")
        .collect();
    assert_eq!(oxc.len(), 1, "{found:#?}");
    assert_eq!(oxc[0]["outcome"], "could_not_decide", "{found:#?}");
}

#[cfg(unix)]
#[test]
fn a_jest_preset_whose_projects_are_calls_is_not_followed() {
    // expo's jest-expo: `preset: 'jest-expo/universal'`, whose jest-preset.js (verbatim) builds
    // each project with a call the reader does not follow. Following it would decide the gate
    // on setup nobody read; the gate stays decided on its own setup, filed where it is.
    let root = project("fcs-expo-universal");
    write(
        &root,
        "packages/jest-expo/package.json",
        r#"{ "name": "jest-expo", "main": "src/index.js", "jest": { "preset": "jest-expo/universal" } }"#,
    );
    write(&root, "packages/jest-expo/universal/jest-preset.js", "const {\n  getWebPreset,\n  getNodePreset,\n  getIOSPreset,\n  getAndroidPreset,\n} = require('../config/getPlatformPreset');\nconst { withWatchPlugins } = require('../config/withWatchPlugins');\n\nmodule.exports = withWatchPlugins({\n  projects: [\n    // Create a new project for each platform.\n    getWebPreset(),\n    getNodePreset(),\n    getIOSPreset(),\n    getAndroidPreset(),\n  ],\n});\n");
    write(
        &root,
        "packages/jest-expo/src/__tests__/a-test.js",
        "it('adds', () => { expect(1 + 1).toBe(2) })\n",
    );
    std::fs::create_dir_all(root.join("node_modules")).unwrap();
    std::os::unix::fs::symlink("../packages/jest-expo", root.join("node_modules/jest-expo"))
        .unwrap();
    let found = findings(&root);
    assert!(
        !found
            .iter()
            .any(|f| f["file"] == "packages/jest-expo/universal/jest-preset.js"),
        "{found:#?}"
    );
    assert!(
        found
            .iter()
            .any(|f| f["file"] == "packages/jest-expo/package.json"),
        "{found:#?}"
    );
}

#[test]
fn a_gate_whose_projects_value_the_reader_cannot_read_is_never_judged_empty() {
    // react-hook-form's scripts/jest/jest.config.js, verbatim: `projects: getProjects()`
    // selects src/**/__tests__ through a call. The reader cannot read that selection, so it
    // cannot say the gate collects nothing: the gate is filed as could-not-decide.
    let root = project("fcs-rhf-projects-call");
    write(&root, "scripts/jest/jest.config.js", "const jestDefaultConfig = {\n  clearMocks: true,\n  resetMocks: true,\n  restoreMocks: true,\n  rootDir: '.',\n  roots: ['<rootDir>/src'],\n  transformIgnorePatterns: ['[/\\\\\\\\]node_modules[/\\\\\\\\].+\\\\.(js|jsx)$'],\n};\n\nconst web = {\n  ...jestDefaultConfig,\n  displayName: {\n    name: 'Web',\n    color: 'cyan',\n  },\n  testMatch: ['**/__tests__/**/*.(spec|test).ts?(x)'],\n  transform: {\n    '^.+\\\\.tsx?$': '@swc/jest',\n  },\n  setupFilesAfterEnv: ['<rootDir>/scripts/jest/setup.ts'],\n  testEnvironment: 'jest-fixed-jsdom',\n  testEnvironmentOptions: {\n    customExportConditions: [''],\n  },\n};\n\nconst server = {\n  ...jestDefaultConfig,\n  displayName: {\n    name: 'Server',\n    color: 'blue',\n  },\n  testMatch: ['**/+([a-zA-Z]).server.(spec|test).ts?(x)'],\n  transform: {\n    '^.+\\\\.tsx?$': '@swc/jest',\n  },\n  testEnvironment: 'node',\n};\n\nconst getProjects = () => {\n  const testEnv = process.env.TEST_ENV;\n  if (!testEnv) {\n    return [web, server];\n  }\n\n  switch (testEnv) {\n    case 'web':\n      return [web];\n    case 'server':\n      return [server];\n  }\n};\n\nmodule.exports = {\n  collectCoverageFrom: [\n    '**/**/*.{ts,tsx}',\n    '!**/**/*.test.{ts,tsx}',\n    '!**/src/types/**',\n    '!**/node_modules/**',\n    '!**/dist/**',\n    '!**/__tests__/**',\n  ],\n  projects: getProjects(),\n};\n");
    write(
        &root,
        "src/__tests__/a.test.ts",
        "it('adds', () => { expect(1 + 1).toBe(2) })\n",
    );
    let found = findings(&root);
    let gate: Vec<&Value> = found
        .iter()
        .filter(|f| f["file"] == "scripts/jest/jest.config.js")
        .collect();
    assert_eq!(gate.len(), 1, "{found:#?}");
    assert_eq!(gate[0]["outcome"], "could_not_decide", "{found:#?}");
}

#[cfg(unix)]
#[test]
fn a_gate_that_inherits_its_test_match_from_a_preset_is_never_judged_empty() {
    // carbon's jest-config-carbon testMatch (index.js, verbatim) collects `*-test.js`, which
    // Jest's default does not; a package config that sets only the preset runs those tests.
    let root = project("fcs-preset-test-match");
    write(
        &root,
        "packages/app/jest.config.js",
        "module.exports = { preset: 'jest-config-x' };\n",
    );
    write(
        &root,
        "packages/app/src/button-test.js",
        "it('adds', () => { expect(1 + 1).toBe(2) })\n",
    );
    write(
        &root,
        "config/jest-config-x/package.json",
        r#"{ "name": "jest-config-x", "private": true, "main": "jest-preset.js" }"#,
    );
    write(
        &root,
        "config/jest-config-x/jest-preset.js",
        "module.exports = {\n\
         \x20 clearMocks: true,\n\
         \x20 testMatch: [\n\
         \x20   '<rootDir>/**/__tests__/**/*.js?(x)',\n\
         \x20   '<rootDir>/**/*.(spec|test).js?(x)',\n\
         \x20   '<rootDir>/**/*-(spec|test).js?(x)',\n\
         \x20 ],\n\
         };\n",
    );
    std::fs::create_dir_all(root.join("node_modules")).unwrap();
    std::os::unix::fs::symlink(
        "../config/jest-config-x",
        root.join("node_modules/jest-config-x"),
    )
    .unwrap();
    let found = invariants(&root);
    assert_eq!(found, ["console-guard", "restore-mocks"], "{found:?}");
}

#[test]
fn a_vitest_gate_with_a_root_is_never_judged_empty() {
    // Vitest resolves `include` from `root`, which the reader does not apply: this config's
    // tests live under `..`, so it cannot say the gate collects nothing.
    let root = project("fcs-vitest-root");
    write(
        &root,
        "config/vitest.config.ts",
        "import { defineConfig } from 'vitest/config';\n\n\
         export default defineConfig({ root: '..', test: { clearMocks: true } });\n",
    );
    write(
        &root,
        "src/a.test.ts",
        "it('adds', () => { expect(1 + 1).toBe(2) })\n",
    );
    let found = invariants(&root);
    assert_eq!(found, ["console-guard", "restore-mocks"], "{found:?}");
}

#[test]
fn a_vitest_gate_with_a_test_root_is_never_judged_empty() {
    // Vitest also takes `root` inside `test`; it moves `include` as the top-level one does.
    let root = project("fcs-vitest-test-root");
    write(
        &root,
        "config/vitest.config.ts",
        "import { defineConfig } from 'vitest/config';\n\n\
         export default defineConfig({ test: { root: '..', clearMocks: true } });\n",
    );
    write(
        &root,
        "src/a.test.ts",
        "it('adds', () => { expect(1 + 1).toBe(2) })\n",
    );
    let found = invariants(&root);
    assert_eq!(found, ["console-guard", "restore-mocks"], "{found:?}");
}

#[test]
fn a_recording_console_checked_in_a_throwing_before_each_is_a_guard() {
    // The review's beforeEach-only recorder: the calls of the previous test fail the next.
    let found = invariants(&jest_setup_project(
        "fcs-recorder-before-each",
        "const calls = [];\n\
         for (const m of ['error','warn']) {\n\
         \x20 console[m] = function (...a) { calls.push(a); };\n\
         \x20 beforeEach(() => { if (calls.length) { throw new Error('console called'); } });\n\
         }\n",
    ));
    assert!(
        !found.contains(&"console-guard".to_owned()),
        "a recorder checked in a throwing beforeEach is a guard: {found:?}"
    );
}

#[test]
fn a_recording_console_with_a_non_throwing_before_each_is_not_a_guard() {
    let found = invariants(&jest_setup_project(
        "fcs-recorder-before-each-quiet",
        "const calls = [];\n\
         for (const m of ['error','warn']) {\n\
         \x20 console[m] = function (...a) { calls.push(a); };\n\
         \x20 beforeEach(() => { if (calls.length) { calls.length = 0; } });\n\
         }\n",
    ));
    assert!(
        found.contains(&"console-guard".to_owned()),
        "a recorder nobody throws on is no guard: {found:?}"
    );
}

#[test]
fn a_console_wrapper_beside_an_unrelated_throwing_hook_is_not_a_guard() {
    // The review's repro: a filter-and-forward assignment, and an afterEach that throws for a
    // reason that has nothing to do with the console. Likewise a silencing `jest.fn()`.
    let sources = [
        (
            "fcs-forward-unrelated-hook",
            "const orig = console.error;\n\
             console.error = (...a) => { if (String(a[0]).includes('act(')) return; orig(...a); };\n\
             afterEach(() => { if (globalThis.__leak) throw new Error('leaked timer'); });\n",
        ),
        (
            "fcs-silence-unrelated-hook",
            "global.console.warn = jest.fn();\n\
             afterAll(() => { if (globalThis.__leak) throw new Error('leaked timer'); });\n",
        ),
    ];
    for (name, setup) in sources {
        let found = invariants(&jest_setup_project(name, setup));
        assert!(
            found.contains(&"console-guard".to_owned()),
            "{name}: an unrelated throwing hook does not make the wrapper a guard: {found:?}"
        );
    }
}

#[test]
fn a_recorder_checked_through_a_called_helper_is_a_guard() {
    // The second review's repros: the hook throws through a helper that reads the recorder.
    let sources = [
        (
            "fcs-recorder-helper-arrow",
            "const calls = [];\n\
             console.error = (...a) => { calls.push(a); };\n\
             const check = () => { if (calls.length) throw new Error('console'); };\n\
             afterEach(() => check());\n",
        ),
        (
            "fcs-recorder-helper-by-reference",
            "const calls = [];\n\
             console.error = (...a) => { calls.push(a); };\n\
             const check = () => { if (calls.length) throw new Error('console'); };\n\
             afterEach(check);\n",
        ),
        (
            "fcs-recorder-helper-function",
            "const calls = [];\n\
             function rec(...a) { calls.push(a); }\n\
             console.error = rec;\n\
             function check() { if (calls.length) { throw new Error('console'); } }\n\
             beforeEach(() => { check(); });\n",
        ),
    ];
    let wrong: Vec<_> = sources
        .iter()
        .filter(|(name, setup)| {
            invariants(&jest_setup_project(name, setup)).contains(&"console-guard".to_owned())
        })
        .map(|(name, _)| *name)
        .collect();
    assert!(
        wrong.is_empty(),
        "a recorder read by a throwing helper is a guard: {wrong:?}"
    );
}

#[test]
fn a_recorder_named_only_in_unrelated_text_is_not_a_guard() {
    // The second review: the recorder must be read, as an identifier, on the way to the throw.
    let sources = [
        (
            "fcs-recorder-name-collision",
            "const calls = [];\n\
             console.error = (...a) => { calls.push(a); };\n\
             afterEach(() => { if (globalThis.recalls) throw new Error('x'); });\n",
        ),
        (
            "fcs-recorder-in-comment",
            "const calls = [];\n\
             console.error = (...a) => { calls.push(a); };\n\
             afterEach(() => { /* no calls check */ if (globalThis.leak) throw new Error('x'); });\n",
        ),
        (
            "fcs-recorder-one-letter",
            "const l = [];\n\
             console.error = (...a) => { l.push(a); };\n\
             afterEach(() => { if (globalThis.leak) throw new Error('x'); });\n",
        ),
        (
            "fcs-mock-fn-console-in-message",
            "global.console.warn = jest.fn();\n\
             afterEach(() => { if (globalThis.leak) throw new Error('timer leaked, see console'); });\n",
        ),
        (
            "fcs-recorder-cleared-only",
            "const orig = console.error;\n\
             const seen = [];\n\
             console.error = (...a) => { seen.push(a); orig(...a); };\n\
             afterEach(() => { seen.length = 0; if (globalThis.leak) throw new Error('x'); });\n",
        ),
        (
            // A silencing spy pairs only with an after hook, never a throwing beforeEach.
            "fcs-spy-silence-before-each",
            "jest.spyOn(console, 'error').mockImplementation(() => {});\n\
             beforeEach(() => { if (globalThis.leak) throw new Error('x'); });\n",
        ),
    ];
    let wrong: Vec<_> = sources
        .iter()
        .filter(|(name, setup)| {
            !invariants(&jest_setup_project(name, setup)).contains(&"console-guard".to_owned())
        })
        .map(|(name, _)| *name)
        .collect();
    assert!(
        wrong.is_empty(),
        "no read of the recorder leads to the throw: {wrong:?}"
    );
}

/// Runs each `(name, setup)` and returns the names whose console-guard presence is not
/// `want_finding`.
fn console_guard_mismatches(sources: &[(&str, &str)], want_finding: bool) -> Vec<String> {
    sources
        .iter()
        .filter(|(name, setup)| {
            invariants(&jest_setup_project(name, setup)).contains(&"console-guard".to_owned())
                != want_finding
        })
        .map(|(name, _)| (*name).to_owned())
        .collect()
}

#[test]
fn a_recorder_checked_with_expect_is_a_guard() {
    // #394's review reproduction: an assertion fails the test without an explicit throw.
    let root = jest_setup_project(
        "fcs-recorder-expect",
        "const calls = [];\n\
         console.error = (...a) => { calls.push(a); };\n\
         afterEach(() => expect(calls).toEqual([]));\n",
    );
    let found = invariants(&root);
    assert!(
        !found.contains(&"console-guard".to_owned()),
        "an assertion on the recorder fails on unexpected console calls: {found:?}"
    );
}

#[test]
fn a_recorder_checked_through_an_object_method_is_a_guard() {
    // #394's object-method reproduction, also executed under Vitest with a console probe.
    let root = jest_setup_project(
        "fcs-recorder-method",
        "const calls = [];\n\
         console.error = (...a) => { calls.push(a); };\n\
         const h = { check() { if (calls.length) throw new Error('console'); } };\n\
         afterEach(() => h.check());\n",
    );
    let found = invariants(&root);
    assert!(
        !found.contains(&"console-guard".to_owned()),
        "a reachable object method checks the recorder: {found:?}"
    );
}

#[test]
fn a_recorder_checked_through_an_alias_is_a_guard() {
    // #394's alias reproduction, executed under Vitest with a console probe.
    let root = jest_setup_project(
        "fcs-recorder-alias",
        "const calls = [];\n\
         console.error = (...a) => { calls.push(a); };\n\
         const rec = calls;\n\
         afterEach(() => { if (rec.length) throw new Error('console'); });\n",
    );
    let found = invariants(&root);
    assert!(
        !found.contains(&"console-guard".to_owned()),
        "an alias preserves the recorder's identity: {found:?}"
    );
}

#[test]
fn a_recorder_checked_through_an_imported_helper_is_a_guard() {
    // #394's imported-helper reproduction, executed under Vitest with a console probe.
    let root = jest_setup_project(
        "fcs-recorder-imported",
        "import { check } from './check.js';\n\
         const calls = [];\n\
         console.error = (...a) => { calls.push(a); };\n\
         afterEach(() => check(calls));\n",
    );
    write(
        &root,
        "src/check.js",
        "export function check(calls) { if (calls.length) throw new Error('console'); }\n",
    );
    let found = invariants(&root);
    assert!(
        !found.contains(&"console-guard".to_owned()),
        "an imported helper can fail the hook: {found:?}"
    );
}

#[test]
fn a_caught_helper_failure_is_not_a_console_guard() {
    // Real Vitest probe: both tests pass when the hook catches the helper's failure.
    let sources = [
        (
            "fcs-recorder-method-caught",
            "const calls = [];\n\
             console.error = (...a) => { calls.push(a); };\n\
             const h = { check() { if (calls.length) throw new Error('console'); } };\n\
             afterEach(() => { try { h.check(); } catch {} });\n",
        ),
        (
            "fcs-recorder-import-caught",
            "import { check } from './check.js';\n\
             const calls = [];\n\
             console.error = (...a) => { calls.push(a); };\n\
             afterEach(() => { try { check(calls); } catch {} });\n",
        ),
    ];
    for (name, setup) in sources {
        let root = jest_setup_project(name, setup);
        write(
            &root,
            "src/check.js",
            "export function check(calls) { if (calls.length) throw new Error('console'); }\n",
        );
        let found = invariants(&root);
        assert!(
            found.contains(&"console-guard".to_owned()),
            "{name}: catching the check lets console calls pass: {found:?}"
        );
    }
}

/// Check sources whose outcomes were recorded by running a real Vitest console probe.
fn check_console_reproductions(names: &[&str]) {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/console-recorder/cases.json")).unwrap();
    for name in names {
        let case = cases.iter().find(|case| case["name"] == *name).unwrap();
        let root = project(&format!("fcs-recorded-{name}"));
        write(
            &root,
            "package.json",
            r#"{ "name": "recorder", "jest": { "setupFilesAfterEnv": ["<rootDir>/src/setup.js"] } }"#,
        );
        write(&root, "src/setup.js", case["setup"].as_str().unwrap());
        write(&root, "src/probe.test.js", "it('probe', () => {});\n");
        for (file, source) in case["helpers"].as_object().unwrap() {
            write(&root, &format!("src/{file}"), source.as_str().unwrap());
        }
        let found = invariants(&root);
        let guard = !found.contains(&"console-guard".to_owned());
        assert_eq!(
            guard,
            case["exit"] == 1,
            "{name}: Spirolo must agree with the executed console probe: {found:?}"
        );
    }
}

#[test]
fn an_object_method_passed_to_a_hook_by_reference_is_a_guard() {
    check_console_reproductions(&["method-reference"]);
}

#[test]
fn recorder_guards_compose_across_aliases_methods_and_imports() {
    check_console_reproductions(&[
        "alias-expect",
        "alias-chain-member",
        "method-expect",
        "import-renamed-barrel",
        "import-default",
        "import-namespace",
    ]);
}

#[test]
fn an_imported_assertion_checks_the_recorder_passed_to_it() {
    check_console_reproductions(&[
        "import-expect",
        "import-unrelated-expect",
        "import-forward-expect",
        "import-capture",
        "import-default-object",
        "method-alias",
        "import-expect-alias",
    ]);
}

#[test]
fn an_assertion_on_a_console_mock_checks_its_recorded_calls() {
    check_console_reproductions(&["mock-expect", "mock-unrelated-expect"]);
}

#[test]
fn a_recursive_silencing_helper_terminates_and_stays_a_finding() {
    check_console_reproductions(&["import-recursive-silencing"]);
}

#[test]
fn a_helper_reached_by_call_reference_or_inside_a_catch_still_reads_the_recorder() {
    // Adversarial review of #394: main followed these helpers; a caught call still reads.
    check_console_reproductions(&[
        "helper-call-method",
        "helper-call-argument",
        "helper-array-reference",
        "caught-read-then-throw",
        "caught-read-in-callback",
    ]);
}

#[test]
fn a_recorder_property_copied_at_load_is_not_the_recorder() {
    // Adversarial review of #394: `const n = calls.length` is a number fixed at load.
    check_console_reproductions(&[
        "alias-property-snapshot",
        "alias-snapshot-argument",
        "import-default-snapshot",
        "argument-property-live",
        "hook-local-property",
    ]);
}

#[test]
fn a_recorder_imported_from_a_module_the_walk_cannot_follow_is_still_a_guard() {
    // Review of #394: an `export *` barrel or a CommonJS module ends the walk; the push and
    // the read still share the setup's import binding, as they did on main.
    check_console_reproductions(&["import-export-all", "import-commonjs"]);
}

#[test]
fn unrelated_assertions_and_silencing_helpers_stay_findings() {
    check_console_reproductions(&[
        "method-unused",
        "method-caught",
        "expect-unrelated",
        "expect-incomplete",
        "expect-caught",
        "expect-shadowed",
        "alias-shadowed",
        "import-silencing",
        "import-caught",
        "method-deferred-reference",
        "import-deferred-reference",
    ]);
}

#[test]
fn a_recorder_read_anywhere_on_the_way_to_a_throw_is_a_guard() {
    // The third review's shapes (early return, a value read first, `&&` into a throwing
    // helper, a member-path recorder), then variants of each with other names, nesting and
    // function styles.
    let sources = [
        (
            "fcs-r3-early-return",
            "const calls = [];\nconsole.error = (...a) => { calls.push(a); };\n\
             afterEach(() => { if (calls.length === 0) return; throw new Error('c'); });\n",
        ),
        (
            "fcs-r3-value-first",
            "const calls = [];\nconsole.error = (...a) => { calls.push(a); };\n\
             afterEach(() => { const n = calls.length; if (n) throw new Error('c'); });\n",
        ),
        (
            "fcs-r3-and-helper",
            "const calls = [];\nconsole.error = (...a) => { calls.push(a); };\n\
             afterEach(() => { calls.length && fail(); });\n\
             function fail() { throw new Error('c'); }\n",
        ),
        (
            "fcs-r3-member-path",
            "const store = { calls: [] };\nconsole.error = (...a) => { store.calls.push(a); };\n\
             afterEach(() => { if (store.calls.length) throw new Error('c'); });\n",
        ),
        (
            "fcs-gen-early-return-function",
            "var logged = [];\n\
             global.console.warn = function () { logged.push(Array.from(arguments)); };\n\
             afterEach(function () {\n  if (!logged.length) {\n    return;\n  }\n\
             \x20 const message = logged.map(String).join('\\n');\n\
             \x20 logged = [];\n  throw new Error(message);\n});\n",
        ),
        (
            "fcs-gen-ternary-nested-helpers",
            "const seenErrors = [];\n\
             function record(...args) { seenErrors.push(args); }\n\
             console.error = record;\n\
             const explode = (count) => { throw new Error(`${count} console errors`); };\n\
             const verify = () => (seenErrors.length > 0 ? explode(seenErrors.length) : undefined);\n\
             describe('all', () => {});\n\
             beforeEach(() => { verify(); });\n",
        ),
        (
            "fcs-gen-loop-and-deep-member",
            "const state = { console: { errors: [] } };\n\
             for (const level of ['error', 'warn']) {\n\
             \x20 console[level] = (...args) => state.console.errors.push({ level, args });\n\
             }\n\
             afterAll(function check() {\n\
             \x20 for (const entry of state.console.errors) {\n\
             \x20   throw new Error(entry.level);\n\
             \x20 }\n});\n",
        ),
        (
            "fcs-gen-helper-cycle",
            "const msgs = [];\nconsole.error = (m) => { msgs.push(m); };\n\
             function a(depth) { if (depth > 3) return; b(depth + 1); }\n\
             function b(depth) { a(depth); if (msgs.length) { throw new Error('x'); } }\n\
             afterEach(() => a(0));\n",
        ),
        (
            "fcs-gen-mock-read",
            "global.console.error = jest.fn();\n\
             afterEach(() => {\n\
             \x20 const { calls } = console.error.mock;\n\
             \x20 if (calls.length) throw new Error('console.error was called');\n});\n",
        ),
    ];
    let wrong = console_guard_mismatches(&sources, false);
    assert!(
        wrong.is_empty(),
        "a recorder read with a reachable throw is a guard: {wrong:?}"
    );
}

#[test]
fn a_shadowed_cleared_or_deferred_recorder_is_not_a_guard() {
    let sources = [
        (
            // The third review's shadowing: a local of the same name is not the recorder.
            "fcs-r3-shadow-local",
            "const calls = [];\nconsole.error = (...a) => { calls.push(a); };\n\
             afterEach(() => { const calls = globalThis.leaks || []; if (calls.length) throw new Error('x'); });\n",
        ),
        (
            "fcs-r3-shadow-param",
            "const calls = [];\nconsole.error = (...a) => { calls.push(a); };\n\
             function check(calls) { if (calls.length) throw new Error('x'); }\n\
             afterEach(() => check(globalThis.leaks));\n",
        ),
        (
            "fcs-gen-shadow-function-param",
            "let buffer = [];\n\
             console.warn = function (msg) { buffer.push(msg); };\n\
             const assertEmpty = function (buffer) {\n  if (buffer.length > 0) {\n    throw new Error('pending');\n  }\n};\n\
             beforeEach(function () { assertEmpty(globalThis.queue); buffer.length = 0; });\n",
        ),
        (
            "fcs-gen-cleared-only",
            "const entries = [];\n\
             console.error = function (...x) { entries.push(x); };\n\
             afterEach(() => { entries.splice(0); if (globalThis.timerLeak) { throw new Error('t'); } });\n",
        ),
        (
            "fcs-gen-reassign-only",
            "let entries = [];\n\
             console.error = (...x) => entries.push(x);\n\
             afterEach(function () { entries = []; if (globalThis.timerLeak) throw new Error('t'); });\n",
        ),
        (
            "fcs-gen-deferred-throw",
            "const errs = [];\nconsole.error = (e) => { errs.push(e); };\n\
             afterEach(() => { if (errs.length) { setTimeout(() => { throw new Error('late'); }, 0); } });\n",
        ),
        (
            "fcs-gen-caught-throw",
            "const errs = [];\nconsole.error = (e) => { errs.push(e); };\n\
             afterEach(() => { try { if (errs.length) throw new Error('x'); } catch (e) { console.log(e); } });\n",
        ),
    ];
    let wrong = console_guard_mismatches(&sources, true);
    assert!(
        wrong.is_empty(),
        "no read of the recorder reaches an escaping throw: {wrong:?}"
    );
}

#[test]
fn a_file_level_row_annotates_its_file_without_a_line() {
    // The two runner invariants anchor at the setup file, not at a line in it: GitHub's
    // lines are 1-based, so the annotation names the file and leaves `line` out (#542).
    let root = project("fcs-annotations-file-level");
    write(
        &root,
        "vitest.config.ts",
        "import { defineConfig } from 'vitest/config'\n\
         export default defineConfig({\n\
         \x20 test: { include: ['src/**/*.test.ts'], setupFiles: ['./src/setup.ts'] },\n\
         })\n",
    );
    write(&root, "src/setup.ts", "export const nothing = true\n");
    write(
        &root,
        "src/example.test.ts",
        "import { it, expect } from 'vitest'\n\
         it('adds', () => { expect(1 + 1).toBe(2) })\n",
    );
    let output = spirolo()
        .args([
            "check",
            "--all",
            "--format",
            "github-annotations",
            "--rule",
            "fail-closed-setup",
        ])
        .arg(&root)
        .output()
        .unwrap();
    let report = String::from_utf8(output.stdout).unwrap();
    let rows: Vec<&str> = report.lines().filter(|l| l.starts_with("::")).collect();
    assert_eq!(rows.len(), 2, "{report}");
    for row in rows {
        assert!(
            row.starts_with("::notice file=src/setup.ts,title="),
            "{row}"
        );
    }
}
