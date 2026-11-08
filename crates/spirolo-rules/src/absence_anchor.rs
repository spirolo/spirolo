//! `absence-anchor`: a render test whose every DOM read asserts absence, with no positive DOM
//! assertion and no throwing query to anchor the render.
//!
//! An absence read is a *negated* jest-dom matcher (`not.toBeInTheDocument`,
//! `not.toBeVisible`, `not.toHaveTextContent` and kin; all but the first only on a subject
//! that survives an empty render, such as `container`), a plain `toBeEmptyDOMElement()`, or
//! the Testing Library absence idiom: a `queryBy*` / `queryAllBy*` subject, which yields
//! `null` or `[]` for a missing element, under `toBeNull()`, `toBeUndefined()`, `toBeFalsy()`,
//! `not.toBeTruthy()`, `toHaveLength(0)`, `toEqual([])` or `toStrictEqual([])`. An anchor is
//! a plain jest-dom matcher (`toBeInTheDocument`, `toBeVisible`, `toHaveTextContent`,
//! `toHaveAttribute`, ... the table on [`DOM_MATCHERS`]), a negated one but
//! `not.toBeInTheDocument` on an element read off the render (`expect(container
//! .querySelector('a')).not.toHaveAttribute('href')`: jest-dom throws on the `null` an empty
//! render leaves before it negates), a `not.toBeEmptyDOMElement()`, a
//! plain call assertion on a spy (`toHaveBeenCalled`, `toHaveBeenCalledWith`,
//! `toHaveBeenCalledTimes(n)` with `n` not the literal `0`, ... [`SPY_MATCHERS`]: a crashed
//! render made no call), or a throwing Testing Library query (`getBy*`, `findBy*`,
//! `within(...).getBy*`) anywhere in the body, including inside `waitFor(…)` callbacks. An
//! anchor in a hook of the enclosing `describe` (or of an outer one, or at file level)
//! anchors every test under it, since the hook's throw fails the test. A test is flagged
//! when it has at least one absence read and no anchor. Only `test(…)` / `it(…)` rows are
//! judged, and never a Playwright one (it drives a page, not a render); a hook, suite,
//! benchmark or step row is never a finding.
//!
//! Not an anchor, by design: a negated call assertion (`not.toHaveBeenCalled()` holds when
//! nothing rendered), a count of the literal `0`, and a hook in a setup file (outside the
//! model). Those hold against an empty body whatever rendered, so they leave the finding
//! standing.
//!
//! Left unread, by design: every other assertion — a value-bearing matcher on a plain
//! subject (`expect(container.innerHTML).toBe('<div…>')`, `expect(captured).toHaveLength(1)`,
//! `expect(window.location.href).toBe('/account')`), a value-less matcher on a plain subject,
//! a matcher the tables do not know. The model cannot tell a value the render produced
//! through a side effect from one the test built itself, and the M2 crash-the-render oracle
//! showed that such an assertion usually anchors the render after all (nine of the rule's
//! twenty-three findings on one corpus monorepo, two of fifty-eight on another, every one
//! of them carrying exactly such an assertion). So a test that carries any assertion the rule
//! does not read is left alone: the finding is made only when every assertion in the test is
//! an absence read or holds against an empty body regardless. Precision over recall — a test
//! whose extra assertion is on a value it built itself is a false negative the rule accepts.
//!
//! The trap it catches: React 19 reports an uncaught render error to `window.reportError`
//! instead of rethrowing it (prep doc addendum A7), so a component that crashes on mount
//! leaves an empty body without failing the test — unless the runner listens for the window
//! error event. A test whose only DOM assertion is `expect(x).not.toBeInTheDocument()` then
//! passes trivially against that empty body: it proves nothing about the render it claims to
//! guard. A throwing query (which raises when the element is missing) or a positive DOM
//! assertion (which fails against an empty body) is a real anchor, so either silences the rule.
//!
//! Severity `warn`: a reading a maintainer ratchets, not a build-breaker. The finding names
//! the absence reads in its evidence (`not.toBeInTheDocument`, `queryBy*.toBeNull`) and
//! repairs by adding an anchor, never by deleting the negation.

use std::collections::{HashMap, HashSet};

use spirolo_model::{Assertion, Enclosing, FlakeNames, Model, TestCase, TestKind, ThrowingQuery};

use crate::finding::{number_collisions, sort, Evidence, Finding, Location, Outcome, TestPath};
use crate::{tests, Config, Family, Rule, Severity};

/// The jest-dom matchers that anchor a render when written plain: each asserts something an
/// empty body does not have, so it fails after a crashed render. Negated, each is an absence
/// read instead. The full bearing table, with `not.` marking a negated chain and `query`
/// marking a `queryBy*` / `queryAllBy*` subject:
///
/// | assertion | bearing |
/// |---|---|
/// | `toBeInTheDocument`, `toBeVisible`, `toHaveTextContent`, `toHaveAttribute`, `toHaveValue`, `toHaveDisplayValue`, `toHaveClass`, `toHaveStyle`, `toHaveFocus`, `toHaveRole`, `toContainElement`, `toContainHTML`, `toBeChecked`, `toBePartiallyChecked`, `toBeDisabled`, `toBeEnabled`, `toBeRequired`, `toBeValid`, `toBeInvalid`, `toHaveFormValues`, `toHaveAccessibleName`, `toHaveAccessibleDescription`, `toHaveAccessibleErrorMessage`, `toHaveDescription`, `toHaveErrorMessage` | anchor |
/// | `not.` any of the above but `toBeInTheDocument`, on a call or a local bound to one (`container.querySelector('a')`, `gridcell(day)`) | anchor (jest-dom checks the subject is an element first, and throws on the `null` an empty render leaves) |
/// | `not.` any of the above on anything else (`container`, `document.body`), and `not.toBeInTheDocument` on anything | absence |
/// | `not.toBeEmptyDOMElement` | anchor (an empty body has no content to fail it otherwise) |
/// | `toBeEmptyDOMElement` | absence (an empty container after a crash is empty too) |
/// | `toBeNull`, `toBeUndefined`, `toBeFalsy`, `not.toBeTruthy`, `toHaveLength(0)`, `toEqual([])`, `toStrictEqual([])` on `query` | absence |
/// | `not.toHaveBeenCalled`, `not.toHaveBeenCalledWith(…)`, `toHaveBeenCalledTimes(0)` | inert (holds against an empty body) |
/// | `not.toBeNull`, `toBeTruthy`, `toHaveLength(n)` on `query` | unread (presence, but not a jest-dom read) |
/// | `toBe`, `toEqual`, `toMatchSnapshot`, every other matcher on a plain subject | unread |
///
/// The list is the jest-dom matcher surface (`@testing-library/jest-dom`): presence,
/// visibility, text, form state, attributes, focus and layout, less the one emptiness
/// matcher ([`EMPTINESS_MATCHERS`]). A matcher outside both, a plain Vitest matcher or a
/// custom one, is not treated as a DOM read.
const DOM_MATCHERS: &[&str] = &[
    "toBeChecked",
    "toBeDisabled",
    "toBeEnabled",
    "toBeInTheDocument",
    "toBeInvalid",
    "toBePartiallyChecked",
    "toBeRequired",
    "toBeValid",
    "toBeVisible",
    "toContainElement",
    "toContainHTML",
    "toHaveAccessibleDescription",
    "toHaveAccessibleErrorMessage",
    "toHaveAccessibleName",
    "toHaveAttribute",
    "toHaveClass",
    "toHaveDescription",
    "toHaveDisplayValue",
    "toHaveErrorMessage",
    "toHaveFocus",
    "toHaveFormValues",
    "toHaveRole",
    "toHaveStyle",
    "toHaveTextContent",
    "toHaveValue",
];

/// The jest-dom matcher that asserts emptiness: `toBeEmptyDOMElement()` passes on an empty
/// container after a crash as readily as on the render it means to test, so written plain
/// it is an absence read, and only its negation anchors. The inverse of [`DOM_MATCHERS`].
const EMPTINESS_MATCHERS: &[&str] = &["toBeEmptyDOMElement"];

/// The call, return and resolution matchers on a spy that anchor a render when written
/// plain: a component that crashed on mount made no call, so `expect(onClose)
/// .toHaveBeenCalled()` fails against the empty body. Vitest's spellings and Jest's aliases.
/// Negated (`not.toHaveBeenCalled()`) none of these anchors: the negation holds when
/// nothing rendered. The count spellings are in [`SPY_COUNT_MATCHERS`], since a count of
/// the literal `0` holds when nothing rendered too.
///
/// The rule cannot tell a spy the render invoked from one the test body called itself, or
/// one a hook called: every plain call assertion is read as an anchor.
const SPY_MATCHERS: &[&str] = &[
    "lastCalledWith",
    "lastReturnedWith",
    "nthCalledWith",
    "nthReturnedWith",
    "toBeCalled",
    "toBeCalledWith",
    "toHaveBeenCalled",
    "toHaveBeenCalledAfter",
    "toHaveBeenCalledBefore",
    "toHaveBeenCalledExactlyOnceWith",
    "toHaveBeenCalledOnce",
    "toHaveBeenCalledWith",
    "toHaveBeenLastCalledWith",
    "toHaveBeenNthCalledWith",
    "toHaveLastResolvedWith",
    "toHaveLastReturnedWith",
    "toHaveNthResolvedWith",
    "toHaveNthReturnedWith",
    "toHaveResolved",
    "toHaveResolvedWith",
    "toHaveReturned",
    "toHaveReturnedWith",
    "toReturn",
    "toReturnWith",
];

/// The spy count matchers: an anchor when plain with any count but the literal `0`
/// (`toHaveBeenCalledTimes(1)`, `toHaveBeenCalledTimes(n)`), never when the count is `0` or
/// the chain is negated.
const SPY_COUNT_MATCHERS: &[&str] = &[
    "toBeCalledTimes",
    "toHaveBeenCalledTimes",
    "toHaveResolvedTimes",
    "toHaveReturnedTimes",
    "toReturnTimes",
];

/// The `absence-anchor` rule. See the module docs for the semantics.
pub struct AbsenceAnchor;

impl Rule for AbsenceAnchor {
    fn name(&self) -> &'static str {
        "absence-anchor"
    }

    fn family(&self) -> Family {
        Family::Audit
    }

    fn subject(&self) -> crate::Subject {
        crate::Subject::Tests
    }

    fn summary(&self) -> &'static str {
        "A render test asserts only that things are absent from the DOM, with no positive assertion proving the render happened at all."
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn run(&self, model: &Model, config: &Config) -> Vec<Finding> {
        let severity = config.severity(self.name(), self.default_severity());
        let helpers = ThrowingHelpers::of(model);
        let hooks = anchoring_hooks(model, &helpers);
        let mut findings: Vec<Finding> = tests(model)
            .filter(|case| {
                !is_playwright(case)
                    && is_unanchored_negation(case)
                    && !helpers.called_by(case)
                    && !hooks.anchor(case)
            })
            .map(|case| self.finding(case, severity))
            .collect();

        sort(&mut findings);
        number_collisions(&mut findings);
        findings
    }
}

/// Whether a row is a Playwright test: it drives a page (`page.goto`, `page.click`) that
/// fails on its own when the app is broken, so it is no render test, and its absence reads
/// (`expect(locator).not.toBeVisible()`) are out of the rule's scope.
fn is_playwright(case: &TestCase) -> bool {
    case.framework.as_deref() == Some("playwright")
}

/// Whether a matcher name is a jest-dom DOM read, anchoring or emptiness.
fn is_dom_matcher(matcher: &str) -> bool {
    DOM_MATCHERS.binary_search(&matcher).is_ok() || EMPTINESS_MATCHERS.contains(&matcher)
}

/// How one assertion bears on the render it sits after.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Bearing {
    /// Fails against an empty body: a non-negated jest-dom matcher, a plain call assertion
    /// on a spy.
    Anchor,
    /// Holds against an empty body and is the DOM read the rule is about: a negated jest-dom
    /// matcher, the emptiness matcher, or the `queryBy*` absence idiom.
    Absence,
    /// Holds against an empty body whatever rendered, without being a DOM read: a negated
    /// call assertion, a count of the literal `0`. Leaves a finding standing.
    Inert,
    /// Anything the rule does not read (`toBe`, `toEqual`, `toBeTruthy` on a plain subject,
    /// a matcher the tables do not know): it may or may not anchor the render, so its
    /// presence silences the rule.
    Unread,
}

/// The bearing of one assertion, per the table on [`DOM_MATCHERS`]: a jest-dom matcher
/// decides, negated for absence and plain for an anchor, the emptiness matcher the other
/// way round; a plain matcher on a `queryBy*` subject is an absence read when it spells
/// the idiom; a spy matcher anchors when plain with a non-zero count and is inert when
/// negated or counting `0`; everything else is unread.
fn bearing(assertion: &Assertion) -> Bearing {
    let Some(matcher) = assertion.matcher.as_deref() else {
        return Bearing::Unread;
    };
    // `not.toBeEmptyDOMElement` anchors; `toBeEmptyDOMElement` is absence.
    let asserts_presence = assertion.negated == EMPTINESS_MATCHERS.contains(&matcher);
    if is_dom_matcher(matcher) {
        return if asserts_presence || checks_element(matcher, assertion) {
            Bearing::Anchor
        } else {
            Bearing::Absence
        };
    }
    if assertion.subject_is_query
        && is_query_absence(matcher, assertion.negated, assertion.argument.as_deref())
    {
        return Bearing::Absence;
    }
    if is_spy_matcher(matcher) {
        return if !assertion.negated && is_spy_anchor(matcher, assertion.argument.as_deref()) {
            Bearing::Anchor
        } else {
            Bearing::Inert
        };
    }
    Bearing::Unread
}

/// Whether a negated jest-dom matcher throws on an empty render: every one but
/// `toBeInTheDocument` (which lets `null` through when negated) runs `checkHtmlElement` on
/// its subject before the negation, so on an element read off the render (a call, an optional
/// call chain, a nullable DOM child property, or a local bound to one), nullish once
/// the render produced nothing, it fails. On a subject that survives an empty render
/// (`container`, `document.body`) it holds, and stays an absence read.
fn checks_element(matcher: &str, assertion: &Assertion) -> bool {
    assertion.negated && assertion.subject_is_call && matcher != "toBeInTheDocument"
}

/// Whether a matcher is a spy matcher at all, counting or not.
fn is_spy_matcher(matcher: &str) -> bool {
    SPY_MATCHERS.binary_search(&matcher).is_ok()
        || SPY_COUNT_MATCHERS.binary_search(&matcher).is_ok()
}

/// Whether a plain matcher is a spy assertion the render must have satisfied: one of
/// [`SPY_MATCHERS`], or a [`SPY_COUNT_MATCHERS`] entry whose count is not the literal `0`.
fn is_spy_anchor(matcher: &str, argument: Option<&str>) -> bool {
    SPY_MATCHERS.binary_search(&matcher).is_ok()
        || (SPY_COUNT_MATCHERS.binary_search(&matcher).is_ok() && argument != Some("0"))
}

/// Whether a matcher on a `queryBy*` / `queryAllBy*` subject spells the Testing Library
/// absence idiom: `toBeNull()`, `toBeUndefined()`, `toBeFalsy()` or `not.toBeTruthy()` on
/// the `null` a missing element yields, `toHaveLength(0)`, `toEqual([])` or
/// `toStrictEqual([])` on the empty array `queryAllBy*` yields. Each holds against an empty
/// body as readily as `not.toBeInTheDocument`. The presence spellings (`not.toBeNull`,
/// `toBeTruthy`, `toHaveLength(2)`) name something an empty body lacks and are not absence.
fn is_query_absence(matcher: &str, negated: bool, argument: Option<&str>) -> bool {
    match (matcher, negated) {
        ("toBeNull" | "toBeUndefined" | "toBeFalsy", false) | ("toBeTruthy", true) => true,
        ("toHaveLength", false) => argument == Some("0"),
        ("toEqual" | "toStrictEqual", false) => argument == Some("[]"),
        _ => false,
    }
}

/// A test is an unanchored absence when it has at least one absence read, no anchoring
/// assertion, no throwing query in its own body, and no assertion the rule leaves unread:
/// every assertion is then an absence read or inert, all of them trivially satisfiable
/// against an empty body after a crashed render, unless a hook around the test anchors it
/// ([`AnchoringHooks`]). One unread assertion, whatever it is on, silences the rule: it may
/// be the anchor the model cannot see.
fn is_unanchored_negation(case: &TestCase) -> bool {
    let bearings: Vec<Bearing> = case.assertions.iter().map(bearing).collect();
    let has_absence = bearings.contains(&Bearing::Absence);
    let all_read = !bearings.contains(&Bearing::Unread);
    has_absence && all_read && !anchors(case)
}

/// Whether a row's own body anchors a render: a throwing query, or an assertion that bears
/// as an anchor.
fn anchors(row: &TestCase) -> bool {
    !row.throwing_queries.is_empty() || row.assertions.iter().any(|a| bearing(a) == Bearing::Anchor)
}

/// The hook rows whose body anchors a render, as (file, suite path): a `beforeEach` that
/// renders and then calls `screen.getByRole(…)` throws for every test under it when the
/// render crashed, so it anchors each of them.
///
/// A hook anchors the tests of its own `describe` and of every `describe` nested in it, in
/// the same file; a module-level hook anchors the whole file. The model does not record
/// which hook a row is, so `beforeAll`, `beforeEach`, `afterEach` and `afterAll` count
/// alike: the first three fail the test when they throw, and a throwing query in an
/// `afterAll` (whose failure lands on the suite, not the test) is not a spelling seen in
/// practice. A setup file's hooks are outside the model and never anchor.
struct AnchoringHooks<'a> {
    scopes: Vec<(&'a str, &'a [String])>,
}

impl AnchoringHooks<'_> {
    /// Whether one of the hooks encloses `case`: same file, suite path a prefix of the
    /// case's.
    fn anchor(&self, case: &TestCase) -> bool {
        self.scopes
            .iter()
            .any(|(file, path)| *file == case.file && case.suite_path.starts_with(path))
    }
}

/// Every anchoring hook in the model: one whose own body anchors, or that calls a helper
/// that runs a throwing query.
fn anchoring_hooks<'m>(model: &'m Model, helpers: &ThrowingHelpers<'_>) -> AnchoringHooks<'m> {
    AnchoringHooks {
        scopes: model
            .test_cases
            .iter()
            .filter(|row| row.kind == TestKind::Hook && (anchors(row) || helpers.called_by(row)))
            .map(|row| (row.file.as_str(), row.suite_path.as_slice()))
            .collect(),
    }
}

/// The helpers whose body runs a throwing query, by the test file that can call them: a
/// function of the file's own (`const openPlainForm = async () => { … await
/// screen.findByTestId(…) … }`), or of a module it imports through a relative or tsconfig
/// path (`dateButton` in `test/elements.ts`, `return screen.getByRole(…)`), that calls a
/// `getBy*` / `findBy*` directly or through another function of the same module. A row that references one in its body calls it, and the
/// query throws when the render came up empty, so the row is anchored. A helper that only
/// renders (`const renderNav = () => render(…)`) runs no query and anchors nothing.
///
/// Read from the model's per-file name graph ([`FlakeNames`]): names as written, never by
/// binding, so two functions of one name are one helper.
struct ThrowingHelpers<'m> {
    /// Per test file, its references with their byte offsets.
    uses: HashMap<&'m str, &'m [(u32, String)]>,
    /// Per test file, the names of the throwing helpers in it and in the modules it imports.
    names: HashMap<&'m str, HashSet<&'m str>>,
}

impl<'m> ThrowingHelpers<'m> {
    fn of(model: &'m Model) -> Self {
        let graphs: HashMap<&str, &FlakeNames> = model
            .flake_names
            .iter()
            .map(|n| (n.file.as_str(), n))
            .collect();
        let imports: HashMap<&str, Vec<&str>> = model
            .modules
            .iter()
            .map(|m| {
                let resolved = m.imports.iter().filter_map(|i| i.resolved.as_deref());
                (m.path.as_str(), resolved.collect())
            })
            .collect();
        let mut uses = HashMap::new();
        let mut names = HashMap::new();
        for file in model.flake_names.iter().filter(|n| !n.uses.is_empty()) {
            uses.insert(file.file.as_str(), file.uses.as_slice());
            let imported = imports
                .get(file.file.as_str())
                .into_iter()
                .flatten()
                .filter_map(|path| graphs.get(path));
            let reach = std::iter::once(file)
                .chain(imported.copied())
                .flat_map(throwing_functions)
                .collect();
            names.insert(file.file.as_str(), reach);
        }
        Self { uses, names }
    }

    /// Whether `row` references one of its file's throwing helpers inside its own span.
    fn called_by(&self, row: &TestCase) -> bool {
        let (Some(uses), Some(names)) = (
            self.uses.get(row.file.as_str()),
            self.names.get(row.file.as_str()),
        ) else {
            return false;
        };
        uses.iter().any(|(offset, name)| {
            (row.span.start..row.span.end).contains(offset) && names.contains(name.as_str())
        })
    }
}

/// The named functions of one file whose body runs a Testing Library throwing query
/// (`getByRole`, not a `getByPath` utility), directly or through another of the same file's
/// named functions.
fn throwing_functions(file: &FlakeNames) -> HashSet<&str> {
    let mut found: HashSet<&str> = HashSet::new();
    loop {
        let before = found.len();
        for (enclosing, called) in &file.calls {
            let Enclosing::Named(name) = enclosing else {
                continue;
            };
            if called
                .iter()
                .any(|c| ThrowingQuery::is_testing_library(c) || found.contains(c.as_str()))
            {
                found.insert(name.as_str());
            }
        }
        if found.len() == before {
            return found;
        }
    }
}

impl AbsenceAnchor {
    /// The finding for one unanchored-negation test: located at the test call, naming the
    /// negated DOM matchers in the evidence and repairing by adding an anchor.
    fn finding(&self, case: &TestCase, severity: Severity) -> Finding {
        let matchers = negated_dom_matchers(case);
        let title = case.title.clone().unwrap_or_else(|| "<dynamic>".to_owned());
        Finding {
            rule: self.name(),
            severity,
            outcome: Outcome::Finding,
            location: Location::at(case.file.clone(), case.span),
            related: Vec::new(),
            message: format!(
                "test {title:?} asserts only that DOM is absent with no anchor ({})",
                matchers.join(", ")
            ),
            evidence: vec![Evidence::new("matchers", matchers.join(", "))],
            explain: format!(
                "The test {title:?} asserts only that DOM is absent \
                 ({}) with nothing to anchor the render: no plain jest-dom assertion \
                 (`toBeInTheDocument`, `toHaveTextContent`, `not.toBeEmptyDOMElement`), no \
                 throwing query (`getBy*`, `findBy*`) and no plain call assertion on a spy \
                 (`toHaveBeenCalled`, `toHaveBeenCalledWith`, a count other than 0), in the \
                 test's body or in a hook of an enclosing describe, and no other assertion \
                 at all besides ones that hold against an empty body regardless (a negated \
                 call assertion, a count of 0). A render error can leave an empty body if \
                 the framework reports it to `window.reportError` and the runner does not \
                 fail the test. Absence reads alone cannot distinguish that from a \
                 successful render: a negated jest-dom matcher and `toBeNull()` on a \
                 `queryBy*` result alike. If the component should render content, anchor \
                 it first: query for what the component *does* show \
                 with a throwing `getBy*`/`findBy*`, or assert a positive `toBeInTheDocument` \
                 on an element that must exist, then assert the absence. If an empty \
                 render is intentional, inspect the render setup and error handling \
                 instead; a presence assertion may not apply. Keep the absence read. \
                 This finding alone does not prove the test would pass after a render \
                 failure.",
                matchers.join(", ")
            ),
            anchor: anchor_of(case),
            test: Some(TestPath::of(case)),
            ordinal: 0,
            proposal: None,
        }
    }
}

/// The distinct absence reads in a test, sorted, spelled the way each reads in source: a
/// negated jest-dom matcher with its leading `not.` (`not.toBeInTheDocument`), the query
/// idiom with its subject and literal (`queryBy*.toBeNull`, `queryBy*.toHaveLength(0)`).
fn negated_dom_matchers(case: &TestCase) -> Vec<String> {
    let mut names: Vec<String> = case
        .assertions
        .iter()
        .filter(|a| bearing(a) == Bearing::Absence)
        .map(spelled)
        .collect();
    names.sort();
    names.dedup();
    names
}

/// One absence read as it reads in source, for the evidence and the message.
fn spelled(assertion: &Assertion) -> String {
    let matcher = assertion.matcher.as_deref().unwrap_or_default();
    let not = if assertion.negated { "not." } else { "" };
    if assertion.subject_is_query && !is_dom_matcher(matcher) {
        let argument = assertion
            .argument
            .as_deref()
            .map(|a| format!("({a})"))
            .unwrap_or_default();
        format!("queryBy*.{not}{matcher}{argument}")
    } else {
        format!("{not}{matcher}")
    }
}

/// The semantic anchor for a case: its suite path joined with its title, the shape the other
/// rules use, so the identity is stable across runs and unrelated edits.
fn anchor_of(case: &TestCase) -> String {
    let mut parts = case.suite_path.clone();
    parts.push(
        case.title
            .clone()
            .unwrap_or_else(|| format!("<dynamic>:{}", case.span.line)),
    );
    parts.join(" › ")
}
