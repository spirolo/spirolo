//! Turning every import specifier into the repo-relative module it names.
//!
//! Resolution runs once, after the whole file set is extracted, because it needs facts
//! the parallel phase cannot have: which `tsconfig` paths and bundler aliases are in
//! play, and which workspace packages exist. It leans on oxc's resolver — the same
//! Node/TypeScript algorithm the runner sees — so a specifier resolves the way the tool
//! being modelled would resolve it.
//!
//! The outcomes for one import, all but the first recorded in the row's `reason`:
//! - resolves to a file inside the project → `resolved` is its repo-relative path;
//! - resolves outside the project (an installed dependency under `node_modules`, or a
//!   file above the root) → left unresolved as `external` / `outside_root`, because
//!   neither is a module we table;
//! - a *relative* specifier the resolver looked for and did not find → left unresolved as
//!   `missing` with an `unresolved_import` diagnostic, because a broken relative import is
//!   a real fault;
//! - a *relative* specifier the resolver failed on for any other reason (a `tsconfig` whose
//!   `extends` names an uninstalled preset fails every import in its package) → left
//!   unresolved as `unknown`, because the failure says nothing about the file;
//! - a *relative* specifier that resolves once a React Native platform suffix (`.ios`,
//!   `.android`, `.native`, `.web`) is put before the extension → that file;
//! - a *relative* specifier that lands in a directory its package builds into (`lib/`,
//!   `dist/`, a `main` entry's directory the checkout lacks, a script's `--out-dir`) → left
//!   unresolved as `build_output`, because the build makes the module before the suite runs.
//!
//! An `import(x)` / `require(x)` with a non-literal specifier arrives already `computed`
//! and is not looked up.
//!
//! A Node builtin (`node:fs`, `path`) is left unresolved as `builtin`: no package supplies
//! it and it imports nothing of the project.
//!
//! A bare specifier that resolves to nothing is left unresolved as `unknown_package` with
//! no diagnostic: without the dependency installed it cannot be told apart from an
//! external package, and guessing would only manufacture false diagnostics.
//!
//! A re-export's `Export` row then points at its origin: the resolved path of the
//! `re_export` import with the same specifier, once that import is resolved. A module
//! mock's target (`vi.mock("./db")`) goes through the same resolver from the test file,
//! so a rule can compare the factory against the real module; a relative target that
//! resolves to nothing is an `unresolved_mock` diagnostic, like a broken import.

use std::path::Path;

use std::path::PathBuf;

use oxc_resolver::{
    Alias, AliasValue, Resolution, ResolveError, ResolveOptions, Resolver, TsconfigDiscovery,
};

use crate::manifest::Manifest;
use crate::paths::relative_slash;
use crate::{Diagnostic, Import, Model, Severity};

/// Resolve every import in every module, filling `resolved` and appending any
/// `unresolved_import` diagnostics. `manifests` are the packages the walk found, whose
/// names become workspace aliases. Mutates `model` in place; callers sort afterwards.
///
/// Returns the resolver, warm with every tsconfig and package it has read, so the later
/// steps that resolve from the same files — the preset reader, the mock targets — reuse
/// that cache instead of discovering each directory's config again.
pub fn resolve(model: &mut Model, manifests: &[Manifest]) -> WorkspaceResolver {
    let root = model.root.clone();
    let resolver = WorkspaceResolver::new(&root, manifests);

    let mappers = module_name_mappers(model);
    let mut diagnostics = Vec::new();
    // The tsconfigs the resolver could not load, each once, with the first error it gave.
    let mut unloadable: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();
    for module in &mut model.modules {
        let file = root.join(&module.path);
        for import in &mut module.imports {
            // A `computed` import (`import(specifier)`) names no module to look up.
            if import.reason.is_some() {
                continue;
            }
            let result = resolver.resolve_file(&file, &import.specifier);
            if let Err(error) = &result {
                if is_tsconfig_failure(error) {
                    unloadable
                        .entry(nearest_tsconfig(&root, &module.path))
                        .or_insert_with(|| error.to_string());
                }
            }
            let (mut path, mut why) = classify(&root, &import.specifier, result);
            // React Native's bundler, and jest-expo's resolver, try a platform suffix before
            // the extension: `../default-extension` loads `default-extension.ios.js`.
            if why == Some("missing") {
                if let Some(platform) = PLATFORMS.iter().find_map(|platform| {
                    let suffixed = format!("{}.{platform}", import.specifier);
                    match classify(&root, &suffixed, resolver.resolve_file(&file, &suffixed)) {
                        (Some(found), None) => Some(found),
                        _ => None,
                    }
                }) {
                    path = Some(platform);
                    why = None;
                }
            }
            // Jest loads a bare specifier its `moduleNameMapper` matches from the mapped path
            // (`@scope/(.*)` → `<rootDir>/packages/$1/src`), not from the package's entries.
            if why == Some("unknown_package") {
                if let Some(mapped) =
                    map_module_name(&mappers, &module.path, &import.specifier, |target| {
                        classify(&root, target, resolver.resolve_file(&file, target))
                    })
                {
                    path = Some(mapped);
                    why = None;
                }
            }
            if why == Some("missing")
                && is_build_output(&root, &module.path, &import.specifier, manifests)
            {
                why = Some("build_output");
            }
            import.resolved = path;
            import.reason = why.map(str::to_owned);
            if why == Some("missing") {
                diagnostics.push(unresolved(&module.path, import));
            }
        }
        // The file-scoped module mocks resolve from the same file as its imports do.
        for mock in &mut module.mocks {
            let Some(target) = mock.target.clone() else {
                continue;
            };
            let (path, why) = classify(&root, &target, resolver.resolve_file(&file, &target));
            mock.resolved = path;
            mock.reason = why.map(str::to_owned);
        }
        // So do the modules it loads through the runner's registry (`jest.requireActual`).
        for load in &mut module.loads {
            let (path, why) = classify(
                &root,
                &load.specifier,
                resolver.resolve_file(&file, &load.specifier),
            );
            load.resolved = path;
            load.reason = why.map(str::to_owned);
        }
        // A re-export points at where its origin resolved, through the import row that
        // carries the same specifier.
        for export in &mut module.exports {
            let Some(origin) = &export.origin else {
                continue;
            };
            export.resolved = module
                .imports
                .iter()
                .find(|import| {
                    import.kind == crate::ImportKind::ReExport && &import.specifier == origin
                })
                .and_then(|import| import.resolved.clone());
        }
    }
    diagnostics.extend(unloadable.into_iter().map(|(config, error)| Diagnostic {
        code: "tsconfig_unloadable".to_owned(),
        severity: Severity::Warning,
        message: format!(
            "cannot load {config} ({error}); imports under it are left unknown, not missing"
        ),
        file: Some(config),
        span: None,
    }));
    model.diagnostics.extend(diagnostics);
    resolver
}

/// The project's resolver: oxc with every workspace package aliased to its directory, plus
/// the one case an alias cannot express. An alias resolves the package directory through
/// `main`, never through the package's `exports` map, and a failed alias stops resolution
/// before `node_modules` is tried — so a workspace package that declares only `exports`
/// (`"." : "./src/index.ts"`, `"./client": …`) would never resolve, installed or not. When
/// the aliased lookup fails on a workspace package's name, the specifier is resolved again
/// from inside that package, where Node's self-reference applies its `exports` map: the
/// target the package manager's symlink would give the runner.
pub struct WorkspaceResolver {
    /// oxc with the workspace aliases.
    aliased: Resolver,
    /// The same options and cache without the aliases, for the self-reference retry.
    unaliased: Resolver,
    /// Every workspace package's name and absolute directory, longest name first so a
    /// nested name wins.
    packages: Vec<(String, PathBuf)>,
}

impl WorkspaceResolver {
    /// A resolver for the project at `root` whose workspace packages are `manifests`.
    pub fn new(root: &Path, manifests: &[Manifest]) -> Self {
        let options = options(root, manifests);
        let aliased = Resolver::new(options.clone());
        let unaliased = aliased.clone_with_options(ResolveOptions {
            alias: Vec::new(),
            ..options
        });
        let mut packages: Vec<(String, PathBuf)> = manifests
            .iter()
            .filter_map(|m| Some((m.name.clone()?, root.join(&m.dir))))
            .collect();
        packages.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then_with(|| a.0.cmp(&b.0)));
        Self {
            aliased,
            unaliased,
            packages,
        }
    }

    /// Resolve `specifier` as an import written in `file` would resolve.
    pub fn resolve_file(&self, file: &Path, specifier: &str) -> Result<Resolution, ResolveError> {
        let error = match self.aliased.resolve_file(file, specifier) {
            Ok(resolution) => return Ok(resolution),
            Err(error) => error,
        };
        let Some((_, dir)) = self.packages.iter().find(|(name, _)| {
            specifier == name
                || specifier
                    .strip_prefix(name.as_str())
                    .is_some_and(|rest| rest.starts_with('/'))
        }) else {
            return Err(error);
        };
        self.unaliased.resolve(dir, specifier).map_err(|_| error)
    }

    /// The underlying oxc resolver, warm with every config and package read so far.
    pub const fn oxc(&self) -> &Resolver {
        &self.aliased
    }
}

/// Fill `resolved` on every module mock whose target is a specifier, resolving it from
/// the test file the way an import from that file would resolve, through the resolver
/// [`resolve`] returned. Runs after `scope`, so only the cases the runner collects are
/// judged. A file-wide mock is copied onto every case in the file, so resolutions are
/// memoised per (file, target) and a broken relative target is reported once per site,
/// not once per case. Mutates `model` in place; callers sort afterwards.
pub fn resolve_mocks(model: &mut Model, resolver: &WorkspaceResolver) {
    let root = model.root.clone();
    let mut diagnostics = Vec::new();
    let mut memo: std::collections::HashMap<(String, String), Target> =
        std::collections::HashMap::new();
    let mut reported: std::collections::HashSet<(String, u32)> = std::collections::HashSet::new();
    for case in &mut model.test_cases {
        for mock in &mut case.mocks {
            if mock.kind != crate::MockKind::Module {
                continue;
            }
            let Some(target) = mock.target.clone() else {
                continue;
            };
            let key = (case.file.clone(), target.clone());
            let answer = memo.entry(key).or_insert_with(|| {
                let file = root.join(&case.file);
                classify(&root, &target, resolver.resolve_file(&file, &target))
            });
            mock.resolved.clone_from(&answer.0);
            mock.reason = answer.1.map(str::to_owned);
            if answer.0.is_none()
                && is_relative(&target)
                && reported.insert((case.file.clone(), mock.span.start))
            {
                diagnostics.push(Diagnostic {
                    code: "unresolved_mock".to_owned(),
                    severity: Severity::Warning,
                    file: Some(case.file.clone()),
                    span: Some(mock.span),
                    message: format!("cannot resolve mocked module {target}"),
                });
            }
        }
    }
    model.diagnostics.extend(diagnostics);
}

/// The `unresolved_import` diagnostic for an import that points into the project but has
/// no file behind it: a broken relative specifier or a dead alias target.
/// A resolved repo-relative path, or `None` and the reason it has none.
type Target = (Option<String>, Option<&'static str>);

/// Where a specifier leads, from one resolver answer: the repo-relative path of the project
/// module it names, or `None` and why (the `reason` vocabulary of an [`Import`] row).
fn classify(root: &Path, specifier: &str, result: Result<Resolution, ResolveError>) -> Target {
    match result {
        // Inside the project: table it. Under a `node_modules` (installed, or a package
        // manager's store inside the repo): external, leave it.
        Ok(resolution) => match relative_slash(root, resolution.path()) {
            Some(spelling) if is_installed(&spelling) => (None, Some("external")),
            Some(spelling) => (Some(spelling), None),
            None => (None, Some("outside_root")),
        },
        // Node's own modules (`node:fs`, `path`): no package supplies them.
        Err(ResolveError::Builtin { .. }) => (None, Some("builtin")),
        Err(error) if is_relative(specifier) && is_not_found(&error) => (None, Some("missing")),
        // Any other failure on a relative specifier — a tsconfig that cannot be loaded, an
        // unreadable file — says nothing about whether the module is there: unknown, never
        // missing.
        Err(_) if is_relative(specifier) => (None, Some("unknown")),
        // A bare specifier that did not resolve is treated as an uninstalled external
        // dependency, not a fault: a reason, no diagnostic.
        Err(_) => (None, Some("unknown_package")),
    }
}

/// One Jest config's `moduleNameMapper`: the repo-relative `rootDir` it applies under, its
/// absolute spelling for `<rootDir>`, and each compiled pattern with its mapped paths.
type Mapper = (String, String, Vec<(regex::Regex, Vec<String>)>);

/// The `moduleNameMapper` of every Jest config, deepest `rootDir` first so the config
/// nearest a file maps it. A pattern the regex engine cannot compile is left out.
fn module_name_mappers(model: &Model) -> Vec<Mapper> {
    let mut mappers: Vec<Mapper> = model
        .gates
        .iter()
        .filter(|gate| gate.tool == crate::GateTool::Jest && !gate.module_name_mapper.is_empty())
        .map(|gate| {
            let dir = gate.collection_root.clone().unwrap_or_else(|| {
                gate.source
                    .rsplit_once('/')
                    .map_or(String::new(), |(dir, _)| dir.to_owned())
            });
            let spelling = model.root.join(&dir).to_string_lossy().into_owned();
            let patterns = gate
                .module_name_mapper
                .iter()
                .filter_map(|(pattern, targets)| {
                    Some((regex::Regex::new(pattern).ok()?, targets.clone()))
                })
                .collect();
            (dir, spelling, patterns)
        })
        .collect();
    mappers.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then_with(|| a.0.cmp(&b.0)));
    mappers
}

/// The project module a bare `specifier` written in `file` maps to under the nearest Jest
/// config whose `rootDir` holds the file: the first pattern that matches decides, and the
/// first of its paths that `resolve` finds in the project is the module.
fn map_module_name(
    mappers: &[Mapper],
    file: &str,
    specifier: &str,
    resolve: impl Fn(&str) -> Target,
) -> Option<String> {
    let (_, root_dir, patterns) = mappers.iter().find(|(dir, ..)| {
        dir.is_empty()
            || file
                .strip_prefix(dir.as_str())
                .is_some_and(|rest| rest.starts_with('/'))
    })?;
    let (pattern, targets) = patterns
        .iter()
        .find(|(pattern, _)| pattern.is_match(specifier))?;
    let captures = pattern.captures(specifier)?;
    targets.iter().find_map(|target| {
        let mut mapped = target.replace("<rootDir>", root_dir);
        for group in (1..captures.len()).rev() {
            let text = captures.get(group).map_or("", |found| found.as_str());
            mapped = mapped.replace(&format!("${group}"), text);
        }
        match resolve(&mapped) {
            (Some(found), None) => Some(found),
            _ => None,
        }
    })
}

/// The React Native platform suffixes a module may carry before its extension.
pub const PLATFORMS: [&str; 4] = ["ios", "android", "native", "web"];

/// Whether a relative specifier that resolved to nothing lands in a directory its package
/// builds into, so the module exists wherever the suite runs after the build: the first
/// directory of a `main` / `module` / `types` / `exports` entry the checkout does not have,
/// the `--out-dir` / `--outDir` / `-d` of a script, or a gitignored `lib/` or `dist/` beside
/// the `package.json`. An entry directory the checkout has (`main: src/index.js`) or a
/// `lib/` the repo commits is source, so a file missing from it was deleted, not left
/// unbuilt.
fn is_build_output(root: &Path, file: &str, specifier: &str, manifests: &[Manifest]) -> bool {
    let dir = file.rfind('/').map_or("", |slash| &file[..slash]);
    let Some(target) = crate::paths::join_relative(dir, specifier) else {
        return false;
    };
    let Some(manifest) = manifests
        .iter()
        .filter(|manifest| {
            manifest.dir.is_empty()
                || target
                    .strip_prefix(&manifest.dir)
                    .is_some_and(|rest| rest.starts_with('/'))
        })
        .max_by_key(|manifest| manifest.dir.len())
    else {
        return false;
    };
    let inside = if manifest.dir.is_empty() {
        target.as_str()
    } else {
        &target[manifest.dir.len() + 1..]
    };
    let Some(first) = inside.split('/').next().filter(|first| !first.is_empty()) else {
        return false;
    };
    let entry_dirs = manifest.entries.iter().filter_map(|entry| {
        let entry = entry.trim_start_matches("./");
        entry.split_once('/').map(|(dir, _)| dir)
    });
    let out_dirs = manifest.scripts.iter().flat_map(|script| {
        let words: Vec<&str> = script.split_whitespace().collect();
        words
            .windows(2)
            .filter(|pair| matches!(pair[0], "--out-dir" | "--outDir" | "-d"))
            .map(|pair| pair[1].trim_start_matches("./").trim_end_matches('/'))
            .collect::<Vec<_>>()
    });
    let package = Path::new(&manifest.dir);
    (["lib", "dist"].contains(&first) && is_gitignored(root, &package.join(first)))
        || (entry_dirs.into_iter().any(|dir| dir == first)
            && !root.join(&manifest.dir).join(first).is_dir())
        || out_dirs.into_iter().any(|dir| dir == first)
}

/// Whether the repo-relative directory `dir` is ignored by the `.gitignore` files from the
/// root down to it, the deepest one that matches deciding.
fn is_gitignored(root: &Path, dir: &Path) -> bool {
    dir.ancestors()
        .find_map(|base| {
            let ignores = root.join(base).join(".gitignore");
            if !ignores.is_file() {
                return None;
            }
            let mut builder = ignore::gitignore::GitignoreBuilder::new(root.join(base));
            builder.add(ignores);
            let matcher = builder.build().ok()?;
            match matcher.matched_path_or_any_parents(root.join(dir), true) {
                ignore::Match::None => None,
                found => Some(found.is_ignore()),
            }
        })
        .unwrap_or(false)
}

fn unresolved(file: &str, import: &Import) -> Diagnostic {
    Diagnostic {
        code: "unresolved_import".to_owned(),
        severity: Severity::Warning,
        file: Some(file.to_owned()),
        span: Some(import.span),
        message: format!("cannot resolve import {}", import.specifier),
    }
}

/// Whether a repo-relative path lies under a `node_modules` directory — an installed
/// dependency, not a module of the project.
fn is_installed(spelling: &str) -> bool {
    spelling.split('/').any(|segment| segment == "node_modules")
}

/// Whether the resolver looked and found nothing — the only failure that says the module
/// is not there. A dead alias target and an extension-aliased specifier with no file behind
/// any of its extensions are not-found too.
const fn is_not_found(error: &ResolveError) -> bool {
    matches!(
        error,
        ResolveError::NotFound(_)
            | ResolveError::MatchedAliasNotFound(..)
            | ResolveError::ExtensionAlias(..)
    )
}

/// Whether the resolver failed on the file's tsconfig rather than on the specifier: an
/// `extends` it cannot find (a preset only an install provides), a config it cannot parse,
/// a circular or self reference.
const fn is_tsconfig_failure(error: &ResolveError) -> bool {
    matches!(
        error,
        ResolveError::TsconfigNotFound(_)
            | ResolveError::TsconfigLoadFailed { .. }
            | ResolveError::TsconfigCircularExtend(_)
            | ResolveError::TsconfigSelfReference(_)
    )
}

/// The repo-relative `tsconfig.json` nearest above `module`, the one auto discovery
/// applies to it. oxc names only the preset it could not find, never the config that
/// extends it; with no config on disk the module itself stands in.
fn nearest_tsconfig(root: &Path, module: &str) -> String {
    Path::new(module)
        .ancestors()
        .skip(1)
        .map(|dir| dir.join("tsconfig.json"))
        .find(|config| root.join(config).is_file())
        .and_then(|config| config.to_str().map(|spelling| spelling.replace('\\', "/")))
        .unwrap_or_else(|| module.to_owned())
}

/// A specifier the resolver must be able to satisfy from the tree alone.
fn is_relative(specifier: &str) -> bool {
    specifier.starts_with("./")
        || specifier.starts_with("../")
        || specifier == "."
        || specifier == ".."
}

/// Resolver options tuned for a TypeScript/JavaScript test suite: every dialect
/// extension, `tsconfig` `paths`/`baseUrl` discovered per file, workspace packages
/// aliased to their directories, and Yarn `PnP` enabled.
///
/// Shared with [`crate::presets`], which follows a config's imported preset through the
/// same resolver the model uses everywhere else, so a preset specifier resolves the way
/// the runner would resolve it.
pub fn options(root: &Path, manifests: &[Manifest]) -> ResolveOptions {
    let mut options = ResolveOptions {
        // Auto discovers the nearest tsconfig/jsconfig from each importing file, so a
        // per-package config in a monorepo applies to that package's files.
        tsconfig: Some(TsconfigDiscovery::Auto),
        alias: workspace_aliases(root, manifests),
        extensions: [
            ".ts", ".tsx", ".d.ts", ".mts", ".cts", ".js", ".jsx", ".mjs", ".cjs", ".json",
        ]
        .iter()
        .map(|extension| (*extension).to_owned())
        .collect(),
        // TypeScript ESM writes the runtime extension in the specifier — `import './x.js'`
        // for `x.ts` — so a `.js`/`.mjs`/`.cjs`/`.jsx` import must try the TS source first,
        // the way `tsc`, Vite and Vitest do. Without this a whole ESM package's relative
        // imports read as unresolved.
        extension_alias: [
            (".js", vec![".ts", ".tsx", ".js", ".jsx"]),
            (".mjs", vec![".mts", ".mjs"]),
            (".cjs", vec![".cts", ".cjs"]),
            (".jsx", vec![".tsx", ".jsx"]),
        ]
        .into_iter()
        .map(|(extension, targets)| {
            (
                extension.to_owned(),
                targets.into_iter().map(str::to_owned).collect(),
            )
        })
        .collect(),
        // A workspace package's entry is usually its `module` (ESM) then `main`.
        main_fields: vec!["module".to_owned(), "main".to_owned()],
        // `fs` and `node:fs` answer as Node builtins, never as an uninstalled package.
        builtin_modules: true,
        condition_names: ["import", "require", "default", "node", "types"]
            .iter()
            .map(|condition| (*condition).to_owned())
            .collect(),
        ..ResolveOptions::default()
    };
    // Yarn Plug'n'Play resolution, only for a project that ships a manifest: with the
    // flag on and no `.pnp.cjs` to read, the resolver fails every bare specifier — the
    // whole workspace alias table would be dead — instead of falling back to
    // `node_modules`.
    options.yarn_pnp = root.join(".pnp.cjs").is_file() || root.join(".pnp.js").is_file();
    options
}

/// Map every workspace package's name to its directory, so a cross-package import like
/// `@scope/pkg-b` resolves the way the package manager's symlink would make it.
///
/// The packages are the `package.json` files the walk found, so a manifest under a
/// `.gitignore`d or skipped directory is no alias — the same tree the tables describe.
/// A package with no `name` contributes no alias. Manifests arrive in path order, and a
/// stable sort by name keeps that order among duplicates, so the resolver — which takes
/// the first matching alias — sees the same table on every run.
fn workspace_aliases(root: &Path, manifests: &[Manifest]) -> Alias {
    let mut aliases: Vec<(String, Vec<AliasValue>)> = manifests
        .iter()
        .filter_map(|manifest| {
            let name = manifest.name.clone()?;
            let dir = root.join(&manifest.dir);
            let spelling = dir.to_str()?.to_owned();
            Some((name, vec![AliasValue::Path(spelling)]))
        })
        .collect();
    aliases.sort_by(|a, b| a.0.cmp(&b.0));
    aliases
}
