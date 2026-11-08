//! The three views of `spirolo impact`: a ranked list for a human, the JSON document for an
//! agent, and the bare paths for a runner's argument list; and the two views of `spirolo
//! impact --hotspots`.

use std::fmt::Write as _;

use spirolo_rules::{Hotspots, Impact};

/// One line per selected test file (rank, path, evidence), then the changed files no test
/// reaches, then what could not be used from the input.
pub fn render_impact_human(impact: &Impact) -> String {
    let mut out = format!(
        "{} test file{} selected for {} changed file{}\n",
        impact.selected.len(),
        plural(impact.selected.len()),
        impact.changed.len(),
        plural(impact.changed.len()),
    );
    let width = impact
        .selected
        .iter()
        .map(|selected| selected.file.len())
        .max()
        .unwrap_or(0);
    let rank_width = impact.selected.len().to_string().len();
    for selected in &impact.selected {
        let mut evidence = Vec::new();
        if selected.distance == Some(0) {
            evidence.push("changed".to_owned());
        }
        if selected.paired {
            evidence.push("paired".to_owned());
        }
        if selected.last_failed {
            evidence.push("last-failed".to_owned());
        }
        if let Some(distance) = selected.distance.filter(|distance| *distance > 0) {
            evidence.push(format!("imports {distance}"));
        }
        if let Some(median) = selected.median_ms {
            evidence.push(format!("median {median} ms"));
        }
        let _ = writeln!(
            out,
            "{:>rank_width$}  {:<width$}  {}",
            selected.rank,
            selected.file,
            evidence.join(", "),
        );
    }
    if !impact.unreached.is_empty() {
        out.push_str("unreached (no test file reaches them):\n");
        for path in &impact.unreached {
            let _ = writeln!(out, "  {path}");
        }
    }
    for diagnostic in &impact.diagnostics {
        let _ = writeln!(out, "warning: {} ({})", diagnostic.message, diagnostic.code);
    }
    out
}

const fn plural(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

/// The JSON document: `{changed, selected: [{rank, file, distance, paired, last_failed,
/// median_ms}], unreached,
/// diagnostics}`, `selected` in rank order and keys sorted at every depth.
///
/// # Panics
///
/// Never in practice: the document is built from owned strings and numbers.
pub fn impact_to_json(impact: &Impact) -> String {
    let selected: Vec<serde_json::Value> = impact
        .selected
        .iter()
        .map(|selected| {
            serde_json::json!({
                "rank": selected.rank,
                "file": selected.file,
                "distance": selected.distance,
                "paired": selected.paired,
                "last_failed": selected.last_failed,
                "median_ms": selected.median_ms,
            })
        })
        .collect();
    let mut document = serde_json::json!({
        "changed": impact.changed,
        "selected": selected,
        "unreached": impact.unreached,
        "diagnostics": impact.diagnostics,
    });
    crate::canonicalise(&mut document);
    let mut json = serde_json::to_string_pretty(&document).expect("a JSON value serializes");
    json.push('\n');
    json
}

/// The selected test files, space-separated on one line, for `vitest run $(…)`; nothing at
/// all when nothing is selected.
pub fn render_impact_print(impact: &Impact) -> String {
    if impact.selected.is_empty() {
        return String::new();
    }
    let files: Vec<&str> = impact.selected.iter().map(|s| s.file.as_str()).collect();
    format!("{}\n", files.join(" "))
}

/// `impact --hotspots` for a human: one line per ranked module (rank, path, score, and the
/// churn × complexity × weakness it multiplies), then what the ranking had to guess.
pub fn render_hotspots_human(hotspots: &Hotspots) -> String {
    let count = hotspots.hotspots.len();
    let mut out = format!(
        "{count} hotspot{} (score = churn × complexity × weakness)\n",
        plural(count)
    );
    let width = hotspots
        .hotspots
        .iter()
        .map(|hotspot| hotspot.file.len())
        .max()
        .unwrap_or(0);
    let rank_width = count.to_string().len();
    for (index, hotspot) in hotspots.hotspots.iter().enumerate() {
        let _ = writeln!(
            out,
            "{:>rank_width$}  {:<width$}  {:.2} = {} × {} × {:.2}",
            index + 1,
            hotspot.file,
            hotspot.score,
            hotspot.churn,
            hotspot.complexity,
            hotspot.weakness,
        );
    }
    for diagnostic in &hotspots.diagnostics {
        let _ = writeln!(out, "note: {} ({})", diagnostic.message, diagnostic.code);
    }
    out
}

/// `impact --hotspots --format json`, in rank order.
///
/// `{hotspots: [{rank, file, churn, complexity, weakness, score}], diagnostics}`, with
/// `weakness` and `score` rounded to two decimals as the finding's evidence prints them and
/// keys sorted at every depth.
///
/// # Panics
///
/// Never in practice: the document is built from owned strings and finite numbers.
pub fn hotspots_to_json(hotspots: &Hotspots) -> String {
    let rows: Vec<serde_json::Value> = hotspots
        .hotspots
        .iter()
        .enumerate()
        .map(|(index, hotspot)| {
            serde_json::json!({
                "rank": index + 1,
                "file": hotspot.file,
                "churn": hotspot.churn,
                "complexity": hotspot.complexity,
                "weakness": hundredths(hotspot.weakness),
                "score": hundredths(hotspot.score),
            })
        })
        .collect();
    let mut document = serde_json::json!({
        "hotspots": rows,
        "diagnostics": hotspots.diagnostics,
    });
    crate::canonicalise(&mut document);
    let mut json = serde_json::to_string_pretty(&document).expect("a JSON value serializes");
    json.push('\n');
    json
}

/// `value` rounded to two decimals.
fn hundredths(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
