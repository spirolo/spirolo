//! Rendering findings: the human `check` view with `ok` rows, the agent view with just
//! findings and their explain, and the `findings` key the JSON grows.
//!
//! These views are over the findings a rule produced, the way the model views are over the
//! model's tables. The human view groups rules under one banner per family, tier first inside it,
//! with capped file groups and one line for rules that found nothing, so a clean run reads as evidence; the
//! agent view drops every `ok` row and every decoration and prints each finding's explain,
//! so an agent spends tokens only on what needs fixing.

use std::fmt::Write as _;
use std::time::Duration;

use serde_json::json;
use spirolo_model::Model;
use spirolo_rules::{status, Family, Finding, Outcome, Rule, Severity, TestPath};

use crate::baseline::Diff;
use crate::human::summary_line;

/// A tally of a run's findings for the closing line and the exit code.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tally {
    /// Rules that ran.
    pub rules: usize,
    /// Rules that found nothing: no row at all, decided or not.
    pub ok: usize,
    /// Rules whose rows are all `could not decide` or listed: neither clean nor a finding.
    pub undecided: usize,
    /// Findings of `warn` severity.
    pub warn: usize,
    /// Findings of `error` severity.
    pub error: usize,
    /// Findings of `info` severity.
    pub info: usize,
    /// `could not decide` outcomes, counted apart from findings.
    pub could_not_decide: usize,
    /// Listing rows (a fact reported for the count, no claim), counted apart from findings.
    pub listed: usize,
}

impl Tally {
    /// The exit code before a baseline exists: `1` when any warn- or error-severity
    /// finding is present, `0` otherwise. `check` never returns `2` for findings — that
    /// code is reserved for a run that could not be done.
    pub const fn exit_code(&self) -> u8 {
        if self.error > 0 || self.warn > 0 {
            1
        } else {
            0
        }
    }
}

/// Count a run's findings against the rules that ran, so the closing line and the exit
/// code agree on one tally.
pub fn tally(ran: &[Box<dyn Rule>], findings: &[Finding]) -> Tally {
    let mut tally = count(findings.iter());
    tally.rules = ran.len();
    for rule in ran {
        let mut rows = findings.iter().filter(|f| f.rule == rule.name()).peekable();
        if rows.peek().is_none() {
            tally.ok += 1;
        } else if rows.all(|f| f.outcome != Outcome::Finding) {
            tally.undecided += 1;
        }
    }
    tally
}

/// Count rows by severity and outcome: decided findings under their severity, the other
/// outcomes apart. `rules` and `ok` stay zero; [`tally`] fills them for a whole run.
fn count<'a>(findings: impl Iterator<Item = &'a Finding>) -> Tally {
    let mut tally = Tally::default();
    for finding in findings {
        match (finding.outcome, finding.severity) {
            (Outcome::CouldNotDecide, _) => tally.could_not_decide += 1,
            (Outcome::Listed, _) => tally.listed += 1,
            (Outcome::Finding, Severity::Error) => tally.error += 1,
            (Outcome::Finding, Severity::Warn) => tally.warn += 1,
            (Outcome::Finding, Severity::Info) => tally.info += 1,
        }
    }
    tally
}

/// The human `check` view.
///
/// The summary line and the health line, then one section per rule under its family's
/// banner, in family order and by tier, severity and registry order inside a family.
/// Each section shows at most ten files and five rows per file. Empty rules collapse
/// into one line in registry order, followed by the closing tally. Always
/// ends with a newline.
pub fn render_check_human(
    model: &Model,
    elapsed: Duration,
    ran: &[Box<dyn Rule>],
    findings: &[Finding],
) -> String {
    render_check_human_with_options(model, elapsed, ran, findings, HumanOptions::default())
}

/// Display options for the human check-family reports. Machine formats ignore these.
#[derive(Debug, Clone, Copy, Default)]
pub struct HumanOptions<'a> {
    /// Show every file and row instead of capping each section at ten files and five rows.
    pub all: bool,
    /// The family subcommand that ran (`spirolo audit`…); `None` for `spirolo check`.
    pub family: Option<Family>,
    /// Rows of rules hidden by default, left out of the default view: counted by rule
    /// under the closing line, never listed, and never read as a rule that found nothing.
    pub hidden: &'a [Finding],
    /// The `--rule` the run narrowed to, named again in the command the ` hidden` line gives.
    pub rule: Option<&'a str>,
    /// Rules whose config severity asks them to gate but that never gate (DROPPED or FAIL):
    /// one ` note` line each says the override was not applied.
    pub not_applied: &'a [&'a str],
}

/// The human check view with explicit display options.
pub fn render_check_human_with_options(
    model: &Model,
    elapsed: Duration,
    ran: &[Box<dyn Rule>],
    findings: &[Finding],
    options: HumanOptions,
) -> String {
    let mut out = summary_line(model, elapsed);
    out.push('\n');
    write_health(&mut out, model, ran, findings, options);
    crate::human::write_resolution_warning(&mut out, model);

    write_ordered_findings(&mut out, ran, findings, options);

    let tally = tally(ran, findings);
    let _ = write!(
        out,
        "\n{} findings in {} of {} rules{} · {} warn · {} error{}{} · exit {}\n",
        tally.warn + tally.error + tally.info,
        tally.rules - tally.ok - tally.undecided,
        tally.rules,
        undecided_rules(&tally),
        tally.warn,
        tally.error,
        info_count(&tally),
        undecided_count(&tally),
        tally.exit_code(),
    );
    write_hidden(&mut out, ran, options);
    write_run_notes(&mut out, model, options);
    out
}

/// The health line under the header (ADR 0003, amended 2026-10-05: no provisional label), from [`crate::score::health`]
/// over the findings shown: the whole score under `check`, only the family's own part under
/// a family subcommand. A `--rule` run has no whole to score and prints neither.
fn write_health(
    out: &mut String,
    model: &Model,
    ran: &[Box<dyn Rule>],
    findings: &[Finding],
    options: HumanOptions,
) {
    let registry = spirolo_rules::registry();
    let scope = registry
        .iter()
        .filter(|rule| options.family.is_none_or(|family| rule.family() == family))
        .count();
    if ran.len() != scope {
        return;
    }
    // The score is the project's, whichever rows the view lists.
    let all: Vec<Finding>;
    let findings = if options.hidden.is_empty() {
        findings
    } else {
        all = findings.iter().chain(options.hidden).cloned().collect();
        &all
    };
    let health = crate::score::health(findings, model, status::table());
    if let Some(family) = options.family {
        if let Some(part) = health
            .parts
            .iter()
            .find(|part| part.family == family.name())
        {
            let _ = writeln!(out, "{} −{:.1} / {}", part.title, part.penalty, part.cap);
        }
        return;
    }
    let _ = writeln!(out, "Health {} ({})", health.grade, health.score.round());
}

/// One ` hidden` line naming how many rows the default view left out: the experimental
/// rules' rows apart from the DROPPED and FAIL rules' rows, each by rule in run order, then
/// the `--all` command that shows them.
fn write_hidden(out: &mut String, ran: &[Box<dyn Rule>], options: HumanOptions) {
    if options.hidden.is_empty() {
        return;
    }
    let group = |hidden: bool| {
        let mut total = 0;
        let by_rule: Vec<String> = ran
            .iter()
            .filter(|rule| (status::tier(rule.name()) == status::Tier::Hidden) == hidden)
            .filter_map(|rule| {
                let count = options
                    .hidden
                    .iter()
                    .filter(|finding| finding.rule == rule.name())
                    .count();
                total += count;
                (count > 0).then(|| format!("{} {count}", rule.name()))
            })
            .collect();
        (total, by_rule.join(", "))
    };
    let groups: Vec<String> = [
        (group(false), "experimental"),
        (group(true), "from DROPPED or FAIL rules"),
    ]
    .into_iter()
    .filter(|((total, _), _)| *total > 0)
    .map(|((total, by_rule), what)| format!("{total} {what} ({by_rule})"))
    .collect();
    let _ = writeln!(
        out,
        " hidden {} findings: {}; spirolo {}{} --all shows them",
        options.hidden.len(),
        groups.join(", "),
        options.family.map_or("check", Family::name),
        options
            .rule
            .map_or(String::new(), |rule| format!(" --rule {rule}")),
    );
}

/// The diagnostics that change how a reader should take the findings above, one ` note`
/// line each after the closing line. Only these few: the model's other diagnostics are
/// parse and config facts `list` already shows, and a run of thousands would bury the tally.
const RUN_NOTES: [&str; 1] = ["untested_hotspot_no_churn"];

/// The [`RUN_NOTES`] lines and a module-resolution failure confined to nested configs
/// ([`crate::human::is_resolution_note`]), then one ` note` line per config severity override the run did
/// not apply because the rule is DROPPED or FAIL (#420).
fn write_run_notes(out: &mut String, model: &Model, options: HumanOptions) {
    for diagnostic in &model.diagnostics {
        if RUN_NOTES.contains(&diagnostic.code.as_str())
            || crate::human::is_resolution_note(diagnostic)
        {
            let _ = writeln!(out, " note   {}", diagnostic.message);
        }
    }
    for rule in options.not_applied {
        let _ = writeln!(
            out,
            " note   config severity for {rule} not applied: rule is {}",
            status::status(rule).verdict.name()
        );
    }
}

/// Sort references for the human surface only, preserving the caller's JSON/SARIF order.
fn write_ordered_findings(
    out: &mut String,
    ran: &[Box<dyn Rule>],
    findings: &[Finding],
    options: HumanOptions,
) {
    let mut by_rule = std::collections::BTreeMap::<_, Vec<_>>::new();
    for finding in findings {
        by_rule.entry(finding.rule).or_default().push(finding);
    }
    let mut sections: Vec<_> = ran
        .iter()
        .enumerate()
        .filter_map(|(index, rule)| by_rule.remove(rule.name()).map(|rows| (index, rule, rows)))
        .collect();
    sections.sort_by_key(|(index, rule, rows)| {
        (
            rule.family(),
            tier(rule.name()),
            rows.iter().map(|f| f.severity).min(),
            *index,
        )
    });
    let mut current_family = None;
    for (_, rule, mut rows) in sections {
        let family = rule.family();
        if current_family != Some(family) {
            let _ = writeln!(
                out,
                "\n── {} ({}) ────────────────────────────────────────",
                family.title(),
                family.name()
            );
            current_family = Some(family);
        }
        rows.sort_by_key(|f| (&f.location.file, f.location.line, f.location.column, f.id()));
        let _ = writeln!(out, "\n{}", rule_heading(rule.name(), &rows));
        write_files(out, &rows, options);
        let _ = writeln!(
            out,
            "  {} · spirolo explain {}",
            rule.summary(),
            rule.name()
        );
        write_actions(out, rule.name(), &rows);
    }
    let silent: Vec<_> = ran
        .iter()
        .filter(|rule| {
            !findings
                .iter()
                .chain(options.hidden)
                .any(|finding| finding.rule == rule.name())
        })
        .map(|rule| rule.name())
        .collect();
    if !silent.is_empty() {
        let _ = writeln!(
            out,
            "\n✓ {} rules found nothing: {}",
            silent.len(),
            silent.join(", ")
        );
    }
}

/// The shared human/sidebar tier rank, from [`status::tier`]: proven, advisory, experimental,
/// then the DROPPED and FAIL rules only `--all` shows.
pub fn tier(rule: &str) -> u8 {
    match status::tier(rule) {
        status::Tier::Proven => 0,
        status::Tier::Advisory => 1,
        status::Tier::Experimental => 2,
        status::Tier::Hidden => 3,
    }
}

/// One rule's file and row counts, its outcome breakdown when mixed, and its [`tier_label`].
/// The empty path counts as one project entry alongside the files.
fn rule_heading(name: &str, mine: &[&Finding]) -> String {
    let counts = count(mine.iter().copied());
    let files = mine
        .iter()
        .map(|f| &f.location.file)
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let parts: Vec<String> = [
        (counts.warn, "warn"),
        (counts.error, "error"),
        (counts.info, "info"),
        (counts.could_not_decide, "could not decide"),
        (counts.listed, "listed"),
    ]
    .into_iter()
    .filter(|(count, _)| *count > 0)
    .map(|(count, word)| format!("{count} {word}"))
    .collect();
    let undecided_only = counts.could_not_decide + counts.listed == mine.len();
    let breakdown = if parts.len() > 1 || undecided_only {
        format!(" · {}", parts.join(" · "))
    } else {
        String::new()
    };
    let rows = if mine.len() == files {
        String::new()
    } else {
        format!(", {} findings", mine.len())
    };
    format!(
        "● {name} ({}{rows}){breakdown}{}",
        files_word(files),
        tier_label(name)
    )
}

/// How far to trust a rule, as its human heading's suffix.
///
/// Nothing for a proven rule, ` · advisory: <truths>/<decided> (<VERDICT>)` or ` · experimental: …` from its row's
/// decided findings (`not yet measured` when none was decided), and ` · rule: DROPPED` or
/// ` · rule: FAIL` for a rule only `--all` shows.
pub fn tier_label(name: &str) -> String {
    let row = status::status(name);
    let word = match row.tier() {
        status::Tier::Proven => return String::new(),
        status::Tier::Hidden => return format!(" · rule: {}", row.verdict.name()),
        status::Tier::Advisory => "advisory",
        status::Tier::Experimental => "experimental",
    };
    let decided = u64::from(row.truths) + u64::from(row.falses);
    if decided == 0 {
        format!(" · {word}: not yet measured")
    } else {
        format!(
            " · {word}: {}/{decided} ({})",
            row.truths,
            row.verdict.name()
        )
    }
}

/// `1 file`, `2 files`.
fn files_word(count: usize) -> String {
    if count == 1 {
        "1 file".to_owned()
    } else {
        format!("{count} files")
    }
}

/// The rule's status line without its `measured on` list: a header naming thirty corpus
/// repositories buries the rows under it, and `spirolo explain <rule>` still prints the list.
pub fn header_status(name: &str) -> String {
    let mut status = status::status(name).clone();
    status.measured_on.clear();
    status.line()
}

/// How to act on a section. A rule with a warn or error row already gates, and a DROPPED or
/// FAIL rule never gates whatever its configured severity, so only the baseline is offered;
/// any other info-only rule is also offered the severity override that gates it.
fn write_actions(out: &mut String, name: &str, rows: &[&Finding]) {
    if rows.iter().any(|f| f.severity != Severity::Info)
        || status::tier(name) == status::Tier::Hidden
    {
        let _ = writeln!(out, "  To accept today's rows: spirolo ratchet");
    } else {
        let _ = writeln!(
            out,
            "  To gate it: .spirolorc.jsonc {{ \"rules\": {{ \"{name}\": {{ \"severity\": \"warn\" }} }} }} · to accept today's rows: spirolo ratchet"
        );
    }
}

/// Group findings by file, sharing the human view's score and path order with the sidebar.
pub fn ordered_files<'a>(
    block: &[&'a Finding],
    ranked: bool,
) -> Vec<(&'a String, Vec<&'a Finding>)> {
    let mut files = std::collections::BTreeMap::<_, Vec<_>>::new();
    for finding in block {
        files
            .entry(&finding.location.file)
            .or_default()
            .push(*finding);
    }
    let mut files: Vec<_> = files.into_iter().collect();
    if ranked && block.iter().any(|f| score(f).is_some()) {
        files.sort_by(|(file_a, a), (file_b, b)| {
            let best =
                |rows: &[&Finding]| rows.iter().filter_map(|f| score(f)).max_by(f64::total_cmp);
            best(b)
                .partial_cmp(&best(a))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| file_a.cmp(file_b))
        });
    }
    files
}

/// File groups, ranked by their highest numeric score when supplied; otherwise by path.
fn write_files(out: &mut String, block: &[&Finding], options: HumanOptions) {
    let file_limit = if options.all { usize::MAX } else { 10 };
    let row_limit = if options.all { usize::MAX } else { 5 };
    let files = ordered_files(block, true);
    for (file, rows) in files.iter().take(file_limit) {
        if rows.len() == 1 {
            write_finding_human(out, rows[0], false);
        } else {
            let path = if file.is_empty() {
                "project"
            } else {
                file.as_str()
            };
            let _ = writeln!(out, "  {path} ({})", rows.len());
            for finding in rows.iter().take(row_limit) {
                write_finding_human(out, finding, true);
            }
            if rows.len() > row_limit {
                let _ = writeln!(out, "    … and {} more", rows.len() - row_limit);
            }
        }
    }
    if files.len() > file_limit {
        let _ = writeln!(
            out,
            "  … and {} more {} (spirolo {} --rule {} --all)",
            files.len() - file_limit,
            if files.len() - file_limit == 1 {
                "file"
            } else {
                "files"
            },
            options.family.map_or("check", Family::name),
            block[0].rule
        );
    }
}

fn write_finding_human(out: &mut String, finding: &Finding, nested: bool) {
    let (indent, location) = if nested {
        ("    ", format!(":{}", finding.location.line))
    } else {
        ("  ", location(finding))
    };
    let _ = writeln!(
        out,
        "{indent}{location} {}{}{}",
        finding
            .message
            .strip_prefix(&format!("{} ", finding.location.file))
            .unwrap_or(&finding.message),
        duplicate_locations(finding),
        undecided_mark(finding)
    );
    if let Some(proposal) = &finding.proposal {
        let _ = writeln!(
            out,
            "{indent}  → {}",
            crate::proposal::proposal_line(proposal)
        );
    }
}

/// Name duplicate partners in the compact human view. Structured reports already
/// carry the full related locations, and their message suffix is read by the
/// adjudicator to recover the primary test's title.
fn duplicate_locations(finding: &Finding) -> String {
    const NAMED: usize = 3;
    if finding.rule != "duplicate-test" || finding.related.is_empty() {
        return String::new();
    }
    let mut named = finding
        .related
        .iter()
        .take(NAMED)
        .map(|related| {
            let file = if related.location.file == finding.location.file {
                ""
            } else {
                related.location.file.as_str()
            };
            format!("{file}:{}", related.location.line)
        })
        .collect::<Vec<_>>()
        .join(", ");
    if finding.related.len() > NAMED {
        let _ = write!(named, " and {} more", finding.related.len() - NAMED);
    }
    format!(", at {named}")
}

/// The suffix a could-not-decide or listing row carries in the human view, so it is never
/// read as a decided finding; empty for a finding.
const fn undecided_mark(finding: &Finding) -> &'static str {
    match finding.outcome {
        Outcome::Finding => "",
        Outcome::CouldNotDecide => " · could not decide",
        Outcome::Listed => " · listed",
    }
}

fn score(finding: &Finding) -> Option<f64> {
    finding
        .evidence
        .iter()
        .find(|e| e.key == "score")
        .and_then(|e| e.value.parse::<f64>().ok())
        .filter(|score| score.is_finite())
}

/// The closing line's info count, present only when there is one: advisory findings never
/// move the exit code, but a closing line that omits them reads as an empty run above
/// thousands of rows.
fn info_count(tally: &Tally) -> String {
    if tally.info > 0 {
        format!(" · {} info", tally.info)
    } else {
        String::new()
    }
}

/// The closing line's count of rules that could not decide anything, present only when
/// there is one: such a rule is neither among the rules with findings nor clean.
fn undecided_rules(tally: &Tally) -> String {
    match tally.undecided {
        0 => String::new(),
        1 => " · 1 rule could not decide".to_owned(),
        count => format!(" · {count} rules could not decide"),
    }
}

/// The closing line's could-not-decide and listed counts, each present only when there is
/// one to report.
fn undecided_count(tally: &Tally) -> String {
    let mut out = String::new();
    if tally.could_not_decide > 0 {
        let _ = write!(out, " · {} could not decide", tally.could_not_decide);
    }
    if tally.listed > 0 {
        let _ = write!(out, " · {} listed", tally.listed);
    }
    out
}

/// The human `check` view against a baseline.
///
/// Like [`render_check_human`], but the rule blocks show only the findings the baseline
/// did not suppress, the closing line carries a `· N new` count, and prune and staleness
/// warnings follow it.
///
/// `diff` holds the surfaced findings (every finding of a rule in a file whose count rose,
/// plus every ratio row above its ceiling), the prune candidates, and the unmatched count.
/// A rule with nothing surfaced still reads as `ok`, so a fully-baselined run reads as
/// evidence and not as silence.
pub fn render_check_human_baselined(
    model: &Model,
    elapsed: Duration,
    ran: &[Box<dyn Rule>],
    diff: &Diff,
    fail_on_stale: bool,
) -> String {
    render_check_human_baselined_with_options(
        model,
        elapsed,
        ran,
        diff,
        fail_on_stale,
        HumanOptions::default(),
    )
}

/// The baselined human check view with the same display options as an unbaselined run.
pub fn render_check_human_baselined_with_options(
    model: &Model,
    elapsed: Duration,
    ran: &[Box<dyn Rule>],
    diff: &Diff,
    fail_on_stale: bool,
    options: HumanOptions,
) -> String {
    let mut out = summary_line(model, elapsed);
    out.push('\n');
    write_health(&mut out, model, ran, &diff.surfaced, options);
    crate::human::write_resolution_warning(&mut out, model);

    write_ordered_findings(&mut out, ran, &diff.surfaced, options);

    let tally = tally(ran, &diff.surfaced);
    let _ = write!(
        out,
        "\n{} findings in {} of {} rules{} · {} warn · {} error{}{} · exit {} · {} new\n",
        tally.warn + tally.error + tally.info,
        tally.rules - tally.ok - tally.undecided,
        tally.rules,
        undecided_rules(&tally),
        tally.warn,
        tally.error,
        info_count(&tally),
        undecided_count(&tally),
        baselined_exit_code(diff, fail_on_stale),
        diff.surfaced.len(),
    );

    for fallen in &diff.fallen {
        let _ = writeln!(
            out,
            " prune  {} {} fell {}→{}; run `spirolo ratchet --prune` to drop it",
            fallen.file, fallen.rule, fallen.was, fallen.now
        );
    }
    if diff.is_stale() {
        let _ = writeln!(
            out,
            " stale  {} of {} baseline entries match nothing; run `spirolo ratchet --prune`{}",
            diff.unmatched,
            diff.total,
            if fail_on_stale {
                " (failing: --fail-on-stale-baseline)"
            } else {
                ""
            }
        );
    }
    write_hidden(&mut out, ran, options);
    write_run_notes(&mut out, model, options);
    out
}

/// The exit code a baselined run gives: `1` when any surfaced finding is warn- or
/// error-severity, or when the baseline is stale and `--fail-on-stale-baseline` is set;
/// `0` otherwise.
pub fn baselined_exit_code(diff: &Diff, fail_on_stale: bool) -> u8 {
    let gating = diff
        .surfaced
        .iter()
        .any(|f| f.severity != Severity::Info && f.outcome == Outcome::Finding);
    u8::from(gating || (fail_on_stale && diff.is_stale()))
}

/// The agent `check` view: each finding as one tab-separated row, its explain under it.
///
/// The row is the rule, its [rule status](rule_status), severity, location and message; the
/// explain is indented on the next line. No summary, no `ok` rows, no decoration. An empty
/// run renders empty.
pub fn render_check_agent(findings: &[Finding]) -> String {
    let mut out = String::new();
    for finding in findings {
        let _ = writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}",
            finding.rule,
            rule_status(finding.rule),
            finding.severity.name(),
            location(finding),
            finding.message,
        );
        let _ = writeln!(out, "\texplain\t{}", finding.explain);
        if let Some(proposal) = &finding.proposal {
            let _ = writeln!(
                out,
                "\tproposal\t{}",
                crate::proposal::proposal_line(proposal)
            );
        }
    }
    out
}

/// The model JSON with a `findings` key added.
///
/// No version bump, since adding a key never bumps [`crate::SCHEMA_VERSION`]. Keys are
/// sorted at every depth like the base document, so two runs over the same tree are
/// byte-equal.
pub fn to_check_json(model: &Model, findings: &[Finding]) -> String {
    let mut document = serde_json::to_value(model).expect("the model serializes infallibly");
    let object = document
        .as_object_mut()
        .expect("a model serializes to a JSON object");
    object.insert("schema_version".to_owned(), json!(crate::SCHEMA_VERSION));
    object.insert("findings".to_owned(), json!(findings_json(findings)));
    crate::canonicalise(&mut document);

    let mut json = serde_json::to_string_pretty(&document).expect("a JSON value re-serializes");
    json.push('\n');
    json
}

/// A finding's title path as `{suite_path, title}`, the two fields `spirolo list` gives the
/// test case, so a consumer joins the two on the same names. The SARIF surface puts the
/// same object in the result's properties.
pub fn test_json(test: &TestPath) -> serde_json::Value {
    json!({ "suite_path": test.suite_path, "title": test.title })
}

/// The `findings` array: one object per finding, keys the report and the harness read.
fn findings_json(findings: &[Finding]) -> Vec<serde_json::Value> {
    findings.iter().map(finding_json).collect()
}

/// One finding's object in the `findings` array; `explain` prints the same object.
pub fn finding_json(finding: &Finding) -> serde_json::Value {
    let mut object = json!({
        "rule": finding.rule,
        "rule_status": rule_status(finding.rule),
        "severity": finding.severity.name(),
        "outcome": outcome_token(finding.outcome),
        "id": finding.id(),
        "file": finding.location.file,
        "line": finding.location.line,
        "column": finding.location.column,
        "message": finding.message,
        "explain": finding.explain,
        "related": finding.related.iter().map(|r| json!({
            "label": r.label,
            "file": r.location.file,
            "line": r.location.line,
            "column": r.location.column,
        })).collect::<Vec<_>>(),
        "evidence": finding.evidence.iter().map(|e| json!({
            "key": e.key,
            "value": e.value,
        })).collect::<Vec<_>>(),
    });
    // Only on a finding anchored at a test: the key is absent, not null, elsewhere.
    if let Some(test) = &finding.test {
        object
            .as_object_mut()
            .expect("a finding serializes to a JSON object")
            .insert("test".to_owned(), test_json(test));
    }
    // Only under `--propose`: the field is absent when the finding carries none, so a
    // consumer that did not ask for proposals never sees the key.
    if let Some(proposal) = &finding.proposal {
        object
            .as_object_mut()
            .expect("a finding serializes to a JSON object")
            .insert(
                "proposal".to_owned(),
                crate::proposal::proposal_json(proposal),
            );
    }
    object
}

/// The rule's verdict token in the status table (`PASS`, `UNADJUDICATED`, … `DROPPED`), the
/// `rule_status` every machine format carries beside a finding's rule.
pub fn rule_status(rule: &str) -> &'static str {
    status::status(rule).verdict.name()
}

/// The finding's location as `file:line`, or `project` for the file-less project-wide
/// rows (the weak-assertion ratio).
fn location(finding: &Finding) -> String {
    if finding.location.file.is_empty() {
        "project".to_owned()
    } else {
        format!("{}:{}", finding.location.file, finding.location.line)
    }
}

/// The JSON token for an outcome.
pub const fn outcome_token(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Finding => "finding",
        Outcome::CouldNotDecide => "could_not_decide",
        Outcome::Listed => "listed",
    }
}
