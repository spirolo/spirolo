//! `spirolo-bench`: two sections over every reachable corpus entry.
//!
//! The counts section (M1) runs `spirolo list --format json`, diffs the per-framework counts
//! against that entry's `truth.json`, and checks the median of several timed runs against the
//! milestone budget.
//! The precision section (M2) runs `spirolo check --format json --config`, joins the reported
//! findings to the entry's `adjudications`, prints precision, recall and the 95 % Wilson
//! interval per rule with a verdict, and replays each recorded commit pair in a temporary
//! git worktree of the clone. An entry with an impact oracle (`impact/truth.json`, M4b)
//! also gets its `impact` safety, ranking, hotspot and budget rows, measured on the clone.
//! `just bench` runs it; `spirolo-bench status --write` runs the same and then writes
//! `corpus/rule-status.json` from its verdicts (PRD 0006). Counts run first; the process exits
//! non-zero when either section is red, and with the error code when nothing was measured
//! at all (every entry pending or un-cloned).
//!
//! The harness logic lives in the library ([`spirolo_bench`]); this file only wires it to the
//! real `spirolo` executable and to git, and turns the verdicts into an exit code.

mod m5_run;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Instant;

use serde_json::Value;
use spirolo_bench::{
    any_failed, memberships, nothing_measured, partition_precision, policy_tables, render,
    render_precision, reported_findings, run_corpus, run_precision, score_entry_impact,
    score_entry_ledger, section_failed, stage_runs, status_mismatches, EntryReport, ImpactBudget,
    ImpactEntryRunners, LedgerRun, M5Budget, M5Rows, PrecisionReport, QuarantineRoundTrip,
    ReplayPair, ReportedFinding, Run, Wallclock, REGISTERED_RULES, RULE_FLOORS,
};

/// Non-zero exit when at least one entry failed its diff or budget, or a rule or replay pair
/// failed its precision.
const EXIT_MISMATCH: u8 = 1;
/// Non-zero exit when the harness could not run at all (no corpus, unreadable truth), or
/// ran and measured nothing (every entry pending or un-cloned) — a vacuous pass is not a
/// pass.
const EXIT_ERROR: u8 = 2;

fn main() -> ExitCode {
    // `spirolo-bench status [--write]`: the same run, then the rule-status file (PRD 0006),
    // printed or written to `corpus/rule-status.json`.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(exit) = nightly_command(&args) {
        return exit;
    }
    // `spirolo-bench tally <repo>`: the `counts` and `substrate` a new entry's truth.json
    // records, from one `spirolo list` of the clone.
    if let [command, repo] = args.as_slice() {
        if command == "tally" {
            return print_tally(Path::new(repo));
        }
    }
    let status = match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        [] => None,
        ["status"] => Some(false),
        ["status", "--write"] => Some(true),
        _ => {
            eprintln!("usage: spirolo-bench [status [--write] | tally <repo> | nightly <ledger> <manifest> | nightly-counts <list> <truth>]");
            return ExitCode::from(EXIT_ERROR);
        }
    };
    let corpus_dir = corpus_dir();
    if !corpus_dir.is_dir() {
        eprintln!(
            "spirolo-bench: no corpus at {} — set SPIROLO_CORPUS_DIR or run from the repo root",
            corpus_dir.display()
        );
        return ExitCode::from(EXIT_ERROR);
    }

    let nightly_ledger = std::env::var_os("SPIROLO_NIGHTLY_LEDGER")
        .map_or_else(|| corpus_dir.join("nightly/ledger.jsonl"), PathBuf::from);
    let nightly_manifest = std::env::var_os("SPIROLO_NIGHTLY_MANIFEST")
        .map_or_else(|| PathBuf::from("corpus/manifest.toml"), PathBuf::from);
    let nightly_failed = nightly_row(&nightly_ledger, &nightly_manifest);
    let spirolo = spirolo_bin();

    let run_list = |repo: &Path| run_spirolo_list(&spirolo, repo);
    let counts = match run_corpus(&corpus_dir, &run_list) {
        Ok(reports) => reports,
        Err(error) => {
            eprintln!("spirolo-bench: {error}");
            return ExitCode::from(EXIT_ERROR);
        }
    };
    // The counts section runs first, so a model regression is caught before a rule is judged.
    print!("{}", render(&counts));

    // The rules the precision section lists, in registry order, so a rule that reported
    // nothing on every entry still shows `SKIPPED`. The registered-rule list mirrors
    // `spirolo_rules::registry()`; the bench does not link the rules crate (it reads `check`'s
    // JSON like any other consumer).
    let rules: Vec<String> = REGISTERED_RULES
        .iter()
        .map(|rule| (*rule).to_owned())
        .collect();
    let run_check = |repo: &Path, config: &Path| staged_pin_check(&spirolo, repo, config);
    let replay =
        |repo: &Path, pair: &ReplayPair, commit: &str| replay_finding(&spirolo, repo, pair, commit);
    let incident_check = |repo: &Path, commit: &str| check_at_commit(&spirolo, repo, commit);
    let mut precision =
        match run_precision(&corpus_dir, &rules, &run_check, &replay, &incident_check) {
            Ok(reports) => reports,
            Err(error) => {
                eprintln!("spirolo-bench: {error}");
                return ExitCode::from(EXIT_ERROR);
            }
        };
    // M4: each measured entry's run ledger, staged at its tree hash beside the clone.
    let ledger_run = |runs: &Path, repo: &Path| run_ledger(&spirolo, runs, repo);
    for report in &mut precision {
        let dir = corpus_dir.join(&report.name);
        let repo = dir.join("repo");
        if !repo.is_dir() {
            continue;
        }
        match score_entry_ledger(&dir, &repo, &ledger_run) {
            Ok(ledger) => report.ledger = ledger,
            Err(error) => {
                eprintln!("spirolo-bench: {}: {error}", report.name);
                return ExitCode::from(EXIT_ERROR);
            }
        }
        // M4b: `impact` and `untested-hotspot` against the entry's frozen mutation oracle,
        // on the clone itself (the installed substrate the oracle was generated on).
        report.impact = impact_rows(&spirolo, &dir, &repo, report.not_measured.as_deref());
    }
    // M5: the surface rows of each entry with an LSP plan, against the committed status file.
    m5_rows(&spirolo, &corpus_dir, &counts, &mut precision);
    println!();
    let precision_failed = match print_precision(&corpus_dir, &precision) {
        Ok(failed) => failed,
        Err(error) => {
            eprintln!("spirolo-bench: corpus selection: {error}");
            return ExitCode::from(EXIT_ERROR);
        }
    };

    if let Some(why) = nothing_measured(&counts, &precision) {
        eprintln!("spirolo-bench: nothing measured — {why}");
        return ExitCode::from(EXIT_ERROR);
    }
    if let Some(write) = status {
        return write_status(&corpus_dir, &precision, write);
    }
    if nightly_failed || any_failed(&counts) || precision_failed {
        ExitCode::from(EXIT_MISMATCH)
    } else {
        ExitCode::SUCCESS
    }
}

/// Print separate release and regression summaries and check each population's floors.
fn print_precision(corpus_dir: &Path, precision: &[PrecisionReport]) -> Result<bool, String> {
    let membership = memberships(corpus_dir)?;
    let (policy, private) = partition_precision(precision, &membership);
    println!("OSS dev release policy");
    print!("{}", render_precision(&policy, RULE_FLOORS));
    println!("\nPrivate regressions (outside release policy)");
    print!("{}", render_precision(&private, RULE_FLOORS));
    Ok(section_failed(&policy, RULE_FLOORS) || section_failed(&private, RULE_FLOORS))
}

/// The measured entries as `name@short-commit`, each at its clone's `HEAD`: the status
/// file's `entries`.
fn measured_entries(corpus_dir: &Path, precision: &[PrecisionReport]) -> Vec<String> {
    precision
        .iter()
        .filter(|report| report.not_measured.is_none() && !report.rules.is_empty())
        .filter_map(|report| {
            let commit = git_head(&corpus_dir.join(&report.name).join("repo")).ok()?;
            Some(format!(
                "{}@{}",
                report.name,
                &commit[..8.min(commit.len())]
            ))
        })
        .collect()
}

/// Each entry's M5 rows, for the entries with an LSP plan and a clone. Every entry's session
/// runs first, so no `explain` round trip loads the box while a save is timed; then each
/// entry's round trip. The committed status file is compared with this run's verdicts once,
/// and the difference is every entry's (it is the table each server embeds).
fn m5_rows(
    spirolo: &Path,
    corpus_dir: &Path,
    counts: &[EntryReport],
    precision: &mut [PrecisionReport],
) {
    let committed = std::fs::read_to_string(corpus_dir.join("rule-status.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .unwrap_or(Value::Null);
    let membership = match memberships(corpus_dir) {
        Ok(membership) => membership,
        Err(error) => {
            eprintln!("spirolo-bench: corpus selection: {error}");
            return;
        }
    };
    let (run, _) = policy_tables(
        precision,
        &membership,
        RULE_FLOORS,
        "",
        &measured_entries(corpus_dir, precision),
    );
    let mismatches = status_mismatches(&committed, &run);

    let staged = |dir: &Path| {
        let repo = dir.join("repo");
        let commit = git_head(&repo)?;
        spirolo_bench::stage_inputs_in_place(dir, &commit, &repo)
            .map_err(|error| format!("stage inputs: {error}"))
    };
    let mut sessions = Vec::new();
    for (at, report) in precision.iter().enumerate() {
        let dir = corpus_dir.join(&report.name);
        if !dir.join("repo").is_dir() || report.not_measured.is_some() {
            continue;
        }
        let Some(plan) = m5_run::plan(&dir) else {
            continue;
        };
        let session = plan.and_then(|plan| {
            let inputs = staged(&dir)?;
            let session = m5_run::session(spirolo, corpus_dir, &report.name, &plan)?;
            drop(inputs);
            Ok((plan, session))
        });
        sessions.push((at, session));
    }
    for (at, session) in sessions {
        let report = &mut precision[at];
        let dir = corpus_dir.join(&report.name);
        let list = counts
            .iter()
            .find(|entry| entry.name == report.name)
            .and_then(|entry| entry.timing);
        report.m5 = Some(session.and_then(|(plan, (summary, save))| {
            let inputs = staged(&dir)?;
            let explain = m5_run::explain(spirolo, &plan, &committed)?;
            drop(inputs);
            Ok(M5Rows {
                summary,
                explain,
                status: mismatches.clone(),
                budget: M5Budget { save, list },
            })
        }));
    }
}

/// Print the rule-status file for this run's `precision`, or write it to
/// `corpus/rule-status.json`. The spirolo commit is this checkout's `HEAD`; the entries are the
/// ones measured, each at its clone's `HEAD`.
fn write_status(
    corpus_dir: &Path,
    precision: &[spirolo_bench::PrecisionReport],
    write: bool,
) -> ExitCode {
    let head = |repo: &Path| {
        Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["rev-parse", "HEAD"])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
    };
    let Some(spirolo_commit) = head(Path::new(".")) else {
        eprintln!("spirolo-bench: cannot read this checkout's HEAD");
        return ExitCode::from(EXIT_ERROR);
    };
    let entries = measured_entries(corpus_dir, precision);
    let membership = match memberships(corpus_dir) {
        Ok(membership) => membership,
        Err(error) => {
            eprintln!("spirolo-bench: corpus selection: {error}");
            return ExitCode::from(EXIT_ERROR);
        }
    };
    let (file, private) = policy_tables(
        precision,
        &membership,
        RULE_FLOORS,
        &spirolo_commit,
        &entries,
    );
    let text = format!(
        "{}\n",
        serde_json::to_string_pretty(&file).expect("a JSON value serialises")
    );
    if !write {
        print!("{text}");
        return ExitCode::SUCCESS;
    }
    let private_path = corpus_dir.join("private-rule-status.json");
    let private_text = format!(
        "{}\n",
        serde_json::to_string_pretty(&private).expect("a JSON value serialises")
    );
    if let Err(error) = std::fs::write(&private_path, private_text) {
        eprintln!(
            "spirolo-bench: cannot write {}: {error}",
            private_path.display()
        );
        return ExitCode::from(EXIT_ERROR);
    }
    let path = corpus_dir.join("rule-status.json");
    match std::fs::write(&path, text) {
        Ok(()) => {
            eprintln!("spirolo-bench: wrote {}", path.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("spirolo-bench: cannot write {}: {error}", path.display());
            ExitCode::from(EXIT_ERROR)
        }
    }
}

/// Run `spirolo check --all --format json --config <config> <repo>` and parse its stdout.
///
/// A missing `spirolorc.jsonc` is run without `--config` (the entry has no per-entry config),
/// so the section still measures the default rules. A non-zero exit whose stdout is still
/// valid JSON (findings were produced, the error-severity exit is expected) is accepted; a
/// run that produced no JSON at all is the error.
fn run_spirolo_check(spirolo: &Path, repo: &Path, config: &Path) -> Result<Value, String> {
    run_spirolo_check_with(spirolo, repo, config, &[])
}

/// [`run_spirolo_check`] with `extra` arguments before the root, such as `--rule`.
fn run_spirolo_check_with(
    spirolo: &Path,
    repo: &Path,
    config: &Path,
    extra: &[&OsStr],
) -> Result<Value, String> {
    let mut command = Command::new(spirolo);
    command.args(["check", "--all", "--format", "json"]);
    if config.is_file() {
        command.arg("--config").arg(config);
    }
    command.args(extra).arg(repo);
    let output = command
        .output()
        .map_err(|error| format!("cannot run {}: {error}", spirolo.display()))?;

    serde_json::from_slice(&output.stdout).map_err(|error| {
        let stderr = String::from_utf8_lossy(&output.stderr);
        format!(
            "stdout was not JSON: {error}: {}",
            stderr.trim().lines().next().unwrap_or("no stderr")
        )
    })
}

/// Whether the rule's finding is present at `commit`: check the commit out in a throwaway
/// git worktree of the clone, run `spirolo check` there, and look for a finding of the pair's
/// rule. The worktree is removed afterwards, so the clone is left as it was found.
///
/// The worktree lives *beside* the clone, at `corpus/<entry>/.spirolo-bench-replay` (the
/// directory `truth.json` sits in), never inside it: `spirolo-model`'s walk does not skip
/// that name, so a worktree nested under `repo/` would be walked by the clone's own `check`
/// while a replay runs and its files counted twice. `.gitignore` covers the sibling path,
/// so a killed run leaves nothing to commit.
///
/// A worktree that will not create, or a `check` that produces no document, is an error the
/// caller records on the pair, which fails it.
fn replay_finding(
    spirolo: &Path,
    repo: &Path,
    pair: &ReplayPair,
    commit: &str,
) -> Result<bool, String> {
    // The same predicate recall applies at the pin (`ReportedFinding::matches`): a decided
    // finding of the pair's rule, in the pair's file when it names one, whose identity carries
    // one of the pair's invariants when it names any.
    check_at_commit(spirolo, repo, commit).map(|findings| {
        findings
            .iter()
            .any(|finding| finding.matches(&pair.rule, &pair.file, &pair.invariants, None))
    })
}

/// The run's findings at `commit`: check the commit out in a throwaway git worktree of the
/// clone beside it, stage the commit's `inputs/`, run `spirolo check` there, and reduce the
/// document to [`ReportedFinding`]s. The worktree is removed afterwards, so the clone is left
/// as it was found.
///
/// This backs both the replay present-check (which asks whether one pair matches) and the
/// incident-ref recall (which measures three positives against the whole list). The worktree
/// lives *beside* the clone, at `corpus/<entry>/.spirolo-bench-replay` (the directory
/// `truth.json` sits in), never inside it: `spirolo-model`'s walk does not skip that name, so a
/// worktree nested under `repo/` would be walked by the clone's own `check` while a replay
/// runs and its files counted twice. `.gitignore` covers the sibling path, so a killed run
/// leaves nothing to commit. A worktree that will not create, or a `check` that produces no
/// document, is an error the caller records.
fn check_at_commit(
    spirolo: &Path,
    repo: &Path,
    commit: &str,
) -> Result<Vec<ReportedFinding>, String> {
    check_document_at_commit(spirolo, repo, commit).map(|json| reported_findings(&json))
}

/// Run `spirolo check` on the clone itself, at its current `HEAD`, with that commit's `inputs/`
/// staged in place, and return the parsed document. This is the main precision check. It
/// reads the clone where the counts section reads it, on the substrate `truth.json` records
/// (the installed clone at its lockfile, #130): a bare worktree has no `node_modules`, so the
/// resolver-backed rules (`unused-mock`, `orphan-test`) would report another population than
/// the one adjudicated. The committed inputs (mutation reports) are staged so a
/// report-reading rule (`redundant-test`) sees them; only files the clone lacks are added, and
/// they are removed when the check returns. A killed run can leave them behind as untracked
/// copies of committed inputs, which the next run finds identical and leaves alone.
fn staged_pin_check(spirolo: &Path, repo: &Path, config: &Path) -> Result<Value, String> {
    let commit = git_head(repo)?;
    let entry_dir = repo
        .parent()
        .ok_or_else(|| "clone path has no parent directory".to_owned())?;
    let staged = spirolo_bench::stage_inputs_in_place(entry_dir, &commit, repo)
        .map_err(|error| format!("stage inputs: {error}"))?;
    let document = run_spirolo_check(spirolo, repo, config);
    drop(staged);
    document
}

/// The clone's `HEAD` commit.
fn git_head(repo: &Path) -> Result<String, String> {
    let head = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|error| format!("git rev-parse HEAD: {error}"))?;
    if !head.status.success() {
        return Err(format!(
            "git rev-parse HEAD: {}",
            String::from_utf8_lossy(&head.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&head.stdout).trim().to_owned())
}

/// An entry's M4b rows on its clone, `None` when it records no impact oracle. A clone off
/// the substrate its truth records (`not_measured`) is not the population the oracle was
/// generated on, so its rows are not measured, and say why.
fn impact_rows(
    spirolo: &Path,
    dir: &Path,
    repo: &Path,
    not_measured: Option<&str>,
) -> Option<Result<spirolo_bench::ImpactRows, String>> {
    if !dir.join("impact").join("truth.json").is_file() {
        return None;
    }
    if let Some(why) = not_measured {
        return Some(Err(why.to_owned()));
    }
    let head = match git_head(repo) {
        Ok(head) => head,
        Err(error) => return Some(Err(error)),
    };
    let config = dir.join("spirolorc.jsonc");
    let impact = |file: &str| run_impact(spirolo, repo, file);
    let hotspots = |churn: &Path| {
        let args = [
            OsStr::new("--rule"),
            OsStr::new("untested-hotspot"),
            OsStr::new("--churn"),
            churn.as_os_str(),
        ];
        run_spirolo_check_with(spirolo, repo, &config, &args).map(|json| reported_findings(&json))
    };
    let budget = |file: &str| time_impact(spirolo, repo, file);
    score_entry_impact(
        dir,
        &head,
        &ImpactEntryRunners {
            impact: &impact,
            hotspots: &hotspots,
            budget: &budget,
        },
    )
}

/// `spirolo impact --root <repo> --format json <file>`, parsed.
fn run_impact(spirolo: &Path, repo: &Path, file: &str) -> Result<Value, String> {
    let stdout = impact_once(spirolo, repo, file)?;
    serde_json::from_slice(&stdout).map_err(|error| format!("stdout was not JSON: {error}"))
}

/// One `spirolo impact --root <repo> --format json <file>`, its stdout on success.
fn impact_once(spirolo: &Path, repo: &Path, file: &str) -> Result<Vec<u8>, String> {
    let output = Command::new(spirolo)
        .args(["impact", "--format", "json", "--root"])
        .arg(repo)
        .arg(file)
        .output()
        .map_err(|error| format!("cannot run {}: {error}", spirolo.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "exited {}: {}",
            output.status,
            stderr.trim().lines().next().unwrap_or("no stderr")
        ));
    }
    Ok(output.stdout)
}

/// The budget row's timings: `impact <file>` and then `list`, each over its own sample of
/// runs after one warm-up and an idle-box check.
fn time_impact(spirolo: &Path, repo: &Path, file: &str) -> Result<ImpactBudget, String> {
    let timed = |run: &dyn Fn() -> Result<Vec<u8>, String>| -> Result<Wallclock, String> {
        run()?;
        let busy = box_busy();
        let mut samples = Vec::new();
        for _ in 0..sample_count() {
            let started = Instant::now();
            run()?;
            samples.push(started.elapsed());
        }
        Ok(Wallclock { samples, busy })
    };
    Ok(ImpactBudget {
        file: file.to_owned(),
        impact: timed(&|| impact_once(spirolo, repo, file))?,
        list: timed(&|| list_once(spirolo, repo))?,
    })
}

/// The parsed `spirolo check` document at `commit`, run in a throwaway staged worktree of the
/// clone, for replay pairs and the incident ref (commits the clone is not checked out at).
fn check_document_at_commit(spirolo: &Path, repo: &Path, commit: &str) -> Result<Value, String> {
    // An absolute path: `git -C <repo> worktree add <path>` reads `<path>` relative to the
    // `-C` directory, so a relative path would nest the worktree under the clone. `spirolo
    // check <path>` runs from the bench's own cwd, so the two must agree on one absolute
    // path. `canonicalize` needs the clone to exist, which it does (the caller checked).
    let repo = repo
        .canonicalize()
        .map_err(|error| format!("resolve clone path: {error}"))?;
    let entry_dir = repo
        .parent()
        .ok_or_else(|| "clone path has no parent directory".to_owned())?;
    let worktree = entry_dir.join(".spirolo-bench-replay");
    // A residual worktree from a killed run would block the add; prune first.
    let _ = Command::new("git")
        .arg("-C")
        .arg(&repo)
        .args(["worktree", "prune"])
        .output();
    let _ = std::fs::remove_dir_all(&worktree);

    // A clone that predates `commit` fetches it from its origin once, and says so (#202).
    if let Some(fetched) = spirolo_bench::ensure_commit(&repo, commit)? {
        eprintln!("spirolo-bench: {fetched}");
    }

    let added = Command::new("git")
        .arg("-C")
        .arg(&repo)
        .args(["worktree", "add", "--detach", "--force"])
        .arg(&worktree)
        .arg(commit)
        .output()
        .map_err(|error| format!("git worktree add: {error}"))?;
    if !added.status.success() {
        return Err(format!(
            "git worktree add {commit}: {}",
            String::from_utf8_lossy(&added.stderr).trim()
        ));
    }

    // Stage the entry's committed inputs for this commit into the worktree at their in-tree
    // paths, so a rule that reads a mutation report finds it where the project would keep it.
    // A commit with no `inputs/<commit>/` directory stages nothing and runs as before. A
    // staging failure leaves the worktree to be removed below before it returns the error.
    if let Err(error) = spirolo_bench::stage_inputs(entry_dir, commit, &worktree) {
        remove_worktree(&repo, &worktree);
        return Err(format!("stage inputs: {error}"));
    }

    // Run with the entry's own config (beside the clone at `<entry>/spirolorc.jsonc`), so a rule
    // that reads config keys — `handler-drift`'s `urlSources` and `env` — can decide the
    // finding rather than report "could not decide" for want of them. A missing config file is
    // read as no config, which the config-less rules run under unchanged.
    let config = entry_dir.join("spirolorc.jsonc");
    let document = run_spirolo_check(spirolo, &worktree, &config);

    // Leave the clone clean whatever the check did.
    remove_worktree(&repo, &worktree);
    document
}

/// `spirolo` over an entry's committed runs: a throwaway worktree of the clone beside it
/// (`corpus/<entry>/.spirolo-bench-ledger`), the runs staged into it and the worktree checked
/// out at their tree hash, then `flake ingest`, `check`, and `flake quarantine` twice. The
/// worktree is removed afterwards, so the clone is left as it was found.
fn run_ledger(spirolo: &Path, runs: &Path, repo: &Path) -> Result<LedgerRun, String> {
    let repo = repo
        .canonicalize()
        .map_err(|error| format!("resolve clone path: {error}"))?;
    let entry_dir = repo
        .parent()
        .ok_or_else(|| "clone path has no parent directory".to_owned())?;
    let worktree = entry_dir.join(".spirolo-bench-ledger");
    let _ = Command::new("git")
        .arg("-C")
        .arg(&repo)
        .args(["worktree", "prune"])
        .output();
    let _ = std::fs::remove_dir_all(&worktree);
    git(
        &repo,
        &[
            "worktree",
            "add",
            "--detach",
            "--force",
            &worktree.to_string_lossy(),
        ],
    )?;
    let result = ledger_in(spirolo, runs, &worktree, entry_dir);
    remove_worktree(&repo, &worktree);
    result
}

/// The ledger steps inside the worktree [`run_ledger`] made.
fn ledger_in(
    spirolo: &Path,
    runs: &Path,
    worktree: &Path,
    entry_dir: &Path,
) -> Result<LedgerRun, String> {
    let staged = stage_runs(runs, worktree).map_err(|error| format!("stage runs: {error}"))?;
    // The staged runs are untracked, so checking the runs' tree out keeps them.
    git(
        worktree,
        &["checkout", "--detach", "--force", &staged.tree_hash],
    )?;
    let ingest = Command::new(spirolo)
        .args(["flake", "ingest", &staged.glob, "--root"])
        .arg(worktree)
        .output()
        .map_err(|error| format!("cannot run {}: {error}", spirolo.display()))?;
    if !ingest.status.success() {
        return Err(format!(
            "flake ingest: {}",
            String::from_utf8_lossy(&ingest.stderr).trim()
        ));
    }
    let document = run_spirolo_check(spirolo, worktree, &entry_dir.join("spirolorc.jsonc"))?;
    let quarantine = || -> Result<Vec<u8>, String> {
        let run = Command::new(spirolo)
            .args(["flake", "quarantine"])
            .arg(worktree)
            .output()
            .map_err(|error| format!("cannot run {}: {error}", spirolo.display()))?;
        if !run.status.success() {
            return Err(format!(
                "flake quarantine: {}",
                String::from_utf8_lossy(&run.stderr).trim()
            ));
        }
        std::fs::read(worktree.join(".spirolo/quarantine.json"))
            .map_err(|error| format!("read .spirolo/quarantine.json: {error}"))
    };
    let first = quarantine()?;
    let second = quarantine()?;
    let entries = serde_json::from_slice::<Value>(&first)
        .map_err(|error| format!(".spirolo/quarantine.json: {error}"))?["entries"]
        .as_array()
        .map_or(0, Vec::len);
    Ok(LedgerRun {
        findings: reported_findings(&document),
        quarantine: QuarantineRoundTrip {
            entries,
            identical: first == second,
        },
    })
}

/// Run `git -C <dir> <args>`, returning its stderr as the error when it fails.
fn git(dir: &Path, args: &[&str]) -> Result<(), String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|error| format!("git {}: {error}", args.join(" ")))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// Remove the throwaway replay worktree, best-effort, so the clone is left as it was found.
fn remove_worktree(repo: &Path, worktree: &Path) {
    let _ = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["worktree", "remove", "--force"])
        .arg(worktree)
        .output();
    let _ = std::fs::remove_dir_all(worktree);
}

/// Timed `spirolo list` runs per entry when `$SPIROLO_BENCH_SAMPLES` does not say otherwise.
const DEFAULT_SAMPLES: usize = 5;

/// Run `spirolo list --format json <repo>` once untimed to warm the page cache, check the box
/// is idle, then take `$SPIROLO_BENCH_SAMPLES` (default 5) timed runs whose median the budget
/// reads: one wall-clock on a shared box is not reproducible. The document is the
/// warm-up's.
fn run_spirolo_list(spirolo: &Path, repo: &Path) -> Result<Run, String> {
    let output = list_once(spirolo, repo)?;
    let json =
        serde_json::from_slice(&output).map_err(|error| format!("stdout was not JSON: {error}"))?;

    let busy = box_busy();
    let mut samples = Vec::new();
    for _ in 0..sample_count() {
        let started = Instant::now();
        list_once(spirolo, repo)?;
        samples.push(started.elapsed());
    }
    Ok(Run {
        json,
        samples,
        busy,
    })
}

/// The fraction of all CPUs busy over one second, read from `/proc/stat` just before the
/// timed runs (the idle-box check); `None` where `/proc/stat` is unreadable (not Linux).
fn box_busy() -> Option<f64> {
    let before = cpu_ticks()?;
    std::thread::sleep(std::time::Duration::from_secs(1));
    let after = cpu_ticks()?;
    let total = after.0.checked_sub(before.0)?;
    let idle = after.1.checked_sub(before.1)?;
    if total == 0 {
        return None;
    }
    #[allow(clippy::cast_precision_loss)]
    let busy = 1.0 - idle as f64 / total as f64;
    Some(busy)
}

/// All CPUs' `(total, idle + iowait)` jiffies from `/proc/stat`'s aggregate `cpu` line.
fn cpu_ticks() -> Option<(u64, u64)> {
    let stat = std::fs::read_to_string("/proc/stat").ok()?;
    let line = stat.lines().find(|line| line.starts_with("cpu "))?;
    let ticks: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        // user … steal; guest and guest_nice are already inside user and nice.
        .take(8)
        .filter_map(|field| field.parse().ok())
        .collect();
    let idle = ticks.get(3)? + ticks.get(4).copied().unwrap_or(0);
    Some((ticks.iter().sum(), idle))
}

/// How many timed runs to take: `$SPIROLO_BENCH_SAMPLES` when it is a positive integer.
fn sample_count() -> usize {
    std::env::var("SPIROLO_BENCH_SAMPLES")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|&count| count > 0)
        .unwrap_or(DEFAULT_SAMPLES)
}

/// Prints `{counts, substrate}` for one clone: [`spirolo_bench::tally`] of its `spirolo list`.
fn print_tally(repo: &Path) -> ExitCode {
    let listed = list_once(&spirolo_bin(), repo)
        .and_then(|stdout| serde_json::from_slice::<Value>(&stdout).map_err(|e| e.to_string()));
    match listed {
        Ok(doc) => {
            let tally = serde_json::json!({
                "counts": spirolo_bench::tally(&doc),
                "substrate": spirolo_bench::substrate_of(repo),
            });
            println!("{tally}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("spirolo-bench tally: {}: {error}", repo.display());
            ExitCode::from(EXIT_ERROR)
        }
    }
}

/// One `spirolo list --format json <repo>`, its stdout on success.
fn list_once(spirolo: &Path, repo: &Path) -> Result<Vec<u8>, String> {
    let output = Command::new(spirolo)
        .args(["list", "--format", "json"])
        .arg(repo)
        .output()
        .map_err(|error| format!("cannot run {}: {error}", spirolo.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "exited {}: {}",
            output.status,
            stderr.trim().lines().next().unwrap_or("no stderr")
        ));
    }
    Ok(output.stdout)
}

/// The corpus directory: `$SPIROLO_CORPUS_DIR`, else `corpus` beside the working directory.
fn corpus_dir() -> PathBuf {
    std::env::var_os("SPIROLO_CORPUS_DIR").map_or_else(|| PathBuf::from("corpus"), PathBuf::from)
}

/// The `spirolo` executable to drive: `$SPIROLO_BIN`, else the sibling of this binary that
/// `cargo build -p spirolo -p spirolo-bench` leaves in the same target directory, else the
/// bare name for `PATH` lookup.
fn spirolo_bin() -> PathBuf {
    if let Some(bin) = std::env::var_os("SPIROLO_BIN") {
        return PathBuf::from(bin);
    }
    if let Ok(current) = std::env::current_exe() {
        if let Some(dir) = current.parent() {
            let sibling = dir.join("spirolo");
            if sibling.is_file() {
                return sibling;
            }
        }
    }
    PathBuf::from("spirolo")
}

/// Score existing measurements without running Spirolo on any corpus entry.
fn nightly_row(ledger: &Path, manifest: &Path) -> bool {
    let manifest = match std::fs::read_to_string(manifest) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("M6 nightly: manifest: {error}");
            return true;
        }
    };
    let expected = spirolo_bench::nightly_entries(&manifest);
    let today = Command::new("date")
        .args(["-u", "+%F"])
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .unwrap_or_default();
    let ledger = std::fs::read_to_string(ledger).unwrap_or_default();
    let score = spirolo_bench::score_nightly(&ledger, &expected, today.trim());
    print!("{}", spirolo_bench::render_nightly(&score, expected.len()));
    score.failed()
}

/// Read existing nightly output, without invoking Spirolo on a corpus entry.
fn nightly_command(args: &[String]) -> Option<ExitCode> {
    if let [command, ledger, manifest] = args {
        if command == "nightly" {
            let score = nightly_row(Path::new(ledger), Path::new(manifest));
            return Some(if score {
                ExitCode::from(EXIT_MISMATCH)
            } else {
                ExitCode::SUCCESS
            });
        }
    }
    if let [command, list, truth] = args {
        if command == "nightly-counts" {
            let result = (|| -> Result<serde_json::Value, String> {
                let text = std::fs::read_to_string(list).map_err(|e| e.to_string())?;
                let doc = serde_json::from_str(&text).map_err(|e| e.to_string())?;
                let truth = spirolo_bench::Truth::read(Path::new(truth))?;
                Ok(spirolo_bench::nightly_counts(&doc, &truth))
            })();
            return Some(match result {
                Ok(doc) => {
                    println!("{doc}");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("spirolo-bench: {error}");
                    ExitCode::from(EXIT_ERROR)
                }
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use spirolo_bench::RULE_FLOORS;

    #[test]
    fn the_precision_section_lists_redundant_test_beside_duplicate_test() {
        let duplicate = RULE_FLOORS
            .iter()
            .position(|floor| floor.rule == "duplicate-test")
            .expect("duplicate-test is listed");
        assert_eq!(
            RULE_FLOORS.get(duplicate + 1).map(|floor| floor.rule),
            Some("redundant-test"),
            "redundant-test must print its own precision and recall row, next to duplicate-test"
        );
    }

    #[test]
    fn the_precision_section_measures_flake_candidate_as_an_m4_floor_carried_to_m6() {
        let floor = RULE_FLOORS
            .iter()
            .find(|floor| floor.rule == "flake-candidate")
            .expect("flake-candidate prints a precision row");
        assert_eq!(floor.milestone, "M4");
        // M4 concluded not passed and carried the floor to M6 (#184): printed, not gating.
        assert_eq!(floor.carried.map(|carried| carried.to), Some("M6"));
    }
}
