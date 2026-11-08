//! The M5 rows' IO for the `spirolo-bench` binary (PRD 0006, Bench): an entry with
//! `corpus/<entry>/lsp/bench.json` gets an `spirolo lsp` session through `corpus/lsp/m5.mjs`,
//! its `check` timings, and an `explain` round trip over every finding id. The verdicts are the
//! library's ([`spirolo_bench::M5Rows`]).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use serde_json::Value;
use spirolo_bench::{
    explain_round_trip, ExplainAnswer, ExplainScore, M5Summary, SaveBudget, LOAD_CEILING,
};

/// How long a timing waits for the load to fall under [`LOAD_CEILING`] before it runs anyway
/// (and the row reads the load it ran at).
const IDLE_WAIT: Duration = Duration::from_secs(300);

/// `explain` processes run at once: the box is shared, and each builds the whole model.
const EXPLAIN_WORKERS: usize = 4;

/// An entry's M5 plan, from `corpus/<entry>/lsp/bench.json`.
pub struct Plan {
    /// The entry directory.
    dir: PathBuf,
    /// The session's scratch copy of the clone, or `None` to run in the clone itself.
    scratch: Option<PathBuf>,
    /// Hover at every expected line.
    hover: bool,
    /// The pre-registered edits, when the entry has them.
    edits: Option<PathBuf>,
    /// The file the timed saves write.
    latency_path: String,
    /// How many timed saves.
    latency_count: u64,
    /// A fixed save budget, or `check`'s median over `check_runs` plus a margin.
    budget: BudgetPlan,
}

/// The save budget as the plan states it.
enum BudgetPlan {
    /// A fixed budget in milliseconds.
    Fixed(u64),
    /// `check`'s median over `runs` timed runs plus `over_ms`.
    OverCheck { over_ms: u64, runs: usize },
}

impl Plan {
    /// The clone.
    fn repo(&self) -> PathBuf {
        self.dir.join("repo")
    }

    /// The entry's config.
    fn config(&self) -> PathBuf {
        self.dir.join("spirolorc.jsonc")
    }
}

/// The entry's plan, `None` when it has no `lsp/bench.json`.
pub fn plan(dir: &Path) -> Option<Result<Plan, String>> {
    let path = dir.join("lsp").join("bench.json");
    if !path.is_file() {
        return None;
    }
    Some(read_plan(dir, &path))
}

fn read_plan(dir: &Path, path: &Path) -> Result<Plan, String> {
    let text =
        std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let json: Value =
        serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
    let budget = &json["budget"];
    let budget = if let Some(ms) = budget["save_ms"].as_u64() {
        BudgetPlan::Fixed(ms)
    } else if let Some(over_ms) = budget["over_check_ms"].as_u64() {
        BudgetPlan::OverCheck {
            over_ms,
            runs: budget["check_runs"]
                .as_u64()
                .and_then(|runs| usize::try_from(runs).ok())
                .unwrap_or(5),
        }
    } else {
        return Err(format!(
            "{}: budget needs save_ms or over_check_ms",
            path.display()
        ));
    };
    Ok(Plan {
        dir: dir.to_path_buf(),
        scratch: json["scratch"].as_str().map(PathBuf::from),
        hover: json["hover"].as_bool().unwrap_or(false),
        edits: json["edits"]
            .as_str()
            .map(|edits| dir.join("lsp").join(edits)),
        latency_path: json["latency_path"]
            .as_str()
            .unwrap_or(".spirolorc.jsonc")
            .to_owned(),
        latency_count: json["latency_count"].as_u64().unwrap_or(10),
        budget,
    })
}

/// The one-minute load average.
fn load() -> Option<f64> {
    std::fs::read_to_string("/proc/loadavg")
        .ok()?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

/// Wait up to [`IDLE_WAIT`] for the load to fall under [`LOAD_CEILING`].
fn wait_idle() {
    let started = Instant::now();
    while load().is_some_and(|load| load >= LOAD_CEILING) && started.elapsed() < IDLE_WAIT {
        std::thread::sleep(Duration::from_secs(5));
    }
}

/// One `spirolo check --all --format json --config <config> <repo>`, its stdout.
fn check_once(spirolo: &Path, plan: &Plan) -> Result<Vec<u8>, String> {
    let output = Command::new(spirolo)
        .args(["check", "--all", "--format", "json", "--config"])
        .arg(plan.config())
        .arg(plan.repo())
        .output()
        .map_err(|error| format!("cannot run {}: {error}", spirolo.display()))?;
    Ok(output.stdout)
}

/// The median of `samples` in whole milliseconds.
fn median_ms(mut samples: Vec<Duration>) -> u64 {
    samples.sort();
    let n = samples.len();
    let middle = if n % 2 == 1 {
        samples[n / 2]
    } else {
        (samples[n / 2 - 1] + samples[n / 2]) / 2
    };
    u64::try_from(middle.as_millis()).unwrap_or(u64::MAX)
}

/// The entry's session: `check`'s timed runs when the budget reads them, then
/// `corpus/lsp/m5.mjs` on the tree, each after waiting for an idle box. The clone's inputs
/// must be staged. Returns the summary and the save budget.
pub fn session(
    spirolo: &Path,
    corpus_dir: &Path,
    name: &str,
    plan: &Plan,
) -> Result<(M5Summary, SaveBudget), String> {
    let save = match plan.budget {
        BudgetPlan::Fixed(ms) => SaveBudget::Fixed { ms },
        BudgetPlan::OverCheck { over_ms, runs } => {
            check_once(spirolo, plan)?;
            wait_idle();
            let mut samples = Vec::new();
            for _ in 0..runs.max(1) {
                let started = Instant::now();
                check_once(spirolo, plan)?;
                samples.push(started.elapsed());
            }
            SaveBudget::OverCheck {
                check_ms: median_ms(samples),
                over_ms,
            }
        }
    };

    let out = PathBuf::from("target").join("m5");
    std::fs::create_dir_all(&out).map_err(|error| format!("{}: {error}", out.display()))?;
    let summary = out.join(format!("{name}-summary.json"));
    let absolute = |path: &Path| std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let mut command = Command::new("node");
    command
        .arg(corpus_dir.join("lsp").join("m5.mjs"))
        .arg("--binary")
        .arg(absolute(spirolo))
        .arg("--status")
        .arg(corpus_dir.join("rule-status.json"))
        .arg("--repo")
        .arg(absolute(&plan.repo()))
        .arg("--config")
        .arg(plan.config())
        .arg("--out")
        .arg(&summary)
        .arg("--transcript")
        .arg(out.join(format!("{name}-transcript.json")))
        .arg("--latency-path")
        .arg(&plan.latency_path)
        .arg("--latency-count")
        .arg(plan.latency_count.to_string());
    if let Some(scratch) = &plan.scratch {
        command.arg("--scratch").arg(scratch);
    }
    if plan.hover {
        command.arg("--hover");
    }
    if let Some(edits) = &plan.edits {
        command.arg("--edits").arg(edits);
    }
    wait_idle();
    let output = command
        .output()
        .map_err(|error| format!("cannot run node: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "m5.mjs exited {}: {}",
            output.status,
            stderr.trim().lines().last().unwrap_or("no stderr")
        ));
    }
    let text = std::fs::read_to_string(&summary)
        .map_err(|error| format!("{}: {error}", summary.display()))?;
    let summary: M5Summary =
        serde_json::from_str(&text).map_err(|error| format!("{}: {error}", summary.display()))?;
    Ok((summary, save))
}

/// `spirolo explain` on every finding id of a fresh `check` of the clone (the entry config, its
/// inputs staged) and on the unknown id, [`EXPLAIN_WORKERS`] at a time.
pub fn explain(spirolo: &Path, plan: &Plan, status: &Value) -> Result<ExplainScore, String> {
    let check: Value = serde_json::from_slice(&check_once(spirolo, plan)?)
        .map_err(|error| format!("check stdout was not JSON: {error}"))?;
    let (config, repo) = (plan.config(), plan.repo());
    let run = |query: &str| {
        let output = Command::new(spirolo)
            .args(["explain", query, "--format", "json", "--config"])
            .arg(&config)
            .arg(&repo)
            .output()
            .map_err(|error| format!("cannot run {}: {error}", spirolo.display()))?;
        Ok(ExplainAnswer {
            code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    };
    Ok(explain_round_trip(&check, status, &run, EXPLAIN_WORKERS))
}
