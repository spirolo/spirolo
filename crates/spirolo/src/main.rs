//! `spirolo` — the fallow of tests. See docs/categories.md for the six families and
//! docs/adr/0001-single-rust-binary.md for why nothing here shells out.
//!
//! The binary is a surface over the model and the rules: it builds one model, runs the
//! rules over it, and hands both to `spirolo-report`. Exit codes follow fallow's scheme — 0
//! clean, 1 warn- or error-severity findings, 2 error — and `init`, `list`, `check`, `audit` and
//! `gate` exist so far.

mod baseline;
mod detect;
mod init;
mod inputs;
mod ledger;
mod lsp;
mod mcp;
mod rc;
mod resolution;

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use clap::{Parser, Subcommand, ValueEnum};
use spirolo_model::{BuildOptions, Model};
use spirolo_rules::{Config, Family};

/// Exit code for a clean run.
const EXIT_OK: u8 = 0;
/// Exit code for anything the tool could not do.
const EXIT_ERROR: u8 = 2;

#[derive(Parser)]
#[command(name = "spirolo", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Detect the stack and write a commented `.spirolorc.jsonc`.
    Init(InitArgs),
    /// Enumerate test files and cases the graph can see.
    List(ListArgs),
    /// Build the model once and run every enabled rule.
    Check(CheckProposeArgs),
    /// Cross-file wiring audit: handler drift, mock drift, absence anchors, weak
    /// assertions.
    Audit(CheckArgs),
    /// What actually blocks a merge: gate truth and dead skips.
    Gate(CheckArgs),
    /// Record today's findings as `.spirolo/baseline.json`; CI then fails only on new ones.
    Ratchet(RatchetArgs),
    /// Test-suite hygiene: duplicate, redundant, orphan and stale tests, and unused mocks.
    Hygiene(CheckProposeArgs),
    /// The test files a change reaches, ranked, each with its evidence.
    Impact(ImpactArgs),
    /// Static flake candidates and the local run ledger.
    Flake(FlakeArgs),
    /// One finding by its id, or one rule by its name, with the rule's measured status.
    Explain(ExplainArgs),
    /// A Language Server over stdio: the whole `check` pass on start and on every save,
    /// published as diagnostics, with hover.
    Lsp {
        /// Publish every rule's findings, not only those of rules shown by default.
        #[arg(long)]
        all: bool,
    },
    /// An MCP server over stdio, exposing the binary's analysis as tools.
    Mcp,
    /// Print the embedded corpus measurement table as JSON.
    RuleStatus {
        /// Output format (JSON).
        #[arg(long, value_parser = ["json"], default_value = "json")]
        format: String,
    },
}

/// `explain`'s arguments: a finding id or a rule name, and what `check` reads to find the
/// finding.
#[derive(clap::Args)]
struct ExplainArgs {
    /// A finding id as `check --format json` prints it, or a rule name.
    query: String,
    /// Project root to read. Defaults to the current directory.
    path: Option<PathBuf>,
    /// Read this config file instead of `.spirolorc.jsonc` in the root.
    #[arg(long)]
    config: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = ExplainFormat::Human)]
    format: ExplainFormat,
}

/// How `explain` renders.
#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum ExplainFormat {
    /// Labelled lines.
    Human,
    /// One JSON object.
    Json,
}

/// `impact`'s arguments: the changed files, from the command line or a list.
#[derive(clap::Args)]
struct ImpactArgs {
    /// Changed files, relative to the project root (as `git diff --name-only` prints them).
    files: Vec<PathBuf>,
    /// Read more changed files, one per line, from this file, or from stdin for `-`.
    #[arg(long)]
    changed: Option<PathBuf>,
    /// Project root to read. Defaults to the current directory.
    #[arg(long)]
    root: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = ImpactFormat::Human)]
    format: ImpactFormat,
    /// Print only the selected test files, space-separated, as a runner's arguments.
    #[arg(long, conflicts_with = "format")]
    print: bool,
    /// Rank source modules by churn × complexity × weakness instead (`untested-hotspot`'s
    /// ranking); the changed files are not read.
    #[arg(long, conflicts_with_all = ["files", "changed", "print"])]
    hotspots: bool,
    /// With `--hotspots`, print only the first N modules.
    #[arg(long, value_name = "N", requires = "hotspots")]
    top: Option<usize>,
    /// With `--hotspots`, read churn from this `git log --numstat` file, or from stdin for
    /// `-`, instead of the `inputs.churn` file.
    #[arg(long, requires = "hotspots")]
    churn: Option<PathBuf>,
}

/// How `impact` renders its answer.
#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum ImpactFormat {
    /// One ranked line per test file with its evidence, then `unreached`.
    Human,
    /// `{changed, selected, unreached, diagnostics}`, sorted by rank.
    Json,
}

/// `flake`'s arguments: a `check`-style run of the flake family, or one of its actions.
#[derive(clap::Args)]
#[command(args_conflicts_with_subcommands = true)]
struct FlakeArgs {
    #[command(subcommand)]
    action: Option<FlakeAction>,
    #[command(flatten)]
    common: CheckArgs,
}

/// What `spirolo flake` does besides reporting.
#[derive(Subcommand)]
enum FlakeAction {
    /// Read vitest JSON or `JUnit` XML reports into one normalized run file per report
    /// under `.spirolo/runs/`.
    Ingest(IngestArgs),
    /// Add the flakes the ledger observed to `.spirolo/quarantine.json`, keeping every entry
    /// already there. Nothing is applied to the runner.
    Quarantine(QuarantineArgs),
}

/// `flake quarantine`'s arguments.
#[derive(clap::Args)]
struct QuarantineArgs {
    /// Project root to read and write into. Defaults to the current directory.
    path: Option<PathBuf>,
}

/// `flake ingest`'s arguments.
#[derive(clap::Args)]
struct IngestArgs {
    /// Globs naming the reports, relative to the project root.
    #[arg(required = true)]
    globs: Vec<String>,
    /// Project root to write into. Defaults to the current directory.
    #[arg(long)]
    root: Option<PathBuf>,
    /// The tree hash the runs were made on; otherwise each report's `run.json` sidecar
    /// must name one.
    #[arg(long)]
    tree_hash: Option<String>,
    /// The run id; defaults to a content hash of the report. Only for a single report.
    #[arg(long)]
    run_id: Option<String>,
    /// A label for the runs' condition, for example `idle` or `load`.
    #[arg(long)]
    condition: Option<String>,
    /// Stable runner/configuration identity for these reports. Keep it the same across
    /// runs of one config, and different for independent configs collecting the same test.
    #[arg(long)]
    runner_home: Option<String>,
}

#[derive(clap::Args)]
struct InitArgs {
    /// Project root to write into. Defaults to the current directory.
    path: Option<PathBuf>,
    /// Overwrite an existing `.spirolorc.jsonc` instead of refusing.
    #[arg(long)]
    force: bool,
}

#[derive(clap::Args)]
struct ListArgs {
    /// Project root to read. Defaults to the current directory.
    path: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = ListFormat::Human)]
    format: ListFormat,
    /// Shorthand for `--format agent`.
    #[arg(long, conflicts_with = "format")]
    agent: bool,
}

/// The arguments `check`, `audit` and `gate` share: a root, a format, a config path and a
/// single-rule filter.
#[derive(clap::Args)]
struct CheckArgs {
    /// Project root to read. Defaults to the current directory.
    path: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = Format::Human)]
    format: Format,
    /// Shorthand for `--format agent`.
    #[arg(long, conflicts_with = "format")]
    agent: bool,
    /// Read this config file instead of `.spirolorc.jsonc` in the root.
    #[arg(long)]
    config: Option<PathBuf>,
    /// Run only the named rule, for example `weak-assertion`.
    #[arg(long)]
    rule: Option<String>,
    /// Show every rule's findings, not only those of rules shown by default (PASS, or enough
    /// decided findings at high enough precision in the corpus status). With `--rule`, also
    /// show every file and finding of that rule in human output, without display caps.
    #[arg(long)]
    all: bool,
    /// Exit non-zero when a quarter or more of `.spirolo/baseline.json` matches nothing,
    /// instead of only warning.
    #[arg(long)]
    fail_on_stale_baseline: bool,
    /// Read churn from this `git log --numstat` file, or from stdin for `-`, instead of the
    /// `inputs.churn` file.
    #[arg(long)]
    churn: Option<PathBuf>,
}

/// `check` and `hygiene`'s arguments: everything `CheckArgs` carries, plus `--propose`. The
/// flag lives here rather than on the shared `CheckArgs` so `audit` and `gate` reject it —
/// proposals are a hygiene concern, and the PRD limits the flag to these two commands.
#[derive(clap::Args)]
struct CheckProposeArgs {
    #[command(flatten)]
    common: CheckArgs,
    /// Add a proposal — `merge`, `delete` or `keep` — to every hygiene finding, in every
    /// format. Off by default; nothing is ever written to the tree.
    #[arg(long)]
    propose: bool,
}

/// `ratchet`'s arguments: a root, a config path, and the two debt-paydown flags.
#[derive(clap::Args)]
struct RatchetArgs {
    /// Project root to record. Defaults to the current directory.
    path: Option<PathBuf>,
    /// Read this config file instead of `.spirolorc.jsonc` in the root.
    #[arg(long)]
    config: Option<PathBuf>,
    /// Lower every count and ratio ceiling to the current run instead of rewriting from
    /// scratch; requires an existing baseline.
    #[arg(long)]
    tighten: bool,
    /// Drop baseline entries the current run matches nothing for; requires an existing
    /// baseline.
    #[arg(long)]
    prune: bool,
}

/// How a command renders its output.
#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Format {
    /// A summary line and readable tables or findings.
    Human,
    /// The versioned JSON schema.
    Json,
    /// SARIF 2.1, for GitHub code scanning and IDE problem panels.
    Sarif,
    /// Tables, findings and diagnostics only, tab-separated: no summary, no decoration.
    Agent,
    /// A Markdown job summary with measured status and collapsed info.
    Markdown,
    /// Escaped GitHub workflow-command warnings, capped per step, and a summary.
    GithubAnnotations,
}

/// Listing formats describe the graph rather than rule findings.
#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum ListFormat {
    Human,
    Json,
    Sarif,
    Agent,
}

/// The `driver.version` SARIF reports: the binary's own crate version.
const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> ExitCode {
    let cli = Cli::parse();
    let code = match cli.command {
        Command::Init(args) => match resolve_root(args.path.as_deref()) {
            Ok(root) => init::init(&root, args.force),
            Err(message) => {
                eprintln!("spirolo init: {message}");
                EXIT_ERROR
            }
        },
        Command::List(args) => list(&args),
        Command::Check(args) => check(&args.common, "check", None, args.propose),
        Command::Audit(args) => check(&args, "audit", Some(Family::Audit), false),
        Command::Gate(args) => check(&args, "gate", Some(Family::Gate), false),
        Command::Ratchet(args) => ratchet(&args),
        Command::Hygiene(args) => {
            check(&args.common, "hygiene", Some(Family::Hygiene), args.propose)
        }
        Command::Impact(args) => impact(&args),
        Command::Flake(FlakeArgs {
            action: Some(FlakeAction::Ingest(args)),
            ..
        }) => ingest(&args),
        Command::Flake(FlakeArgs {
            action: Some(FlakeAction::Quarantine(args)),
            ..
        }) => quarantine(&args),
        Command::Flake(args) => check(&args.common, "flake", Some(Family::Flake), false),
        Command::Explain(args) => explain(&args),
        Command::Lsp { all } => lsp::serve(all),
        Command::Mcp => mcp::serve(),
        Command::RuleStatus { .. } => write_stdout("rule-status", spirolo_rules::status::json()),
    };
    ExitCode::from(code)
}

/// Which rules a command runs: the one `--rule` names, the ones in `family`, or all of
/// them. Returns the rules and the families to filter model-side reporting by.
///
/// An unknown `--rule` name is the caller's error, reported by `check` as exit 2.
fn rules_to_run(
    rule: Option<&str>,
    family: Option<Family>,
) -> Result<Vec<Box<dyn spirolo_rules::Rule>>, String> {
    if let Some(name) = rule {
        let rule =
            spirolo_rules::rule_named(name).ok_or_else(|| format!("no such rule {name:?}"))?;
        if let Some(family) = family {
            if rule.family() != family {
                return Err(format!(
                    "rule {name:?} is not in the {} family",
                    family.name()
                ));
            }
        }
        return Ok(vec![rule]);
    }
    Ok(spirolo_rules::registry()
        .into_iter()
        .filter(|rule| family.is_none_or(|family| rule.family() == family))
        .collect())
}

/// One run's product: the model, how long it took to build, the chosen rules and their
/// findings. `check` and `ratchet` both build this, then diverge on what they do with it.
struct Run {
    /// The resolved project root, so callers read and write `.spirolo/baseline.json` under it
    /// without re-resolving the path.
    root: PathBuf,
    model: Model,
    elapsed: std::time::Duration,
    ran: Vec<Box<dyn spirolo_rules::Rule>>,
    findings: Vec<spirolo_rules::Finding>,
    /// The ran rules a config severity override asks to gate (`warn` or `error`) and whose
    /// rows it does gate: the default view shows them whatever their tier.
    opted_in: Vec<&'static str>,
    /// The ran rules a config override asks to gate but that never gate: DROPPED or FAIL.
    not_applied: Vec<&'static str>,
    /// True when the config could not be parsed: the run may have lost information, so the
    /// caller exits 2 the way a model error does, after printing what it could.
    config_failed: bool,
}

/// Build the model once and run the chosen rules over it: the shared body of `check`,
/// `audit`, `gate` and `ratchet`. `family` scopes the rules; `None` runs every family.
fn build_run(
    name: &str,
    path: Option<&Path>,
    config: Option<&Path>,
    rule: Option<&str>,
    family: Option<Family>,
    propose: bool,
    churn: Option<&Path>,
) -> Result<Run, u8> {
    try_build_run(path, config, rule, family, propose, churn).map_err(|message| {
        eprintln!("spirolo {name}: {message}");
        EXIT_ERROR
    })
}

/// [`build_run`] without the printing: an error is the message `build_run` would print.
fn try_build_run(
    path: Option<&Path>,
    config: Option<&Path>,
    rule: Option<&str>,
    family: Option<Family>,
    propose: bool,
    churn: Option<&Path>,
) -> Result<Run, String> {
    let root = resolve_root(path)?;
    let ran = rules_to_run(rule, family)?;

    // `--config <path>` reads a config outside the root; a mistyped path is an error, but a
    // missing default beside the repo is simply no config. A malformed config is a
    // diagnostic, never an abort.
    let config_path = rc::resolve_config_path(&root, config)?;
    let (mut rc, rc_diagnostics) = rc::load_path(&config_path);
    // The claims documents are read here, not in the rules crate: the rule diffs their lines but
    // never opens a file. Paths are relative to the project root, so a pinned corpus clone
    // checked with `--config` reads its docs from the clone, not from beside the config.
    rc.config.claim_documents = rc::read_claim_documents(&root, &rc.config.claims);
    // `--propose` is a run-time choice, not a config key: the flag turns proposals on for
    // this run, and the hygiene rules read it off the config the binary hands them.
    rc.config.propose = propose;

    let started = Instant::now();
    let mut model = Model::build_with(
        &root,
        &BuildOptions {
            handler_globs: rc.handler_globs.clone(),
            handler_globs_source: Some(rc::display_name(&config_path)),
            ..BuildOptions::default()
        },
        &spirolo_frameworks::recognisers(),
    );
    let elapsed = started.elapsed();

    if !rc_diagnostics.is_empty() {
        model.diagnostics.extend(rc_diagnostics);
        model.diagnostics.sort();
    }
    attach_mutation_reports(&mut model, &root, &rc.mutation_globs);
    attach_snapshots(&mut model, &root);
    attach_runs(&mut model, &root);
    attach_churn(&mut model, &root, rc.churn_file.as_deref(), churn);
    // `untested-hotspot` counts churn as 1 without an input; the run says so once.
    if ran.iter().any(|rule| rule.name() == "untested-hotspot") {
        if let Some(diagnostic) = spirolo_rules::churn_missing(&model) {
            model.diagnostics.push(diagnostic);
            model.diagnostics.sort();
        }
    }
    let config_failed = model
        .diagnostics
        .iter()
        .any(|d| d.code == "spirolorc_parse_failed" || d.code == "hygiene_severity_rejected");

    // A tsconfig that could not load leaves holes in the import graph; say so, and mark the
    // rows that read it.
    let unresolved = resolution::Failure::of(&model, &root);
    let mut findings = run_rules(&ran, &model, &rc.config);
    if let Some(failure) = unresolved {
        let marked = failure.mark(&mut findings);
        model.diagnostics.push(failure.diagnostic(marked));
        model.diagnostics.sort();
    }
    let (opted_in, not_applied) = ran
        .iter()
        .filter(|rule| {
            rc.config
                .rules
                .get(rule.name())
                .and_then(|entry| entry.severity)
                .is_some_and(|severity| severity != spirolo_rules::Severity::Info)
        })
        .map(|rule| rule.name())
        .partition(|rule| gates(rule));

    Ok(Run {
        root,
        model,
        elapsed,
        ran,
        findings,
        opted_in,
        not_applied,
        config_failed,
    })
}

/// `check`, `audit` and `gate`: build the model once, run the chosen rules over it, and
/// render the findings. `family` scopes the rules; `None` runs every family. When
/// `.spirolo/baseline.json` exists, only findings above the recorded counts and ratio
/// ceilings surface, and prune and staleness warnings follow the closing line.
fn check(args: &CheckArgs, name: &str, family: Option<Family>, propose: bool) -> u8 {
    match render_check(args, family, propose) {
        Ok((rendered, code)) => {
            let write = write_stdout(name, &rendered);
            if write == EXIT_OK {
                code
            } else {
                write
            }
        }
        Err(message) => {
            eprintln!("spirolo {name}: {message}");
            EXIT_ERROR
        }
    }
}

/// Shared CLI/MCP reporting: the same config, rules, baseline and JSON renderer.
fn render_check(
    args: &CheckArgs,
    family: Option<Family>,
    propose: bool,
) -> Result<(String, u8), String> {
    let run = try_build_run(
        args.path.as_deref(),
        args.config.as_deref(),
        args.rule.as_deref(),
        family,
        propose,
        args.churn.as_deref(),
    )?;

    let baseline = baseline::load(&run.root)?;

    let format = if args.agent {
        Format::Agent
    } else {
        args.format
    };

    // Against a baseline, `check` renders only what rose; without one, it renders every
    // finding, as before.
    let (rendered, code) = if let Some(baseline) = &baseline {
        {
            let mut diff = baseline.diff(&run.findings);
            let hidden = shown_only(&mut diff.surfaced, args.all, &run.opted_in);
            let rendered = match format {
                Format::Human => spirolo_report::render_check_human_baselined_with_options(
                    &run.model,
                    run.elapsed,
                    &run.ran,
                    &diff,
                    args.fail_on_stale_baseline,
                    spirolo_report::HumanOptions {
                        all: args.all && args.rule.is_some(),
                        family,
                        hidden: &hidden,
                        rule: args.rule.as_deref(),
                        not_applied: &run.not_applied,
                    },
                ),
                Format::Json => spirolo_report::to_check_json(&run.model, &diff.surfaced),
                Format::Sarif => spirolo_report::to_sarif(&diff.surfaced, VERSION),
                Format::Agent => spirolo_report::render_check_agent(&diff.surfaced),
                Format::Markdown => spirolo_report::render_check_markdown(&diff.surfaced),
                Format::GithubAnnotations => {
                    spirolo_report::render_github_annotations(&diff.surfaced)
                }
            };
            let code = spirolo_report::baselined_exit_code(&diff, args.fail_on_stale_baseline);
            (rendered, code)
        }
    } else {
        let mut findings = run.findings;
        let hidden = shown_only(&mut findings, args.all, &run.opted_in);
        let rendered = match format {
            Format::Human => spirolo_report::render_check_human_with_options(
                &run.model,
                run.elapsed,
                &run.ran,
                &findings,
                spirolo_report::HumanOptions {
                    all: args.all && args.rule.is_some(),
                    family,
                    hidden: &hidden,
                    rule: args.rule.as_deref(),
                    not_applied: &run.not_applied,
                },
            ),
            Format::Json => spirolo_report::to_check_json(&run.model, &findings),
            Format::Sarif => spirolo_report::to_sarif(&findings, VERSION),
            Format::Agent => spirolo_report::render_check_agent(&findings),
            Format::Markdown => spirolo_report::render_check_markdown(&findings),
            Format::GithubAnnotations => spirolo_report::render_github_annotations(&findings),
        };
        let code = spirolo_report::tally(&run.ran, &findings).exit_code();
        (rendered, code)
    };

    // A parse-failed config is an error diagnostic; exit 2 the way a model error does, after
    // printing what it could.
    let code = if run.config_failed { EXIT_ERROR } else { code };

    Ok((rendered, code))
}

/// The default view keeps the rows of rules [shown by default](spirolo_rules::status::Tier)
/// (proven and advisory) and of the `opted_in` rules a config override gates, and returns
/// the rest, in order; `--all` keeps every row and hides nothing.
fn shown_only(
    findings: &mut Vec<spirolo_rules::Finding>,
    all: bool,
    opted_in: &[&str],
) -> Vec<spirolo_rules::Finding> {
    if all {
        return Vec::new();
    }
    let (shown, hidden) = std::mem::take(findings).into_iter().partition(|finding| {
        spirolo_rules::status::shown_by_default(finding.rule) || opted_in.contains(&finding.rule)
    });
    *findings = shown;
    hidden
}

/// What one `check` pass over a root leaves behind for `spirolo lsp`.
struct Checked {
    /// The model the rules ran over.
    model: Model,
    /// The findings `check` prints: after the baseline filter, every rule's under `--all`,
    /// otherwise only those of rules shown by default (proven and advisory) or opted in by a
    /// config override that gates.
    findings: Vec<spirolo_rules::Finding>,
    /// The rows of rules hidden by default the default view left out; empty under `--all`.
    hidden: Vec<spirolo_rules::Finding>,
    /// The number of entries in `.spirolo/baseline.json`, or `None` when there is none.
    baseline: Option<usize>,
}

/// The findings `check` (with `--all` when `all`) prints for `root` with its default config
/// lookup, after the baseline filter when `.spirolo/baseline.json` exists. An error is why the
/// run could not be trusted: the root, the baseline or a config `check` would exit 2 on.
fn checked(root: &Path, all: bool) -> Result<Checked, String> {
    let run = try_build_run(Some(root), None, None, None, false, None)?;
    if run.config_failed {
        let reason = run
            .model
            .diagnostics
            .iter()
            .find(|d| d.code == "spirolorc_parse_failed" || d.code == "hygiene_severity_rejected")
            .map_or("the config could not be read", |d| d.message.as_str());
        return Err(reason.to_owned());
    }
    let (mut findings, baseline) = match baseline::load(&run.root)? {
        Some(baseline) => {
            let diff = baseline.diff(&run.findings);
            (diff.surfaced, Some(diff.total))
        }
        None => (run.findings, None),
    };
    let hidden = shown_only(&mut findings, all, &run.opted_in);
    Ok(Checked {
        model: run.model,
        findings,
        hidden,
        baseline,
    })
}

/// `explain`: a rule name prints the rule; anything else is a finding id, looked up in
/// the findings `check` would produce before the baseline filter.
fn explain(args: &ExplainArgs) -> u8 {
    match render_explain(args) {
        Ok(rendered) => write_stdout("explain", &rendered),
        Err(message) => {
            eprintln!("spirolo explain: {message}");
            EXIT_ERROR
        }
    }
}

/// Shared CLI/MCP explanation, including lookup before the baseline filter.
fn render_explain(args: &ExplainArgs) -> Result<String, String> {
    if let Some(rule) = spirolo_rules::rule_named(&args.query) {
        let rendered = match args.format {
            ExplainFormat::Json => spirolo_report::explain_rule_json(rule.as_ref()),
            ExplainFormat::Human => spirolo_report::explain_rule_human(rule.as_ref()),
        };
        return Ok(rendered);
    }

    let run = try_build_run(
        args.path.as_deref(),
        args.config.as_deref(),
        None,
        None,
        false,
        None,
    )?;
    let Some(finding) = run.findings.iter().find(|f| f.id() == args.query) else {
        return Err(format!("no finding with id '{}'", args.query));
    };
    let rendered = match args.format {
        ExplainFormat::Json => spirolo_report::explain_json(finding),
        ExplainFormat::Human => spirolo_report::explain_human(finding),
    };
    Ok(rendered)
}

/// `ratchet`: record today's findings as `.spirolo/baseline.json`, so CI fails only on new
/// ones. With no flag it writes a fresh baseline; `--tighten` lowers an existing one's
/// counts and ceilings to the current run; `--prune` drops entries the run no longer
/// matches. The two flags read the existing baseline and rewrite it against this run.
fn ratchet(args: &RatchetArgs) -> u8 {
    let run = match build_run(
        "ratchet",
        args.path.as_deref(),
        args.config.as_deref(),
        None,
        None,
        false,
        None,
    ) {
        Ok(run) => run,
        Err(code) => return code,
    };
    if run.config_failed {
        return EXIT_ERROR;
    }

    let baseline = if args.tighten || args.prune {
        // Edit an existing baseline: read it, apply the transforms against this run, write it
        // back. A first-run edit with no baseline is a fresh recording of today's findings,
        // which is the same as no flag — so a missing file falls through to `from_findings`.
        let mut baseline = match baseline::load(&run.root) {
            Ok(Some(baseline)) => baseline,
            Ok(None) => spirolo_report::Baseline::from_findings(&run.findings),
            Err(message) => {
                eprintln!("spirolo ratchet: {message}");
                return EXIT_ERROR;
            }
        };
        if args.tighten {
            baseline.tighten(&run.findings);
        }
        if args.prune {
            baseline.prune(&run.findings);
        }
        baseline
    } else {
        spirolo_report::Baseline::from_findings(&run.findings)
    };

    if let Err(message) = baseline::store(&run.root, &baseline) {
        eprintln!("spirolo ratchet: {message}");
        return EXIT_ERROR;
    }
    EXIT_OK
}

/// Run each chosen rule over the model, concatenate their findings, keeping each rule's
/// own order (the registry order across rules), with every proposal gated on its rule's
/// independent-oracle verdict.
fn run_rules(
    ran: &[Box<dyn spirolo_rules::Rule>],
    model: &Model,
    config: &Config,
) -> Vec<spirolo_rules::Finding> {
    let mut findings = Vec::new();
    for rule in ran {
        let mut mine = rule.run(model, config);
        let override_severity = config
            .rules
            .get(rule.name())
            .and_then(|entry| entry.severity);
        // A DROPPED or FAIL rule never gates, whatever the config asks.
        if let (Some(severity), true) = (override_severity, gates(rule.name())) {
            for finding in &mut mine {
                if finding.outcome == spirolo_rules::Outcome::Finding {
                    finding.severity = severity;
                }
            }
        } else if !spirolo_rules::status::is_proven(rule.name()) {
            for finding in &mut mine {
                finding.severity = spirolo_rules::Severity::Info;
            }
        }
        findings.extend(mine);
    }
    spirolo_rules::gate_proposals(&mut findings, spirolo_rules::PROPOSALS_CLEARED);
    findings
}

/// Whether a config severity override can make the rule gate: every rule but a DROPPED or
/// FAIL one, which reports at `info` and stays hidden by default whatever the config asks.
fn gates(rule: &str) -> bool {
    spirolo_rules::status::tier(rule) != spirolo_rules::status::Tier::Hidden
}

fn list(args: &ListArgs) -> u8 {
    let (model, elapsed) = match build_list(args.path.as_deref()) {
        Ok(built) => built,
        Err(message) => {
            eprintln!("spirolo list: {message}");
            return EXIT_ERROR;
        }
    };

    let format = if args.agent {
        ListFormat::Agent
    } else {
        args.format
    };
    let rendered = match format {
        ListFormat::Human => spirolo_report::render_human(&model, elapsed),
        ListFormat::Json => spirolo_report::to_json(&model),
        // `list` enumerates the graph; it has no findings, so SARIF is a valid empty run.
        ListFormat::Sarif => spirolo_report::to_sarif(&[], VERSION),
        ListFormat::Agent => spirolo_report::render_agent(&model),
    };
    write_stdout("list", &rendered)
}

/// Build `list`'s model, including config diagnostics and mutation artifacts, for either
/// CLI rendering or MCP's JSON result.
fn build_list(path: Option<&Path>) -> Result<(Model, std::time::Duration), String> {
    let root = resolve_root(path)?;
    // `.spirolorc.jsonc` supplies the handler globs; a malformed one is a diagnostic, not a
    // failure, and leaves the build on its defaults.
    let (config, rc_diagnostics) = rc::load(&root);

    let started = Instant::now();
    let mut model = Model::build_with(
        &root,
        &BuildOptions {
            handler_globs: config.handler_globs,
            handler_globs_source: Some(rc::FILE_NAME.to_owned()),
            ..BuildOptions::default()
        },
        &spirolo_frameworks::recognisers(),
    );
    let elapsed = started.elapsed();

    if !rc_diagnostics.is_empty() {
        model.diagnostics.extend(rc_diagnostics);
        model.diagnostics.sort();
    }
    attach_mutation_reports(&mut model, &root, &config.mutation_globs);

    Ok((model, elapsed))
}

/// `impact`: build the model, rank the test files the changed files reach, and print them.
/// Exit 0 unless the root or the changed-file list could not be read.
fn impact(args: &ImpactArgs) -> u8 {
    let root = match resolve_root(args.root.as_deref()) {
        Ok(root) => root,
        Err(message) => {
            eprintln!("spirolo impact: {message}");
            return EXIT_ERROR;
        }
    };
    if args.hotspots {
        return impact_hotspots(args, &root);
    }
    let mut changed: Vec<String> = args
        .files
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    if let Some(list) = &args.changed {
        match read_changed_list(list) {
            Ok(lines) => changed.extend(lines),
            Err(message) => {
                eprintln!("spirolo impact: {message}");
                return EXIT_ERROR;
            }
        }
    }
    let impact = build_impact(&root, &changed);
    let rendered = if args.print {
        // The paths go to a runner; what could not be used goes to stderr, not its argv.
        for diagnostic in &impact.diagnostics {
            eprintln!("spirolo impact: {}", diagnostic.message);
        }
        spirolo_report::render_impact_print(&impact)
    } else {
        match args.format {
            ImpactFormat::Human => spirolo_report::render_impact_human(&impact),
            ImpactFormat::Json => spirolo_report::impact_to_json(&impact),
        }
    };
    write_stdout("impact", &rendered)
}

/// The shared CLI/MCP impact pass; the ranking reads the same ingested run artifacts.
fn build_impact(root: &Path, changed: &[String]) -> spirolo_rules::Impact {
    let mut model = Model::build_with(
        root,
        &BuildOptions::default(),
        &spirolo_frameworks::recognisers(),
    );
    // Keys 3 and 5 read the ingested runs; with none, they are inert.
    attach_runs(&mut model, root);
    spirolo_rules::impact(&model, changed)
}

/// `impact --hotspots`: build the model with the churn input and print `untested-hotspot`'s
/// ranking, the first `--top` modules of it when given. Exit 0: an unreadable churn input is
/// a diagnostic, as in `check`.
fn impact_hotspots(args: &ImpactArgs, root: &Path) -> u8 {
    let (rc, _) = rc::load(root);
    let mut model = Model::build_with(
        root,
        &BuildOptions {
            handler_globs: rc.handler_globs,
            handler_globs_source: Some(rc::FILE_NAME.to_owned()),
            ..BuildOptions::default()
        },
        &spirolo_frameworks::recognisers(),
    );
    attach_churn(
        &mut model,
        root,
        rc.churn_file.as_deref(),
        args.churn.as_deref(),
    );
    let mut hotspots = spirolo_rules::hotspots(&model);
    // What the churn reader could not use is part of the answer, beside the ranking's own.
    hotspots.diagnostics.extend(
        model
            .diagnostics
            .iter()
            .filter(|d| d.code.starts_with("churn_"))
            .cloned(),
    );
    hotspots.diagnostics.sort();
    if let Some(top) = args.top {
        hotspots.hotspots.truncate(top);
    }
    let rendered = match args.format {
        ImpactFormat::Human => spirolo_report::render_hotspots_human(&hotspots),
        ImpactFormat::Json => spirolo_report::hotspots_to_json(&hotspots),
    };
    write_stdout("impact", &rendered)
}

/// The changed files `--changed` names, one per line, blank lines skipped: from stdin for
/// `-`, otherwise from the file at that path.
fn read_changed_list(list: &Path) -> Result<Vec<String>, String> {
    let text = if list == Path::new("-") {
        std::io::read_to_string(std::io::stdin())
            .map_err(|error| format!("cannot read the changed files from stdin: {error}"))?
    } else {
        std::fs::read_to_string(list)
            .map_err(|error| format!("cannot read {}: {error}", list.display()))?
    };
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect())
}

/// Read the mutation reports `inputs.mutation` names (or its default) into the model, so
/// `list`, `check` and friends all carry the `Mutants` table. An absent key falls back to
/// the default report path, so a first run reads a committed report with no configuration.
fn attach_mutation_reports(model: &mut Model, root: &Path, mutation_globs: &[String]) {
    let default = [rc::DEFAULT_MUTATION_GLOB.to_owned()];
    let globs = if mutation_globs.is_empty() {
        &default[..]
    } else {
        mutation_globs
    };
    let mutation = inputs::read_mutation_reports(root, globs, &model.test_cases);
    model.mutants = mutation.mutants;
    model.mutation_tests = mutation.tests;
    model.mutation_reports = mutation.reports;
    if !mutation.diagnostics.is_empty() {
        model.diagnostics.extend(mutation.diagnostics);
        model.diagnostics.sort();
    }
}

/// Walk the `__snapshots__/*.snap` files under `root`, read each with the frameworks
/// reader, and attach the rows to the model so `stale-snapshot` can query them. The sibling
/// test file is set when it is a module the model read; a malformed key is a diagnostic.
fn attach_snapshots(model: &mut Model, root: &Path) {
    let snapshots = inputs::read_snapshots(root, &model.modules);
    model.snapshots = snapshots.snapshots;
    if !snapshots.diagnostics.is_empty() {
        model.diagnostics.extend(snapshots.diagnostics);
        model.diagnostics.sort();
    }
}

/// Read the normalized run files under `.spirolo/runs/` into the model's `Run` table, joined
/// to its tests. `list` never calls this: raw reports are read only by `flake ingest`.
fn attach_runs(model: &mut Model, root: &Path) {
    let (runs, diagnostics) = ledger::read_runs(root, &model.test_cases);
    model.runs = runs;
    if !diagnostics.is_empty() {
        model.diagnostics.extend(diagnostics);
        model.diagnostics.sort();
    }
}

/// Read the churn input into the model: the `--churn` file when given (stdin for `-`),
/// otherwise the `inputs.churn` file under `root`. With neither, `model.churn` stays `None`.
fn attach_churn(model: &mut Model, root: &Path, rc_file: Option<&str>, flag: Option<&Path>) {
    let (churn, diagnostics) = inputs::read_churn(root, rc_file, flag);
    model.churn = churn;
    if !diagnostics.is_empty() {
        model.diagnostics.extend(diagnostics);
        model.diagnostics.sort();
    }
}

/// `flake ingest`: write one run file per report, print each path written and any reader
/// diagnostic, and exit 2 when a report is refused.
fn ingest(args: &IngestArgs) -> u8 {
    let root = match resolve_root(args.root.as_deref()) {
        Ok(root) => root,
        Err(message) => {
            eprintln!("spirolo flake ingest: {message}");
            return EXIT_ERROR;
        }
    };
    let request = ledger::Ingest {
        globs: &args.globs,
        tree_hash: args.tree_hash.as_deref(),
        run_id: args.run_id.as_deref(),
        condition: args.condition.as_deref(),
        runner_home: args.runner_home.as_deref(),
    };
    match ledger::ingest(&root, &request) {
        Ok((written, diagnostics)) => {
            for diagnostic in diagnostics {
                eprintln!("spirolo flake ingest: {}", diagnostic.message);
            }
            let mut out = String::new();
            for path in written {
                out.push_str(&path);
                out.push('\n');
            }
            write_stdout("flake ingest", &out)
        }
        Err(message) => {
            eprintln!("spirolo flake ingest: {message}");
            EXIT_ERROR
        }
    }
}

/// `flake quarantine`: run `flake-observed` over the ledger, merge its flakes into
/// `.spirolo/quarantine.json`, and print the file's path. Exit 2 when the existing file does
/// not parse, which is then left as it was.
fn quarantine(args: &QuarantineArgs) -> u8 {
    let name = "flake quarantine";
    let run = match build_run(
        name,
        args.path.as_deref(),
        None,
        Some("flake-observed"),
        None,
        false,
        None,
    ) {
        Ok(run) => run,
        Err(code) => return code,
    };
    match ledger::quarantine(&run.root, &run.findings, &run.model.runs) {
        Ok(()) => write_stdout(name, &format!("{}\n", ledger::QUARANTINE_FILE)),
        Err(message) => {
            eprintln!("spirolo {name}: {message}");
            EXIT_ERROR
        }
    }
}

/// Write the rendered view, treating a closed pipe (`spirolo check | head`) as a clean exit
/// rather than the panic `print!` would raise.
fn write_stdout(command: &str, rendered: &str) -> u8 {
    let mut stdout = std::io::stdout().lock();
    match stdout
        .write_all(rendered.as_bytes())
        .and_then(|()| stdout.flush())
    {
        Ok(()) => EXIT_OK,
        Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => EXIT_OK,
        Err(error) => {
            eprintln!("spirolo {command}: cannot write to stdout: {error}");
            EXIT_ERROR
        }
    }
}

/// The directory to read: the argument, or the current directory when there is none.
fn resolve_root(path: Option<&Path>) -> Result<PathBuf, String> {
    let root = match path {
        Some(path) => path.to_path_buf(),
        None => std::env::current_dir()
            .map_err(|error| format!("cannot read the current directory: {error}"))?,
    };

    if !root.is_dir() {
        return Err(format!("{} is not a directory", root.display()));
    }

    Ok(root)
}
#[cfg(test)]
mod display_tests;
