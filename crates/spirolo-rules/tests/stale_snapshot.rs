//! `stale-snapshot` on hand-built models: one case per branch the acceptance criteria name
//! — a live key (no finding), a stale key (a finding), an orphan `.snap` file (the whole file
//! stale), a project with a custom resolver (could-not-decide), and the keys a test's name
//! reaches by prefix: a hint, a title holding ` > `, and an `.each`, `describe.each` or
//! non-literal title that could write the key (could-not-decide, kept, never stale).
//!
//! The rule is a pure function of the model, so these tests build the `Snapshot` rows, the
//! sibling `TestCase`s and the `Gate` by hand and read the findings out. The binary fixture
//! (Seam 2) proves the same rule end to end through `spirolo hygiene --rule stale-snapshot`.

use spirolo_model::{
    Diagnostic, Each, Gate, GateTool, Model, Snapshot, SnapshotEntry, Span, TestCase, TestFile,
    TestKind,
};
use spirolo_rules::{Config, Finding, Outcome, Proposal, Rule, Severity, StaleSnapshot};

/// A collected test case in `file` under `suite_path` with `title`.
fn test_case(file: &str, suite_path: &[&str], title: &str) -> TestCase {
    TestCase {
        file: file.to_owned(),
        span: Span::default(),
        framework: Some("vitest".to_owned()),
        title: Some(title.to_owned()),
        kind: TestKind::Test,
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

/// A `.snap` entry with the given suite path, title and ordinal 1, on the file's first line.
fn entry(suite_path: &[&str], title: &str) -> SnapshotEntry {
    let mut key = String::new();
    for segment in suite_path {
        key.push_str(segment);
        key.push_str(" > ");
    }
    key.push_str(title);
    key.push_str(" 1");
    SnapshotEntry {
        key,
        ordinal: 1,
        line: 1,
    }
}

/// A `.snap` file with a sibling test file and the given entries.
fn snapshot(file: &str, sibling: Option<&str>, entries: Vec<SnapshotEntry>) -> Snapshot {
    Snapshot {
        file: file.to_owned(),
        sibling: sibling.map(str::to_owned),
        entries,
        imported_calls: Vec::new(),
    }
}

fn findings(model: &Model) -> Vec<Finding> {
    StaleSnapshot.run(model, &Config::default())
}

/// The findings a run under `--propose` produces.
fn proposed(model: &Model) -> Vec<Finding> {
    StaleSnapshot.run(
        model,
        &Config {
            propose: true,
            ..Config::default()
        },
    )
}

#[test]
fn under_propose_a_stale_entry_is_deleted_by_its_key() {
    let model = Model {
        test_cases: vec![test_case("src/widget.test.ts", &["Widget"], "renders")],
        snapshots: vec![snapshot(
            "src/__snapshots__/widget.test.ts.snap",
            Some("src/widget.test.ts"),
            vec![entry(&["Widget"], "was removed long ago")],
        )],
        ..Model::default()
    };
    assert_eq!(
        proposed(&model)[0].proposal,
        Some(Proposal::Delete {
            what: Some("Widget > was removed long ago 1".to_owned())
        }),
        "the stale entry is deleted, named by the obsolete key"
    );
}

// ------------------------------------------------------------------ stale key

#[test]
fn an_entry_no_sibling_test_names_is_a_stale_finding() {
    let model = Model {
        test_cases: vec![test_case("src/widget.test.ts", &["Widget"], "renders")],
        snapshots: vec![snapshot(
            "src/__snapshots__/widget.test.ts.snap",
            Some("src/widget.test.ts"),
            vec![entry(&["Widget"], "was removed long ago")],
        )],
        ..Model::default()
    };
    let findings = findings(&model);
    assert_eq!(findings.len(), 1, "{findings:#?}");
    let finding = &findings[0];
    assert_eq!(finding.rule, "stale-snapshot");
    assert_eq!(finding.severity, Severity::Info);
    assert_eq!(finding.outcome, Outcome::Finding);
    assert_eq!(
        finding.location.file,
        "src/__snapshots__/widget.test.ts.snap"
    );
}

// ------------------------------------------------------------------ live key

#[test]
fn an_entry_a_sibling_test_still_names_does_not_fire() {
    let model = Model {
        test_cases: vec![test_case("src/widget.test.ts", &["Widget"], "renders")],
        snapshots: vec![snapshot(
            "src/__snapshots__/widget.test.ts.snap",
            Some("src/widget.test.ts"),
            vec![entry(&["Widget"], "renders")],
        )],
        ..Model::default()
    };
    assert!(
        findings(&model).is_empty(),
        "a snapshot whose test still exists is live"
    );
}

// ------------------------------------------------------------------ orphan file

#[test]
fn a_snap_with_no_sibling_test_file_is_one_whole_file_finding() {
    let model = Model {
        snapshots: vec![snapshot(
            "src/__snapshots__/gone.test.ts.snap",
            None,
            vec![entry(&["Gone"], "a"), entry(&["Gone"], "b")],
        )],
        ..Model::default()
    };
    let findings = findings(&model);
    assert_eq!(
        findings.len(),
        1,
        "one finding for the whole orphan file, not per entry"
    );
    let finding = &findings[0];
    assert_eq!(finding.location.file, "src/__snapshots__/gone.test.ts.snap");
    let kind = finding.evidence.iter().find(|e| e.key == "kind").unwrap();
    assert_eq!(kind.value, "orphan-file");
}

// ------------------------------------------------------------------ custom resolver

/// A vitest gate that sets a custom `resolveSnapshotPath`.
fn resolver_gate(source: &str) -> Gate {
    Gate {
        snapshot_resolver: true,
        ..Gate::blank(source, GateTool::Vitest)
    }
}

#[test]
fn a_custom_resolver_defers_the_whole_project() {
    let model = Model {
        gates: vec![resolver_gate("vitest.config.ts")],
        test_cases: vec![test_case("src/widget.test.ts", &["Widget"], "renders")],
        snapshots: vec![snapshot(
            "src/__snapshots__/widget.test.ts.snap",
            Some("src/widget.test.ts"),
            vec![entry(&["Widget"], "was removed long ago")],
        )],
        ..Model::default()
    };
    let findings = findings(&model);
    assert_eq!(
        findings.len(),
        1,
        "one could-not-decide finding, not one per would-be-stale entry: {findings:#?}"
    );
    assert_eq!(findings[0].outcome, Outcome::CouldNotDecide);
    assert!(
        findings[0].explain.contains("resolveSnapshotPath"),
        "the explain names the resolver: {}",
        findings[0].explain
    );
}

// ------------------------------------------------------------------ keys a name reaches by prefix

/// An `.each`/`.for` table of `cases` literal rows.
const fn table(cases: u32) -> Each {
    Each {
        cases: Some(cases),
        params: Some(1),
        source: None,
    }
}

/// The outcomes the rule gives the widget `.snap`'s `entries` beside `cases`.
fn outcomes(cases: Vec<TestCase>, entries: Vec<SnapshotEntry>) -> Vec<(String, Outcome)> {
    let model = Model {
        test_cases: cases,
        snapshots: vec![snapshot(
            "src/__snapshots__/widget.test.ts.snap",
            Some("src/widget.test.ts"),
            entries,
        )],
        ..Model::default()
    };
    findings(&model)
        .into_iter()
        .map(|finding| (finding.anchor, finding.outcome))
        .collect()
}

#[test]
fn a_hinted_entry_and_a_title_holding_the_separator_are_live() {
    let outcomes = outcomes(
        vec![
            test_case("src/widget.test.ts", &["Widget"], "renders"),
            test_case("src/widget.test.ts", &["Widget"], "a > b"),
        ],
        vec![
            entry(&["Widget", "renders"], "with hint"),
            entry(&["Widget"], "renders: jest hint"),
            entry(&["Widget"], "a > b"),
        ],
    );
    assert!(outcomes.is_empty(), "{outcomes:?}");
}

#[test]
fn a_name_that_only_starts_a_key_does_not_write_it() {
    let outcomes = outcomes(
        vec![test_case("src/widget.test.ts", &["Widget"], "render")],
        vec![entry(&["Widget"], "renders")],
    );
    assert_eq!(
        outcomes,
        [("Widget > renders 1".to_owned(), Outcome::Finding)],
        "`render` writes `render 1` or `render > hint 1`, never `renders 1`"
    );
}

#[test]
fn an_entry_an_each_template_could_write_is_could_not_decide() {
    let mut each = test_case("src/widget.test.ts", &["Widget"], "case %s");
    each.each = Some(table(2));
    let outcomes = outcomes(
        vec![each],
        vec![entry(&["Widget"], "case one"), entry(&["Widget"], "other")],
    );
    assert_eq!(
        outcomes,
        [
            ("Widget > case one 1".to_owned(), Outcome::CouldNotDecide),
            ("Widget > other 1".to_owned(), Outcome::Finding),
        ],
        "the template claims only the keys its literal text starts"
    );
}

#[test]
fn an_entry_under_a_computed_title_or_suite_is_could_not_decide() {
    let mut computed = test_case("src/widget.test.ts", &["Widget"], "unused");
    computed.title = None;
    let mut each_suite = test_case("src/widget.test.ts", &[], "Suite %s");
    each_suite.kind = TestKind::Suite;
    each_suite.each = Some(table(1));
    let outcomes = outcomes(
        vec![
            computed,
            each_suite,
            test_case("src/widget.test.ts", &["Suite %s"], "renders"),
            test_case("src/widget.test.ts", &["<dynamic>"], "renders"),
        ],
        vec![
            entry(&["Widget"], "title dyn"),
            entry(&["Suite alpha"], "renders"),
            entry(&["Area north"], "renders"),
        ],
    );
    assert!(
        outcomes
            .iter()
            .all(|(_, outcome)| *outcome == Outcome::CouldNotDecide),
        "{outcomes:?}"
    );
    assert_eq!(outcomes.len(), 3, "{outcomes:?}");
}

#[test]
fn under_propose_a_could_not_decide_entry_is_kept() {
    let mut each = test_case("src/widget.test.ts", &["Widget"], "case %s");
    each.each = Some(table(2));
    let model = Model {
        test_cases: vec![each],
        snapshots: vec![snapshot(
            "src/__snapshots__/widget.test.ts.snap",
            Some("src/widget.test.ts"),
            vec![entry(&["Widget"], "case one")],
        )],
        ..Model::default()
    };
    assert!(
        matches!(proposed(&model)[0].proposal, Some(Proposal::Keep { .. })),
        "never delete what a computed title may write"
    );
}

// ------------------------------------------------------------------ the runner's join

/// A `.snap` entry with `key` verbatim, as the reader lifts it, ordinal 1.
fn raw(key: &str) -> SnapshotEntry {
    SnapshotEntry {
        key: key.to_owned(),
        ordinal: 1,
        line: 1,
    }
}

/// The outcomes for `entries` of `snap` beside `sibling`, a file `framework` collects.
fn outcomes_under(
    framework: &str,
    sibling: &str,
    cases: Vec<TestCase>,
    entries: Vec<SnapshotEntry>,
) -> Vec<(String, Outcome)> {
    let model = Model {
        test_cases: cases,
        test_files: vec![TestFile {
            file: sibling.to_owned(),
            framework: framework.to_owned(),
        }],
        snapshots: vec![snapshot(
            "__snapshots__/sibling.snap",
            Some(sibling),
            entries,
        )],
        ..Model::default()
    };
    findings(&model)
        .into_iter()
        .map(|finding| (finding.anchor, finding.outcome))
        .collect()
}

#[test]
fn under_jest_a_space_joined_entry_is_live() {
    // akveo__react-native-ui-kitten mapping.spec.js:58 and callstack__react-native-paper
    // RadioButton.test.tsx:53, keys as their committed `.snap` files hold them.
    let mapping = "src/processor/js/src/service/mapping/mapping.spec.js";
    let outcomes = outcomes_under(
        "jest",
        mapping,
        vec![test_case(
            mapping,
            &[
                "@mapping: service methods checks",
                "* appearance",
                "* custom",
                "* state",
            ],
            "* active",
        )],
        vec![raw(
            "@mapping: service methods checks * appearance * custom * state * active 1",
        )],
    );
    assert!(outcomes.is_empty(), "{outcomes:?}");
    let radio = "src/components/__tests__/RadioButton/RadioButton.test.tsx";
    let outcomes = outcomes_under(
        "jest",
        radio,
        vec![test_case(
            radio,
            &["RadioButton", "on ios platform"],
            "renders properly",
        )],
        vec![raw("RadioButton on ios platform renders properly 1")],
    );
    assert!(outcomes.is_empty(), "{outcomes:?}");
}

#[test]
fn under_jest_an_entry_no_test_writes_is_still_stale() {
    let radio = "src/components/__tests__/RadioButton/RadioButton.test.tsx";
    let cases = || {
        vec![test_case(
            radio,
            &["RadioButton", "on ios platform"],
            "renders properly after the rename",
        )]
    };
    assert_eq!(
        outcomes_under(
            "jest",
            radio,
            cases(),
            vec![raw("RadioButton on ios platform renders properly 1")],
        ),
        [(
            "RadioButton on ios platform renders properly 1".to_owned(),
            Outcome::Finding
        )],
        "the test was renamed: no test writes the old key"
    );
    assert_eq!(
        outcomes_under(
            "jest",
            radio,
            vec![test_case(
                radio,
                &["RadioButton", "on ios platform"],
                "renders properly"
            )],
            vec![raw("RadioButton > on ios platform > renders properly 1")],
        ),
        [(
            "RadioButton > on ios platform > renders properly 1".to_owned(),
            Outcome::Finding
        )],
        "Jest never writes the ` > ` join, so that key is obsolete under Jest"
    );
}

#[test]
fn when_the_runner_cannot_be_told_either_join_is_live() {
    let sibling = "src/widget.test.ts";
    let outcomes = outcomes_under(
        "vitest",
        sibling,
        vec![test_case(sibling, &["Widget"], "renders")],
        vec![
            raw("Widget > renders 1"),
            raw("Widget renders 1"),
            raw("Widget > was removed long ago 1"),
        ],
    );
    assert_eq!(
        outcomes,
        [(
            "Widget > was removed long ago 1".to_owned(),
            Outcome::Finding
        )],
        "either join of a live name is live; a key neither join writes is stale"
    );
}

#[test]
fn under_jest_an_entry_under_a_computed_suite_title_is_could_not_decide() {
    // algolia__instantsearch templates.test.js:27: `describe(`${templateName}`)` inside
    // `describe('Templates')`, its `test('File content')` writing one hinted key per file.
    let templates = "packages/create-instantsearch-app/e2e/templates.test.js";
    let key = "Templates InstantSearch iOS File content: App.xcodeproj/project.xcworkspace/contents.xcworkspacedata 1";
    assert_eq!(
        outcomes_under(
            "jest",
            templates,
            vec![
                test_case(templates, &["Templates", "<dynamic>"], "Folder structure"),
                test_case(templates, &["Templates", "<dynamic>"], "File content"),
            ],
            vec![raw(key)],
        ),
        [(key.to_owned(), Outcome::CouldNotDecide)]
    );
}

// ------------------------------------------------------------------ a sibling the model cannot read

#[test]
fn a_sibling_that_failed_to_parse_is_one_could_not_decide_for_its_snap() {
    // react__metro symbolicate-test.js: Flow, so the model holds no test for it.
    let sibling = "packages/metro-symbolicate/src/__tests__/symbolicate-test.js";
    let snap = "packages/metro-symbolicate/src/__tests__/__snapshots__/symbolicate-test.js.snap";
    let model = Model {
        test_files: vec![TestFile {
            file: sibling.to_owned(),
            framework: "jest".to_owned(),
        }],
        snapshots: vec![snapshot(
            snap,
            Some(sibling),
            vec![
                raw("symbolicating a crash reporter stack trace with 0:0 native frames 1"),
                raw("symbolicating a profiler map 1"),
            ],
        )],
        diagnostics: vec![Diagnostic {
            code: "parse_failed".to_owned(),
            severity: spirolo_model::Severity::Error,
            file: Some(sibling.to_owned()),
            span: None,
            message: "Flow is not supported".to_owned(),
        }],
        ..Model::default()
    };
    let findings = findings(&model);
    assert_eq!(findings.len(), 1, "{findings:#?}");
    assert_eq!(findings[0].outcome, Outcome::CouldNotDecide);
    assert_eq!(findings[0].location.file, snap);
}

#[test]
fn a_sibling_that_declares_no_test_leaves_every_entry_could_not_decide() {
    // rjsf-team__react-jsonschema-form packages/semantic-ui/test/Grid.test.tsx is
    // `gridTests(Form, …)`: its suite is registered by an imported function.
    let sibling = "packages/semantic-ui/test/Grid.test.tsx";
    let key = "Complex grid > renders person and address and employment in a complex grid, no form data 1";
    assert_eq!(
        outcomes_under("vitest", sibling, Vec::new(), vec![raw(key)]),
        [(key.to_owned(), Outcome::CouldNotDecide)]
    );
}

#[test]
fn when_the_runner_is_unknown_a_title_spanning_lines_is_live_under_either_newline_spelling() {
    let model = Model {
        test_cases: vec![test_case("src/tabs.test.ts", &["Tabs"], "is\n  defined")],
        snapshots: vec![snapshot(
            "src/__snapshots__/tabs.test.ts.snap",
            Some("src/tabs.test.ts"),
            vec![
                entry(&["Tabs"], "is\n  defined"),
                entry(&["Tabs"], "is\\n  defined"),
            ],
        )],
        ..Model::default()
    };
    let findings = findings(&model);
    assert!(findings.is_empty(), "{findings:#?}");
}
