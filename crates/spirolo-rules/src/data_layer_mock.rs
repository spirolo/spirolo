//! `data-layer-mock`: a test that module-mocks the data layer instead of the network, seen
//! for what it is — a proof of the mock, not of the flow.
//!
//! The rule reads `TestCase.mocks` and nothing else it did not resolve upstream. It reports
//! a module mock (`vi.mock` / `jest.mock`) whose target, resolved through the model's
//! resolver, matches a `dataLayer` entry the maintainer declared, but only in the test files
//! the `dataLayerScope` globs cover (default: files named `*.integration.*`, `*.msw.*`, and
//! anything under a directory named `integration`). Outside that scope the rule is silent, so
//! a pure unit test that legitimately stubs a client is not noise (prep doc addendum A9).
//!
//! Matching handles the two shapes a `dataLayer` entry takes:
//!
//! - a **project module path** (`src/supabaseClient`): matched against the mock's *resolved*
//!   path (`src/supabaseClient.ts`), so an alias (`@/supabaseClient`), a relative specifier
//!   (`../../supabase/client`) and any other spelling that resolves to the same file all
//!   match one entry. The comparison is extension-insensitive, since a `dataLayer` entry is
//!   written without one.
//! - a **package name** (`@supabase/supabase-js`): matched against the mock's *specifier as
//!   written*, because an external package does not resolve to a project file and so has no
//!   resolved path. A subpath of the package (`@acme/core-ai-kit/providers`) matches the
//!   entry `@acme/core-ai-kit`: the specifier equals the entry or starts with the entry
//!   and a `/`. A specifier that merely shares a prefix (`@acme/core-ai-kit-extras`) does
//!   not.
//!
//! Severity `warn`: this is a reading a maintainer ratchets, not a build-breaker. The finding
//! names the resolved data-layer module as its related location and carries the specifier as
//! written in the evidence, so an agent sees both the module and how the test spelled it.
//!
//! The finding is one per mock site and its identity is the file plus the specifier as
//! written: `mock:@supabase/supabase-js` for a `vi.mock` hoisted to file scope, and
//! `<suite › title> › mock:@supabase/supabase-js` for a `vi.doMock` written inside a test's
//! body, with the collision ordinal counting the same specifier mocked again at the same
//! scope. The message still names the first test the mock is in effect for, which is the
//! test an adjudicator runs; a retitled or inserted test does not re-key a hoisted mock.

use globset::{Glob, GlobSetBuilder};
use spirolo_model::{Mock, MockKind, Model, TestCase};

use crate::finding::{
    number_collisions, sort, Evidence, Finding, Location, Outcome, RelatedLocation, TestPath,
};
use crate::{Config, Family, Rule, Severity};

/// The scope globs the rule falls back to when the config sets no `dataLayerScope`:
/// integration and MSW specs, and anything under an `integration` directory. Written as
/// `globset` patterns matched against the repo-relative, slash-separated file path.
const DEFAULT_SCOPE: &[&str] = &["**/*.integration.*", "**/*.msw.*", "**/integration/**"];

/// The file extensions stripped before comparing a resolved path to a `dataLayer` entry, so
/// `src/supabaseClient.ts` matches the entry `src/supabaseClient`. Longest first, so `.d.ts`
/// is tried before `.ts`.
const MODULE_EXTENSIONS: &[&str] = &[
    ".d.ts", ".tsx", ".ts", ".jsx", ".js", ".mjs", ".cjs", ".mts", ".cts",
];

/// The `data-layer-mock` rule. See the module docs for the semantics.
pub struct DataLayerMock;

impl Rule for DataLayerMock {
    fn name(&self) -> &'static str {
        "data-layer-mock"
    }

    fn family(&self) -> Family {
        Family::Audit
    }

    fn subject(&self) -> crate::Subject {
        crate::Subject::Tests
    }

    fn summary(&self) -> &'static str {
        "A test module-mocks the data layer instead of the network, so it proves the mock and not the flow it names."
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn run(&self, model: &Model, config: &Config) -> Vec<Finding> {
        // With no data layer declared the rule cannot know what to flag, so it is silent —
        // the prep doc's "unconfigured" stance, not a guess.
        if config.data_layer.is_empty() {
            return Vec::new();
        }
        let scope = build_scope(config);
        let severity = config.severity(self.name(), self.default_severity());

        // A module mock hoisted to file scope is copied onto every case in the file, so the
        // same (file, mock span) appears on each. Dedupe by that site so a data-layer mock is
        // reported once, not once per test, keeping the first occurrence.
        let mut seen: std::collections::HashSet<(String, u32)> = std::collections::HashSet::new();
        let mut findings: Vec<Finding> = model
            .test_cases
            .iter()
            .filter(|case| scope.is_match(&case.file))
            .flat_map(|case| case.mocks.iter().map(move |mock| (case, mock)))
            .filter(|(_, mock)| mock.kind == MockKind::Module)
            .filter(|(case, mock)| seen.insert((case.file.clone(), mock.span.start)))
            .filter_map(|(case, mock)| {
                matched_entry(mock, &config.data_layer)
                    .map(|entry| self.finding(case, mock, entry, severity))
            })
            .collect();

        sort(&mut findings);
        number_collisions(&mut findings);
        findings
    }
}

impl DataLayerMock {
    /// The finding for one data-layer module mock: located at the mock call, the resolved
    /// data-layer module as a related location (when the target resolved), the specifier and
    /// the resolved target in the evidence, and a repair that points at MSW rather than
    /// deletion.
    fn finding(&self, case: &TestCase, mock: &Mock, entry: &str, severity: Severity) -> Finding {
        let specifier = mock.target.clone().unwrap_or_default();
        let title = case.title.clone().unwrap_or_else(|| "<dynamic>".to_owned());
        let mut related = Vec::new();
        let mut evidence = vec![
            Evidence::new("entry", entry.to_owned()),
            Evidence::new("specifier", specifier.clone()),
        ];
        // When the target resolved, the finding points at the data-layer module and records
        // the resolved path as the mock's true target; a package-name match has no project
        // file to point at, so its target is the specifier as written.
        if let Some(resolved) = &mock.resolved {
            related.push(RelatedLocation {
                label: "data layer".to_owned(),
                location: Location {
                    file: resolved.clone(),
                    line: 1,
                    column: 1,
                },
            });
            evidence.push(Evidence::new("target", resolved.clone()));
        } else {
            evidence.push(Evidence::new("target", specifier.clone()));
        }
        evidence.sort_by(|a, b| a.key.cmp(&b.key));

        let target = mock.resolved.clone().unwrap_or_else(|| specifier.clone());
        Finding {
            rule: self.name(),
            severity,
            outcome: Outcome::Finding,
            location: Location::at(case.file.clone(), mock.span),
            related,
            message: format!("test {title:?} module-mocks the data layer ({target})"),
            evidence,
            explain: format!(
                "The test {title:?} replaces the data-layer module `{target}` \
                 (mocked as `{specifier}`) with `vi.mock`, so it proves the mock, not the flow: \
                 the real client, its query and the data it returns never run. In an \
                 integration or container spec, let the data layer run and mock the network \
                 underneath it with MSW so the test exercises the query it claims to. Keep \
                 the test — move the seam from the module down to the network.",
            ),
            anchor: anchor_of(case, mock, &specifier),
            test: in_body(case, mock).then(|| TestPath::of(case)),
            ordinal: 0,
            proposal: None,
        }
    }
}

/// Whether the mock is written inside the case's body (a `vi.doMock` in a test), so the
/// finding is anchored at that test; a mock hoisted to file scope reaches every test in the
/// file and is anchored at the file.
const fn in_body(case: &TestCase, mock: &Mock) -> bool {
    case.span.start <= mock.span.start && mock.span.end <= case.span.end
}

/// The semantic anchor of a mock site: the specifier as written, prefixed with the suite
/// path and title of the case whose body holds it when the mock is written inside a test
/// (`vi.doMock` in a body) rather than hoisted to file scope.
fn anchor_of(case: &TestCase, mock: &Mock, specifier: &str) -> String {
    if !in_body(case, mock) {
        return format!("mock:{specifier}");
    }
    let mut parts = case.suite_path.clone();
    parts.push(
        case.title
            .clone()
            .unwrap_or_else(|| format!("<dynamic>:{}", case.span.line)),
    );
    format!("{} › mock:{specifier}", parts.join(" › "))
}

/// The `dataLayer` entry a mock matches, or `None` when none does. A resolved path is
/// compared extension-insensitively against every entry; the specifier as written matches
/// an entry it equals or a subpath of it, so a package-name entry matches an unresolved
/// external mock and the package's subpath exports.
fn matched_entry<'a>(mock: &Mock, data_layer: &'a [String]) -> Option<&'a str> {
    data_layer.iter().find_map(|entry| {
        let entry = entry.as_str();
        let by_path = mock
            .resolved
            .as_deref()
            .is_some_and(|resolved| path_matches(resolved, entry));
        let by_specifier = mock
            .target
            .as_deref()
            .is_some_and(|specifier| specifier_matches(specifier, entry));
        (by_path || by_specifier).then_some(entry)
    })
}

/// Whether a specifier as written names a `dataLayer` entry or a subpath of it:
/// `@acme/core-ai-kit` and `@acme/core-ai-kit/providers` both match the entry
/// `@acme/core-ai-kit`; `@acme/core-ai-kit-extras` does not.
fn specifier_matches(specifier: &str, entry: &str) -> bool {
    specifier == entry
        || specifier
            .strip_prefix(entry)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// Whether a resolved repo-relative path matches a `dataLayer` entry: equal after stripping a
/// module extension from the path, so `src/supabaseClient.ts` matches `src/supabaseClient`
/// and an entry written with its extension still matches exactly.
fn path_matches(resolved: &str, entry: &str) -> bool {
    resolved == entry || strip_extension(resolved) == entry
}

/// A repo-relative path with its module extension removed, or the path unchanged when it has
/// none the model writes.
fn strip_extension(path: &str) -> &str {
    for ext in MODULE_EXTENSIONS {
        if let Some(stem) = path.strip_suffix(ext) {
            return stem;
        }
    }
    path
}

/// The compiled scope: the configured `dataLayerScope` globs, or [`DEFAULT_SCOPE`] when the
/// config sets none. A glob that does not compile is skipped rather than aborting the run —
/// the model already diagnoses a bad glob elsewhere, and the rule fails safe (a skipped glob
/// narrows the scope, never widens it).
fn build_scope(config: &Config) -> globset::GlobSet {
    let patterns: Vec<&str> = if config.data_layer_scope.is_empty() {
        DEFAULT_SCOPE.to_vec()
    } else {
        config.data_layer_scope.iter().map(String::as_str).collect()
    };
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        if let Ok(glob) = Glob::new(pattern) {
            builder.add(glob);
        }
    }
    builder
        .build()
        .unwrap_or_else(|_| globset::GlobSet::empty())
}
