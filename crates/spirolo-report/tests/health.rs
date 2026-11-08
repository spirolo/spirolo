//! Formula-v1 seam tests. Base rows have the shape of a real `check` capture with neutral
//! names (see `fixtures/health/README.md`); targeted populations below are hand-built
//! inputs, as #311 requires.
use serde_json::Value;
use spirolo_model::Model;
use spirolo_report::score::health;
use spirolo_rules::{status, Evidence, Finding, Location, Outcome, Severity, TestPath};

fn capture() -> Value {
    serde_json::from_str(include_str!("fixtures/health/base-rows.json")).unwrap()
}

fn model() -> Model {
    let capture = capture();
    Model {
        test_cases: serde_json::from_value(capture["test_cases"].clone()).unwrap(),
        test_files: serde_json::from_value(capture["test_files"].clone()).unwrap(),
        gates: serde_json::from_value(capture["gates"].clone()).unwrap(),
        modules: serde_json::from_value(capture["modules"].clone()).unwrap(),
        ..Model::default()
    }
}

fn finding(rule: &str) -> Finding {
    let capture = capture();
    let row = capture["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["rule"] == rule)
        .unwrap();
    let rule = spirolo_rules::rule_named(rule).unwrap();
    Finding {
        rule: rule.name(),
        severity: Severity::Info,
        outcome: Outcome::Finding,
        location: Location {
            file: row["file"].as_str().unwrap().to_owned(),
            line: row["line"].as_u64().unwrap().try_into().unwrap(),
            column: row["column"].as_u64().unwrap().try_into().unwrap(),
        },
        test: row.get("test").map(|test| TestPath {
            suite_path: serde_json::from_value(test["suite_path"].clone()).unwrap(),
            title: serde_json::from_value(test["title"].clone()).unwrap(),
        }),
        evidence: row["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| Evidence::new(e["key"].as_str().unwrap(), e["value"].as_str().unwrap()))
            .collect(),
        message: row["message"].as_str().unwrap().to_owned(),
        explain: row["explain"].as_str().unwrap().to_owned(),
        related: vec![],
        anchor: row["id"].as_str().unwrap().to_owned(),
        ordinal: 0,
        proposal: None,
    }
}

#[test]
fn no_flagged_subjects_means_zero_penalties_in_every_family() {
    let answer = health(&[], &Model::default(), table());
    assert_eq!(
        (answer.score, answer.grade, answer.formula_version),
        (100.0, 'A', 1)
    );
    assert_eq!(
        answer
            .parts
            .iter()
            .map(|part| (part.family, part.title, part.cap, part.penalty, part.ratio))
            .collect::<Vec<_>>(),
        vec![
            ("audit", "Weak tests", 25.0, 0.0, 0.0),
            ("gate", "Merge gate", 30.0, 0.0, 0.0),
            ("hygiene", "Unused tests", 10.0, 0.0, 0.0),
            ("flake", "Flaky", 20.0, 0.0, 0.0),
            ("impact", "Health", 15.0, 0.0, 0.0),
        ]
    );
}

fn proven_table() -> status::Table {
    let mut table = table().clone();
    for row in table.rules.values_mut() {
        row.verdict = status::Verdict::Pass;
        row.interval = Some([0.9, 1.0]);
    }
    table
}

fn tests(count: usize) -> Model {
    let mut model = model();
    let case = model.test_cases[0].clone();
    model.test_cases = (0..count)
        .map(|i| {
            let mut case = case.clone();
            if i > 0 {
                case.title = Some(format!("test {i}"));
            }
            case
        })
        .collect();
    model
}

#[test]
fn an_advisory_rule_deducts_its_full_flagged_share() {
    let answer = health(&[finding("weak-assertion")], &tests(20), table());
    let part = &answer.parts[0];
    assert_eq!((part.ratio, part.penalty), (0.05, 12.5));
    let rule = part
        .rules
        .iter()
        .find(|rule| rule.rule == "weak-assertion")
        .unwrap();
    assert_eq!((rule.weight, rule.flagged, rule.subjects), (1.0, 1, 20));
    close(answer.score, 87.5);
}

#[test]
fn an_unproven_rule_counts_half_and_any_unproven_rule_with_subjects_labels_the_score_provisional() {
    let mut table = table().clone();
    let row = table.rules.get_mut("weak-assertion").unwrap();
    row.verdict = status::Verdict::Fail;
    row.interval = Some([0.8, 0.9]); // FAIL never leans true, regardless of the interval.
    let answer = health(&[finding("weak-assertion")], &tests(20), &table);
    close(answer.parts[0].penalty, 6.25);
    assert_eq!(
        (answer.provisional, answer.lean_true, answer.proven),
        (true, 4, 0)
    );
    // A clean run is still provisional: an unproven rule that found nothing may be missing things (ADR 0003).
    assert!(health(&[], &tests(20), &table).provisional);
    let proven = health(&[finding("weak-assertion")], &tests(20), &proven_table());
    assert_eq!(
        (proven.provisional, proven.lean_true, proven.proven),
        (false, 16, 16)
    );
}

#[test]
fn all_families_reach_their_frozen_saturation() {
    let mut model = tests(10);
    let gate = model.gates[0].clone();
    model.gates = (0..10)
        .map(|i| {
            let mut gate = gate.clone();
            gate.source = format!("gate{i}");
            gate
        })
        .collect();
    let file = model.test_files[0].clone();
    model.test_files = (0..10)
        .map(|i| {
            let mut file = file.clone();
            file.file = format!("file{i}");
            file
        })
        .collect();
    for (family, rule, flagged, cap, ratio) in [
        ("audit", "weak-assertion", 1, 25.0, 0.10),
        ("gate", "fail-closed-setup", 2, 30.0, 0.20),
        ("hygiene", "orphan-test", 1, 10.0, 0.10),
        ("flake", "flake-candidate", 1, 20.0, 0.10),
        ("impact", "untested-hotspot", 2, 15.0, 1.00),
    ] {
        let findings: Vec<_> = (0..flagged)
            .map(|i| {
                let mut f = finding(rule);
                f.location.file = format!("file{i}");
                f
            })
            .collect();
        let answer = health(&findings, &model, &proven_table());
        let part = answer
            .parts
            .iter()
            .find(|part| part.family == family)
            .unwrap();
        assert_eq!((part.ratio, part.penalty), (ratio, cap), "{family}");
    }
}

#[test]
fn grades_use_the_unrounded_score_at_every_boundary() {
    let mut model = tests(25_000);
    model.gates = (0..10)
        .map(|i| {
            let mut gate = model.gates[0].clone();
            gate.source = format!("gate{i}");
            gate
        })
        .collect();
    model.test_files = (0..10_000)
        .map(|i| {
            let mut file = model.test_files[0].clone();
            file.file = format!("file{i}");
            file
        })
        .collect();
    let audit = finding("weak-assertion");
    let gate = finding("fail-closed-setup");
    let hygiene = finding("orphan-test");
    for (gate_count, audit_count, file_count, score, grade) in [
        (0, 1500, 0, 85.0, 'A'),
        (0, 1501, 0, 84.99, 'B'),
        (2, 0, 0, 70.0, 'B'),
        (2, 1, 0, 69.99, 'C'),
        (2, 1500, 0, 55.0, 'C'),
        (2, 1501, 0, 54.99, 'D'),
        (2, 2500, 500, 40.0, 'D'),
        (2, 2500, 501, 39.99, 'F'),
    ] {
        let mut findings: Vec<_> = (0..audit_count)
            .map(|i| {
                let mut f = audit.clone();
                f.test = Some(TestPath::of(&model.test_cases[i]));
                f
            })
            .collect();
        findings.extend((0..gate_count).map(|i| {
            let mut f = gate.clone();
            f.location.file = format!("gate{i}");
            f
        }));
        findings.extend((0..file_count).map(|i| {
            let mut f = hygiene.clone();
            f.location.file = format!("file{i}");
            f
        }));
        let answer = health(&findings, &model, &proven_table());
        assert!(
            (answer.score - score).abs() < 1e-8,
            "{} != {score}",
            answer.score
        );
        assert_eq!(answer.grade, grade, "{score}");
    }
}

#[test]
fn health_facts_use_model_tests_files_gates_and_all_assertions() {
    let mut model = tests(2);
    let mut hook = model.test_cases[0].clone();
    hook.kind = spirolo_model::TestKind::Hook;
    model.test_cases.push(hook);
    let mut suite = model.test_cases[0].clone();
    suite.kind = spirolo_model::TestKind::Suite;
    suite.assertions.clear();
    model.test_cases.push(suite);
    model.test_files.push(model.test_files[0].clone()); // A file collected by two scopes is one file.
    let answer = health(&[], &model, table());
    assert_eq!(
        (
            answer.facts.tests,
            answer.facts.test_files,
            answer.facts.gates,
            answer.facts.expects,
            answer.facts.expects_per_test
        ),
        (2, 1, 1, 3, 1.5)
    );
    close(
        health(&[], &Model::default(), table())
            .facts
            .expects_per_test,
        0.0,
    );
}

fn evidence(finding: &mut Finding, key: &str, value: impl ToString + Copy) {
    finding
        .evidence
        .iter_mut()
        .find(|e| e.key == key)
        .unwrap()
        .value = value.to_string();
}

#[test]
fn hotspot_penalty_uses_complexity_top_200_while_display_uses_score_top_200() {
    let hotspot = finding("untested-hotspot");
    let mut findings: Vec<_> = (0..201)
        .map(|i| {
            let mut f = hotspot.clone();
            f.location.file = format!("module{i:03}.ts");
            evidence(&mut f, "complexity", 10);
            // module199 is the last kept and module200 the one cut: a reversed or missing tie-break
            // keeps module200 too and flags 2.
            evidence(&mut f, "weakness", if i >= 199 { 0.5 } else { 0.1 });
            evidence(&mut f, "score", if i == 200 { 999 } else { 1 });
            f
        })
        .collect();
    findings.reverse(); // Input order must not choose which equal-complexity module is cut.
    let answer = health(&findings, &Model::default(), table());
    let part = answer
        .parts
        .iter()
        .find(|part| part.family == "impact")
        .unwrap();
    assert_eq!((part.rules[0].subjects, part.rules[0].flagged), (200, 1));
    assert!((part.penalty - 0.0375).abs() < 1e-8);
    assert_eq!(
        (answer.hotspots.total, answer.hotspots.shown.len()),
        (201, 200)
    );
    assert_eq!(
        (
            &*answer.hotspots.shown[0].path,
            answer.hotspots.shown[0].score
        ),
        ("module200.ts", 999.0)
    );
    assert_eq!(answer.hotspots.shown[1].path, "module000.ts");
    assert_eq!(
        (
            answer.hotspots.shown[0].complexity,
            answer.hotspots.shown[0].weakness
        ),
        (10, 0.5)
    );
}

#[test]
fn observed_flakes_count_distinct_implicated_ledger_runs_not_test_outcome_rows() {
    use spirolo_model::{Run, RunOutcome};
    use spirolo_rules::{Config, FlakeObserved, Rule};
    let row = |i: usize, title: &str, outcome| Run {
        file: "a.test.ts".to_owned(),
        title: title.to_owned(),
        run_id: format!("r{i}"),
        attempt: 0,
        title_path: vec![title.to_owned()],
        outcome,
        duration_ms: None,
        retries: None,
        tree_hash: Some("one-tree".to_owned()),
        started_at_ms: None,
        condition: None,
        report: format!("runs/r{i}.json"),
        runner_home: None,
        joined: None,
    };
    let mut model = Model {
        runs: (0..20)
            .map(|i| row(i, "stable", RunOutcome::Passed))
            .collect(),
        ..Model::default()
    };
    model.runs.extend([
        row(0, "flaky", RunOutcome::Passed),
        row(1, "flaky", RunOutcome::Failed),
    ]);
    let mut findings = FlakeObserved.run(&model, &Config::default());
    assert_eq!(findings.len(), 1);
    findings.push(findings[0].clone());
    let answer = health(&findings, &model, &proven_table());
    let part = answer
        .parts
        .iter()
        .find(|part| part.family == "flake")
        .unwrap();
    let rule = part
        .rules
        .iter()
        .find(|rule| rule.rule == "flake-observed")
        .unwrap();
    assert_eq!((rule.flagged, rule.subjects, part.penalty), (2, 20, 20.0));
}

#[test]
fn undecided_and_listed_rows_flag_nothing_in_any_subject_population() {
    let mut findings = vec![];
    for rule in [
        "weak-assertion",
        "fail-closed-setup",
        "orphan-test",
        "flake-candidate",
        "untested-hotspot",
    ] {
        for outcome in [Outcome::CouldNotDecide, Outcome::Listed] {
            let mut f = finding(rule);
            f.outcome = outcome;
            findings.push(f);
        }
    }
    let answer = health(&findings, &model(), table());
    close(answer.score, 100.0);
    // Nothing decided is still unproven evidence: the clean score stays provisional (ADR 0003).
    assert!(answer.provisional);
    assert!(answer
        .parts
        .iter()
        .flat_map(|part| &part.rules)
        .all(|rule| rule.flagged == 0));
    assert_eq!(answer.hotspots.total, 0);
}

#[test]
fn per_test_subjects_deduplicate_by_file_suite_path_and_title() {
    let first = finding("weak-assertion");
    let mut duplicate = first.clone();
    duplicate.ordinal = 1;
    duplicate.location.line += 1;
    let mut suite = first.clone();
    suite
        .test
        .as_mut()
        .unwrap()
        .suite_path
        .push("other suite".to_owned());
    let mut title = first.clone();
    title.test.as_mut().unwrap().title = Some("another test".to_owned());
    let mut file = first.clone();
    file.location.file = "other.test.ts".to_owned();
    let answer = health(&[first, duplicate, suite, title, file], &tests(10), table());
    let rule = answer.parts[0]
        .rules
        .iter()
        .find(|rule| rule.rule == "weak-assertion")
        .unwrap();
    assert_eq!(rule.flagged, 4);
}

#[test]
fn per_file_and_gate_subjects_deduplicate_by_file() {
    for name in ["fail-closed-setup", "orphan-test"] {
        let first = finding(name);
        let mut duplicate = first.clone();
        duplicate.ordinal = 1;
        let answer = health(&[first, duplicate], &model(), table());
        let rule = answer
            .parts
            .iter()
            .flat_map(|part| &part.rules)
            .find(|rule| rule.rule == name)
            .unwrap();
        assert_eq!((rule.flagged, rule.subjects), (1, 1));
    }
}

#[test]
fn a_rule_with_no_subjects_contributes_zero_even_with_a_finding() {
    let mut model = model();
    model.test_cases.clear();
    model.test_files.clear();
    model.gates.clear();
    let answer = health(
        &[
            finding("weak-assertion"),
            finding("fail-closed-setup"),
            finding("orphan-test"),
        ],
        &model,
        table(),
    );
    close(answer.score, 100.0);
    // No rule has a subject, so none contributes and nothing makes the score provisional.
    assert!(!answer.provisional);
    assert!(answer.parts.iter().all(|part| part.ratio == 0.0));
}

#[test]
fn penalties_stop_at_the_cap_beyond_saturation_in_each_family() {
    let mut model = tests(10);
    model.gates = (0..10)
        .map(|i| {
            let mut gate = model.gates[0].clone();
            gate.source = format!("gate{i}");
            gate
        })
        .collect();
    model.test_files = (0..10)
        .map(|i| {
            let mut file = model.test_files[0].clone();
            file.file = format!("file{i}");
            file
        })
        .collect();
    for (family, name, count, cap) in [
        ("audit", "weak-assertion", 2, 25.0),
        ("gate", "fail-closed-setup", 3, 30.0),
        ("hygiene", "orphan-test", 2, 10.0),
        ("flake", "flake-candidate", 2, 20.0),
        // Impact's ratio cannot exceed one: extra modules are cut from the population.
        ("impact", "untested-hotspot", 201, 15.0),
    ] {
        let row = finding(name);
        let findings: Vec<_> = (0..count)
            .map(|i| {
                let mut f = row.clone();
                f.location.file = format!("file{i}");
                f
            })
            .collect();
        let answer = health(&findings, &model, &proven_table());
        let part = answer
            .parts
            .iter()
            .find(|part| part.family == family)
            .unwrap();
        close(part.penalty, cap);
    }
}

#[test]
fn any_decided_weak_hotspot_row_flags_its_module_once() {
    let decided = finding("untested-hotspot");
    let mut undecided = decided.clone();
    undecided.outcome = Outcome::CouldNotDecide;
    let answer = health(
        &[undecided, decided.clone(), decided],
        &Model::default(),
        table(),
    );
    let part = answer
        .parts
        .iter()
        .find(|part| part.family == "impact")
        .unwrap();
    assert_eq!((part.rules[0].subjects, part.rules[0].flagged), (1, 1));
    assert_eq!(answer.hotspots.total, 1);
}

#[test]
fn saga_sites_without_test_paths_count_their_enclosing_test_once() {
    use spirolo_model::{SagaStepKind, SagaWalk, Span};
    use spirolo_rules::{Config, GeneratorWalk, Rule};
    let mut model = tests(2);
    let case = &model.test_cases[0];
    model.saga_walks = (0..2)
        .map(|i| SagaWalk {
            file: case.file.clone(),
            kind: SagaStepKind::GenNext,
            effect: Some("call".to_owned()),
            span: Span {
                start: case.span.start + i + 1,
                end: case.span.start + i + 2,
                line: case.span.line + i + 1,
                column: 1,
            },
        })
        .collect();
    // This second test is later in the file, as in a real model.
    model.test_cases[1].span.start = model.test_cases[0].span.end + 1;
    model.test_cases[1].span.end = model.test_cases[1].span.start + 10;
    let findings = GeneratorWalk.run(&model, &Config::default());
    assert!(findings.iter().all(|f| f.test.is_none()));
    let answer = health(&findings, &model, table());
    let rule = answer.parts[0]
        .rules
        .iter()
        .find(|rule| rule.rule == "generator-walk")
        .unwrap();
    assert_eq!((rule.flagged, rule.subjects), (1, 2));
}

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
}

// A real rule-status capture keeps formula tests independent of later corpus measurements.
fn table() -> &'static status::Table {
    static TABLE: std::sync::OnceLock<status::Table> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| status::parse(include_str!("fixtures/health/rule-status.json")).unwrap())
}

#[test]
fn a_drifted_handler_flags_the_tests_that_import_it_or_the_one_case_it_sits_in() {
    use spirolo_model::BuildOptions;
    use spirolo_rules::{Config, HandlerDrift, Rule};
    // A small project with a shipped drift: `orders.ts` is reached by the
    // orders spec through `mocks/server`, and the accounts spec declares a drifted override
    // inside one of its two cases. The format spec loads neither.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/health/handler-drift-reach");
    let options = BuildOptions {
        handler_globs: vec![
            "src/mocks/**/*.ts".to_owned(),
            "src/**/*.msw.test.tsx".to_owned(),
        ],
        ..BuildOptions::default()
    };
    let model = Model::build_with(&root, &options, &spirolo_frameworks::recognisers());
    let config = Config {
        url_sources: vec!["http://localhost:5000/${API_VERSION}".to_owned()],
        env: [("API_VERSION".to_owned(), "v1".to_owned())].into(),
        ..Config::default()
    };
    let findings = HandlerDrift.run(&model, &config);
    let drifts = findings.iter().filter(|f| f.outcome == Outcome::Finding);
    assert_eq!(drifts.clone().count(), 2);
    assert!(drifts.clone().all(|f| f.test.is_none()));

    let answer = health(&findings, &model, table());
    let rule = answer.parts[0]
        .rules
        .iter()
        .find(|rule| rule.rule == "handler-drift")
        .unwrap();
    assert_eq!((rule.flagged, rule.subjects), (3, 5));
}
