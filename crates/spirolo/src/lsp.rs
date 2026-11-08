//! `spirolo lsp`: a Language Server over stdio (PRD 0006, Passes, Diagnostics, Hover and the
//! sidebar's `spirolo/tree`, `spirolo/rules`, `spirolo/health` and `spirolo/file`).
//!
//! Every pass is the whole `check` pass: the model is built from disk, the rules run, the
//! baseline filters, and each publishable finding becomes one diagnostic. The sidebar
//! requests answer from the last good pass. The transport is
//! `lsp-server`; the handful of messages used are the hand-written structs below.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Instant;

use lsp_server::{Connection, ErrorCode, Message, Notification, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;
use spirolo_rules::Finding;

use crate::{EXIT_ERROR, EXIT_OK};

/// Serve one client over stdio until `exit`.
pub fn serve(all: bool) -> u8 {
    let (connection, io_threads) = Connection::stdio();
    let capabilities = json!({
        "textDocumentSync": {"openClose": true, "change": 0, "save": {"includeText": false}},
        "hoverProvider": true,
    });
    let params = match connection.initialize(capabilities) {
        Ok(params) => params,
        Err(error) => {
            eprintln!("spirolo lsp: {error}");
            return EXIT_ERROR;
        }
    };
    let root = match root_of(params) {
        Ok(root) => root,
        Err(message) => {
            eprintln!("spirolo lsp: {message}");
            return EXIT_ERROR;
        }
    };

    // `lsp-server` hands over one message at a time; a pump queues them while a pass runs, so
    // every trigger that arrived during it is waiting when it ends.
    let (queue, incoming) = mpsc::channel();
    let receiver = connection.receiver.clone();
    std::thread::spawn(move || {
        for message in receiver {
            if queue.send(message).is_err() {
                break;
            }
        }
    });

    let mut server = Server::new(root, all);
    let mut trigger = true;
    loop {
        if trigger {
            server.pass(&connection);
            // Triggers that arrived during the pass fold into exactly one more.
            trigger = false;
            for message in incoming.try_iter() {
                match server.handle(&connection, message) {
                    Handled::Trigger => trigger = true,
                    Handled::Exit => return finish(connection, io_threads),
                    Handled::Done => {}
                }
            }
            continue;
        }
        let Ok(message) = incoming.recv() else { break };
        match server.handle(&connection, message) {
            Handled::Trigger => trigger = true,
            Handled::Exit => break,
            Handled::Done => {}
        }
    }
    finish(connection, io_threads)
}

/// Close the connection and wait for the transport threads.
fn finish(connection: Connection, io_threads: lsp_server::IoThreads) -> u8 {
    drop(connection);
    match io_threads.join() {
        Ok(()) => EXIT_OK,
        Err(error) => {
            eprintln!("spirolo lsp: {error}");
            EXIT_ERROR
        }
    }
}

/// The project root from `initialize`: `rootUri`, else the first workspace folder, else the
/// current directory.
fn root_of(params: serde_json::Value) -> Result<PathBuf, String> {
    let params: InitializeParams = serde_json::from_value(params)
        .map_err(|error| format!("bad initialize params: {error}"))?;
    let uri = params
        .root_uri
        .or_else(|| params.workspace_folders?.into_iter().next().map(|f| f.uri));
    uri.map_or_else(
        || {
            std::env::current_dir()
                .map_err(|error| format!("cannot read the current directory: {error}"))
        },
        |uri| path_of(&uri).ok_or_else(|| format!("not a file URI: {uri}")),
    )
}

/// What one incoming message asks of the loop.
enum Handled {
    /// Run a pass (or one more, if one is running).
    Trigger,
    /// Stop serving.
    Exit,
    /// Nothing further.
    Done,
}

/// What the server remembers between passes.
struct Server {
    root: PathBuf,
    /// Publish every rule's findings (`spirolo lsp --all`), not only those of rules shown by default.
    all: bool,
    /// Passes run so far; the next `spirolo/checked` carries this plus one.
    generation: u64,
    /// The files the last pass published diagnostics for, so a file that lost them all is
    /// published empty.
    published: Vec<String>,
    /// The last pass's published findings by repo-relative file, what hover reads.
    hoverable: BTreeMap<String, Vec<Published>>,
    /// The last good pass: what `spirolo/tree` and `spirolo/file` read. `None` before one.
    checked: Option<crate::Checked>,
}

/// What hover shows of one published finding.
struct Published {
    /// The diagnostic's zero-based line.
    line: u32,
    id: String,
    rule: &'static str,
    message: String,
    explain: String,
}

impl Server {
    const fn new(root: PathBuf, all: bool) -> Self {
        Self {
            root,
            all,
            generation: 0,
            published: Vec::new(),
            hoverable: BTreeMap::new(),
            checked: None,
        }
    }

    /// Answer one message: saves and watched-file changes trigger a pass; opens and edits do
    /// not, since the pass reads the disk.
    fn handle(&self, connection: &Connection, message: Message) -> Handled {
        match message {
            Message::Notification(notification) => match notification.method.as_str() {
                "textDocument/didSave" | "workspace/didChangeWatchedFiles" => Handled::Trigger,
                "exit" => Handled::Exit,
                _ => Handled::Done,
            },
            Message::Request(request) => {
                let response = match request.method.as_str() {
                    "textDocument/hover" => match serde_json::from_value(request.params) {
                        Ok(params) => Response::new_ok(request.id, self.hover(&params)),
                        Err(error) => Response::new_err(
                            request.id,
                            ErrorCode::InvalidParams as i32,
                            error.to_string(),
                        ),
                    },
                    "spirolo/tree" => Response::new_ok(request.id, self.tree()),
                    "spirolo/rules" => match self.rules(request.params) {
                        Ok(rules) => Response::new_ok(request.id, rules),
                        Err(error) => {
                            Response::new_err(request.id, ErrorCode::InvalidParams as i32, error)
                        }
                    },
                    "spirolo/health" => Response::new_ok(request.id, self.health()),
                    "spirolo/file" => match serde_json::from_value(request.params) {
                        Ok(params) => Response::new_ok(request.id, self.file(&params)),
                        Err(error) => Response::new_err(
                            request.id,
                            ErrorCode::InvalidParams as i32,
                            error.to_string(),
                        ),
                    },
                    "shutdown" => Response::new_ok(request.id, ()),
                    method => Response::new_err(
                        request.id,
                        ErrorCode::MethodNotFound as i32,
                        format!("spirolo lsp does not answer {method}"),
                    ),
                };
                let _ = connection.sender.send(response.into());
                Handled::Done
            }
            Message::Response(_) => Handled::Done,
        }
    }

    /// Every published finding on the hovered line of the hovered file, in id order, as
    /// Markdown; `None` when there is none.
    fn hover(&self, params: &HoverParams) -> Option<Hover> {
        let path = path_of(&params.text_document.uri)?;
        let file = path.strip_prefix(&self.root).ok()?.to_str()?.to_owned();
        let mut here: Vec<&Published> = self
            .hoverable
            .get(&file)?
            .iter()
            .filter(|published| published.line == params.position.line)
            .collect();
        if here.is_empty() {
            return None;
        }
        here.sort_by(|a, b| a.id.cmp(&b.id));
        let value = here
            .iter()
            .map(|published| {
                format!(
                    "**{}** · {}\n\n{}\n\n{}",
                    published.rule,
                    spirolo_rules::status::status_line(published.rule),
                    published.message,
                    published.explain
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");
        Some(Hover {
            contents: MarkupContent {
                kind: "markdown",
                value,
            },
        })
    }

    /// `spirolo/tree` over the last good pass; `None` before one.
    fn tree(&self) -> Option<spirolo_report::SidebarTree> {
        let checked = self.checked.as_ref()?;
        Some(spirolo_report::sidebar_tree(
            &checked.model,
            &checked.findings,
            checked.baseline,
        ))
    }

    /// `spirolo/rules` for the scope its params name, over the last good pass; `None` before
    /// one. Params that name no family or rule are an error.
    fn rules(
        &self,
        params: serde_json::Value,
    ) -> Result<Option<spirolo_report::SidebarRules>, String> {
        let params: Option<RulesParams> =
            serde_json::from_value(params).map_err(|error| error.to_string())?;
        let params = params.unwrap_or_default();
        let scope =
            spirolo_report::RulesScope::parse(params.family.as_deref(), params.rule.as_deref())?;
        Ok(self
            .checked
            .as_ref()
            .map(|checked| spirolo_report::sidebar_rules(&checked.model, &checked.findings, scope)))
    }

    /// `spirolo/health`: the provisional score over the last good pass; `None` before one.
    fn health(&self) -> Option<spirolo_report::score::Health> {
        let checked = self.checked.as_ref()?;
        // The score is the project's, as `check` prints it, whichever rows are published.
        let findings: Vec<_> = checked
            .findings
            .iter()
            .chain(&checked.hidden)
            .cloned()
            .collect();
        Some(spirolo_report::score::health(
            &findings,
            &checked.model,
            spirolo_rules::status::table(),
        ))
    }

    /// `spirolo/file` for a file under the root, over the last good pass; `None` before one or
    /// for a URI outside the root.
    fn file(&self, params: &FileParams) -> Option<spirolo_report::SidebarFile> {
        let checked = self.checked.as_ref()?;
        let path = path_of(&params.uri)?;
        let file = path.strip_prefix(&self.root).ok()?.to_str()?;
        Some(spirolo_report::sidebar_file(
            &checked.model,
            &checked.findings,
            file,
        ))
    }

    /// Run the whole `check` pass, publish its diagnostics, then `spirolo/checked`.
    fn pass(&mut self, connection: &Connection) {
        let started = Instant::now();
        self.generation += 1;
        let checked = match crate::checked(&self.root, self.all) {
            Ok(checked) => checked,
            Err(message) => {
                // The last good pass stays published; the client hears why this one is not.
                notify(
                    connection,
                    "window/showMessage",
                    &ShowMessageParams {
                        typ: 1,
                        message: format!("spirolo: {message}"),
                    },
                );
                let checked = Checked {
                    generation: self.generation,
                    files: 0,
                    findings: -1,
                    elapsed_ms: elapsed_ms(started),
                };
                notify(connection, "spirolo/checked", &checked);
                return;
            }
        };

        let mut lines = Lines::default();
        let mut by_file: BTreeMap<String, Vec<Diagnostic>> = BTreeMap::new();
        let mut count = 0_i64;
        let mut hoverable: BTreeMap<String, Vec<Published>> = BTreeMap::new();
        for finding in &checked.findings {
            // A project-level finding (`file: ""`) belongs to no file: `spirolo/tree` carries it.
            if finding.location.file.is_empty() {
                continue;
            }
            let Some(diagnostic) = self.diagnostic(finding, &mut lines) else {
                continue;
            };
            count += 1;
            hoverable
                .entry(finding.location.file.clone())
                .or_default()
                .push(Published {
                    line: diagnostic.range.start.line,
                    id: diagnostic.data.id.clone(),
                    rule: finding.rule,
                    message: finding.message.clone(),
                    explain: finding.explain.clone(),
                });
            by_file
                .entry(finding.location.file.clone())
                .or_default()
                .push(diagnostic);
        }
        for file in &self.published {
            if !by_file.contains_key(file) {
                by_file.insert(file.clone(), Vec::new());
            }
        }
        for (file, diagnostics) in &by_file {
            let params = PublishDiagnosticsParams {
                uri: uri_of(&self.root, file),
                diagnostics,
            };
            notify(connection, "textDocument/publishDiagnostics", &params);
        }
        self.published = by_file
            .iter()
            .filter(|(_, diagnostics)| !diagnostics.is_empty())
            .map(|(file, _)| file.clone())
            .collect();
        self.hoverable = hoverable;
        self.checked = Some(checked);
        let files = self.published.len();
        let checked = Checked {
            generation: self.generation,
            files,
            findings: count,
            elapsed_ms: elapsed_ms(started),
        };
        notify(connection, "spirolo/checked", &checked);
    }

    /// The diagnostic for `finding`, or `None` for a `listed` row.
    fn diagnostic(&self, finding: &Finding, lines: &mut Lines) -> Option<Diagnostic> {
        let severity = spirolo_report::editor_severity(finding)?;
        let related = finding
            .related
            .iter()
            .map(|related| RelatedInformation {
                location: LspLocation {
                    uri: uri_of(&self.root, &related.location.file),
                    range: lines.range(&self.root, &related.location),
                },
                message: related.label.clone(),
            })
            .collect();
        Some(Diagnostic {
            range: lines.range(&self.root, &finding.location),
            severity: severity.code(),
            source: "spirolo",
            code: finding.rule,
            message: finding.message.clone(),
            data: DiagnosticData {
                id: finding.id(),
                status: spirolo_rules::status::status_line(finding.rule),
            },
            related_information: related,
        })
    }
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// Send a notification; a closed channel means the client is gone, and the loop ends on its
/// own when the reader stops.
fn notify(connection: &Connection, method: &str, params: &impl Serialize) {
    let _ = connection
        .sender
        .send(Notification::new(method.to_owned(), params).into());
}

/// File contents read once per pass, to turn a finding's byte column into a UTF-16 range.
#[derive(Default)]
struct Lines {
    files: BTreeMap<String, Option<String>>,
}

impl Lines {
    /// The range of a location: from its column to the end of its line. A file-level
    /// location (line 0) starts at the file's first line.
    fn range(&mut self, root: &Path, location: &spirolo_rules::Location) -> Range {
        let line = location.line.saturating_sub(1);
        let text = self
            .files
            .entry(location.file.clone())
            .or_insert_with(|| std::fs::read_to_string(root.join(&location.file)).ok());
        let row = text
            .as_deref()
            .and_then(|text| text.split('\n').nth(line as usize))
            .map(|row| row.strip_suffix('\r').unwrap_or(row));
        let column = location.column.saturating_sub(1) as usize;
        let (start, end) = row.map_or_else(
            || {
                let start = u32::try_from(column).unwrap_or(u32::MAX);
                (start, start)
            },
            |row| {
                let mut at = column.min(row.len());
                while !row.is_char_boundary(at) {
                    at -= 1;
                }
                (utf16_len(&row[..at]), utf16_len(row))
            },
        );
        Range {
            start: Position {
                line,
                character: start,
            },
            end: Position {
                line,
                character: end,
            },
        }
    }
}

fn utf16_len(text: &str) -> u32 {
    u32::try_from(text.encode_utf16().count()).unwrap_or(u32::MAX)
}

/// `file://` + the root joined with a repo-relative path, as the PRD writes it: the path is
/// not percent-encoded (an editor normalises either form).
fn uri_of(root: &Path, file: &str) -> String {
    if file.is_empty() {
        format!("file://{}", root.display())
    } else {
        format!("file://{}", root.join(file).display())
    }
}

/// The path a `file://` URI names, percent-decoded.
fn path_of(uri: &str) -> Option<PathBuf> {
    let encoded = uri.strip_prefix("file://")?.as_bytes();
    let mut bytes = Vec::with_capacity(encoded.len());
    let mut i = 0;
    while i < encoded.len() {
        if encoded[i] == b'%' {
            let hex = std::str::from_utf8(encoded.get(i + 1..i + 3)?).ok()?;
            bytes.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            bytes.push(encoded[i]);
            i += 1;
        }
    }
    let path = PathBuf::from(String::from_utf8(bytes).ok()?);
    Some(path.components().collect())
}

// ------------------------------------------------------------------------ protocol structs

/// The part of `initialize`'s params the server reads.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InitializeParams {
    root_uri: Option<String>,
    workspace_folders: Option<Vec<WorkspaceFolder>>,
}

#[derive(Deserialize)]
struct WorkspaceFolder {
    uri: String,
}

#[derive(Serialize)]
struct PublishDiagnosticsParams<'a> {
    uri: String,
    diagnostics: &'a [Diagnostic],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Diagnostic {
    range: Range,
    severity: u8,
    source: &'static str,
    code: &'static str,
    message: String,
    data: DiagnosticData,
    related_information: Vec<RelatedInformation>,
}

#[derive(Serialize)]
struct DiagnosticData {
    id: String,
    status: String,
}

#[derive(Serialize)]
struct RelatedInformation {
    location: LspLocation,
    message: String,
}

#[derive(Serialize)]
struct LspLocation {
    uri: String,
    range: Range,
}

#[derive(Serialize, Clone, Copy)]
struct Range {
    start: Position,
    end: Position,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
struct Position {
    line: u32,
    character: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HoverParams {
    text_document: TextDocument,
    position: Position,
}

#[derive(Deserialize)]
struct TextDocument {
    uri: String,
}

/// `spirolo/rules`' params: none for the summary, or the family or rule to fill files for.
#[derive(Deserialize, Default)]
struct RulesParams {
    family: Option<String>,
    rule: Option<String>,
}

/// `spirolo/file`'s params.
#[derive(Deserialize)]
struct FileParams {
    uri: String,
}

#[derive(Serialize)]
struct Hover {
    contents: MarkupContent,
}

#[derive(Serialize)]
struct MarkupContent {
    kind: &'static str,
    value: String,
}

/// `window/showMessage`; `type` 1 is an error.
#[derive(Serialize)]
struct ShowMessageParams {
    #[serde(rename = "type")]
    typ: u8,
    message: String,
}

/// `spirolo/checked`: a pass is published.
#[derive(Serialize)]
struct Checked {
    generation: u64,
    files: usize,
    findings: i64,
    elapsed_ms: u64,
}
