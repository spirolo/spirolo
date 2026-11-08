//! `spirolo init` end to end, and `spirolo list`'s reading of the `.spirolorc.jsonc` it writes.
//!
//! The generated file is snapshotted like code; the behavioural criteria — refuse to
//! overwrite, list the detected stack, honour a `handlers` glob, survive malformed JSONC —
//! are asserted against the binary a user actually runs.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(dead_code)]
#[path = "support/status.rs"]
mod status_support;

fn spirolo() -> Command {
    Command::cargo_bin("spirolo").unwrap()
}

/// Assert generated severities against the actual public status, then redact only those
/// measured defaults so later adjudications cannot invalidate the config layout snapshots.
fn checked_defaults(mut written: String) -> String {
    for rule in spirolo_rules::registry() {
        let actual = format!("\"{}\": \"{}\"", rule.name(), expected_default(rule.name()));
        assert!(
            written.contains(&actual),
            "generated severity for {}",
            rule.name()
        );
        written = written.replace(
            &actual,
            &format!("\"{}\": \"<default-severity>\"", rule.name()),
        );
    }
    written
}

fn expected_default(name: &str) -> &'static str {
    if status_support::current().rules[name].verdict == spirolo_rules::status::Verdict::Pass {
        spirolo_rules::rule_named(name)
            .unwrap()
            .default_severity()
            .name()
    } else {
        "info"
    }
}

/// A fresh empty directory named after the test that asked for one.
fn project(name: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}

/// Write `contents` to `root/relative`, creating parent directories as needed.
fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

/// A project with a package.json, a tsconfig with a path alias, and a Vitest config that
/// names a setup file: enough for detection to have something to report.
fn detected_stack(name: &str) -> PathBuf {
    let root = project(name);
    write(
        &root,
        "package.json",
        r#"{
          "name": "example",
          "devDependencies": {
            "vitest": "^2.1.9",
            "@playwright/test": "1.47.0",
            "msw": "~2.4.11"
          }
        }
        "#,
    );
    write(&root, "pnpm-lock.yaml", "lockfileVersion: '9.0'\n");
    write(
        &root,
        "tsconfig.json",
        r#"{
          // a path alias the resolver honours
          "compilerOptions": { "paths": { "@/*": ["./src/*"] } }
        }
        "#,
    );
    write(
        &root,
        "vitest.config.ts",
        "import { defineConfig } from 'vitest/config'\n\
         export default defineConfig({ test: { setupFiles: ['./test/setup.ts'] } })\n",
    );
    root
}

// ---------------------------------------------------------------------- the generated file

#[test]
fn init_writes_a_jsonc_file_where_every_key_is_documented() {
    let root = detected_stack("init-documented");

    spirolo().arg("init").arg(&root).assert().success();

    let written = std::fs::read_to_string(root.join(".spirolorc.jsonc")).unwrap();
    for key in [
        "handlers",
        "dataLayer",
        "dataLayerScope",
        "urlSources",
        "env",
        "claims",
        "inputs",
        "rules",
    ] {
        assert!(
            key_is_commented(&written, key),
            "key {key:?} has no comment above it in:\n{written}"
        );
    }
}

/// Whether the object key `key` is immediately preceded (ignoring blank lines) by a `//`
/// comment line, which is what "every default explained in place" means for the file.
fn key_is_commented(text: &str, key: &str) -> bool {
    let needle = format!("\"{key}\":");
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = lines
        .iter()
        .position(|line| line.trim_start().starts_with(&needle))
    else {
        return false;
    };
    lines[..at]
        .iter()
        .rev()
        .find(|line| !line.trim().is_empty())
        .is_some_and(|line| line.trim_start().starts_with("//"))
}

#[test]
fn init_writes_the_same_bytes_for_the_same_stack() {
    let root = detected_stack("init-snapshot");

    spirolo().arg("init").arg(&root).assert().success();

    let written = std::fs::read_to_string(root.join(".spirolorc.jsonc")).unwrap();
    insta::assert_snapshot!("spirolorc_for_a_vitest_stack", checked_defaults(written));
}

// ---------------------------------------------------------------- detecting the rule inputs

/// A project that gives every M2 detector something to find: MSW handlers in a directory,
/// a base-URL-named export, a spec that module-mocks a client, and a doc that claims a gate.
fn rule_inputs(name: &str) -> PathBuf {
    let root = project(name);
    write(
        &root,
        "package.json",
        r#"{ "devDependencies": { "vitest": "^2.1.9", "msw": "~2.4.11" } }"#,
    );
    write(&root, "pnpm-lock.yaml", "lockfileVersion: '9.0'\n");
    // An MSW handler under a `mocks` directory: the model finds it, so `init` proposes the
    // directory as a `handlers` glob.
    write(
        &root,
        "src/mocks/handlers/users.ts",
        "import { http, HttpResponse } from 'msw'\n\
         export const handlers = [ http.get('/api/users', () => HttpResponse.json({})) ]\n",
    );
    // A production module that exports a base-URL-named binding: an `urlSources` candidate.
    write(
        &root,
        "src/config/app.ts",
        "const urlApi = `${process.env.API_URL}/v1`\nexport { urlApi }\n",
    );
    // A spec that module-mocks the client at file scope: a `dataLayer` candidate.
    write(
        &root,
        "src/feature.integration.test.ts",
        "import { vi } from 'vitest'\n\
         vi.mock('@/supabaseClient')\n\
         test('flow', () => {})\n",
    );
    // A doc that claims a gate: a `claims` candidate.
    write(
        &root,
        "docs/TESTING_PLAYBOOK.md",
        "# Playbook\n\nCoverage thresholds are enforced in CI.\n",
    );
    root
}

#[test]
fn init_detects_and_writes_the_rule_inputs() {
    let root = rule_inputs("init-inputs");

    spirolo().arg("init").arg(&root).assert().success();

    let written = std::fs::read_to_string(root.join(".spirolorc.jsonc")).unwrap();
    // handlers: the directory the handler lives in, as a live glob.
    assert!(
        written.contains("\"handlers\": [\"src/mocks/handlers/**\"]"),
        "handlers glob not proposed:\n{written}"
    );
    // urlSources: the base-URL-named export, commented for review.
    assert!(
        written.contains("// \"src/config/app#urlApi\""),
        "urlApi not proposed as a urlSource:\n{written}"
    );
    // dataLayer: the module-mocked client, commented for review.
    assert!(
        written.contains("// \"@/supabaseClient\""),
        "supabase client not proposed as a dataLayer candidate:\n{written}"
    );
    // claims: the doc that mentions a gate, commented for review.
    assert!(
        written.contains("// \"docs/TESTING_PLAYBOOK.md\""),
        "playbook not proposed as a claim:\n{written}"
    );
    // rules: every registered rule with its default severity, live.
    assert!(
        written.contains(&format!(
            "\"weak-assertion\": \"{}\"",
            expected_default("weak-assertion")
        )),
        "rules block missing weak-assertion default:\n{written}"
    );
    // dataLayerScope and env are written live with their defaults.
    assert!(
        written.contains("\"dataLayerScope\": [\"**/*.integration.*\""),
        "default dataLayerScope not written:\n{written}"
    );
    assert!(
        written.contains("\"env\": {}"),
        "empty env not written:\n{written}"
    );
}

#[test]
fn the_written_file_round_trips_through_the_config_reader_with_no_diagnostics() {
    let root = rule_inputs("init-roundtrip");

    spirolo().arg("init").arg(&root).assert().success();

    // `check` reads the config `init` just wrote; a malformed or bad-glob config would print
    // an `spirolorc_` diagnostic. None must appear: every commented candidate is inert, the
    // live `handlers` glob compiles, and the file parses.
    spirolo()
        .args(["check", "--all"])
        .arg(&root)
        .assert()
        .stdout(predicate::str::contains("spirolorc_").not());
}

#[test]
fn init_writes_the_same_bytes_for_a_project_with_rule_inputs() {
    let root = rule_inputs("init-inputs-snapshot");

    spirolo().arg("init").arg(&root).assert().success();

    let written = std::fs::read_to_string(root.join(".spirolorc.jsonc")).unwrap();
    insta::assert_snapshot!("spirolorc_with_rule_inputs", checked_defaults(written));
}

// ------------------------------------------------------------------------- refuse to clobber

#[test]
fn init_refuses_to_overwrite_without_force() {
    let root = project("init-no-clobber");
    write(
        &root,
        ".spirolorc.jsonc",
        "{ \"handlers\": [\"mine/**\"] }\n",
    );

    spirolo().arg("init").arg(&root).assert().code(2).stderr(
        predicate::str::contains("already exists").and(predicate::str::contains("--force")),
    );

    // The user's file is left exactly as it was.
    let after = std::fs::read_to_string(root.join(".spirolorc.jsonc")).unwrap();
    assert_eq!(after, "{ \"handlers\": [\"mine/**\"] }\n");
}

#[test]
fn init_overwrites_with_force() {
    let root = project("init-force");
    write(&root, ".spirolorc.jsonc", "stale\n");

    spirolo()
        .args(["init", "--force"])
        .arg(&root)
        .assert()
        .success();

    let after = std::fs::read_to_string(root.join(".spirolorc.jsonc")).unwrap();
    assert!(
        after.contains("\"handlers\""),
        "the file was rewritten: {after}"
    );
}

// -------------------------------------------------------------------- the detection summary

#[test]
fn init_reports_the_detected_stack_with_versions_setup_and_aliases() {
    let root = detected_stack("init-detection");

    spirolo().arg("init").arg(&root).assert().success().stdout(
        predicate::str::contains("pnpm")
            .and(predicate::str::contains("vitest 2.1.9"))
            .and(predicate::str::contains("@playwright/test 1.47.0"))
            .and(predicate::str::contains("msw 2.4.11"))
            .and(predicate::str::contains("test/setup.ts"))
            .and(predicate::str::contains("@/*")),
    );
}

#[test]
fn init_on_a_bare_project_reports_nothing_detected_but_still_writes() {
    let root = project("init-bare");

    spirolo()
        .arg("init")
        .arg(&root)
        .assert()
        .success()
        .stdout(predicate::str::contains("none"));
    assert!(root.join(".spirolorc.jsonc").is_file());
}

// ---------------------------------------------------------------- list honours the rc file

#[test]
fn list_honours_a_handlers_glob_from_the_rc_file() {
    let root = project("rc-handlers");
    // A handler file outside the default `mocks`/`msw` directories: the default gate would
    // never read it.
    write(
        &root,
        "src/testing/server.ts",
        "import { http, HttpResponse } from 'msw'\n\
         export const handlers = [ http.get('/api/user', () => HttpResponse.json({})) ]\n",
    );

    // Without a config, the file is not a handler file.
    spirolo()
        .arg("list")
        .arg(&root)
        .assert()
        .success()
        .stdout(predicate::str::contains("· 0 handlers ·"));

    // A `handlers` glob naming its directory makes it one.
    write(
        &root,
        ".spirolorc.jsonc",
        "{\n  // where handlers live\n  \"handlers\": [\"src/testing/**\"]\n}\n",
    );

    spirolo().arg("list").arg(&root).assert().success().stdout(
        predicate::str::contains("· 1 handlers ·").and(predicate::str::contains(
            "src/testing/server.ts:2  GET  /api/user",
        )),
    );
}

#[test]
fn an_invalid_handlers_glob_is_reported_rather_than_silently_dropped() {
    let root = project("rc-bad-glob");
    write(
        &root,
        ".spirolorc.jsonc",
        "{ \"handlers\": [\"src/[bad\"] }\n",
    );

    spirolo().arg("list").arg(&root).assert().success().stdout(
        predicate::str::contains(".spirolorc.jsonc ignoring handler glob")
            .and(predicate::str::contains("src/[bad"))
            .and(predicate::str::contains("[glob_invalid]")),
    );
}

#[test]
fn a_malformed_rc_file_is_a_diagnostic_with_a_location_not_a_panic() {
    let root = project("rc-malformed");
    // A trailing key with no value: valid neither as JSON nor JSONC.
    write(
        &root,
        ".spirolorc.jsonc",
        "{\n  \"handlers\": [\n  \"oops\"\n  \n",
    );

    spirolo().arg("list").arg(&root).assert().success().stdout(
        predicate::str::contains("diagnostics")
            .and(predicate::str::contains(".spirolorc.jsonc:"))
            .and(predicate::str::contains("[spirolorc_parse_failed]")),
    );
}

#[test]
fn a_malformed_rc_file_falls_back_to_defaults() {
    let root = project("rc-malformed-defaults");
    write(
        &root,
        "src/mocks/handlers.ts",
        "import { http, HttpResponse } from 'msw'\n\
         export const handlers = [ http.get('/api/user', () => HttpResponse.json({})) ]\n",
    );
    write(&root, ".spirolorc.jsonc", "{ not json");

    // The broken file does not suppress the default mocks/msw handler recognition.
    spirolo()
        .arg("list")
        .arg(&root)
        .assert()
        .success()
        .stdout(predicate::str::contains("· 1 handlers ·"));
}

#[test]
fn init_writes_inputs_mutation_with_a_detected_report_path() {
    let root = project("init-mutation-detected");
    write(&root, "packages/a/reports/mutation/mutation.json", "{}");

    spirolo().arg("init").arg(&root).assert().success();

    let written = std::fs::read_to_string(root.join(".spirolorc.jsonc")).unwrap();
    assert!(
        written.contains("\"mutation\": [\"packages/a/reports/mutation/mutation.json\"]"),
        "detected report path missing from:\n{written}"
    );
}

#[test]
fn init_with_no_report_writes_the_default_mutation_glob() {
    let root = project("init-mutation-default");

    spirolo().arg("init").arg(&root).assert().success();

    let written = std::fs::read_to_string(root.join(".spirolorc.jsonc")).unwrap();
    assert!(
        written.contains("\"mutation\": [\"**/reports/mutation/mutation.json\"]"),
        "default mutation glob missing from:\n{written}"
    );
}
