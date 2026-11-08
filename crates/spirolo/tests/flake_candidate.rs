//! `spirolo flake` / `spirolo check` end to end on `flake-candidate` fixture projects: what a
//! maintainer and an agent get from the binary when the static rule names a test that asserts
//! an elapsed-time budget or a calendar value it re-reads from the wall clock.
//!
//! Seam 2 of PRD 0004's testing decisions, narrowed to `wall-clock` (#449): a true case, a
//! near-miss under fake timers and a could-not-decide case; a test-only helper whose budget
//! is attributed to the tests that call into it; a helper production also imports, which is
//! not reported; a skip-gated test at `info`; and the real rows of every other kind, which
//! the rule no longer reports. Driven through the real binary with `assert_cmd`, the JSON
//! view snapshotted with `insta`. The rule's own branches are covered on hand-built models in
//! `spirolo-rules`; here we prove the wiring — extraction, model, rule, report, family and
//! command — is whole.

use std::path::{Path, PathBuf};

use assert_cmd::Command;

#[allow(dead_code)]
#[path = "support/status.rs"]
mod status_support;

fn spirolo() -> Command {
    Command::cargo_bin("spirolo").unwrap()
}

/// A fresh project directory under the target tree, named after the test that asked for it.
/// The `flake-candidate-` prefix keeps these dirs distinct from the other integration test
/// files' (they share one `CARGO_TARGET_TMPDIR` and run in parallel under nextest).
fn project(name: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("flake-candidate-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
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

/// The `flake-candidate` findings of a `--format json` run of `subcommand`, as parsed JSON.
fn candidates(root: &Path, subcommand: &str) -> Vec<serde_json::Value> {
    let output = spirolo()
        .args([
            subcommand,
            "--all",
            "--format",
            "json",
            "--rule",
            "flake-candidate",
        ])
        .arg(root)
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    json["findings"].as_array().unwrap().clone()
}

/// Copy the fixture tree `tests/fixtures/<from>` into `root`.
fn copy(from: &Path, root: &Path) {
    std::fs::create_dir_all(root).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = root.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[test]
fn the_rows_read_false_for_every_kind_but_wall_clock_yield_no_finding() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/m6-flake-narrowed");
    for entry in std::fs::read_dir(&fixtures).unwrap() {
        let entry = entry.unwrap();
        if !entry.file_type().unwrap().is_dir() {
            continue;
        }
        let name = entry.file_name().into_string().unwrap();
        let root = project(&format!("narrowed-{name}"));
        copy(&entry.path(), &root);
        assert_eq!(summary(&candidates(&root, "check")), [], "{name}");
    }
}

/// The `(title, kind, outcome)` of every finding, for a compact assertion.
fn summary(findings: &[serde_json::Value]) -> Vec<(String, String, String)> {
    findings
        .iter()
        .map(|f| {
            let kind = f["evidence"]
                .as_array()
                .unwrap()
                .iter()
                .find(|e| e["key"] == "kind")
                .map(|e| e["value"].as_str().unwrap().to_owned())
                .unwrap_or_default();
            (
                f["test"]["title"].as_str().unwrap_or_default().to_owned(),
                kind,
                f["outcome"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

// ---------------------------------------------------------------- wall-clock

/// A spec whose first test asserts on elapsed wall time, whose second installs fake timers
/// before it does the same, and whose third passes the elapsed value through a function the
/// rule cannot follow before asserting on the result.
fn wall_clock(name: &str) -> PathBuf {
    let root = project(name);
    write(
        &root,
        "src/cache.test.ts",
        "import { test, expect, vi } from 'vitest'\n\
         import { warm, format } from './cache'\n\
         test('is fast when warm', async () => {\n\
         \x20 const start = Date.now()\n\
         \x20 await warm()\n\
         \x20 const elapsed = Date.now() - start\n\
         \x20 expect(elapsed).toBeLessThan(50)\n\
         })\n\
         test('is fast under fake timers', async () => {\n\
         \x20 vi.useFakeTimers()\n\
         \x20 const start = Date.now()\n\
         \x20 await warm()\n\
         \x20 expect(Date.now() - start).toBeLessThan(50)\n\
         })\n\
         test('formats the elapsed time', async () => {\n\
         \x20 const start = performance.now()\n\
         \x20 await warm()\n\
         \x20 expect(format(performance.now() - start)).toMatch(/ms$/)\n\
         })\n",
    );
    write(
        &root,
        "src/cache.ts",
        "export async function warm() {}\nexport function format(ms: number) { return `${ms}ms` }\n",
    );
    root
}

#[test]
fn a_clock_read_reaching_an_assertion_is_a_wall_clock_candidate() {
    let root = wall_clock("wall-clock");
    assert_eq!(
        summary(&candidates(&root, "check")),
        [
            (
                "is fast when warm".to_owned(),
                "wall-clock".to_owned(),
                "finding".to_owned()
            ),
            (
                "formats the elapsed time".to_owned(),
                "wall-clock".to_owned(),
                "could_not_decide".to_owned()
            ),
        ]
    );
}

#[test]
fn spirolo_flake_runs_the_flake_family_and_an_advisory_candidate_leaves_the_exit_code_at_zero() {
    let root = wall_clock("flake-command");
    assert_eq!(candidates(&root, "flake"), candidates(&root, "check"));
    spirolo().arg("flake").arg(&root).assert().code(0);
}

/// The `(line, outcome, message)` of every finding on a copy of the `m6-wall-clock` fixture
/// of `entry`.
fn wall_clock_rows(entry: &str) -> Vec<(u64, String, String)> {
    let root = project(&format!("wall-clock-{entry}"));
    copy(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/m6-wall-clock")
            .join(entry),
        &root,
    );
    candidates(&root, "check")
        .iter()
        .map(|f| {
            (
                f["line"].as_u64().unwrap(),
                f["outcome"].as_str().unwrap().to_owned(),
                f["message"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

#[test]
fn two_clock_reads_whose_difference_an_assertion_bounds_are_an_elapsed_budget_candidate() {
    let budget = |line: u64, title: &str, read: &str| {
        (
            line,
            "finding".to_owned(),
            format!(
                "test {title:?} asserts an elapsed-time budget ({read}), so a slower or busier \
                 machine can fail it"
            ),
        )
    };
    // `expect(endTime - startTime).toBeLessThan(10000)` from two `Date.now()` reads.
    assert_eq!(
        wall_clock_rows("chrisvel__tududi"),
        [budget(
            580,
            "should handle many subtasks efficiently without affecting parent",
            "Date.now()"
        )]
    );
    // `expect(performance.now() - start < 250).toBe(true)`.
    assert_eq!(
        wall_clock_rows("fuma-nama__fumadocs"),
        [budget(
            225,
            "handles pathological `<` input in linear time",
            "performance.now()"
        )]
    );
    // Every budget of the file, the read row 373 among them.
    let lines: Vec<u64> = wall_clock_rows("react-hook-form__react-hook-form")
        .into_iter()
        .map(|(line, _, _)| line)
        .collect();
    assert_eq!(lines, [151, 167, 195, 215, 242, 373]);
}

#[test]
fn a_calendar_field_the_assertion_re_reads_from_the_clock_is_a_date_boundary_candidate() {
    let boundary = |line: u64, title: &str| {
        (
            line,
            "finding".to_owned(),
            format!(
                "test {title:?} re-reads a calendar value from the clock in its expectation \
                 (new Date()), so a run that crosses a date boundary fails it"
            ),
        )
    };
    // `String(new Date().getFullYear()).padStart(4, '0')` as the text the component seeded
    // from its own `new Date()`.
    assert_eq!(
        wall_clock_rows("grommet__grommet"),
        [boundary(
            597,
            "seeds empty year to current year on first arrow interaction"
        )]
    );
    assert_eq!(
        wall_clock_rows("gpbl__react-day-picker"),
        [boundary(8, "should display calendar in December")]
    );
    // A controlled variant of the real test: the same expectation under a set system time.
    let root = project("wall-clock-calendar-set-time");
    write(
        &root,
        "examples/InvalidMonthDefaultLimit.test.tsx",
        &include_str!("fixtures/m6-wall-clock/gpbl__react-day-picker/examples/InvalidMonthDefaultLimit.test.tsx")
            .replace(
                "  render(<InvalidMonthDefaultLimit />);",
                "  jest.useFakeTimers().setSystemTime(new Date(2024, 0, 1));\n  render(<InvalidMonthDefaultLimit />);",
            ),
    );
    assert_eq!(candidates(&root, "check"), [] as [serde_json::Value; 0]);
}

#[test]
fn a_single_clock_read_passed_through_and_compared_to_itself_is_not_a_candidate() {
    let lines = |entry: &str| -> Vec<u64> {
        wall_clock_rows(entry)
            .into_iter()
            .map(|(line, _, _)| line)
            .collect()
    };
    // `const newDatetime = new Date().toJSON()` typed in, then `toHaveValue(newDatetime)`;
    // line 1286 compares one read with the component's own, which no test read measures.
    assert_eq!(lines("rjsf-team__react-jsonschema-form"), [] as [u64; 0]);
    // `const reset = Date.now() + 60000` handed to a double, then compared to the header.
    assert_eq!(lines("middleapi__orpc"), [] as [u64; 0]);
}

// ---------------------------------------------------------------- helpers and gates

/// A project with a test-only helper, a helper that production also imports, and a
/// skip-gated test:
///
/// - `warm.test.ts`: a spec calling the test-only helper `withinBudget`, whose own elapsed
///   budget is attributed to its tests.
/// - `boot.test.ts`: a spec calling `boot`, whose elapsed check lives in a module `app.ts`
///   imports, so it is production code and not reported.
/// - `perf.test.ts`: a wall-clock budget behind `skipIf(!process.env.PERF)`, listed at info.
fn budgets(name: &str) -> PathBuf {
    let root = project(name);
    write(
        &root,
        "src/test-utils/budget.ts",
        "import { expect } from 'vitest'\n\
         export async function withinBudget(work: () => Promise<void>, ms: number) {\n\
         \x20 const start = performance.now()\n\
         \x20 await work()\n\
         \x20 expect(performance.now() - start).toBeLessThan(ms)\n\
         }\n",
    );
    write(
        &root,
        "src/warm.test.ts",
        "import { test } from 'vitest'\n\
         import { warm } from './cache'\n\
         import { withinBudget } from './test-utils/budget'\n\
         test('warms within budget', async () => {\n\
         \x20 await withinBudget(warm, 50)\n\
         })\n",
    );
    write(&root, "src/cache.ts", "export async function warm() {}\n");
    write(
        &root,
        "src/timing.ts",
        "export function timed(work: () => void) {\n\
         \x20 const t0 = performance.now()\n\
         \x20 work()\n\
         \x20 if (performance.now() - t0 > 100) throw new Error('slow')\n\
         }\n",
    );
    write(
        &root,
        "src/app.ts",
        "import { timed } from './timing'\nexport function boot() { timed(() => {}) }\n",
    );
    write(
        &root,
        "src/boot.test.ts",
        "import { test, expect } from 'vitest'\n\
         import { boot } from './app'\n\
         import { timed } from './timing'\n\
         test('boots', () => {\n\
         \x20 expect(() => timed(boot)).not.toThrow()\n\
         })\n",
    );
    write(
        &root,
        "src/perf.test.ts",
        "import { test, expect } from 'vitest'\n\
         import { warm } from './cache'\n\
         test.skipIf(!process.env.PERF)('warms within budget', async () => {\n\
         \x20 const t0 = performance.now()\n\
         \x20 await warm()\n\
         \x20 expect(performance.now() - t0).toBeLessThan(5)\n\
         })\n",
    );
    root
}

#[test]
fn a_test_only_helpers_budget_is_its_callers_and_a_gated_one_is_listed() {
    let root = budgets("budgets");
    let row = |title: &str, outcome: &str| {
        (
            title.to_owned(),
            "wall-clock".to_owned(),
            outcome.to_owned(),
        )
    };
    assert_eq!(
        summary(&candidates(&root, "check")),
        [
            row("warms within budget", "listed"),
            row("warms within budget", "finding"),
        ]
    );
}

/// Install a stub package `name` under `root/node_modules`, so the resolver reads its imports
/// of `name` as an installed external dependency.
fn install(root: &Path, name: &str) {
    write(
        root,
        &format!("node_modules/{name}/package.json"),
        &format!("{{ \"name\": \"{name}\", \"main\": \"index.js\" }}\n"),
    );
    write(root, &format!("node_modules/{name}/index.js"), "\n");
}

#[test]
fn a_bare_worktree_gives_the_same_findings_as_an_installed_one() {
    let root = budgets("bare-then-installed");
    // A tsconfig that extends a preset package, as a monorepo's packages do: on a bare
    // worktree the preset is not installed and the resolver fails every relative import.
    write(
        &root,
        "tsconfig.json",
        "{ \"extends\": \"@lab/tsconfig/base.json\" }\n",
    );
    let bare = candidates(&root, "check");
    install(&root, "vitest");
    write(
        &root,
        "node_modules/@lab/tsconfig/package.json",
        "{ \"name\": \"@lab/tsconfig\" }\n",
    );
    write(
        &root,
        "node_modules/@lab/tsconfig/base.json",
        "{ \"compilerOptions\": { \"strict\": true } }\n",
    );
    let installed = candidates(&root, "check");
    assert!(!bare.is_empty());
    assert_eq!(installed, bare);
}

#[test]
fn the_findings_json_is_stable() {
    let root = budgets("json-snapshot");
    let first = candidates(&root, "check");
    assert_eq!(candidates(&root, "check"), first, "two runs, one answer");
    insta::assert_json_snapshot!(
        "check_json_flake_candidate",
        status_support::redact_status(&serde_json::Value::Array(first))
    );
}

// ---------------------------------------------------------------- poll loops (#197)

/// A test-only teardown helper that retries until a process is gone, bounded by a deadline
/// (#197's A3), registered by an `afterAll`, beside a spec that asserts on elapsed time:
///
/// - `waitUntilGone` compares `Date.now()` against a deadline in the condition of a loop that
///   also re-reads the awaited state (`isAlive()`): the clock caps the wait, and what the test
///   asserts never sees it;
/// - `elapsed.test.ts` asserts the work took under a second: its outcome is the clock's.
fn deadline_teardown(name: &str) -> PathBuf {
    let root = project(name);
    write(
        &root,
        "src/test-utils/cleanup.ts",
        "import { spawnSync } from 'node:child_process'\n\
         \n\
         export function waitUntilGone(isAlive: () => boolean): void {\n\
         \x20 const deadline = Date.now() + 2000\n\
         \x20 while (Date.now() < deadline && isAlive()) {\n\
         \x20   spawnSync('sleep', ['0.05'])\n\
         \x20 }\n\
         }\n",
    );
    write(
        &root,
        "src/a3-deadline-teardown.test.ts",
        "import { afterAll, expect, it } from 'vitest'\n\
         import { waitUntilGone } from './test-utils/cleanup'\n\
         \n\
         afterAll(() => waitUntilGone(() => false))\n\
         \n\
         it('does its work', () => {\n\
         \x20 expect(1 + 1).toBe(2)\n\
         })\n",
    );
    write(
        &root,
        "src/elapsed.test.ts",
        "import { expect, it } from 'vitest'\n\
         \n\
         it('finishes the work within a second', () => {\n\
         \x20 const start = Date.now()\n\
         \x20 let total = 0\n\
         \x20 for (let i = 0; i < 1000; i++) total += i\n\
         \x20 expect(total).toBe(499500)\n\
         \x20 expect(Date.now() - start).toBeLessThan(1000)\n\
         })\n",
    );
    root
}

#[test]
fn a_deadline_bounding_a_teardown_retry_is_not_a_clock_dependence_and_an_elapsed_assertion_is() {
    let root = deadline_teardown("wall-clock-deadline-teardown");
    assert_eq!(
        summary(&candidates(&root, "check")),
        [(
            "finishes the work within a second".to_owned(),
            "wall-clock".to_owned(),
            "finding".to_owned()
        )]
    );
}

/// A spec polling state until it is ready within a deadline (#197's A5), and one that polls
/// the same way and then asserts it finished before the deadline:
///
/// - the first compares `Date.now()` against the deadline only in the loop's condition,
///   beside the state it waits for: the clock caps the wait, and the test asserts the state;
/// - the second asserts on a clock read: whether it passes is the clock's.
fn deadline_poll(name: &str) -> PathBuf {
    let root = project(name);
    write(
        &root,
        "src/a5-deadline-poll.test.ts",
        "import { expect, it } from 'vitest'\n\
         \n\
         let calls = 0\n\
         function readState(): string {\n\
         \x20 calls += 1\n\
         \x20 return calls > 3 ? 'ready' : 'pending'\n\
         }\n\
         \n\
         it('polls until the state is ready, within a deadline', () => {\n\
         \x20 const deadline = Date.now() + 10_000\n\
         \x20 let state = readState()\n\
         \x20 while (Date.now() < deadline && state !== 'ready') {\n\
         \x20   state = readState()\n\
         \x20 }\n\
         \x20 expect(state).toBe('ready')\n\
         })\n",
    );
    write(
        &root,
        "src/within-deadline.test.ts",
        "import { expect, it } from 'vitest'\n\
         \n\
         let calls = 0\n\
         function readState(): string {\n\
         \x20 calls += 1\n\
         \x20 return calls > 3 ? 'ready' : 'pending'\n\
         }\n\
         \n\
         it('is ready before the deadline', () => {\n\
         \x20 const deadline = Date.now() + 10_000\n\
         \x20 let state = readState()\n\
         \x20 while (Date.now() < deadline && state !== 'ready') {\n\
         \x20   state = readState()\n\
         \x20 }\n\
         \x20 expect(Date.now()).toBeLessThan(deadline)\n\
         })\n",
    );
    root
}

#[test]
fn a_deadline_bounding_a_poll_in_a_test_is_not_a_clock_dependence_and_asserting_on_it_is() {
    let root = deadline_poll("wall-clock-deadline-poll");
    assert_eq!(
        summary(&candidates(&root, "check")),
        [(
            "is ready before the deadline".to_owned(),
            "wall-clock".to_owned(),
            "finding".to_owned()
        )]
    );
}
