//! `spirolo check` end to end on an `absence-anchor` fixture project: what a maintainer and an
//! agent get from the binary when a render test asserts only that DOM is absent, with nothing
//! to anchor the render.
//!
//! Seam 2 of the PRD's testing decisions: a small fixture with the four cases the acceptance
//! criteria name — a test with only `.not.toBeInTheDocument()` (a finding), the same test plus
//! a throwing `getByRole` (silent), the same plus a positive `toBeInTheDocument` (silent), and
//! a negation collected from inside `waitFor` (a finding) — driven through the real binary with
//! `assert_cmd`, its JSON view snapshotted with `insta`. The rule's branches are covered on
//! hand-built models in `spirolo-rules`; here we prove the wiring — the recogniser filling
//! `throwing_queries`, the DOM-matcher classification, the rule and the report — is whole.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;

#[path = "support/policy.rs"]
mod policy_support;

#[allow(dead_code)]
#[path = "support/status.rs"]
mod status_support;

fn spirolo() -> Command {
    Command::cargo_bin("spirolo").unwrap()
}

/// A fresh project directory under the target tree, named after the test that asked for it.
fn project(name: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    policy_support::info(&path);
    path
}

/// Write `contents` to `path` under `root`, creating parents.
fn write(root: &Path, path: &str, contents: &str) {
    let full = root.join(path);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(full, contents).unwrap();
}

/// A fixture carrying the four acceptance cases, each in its own test:
///
/// - `absent` — only `expect(screen.queryByText('gone')).not.toBeInTheDocument()`: a finding.
/// - `absent_but_anchored` — the same negation, but the body also calls `screen.getByRole` (a
///   throwing query): silent.
/// - `absent_and_positive` — the same negation plus a positive `toBeInTheDocument`: silent.
/// - `absent_in_waitfor` — the negation reached from inside a `waitFor(…)` callback: a finding.
/// - `absent_by_null` — `expect(screen.queryByText('gone')).toBeNull()`, the Testing Library
///   absence idiom, with nothing else: a finding, spelled `queryBy*.toBeNull`.
/// - `absent_by_length` — `expect(screen.queryAllByRole('row')).toHaveLength(0)`: a finding.
/// - `absent_by_null_anchored` — the idiom next to a positive `toBeInTheDocument`: silent.
/// - `absent_but_spied` — the negation next to `expect(onClose).toHaveBeenCalled()`, a call a
///   crashed render would not have made: silent.
/// - `absent_and_never_called` — the negation next to `toHaveBeenCalledTimes(0)`, which holds
///   when nothing rendered: a finding.
/// - `anchored › absent_by_hook` — the negation alone in a `describe` whose `beforeEach` calls
///   `screen.getByRole`: the hook anchors every test under it, silent.
/// - `anchored › sibling › absent_in_sibling` — the same, one describe deeper: still under the
///   hook, silent.
/// - `unhooked › absent_by_hook_elsewhere` — the negation in a describe whose `beforeEach`
///   has no throwing query: a finding.
/// - `absent_but_valued` — the negation next to `expect(container.innerHTML).toBe(…)`, an
///   assertion the rule leaves unread (it may be the anchor the model cannot see): silent.
/// - `absent_but_truthy` — the negation next to `expect(flag).toBeTruthy()` on a plain
///   subject, likewise unread: silent.
fn fixture(name: &str) -> PathBuf {
    let root = project(name);
    write(
        &root,
        "src/banner.test.tsx",
        "import { describe, it, expect, beforeEach } from 'vitest'\n\
         import { screen, waitFor } from '@testing-library/react'\n\
         it('absent', () => {\n\
         \x20 expect(screen.queryByText('gone')).not.toBeInTheDocument()\n\
         })\n\
         it('absent_but_anchored', () => {\n\
         \x20 screen.getByRole('banner')\n\
         \x20 expect(screen.queryByText('gone')).not.toBeInTheDocument()\n\
         })\n\
         it('absent_and_positive', () => {\n\
         \x20 expect(screen.getByRole('banner')).toBeInTheDocument()\n\
         \x20 expect(screen.queryByText('gone')).not.toBeInTheDocument()\n\
         })\n\
         it('absent_in_waitfor', async () => {\n\
         \x20 await waitFor(() => expect(screen.queryByText('gone')).not.toBeInTheDocument())\n\
         })\n\
         it('absent_by_null', () => {\n\
         \x20 expect(screen.queryByText('gone')).toBeNull()\n\
         })\n\
         it('absent_by_length', () => {\n\
         \x20 expect(screen.queryAllByRole('row')).toHaveLength(0)\n\
         })\n\
         it('absent_by_null_anchored', () => {\n\
         \x20 expect(screen.getByRole('banner')).toBeInTheDocument()\n\
         \x20 expect(screen.queryByText('gone')).toBeNull()\n\
         })\n\
         it('absent_but_spied', () => {\n\
         \x20 expect(onClose).toHaveBeenCalled()\n\
         \x20 expect(screen.queryByText('gone')).not.toBeInTheDocument()\n\
         })\n\
         it('absent_and_never_called', () => {\n\
         \x20 expect(onClose).toHaveBeenCalledTimes(0)\n\
         \x20 expect(screen.queryByText('gone')).not.toBeInTheDocument()\n\
         })\n\
         describe('anchored', () => {\n\
         \x20 beforeEach(() => { screen.getByRole('banner') })\n\
         \x20 it('absent_by_hook', () => {\n\
         \x20   expect(screen.queryByText('gone')).not.toBeInTheDocument()\n\
         \x20 })\n\
         \x20 describe('sibling', () => {\n\
         \x20   it('absent_in_sibling', () => {\n\
         \x20     expect(screen.queryByText('gone')).toBeNull()\n\
         \x20   })\n\
         \x20 })\n\
         })\n\
         describe('unhooked', () => {\n\
         \x20 beforeEach(() => { reset() })\n\
         \x20 it('absent_by_hook_elsewhere', () => {\n\
         \x20   expect(screen.queryByText('gone')).not.toBeInTheDocument()\n\
         \x20 })\n\
         })\n\
         it('absent_but_valued', () => {\n\
         \x20 const { container } = render(<Banner />)\n\
         \x20 expect(container.innerHTML).toBe('<div class=\"spin\"></div>')\n\
         \x20 expect(screen.queryByText('gone')).not.toBeInTheDocument()\n\
         })\n\
         it('absent_but_truthy', () => {\n\
         \x20 expect(flag).toBeTruthy()\n\
         \x20 expect(screen.queryByText('gone')).not.toBeInTheDocument()\n\
         })\n",
    );
    root
}

// -------------------------------------------------------------------------- the human view

#[test]
fn check_flags_the_unanchored_negations_and_leaves_the_anchored_alone() {
    let root = fixture("aa-cases");

    spirolo()
        .args(["check", "--all"])
        .arg(&root)
        .assert()
        .success()
        .stdout(
            predicate::str::contains("absence-anchor")
                // The two unanchored tests are named.
                .and(predicate::str::contains("absent"))
                // The matcher is named as it reads in source.
                .and(predicate::str::contains("not.toBeInTheDocument")),
        );
}

// -------------------------------------------------------------------------- the findings json

#[test]
fn exactly_the_two_unanchored_tests_are_findings() {
    let root = fixture("aa-cases-json");

    let output = spirolo()
        .args([
            "check",
            "--all",
            "--format",
            "json",
            "--rule",
            "absence-anchor",
        ])
        .arg(&root)
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let findings = json["findings"].as_array().unwrap();

    // Exactly six findings: `absent`, `absent_in_waitfor`, `absent_by_null`,
    // `absent_by_length`, `absent_and_never_called` and `absent_by_hook_elsewhere`. The
    // anchored ones are absent.
    assert_eq!(findings.len(), 6, "{findings:#?}");
    assert!(findings.iter().all(|f| f["rule"] == "absence-anchor"));
    assert!(findings.iter().all(|f| f["severity"] == "info"));

    let titles: Vec<&str> = findings
        .iter()
        .map(|f| f["message"].as_str().unwrap())
        .collect();
    assert!(
        titles.iter().any(|m| m.contains("\"absent\"")),
        "the only-negation test is flagged: {titles:#?}"
    );
    assert!(
        titles.iter().any(|m| m.contains("\"absent_in_waitfor\"")),
        "the waitFor negation is flagged: {titles:#?}"
    );
    assert!(
        titles.iter().any(|m| m.contains("\"absent_by_null\"")),
        "the queryBy + toBeNull idiom is flagged: {titles:#?}"
    );
    assert!(
        titles.iter().any(|m| m.contains("\"absent_by_length\"")),
        "the queryAllBy + toHaveLength(0) idiom is flagged: {titles:#?}"
    );
    assert!(
        !titles.iter().any(|m| m.contains("absent_but_anchored")),
        "the throwing-query-anchored test is not flagged: {titles:#?}"
    );
    assert!(
        !titles.iter().any(|m| m.contains("absent_by_null_anchored")),
        "the idiom next to a positive DOM assertion is not flagged: {titles:#?}"
    );
    assert!(
        !titles.iter().any(|m| m.contains("absent_but_spied")),
        "a plain spy assertion anchors: {titles:#?}"
    );
    assert!(
        titles
            .iter()
            .any(|m| m.contains("\"absent_and_never_called\"")),
        "toHaveBeenCalledTimes(0) is no anchor: {titles:#?}"
    );
    assert!(
        !titles.iter().any(|m| m.contains("absent_by_hook\"")),
        "a throwing query in the describe's beforeEach anchors: {titles:#?}"
    );
    assert!(
        !titles.iter().any(|m| m.contains("absent_in_sibling")),
        "the hook reaches a nested describe: {titles:#?}"
    );
    assert!(
        titles
            .iter()
            .any(|m| m.contains("\"absent_by_hook_elsewhere\"")),
        "a hook without a throwing query anchors nothing: {titles:#?}"
    );
    assert!(
        !titles.iter().any(|m| m.contains("absent_and_positive")),
        "the positive-assertion-anchored test is not flagged: {titles:#?}"
    );
    assert!(
        !titles.iter().any(|m| m.contains("absent_but_valued")),
        "a value-bearing assertion on a plain subject is unread and silences: {titles:#?}"
    );
    assert!(
        !titles.iter().any(|m| m.contains("absent_but_truthy")),
        "a value-less assertion on a plain subject is unread and silences: {titles:#?}"
    );
}

#[test]
fn every_finding_carries_the_title_path_of_its_test() {
    let root = fixture("aa-cases-title-path");

    let output = spirolo()
        .args([
            "check",
            "--all",
            "--format",
            "json",
            "--rule",
            "absence-anchor",
        ])
        .arg(&root)
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let findings = json["findings"].as_array().unwrap();

    // Each finding is anchored at a test: its title path is the suite path and title of
    // the test case at the finding's location, as the model (and `spirolo list`) names it.
    for finding in findings {
        let case = json["test_cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["file"] == finding["file"] && c["span"]["line"] == finding["line"])
            .unwrap_or_else(|| panic!("no test case at {finding}"));
        assert_eq!(
            finding["test"],
            serde_json::json!({ "suite_path": case["suite_path"], "title": case["title"] }),
        );
    }
    assert_eq!(findings.len(), 6);
}

#[test]
fn the_findings_json_is_stable() {
    let root = fixture("aa-cases-snapshot");

    let output = spirolo()
        .args([
            "check",
            "--all",
            "--format",
            "json",
            "--rule",
            "absence-anchor",
        ])
        .arg(&root)
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    insta::assert_json_snapshot!(
        "check_json_absence_anchor",
        status_support::redact_status(&json["findings"])
    );
}

// ----------------------------------------------------------------------------- family select

#[test]
fn absence_anchor_runs_under_the_audit_family() {
    let root = fixture("aa-audit-family");
    spirolo()
        .args(["audit", "--all"])
        .arg(&root)
        .assert()
        .success()
        .stdout(predicate::str::contains("absence-anchor"));
}

// ------------------------------------------------------------------ round-2 shapes (#397)

/// The titles of the tests `spirolo check` flags under `absence-anchor` in `root`.
fn flagged(root: &Path) -> Vec<String> {
    let output = spirolo()
        .args([
            "check",
            "--all",
            "--format",
            "json",
            "--rule",
            "absence-anchor",
        ])
        .arg(root)
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    json["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["test"]["title"].as_str().unwrap().to_owned())
        .collect()
}

/// Round 3's `DaisyUI` optional-chain result is undefined after an empty render, so
/// jest-dom throws before negating `toHaveAttribute`. The recorded `LayoutGridField`
/// absence reads still pass on an empty render and must remain a finding.
#[test]
fn an_optional_chain_element_result_anchors_but_absence_reads_do_not() {
    let root = project("aa-584-optional-chain");
    write(
        &root,
        "src/layout.test.tsx",
        include_str!("fixtures/absence-anchor/rjsf.test.tsx"),
    );
    assert_eq!(
        flagged(&root),
        ["renders nothing when there is no uiSchema"]
    );
}

/// Round 3's Dialog `firstChild` read is null after an empty render, unlike the
/// persistent container. The real `AddSelect` absence-only test stays a finding.
#[test]
fn a_nullable_dom_property_anchors_but_query_absence_does_not() {
    let root = project("aa-584-dom-property");
    write(
        &root,
        "src/dialog.test.jsx",
        include_str!("fixtures/absence-anchor/carbon.test.jsx"),
    );
    assert_eq!(
        flagged(&root),
        ["does not render arbitrary item fields (e.g. value, Region) via defaultContent"]
    );
}

/// The recorded `LayoutGridField` render intentionally produces nothing. The rule
/// still reports the absence-only reads, but its advice must account for that intent.
#[test]
fn an_intentionally_empty_render_gets_applicable_guidance() {
    let root = project("aa-584-empty-render-guidance");
    write(
        &root,
        "src/layout.test.tsx",
        include_str!("fixtures/absence-anchor/rjsf.test.tsx"),
    );
    let output = spirolo()
        .args([
            "check",
            "--all",
            "--format",
            "json",
            "--rule",
            "absence-anchor",
        ])
        .arg(&root)
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let findings = json["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1);
    assert_eq!(
        findings[0]["test"]["title"],
        "renders nothing when there is no uiSchema"
    );
    let explain = findings[0]["explain"].as_str().unwrap();
    assert!(
        explain.contains("If the component should render content"),
        "{explain}"
    );
    assert!(
        explain.contains("If an empty render is intentional"),
        "{explain}"
    );
    assert!(explain.contains("error handling"), "{explain}");
}

/// A negated jest-dom matcher other than `not.toBeInTheDocument` runs `checkHtmlElement` on
/// its subject before the negation, so on a node read from the render (null after an empty
/// render) it throws: it anchors. The tests are verbatim from decap-cms
/// `renderer.spec.js:234` and `:278` (round 2 rows 11 and 12) and instantsearch
/// `ChatOverlayLayout.test.tsx:65` (row 10); the planted `container` read and the
/// `queryBy*` + `not.toBeInTheDocument` read survive an empty render and stay flagged.
#[test]
fn a_negated_checking_matcher_on_a_queried_node_anchors() {
    let root = project("aa-397-checking-matcher");
    write(
        &root,
        "src/renderer.spec.js",
        r#"import React from 'react';
import { render, screen } from '@testing-library/react';

describe('renderer', () => {
    it('should NOT preserve blob: URLs on non-<img> attributes', () => {
      const blobUrl = `blob:${window.location.origin}/should-be-stripped`;
      const value = [`<a href="${blobUrl}">click</a>`, `<form action="${blobUrl}"></form>`].join(
        '',
      );
      const field = Map({ sanitize_preview: true });

      const { container } = render(
        <MarkdownPreview
          value={value}
          getAsset={jest.fn()}
          resolveWidget={jest.fn()}
          field={field}
        />,
      );
      expect(container.querySelector('a')).not.toHaveAttribute('href');
      expect(container.querySelector('form')).not.toHaveAttribute('action');
    });

    it('should sanitize dangerous link protocols', () => {
      const value = '<a href="javascript:alert(1)">click</a>';

      const { container } = render(
        <MarkdownPreview value={value} getAsset={jest.fn()} resolveWidget={jest.fn()} />,
      );
      const link = container.querySelector('a');
      expect(link).not.toHaveAttribute('href');
    });

  test('renders closed state', () => {
    const { container } = render(
      <ChatOverlayLayout {...defaultProps} open={false} />
    );
    expect(container.querySelector('.ais-Chat-container')).not.toHaveClass(
      'ais-Chat-container--open'
    );
  });

    it('planted: reads only the container', () => {
      const { container } = render(<MarkdownPreview value="" />);
      expect(container).not.toHaveTextContent('x');
    });

    it('planted: queryBy not in the document', () => {
      render(<MarkdownPreview value="" />);
      expect(screen.queryByText('x')).not.toBeInTheDocument();
    });
});
"#,
    );

    let mut titles = flagged(&root);
    titles.sort();
    assert_eq!(
        titles,
        [
            "planted: queryBy not in the document",
            "planted: reads only the container"
        ]
    );
}

/// A plain `toBeEmptyDOMElement()` runs the same element check as the negated matchers: on a
/// node read from the render (`null` after an empty render) it throws, so it anchors (#51).
/// On `container` or `document.body`, which survive an empty render, it passes and stays an
/// absence read, as does `not.toBeInTheDocument` on a queried node.
#[test]
fn to_be_empty_dom_element_on_a_queried_node_anchors() {
    let root = project("aa-51-empty-dom-element");
    write(
        &root,
        "src/grid.test.tsx",
        r"import { render } from '@testing-library/react';
import { Grid } from './grid';

test('queried button is empty', () => {
  const { container } = render(<Grid />);
  expect(container.querySelector('button')).toBeEmptyDOMElement();
});

test('first child is empty', () => {
  const { container } = render(<Grid />);
  expect(container.firstChild).toBeEmptyDOMElement();
});

test('bound button is empty', () => {
  const { container } = render(<Grid />);
  const button = container.querySelector('button');
  expect(button).toBeEmptyDOMElement();
});

test('container is empty', () => {
  const { container } = render(<Grid hidden />);
  expect(container).toBeEmptyDOMElement();
});

test('body is empty', () => {
  render(<Grid hidden />);
  expect(document.body).toBeEmptyDOMElement();
});

test('queried link is not in the document', () => {
  const { container } = render(<Grid hidden />);
  expect(container.querySelector('a')).not.toBeInTheDocument();
});
",
    );

    let mut titles = flagged(&root);
    titles.sort();
    assert_eq!(
        titles,
        [
            "body is empty",
            "container is empty",
            "queried link is not in the document"
        ]
    );
}

/// A same-file helper whose body calls a throwing query (`findByTestId`, `getByText`) throws
/// when the render came up empty, so a test that awaits it is anchored; a helper that only
/// renders anchors nothing. Verbatim from tududi `AdminUsersPage.test.tsx:704` (round 2 row
/// 16) and `SidebarNavAdminVisibility.test.tsx:68` (row 2, stays flagged).
#[test]
fn a_same_file_helper_with_a_throwing_query_anchors() {
    let root = project("aa-397-same-file-helper");
    write(
        &root,
        "src/AdminUsersPage.test.tsx",
        r"import { render, screen, fireEvent } from '@testing-library/react';

const renderPage = (initialEntry = '/admin/users') =>
    render(<AdminUsersPage initialEntry={initialEntry} />);

describe('Admin users and groups page', () => {
    describe('members without an email', () => {
        const openPlainForm = async () => {
            mockAdminApi();
            renderPage();
            await screen.findByTestId('admin-users-panel');
            fireEvent.click(screen.getByText('Add user'));
            return await screen.findByTestId('admin-user-email');
        };

        it('offers no password or verification without an email', async () => {
            await openPlainForm();

            expect(screen.queryByTestId('admin-user-password')).toBeNull();
            expect(screen.queryByTestId('generate-password')).toBeNull();
            expect(
                screen.queryByTestId('require-verification-switch')
            ).toBeNull();
        });
    });
});

describe('SidebarNav admin visibility', () => {
    const renderNav = () =>
        render(
            <SidebarNav
                handleNavClick={jest.fn()}
                location={location}
                isDarkMode={false}
                openTaskModal={jest.fn()}
            />
        );

    it('hides Calendar when the feature is off', () => {
        setSettings({ calendarEnabled: false });

        renderNav();

        expect(screen.queryByText('Calendar')).toBeNull();
    });
});
",
    );

    assert_eq!(flagged(&root), ["hides Calendar when the feature is off"]);
}

/// A helper imported through a tsconfig path whose body returns a throwing query anchors
/// the tests whose describe calls it in a `beforeEach`. Verbatim from react-day-picker
/// `examples/CustomSingle.test.tsx:37` (round 2 row 13) and `test/elements.ts`.
#[test]
fn an_imported_helper_with_a_throwing_query_anchors() {
    let root = project("aa-397-imported-helper");
    write(
        &root,
        "tsconfig.json",
        r#"{ "compilerOptions": { "baseUrl": ".", "paths": { "@/test/*": ["./test/*"] } } }"#,
    );
    write(
        &root,
        "test/elements.ts",
        r#"import { type ByRoleOptions, screen } from "@testing-library/react";

/**
 * Return the button element with the date
 *
 * @param {Date} date - The date to match the button name.
 */
export function dateButton(date: Date) {
  return screen.getByRole("button", {
    name: new RegExp(labelDayButton(date), "s"),
  });
}
"#,
    );
    write(
        &root,
        "examples/CustomSingle.test.tsx",
        r#"import React from "react";

import { dateButton, gridcell } from "@/test/elements";
import { render, screen } from "@/test/render";
import { user } from "@/test/user";

import { CustomSingle } from "./CustomSingle";

const today = new Date();

beforeEach(() => {
  render(<CustomSingle />);
});

describe("when a day is clicked", () => {
  beforeEach(async () => {
    await user.click(dateButton(today));
  });
  describe("when clicking the day again", () => {
    beforeEach(async () => {
      await user.click(dateButton(today));
    });
    test("should update the footer", () => {
      expect(
        screen.queryByText("You selected Thu Nov 25 2021"),
      ).not.toBeInTheDocument();
    });
  });
});

describe("planted: no helper", () => {
  test("should update the footer", () => {
    expect(
      screen.queryByText("You selected Thu Nov 25 2021"),
    ).not.toBeInTheDocument();
  });
});
"#,
    );

    let output = spirolo()
        .args([
            "check",
            "--all",
            "--format",
            "json",
            "--rule",
            "absence-anchor",
        ])
        .arg(&root)
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let suites: Vec<&serde_json::Value> = json["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| &f["test"]["suite_path"])
        .collect();
    assert_eq!(suites, [&serde_json::json!(["planted: no helper"])]);
}

/// A Playwright test is no render test: `page.goto` / `page.click` fail on a broken page
/// before any absence read, so a file importing from `@playwright/test` gives no finding.
/// Verbatim from expo `packages/@expo/cli/e2e/playwright/prod/server-loader.test.ts:136`
/// (round 2 row 17).
#[test]
fn a_playwright_test_is_out_of_scope() {
    let root = project("aa-397-playwright");
    write(
        &root,
        "e2e/playwright/prod/server-loader.test.ts",
        r#"import { test, expect } from '@playwright/test';

import { createExpoServe, executeExpoAsync } from '../../utils/expo';

test.describe('server loaders in production', () => {
  test('handles loader module fetch errors gracefully', async ({ page }) => {
    await page.goto(expoServe.url.href);

    await page.route('**/_expo/loaders/**', (route) => {
      route.abort('failed');
    });

    await page.click('a[href="/posts/static-post-1"]');

    await expect(page.locator('[data-testid="loader-result"]')).not.toBeVisible();
  });
});
"#,
    );

    assert!(flagged(&root).is_empty());
}

/// A name that only starts like a throwing query is no Testing Library query: rjsf's
/// `getByPath` utility, called by the component the test imports, anchors nothing.
/// Verbatim from rjsf `packages/core/test/LayoutGridField.test.tsx:1396` (round 2, an
/// evaluator "wording" row: the finding is right) and the component's `getByPath` reads.
#[test]
fn a_get_by_utility_is_no_throwing_query() {
    let root = project("aa-397-get-by-utility");
    write(
        &root,
        "src/components/fields/LayoutGridField.tsx",
        r"import { getByPath } from '@rjsf/utils';

export default function LayoutGridField(props) {
  const { errorSchema, formData, namePath, Field } = props;
  return (
      <Field
        errorSchema={getByPath(errorSchema, namePath)}
        formData={getByPath(formData, namePath)}
      />
  );
}
",
    );
    write(
        &root,
        "test/LayoutGridField.test.tsx",
        r"import { render, screen } from '@testing-library/react';
import LayoutGridField from '../src/components/fields/LayoutGridField';

describe('LayoutGridField', () => {
  test('renders nothing when there is no uiSchema', () => {
    const props = getProps();
    render(<LayoutGridField {...props} />);

    // Check for all the possible things rendered by the grid
    const uiComponent = screen.queryByTestId(LayoutGridField.TEST_IDS.uiComponent);
    const field = screen.queryByTestId(LayoutGridField.TEST_IDS.field);

    // and find none of them
    expect(uiComponent).not.toBeInTheDocument();
    expect(field).not.toBeInTheDocument();
  });
});
",
    );

    assert_eq!(
        flagged(&root),
        ["renders nothing when there is no uiSchema"]
    );
}

/// An element read off a render helper's result, by destructuring or member access, or an
/// index into the list a query returned, is nullish after an empty render, so a negated
/// jest-dom matcher on it throws and anchors the test (#52). The `container` and
/// `baseElement` of a render result survive an empty render and stay absence reads.
#[test]
fn an_element_read_through_a_helper_result_or_an_index_anchors() {
    let root = project("aa-52-helper-result");
    write(
        &root,
        "src/shapes.test.tsx",
        r"import { render } from '@testing-library/react';

function renderGrid() {
  const { container } = render(<Grid />);
  return {
    container,
    row: container.querySelector('.row'),
    column: container.querySelector('.item'),
  };
}

test('A1 destructured member of a helper result', () => {
  const { row } = renderGrid();
  expect(row).not.toHaveStyle({ display: 'grid' });
});

test('A2 member access on a helper call', () => {
  expect(renderGrid().row).not.toHaveAttribute('span');
});

test('A3 indexed element of querySelectorAll', () => {
  const { container } = render(<Grid />);
  const items = container.querySelectorAll('.item');
  expect(items[0]).not.toHaveAttribute('data-x');
});

test('C0 direct call subject', () => {
  const { container } = render(<Grid />);
  expect(container.querySelector('.row')).not.toHaveClass('wide');
});

test('K1 destructured container', () => {
  const { container } = render(<Grid />);
  expect(container).not.toHaveClass('x');
});

test('K2 destructured baseElement', () => {
  const { baseElement } = render(<Grid />);
  expect(baseElement).not.toHaveClass('x');
});

test('K3 container member of a render call', () => {
  expect(render(<Grid />).container).not.toHaveClass('x');
});

test('K4 container of a helper result', () => {
  const { container } = renderGrid();
  expect(container).not.toHaveClass('x');
});

test('K5 document body', () => {
  render(<Grid />);
  expect(document.body).not.toHaveClass('x');
});

test('K6 container by a template literal key', () => {
  const view = render(<Grid />);
  expect(view[`container`]).not.toHaveClass('x');
});
",
    );

    let mut titles = flagged(&root);
    titles.sort();
    assert_eq!(
        titles,
        [
            "K1 destructured container",
            "K2 destructured baseElement",
            "K3 container member of a render call",
            "K4 container of a helper result",
            "K5 document body",
            "K6 container by a template literal key",
        ]
    );
}

/// An index into a list query for `body`, `html` or `head` returns the document's own node
/// whatever the component rendered, so a negated jest-dom matcher on it holds against an
/// empty render: those tests stay findings, while an index into a list query for a
/// rendered tag still anchors.
#[test]
fn an_index_into_a_list_query_for_the_document_body_stays_reported() {
    let root = project("aa-52-persistent-list");
    write(
        &root,
        "src/modal.test.tsx",
        r"import { render } from '@testing-library/react';

test('T1 body by tag name', () => {
  render(<Modal open={false} />);
  expect(document.getElementsByTagName('body')[0]).not.toHaveClass('modal-open');
});

test('T2 html by tag name', () => {
  render(<Theme />);
  expect(document.getElementsByTagName('html')[0]).not.toHaveAttribute('data-theme');
});

test('T3 body through querySelectorAll', () => {
  render(<Modal open={false} />);
  const bodies = document.querySelectorAll('body');
  expect(bodies[0]).not.toHaveStyle({ overflow: 'hidden' });
});

test('C1 rendered tag by tag name', () => {
  render(<Grid />);
  expect(document.getElementsByTagName('table')[0]).not.toHaveClass('wide');
});
",
    );

    let mut titles = flagged(&root);
    titles.sort();
    assert_eq!(
        titles,
        [
            "T1 body by tag name",
            "T2 html by tag name",
            "T3 body through querySelectorAll",
        ]
    );
}

// ------------------------------------------------------------- asserting helpers (#53)

/// One test per shape, calling the helpers of [`asserting_helpers`] and its own.
const SHAPES: &str = r"import { describe, test, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expectSubmitted, expectNotSubmitted } from './helpers';

const expectCalledOnce = (spy: unknown) => {
  expect(spy).toHaveBeenCalledTimes(1);
};

const expectNeverCalled = (spy: unknown) => {
  expect(spy).toHaveBeenCalledTimes(0);
};

function expectShown(el: unknown) {
  expect(el).toBeInTheDocument();
}

function expectGone(el: unknown) {
  expect(el).not.toBeInTheDocument();
}

function expectSubmittedOnce(spy: unknown) {
  expectCalledOnce(spy);
}

test('imported spy helper', async () => {
  const onSubmit = vi.fn();
  const { container } = render(<Grid onSubmit={onSubmit} />);
  expect(container.querySelector('.missing')).not.toBeInTheDocument();
  await userEvent.click(container.querySelector('button')!);
  expectSubmitted(onSubmit, { a: 1 });
});

test('inline spy assertion', async () => {
  const onSubmit = vi.fn();
  const { container } = render(<Grid onSubmit={onSubmit} />);
  expect(container.querySelector('.missing')).not.toBeInTheDocument();
  await userEvent.click(container.querySelector('button')!);
  expect(onSubmit).toHaveBeenLastCalledWith({ a: 1 });
});

test('same-file count helper', async () => {
  const onSubmit = vi.fn();
  render(<Grid onSubmit={onSubmit} />);
  expect(screen.queryByText('error')).toBeNull();
  expectCalledOnce(onSubmit);
});

test('same-file presence helper', () => {
  render(<Grid />);
  expect(screen.queryByText('error')).toBeNull();
  expectShown(screen.queryByText('ready'));
});

test('helper through a helper', () => {
  const onSubmit = vi.fn();
  render(<Grid onSubmit={onSubmit} />);
  expect(screen.queryByText('error')).toBeNull();
  expectSubmittedOnce(onSubmit);
});

describe('hooked', () => {
  const onLoad = vi.fn();
  beforeEach(() => {
    render(<Grid onLoad={onLoad} />);
    expectCalledOnce(onLoad);
  });

  test('anchored by a hook that calls a spy helper', () => {
    expect(screen.queryByText('error')).toBeNull();
  });
});

test('imported negated spy helper', async () => {
  const onSubmit = vi.fn();
  render(<Grid onSubmit={onSubmit} />);
  expect(screen.queryByText('error')).toBeNull();
  expectNotSubmitted(onSubmit);
});

test('zero count helper', () => {
  const onSubmit = vi.fn();
  render(<Grid onSubmit={onSubmit} />);
  expect(screen.queryByText('error')).toBeNull();
  expectNeverCalled(onSubmit);
});

test('absence helper', () => {
  render(<Grid />);
  expectGone(screen.queryByText('error'));
  expect(screen.queryByText('warning')).toBeNull();
});

test('interaction only', async () => {
  const { container } = render(<Grid />);
  await userEvent.click(container.querySelector('button')!);
  expect(screen.queryByText('error')).not.toBeInTheDocument();
});
";

/// A project with helpers that assert, imported (`src/helpers.tsx`) and of the test file's
/// own, and one test per shape. The tests that call a helper with an anchoring assertion,
/// directly, through another helper or from a hook, and the one written inline, are
/// anchored; the others are not.
fn asserting_helpers(name: &str) -> PathBuf {
    let root = project(name);
    write(
        &root,
        "src/helpers.tsx",
        r"import { expect } from 'vitest';

export function expectSubmitted(spy: unknown, data: unknown) {
  expect(spy).toHaveBeenLastCalledWith(data);
}

export function expectNotSubmitted(spy: unknown) {
  expect(spy).not.toHaveBeenCalled();
}
",
    );
    write(&root, "src/shapes.test.tsx", SHAPES);
    root
}

/// A helper whose own body holds an assertion that anchors when written inline (a plain
/// spy-call assertion with a count other than `0`, a positive jest-dom matcher) fails on an
/// empty render the same way, so a test that calls it is anchored: imported or of the same
/// file, directly or through another helper of that file, from the test or from a hook
/// around it.
#[test]
fn a_helper_with_an_anchoring_assertion_anchors() {
    let root = asserting_helpers("aa-53-anchoring-helper");
    let titles = flagged(&root);
    for anchored in [
        "imported spy helper",
        "inline spy assertion",
        "same-file count helper",
        "same-file presence helper",
        "helper through a helper",
        "anchored by a hook that calls a spy helper",
    ] {
        assert!(
            !titles.iter().any(|t| t == anchored),
            "{anchored:?} is flagged: {titles:?}"
        );
    }
}

/// A helper that holds only absence reads or assertions that hold against an empty body
/// (`not.toHaveBeenCalled()`, `toHaveBeenCalledTimes(0)`) anchors nothing, and neither does
/// an interaction alone: a `userEvent.click` on a missing element does not throw.
#[test]
fn a_helper_with_only_absence_or_inert_assertions_does_not_anchor() {
    let root = asserting_helpers("aa-53-inert-helper");
    let mut titles = flagged(&root);
    titles.sort();
    assert_eq!(
        titles,
        [
            "absence helper",
            "imported negated spy helper",
            "interaction only",
            "zero count helper",
        ]
    );
}
