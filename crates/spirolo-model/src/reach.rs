//! The import closures between test files and modules. Reversed: which test files reach a
//! set of changed modules through their resolved imports, and in how many hops, which
//! `impact` selects from. Forward: which modules each test file reaches, and which of their
//! named exports it references, which `untested-hotspot` reads as weakness.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

use crate::{ExportKind, Model};

/// One module a test file reaches, with the module's named exports the test file references.
///
/// A query on the model ([`Model::test_reach`]), not a table: `list` never computes it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TestReach {
    /// Repo-relative path of the test file (a file the `TestCase` table has a row in).
    pub test_file: String,
    /// Repo-relative path of a module in the test file's transitive closure over resolved
    /// imports, or paired with it.
    pub module: String,
    /// The module's named exports (`ExportKind::Named`) whose name the test file reads as an
    /// identifier, sorted and deduplicated. Empty when it reads none of them.
    pub referenced: Vec<String>,
}

/// The model's import edges reversed (each module to the modules that import it), with the
/// set of test files, built once and walked for as many changed sets as a caller asks.
#[derive(Debug, Clone)]
pub struct Importers<'m> {
    importers: HashMap<&'m str, Vec<&'m str>>,
    tests: HashSet<&'m str>,
}

impl Importers<'_> {
    /// Every test file whose transitive closure over resolved imports contains one of
    /// `changed`, keyed by path, with the shortest hop count to the nearest changed module
    /// (1 = a direct import). A changed file that is itself a test file is at distance 0.
    pub fn test_distances(&self, changed: &[&str]) -> BTreeMap<String, u32> {
        let mut seen: HashMap<&str, u32> = HashMap::new();
        let mut queue = VecDeque::new();
        for path in changed {
            if !seen.contains_key(path) {
                seen.insert(path, 0);
                queue.push_back(*path);
            }
        }
        while let Some(path) = queue.pop_front() {
            let distance = seen[path];
            for importer in self.importers.get(path).into_iter().flatten() {
                if !seen.contains_key(importer) {
                    seen.insert(importer, distance + 1);
                    queue.push_back(importer);
                }
            }
        }
        seen.into_iter()
            .filter(|(path, _)| self.tests.contains(path))
            .map(|(path, distance)| (path.to_owned(), distance))
            .collect()
    }

    /// True when `path` is a test file: the `TestCase` table has a row in it.
    pub fn is_test(&self, path: &str) -> bool {
        self.tests.contains(path)
    }
}

impl Model {
    /// The reverse import graph over resolved imports. A test file is one the `TestCase`
    /// table has a row in; any other module only carries the walk. An import that did not
    /// resolve has no target and is not an edge.
    pub fn importers(&self) -> Importers<'_> {
        let mut importers: HashMap<&str, Vec<&str>> = HashMap::new();
        for module in &self.modules {
            for import in &module.imports {
                if let Some(target) = &import.resolved {
                    importers
                        .entry(target.as_str())
                        .or_default()
                        .push(module.path.as_str());
                }
            }
        }
        Importers {
            importers,
            tests: self
                .test_cases
                .iter()
                .map(|case| case.file.as_str())
                .collect(),
        }
    }

    /// [`Importers::test_distances`] over a graph built for this one call.
    pub fn test_distances(&self, changed: &[&str]) -> BTreeMap<String, u32> {
        self.importers().test_distances(changed)
    }
}

impl Model {
    /// Every module each test file reaches, with the module's named exports the test file
    /// references by identifier. A test file reaches the modules in its transitive closure
    /// over resolved imports and the modules it is paired with (not their imports). Sorted
    /// by test file then module; the test file itself is not a row, nor a resolved target
    /// the model has no module for.
    pub fn test_reach(&self) -> Vec<TestReach> {
        let modules: HashMap<&str, &crate::Module> =
            self.modules.iter().map(|m| (m.path.as_str(), m)).collect();
        let tests: BTreeSet<&str> = self.test_cases.iter().map(|c| c.file.as_str()).collect();
        let mut rows = Vec::new();
        for test in tests {
            let Some(test_module) = modules.get(test) else {
                continue;
            };
            let names: HashSet<&str> = test_module
                .references
                .iter()
                .map(|reference| reference.name.as_str())
                .collect();
            let mut seen: BTreeSet<&str> = BTreeSet::new();
            let mut queue = VecDeque::from([test]);
            while let Some(path) = queue.pop_front() {
                let Some(module) = modules.get(path) else {
                    continue;
                };
                for target in module.imports.iter().filter_map(|i| i.resolved.as_deref()) {
                    if target != test && seen.insert(target) {
                        queue.push_back(target);
                    }
                }
            }
            // A paired module is reached on its own, not through its imports.
            seen.extend(
                self.pairings
                    .iter()
                    .filter(|pairing| pairing.test_file == test && pairing.source_module != test)
                    .map(|pairing| pairing.source_module.as_str()),
            );
            for path in seen.into_iter().filter(|path| modules.contains_key(path)) {
                let referenced: BTreeSet<String> = modules
                    .get(path)
                    .into_iter()
                    .flat_map(|module| &module.exports)
                    .filter(|export| export.kind == ExportKind::Named)
                    .filter(|export| names.contains(export.name.as_str()))
                    .map(|export| export.name.clone())
                    .collect();
                rows.push(TestReach {
                    test_file: test.to_owned(),
                    module: path.to_owned(),
                    referenced: referenced.into_iter().collect(),
                });
            }
        }
        rows
    }
}
