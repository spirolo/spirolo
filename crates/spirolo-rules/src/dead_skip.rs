//! `dead-skip`: skips a suite carries that nobody is watching — an unconditional skip with
//! no linked issue, a CI-scoped `skipIf`, or an `only` that silently halves the run.
//!
//! The rule reads `TestCase.skip` and nothing else. It reports three things a maintainer
//! cannot see by memory:
//!
//! - an **unconditional** skip (`it.skip`, `.todo`, `.fixme`, a `skip(true, …)` whose
//!   condition is the literal `true`, or a `runIf(false)`) with **no issue reference** — a
//!   `#123`, an issue or pull-request URL, or a `KEY-123` ticket — in its title, its reason, a
//!   trailing comment or the comment lines just above it ([`Skip::comments`]), so a skip that
//!   was meant to be temporary but lost its ticket is a finding, not a memory. A
//!   `test.skip()` / `test.skip(true, …)` written as a statement inside the test body
//!   usually sits under an `if` the rule cannot see, so it is left alone (its modifier span
//!   sits on a later line than the definition — see `is_definition_level`).
//! - a **CI-scoped** skip whose condition reads an environment variable (`process.env.CI`,
//!   `import.meta.env.X`) — a `skipIf` / `runIf`, or a Playwright `test.skip(cond, why)` /
//!   `test.fixme(cond, why)` annotation, which is classified by its condition the same way:
//!   a whole suite can be silently halved in CI this way, so the rule names the variable and
//!   reports it at `info`.
//! - an **`only`**, at error severity: `.only` skips every case without it, so a stray
//!   `only` turns a green run into a run of one test.
//!
//! A skip whose condition is neither an environment read nor a literal (`browserName ===
//! 'webkit'`, `features.beta`) is *conditional*: the rule cannot call it dead or CI-scoped,
//! so it is not flagged.
//!
//! Beside the flagged rows the rule **lists every** `skip`, `todo`, `fixme`, `skipIf` and
//! `runIf` it saw, one [`Outcome::Listed`] row per site at `info`, with the modifier, the
//! condition, how it was classified (`scope`), the reason and whether it is tracked — so a
//! whole suite silently halved in CI is visible as a count, not a memory. A listing row is
//! an inventory line, never a finding: it does not fail the build, is not baselined, and is
//! not sampled for precision. Its identity is `listed:` plus the case's anchor, so the
//! flagged rows keep the ids they had.
//!
//! No blame age in M2 (no git access): the rule is a pure query over the model, and a
//! skip's age is an M4 join beside the `Run` table.

use spirolo_model::{Model, Skip, SkipKind, TestCase};

use crate::finding::{number_collisions, sort, Evidence, Finding, Location, Outcome, TestPath};
use crate::{Config, Family, Rule, Severity};

/// The `dead-skip` rule. See the module docs for the semantics.
pub struct DeadSkip;

impl Rule for DeadSkip {
    fn name(&self) -> &'static str {
        "dead-skip"
    }

    fn family(&self) -> Family {
        Family::Gate
    }

    fn subject(&self) -> crate::Subject {
        crate::Subject::Tests
    }

    fn summary(&self) -> &'static str {
        "A skipped or focused test disagrees with the suite that is believed to run, with no linked issue or visible condition watching it."
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn run(&self, model: &Model, config: &Config) -> Vec<Finding> {
        let severity = config.severity(self.name(), self.default_severity());
        let mut findings = Vec::new();
        for case in &model.test_cases {
            let Some(skip) = case.skip.as_ref() else {
                continue;
            };
            if let Some(listing) = self.listing(case, skip) {
                findings.push(listing);
            }
            if let Some(flagged) = self.judge(case, severity) {
                findings.push(flagged);
            }
        }

        sort(&mut findings);
        number_collisions(&mut findings);
        findings
    }
}

impl DeadSkip {
    /// Judge one case's skip, or `None` when the case carries no skip the rule flags (no
    /// skip at all, a `fails`/`slow`, a conditional whose condition is neither env-scoped nor
    /// a literal, a bare in-body `test.skip()` annotation, or an unconditional skip that
    /// carries an issue reference).
    fn judge(&self, case: &TestCase, severity: Severity) -> Option<Finding> {
        let skip = case.skip.as_ref()?;
        match classify(skip) {
            Verdict::Only => Some(self.only_finding(case, skip)),
            Verdict::CiScoped(variable) => Some(self.ci_scoped_finding(case, skip, &variable)),
            Verdict::Unconditional => {
                // A `test.skip()` or `test.skip(true, …)` written as a statement inside the
                // test body (its modifier span sits on a later line than the definition)
                // usually sits under an `if` guard the rule cannot see — for example
                // `if (rowCount === 0) { test.skip(true, 'No rows available') }` — so it is
                // a conditional early-return, not a dead definition skip. Flagging it is a
                // false positive, so the rule reports an unconditional skip only as a
                // definition-level modifier (`it.skip('t', …)`, on the definition's own
                // line). CI-scoped and `only` need no such guard.
                if !is_definition_level(case, skip) || has_issue_reference(case, skip) {
                    None
                } else {
                    Some(self.unconditional_finding(case, skip, severity))
                }
            }
            Verdict::Conditional | Verdict::Ignore => None,
        }
    }

    /// The listing row for one skip site — every `skip`, `todo`, `fixme`, `skipIf` and
    /// `runIf`, whatever its condition — or `None` for a modifier that is not a skip
    /// (`only`, `fails`, `slow`). Info, [`Outcome::Listed`], anchored apart from the
    /// flagged row the same site may also produce.
    fn listing(&self, case: &TestCase, skip: &Skip) -> Option<Finding> {
        if !matches!(
            skip.kind,
            SkipKind::Skip | SkipKind::Todo | SkipKind::Fixme | SkipKind::SkipIf | SkipKind::RunIf
        ) {
            return None;
        }
        let kind = modifier_word(skip.kind);
        let title = title_of(case);
        let condition = skip.condition.as_deref().unwrap_or("none");
        let reason = skip.reason.as_deref().unwrap_or("none");
        let tracked = if has_issue_reference(case, skip) {
            "yes"
        } else {
            "no"
        };
        let (scope, how) = match classify(skip) {
            Verdict::CiScoped(variable) => ("ci", format!("CI-scoped on {variable}")),
            Verdict::Unconditional if is_definition_level(case, skip) => {
                ("unconditional", "unconditional".to_owned())
            }
            // An in-body `test.skip()` / `test.skip(true, …)`: a runtime early-return under
            // a guard the rule cannot see.
            Verdict::Unconditional => ("runtime", "skipped at runtime under a guard".to_owned()),
            Verdict::Ignore => (
                "open",
                "never skips (the literal opens the gate)".to_owned(),
            ),
            Verdict::Conditional | Verdict::Only => {
                ("conditional", format!("conditional on {condition}"))
            }
        };
        Some(Finding {
            rule: self.name(),
            severity: Severity::Info,
            outcome: Outcome::Listed,
            location: Location::at(case.file.clone(), skip.span),
            related: Vec::new(),
            message: format!("{kind} {title}: {how}"),
            evidence: vec![
                Evidence::new("condition", condition.to_owned()),
                Evidence::new("kind", "skip-listed"),
                Evidence::new("modifier", kind.to_owned()),
                Evidence::new("reason", reason.to_owned()),
                Evidence::new("scope", scope),
                Evidence::new("tracked", tracked),
            ],
            explain: format!(
                "Listed for the count: every skip, todo, fixme, skipIf and runIf is inventoried \
                 with its condition, so a suite silently halved in CI is a number and not a \
                 memory. This {kind} on {title} is {how}; it is tracked: {tracked}. A listing \
                 row is not a finding and never fails the build — the flagged rows beside it \
                 say what needs a ticket."
            ),
            anchor: format!("listed:{}", anchor_of(case)),
            test: Some(TestPath::of(case)),
            ordinal: 0,
            proposal: None,
        })
    }

    /// The finding for an unconditional skip with no linked issue: located at the skip, the
    /// kind and the missing reference named in the message and the explain.
    fn unconditional_finding(&self, case: &TestCase, skip: &Skip, severity: Severity) -> Finding {
        let kind = kind_word(skip.kind);
        let title = title_of(case);
        Finding {
            rule: self.name(),
            severity,
            outcome: Outcome::Finding,
            location: Location::at(case.file.clone(), skip.span),
            related: Vec::new(),
            message: format!("{kind} {title} has no linked issue"),
            evidence: vec![
                Evidence::new("kind", kind.to_owned()),
                Evidence::new("reference", "none"),
            ],
            explain: format!(
                "The {kind} on {title} is unconditional and names no issue (no `#123`, issue \
                 or pull-request URL or `KEY-123` ticket in its title, its reason, a trailing \
                 comment or the comment lines just above it), so nothing tracks when it should \
                 come back. Link the ticket that justifies the skip there, or fix the test and \
                 remove the skip (delete it if it is dead) — do not leave it unexplained.",
            ),
            anchor: anchor_of(case),
            test: Some(TestPath::of(case)),
            ordinal: 0,
            proposal: None,
        }
    }

    /// The finding for a CI-scoped `skipIf`/`runIf`: named at `info` (context, never a
    /// failure), the environment variable it reads in the message and the evidence.
    fn ci_scoped_finding(&self, case: &TestCase, skip: &Skip, variable: &str) -> Finding {
        let kind = kind_word(skip.kind);
        let title = title_of(case);
        let condition = skip.condition.as_deref().unwrap_or("");
        Finding {
            rule: self.name(),
            severity: Severity::Info,
            outcome: Outcome::Finding,
            location: Location::at(case.file.clone(), skip.span),
            related: Vec::new(),
            message: format!("{kind} {title} is CI-scoped on {variable}"),
            evidence: vec![
                Evidence::new("condition", condition.to_owned()),
                Evidence::new("kind", kind.to_owned()),
                Evidence::new("scope", "ci"),
                Evidence::new("variable", variable.to_owned()),
            ],
            explain: format!(
                "The {kind} on {title} reads the environment variable `{variable}` \
                 (`{condition}`), so this test runs in one environment and not another — a \
                 CI-scoped skip can silently halve the suite the moment CI is green. Confirm \
                 the environment split is deliberate and documented; if the test needs a \
                 service CI lacks, make CI provide it rather than skip the coverage.",
            ),
            anchor: anchor_of(case),
            test: Some(TestPath::of(case)),
            ordinal: 0,
            proposal: None,
        }
    }

    /// The finding for an `only`: always error severity — a stray `only` runs one test and
    /// passes the build on it.
    fn only_finding(&self, case: &TestCase, skip: &Skip) -> Finding {
        let title = title_of(case);
        Finding {
            rule: self.name(),
            severity: Severity::Error,
            outcome: Outcome::Finding,
            location: Location::at(case.file.clone(), skip.span),
            related: Vec::new(),
            message: format!("only on {title} skips every other case"),
            evidence: vec![Evidence::new("kind", "only".to_owned())],
            explain: format!(
                "`.only` on {title} makes the runner skip every case without `.only`, so the \
                 build passes on one test while the rest never run. Remove the `.only` before \
                 merging; it is almost always a debugging leftover, never a gate.",
            ),
            anchor: anchor_of(case),
            test: Some(TestPath::of(case)),
            ordinal: 0,
            proposal: None,
        }
    }
}

/// What the rule decides about one skip.
enum Verdict {
    /// An `.only`: reported at error.
    Only,
    /// A skip whose condition reads the named environment variable.
    CiScoped(String),
    /// An unconditional skip (`skip`, `todo`, `fixme`, a `skip(true)`, a `runIf(false)`):
    /// reported when it carries no issue reference.
    Unconditional,
    /// A skip whose condition is neither an environment read nor a literal: listed, never
    /// flagged.
    Conditional,
    /// Not a skip (`fails`, `slow`, a `skipIf(false)` that always runs).
    Ignore,
}

/// Classify one skip into the verdict the rule acts on.
///
/// A `skip`, `todo`, `fixme`, `skipIf` or `runIf` is classified by its condition: none is
/// unconditional; a literal that folds the skip closed (`skip(true, …)`, `runIf(false)`) is
/// unconditional too; a literal that folds it open (`skipIf(false)`, `runIf(true)`) is not a
/// skip; an environment read is CI-scoped; anything else is conditional. A Playwright
/// `test.skip(cond, why)` annotation carries its condition on a `Skip` kind and is read the
/// same way, so it is never mistaken for a definition-level `test.skip('t', fn)`.
fn classify(skip: &Skip) -> Verdict {
    let inverted = skip.kind == SkipKind::RunIf;
    match skip.kind {
        SkipKind::Only => Verdict::Only,
        // `fails` and `slow` change how a case runs, not whether it is dead.
        SkipKind::Fails | SkipKind::Slow => Verdict::Ignore,
        SkipKind::Skip | SkipKind::Todo | SkipKind::Fixme | SkipKind::SkipIf | SkipKind::RunIf => {
            match skip.condition.as_deref() {
                // A `skipIf()` / `runIf()` with nothing to test is malformed, not a skip.
                None if inverted || skip.kind == SkipKind::SkipIf => Verdict::Ignore,
                None => Verdict::Unconditional,
                Some(condition) => match literal_bool(condition) {
                    // The literal that closes the gate: `skip(true)` never runs, and neither
                    // does `runIf(false)`.
                    Some(value) if value != inverted => Verdict::Unconditional,
                    // The literal that opens it: `skipIf(false)` / `runIf(true)` always run.
                    Some(_) => Verdict::Ignore,
                    None => env_variable(condition).map_or(Verdict::Conditional, Verdict::CiScoped),
                },
            }
        }
    }
}

/// The boolean a condition's source text spells as a bare literal (`true` / `false`), or
/// `None` for anything else — `!!true`, `Boolean(true)` and the like are not folded here.
fn literal_bool(condition: &str) -> Option<bool> {
    match condition.trim() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

/// The environment variable a condition reads, when it reads one: the identifier after
/// `process.env.` or `import.meta.env.`, or the string key of `process.env["X"]`. `None`
/// when the condition reads no environment variable, so a data-driven `skipIf` the rule
/// cannot call CI-scoped is left alone rather than guessed.
pub fn env_variable(condition: &str) -> Option<String> {
    for prefix in ["process.env.", "import.meta.env."] {
        if let Some(rest) = find_after(condition, prefix) {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '$')
                .collect();
            if !name.is_empty() {
                return Some(name);
            }
        }
    }
    for bracket in ["process.env[", "import.meta.env["] {
        if let Some(rest) = find_after(condition, bracket) {
            if let Some(name) = bracket_key(rest) {
                return Some(name);
            }
        }
    }
    None
}

/// The text just after the first occurrence of `needle` in `haystack`, or `None`.
fn find_after<'a>(haystack: &'a str, needle: &str) -> Option<&'a str> {
    haystack
        .find(needle)
        .map(|index| &haystack[index + needle.len()..])
}

/// The string key of a `["X"]` / `['X']` member access at the start of `rest`, or `None`.
fn bracket_key(rest: &str) -> Option<String> {
    let rest = rest.trim_start();
    let quote = rest.chars().next().filter(|c| *c == '"' || *c == '\'')?;
    let inner = &rest[1..];
    let end = inner.find(quote)?;
    let key = &inner[..end];
    if key.is_empty() {
        None
    } else {
        Some(key.to_owned())
    }
}

/// Whether a skip is a definition-level chain modifier (`it.skip('t', …)`) rather than a
/// runtime `test.skip()` statement written inside the test body.
///
/// A definition modifier's span sits on the definition's own line — the recogniser records it
/// at the `it`/`test`/`describe` call. A body annotation's span sits on a later line, inside
/// the callback the definition opened. The line comparison separates the two: an in-body
/// skip is usually an early-return under an `if` guard the rule cannot see, so it is not a
/// dead definition skip.
const fn is_definition_level(case: &TestCase, skip: &Skip) -> bool {
    skip.span.line == case.span.line
}

/// Whether the skip carries an issue reference — a `#123`, an issue URL, or a `KEY-123`
/// ticket — in the case's title, the skip's reason, or the comments beside the definition
/// (the trailing comment and the comment lines just above). A reference makes an
/// unconditional skip tracked, so the rule stays silent on it.
fn has_issue_reference(case: &TestCase, skip: &Skip) -> bool {
    let title = case.title.as_deref().unwrap_or("");
    let reason = skip.reason.as_deref().unwrap_or("");
    contains_reference(title)
        || contains_reference(reason)
        || skip
            .comments
            .iter()
            .any(|comment| contains_reference(comment))
}

/// Whether `text` holds an issue reference in one of the three recognised forms.
fn contains_reference(text: &str) -> bool {
    has_hash_number(text) || has_issue_url(text) || has_ticket(text)
}

/// A `#123`: a `#` immediately followed by a digit.
fn has_hash_number(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes
        .iter()
        .enumerate()
        .any(|(i, &b)| b == b'#' && bytes.get(i + 1).is_some_and(u8::is_ascii_digit))
}

/// An issue URL: a GitHub/GitLab-style `/issues/123` (or `/-/issues/123`) path, or a pull
/// or merge request (`/pull/123`, `/pulls/123`, `/merge_requests/123`) — the tracker item
/// that explains the skip or the change it waits on.
fn has_issue_url(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "/issues/",
        "/issue/",
        "/pulls/",
        "/pull/",
        "/merge_requests/",
    ]
    .iter()
    .any(|marker| {
        find_after(&lower, marker).is_some_and(|rest| {
            rest.trim_start_matches('-')
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit())
        })
    })
}

/// A ticket key: an upper-case tracker prefix, a hyphen, and a number (`JIRA-42`,
/// `PROJ-1234`), the Jira/Linear convention. At least two upper-case letters so a lone word
/// is not mistaken for a key.
fn has_ticket(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // A ticket key starts at a word boundary: the previous byte is not part of an
        // identifier.
        let boundary = i == 0 || !is_ident_byte(bytes[i - 1]);
        if boundary {
            let mut j = i;
            while j < bytes.len() && bytes[j].is_ascii_uppercase() {
                j += 1;
            }
            let letters = j - i;
            if letters >= 2 && bytes.get(j) == Some(&b'-') {
                let mut k = j + 1;
                let digit_start = k;
                while k < bytes.len() && bytes[k].is_ascii_digit() {
                    k += 1;
                }
                if k > digit_start {
                    return true;
                }
            }
        }
        i += 1;
    }
    false
}

/// Whether a byte is part of a JS identifier, so a ticket key is only matched at a word
/// boundary.
const fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}

/// The modifier as the source spells it, for the listing row (`skipIf` stays `skipIf`,
/// where [`kind_word`] folds it into `skip` for the flagged rows' `kind` evidence).
pub const fn modifier_word(kind: SkipKind) -> &'static str {
    match kind {
        SkipKind::SkipIf => "skipIf",
        other => kind_word(other),
    }
}

/// The word the report spells a skip kind with.
const fn kind_word(kind: SkipKind) -> &'static str {
    match kind {
        SkipKind::Skip | SkipKind::SkipIf => "skip",
        SkipKind::Todo => "todo",
        SkipKind::Fixme => "fixme",
        SkipKind::RunIf => "runIf",
        SkipKind::Only => "only",
        SkipKind::Fails => "fails",
        SkipKind::Slow => "slow",
    }
}

/// The test's title in quotes, or a placeholder for a dynamic one, for the message.
fn title_of(case: &TestCase) -> String {
    case.title
        .as_deref()
        .map_or_else(|| "<dynamic>".to_owned(), |title| format!("{title:?}"))
}

/// The semantic anchor for a case: its suite path joined with its title, the same shape the
/// other rules use, so the identity is stable across runs and unrelated edits.
fn anchor_of(case: &TestCase) -> String {
    let mut parts = case.suite_path.clone();
    parts.push(
        case.title
            .clone()
            .unwrap_or_else(|| format!("<dynamic>:{}", case.span.line)),
    );
    parts.join(" › ")
}
