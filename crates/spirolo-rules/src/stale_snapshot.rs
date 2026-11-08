//! `stale-snapshot`: a `.snap` entry the runner would report obsolete.
//!
//! For each `__snapshots__/<name>.snap` beside a test file, the model carries a
//! [`Snapshot`](spirolo_model::Snapshot) row with the entries the `.snap` reader lifted and
//! the sibling test file it belongs to. A test writes the keys that start with its full
//! name (`describe` titles and title, ` > `-joined under Vitest, space-joined under Jest;
//! either join when the model cannot tell the runner), followed by the ordinal or by a
//! snapshot hint (` > hint`, Jest's `: hint`) and the ordinal. The key is matched by that prefix,
//! never split on ` > `, since a hint or a title may hold one. An entry is stale when no
//! test in the sibling file writes it — the test it was taken for is gone. An entry that
//! only a test with a computed name could write (a `describe.each` or `.each`/`.for`
//! template, a title that is not a literal) is could-not-decide, never stale: the model
//! does not format rows. A `.snap` file with no sibling test file at all is stale as a
//! whole, one finding for the file. A sibling that failed to parse holds no test the model
//! can read, so its `.snap` is one could-not-decide finding, never a stale entry; a sibling
//! that holds no test the model reads (its suite registered by an imported function, or the
//! file collected by a runner config the model does not apply) leaves each entry
//! could-not-decide. An entry no local test writes, in a sibling that calls an imported
//! function where tests are registered (a shared suite, `formTests(Form)`, or a wrapper that
//! names the tests in its callback, `describeWithPointerEvent(…)`), is could-not-decide too.
//! A newline in a title is matched as Jest writes it in the key, the two characters `\n`
//! (either spelling when the model cannot tell the runner). Inline snapshots are not read;
//! the runner keeps those in the test.
//!
//! When any runner config sets a custom `resolveSnapshotPath`, the `__snapshots__`
//! convention the rule follows no longer holds, so the rule defers for the whole project
//! with one could-not-decide finding rather than report a hundred false orphans. Severity
//! `info`; identity is the snapshot file and the key.

use std::collections::BTreeSet;

use spirolo_model::{Model, Snapshot, SnapshotEntry, TestCase, TestKind};

use crate::finding::{Evidence, Finding, Location, Outcome, Proposal};
use crate::{Config, Family, Rule, Severity};

/// The suite-path segment the framework readers write for a `describe` whose title is not
/// a literal.
const DYNAMIC: &str = "<dynamic>";

/// The `stale-snapshot` rule. See the module docs for the semantics.
pub struct StaleSnapshot;

impl Rule for StaleSnapshot {
    fn name(&self) -> &'static str {
        "stale-snapshot"
    }

    fn family(&self) -> Family {
        Family::Hygiene
    }

    fn subject(&self) -> crate::Subject {
        crate::Subject::TestFiles
    }

    fn summary(&self) -> &'static str {
        "A stored snapshot entry has no test left to produce it, so the runner would report it obsolete."
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn run(&self, model: &Model, config: &Config) -> Vec<Finding> {
        let severity = config.severity(self.name(), self.default_severity());
        if let Some(gate) = model.gates.iter().find(|gate| gate.snapshot_resolver) {
            return vec![self.deferred(&gate.source, severity)];
        }
        let mut findings = Vec::new();
        for snapshot in &model.snapshots {
            findings.extend(self.stale_entries(model, snapshot, severity, config.propose));
        }
        crate::finding::sort(&mut findings);
        crate::finding::number_collisions(&mut findings);
        findings
    }
}

impl StaleSnapshot {
    /// One finding per entry of `snapshot` no literal test in the sibling file writes: stale,
    /// or could-not-decide when a test with a computed name could write it.
    fn stale_entries(
        &self,
        model: &Model,
        snapshot: &Snapshot,
        severity: Severity,
        propose: bool,
    ) -> Vec<Finding> {
        let Some(sibling) = snapshot.sibling.as_deref() else {
            return vec![self.orphan_file(snapshot, severity, propose)];
        };
        let unparsed = model.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "parse_failed" && diagnostic.file.as_deref() == Some(sibling)
        });
        if unparsed {
            return vec![self.unread_sibling(snapshot, sibling, severity)];
        }
        let in_sibling = || model.test_cases.iter().filter(|case| case.file == sibling);
        // `describe.each` titles: templates, like a test's `.each` title.
        let each_suites: BTreeSet<&str> = in_sibling()
            .filter(|case| case.kind == TestKind::Suite && case.each.is_some())
            .filter_map(|case| case.title.as_deref())
            .collect();
        let spellings = spellings(model, sibling);
        let (exact, templated): (Vec<Name>, Vec<Name>) = in_sibling()
            .filter(|case| case.kind == TestKind::Test)
            .flat_map(|case| {
                spellings
                    .iter()
                    .map(|spelling| name(case, &each_suites, spelling))
            })
            .partition(|name| !name.templated);
        if exact.is_empty() && templated.is_empty() {
            return snapshot
                .entries
                .iter()
                .map(|entry| self.registered_elsewhere(snapshot, sibling, entry, severity, propose))
                .collect();
        }
        let mut findings = Vec::new();
        for entry in &snapshot.entries {
            if exact.iter().any(|name| produced(&name.text, entry)) {
                continue;
            }
            if templated
                .iter()
                .any(|name| could_produce(&name.text, entry))
            {
                findings.push(self.undecided(snapshot, sibling, entry, severity, propose));
            } else if let Some(call) = snapshot.imported_calls.first() {
                findings
                    .push(self.imported_call(snapshot, sibling, call, entry, severity, propose));
            } else {
                findings.push(self.stale(snapshot, sibling, entry, severity, propose));
            }
        }
        findings
    }

    /// The finding for an entry no test in `sibling` writes: the runner reports it obsolete.
    fn stale(
        &self,
        snapshot: &Snapshot,
        sibling: &str,
        entry: &SnapshotEntry,
        severity: Severity,
        propose: bool,
    ) -> Finding {
        Finding {
            rule: self.name(),
            severity,
            outcome: Outcome::Finding,
            location: Location {
                file: snapshot.file.clone(),
                line: entry.line,
                column: 1,
            },
            related: Vec::new(),
            message: format!(
                "the snapshot {} names {}, a test {} no longer has",
                snapshot.file, entry.key, sibling
            ),
            evidence: vec![
                Evidence::new("kind", "stale-entry"),
                Evidence::new("key", entry.key.clone()),
                Evidence::new("sibling", sibling.to_owned()),
            ],
            explain: format!(
                "{} keeps the snapshot {:?}, but {} has no test with that suite path and \
                     title — the test it was taken for was renamed or removed. The runner \
                     reports it obsolete. Delete the entry (or run the runner with `-u`).",
                snapshot.file, entry.key, sibling,
            ),
            anchor: entry.key.clone(),
            test: None,
            ordinal: 0,
            proposal: propose.then(|| Proposal::Delete {
                what: Some(entry.key.clone()),
            }),
        }
    }

    /// The could-not-decide finding for an entry only a test with a computed title in
    /// `sibling` could have written — an `.each`/`.for` template formatted per row, or a
    /// title that is not a literal. The model does not evaluate either, so the entry may be
    /// live and is never reported stale.
    fn undecided(
        &self,
        snapshot: &Snapshot,
        sibling: &str,
        entry: &SnapshotEntry,
        severity: Severity,
        propose: bool,
    ) -> Finding {
        Finding {
            rule: self.name(),
            severity,
            outcome: Outcome::CouldNotDecide,
            location: Location {
                file: snapshot.file.clone(),
                line: entry.line,
                column: 1,
            },
            related: Vec::new(),
            message: format!(
                "the snapshot {} names {}, which only a test with a computed title in {} could write",
                snapshot.file, entry.key, sibling
            ),
            evidence: vec![
                Evidence::new("kind", "computed-title"),
                Evidence::new("key", entry.key.clone()),
                Evidence::new("sibling", sibling.to_owned()),
            ],
            explain: format!(
                "{} keeps the snapshot {:?}. No test in {} has that name literally, but a \
                 test there computes its title — an `.each`/`.for` template formatted per row, \
                 or a title that is not a literal — and could write this key. Whether it does is the runner's call, so the \
                 entry is not reported stale.",
                snapshot.file, entry.key, sibling,
            ),
            anchor: entry.key.clone(),
            test: None,
            ordinal: 0,
            proposal: propose.then(|| Proposal::Keep {
                reason: "a test with a computed title may write it".to_owned(),
            }),
        }
    }

    /// The could-not-decide finding for an entry of a `.snap` whose `sibling` holds no test
    /// the model reads: its suite is registered by a function it imports (`formTests(Form)`),
    /// or the file sits outside the runner config the model applies (a second config such as
    /// `jest.int.config.js` runs it). Either way the entry may be live and is never stale.
    fn registered_elsewhere(
        &self,
        snapshot: &Snapshot,
        sibling: &str,
        entry: &SnapshotEntry,
        severity: Severity,
        propose: bool,
    ) -> Finding {
        Finding {
            rule: self.name(),
            severity,
            outcome: Outcome::CouldNotDecide,
            location: Location {
                file: snapshot.file.clone(),
                line: entry.line,
                column: 1,
            },
            related: Vec::new(),
            message: format!(
                "the snapshot {} names {}, but the model holds no test in {} to check it against",
                snapshot.file, entry.key, sibling
            ),
            evidence: vec![
                Evidence::new("kind", "no-local-test"),
                Evidence::new("key", entry.key.clone()),
                Evidence::new("sibling", sibling.to_owned()),
            ],
            explain: format!(
                "{} keeps the snapshot {:?}, but the model holds no test in {}: its tests are \
                 registered by a function it calls (`formTests(Form)`), or the file is collected \
                 by a runner config other than the one the model applies. Whether a test writes \
                 this key is the runner's call, so the entry is not reported stale.",
                snapshot.file, entry.key, sibling,
            ),
            anchor: entry.key.clone(),
            test: None,
            ordinal: 0,
            proposal: propose.then(|| Proposal::Keep {
                reason: "a test the model does not read may write it".to_owned(),
            }),
        }
    }

    /// The could-not-decide finding for an entry no local test in `sibling` writes, beside an
    /// imported function the sibling calls where tests are registered (`call`): a shared
    /// suite (`formTests(Form)`) or a wrapper that computes its tests' title prefix
    /// (`describeWithPointerEvent(…)`) may write it, so it is never stale.
    fn imported_call(
        &self,
        snapshot: &Snapshot,
        sibling: &str,
        call: &str,
        entry: &SnapshotEntry,
        severity: Severity,
        propose: bool,
    ) -> Finding {
        Finding {
            rule: self.name(),
            severity,
            outcome: Outcome::CouldNotDecide,
            location: Location {
                file: snapshot.file.clone(),
                line: entry.line,
                column: 1,
            },
            related: Vec::new(),
            message: format!(
                "the snapshot {} names {}, which no local test in {} writes but its call to {call} may",
                snapshot.file, entry.key, sibling
            ),
            evidence: vec![
                Evidence::new("kind", "imported-call"),
                Evidence::new("key", entry.key.clone()),
                Evidence::new("sibling", sibling.to_owned()),
                Evidence::new("call", call.to_owned()),
            ],
            explain: format!(
                "{} keeps the snapshot {:?}. No test {} declares has that name, but it calls \
                 `{call}`, a function it imports, where tests are registered: a shared suite or \
                 a wrapper that names its tests may write this key. Whether it does is the \
                 runner's call, so the entry is not reported stale.",
                snapshot.file, entry.key, sibling,
            ),
            anchor: entry.key.clone(),
            test: None,
            ordinal: 0,
            proposal: propose.then(|| Proposal::Keep {
                reason: "a test an imported function registers may write it".to_owned(),
            }),
        }
    }

    /// One could-not-decide finding for a project whose runner config sets a custom
    /// `resolveSnapshotPath`: the `__snapshots__` convention no longer locates the files, so
    /// the rule defers rather than report every `.snap` as orphaned.
    fn deferred(&self, source: &str, severity: Severity) -> Finding {
        Finding {
            rule: self.name(),
            severity,
            outcome: Outcome::CouldNotDecide,
            location: Location {
                file: source.to_owned(),
                line: 1,
                column: 1,
            },
            related: Vec::new(),
            message: format!(
                "{source} sets a custom resolveSnapshotPath, so stale snapshots cannot be decided"
            ),
            evidence: vec![
                Evidence::new("kind", "custom-resolver"),
                Evidence::new("config", source.to_owned()),
            ],
            explain: format!(
                "{source} sets a custom `resolveSnapshotPath`, which moves the `.snap` files off \
                 the `__snapshots__` convention this rule follows. Deciding staleness would mean \
                 evaluating that function, so the rule defers for the whole project rather than \
                 report every snapshot as orphaned."
            ),
            anchor: "custom-resolver".to_owned(),
            test: None,
            ordinal: 0,
            proposal: None,
        }
    }

    /// One could-not-decide finding for a `.snap` file whose sibling test file failed to
    /// parse (Flow, say): the model holds none of its tests, so no entry can be called stale.
    fn unread_sibling(&self, snapshot: &Snapshot, sibling: &str, severity: Severity) -> Finding {
        Finding {
            rule: self.name(),
            severity,
            outcome: Outcome::CouldNotDecide,
            location: Location {
                file: snapshot.file.clone(),
                line: 1,
                column: 1,
            },
            related: Vec::new(),
            message: format!(
                "the snapshot {}'s test file {} could not be parsed, so its entries cannot be decided",
                snapshot.file, sibling
            ),
            evidence: vec![
                Evidence::new("kind", "unparsed-sibling"),
                Evidence::new("sibling", sibling.to_owned()),
                Evidence::new("entries", snapshot.entries.len().to_string()),
            ],
            explain: format!(
                "{sibling} did not parse, so the model holds none of its tests and cannot tell \
                 which of the {} entries of {} they still write. None is reported stale.",
                snapshot.entries.len(),
                snapshot.file,
            ),
            anchor: "unparsed-sibling".to_owned(),
            test: None,
            ordinal: 0,
            proposal: None,
        }
    }

    /// One finding for a `.snap` file whose sibling test file is gone: the whole file is
    /// stale, so it is reported once rather than once per entry.
    fn orphan_file(&self, snapshot: &Snapshot, severity: Severity, propose: bool) -> Finding {
        Finding {
            rule: self.name(),
            severity,
            outcome: Outcome::Finding,
            location: Location {
                file: snapshot.file.clone(),
                line: 1,
                column: 1,
            },
            related: Vec::new(),
            message: format!(
                "the snapshot {} has no sibling test file, so every entry is stale",
                snapshot.file
            ),
            evidence: vec![
                Evidence::new("kind", "orphan-file"),
                Evidence::new("entries", snapshot.entries.len().to_string()),
            ],
            explain: format!(
                "{} sits beside no test file — the test file it belonged to was deleted or \
                 moved, so the runner never writes to it again and all {} of its entries are \
                 obsolete. Delete the file.",
                snapshot.file,
                snapshot.entries.len(),
            ),
            anchor: String::new(),
            test: None,
            ordinal: 0,
            proposal: propose.then_some(Proposal::Delete { what: None }),
        }
    }
}

/// A sibling test's name as the runner writes it into a key. For a test whose name is
/// computed anywhere — a `describe.each` or `.each`/`.for` template, a title that is not a
/// literal — `text` is only the part known before the first computed piece and `templated`
/// is set.
struct Name {
    text: String,
    templated: bool,
}

/// How a runner spells a test's name in a key: the separator its `describe` titles and
/// title are joined with, and whether a newline in a title is written as the two characters
/// `\n`.
struct Spelling {
    join: &'static str,
    escaped_newline: bool,
}

/// The spellings the runner of `sibling` could write a test's name in: Jest's (single-space
/// join, a newline written `\n`), or, when the model cannot tell the runner (a file it does
/// not place under a Jest config or import reads as Vitest by default), Vitest's ` > ` or
/// Jest's join with a newline written either way.
fn spellings(model: &Model, sibling: &str) -> &'static [Spelling] {
    let jest = model
        .test_files
        .iter()
        .any(|file| file.file == sibling && file.framework == "jest");
    if jest {
        &[Spelling {
            join: " ",
            escaped_newline: true,
        }]
    } else {
        &[
            Spelling {
                join: " > ",
                escaped_newline: false,
            },
            Spelling {
                join: " > ",
                escaped_newline: true,
            },
            Spelling {
                join: " ",
                escaped_newline: false,
            },
            Spelling {
                join: " ",
                escaped_newline: true,
            },
        ]
    }
}

/// The name of `case` in `spelling`: its `describe` titles and its own title, joined and
/// with newlines written as the runner writes them, cut at the first computed piece.
/// `each_suites` are the titles of the file's `describe.each` suites.
fn name(case: &TestCase, each_suites: &BTreeSet<&str>, spelling: &Spelling) -> Name {
    let mut name = joined(case, each_suites, spelling.join);
    if spelling.escaped_newline {
        name.text = name.text.replace('\n', "\\n");
    }
    name
}

/// The name of `case`: its `describe` titles and its own title, joined by `join`, cut at the
/// first computed piece.
fn joined(case: &TestCase, each_suites: &BTreeSet<&str>, join: &str) -> Name {
    let mut text = String::new();
    for segment in &case.suite_path {
        if segment == DYNAMIC {
            return Name {
                text,
                templated: true,
            };
        }
        if each_suites.contains(segment.as_str()) {
            text.push_str(before_placeholder(segment));
            return Name {
                text,
                templated: true,
            };
        }
        text.push_str(segment);
        text.push_str(join);
    }
    match case.title.as_deref() {
        None => Name {
            text,
            templated: true,
        },
        Some(title) if case.each.is_some() => {
            text.push_str(before_placeholder(title));
            Name {
                text,
                templated: true,
            }
        }
        Some(title) => {
            text.push_str(title);
            Name {
                text,
                templated: false,
            }
        }
    }
}

/// The text of an `.each` template up to its first placeholder (`%s`, `$field`), after
/// which the rows decide.
fn before_placeholder(template: &str) -> &str {
    template
        .find(['%', '$'])
        .map_or(template, |at| &template[..at])
}

/// Whether the test named `name` writes `entry`: the key is the name then the ordinal, or
/// the name, a snapshot hint (` > hint`, or Jest's `: hint`) and the ordinal.
fn produced(name: &str, entry: &SnapshotEntry) -> bool {
    let Some(head) = entry.key.strip_suffix(&format!(" {}", entry.ordinal)) else {
        return false;
    };
    head.strip_prefix(name)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(" > ") || rest.starts_with(": "))
}

/// Whether a test whose name is computed, known up to `name`, could write `entry`.
fn could_produce(name: &str, entry: &SnapshotEntry) -> bool {
    entry.key.starts_with(name)
}
