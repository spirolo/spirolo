//! `spirolo lsp`, driven over stdio by a small Rust client on fixture projects (PRD 0006,
//! seam 2).
//!
//! The server runs the whole `check` pass on `initialized` and on every save, publishes
//! each publishable finding as a diagnostic, and sends `spirolo/checked` once a pass is
//! published. The client here frames JSON-RPC by hand and records what the server sends.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use serde_json::{json, Value};

#[allow(dead_code)]
#[path = "support/status.rs"]
mod status_support;

#[path = "support/policy.rs"]
mod policy_support;

/// How long the client waits for any one message before it calls the server stuck.
const PATIENCE: Duration = Duration::from_secs(20);

/// A running `spirolo lsp` and the messages it has sent, in order.
struct Client {
    child: Child,
    stdin: ChildStdin,
    messages: Receiver<Value>,
    next_id: u64,
    root: PathBuf,
}

impl Client {
    /// Start `spirolo lsp --all` and send `initialize` with `root` as the `rootUri`, then
    /// `initialized`.
    fn start(root: &Path) -> Self {
        Self::start_with(root, &["--all"])
    }

    /// [`Client::start`] with `args` after `spirolo lsp` instead of `--all`.
    fn start_with(root: &Path, args: &[&str]) -> Self {
        let mut client = Self::spawn_with(root, args);
        let uri = uri(root, "");
        let id = client.request(
            "initialize",
            &json!({"processId": null, "rootUri": uri, "capabilities": {}}),
        );
        let response = client.response(id);
        assert!(response["result"]["capabilities"].is_object(), "{response}");
        client.notify("initialized", &json!({}));
        client
    }

    fn spawn(root: &Path) -> Self {
        Self::spawn_with(root, &["--all"])
    }

    fn spawn_with(root: &Path, args: &[&str]) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_spirolo"))
            .arg("lsp")
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (sender, messages) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            while let Some(message) = read_message(&mut reader) {
                if sender.send(message).is_err() {
                    break;
                }
            }
        });
        Self {
            child,
            stdin,
            messages,
            next_id: 1,
            root: root.to_path_buf(),
        }
    }

    fn send(&mut self, message: &Value) {
        let body = message.to_string();
        write!(self.stdin, "Content-Length: {}\r\n\r\n{body}", body.len()).unwrap();
        self.stdin.flush().unwrap();
    }

    fn request(&mut self, method: &str, params: &Value) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        id
    }

    fn notify(&mut self, method: &str, params: &Value) {
        self.send(&json!({"jsonrpc": "2.0", "method": method, "params": params}));
    }

    fn next(&self) -> Value {
        self.messages
            .recv_timeout(PATIENCE)
            .expect("the server sent nothing in time")
    }

    /// The response to request `id`, skipping notifications.
    fn response(&self, id: u64) -> Value {
        loop {
            let message = self.next();
            if message["id"] == id && message.get("method").is_none() {
                return message;
            }
        }
    }

    /// Every message up to and including the next `spirolo/checked`, with the root replaced
    /// by `<root>` and `elapsed_ms` by `<ms>`, so a transcript is stable across runs.
    fn pass(&self) -> Vec<Value> {
        let mut transcript = Vec::new();
        loop {
            let mut message = self.next();
            let checked = message["method"] == "spirolo/checked";
            if checked {
                assert!(message["params"]["elapsed_ms"].is_u64(), "{message}");
                message["params"]["elapsed_ms"] = json!("<ms>");
            }
            let text = message
                .to_string()
                .replace(&format!("file://{}", self.root.display()), "file://<root>");
            transcript.push(serde_json::from_str(&text).unwrap());
            if checked {
                return self.with_captured_status(transcript);
            }
        }
    }
    /// Validate the real exchange against the reported table before fixing snapshot metadata.
    fn with_captured_status(&self, mut transcript: Vec<Value>) -> Vec<Value> {
        if transcript.last().unwrap()["params"]["findings"] == -1 {
            return transcript;
        }
        let (_, findings) = status_support::check(&self.root);
        for message in &mut transcript {
            if message["method"] != "textDocument/publishDiagnostics" {
                continue;
            }
            for diagnostic in message["params"]["diagnostics"].as_array_mut().unwrap() {
                let id = diagnostic["data"]["id"].as_str().unwrap();
                let finding = findings.iter().find(|finding| finding.id() == id).unwrap();
                let severity = |table| {
                    spirolo_rules::status::with_table(table, || {
                        spirolo_report::editor_severity(finding)
                            .map(spirolo_report::EditorSeverity::code)
                    })
                };
                assert_eq!(
                    diagnostic["severity"],
                    json!(severity(status_support::current())),
                    "{id}"
                );
                assert_eq!(
                    diagnostic["data"]["status"],
                    spirolo_rules::status::with_table(status_support::current(), || {
                        spirolo_rules::status::status_line(finding.rule)
                    }),
                    "{id}"
                );
                diagnostic["severity"] = json!(severity(status_support::captured::table()));
                diagnostic["data"]["status"] = json!(status_support::captured::with(|| {
                    spirolo_rules::status::status_line(finding.rule)
                }));
            }
        }
        transcript
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

fn read_message(reader: &mut impl BufRead) -> Option<Value> {
    let mut length = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some(value) = line.strip_prefix("Content-Length: ") {
            length = value.parse::<usize>().ok();
        }
    }
    let mut body = vec![0; length?];
    reader.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
}

/// `file://` + the root joined with `path`.
fn uri(root: &Path, path: &str) -> String {
    format!("file://{}/{path}", root.display())
}

/// A fresh project directory under the target tree; names are prefixed `lsp-` so no other
/// test's fixture shares the directory.
fn project(name: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("lsp-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(path.join("src")).unwrap();
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

/// A weak test (a proven rule's `warn`, so a Warning) and two tests sharing a body (an
/// unproven rule's `warn`, so Information), in two files.
fn weak_project(name: &str) -> PathBuf {
    let root = project(name);
    write(
        &root,
        "src/a.test.ts",
        "import { describe, it, expect } from 'vitest'\n\
         describe('widget panel', () => {\n\
         \x20 it('renders the widget', () => { expect(widget()).toBeTruthy() })\n\
         \x20 it('renders the widget', () => { expect(widget()).toBeDefined() })\n\
         })\n",
    );
    write(
        &root,
        "src/b.test.ts",
        "import { it, expect } from 'vitest'\n\
         it('b one', () => { expect(widget()).toBeTruthy() })\n",
    );
    root
}

/// The ids of the findings `check --all --format json` prints for `root` that are published:
/// every outcome but `listed`, on a file (a project-level finding, `file: ""`, is on none),
/// sorted.
fn check_ids(root: &Path) -> Vec<String> {
    check_ids_with(root, &["--all"])
}

/// [`check_ids`] with `args` after `check --format json` instead of `--all`.
fn check_ids_with(root: &Path, args: &[&str]) -> Vec<String> {
    let output = Command::new(env!("CARGO_BIN_EXE_spirolo"))
        .args(["check", "--format", "json"])
        .args(args)
        .arg(root)
        .output()
        .unwrap();
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    let mut ids: Vec<String> = document["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["outcome"] != "listed" && f["file"] != "")
        .map(|f| f["id"].as_str().unwrap().to_owned())
        .collect();
    ids.sort();
    ids
}

/// The `data.id` of every diagnostic a pass published, sorted.
fn published_ids(pass: &[Value]) -> Vec<String> {
    let mut ids: Vec<String> = pass
        .iter()
        .filter(|m| m["method"] == "textDocument/publishDiagnostics")
        .flat_map(|m| m["params"]["diagnostics"].as_array().unwrap().clone())
        .map(|d| d["data"]["id"].as_str().unwrap().to_owned())
        .collect();
    ids.sort();
    ids
}

#[test]
fn without_all_the_server_publishes_only_what_the_default_check_shows() {
    let root = weak_project("default-view");
    assert!(!check_ids(&root).is_empty());
    let client = Client::start_with(&root, &[]);
    let pass = client.pass();
    assert_eq!(published_ids(&pass), check_ids_with(&root, &[]));
}

#[test]
fn initialized_publishes_every_finding_then_checked() {
    let root = weak_project("initialized");
    let client = Client::start(&root);
    let pass = client.pass();
    assert_eq!(published_ids(&pass), check_ids(&root));
    insta::assert_json_snapshot!("lsp_initialized_pass", pass);
}

impl Client {
    /// Send `textDocument/didSave` for `path` under the root.
    fn save(&mut self, path: &str) {
        let uri = uri(&self.root.clone(), path);
        self.notify(
            "textDocument/didSave",
            &json!({"textDocument": {"uri": uri}}),
        );
    }
}

#[test]
fn a_save_that_clears_a_finding_publishes_its_file_empty() {
    let root = weak_project("save-clears");
    let mut client = Client::start(&root);
    client.pass();

    // b's only test now asserts a value: its weak-assertion goes, and so does the
    // duplicate-test finding in a.test.ts that pointed at it.
    write(
        &root,
        "src/b.test.ts",
        "import { it, expect } from 'vitest'\n\
         it('b one', () => { expect(widget()).toBe(3) })\n",
    );
    client.save("src/b.test.ts");
    let pass = client.pass();
    assert_eq!(published_ids(&pass), check_ids(&root));
    insta::assert_json_snapshot!("lsp_save_clears", pass);
}

#[test]
fn a_created_file_adds_a_finding_in_another_file() {
    let root = weak_project("created");
    let mut client = Client::start(&root);
    client.pass();

    // A new test file repeats a.test.ts's second body, so duplicate-test now fires in
    // a.test.ts, not in the file that changed.
    write(
        &root,
        "src/c.test.ts",
        "import { it, expect } from 'vitest'\n\
         it('c one', () => { expect(widget()).toBeDefined() })\n",
    );
    let created = uri(&root, "src/c.test.ts");
    client.notify(
        "workspace/didChangeWatchedFiles",
        &json!({"changes": [{"uri": created, "type": 1}]}),
    );
    let pass = client.pass();
    assert_eq!(published_ids(&pass), check_ids(&root));
    insta::assert_json_snapshot!("lsp_created_file", pass);
}

impl Client {
    /// The result of `textDocument/hover` at `line` (zero-based) of `path`.
    fn hover(&mut self, path: &str, line: u32) -> Value {
        let uri = uri(&self.root.clone(), path);
        let id = self.request(
            "textDocument/hover",
            &json!({"textDocument": {"uri": uri}, "position": {"line": line, "character": 0}}),
        );
        let actual = self.response(id)["result"].clone();
        let path = path.replace("%5B", "[").replace("%5D", "]");
        let (_, findings) = status_support::check(&self.root);
        let mut here: Vec<_> = findings
            .iter()
            .filter(|finding| {
                finding.location.file == path
                    && finding.location.line == line + 1
                    && finding.outcome != spirolo_rules::Outcome::Listed
            })
            .collect();
        here.sort_by_key(|finding| finding.id());
        let render = |table| {
            if here.is_empty() {
                return Value::Null;
            }
            spirolo_rules::status::with_table(table, || {
                let value = here
                    .iter()
                    .map(|finding| {
                        format!(
                            "**{}** · {}\n\n{}\n\n{}",
                            finding.rule,
                            spirolo_rules::status::status_line(finding.rule),
                            finding.message,
                            finding.explain
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n---\n\n");
                json!({"contents": {"kind": "markdown", "value": value}})
            })
        };
        assert_eq!(actual, render(status_support::current()));
        render(status_support::captured::table())
    }
}

#[test]
fn hover_on_a_finding_line_shows_every_finding_there_in_id_order() {
    let root = weak_project("hover-hit");
    let mut client = Client::start(&root);
    client.pass();
    // Line 3 (zero-based 2) carries a weak-assertion and a duplicate-test finding.
    insta::assert_json_snapshot!("lsp_hover_hit", client.hover("src/a.test.ts", 2));
}

#[test]
fn hover_on_a_line_without_a_finding_is_null() {
    let root = weak_project("hover-miss");
    let mut client = Client::start(&root);
    client.pass();
    // Line 2 (zero-based 1) opens the describe block; no finding starts there.
    assert_eq!(client.hover("src/a.test.ts", 1), Value::Null);
    assert_eq!(client.hover("src/elsewhere.ts", 0), Value::Null);
}

impl Client {
    /// Assert the server sends nothing for `wait`: no pass is running or queued.
    fn quiet(&self, wait: Duration) {
        if let Ok(message) = self.messages.recv_timeout(wait) {
            panic!("expected no message, got {message}");
        }
    }
}

#[test]
fn saves_during_a_pass_fold_into_exactly_one_more_pass() {
    let root = weak_project("coalesced");
    let mut client = Client::start(&root);
    // Pass 1 starts on `initialized`; all three saves arrive while it runs or before it
    // ends, so exactly one more pass follows.
    for _ in 0..3 {
        client.save("src/b.test.ts");
    }
    let first = client.pass();
    let second = client.pass();
    assert_eq!(first.last().unwrap()["params"]["generation"], 1);
    assert_eq!(second.last().unwrap()["params"]["generation"], 2);
    client.quiet(Duration::from_secs(1));
}

#[test]
fn opening_and_editing_a_file_do_not_run_a_pass() {
    let root = weak_project("open-change");
    let mut client = Client::start(&root);
    client.pass();
    let a = uri(&root, "src/a.test.ts");
    client.notify(
        "textDocument/didOpen",
        &json!({"textDocument": {"uri": a, "languageId": "typescript", "version": 1, "text": ""}}),
    );
    client.notify(
        "textDocument/didChange",
        &json!({"textDocument": {"uri": a, "version": 2}, "contentChanges": [{"text": "x"}]}),
    );
    client.quiet(Duration::from_secs(1));
}

#[test]
fn a_config_that_cannot_be_read_publishes_nothing_and_checks_minus_one() {
    let root = weak_project("bad-config");
    let mut client = Client::start(&root);
    client.pass();

    write(&root, ".spirolorc.jsonc", "{ \"rules\": ");
    client.save(".spirolorc.jsonc");
    let failed = client.pass();
    insta::assert_json_snapshot!("lsp_bad_config", failed);

    // Repaired, the next pass publishes again.
    std::fs::remove_file(root.join(".spirolorc.jsonc")).unwrap();
    client.save(".spirolorc.jsonc");
    let repaired = client.pass();
    assert_eq!(published_ids(&repaired), check_ids(&root));
    assert_eq!(repaired.last().unwrap()["params"]["generation"], 3);
}

#[test]
fn shutdown_then_exit_ends_the_server_cleanly() {
    let root = weak_project("shutdown");
    let mut client = Client::start(&root);
    client.pass();
    let id = client.request("shutdown", &Value::Null);
    let response = client.response(id);
    assert_eq!(response["result"], Value::Null, "{response}");
    assert!(response.get("error").is_none(), "{response}");
    client.notify("exit", &Value::Null);

    let started = std::time::Instant::now();
    let status = loop {
        if let Some(status) = client.child.try_wait().unwrap() {
            break status;
        }
        assert!(started.elapsed() < PATIENCE, "the server did not exit");
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(status.code(), Some(0));
}

#[test]
fn the_root_comes_from_the_first_workspace_folder_without_a_root_uri() {
    let root = weak_project("workspace-folder");
    let mut client = Client::spawn(&root);
    let folder = uri(&root, "");
    let id = client.request(
        "initialize",
        &json!({"processId": null, "rootUri": null, "capabilities": {},
                "workspaceFolders": [{"uri": folder, "name": "fixture"}]}),
    );
    client.response(id);
    client.notify("initialized", &json!({}));
    let pass = client.pass();
    assert_eq!(published_ids(&pass).len(), 4);
    assert_eq!(published_ids(&pass), check_ids(&root));
}

#[test]
fn a_baselined_project_publishes_only_what_check_surfaces() {
    let root = weak_project("baselined");
    let ratchet = Command::new(env!("CARGO_BIN_EXE_spirolo"))
        .arg("ratchet")
        .arg(&root)
        .output()
        .unwrap();
    assert!(ratchet.status.success());
    let mut client = Client::start(&root);
    let pass = client.pass();
    assert_eq!(published_ids(&pass), Vec::<String>::new());
    assert_eq!(check_ids(&root), Vec::<String>::new());

    // A new weak test rises above the recorded count: check surfaces the file's findings,
    // and so does the server.
    write(
        &root,
        "src/b.test.ts",
        "import { it, expect } from 'vitest'\n\
         it('b one', () => { expect(widget()).toBeTruthy() })\n\
         it('b two', () => { expect(widget()).toBeDefined() })\n",
    );
    client.save("src/b.test.ts");
    let pass = client.pass();
    assert!(!published_ids(&pass).is_empty());
    assert_eq!(published_ids(&pass), check_ids(&root));
}

/// Every diagnostic a pass published as `file line code severity id`, one per line, the
/// multiset the oracle compares.
fn compact(pass: &[Value]) -> String {
    let mut rows = Vec::new();
    for message in pass {
        if message["method"] != "textDocument/publishDiagnostics" {
            continue;
        }
        let uri = message["params"]["uri"].as_str().unwrap();
        for d in message["params"]["diagnostics"].as_array().unwrap() {
            rows.push(format!(
                "{uri} {} {} {} {}",
                d["range"]["start"]["line"],
                d["code"].as_str().unwrap(),
                d["severity"],
                d["data"]["id"].as_str().unwrap()
            ));
        }
    }
    rows.join("\n")
}

#[test]
fn listed_rows_are_not_published_and_undecided_ones_are_hints() {
    let root = project("outcomes");
    write(&root, "src/widget.ts", "export const w = 1\n");
    // An unresolved import leaves the mock undecided; `it.skip` and `it.skipIf` leave
    // dead-skip's listed rows beside its findings.
    write(
        &root,
        "src/a.test.ts",
        "import { it, expect, vi } from 'vitest'\n\
         import { thing } from './missing'\n\
         vi.mock('./widget')\n\
         it('uses thing', () => { expect(thing()).toBe(3) })\n\
         it.skip('later', () => { expect(thing()).toBe(3) })\n\
         it.skipIf(process.env.CI)('local only', () => { expect(thing()).toBe(4) })\n",
    );
    let client = Client::start(&root);
    let pass = client.pass();
    assert_eq!(published_ids(&pass), check_ids(&root));
    insta::assert_snapshot!("lsp_outcomes", compact(&pass));
}

/// The captured table with `rule` at `verdict` and `truths`/`falses` decided rows.
fn with_row(
    rule: &str,
    verdict: spirolo_rules::status::Verdict,
    truths: u32,
    falses: u32,
) -> &'static spirolo_rules::status::Table {
    let mut table = status_support::captured::table().clone();
    let row = table.rules.get_mut(rule).unwrap();
    row.verdict = verdict;
    row.truths = truths;
    row.falses = falses;
    Box::leak(Box::new(table))
}

#[test]
fn advisory_info_rows_are_information_and_rows_shown_only_under_all_are_hints() {
    use spirolo_rules::status::{Tier, Verdict};
    let root = weak_project("advisory-severity");
    // `pass` checks each published severity is `editor_severity` under the binary's table.
    let pass = Client::start(&root).pass();
    assert_eq!(published_ids(&pass), check_ids(&root));
    let (_, findings) = status_support::check(&root);
    let rows: Vec<_> = findings
        .iter()
        .filter(|f| f.rule == "weak-assertion" && f.outcome == spirolo_rules::Outcome::Finding)
        .collect();
    assert!(!rows.is_empty());
    assert!(rows
        .iter()
        .all(|f| f.severity == spirolo_rules::Severity::Info));
    let published = |table| {
        spirolo_rules::status::with_table(table, || {
            let tier = spirolo_rules::status::tier("weak-assertion");
            let severities: Vec<_> = rows
                .iter()
                .map(|f| spirolo_report::editor_severity(f).unwrap())
                .collect();
            (tier, severities)
        })
    };
    let information = vec![spirolo_report::EditorSeverity::Information; rows.len()];
    let hint = vec![spirolo_report::EditorSeverity::Hint; rows.len()];
    // Advisory: unproven, 19/0 decided, shown by default.
    assert_eq!(
        published(with_row("weak-assertion", Verdict::Unadjudicated, 19, 0)),
        (Tier::Advisory, information)
    );
    // Experimental: 14 decided is under the floor, shown only under `--all`.
    assert_eq!(
        published(with_row("weak-assertion", Verdict::Insufficient, 14, 0)),
        (Tier::Experimental, hint.clone())
    );
    // FAIL and DROPPED are hidden, also shown only under `--all`.
    for verdict in [Verdict::Fail, Verdict::Dropped] {
        assert_eq!(
            published(with_row("weak-assertion", verdict, 19, 0)),
            (Tier::Hidden, hint.clone())
        );
    }
}

#[test]
fn a_column_after_wide_characters_is_a_utf16_offset() {
    let root = project("utf16");
    let line = "describe('😀é', () => { it('x', () => { expect(w()).toBeTruthy() }) })";
    write(
        &root,
        "src/a.test.ts",
        &format!("import {{ describe, it, expect }} from 'vitest'\n{line}\n"),
    );
    let client = Client::start(&root);
    let pass = client.pass();
    let diagnostic = pass
        .iter()
        .filter(|m| m["params"]["uri"] == "file://<root>/src/a.test.ts")
        .flat_map(|m| m["params"]["diagnostics"].as_array().unwrap().clone())
        .find(|d| d["code"] == "weak-assertion")
        .unwrap();
    // `it` starts at byte 27 (column 28): 10 ASCII bytes, a 4-byte emoji (2 UTF-16 units), a
    // 2-byte `é` (1 unit), then 11 ASCII bytes.
    assert_eq!(
        diagnostic["range"],
        json!({"start": {"line": 1, "character": 24},
               "end": {"line": 1, "character": line.encode_utf16().count()}})
    );
}

#[test]
fn a_bracketed_path_is_published_as_written_and_hovered_encoded() {
    let root = project("brackets");
    write(
        &root,
        "src/[id].test.ts",
        "import { it, expect } from 'vitest'\n\
         it('route', () => { expect(page()).toBeTruthy() })\n",
    );
    let mut client = Client::start(&root);
    let pass = client.pass();
    let uris: Vec<&str> = pass
        .iter()
        .filter_map(|m| m["params"]["uri"].as_str())
        .collect();
    // `file://` + the root joined with the path, as the PRD writes it: no encoding.
    assert!(uris.contains(&"file://<root>/src/[id].test.ts"), "{uris:?}");
    // An editor sends the same file percent-encoded; hover still finds it.
    let hover = client.hover("src/%5Bid%5D.test.ts", 1);
    assert!(hover["contents"]["value"]
        .as_str()
        .unwrap()
        .starts_with("**weak-assertion**"));
}

/// A paired test with a weak assertion, a skip (a `dead-skip` finding and its listed row) and
/// a mock of a module that does not resolve (undecided), beside a drifted msw handler.
fn sidebar_project(name: &str) -> PathBuf {
    let root = project(name);
    write(
        &root,
        "src/config/api.ts",
        "export const urlApi = 'http://localhost:5000/v1'\n",
    );
    write(
        &root,
        "src/mocks/handlers.ts",
        "import { http, HttpResponse } from 'msw'\n\
         export const handlers = [\n\
         \x20 http.get('*/api/v1/materials', () => HttpResponse.json({})),\n\
         ]\n",
    );
    write(
        &root,
        ".spirolorc.jsonc",
        "{\n  \"urlSources\": [\"src/config/api.ts\"],\n  \"handlers\": [\"src/mocks/**\"],\n  \"rules\": {\"weak-assertion\": \"info\", \"dead-skip\": \"info\", \"unused-mock\": \"info\"}\n}\n",
    );
    write(&root, "src/widget.ts", "export const widget = () => 1\n");
    write(
        &root,
        "src/widget.test.ts",
        "import { it, expect, vi } from 'vitest'\n\
         import { widget } from './widget'\n\
         vi.mock('./missing')\n\
         it('renders the widget', () => { expect(widget()).toBeTruthy() })\n\
         it.skip('later', () => { expect(widget()).toBe(1) })\n",
    );
    root
}

impl Client {
    /// Send a request and wait for its result.
    fn ask(&mut self, method: &str, params: &Value) -> Value {
        let id = self.request(method, params);
        let response = self.response(id);
        assert!(response.get("error").is_none(), "{method}: {response}");
        let actual = response["result"].clone();
        if method == "spirolo/tree" {
            let (model, findings) = status_support::check(&self.root);
            let baseline = actual["baseline"]
                .as_u64()
                .map(|n| usize::try_from(n).unwrap());
            let render = |table| {
                spirolo_rules::status::with_table(table, || {
                    serde_json::to_value(spirolo_report::sidebar_tree(&model, &findings, baseline))
                        .unwrap()
                })
            };
            assert_eq!(actual, render(status_support::current()));
            return render(status_support::captured::table());
        }
        if method == "spirolo/rules" && actual.is_object() {
            let (model, findings) = status_support::check(&self.root);
            let scope = match (params["family"].as_str(), params["rule"].as_str()) {
                (None, Some(rule)) => spirolo_report::RulesScope::Rule(
                    spirolo_rules::rule_named(rule).unwrap().name(),
                ),
                (Some(family), None) => spirolo_report::RulesScope::Family(
                    spirolo_rules::registry()
                        .iter()
                        .map(|rule| rule.family())
                        .find(|candidate| candidate.name() == family)
                        .unwrap(),
                ),
                _ => spirolo_report::RulesScope::Summary,
            };
            let render = |table| {
                spirolo_rules::status::with_table(table, || {
                    serde_json::to_value(spirolo_report::sidebar_rules(&model, &findings, scope))
                        .unwrap()
                })
            };
            assert_eq!(actual, render(status_support::current()));
            return render(status_support::captured::table());
        }
        if method == "spirolo/health" && actual.is_object() {
            let (model, findings) = status_support::check(&self.root);
            let render = |table| {
                serde_json::to_value(spirolo_report::score::health(&findings, &model, table))
                    .unwrap()
            };
            assert_eq!(actual, render(status_support::current()));
            return render(status_support::captured::table());
        }
        if method == "spirolo/file" {
            let (model, findings) = status_support::check(&self.root);
            let file = params["uri"]
                .as_str()
                .unwrap()
                .strip_prefix(&format!("file://{}/", self.root.display()))
                .unwrap();
            let render = |table| {
                spirolo_rules::status::with_table(table, || {
                    serde_json::to_value(spirolo_report::sidebar_file(&model, &findings, file))
                        .unwrap()
                })
            };
            assert_eq!(actual, render(status_support::current()));
            return render(status_support::captured::table());
        }
        actual
    }
}

/// `value` with the fixture's absolute root replaced by `<root>`.
fn rooted(value: &Value, root: &Path) -> Value {
    let text = serde_json::to_string(value).unwrap();
    serde_json::from_str(&text.replace(&root.display().to_string(), "<root>")).unwrap()
}

#[test]
fn the_tree_request_answers_the_last_passes_tree() {
    let root = sidebar_project("tree");
    let mut client = Client::start(&root);
    client.pass();
    let tree = client.ask("spirolo/tree", &Value::Null);
    assert_eq!(
        tree,
        client.ask("spirolo/tree", &Value::Null),
        "byte-equal twice"
    );
    assert_eq!(tree["root"], root.display().to_string());
    insta::assert_json_snapshot!("lsp_tree", rooted(&tree, &root));
}

#[test]
fn a_project_level_finding_is_published_on_no_uri() {
    let root = sidebar_project("project-level");
    let pass = Client::start(&root).pass();
    let output = Command::new(env!("CARGO_BIN_EXE_spirolo"))
        .args(["check", "--all", "--format", "json"])
        .arg(&root)
        .output()
        .unwrap();
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    let project_level = document["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|finding| finding["file"] == "")
        .count();
    assert_eq!(
        project_level, 1,
        "the fixture carries a project-level finding"
    );
    let uris: Vec<&str> = pass
        .iter()
        .filter(|message| message["method"] == "textDocument/publishDiagnostics")
        .map(|message| message["params"]["uri"].as_str().unwrap())
        .collect();
    assert!(!uris.is_empty(), "{pass:?}");
    assert!(
        uris.iter().all(|uri| uri.starts_with("file://<root>/")),
        "{uris:?}"
    );
}

#[test]
fn a_project_level_finding_is_the_trees_project_row_not_a_file_node() {
    let root = sidebar_project("tree-project");
    let mut client = Client::start(&root);
    client.pass();
    let tree = client.ask("spirolo/tree", &Value::Null);
    let paths: Vec<&str> = tree["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| node["path"].as_str().unwrap())
        .collect();
    assert!(!paths.contains(&""), "{paths:?}");
    assert_eq!(
        tree["project"],
        json!({"counts": {"error": 0, "warn": 0, "info": 1, "hint": 0}, "state": "info"})
    );
}

#[test]
fn the_file_request_answers_a_test_files_rows_and_a_sources_rows() {
    let root = sidebar_project("file");
    let mut client = Client::start(&root);
    client.pass();
    let test = client.ask(
        "spirolo/file",
        &json!({"uri": uri(&root, "src/widget.test.ts")}),
    );
    let again = client.ask(
        "spirolo/file",
        &json!({"uri": uri(&root, "src/widget.test.ts")}),
    );
    assert_eq!(test, again, "byte-equal twice");
    insta::assert_json_snapshot!("lsp_file_test", test);
    let handlers = client.ask(
        "spirolo/file",
        &json!({"uri": uri(&root, "src/mocks/handlers.ts")}),
    );
    insta::assert_json_snapshot!("lsp_file_source", handlers);
}

#[test]
fn the_tree_counts_the_baselines_entries() {
    let root = weak_project("tree-baseline");
    let ratchet = Command::new(env!("CARGO_BIN_EXE_spirolo"))
        .arg("ratchet")
        .arg(&root)
        .output()
        .unwrap();
    assert!(ratchet.status.success());
    let baseline: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join(".spirolo/baseline.json")).unwrap(),
    )
    .unwrap();
    let entries = baseline["files"]
        .as_object()
        .unwrap()
        .values()
        .map(|rules| rules.as_object().unwrap().len())
        .sum::<usize>()
        + baseline["ratios"]
            .as_object()
            .map_or(0, serde_json::Map::len);
    let mut client = Client::start(&root);
    client.pass();
    let tree = client.ask("spirolo/tree", &Value::Null);
    assert!(entries > 0);
    assert_eq!(tree["baseline"], entries);
}

#[test]
fn the_rules_summary_lists_every_rule_of_every_family_without_files() {
    let root = sidebar_project("rules-summary");
    let mut client = Client::start(&root);
    client.pass();
    let summary = client.ask("spirolo/rules", &Value::Null);
    let rules = summary["rules"].as_array().unwrap();
    let mut listed: Vec<_> = rules
        .iter()
        .map(|rule| {
            (
                rule["family"].as_str().unwrap(),
                rule["rule"].as_str().unwrap(),
            )
        })
        .collect();
    listed.sort_unstable();
    let mut registered: Vec<_> = spirolo_rules::registry()
        .iter()
        .map(|rule| (rule.family().name(), rule.name()))
        .collect();
    registered.sort_unstable();
    assert_eq!(listed, registered, "every rule once, empty ones included");
    assert!(rules.iter().all(|rule| rule["files"].is_null()));
    let file_count =
        |name: &str| rules.iter().find(|rule| rule["rule"] == name).unwrap()["file_count"].as_u64();
    // The test file and the project entry (`path: ""`) that carries the ratio row.
    assert_eq!(file_count("weak-assertion"), Some(2));
    assert_eq!(file_count("flake-observed"), Some(0));
}

#[test]
fn a_family_answers_every_rule_of_that_family_with_its_files() {
    let root = sidebar_project("rules-family");
    let mut client = Client::start(&root);
    client.pass();
    let gate = client.ask("spirolo/rules", &json!({"family": "gate"}));
    let rules = gate["rules"].as_array().unwrap();
    let mut names: Vec<_> = rules
        .iter()
        .map(|rule| rule["rule"].as_str().unwrap())
        .collect();
    names.sort_unstable();
    assert_eq!(names, ["dead-skip", "fail-closed-setup", "gate-truth"]);
    for rule in rules {
        assert_eq!(rule["family"], "gate");
        assert_eq!(
            rule["files"].as_array().unwrap().len() as u64,
            rule["file_count"].as_u64().unwrap()
        );
    }
    insta::assert_json_snapshot!("lsp_rules_gate", rooted(&gate, &root));
}

#[test]
fn a_rule_answers_that_rule_alone_with_its_files() {
    let root = sidebar_project("rules-rule");
    let mut client = Client::start(&root);
    client.pass();
    let gate = client.ask("spirolo/rules", &json!({"family": "gate"}));
    let dead_skip = client.ask("spirolo/rules", &json!({"rule": "dead-skip"}));
    assert_eq!(
        dead_skip["rules"],
        json!([gate["rules"]
            .as_array()
            .unwrap()
            .iter()
            .find(|rule| rule["rule"] == "dead-skip")
            .unwrap()]),
        "the same row the family answer carries"
    );
    let empty = client.ask("spirolo/rules", &json!({"rule": "flake-observed"}));
    assert_eq!(empty["rules"][0]["files"], json!([]));
    assert_eq!(empty["rules"].as_array().unwrap().len(), 1);
}

#[test]
fn an_unknown_family_or_rule_is_invalid_params() {
    let root = sidebar_project("rules-unknown");
    let mut client = Client::start(&root);
    client.pass();
    for params in [
        json!({"family": "tests"}),
        json!({"rule": "no-such-rule"}),
        json!({"family": "gate", "rule": "dead-skip"}),
        json!({"family": 3}),
    ] {
        let id = client.request("spirolo/rules", &params);
        let response = client.response(id);
        assert_eq!(response["error"]["code"], -32602, "{params}: {response}");
    }
}

#[test]
fn a_rules_status_is_the_bare_status_line_without_the_measured_corpus() {
    let root = sidebar_project("rules-status");
    let mut client = Client::start(&root);
    client.pass();
    let id = client.request("spirolo/rules", &Value::Null);
    let summary = client.response(id)["result"].clone();
    for rule in summary["rules"].as_array().unwrap() {
        let name = rule["rule"].as_str().unwrap();
        let mut bare = spirolo_rules::status::status(name).clone();
        bare.measured_on.clear();
        assert_eq!(rule["status"], bare.line(), "{name}");
    }
}

#[test]
fn health_scores_every_row_whichever_the_server_publishes() {
    let root = weak_project("health-default");
    let health = |args: &[&str]| {
        let mut client = Client::start_with(&root, args);
        client.pass();
        client.ask("spirolo/health", &Value::Null)
    };
    let all = health(&["--all"]);
    assert!(all["score"].as_f64().unwrap() < 100.0, "{all}");
    assert_eq!(health(&[]), all);
}

#[test]
fn health_is_null_before_a_good_pass_then_follows_every_pass() {
    let root = weak_project("health");
    write(&root, ".spirolorc.jsonc", "{ \"rules\": ");
    let mut client = Client::start(&root);
    client.pass();
    assert_eq!(client.ask("spirolo/health", &Value::Null), Value::Null);

    policy_support::info(&root);
    client.save(".spirolorc.jsonc");
    client.pass();
    let before = client.ask("spirolo/health", &Value::Null);
    assert_eq!(before["formula_version"], 1);
    insta::assert_json_snapshot!("lsp_health", before);

    // Two tests that shared a body now differ: the save moves the score.
    write(
        &root,
        "src/a.test.ts",
        "import { describe, it, expect } from 'vitest'\n\
         describe('widget panel', () => {\n\
         \x20 it('renders the widget', () => { expect(widget()).toBe(1) })\n\
         })\n",
    );
    client.save("src/a.test.ts");
    client.pass();
    let after = client.ask("spirolo/health", &Value::Null);
    assert_ne!(after["score"], before["score"]);
}
