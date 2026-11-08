//! Per-framework counts derived from one `spirolo list --format json` document.
//!
//! The harness never re-implements the model; it reads the same JSON any consumer gets
//! and tallies it the way the runner's own listing does — `vitest list --json` and
//! `playwright test --list`, which `truth.json` was measured with:
//!
//! - A *test* is a `test_cases` row of kind `test`; suites, hooks, steps and benches are
//!   not tests. A framework's *files* are the distinct files its counted tests live in.
//! - A parameterised row counts once per data case: its own `.each` table, the loops it
//!   sits in, and the tables and loops of every suite enclosing it multiply together. A
//!   table the model could not count multiplies by one.
//! - `vitest list` lists a test only when its mode is `run` or `only`: a `.skip` /
//!   `.todo` row, and every row under a skipped suite, is not counted. (Playwright's
//!   `--list` prints every declared test, skipped or not, so its rows are all counted.)
//!   A conditional
//!   (`.skipIf(c)` / `.runIf(c)`) is decided the way the measurement environment decided
//!   it: `truth.json` was produced in a bare shell, so `process.env.X` is undefined and a
//!   gate whose condition the harness cannot evaluate statically — a constant computed
//!   from credentials, a probe for an optional dependency — is taken as *closed*, the way
//!   it was when the listing ran without any of them.
//! - A package `truth.json` lists under `excluded_packages` was never listed by the
//!   runner, and a file under `excluded_files` has tests the runner generated at load
//!   time; both are left out of the tally so the two sides count the same set.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::truth::RunnerCounts;

/// One framework's tally from an `spirolo list` run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FrameworkCount {
    /// Distinct files that carry at least one counted test of this framework.
    pub files: u64,
    /// Test cases of this framework, expanded per data case and per loop iteration.
    pub tests: u64,
}

/// Tally the `test_cases` table of an `spirolo list --format json` document by framework,
/// leaving out every file under one of the `excluded_packages` paths and every file named
/// in `excluded_files`.
///
/// A row with no `framework` is counted under `unknown`, so a recogniser that forgot to
/// stamp its rows shows up as a discrepancy rather than vanishing.
pub fn by_framework(
    list_json: &Value,
    excluded_packages: &[String],
    excluded_files: &[String],
) -> BTreeMap<String, FrameworkCount> {
    let rows = list_json
        .get("test_cases")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();

    // Suites by file, so a row finds its enclosing suites without scanning every row.
    let mut suites_by_file: BTreeMap<&str, Vec<&Value>> = BTreeMap::new();
    for row in rows {
        if row.get("kind").and_then(Value::as_str) == Some("suite") {
            let file = row.get("file").and_then(Value::as_str).unwrap_or("");
            suites_by_file.entry(file).or_default().push(row);
        }
    }

    let mut tests: BTreeMap<String, u64> = BTreeMap::new();
    let mut files: BTreeMap<String, BTreeSet<&str>> = BTreeMap::new();

    for row in rows {
        if row.get("kind").and_then(Value::as_str) != Some("test") {
            continue;
        }
        let file = row.get("file").and_then(Value::as_str).unwrap_or("");
        if excluded_packages
            .iter()
            .any(|package| file.starts_with(&format!("{package}/")))
            || excluded_files.iter().any(|excluded| excluded == file)
        {
            continue;
        }
        let framework = row
            .get("framework")
            .and_then(Value::as_str)
            .unwrap_or("unknown");

        let suites = enclosing_suites(
            suites_by_file
                .get(file)
                .map(Vec::as_slice)
                .unwrap_or_default(),
            row,
        );
        let lists_by_mode = matches!(framework, "vitest" | "jest");
        if lists_by_mode && (skipped(row) || suites.iter().any(|suite| skipped(suite))) {
            continue;
        }
        let weight = suites.iter().fold(multiplicity(row), |product, suite| {
            product.saturating_mul(multiplicity(suite))
        });

        *tests.entry(framework.to_owned()).or_default() += weight;
        if !file.is_empty() {
            files.entry(framework.to_owned()).or_default().insert(file);
        }
    }

    let mut counts = BTreeMap::new();
    for (framework, test_count) in tests {
        let file_count = files.get(&framework).map_or(0, BTreeSet::len);
        counts.insert(
            framework,
            FrameworkCount {
                files: u64::try_from(file_count).unwrap_or(u64::MAX),
                tests: test_count,
            },
        );
    }
    counts
}

/// The `counts` object a new corpus entry's `truth.json` records.
///
/// It is [`by_framework`] of an `spirolo list --format json` document with no exclusions,
/// as `{framework: {files, tests}}`. `spirolo-bench tally` prints it when a new entry is onboarded.
pub fn tally(list_json: &Value) -> Value {
    let counts = by_framework(list_json, &[], &[])
        .into_iter()
        .map(|(framework, count)| {
            let row = serde_json::json!({ "files": count.files, "tests": count.tests });
            (framework, row)
        })
        .collect();
    Value::Object(counts)
}

/// Tally the document once per runner record, with that record's own exclusions.
///
/// Each record's `excluded_packages` and `excluded_files` are the runner's gaps at its
/// listing, not the `counts` exclusions; only the record's own framework is kept.
///
/// A framework the document has no rows for tallies to zero files and zero tests, so a
/// runner record for a framework the model missed still produces a (mismatching) row.
pub fn by_runner_record(
    list_json: &Value,
    runner: &BTreeMap<String, RunnerCounts>,
) -> BTreeMap<String, FrameworkCount> {
    runner
        .iter()
        .map(|(framework, record)| {
            let tally = by_framework(
                list_json,
                &record.excluded_packages,
                &record.excluded_file_paths(),
            );
            (
                framework.clone(),
                tally.get(framework).copied().unwrap_or_default(),
            )
        })
        .collect()
}

/// The suites among `suites` (all of the row's file) whose span contains `row`'s span.
fn enclosing_suites<'a>(suites: &[&'a Value], row: &Value) -> Vec<&'a Value> {
    let (start, end) = span_of(row);
    suites
        .iter()
        .copied()
        .filter(|candidate| {
            let (suite_start, suite_end) = span_of(candidate);
            suite_start <= start && end <= suite_end
        })
        .collect()
}

/// A row's `(start, end)` byte span.
fn span_of(row: &Value) -> (u64, u64) {
    let span = row.get("span");
    let at = |key: &str| {
        span.and_then(|span| span.get(key))
            .and_then(Value::as_u64)
            .unwrap_or(0)
    };
    (at("start"), at("end"))
}

/// How many times the runner defines a row: its `.each` cases times the cases of every
/// loop it sits in, with an uncounted table taken as one.
fn multiplicity(row: &Value) -> u64 {
    let cases = |table: &Value| table.get("cases").and_then(Value::as_u64).unwrap_or(1);
    let own = row
        .get("each")
        .filter(|each| !each.is_null())
        .map_or(1, cases);
    row.get("loops")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .fold(own, |product, table| product.saturating_mul(cases(table)))
}

/// Whether a row's own modifier keeps it out of the runner's listing.
fn skipped(row: &Value) -> bool {
    let Some(skip) = row.get("skip").filter(|skip| !skip.is_null()) else {
        return false;
    };
    let condition = skip.get("condition").and_then(Value::as_str).unwrap_or("");
    match skip.get("kind").and_then(Value::as_str) {
        Some("skip" | "todo") => true,
        // A gate is open only when its condition provably evaluates the open way in the
        // measurement environment; an undecidable gate is closed.
        Some("skipIf") => bare_environment_truth(condition) != Some(false),
        Some("runIf") => bare_environment_truth(condition) != Some(true),
        _ => false,
    }
}

/// The truth value of a skip condition in a bare shell, or `None` when it depends on
/// something the harness cannot see.
///
/// Decidable: boolean literals, `process.env.NAME` (undefined, so falsy), and any `!`
/// prefix over those. Everything else — an identifier, a call, a comparison — is
/// undecidable here, and [`skipped`] treats such a gate as closed.
fn bare_environment_truth(condition: &str) -> Option<bool> {
    let mut text = condition.trim();
    let mut negations = 0;
    while let Some(rest) = text.strip_prefix('!') {
        negations += 1;
        text = rest.trim();
    }
    let value = match text {
        "true" => true,
        "false" => false,
        _ if is_env_lookup(text) => false,
        _ => return None,
    };
    Some(if negations % 2 == 0 { value } else { !value })
}

/// `process.env.NAME`, and nothing else.
fn is_env_lookup(text: &str) -> bool {
    text.strip_prefix("process.env.").is_some_and(|name| {
        !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_')
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::json;

    use super::{by_framework, by_runner_record};
    use crate::truth::{RunnerCounts, RunnerExcludedFile};

    fn span(start: u64, end: u64) -> serde_json::Value {
        json!({ "start": start, "end": end, "line": 1, "column": 1 })
    }

    #[test]
    fn tests_are_counted_by_framework_and_suites_are_not() {
        let doc = json!({
            "test_cases": [
                { "kind": "suite", "framework": "vitest", "file": "a.test.ts", "span": span(0, 100) },
                { "kind": "test",  "framework": "vitest", "file": "a.test.ts", "span": span(10, 20) },
                { "kind": "test",  "framework": "vitest", "file": "a.test.ts", "span": span(30, 40) },
                { "kind": "hook",  "framework": "vitest", "file": "a.test.ts", "span": span(50, 60) },
                { "kind": "test",  "framework": "playwright", "file": "e2e/home.spec.ts", "span": span(0, 10) }
            ]
        });
        let counts = by_framework(&doc, &[], &[]);
        assert_eq!(counts["vitest"].tests, 2);
        assert_eq!(counts["vitest"].files, 1);
        assert_eq!(counts["playwright"].tests, 1);
        assert_eq!(counts["playwright"].files, 1);
    }

    #[test]
    fn distinct_files_are_counted_once_each() {
        let doc = json!({
            "test_cases": [
                { "kind": "test", "framework": "vitest", "file": "a.test.ts", "span": span(0, 1) },
                { "kind": "test", "framework": "vitest", "file": "a.test.ts", "span": span(2, 3) },
                { "kind": "test", "framework": "vitest", "file": "b.test.ts", "span": span(0, 1) }
            ]
        });
        let counts = by_framework(&doc, &[], &[]);
        assert_eq!(counts["vitest"].tests, 3);
        assert_eq!(counts["vitest"].files, 2);
    }

    #[test]
    fn an_empty_document_tallies_to_nothing() {
        assert!(by_framework(&json!({ "test_cases": [] }), &[], &[]).is_empty());
        assert!(by_framework(&json!({}), &[], &[]).is_empty());
    }

    #[test]
    fn each_tables_and_loops_multiply_through_enclosing_suites() {
        let doc = json!({
            "test_cases": [
                // describe.each with 2 rows, inside a for-of over 3 elements
                { "kind": "suite", "framework": "vitest", "file": "a.test.ts", "span": span(0, 100),
                  "each": { "cases": 2, "params": 1, "source": null },
                  "loops": [{ "cases": 3, "params": null, "source": "xs" }] },
                // it.each with 4 rows under it: 4 * 2 * 3
                { "kind": "test", "framework": "vitest", "file": "a.test.ts", "span": span(10, 20),
                  "each": { "cases": 4, "params": 2, "source": null } },
                // a table the model could not count multiplies by one: 1 * 2 * 3
                { "kind": "test", "framework": "vitest", "file": "a.test.ts", "span": span(30, 40),
                  "each": { "cases": null, "params": null, "source": "ROWS" } }
            ]
        });
        assert_eq!(by_framework(&doc, &[], &[])["vitest"].tests, 24 + 6);
    }

    #[test]
    fn skipped_rows_and_rows_under_a_skipped_suite_are_not_listed() {
        let doc = json!({
            "test_cases": [
                { "kind": "test", "framework": "vitest", "file": "a.test.ts", "span": span(0, 1),
                  "skip": { "kind": "skip", "condition": null } },
                { "kind": "test", "framework": "vitest", "file": "a.test.ts", "span": span(2, 3),
                  "skip": { "kind": "todo", "condition": null } },
                { "kind": "test", "framework": "vitest", "file": "a.test.ts", "span": span(4, 5),
                  "skip": { "kind": "only", "condition": null } },
                { "kind": "suite", "framework": "vitest", "file": "a.test.ts", "span": span(10, 30),
                  "skip": { "kind": "skip", "condition": null } },
                { "kind": "test", "framework": "vitest", "file": "a.test.ts", "span": span(12, 14) },
                { "kind": "test", "framework": "vitest", "file": "b.test.ts", "span": span(0, 1) }
            ]
        });
        let counts = by_framework(&doc, &[], &[]);
        assert_eq!(
            counts["vitest"].tests, 2,
            "only `.only` and the plain test run"
        );
        assert_eq!(counts["vitest"].files, 2);
    }

    #[test]
    fn conditional_gates_are_decided_in_a_bare_environment_or_taken_as_closed() {
        let case = |kind: &str, condition: &str, start: u64| {
            json!({ "kind": "test", "framework": "vitest", "file": "a.test.ts",
                    "span": span(start, start + 1),
                    "skip": { "kind": kind, "condition": condition } })
        };
        let doc = json!({ "test_cases": [
            case("skipIf", "!process.env.PERF", 0),   // env unset → skipped
            case("skipIf", "process.env.PERF", 2),    // env unset → runs
            case("runIf", "process.env.LIVE", 4),     // env unset → skipped
            case("skipIf", "!HAS_CREDS", 6),          // undecidable → closed
            case("runIf", "hasReact", 8),             // undecidable → closed
            case("skipIf", "false", 10),              // runs
        ]});
        assert_eq!(by_framework(&doc, &[], &[])["vitest"].tests, 2);
    }

    #[test]
    fn playwright_lists_skipped_tests_so_they_count() {
        let doc = json!({
            "test_cases": [
                { "kind": "test", "framework": "playwright", "file": "e2e/a.spec.ts", "span": span(0, 1),
                  "skip": { "kind": "skip", "condition": "browserName === 'webkit'" } },
                { "kind": "test", "framework": "playwright", "file": "e2e/a.spec.ts", "span": span(2, 3),
                  "skip": { "kind": "fixme", "condition": null } }
            ]
        });
        assert_eq!(by_framework(&doc, &[], &[])["playwright"].tests, 2);
    }

    #[test]
    fn excluded_packages_are_left_out_of_the_tally() {
        let doc = json!({
            "test_cases": [
                { "kind": "test", "framework": "vitest", "file": "apps/dashboard/a.test.ts", "span": span(0, 1) },
                { "kind": "test", "framework": "vitest", "file": "apps/dashboard-two/a.test.ts", "span": span(0, 1) },
                { "kind": "test", "framework": "vitest", "file": "packages/x/a.test.ts", "span": span(0, 1) }
            ]
        });
        let counts = by_framework(&doc, &["apps/dashboard".to_owned()], &[]);
        assert_eq!(
            counts["vitest"].tests, 2,
            "the prefix is a path, not a string prefix"
        );
        assert_eq!(counts["vitest"].files, 2);
    }

    /// Each runner record tallies with its own exclusions: the vitest record leaves out its
    /// crasher, the playwright record its load-time file, and neither the other's; a record
    /// for a framework the document lacks tallies to zero rather than vanishing.
    #[test]
    fn runner_records_tally_with_their_own_exclusions_only() {
        let doc = json!({
            "test_cases": [
                { "kind": "test", "framework": "vitest", "file": "apps/dashboard/a.test.ts", "span": span(0, 1) },
                { "kind": "test", "framework": "vitest", "file": "packages/x/a.test.ts", "span": span(0, 1) },
                { "kind": "test", "framework": "vitest", "file": "packages/x/b.test.ts", "span": span(0, 1) },
                { "kind": "test", "framework": "playwright", "file": "e2e/smoke.spec.ts", "span": span(0, 1) },
                { "kind": "test", "framework": "playwright", "file": "e2e/home.spec.ts", "span": span(0, 1) }
            ]
        });
        let record = |packages: &[&str], files: &[&str]| RunnerCounts {
            files: 0,
            tests: Some(0),
            excluded_packages: packages.iter().map(|p| (*p).to_owned()).collect(),
            excluded_files: files
                .iter()
                .map(|path| RunnerExcludedFile {
                    path: (*path).to_owned(),
                    listed_tests: 1,
                })
                .collect(),
            tolerance_tests: 0,
            tolerance_files: 0,
        };
        let runner: BTreeMap<String, RunnerCounts> = [
            ("vitest".to_owned(), record(&["apps/dashboard"], &[])),
            ("playwright".to_owned(), record(&[], &["e2e/smoke.spec.ts"])),
            ("jest".to_owned(), record(&[], &[])),
        ]
        .into_iter()
        .collect();

        let tallies = by_runner_record(&doc, &runner);
        assert_eq!((tallies["vitest"].files, tallies["vitest"].tests), (2, 2));
        assert_eq!(
            (tallies["playwright"].files, tallies["playwright"].tests),
            (1, 1)
        );
        assert_eq!((tallies["jest"].files, tallies["jest"].tests), (0, 0));
    }

    #[test]
    fn excluded_files_are_left_out_of_the_tally() {
        let doc = json!({
            "test_cases": [
                { "kind": "test", "framework": "playwright", "file": "e2e/smoke.spec.ts", "span": span(0, 1) },
                { "kind": "test", "framework": "playwright", "file": "e2e/smoke.spec.ts.bak", "span": span(0, 1) },
                { "kind": "test", "framework": "playwright", "file": "e2e/home.spec.ts", "span": span(0, 1) }
            ]
        });
        let counts = by_framework(&doc, &[], &["e2e/smoke.spec.ts".to_owned()]);
        assert_eq!(counts["playwright"].tests, 2, "an exact path, not a prefix");
        assert_eq!(counts["playwright"].files, 2);
    }
}
