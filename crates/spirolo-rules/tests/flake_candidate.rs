//! `flake-candidate` on hand-built models: the runs-blindness guard, and one case per branch
//! of its scope — a mechanism in the test body, in a hook in scope, in a hook of a sibling
//! suite, in a setup file, in a test-only helper (at its top level, in a function the test
//! calls into, in one it does not), in a helper production also imports — and of its guards
//! and skip gates.
//!
//! The rule is a pure function of the model, so these tests build `FlakeSite`, `FlakeGuard`,
//! `Module` and `TestCase` rows by hand and read the findings out. Which source text becomes
//! which site is the extractor's business, tested in `spirolo-model`; the binary fixtures
//! (Seam 2) prove the two together.

use spirolo_model::{
    Enclosing, FlakeGuard, FlakeKind, FlakeNames, FlakeSite, FlakeSource, Gate, GateTool, Import,
    ImportKind, Model, Module, Run, RunOutcome, Skip, SkipKind, Span, TestCase, TestKind,
};
use spirolo_rules::{Config, Finding, FlakeCandidate, Outcome, Rule, Severity};

/// A span covering lines `from..=to`, with byte offsets that track the lines so containment
/// by line is containment by byte.
const fn lines(from: u32, to: u32) -> Span {
    Span {
        start: from * 100,
        end: to * 100 + 50,
        line: from,
        column: 1,
    }
}

/// A case of `kind` in `file` spanning `from..=to`, under `suite_path`.
fn case(
    file: &str,
    kind: TestKind,
    title: &str,
    suite_path: &[&str],
    from: u32,
    to: u32,
) -> TestCase {
    TestCase {
        file: file.to_owned(),
        span: lines(from, to),
        framework: Some("vitest".to_owned()),
        title: Some(title.to_owned()),
        kind,
        suite_path: suite_path.iter().map(|s| (*s).to_owned()).collect(),
        assertions: Vec::new(),
        throwing_queries: Vec::new(),
        mocks: Vec::new(),
        skip: None,
        each: None,
        loops: Vec::new(),
        modifiers: Vec::new(),
        tags: Vec::new(),
        body_hash: None,
        empty: false,
        context: Vec::new(),
    }
}

/// A decided site of `kind` at `line` of `file`.
fn site(file: &str, kind: FlakeKind, source: FlakeSource, line: u32) -> FlakeSite {
    FlakeSite {
        file: file.to_owned(),
        span: lines(line, line),
        kind,
        source,
        what: "Date.now()".to_owned(),
        decided: true,
        hook: None,
        target: None,
        writes: false,
        calendar: false,
        enclosing: Enclosing::Module,
    }
}

/// A wall-clock site at `line` of `file`.
fn clock(file: &str, line: u32) -> FlakeSite {
    site(file, FlakeKind::WallClock, FlakeSource::Clock, line)
}

/// A fake-timers guard at `line` of `file`.
fn fake_timers(file: &str, line: u32) -> FlakeGuard {
    FlakeGuard {
        file: file.to_owned(),
        span: lines(line, line),
        covers: vec![FlakeSource::Clock, FlakeSource::Timer],
        target: None,
        what: "vi.useFakeTimers()".to_owned(),
    }
}

/// The findings the rule reports over `model`.
fn findings(model: &Model) -> Vec<Finding> {
    FlakeCandidate.run(model, &Config::default())
}

/// The `(title, kind evidence)` of every decided finding.
fn flagged(model: &Model) -> Vec<(String, String)> {
    findings(model)
        .into_iter()
        .filter(|f| f.outcome == Outcome::Finding)
        .map(|f| {
            let kind = f
                .evidence
                .iter()
                .find(|e| e.key == "kind")
                .map(|e| e.value.clone())
                .unwrap_or_default();
            (f.test.unwrap().title.unwrap(), kind)
        })
        .collect()
}

const SPEC: &str = "src/a.test.ts";

#[test]
fn the_findings_are_byte_equal_with_an_empty_and_a_populated_run_table() {
    let mut model = Model {
        test_cases: vec![case(SPEC, TestKind::Test, "reads the clock", &[], 1, 5)],
        flake_sites: vec![clock(SPEC, 3)],
        ..Model::default()
    };
    let empty = format!("{:?}", findings(&model));
    model.runs = [RunOutcome::Passed, RunOutcome::Failed]
        .into_iter()
        .enumerate()
        .map(|(attempt, outcome)| Run {
            file: SPEC.to_owned(),
            title: "reads the clock".to_owned(),
            run_id: format!("run-{attempt}"),
            attempt: 0,
            title_path: vec!["reads the clock".to_owned()],
            outcome,
            duration_ms: Some(3),
            retries: None,
            tree_hash: Some("abc".to_owned()),
            started_at_ms: None,
            condition: Some("idle".to_owned()),
            report: "report.json".to_owned(),
            runner_home: None,
            joined: None,
        })
        .collect();
    assert!(!empty.is_empty() && empty != "[]");
    assert_eq!(format!("{:?}", findings(&model)), empty);
}

#[test]
fn a_mechanism_in_a_hook_in_scope_is_attributed_to_the_suites_tests_and_not_to_a_sibling_suite() {
    let model = Model {
        test_cases: vec![
            case(SPEC, TestKind::Suite, "timed", &[], 1, 10),
            case(SPEC, TestKind::Hook, "beforeEach", &["timed"], 2, 4),
            case(SPEC, TestKind::Test, "inside", &["timed"], 5, 9),
            case(SPEC, TestKind::Suite, "other", &[], 11, 20),
            case(SPEC, TestKind::Test, "sibling", &["other"], 12, 19),
        ],
        flake_sites: vec![clock(SPEC, 3)],
        ..Model::default()
    };
    let findings = findings(&model);
    assert_eq!(
        flagged(&model),
        [("inside".to_owned(), "wall-clock".to_owned())]
    );
    // The mechanism's own location is the related location.
    assert_eq!(findings[0].related[0].location.line, 3);
    assert_eq!(findings[0].location.line, 5);
}

/// A module at `path` importing each of `imports` (all resolved project paths).
fn module(path: &str, imports: &[&str]) -> Module {
    Module {
        path: path.to_owned(),
        language: spirolo_model::Language::TypeScript,
        parse_status: spirolo_model::ParseStatus::Parsed,
        imports: imports
            .iter()
            .enumerate()
            .map(|(i, resolved)| Import {
                specifier: format!("./{resolved}"),
                resolved: Some((*resolved).to_owned()),
                reason: None,
                kind: ImportKind::Static,
                span: lines(u32::try_from(i).unwrap(), u32::try_from(i).unwrap()),
            })
            .collect(),
        exports: Vec::new(),
        constants: Vec::new(),
        mocks: Vec::new(),
        references: Vec::new(),
        spies: Vec::new(),
        loads: Vec::new(),
        complexity: 1,
        generated: false,
    }
}

#[test]
fn a_mechanism_in_a_module_only_test_files_import_reaches_their_tests_and_no_other_module_does() {
    let model = Model {
        modules: vec![
            module(
                SPEC,
                &["src/test-utils/wait.ts", "src/clock.ts", "src/kit/index.ts"],
            ),
            module("src/test-utils/wait.ts", &["src/test-utils/deeper.ts"]),
            module("src/test-utils/deeper.ts", &[]),
            module("src/clock.ts", &[]),
            module("src/app.ts", &["src/clock.ts"]),
            module("src/kit/index.ts", &[]),
        ],
        test_cases: vec![
            case(SPEC, TestKind::Test, "first", &[], 10, 12),
            case(SPEC, TestKind::Test, "second", &[], 13, 15),
        ],
        flake_sites: vec![
            clock("src/test-utils/wait.ts", 4),
            // Only the helper imports this one: a non-test file, so it is not test-only
            // (PRD 0004 story 10, "one that only test files import").
            clock("src/test-utils/deeper.ts", 1),
            // Production imports this one.
            clock("src/clock.ts", 2),
            // Only the spec imports this package entry, but the package publishes it.
            clock("src/kit/index.ts", 3),
        ],
        package_entries: vec!["src/kit/index.ts".to_owned()],
        ..Model::default()
    };
    let findings = findings(&model);
    assert_eq!(
        flagged(&model),
        [
            ("first".to_owned(), "wall-clock".to_owned()),
            ("second".to_owned(), "wall-clock".to_owned()),
        ]
    );
    let related: Vec<&str> = findings[0]
        .related
        .iter()
        .map(|r| r.location.file.as_str())
        .collect();
    assert_eq!(related, ["src/test-utils/wait.ts"]);
    assert_eq!(findings[0].location.file, SPEC);
}

#[test]
fn a_mechanism_in_a_setup_file_or_its_helper_reaches_every_test_the_config_governs() {
    let mut gate = Gate::blank("pkg/vitest.config.ts", GateTool::Vitest);
    gate.setup_files = vec!["./test/setup.ts".to_owned()];
    let model = Model {
        modules: vec![
            module("pkg/src/a.test.ts", &[]),
            module("pkg/test/setup.ts", &["pkg/test/seed.ts"]),
            module("pkg/test/seed.ts", &[]),
            module("other/src/b.test.ts", &[]),
        ],
        test_cases: vec![
            case("pkg/src/a.test.ts", TestKind::Test, "governed", &[], 1, 3),
            case(
                "other/src/b.test.ts",
                TestKind::Test,
                "elsewhere",
                &[],
                1,
                3,
            ),
        ],
        flake_sites: vec![clock("pkg/test/setup.ts", 5), clock("pkg/test/seed.ts", 2)],
        gates: vec![gate],
        ..Model::default()
    };
    let findings = findings(&model);
    assert_eq!(
        flagged(&model),
        [("governed".to_owned(), "wall-clock".to_owned())]
    );
    let related: Vec<&str> = findings[0]
        .related
        .iter()
        .map(|r| r.location.file.as_str())
        .collect();
    assert_eq!(related, ["pkg/test/seed.ts", "pkg/test/setup.ts"]);
}

/// `site` written in the function, method or class called `name`.
fn within(mut site: FlakeSite, name: &str) -> FlakeSite {
    site.enclosing = Enclosing::Named(name.to_owned());
    site
}

/// The names row of `file`: each enclosing code with the names it references, and each use
/// at the start of its line.
fn names(file: &str, calls: &[(Enclosing, &[&str])], uses: &[(u32, &str)]) -> FlakeNames {
    FlakeNames {
        file: file.to_owned(),
        calls: calls
            .iter()
            .map(|(enclosing, names)| {
                (
                    enclosing.clone(),
                    names.iter().map(|n| (*n).to_owned()).collect(),
                )
            })
            .collect(),
        uses: uses
            .iter()
            .map(|(line, name)| (lines(*line, *line).start, (*name).to_owned()))
            .collect(),
    }
}

#[test]
fn a_mechanism_in_a_named_helper_function_reaches_only_the_tests_whose_code_calls_into_it() {
    let helper = "src/test-utils/provider.ts";
    let named = |name: &str| Enclosing::Named(name.to_owned());
    let model = Model {
        modules: vec![module(SPEC, &[helper]), module(helper, &[])],
        test_cases: vec![
            case(SPEC, TestKind::Test, "by id", &[], 10, 12),
            case(SPEC, TestKind::Test, "random", &[], 13, 15),
        ],
        flake_sites: vec![
            within(clock(helper, 4), "search"),
            // Only code the reader could not name calls `pick`.
            within(clock(helper, 7), "pick"),
        ],
        flake_names: vec![
            names(
                SPEC,
                &[],
                &[(11, "provider"), (11, "getById"), (14, "getRandom")],
            ),
            names(
                helper,
                &[
                    (named("getRandom"), &["search"]),
                    (named("search"), &["Math", "random"]),
                    (Enclosing::Anonymous, &["pick"]),
                ],
                &[],
            ),
        ],
        ..Model::default()
    };
    let outcomes: Vec<(String, String, Outcome)> = findings(&model)
        .into_iter()
        .map(|f| {
            let kind = f.evidence.iter().find(|e| e.key == "kind").unwrap();
            (
                f.test.unwrap().title.unwrap(),
                kind.value.clone(),
                f.outcome,
            )
        })
        .collect();
    assert_eq!(
        outcomes,
        [
            (
                "by id".to_owned(),
                "wall-clock".to_owned(),
                Outcome::CouldNotDecide
            ),
            // The clock read in `search` it reaches is decided, so the finding is.
            (
                "random".to_owned(),
                "wall-clock".to_owned(),
                Outcome::Finding
            ),
        ]
    );
}

#[test]
fn a_named_function_of_a_setup_files_helper_reaches_the_tests_when_the_setup_file_calls_it() {
    let mut gate = Gate::blank("pkg/vitest.config.ts", GateTool::Vitest);
    gate.setup_files = vec!["./test/setup.ts".to_owned()];
    let seed = "pkg/test/seed.ts";
    let model = Model {
        modules: vec![
            module("pkg/src/a.test.ts", &[]),
            module("pkg/test/setup.ts", &[seed]),
            module(seed, &[]),
        ],
        test_cases: vec![case(
            "pkg/src/a.test.ts",
            TestKind::Test,
            "governed",
            &[],
            1,
            3,
        )],
        flake_sites: vec![
            within(clock(seed, 2), "seedAll"),
            within(clock(seed, 5), "unused"),
        ],
        flake_names: vec![
            names(
                "pkg/test/setup.ts",
                &[(Enclosing::Module, &["seedAll"])],
                &[],
            ),
            names(seed, &[], &[]),
        ],
        gates: vec![gate],
        ..Model::default()
    };
    let findings = findings(&model);
    assert_eq!(
        flagged(&model),
        [("governed".to_owned(), "wall-clock".to_owned())]
    );
    assert_eq!(findings[0].related.len(), 1);
    assert_eq!(findings[0].related[0].location.line, 2);
}

/// `case` under a `kind` modifier whose condition is `condition`.
fn gated(mut case: TestCase, kind: SkipKind, condition: &str) -> TestCase {
    case.skip = Some(Skip {
        kind,
        condition: Some(condition.to_owned()),
        reason: None,
        comments: Vec::new(),
        span: case.span,
    });
    case
}

#[test]
fn a_test_behind_an_env_gate_that_is_off_by_default_is_listed_at_info_with_the_gate() {
    let model = Model {
        test_cases: vec![
            gated(
                case(SPEC, TestKind::Test, "perf only", &[], 1, 3),
                SkipKind::SkipIf,
                "!process.env.PERF",
            ),
            gated(
                case(SPEC, TestKind::Suite, "perf suite", &[], 4, 10),
                SkipKind::RunIf,
                "process.env.PERF === '1'",
            ),
            case(
                SPEC,
                TestKind::Test,
                "in the perf suite",
                &["perf suite"],
                5,
                9,
            ),
            gated(
                case(SPEC, TestKind::Test, "skipped on CI", &[], 11, 13),
                SkipKind::SkipIf,
                "!!process.env.CI",
            ),
        ],
        flake_sites: vec![clock(SPEC, 2), clock(SPEC, 6), clock(SPEC, 12)],
        ..Model::default()
    };
    let findings = findings(&model);
    let rows: Vec<(String, Outcome, Severity, Option<String>)> = findings
        .iter()
        .map(|f| {
            let gate = f
                .evidence
                .iter()
                .find(|e| e.key == "gate")
                .map(|e| e.value.clone());
            (
                f.test.clone().unwrap().title.unwrap(),
                f.outcome,
                f.severity,
                gate,
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            (
                "perf only".to_owned(),
                Outcome::Listed,
                Severity::Info,
                Some("skipIf(!process.env.PERF)".to_owned())
            ),
            (
                "in the perf suite".to_owned(),
                Outcome::Listed,
                Severity::Info,
                Some("runIf(process.env.PERF === '1')".to_owned())
            ),
            // Skipped only where CI is set: it runs by default, so it is a live candidate —
            // a finding at the rule's advisory default, not a listed row.
            (
                "skipped on CI".to_owned(),
                Outcome::Finding,
                Severity::Info,
                None
            ),
        ]
    );
}

#[test]
fn fake_timers_in_a_hook_in_scope_suppress_a_clock_site_but_fake_timers_in_a_sibling_test_do_not() {
    let model = Model {
        test_cases: vec![
            case(SPEC, TestKind::Suite, "faked", &[], 1, 10),
            case(SPEC, TestKind::Hook, "beforeEach", &["faked"], 2, 3),
            case(SPEC, TestKind::Test, "under the hook", &["faked"], 4, 9),
            case(SPEC, TestKind::Test, "fakes its own", &[], 11, 14),
            case(SPEC, TestKind::Test, "real clock", &[], 15, 18),
        ],
        flake_sites: vec![clock(SPEC, 5), clock(SPEC, 13), clock(SPEC, 16)],
        flake_guards: vec![fake_timers(SPEC, 2), fake_timers(SPEC, 12)],
        ..Model::default()
    };
    assert_eq!(
        flagged(&model),
        [("real clock".to_owned(), "wall-clock".to_owned())]
    );
}
