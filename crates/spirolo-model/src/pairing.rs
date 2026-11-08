//! Pairing each test file with the source module it exercises.
//!
//! Runs once, after resolution, because the first and strongest strategy reads a test
//! file's *resolved* imports. Three strategies are tried in order of how directly they
//! tie a test to its subject, and the first that matches wins:
//!
//! 1. **import** — the test imports exactly one non-test module in its own package; when
//!    it imports several, the one whose name matches the test's own wins and the pairing
//!    is marked ambiguous so a later rule can weigh it accordingly;
//! 2. **sibling** — `x.test.ts` sits next to `x.ts` (`.spec`, `.msw.test` and other
//!    flavour tags are stripped the same way);
//! 3. **tests-dir** — `__tests__/x.test.ts` sits one directory above `x.ts`.
//!
//! A test file that none of the three pairs is not an error: it becomes an
//! `unpaired_test` diagnostic, so the count is visible without aborting the run.
//!
//! A spec a Playwright config collects is not a candidate at all: an end-to-end test
//! drives the running application, not a module, so there is no source to pair it with
//! and no warning to raise. Pairing therefore runs after [`crate::scope`] and asks it
//! which files Playwright collects.

use std::collections::{HashMap, HashSet};

use crate::{Diagnostic, Model, Pairing, PairingReason, Severity};

/// Pair every test file in `model`, filling [`Model::pairings`] and appending an
/// `unpaired_test` diagnostic for each test file no strategy could pair. `package_dirs`
/// are the directories holding a `package.json` the walk found — the package boundary
/// the `import` strategy respects. Files for which `playwright_collects` holds are
/// end-to-end specs and are left out. Mutates `model` in place; callers sort afterwards.
pub fn pair(
    model: &mut Model,
    package_dirs: &HashSet<String>,
    playwright_collects: &dyn Fn(&str) -> bool,
) {
    let source_paths: HashSet<&str> = model
        .modules
        .iter()
        .filter(|module| !is_test_file(&module.path))
        .map(|module| module.path.as_str())
        .collect();
    let by_place = SourcesByPlace::new(&source_paths);

    let mut packages = PackageIndex::new(package_dirs);
    let mut pairings = Vec::new();
    let mut diagnostics = Vec::new();
    for module in &model.modules {
        if !is_test_file(&module.path) || playwright_collects(&module.path) {
            continue;
        }
        let paired = pair_by_import(module, &source_paths, &mut packages)
            .or_else(|| pair_by_sibling(&module.path, &by_place))
            .or_else(|| pair_by_tests_dir(&module.path, &by_place));
        match paired {
            Some((source_module, reason, ambiguous)) => pairings.push(Pairing {
                test_file: module.path.clone(),
                source_module,
                reason,
                ambiguous,
            }),
            None => diagnostics.push(Diagnostic {
                code: "unpaired_test".to_owned(),
                severity: Severity::Warning,
                file: Some(module.path.clone()),
                span: None,
                message: format!("no source module paired with test file {}", module.path),
            }),
        }
    }

    model.pairings = pairings;
    model.diagnostics.extend(diagnostics);
}

/// A resolved pairing: the source module, the strategy that decided it, and whether the
/// decision had to break a tie between several candidates.
type Paired = (String, PairingReason, bool);

/// Strategy 1: the test's own resolved imports.
///
/// Candidates are the in-project, non-test modules the test imports from its own package,
/// deduplicated. One candidate pairs outright; several pair only if exactly one carries
/// the test's own base name, and that pairing is marked ambiguous. Zero candidates, or an
/// unbreakable tie, fall through to the naming strategies.
fn pair_by_import(
    test: &crate::Module,
    source_paths: &HashSet<&str>,
    packages: &mut PackageIndex,
) -> Option<Paired> {
    let test_package = packages.package_of(&test.path);
    let mut candidates: Vec<&str> = Vec::new();
    for import in &test.imports {
        let Some(target) = import.resolved.as_deref() else {
            continue;
        };
        if source_paths.contains(target)
            && packages.package_of(target) == test_package
            && !candidates.contains(&target)
        {
            candidates.push(target);
        }
    }

    match candidates.as_slice() {
        [] => None,
        [only] => Some(((*only).to_owned(), PairingReason::DirectImport, false)),
        many => {
            let bases = source_bases(basename(&test.path));
            let mut named = many
                .iter()
                .filter(|target| bases.iter().any(|base| stem(basename(target)) == base));
            let first = named.next()?;
            // Only a single name match breaks the tie; two would be ambiguous again.
            if named.next().is_some() {
                return None;
            }
            Some(((*first).to_owned(), PairingReason::DirectImport, true))
        }
    }
}

/// Strategy 2: a source file next to the test with the test's base name.
fn pair_by_sibling(test_path: &str, by_place: &SourcesByPlace<'_>) -> Option<Paired> {
    let dir = directory(test_path);
    let source = find_in_directory(dir, basename(test_path), by_place)?;
    Some((source, PairingReason::SiblingName, false))
}

/// Strategy 3: a source file one directory above the test's `__tests__` folder.
fn pair_by_tests_dir(test_path: &str, by_place: &SourcesByPlace<'_>) -> Option<Paired> {
    let parts: Vec<&str> = test_path.split('/').collect();
    let index = parts.iter().position(|part| *part == "__tests__")?;
    let parent = parts[..index].join("/");
    let source = find_in_directory(&parent, basename(test_path), by_place)?;
    Some((source, PairingReason::TestsDirectory, false))
}

/// The one source file in `dir` whose stem is one of the test's base names, most specific
/// base first (`api.msw` before `api`), so a `foo.msw.test.ts` prefers `foo.msw.ts` when
/// both it and `foo.ts` exist.
///
/// When several files in `dir` share the stem for a base (`widget.ts` and `widget.tsx`
/// during a migration), the lexicographically smallest path is chosen so the pairing is
/// the same on every run — `source_paths` is a `HashSet`, whose iteration order is not.
fn find_in_directory(dir: &str, test_name: &str, by_place: &SourcesByPlace<'_>) -> Option<String> {
    let bases = source_bases(test_name);
    bases
        .iter()
        .find_map(|base| by_place.get(dir, base))
        .map(str::to_owned)
}

/// The source paths keyed by directory and stem, each key holding the lexicographically
/// smallest path, so a directory lookup is one hash probe rather than a scan of every
/// source per test file (#153).
struct SourcesByPlace<'a> {
    smallest: HashMap<(&'a str, &'a str), &'a str>,
}

impl<'a> SourcesByPlace<'a> {
    fn new(source_paths: &HashSet<&'a str>) -> Self {
        let mut smallest: HashMap<(&str, &str), &str> = HashMap::new();
        for &path in source_paths {
            smallest
                .entry((directory(path), stem(basename(path))))
                .and_modify(|kept| *kept = (*kept).min(path))
                .or_insert(path);
        }
        Self { smallest }
    }

    /// The smallest source path in `dir` whose stem is `base`.
    fn get(&self, dir: &str, base: &str) -> Option<&'a str> {
        self.smallest.get(&(dir, base)).copied()
    }
}

/// Whether the model treats a path as a test file: a `.test.`/`.spec.` infix anywhere in
/// the file name, or any `__tests__` segment on the path. This is the naming half of the
/// runner's own view; a file the recognisers found no cases in is still named like a test
/// and is paired as one, which is harmless and keeps pairing free of the recognisers.
fn is_test_file(path: &str) -> bool {
    let name = basename(path);
    name.contains(".test.")
        || name.contains(".spec.")
        || path.split('/').any(|segment| segment == "__tests__")
}

/// The base names a test file's source might carry, most specific first.
///
/// The stem loses its trailing `.test`/`.spec`, then, if a flavour tag remains
/// (`api.msw`), a second candidate drops that too (`api`). `sum.test` yields `["sum"]`;
/// `api.msw.test` yields `["api.msw", "api"]`.
fn source_bases(test_name: &str) -> Vec<String> {
    let stem = stem(test_name);
    let trimmed = stem
        .strip_suffix(".test")
        .or_else(|| stem.strip_suffix(".spec"))
        .unwrap_or(stem);
    let mut bases = vec![trimmed.to_owned()];
    if let Some(dot) = trimmed.rfind('.') {
        bases.push(trimmed[..dot].to_owned());
    }
    bases
}

/// The last `/`-separated component of a path.
fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Everything before a path's last `/`; the empty string for a path with no `/`.
fn directory(path: &str) -> &str {
    path.rfind('/').map_or("", |slash| &path[..slash])
}

/// A file name without its final extension: `sum.test.ts` becomes `sum.test`.
fn stem(name: &str) -> &str {
    name.rfind('.').map_or(name, |dot| &name[..dot])
}

/// Which package a repo-relative path belongs to, cached per directory.
///
/// A package is the nearest ancestor directory holding a `package.json` the walk found;
/// a path with no such ancestor belongs to the root package, spelled `""`. The `import`
/// strategy uses this so a monorepo test pairs only inside its own package, never across
/// a workspace boundary a sibling package's re-export happens to cross.
struct PackageIndex<'p> {
    package_dirs: &'p HashSet<String>,
    cache: HashMap<String, String>,
}

impl<'p> PackageIndex<'p> {
    fn new(package_dirs: &'p HashSet<String>) -> Self {
        Self {
            package_dirs,
            cache: HashMap::new(),
        }
    }

    /// The package directory (repo-relative, `""` for the root) owning `path`.
    ///
    /// Every directory walked on the way up is memoised against the answer, not just the
    /// package directory found: sibling test files in the same package then answer from the
    /// cache instead of re-walking the shared ancestor chain.
    fn package_of(&mut self, path: &str) -> String {
        let mut walked: Vec<String> = Vec::new();
        let mut dir = directory(path).to_owned();
        let answer = loop {
            if let Some(found) = self.cache.get(&dir) {
                break found.clone();
            }
            if self.has_manifest(&dir) {
                break dir.clone();
            }
            if dir.is_empty() {
                break String::new();
            }
            walked.push(dir.clone());
            match dir.rfind('/') {
                Some(slash) => dir.truncate(slash),
                None => dir.clear(),
            }
        };
        self.cache.insert(dir, answer.clone());
        for visited in walked {
            self.cache.insert(visited, answer.clone());
        }
        answer
    }

    /// Whether the repo-relative directory `dir` holds a `package.json` the walk found.
    fn has_manifest(&self, dir: &str) -> bool {
        self.package_dirs.contains(dir)
    }
}
