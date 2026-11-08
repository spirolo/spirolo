//! The two text views over the model: one for a person, one for an agent.
//!
//! Both are views over the same rows the JSON carries. The only thing the human view
//! knows that the JSON does not is how long the run took.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::time::Duration;

use serde::Serialize;
use spirolo_model::{Diagnostic, Model, Severity, TestKind};

/// The one line `spirolo list` opens with: what the model saw, and how long it took.
///
/// The timing lives here and nowhere else — putting it in the JSON would make two runs
/// over the same tree differ.
pub fn summary_line(model: &Model, elapsed: Duration) -> String {
    let tests = model
        .test_cases
        .iter()
        .filter(|case| case.kind == TestKind::Test)
        .count();
    format!(
        "{} files · {} test files · {tests} tests{} · {} expects · {} handlers · {} configs · {}",
        model.modules.len(),
        model.test_files.len(),
        framework_breakdown(model),
        model.assertion_count(),
        model.handlers.len(),
        model.gates.len(),
        format_elapsed(elapsed),
    )
}

/// The per-framework test counts, as ` (97 vitest, 12 playwright)`, so a suite that
/// mixes runners reads each apart. Empty when there are no tests, so an empty project's
/// summary stays `0 tests`. Frameworks are listed in name order for a stable line.
fn framework_breakdown(model: &Model) -> String {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for case in &model.test_cases {
        if case.kind == TestKind::Test {
            *counts
                .entry(case.framework.as_deref().unwrap_or("unknown"))
                .or_default() += 1;
        }
    }
    if counts.is_empty() {
        return String::new();
    }
    let parts: Vec<String> = counts
        .iter()
        .map(|(framework, count)| format!("{count} {framework}"))
        .collect();
    format!(" ({})", parts.join(", "))
}

/// The default view: the summary line, then one section per non-empty table, then the
/// diagnostics. Always ends with a newline.
pub fn render_human(model: &Model, elapsed: Duration) -> String {
    let mut out = summary_line(model, elapsed);
    out.push('\n');

    for (name, rows) in sections(model) {
        if rows.is_empty() {
            continue;
        }
        let _ = write!(out, "\n{name}\n");
        for row in rows {
            let _ = writeln!(out, "  {}", row.join("  "));
        }
    }

    let diagnostics = render_diagnostics(model, "  ");
    if !diagnostics.is_empty() {
        let _ = write!(out, "\ndiagnostics\n{diagnostics}");
    }

    out
}

/// The `--agent` view: tables and diagnostics and nothing else.
///
/// Tab-separated, one row per line, first field naming the table. No summary, no
/// headings, no alignment, nothing to parse around. An empty model renders as the
/// empty string.
pub fn render_agent(model: &Model) -> String {
    let mut out = String::new();

    for (name, rows) in sections(model) {
        for row in rows {
            let _ = writeln!(out, "{name}\t{}", row.join("\t"));
        }
    }

    for diagnostic in &model.diagnostics {
        let _ = writeln!(out, "diagnostic\t{}", agent_diagnostic(diagnostic));
    }

    out
}

/// Every table as (name, rows of already-stringified cells), in a fixed order.
fn sections(model: &Model) -> Vec<(&'static str, Vec<Vec<String>>)> {
    vec![
        (
            "modules",
            model
                .modules
                .iter()
                .map(|module| {
                    vec![
                        module.path.clone(),
                        token(&module.language),
                        format!("{} imports", module.imports.len()),
                        format!("{} exports", module.exports.len()),
                    ]
                })
                .collect(),
        ),
        (
            "test_cases",
            model
                .test_cases
                .iter()
                .map(|case| {
                    vec![
                        format!("{}:{}", case.file, case.span.line),
                        token(&case.kind),
                        case.title.clone().unwrap_or_else(|| "<dynamic>".to_owned()),
                        format!("{} expects", case.assertions.len()),
                    ]
                })
                .collect(),
        ),
        (
            "pairings",
            model
                .pairings
                .iter()
                .map(|pairing| {
                    vec![
                        pairing.test_file.clone(),
                        pairing.source_module.clone(),
                        token(&pairing.reason),
                    ]
                })
                .collect(),
        ),
        (
            "handlers",
            model
                .handlers
                .iter()
                .map(|handler| {
                    vec![
                        format!("{}:{}", handler.file, handler.span.line),
                        handler.method.clone().unwrap_or_else(|| "ANY".to_owned()),
                        handler
                            .pattern
                            .clone()
                            .unwrap_or_else(|| "<opaque>".to_owned()),
                    ]
                })
                .collect(),
        ),
        (
            "gates",
            model
                .gates
                .iter()
                .map(|gate| vec![gate.source.clone(), token(&gate.tool)])
                .collect(),
        ),
    ]
}

fn render_diagnostics(model: &Model, indent: &str) -> String {
    let mut out = String::new();
    for diagnostic in &model.diagnostics {
        let where_ = diagnostic.file.as_deref().map_or_else(String::new, |file| {
            diagnostic.span.map_or_else(
                || format!("{file} "),
                |span| format!("{file}:{} ", span.line),
            )
        });
        let _ = writeln!(
            out,
            "{indent}{} {where_}{} [{}]",
            token(&diagnostic.severity),
            diagnostic.message,
            diagnostic.code
        );
    }
    out
}

fn agent_diagnostic(diagnostic: &Diagnostic) -> String {
    format!(
        "{}\t{}\t{}\t{}",
        token(&diagnostic.severity),
        diagnostic.code,
        diagnostic.file.as_deref().unwrap_or("-"),
        diagnostic.message,
    )
}

/// The one spelling of an enum value, taken from the JSON schema so the text views and
/// the JSON never teach an agent two different vocabularies for the same thing.
fn token<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| "unknown".to_owned())
}

/// Milliseconds under a second, one decimal of seconds above it.
fn format_elapsed(elapsed: Duration) -> String {
    if elapsed.as_secs() == 0 {
        format!("{}ms", elapsed.as_millis())
    } else {
        format!("{:.1}s", elapsed.as_secs_f64())
    }
}

/// How many `tsconfig_unloadable` lines the resolution warning lists before it counts the rest.
const UNLOADABLE_SHOWN: usize = 3;

/// The warning a check-family human report prints under its health line when the run's
/// module resolution could not load for the project (a `module_resolution_failed` warning):
/// a banner, the configs that failed, then the cause and the fix one clause a line. Nothing
/// when resolution loaded, or when only nested configs failed ([`is_resolution_note`]).
pub fn write_resolution_warning(out: &mut String, model: &Model) {
    let Some(failure) = model
        .diagnostics
        .iter()
        .find(|d| d.code == "module_resolution_failed" && d.severity == Severity::Warning)
    else {
        return;
    };
    let _ = writeln!(
        out,
        "! module resolution failed: the rows that depend on it are marked unreliable"
    );
    let unloadable: Vec<&Diagnostic> = model
        .diagnostics
        .iter()
        .filter(|d| d.code == "tsconfig_unloadable")
        .collect();
    for diagnostic in unloadable.iter().take(UNLOADABLE_SHOWN) {
        let _ = writeln!(out, "  {}", diagnostic.message);
    }
    if unloadable.len() > UNLOADABLE_SHOWN {
        let _ = writeln!(
            out,
            "  and {} more tsconfigs (spirolo list shows them)",
            unloadable.len() - UNLOADABLE_SHOWN
        );
    }
    // The message's clauses (what was left unresolved, the cause, the fix, what is marked)
    // are `; `-separated; one line each reads louder than one long line.
    for clause in failure.message.split("; ") {
        let _ = writeln!(out, "  {clause}");
    }
}

/// Whether `diagnostic` is a module-resolution failure confined to nested configs of an
/// installed project: an `info`, printed as one closing ` note` line rather than a warning.
pub fn is_resolution_note(diagnostic: &Diagnostic) -> bool {
    diagnostic.code == "module_resolution_failed" && diagnostic.severity == Severity::Info
}
