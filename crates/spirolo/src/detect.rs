//! Detecting a project's test stack from its lockfile, `package.json` and runner configs.
//!
//! Detection reads files and nothing else — no install is inspected, no script run. The
//! package manager comes from which lockfile is present; the tools and their versions from
//! `package.json`'s dependencies; the path aliases from `tsconfig`/`jsconfig`; the setup
//! files from the `Gate` rows the model already read out of the runner configs, so `init`
//! and `list` see configs the same way.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use spirolo_model::{Gate, Model};

use crate::rc;
use jsonc_parser::{parse_to_value, JsonValue, ParseOptions};

/// How many `dataLayer` candidates `init` proposes: the most-mocked module specifiers,
/// capped so the commented block stays a shortlist to review rather than a census.
const DATA_LAYER_CANDIDATES: usize = 6;

/// The rule severities `init` writes into the `rules` block, in the registry's order: each
/// rule's name and the severity it carries unless a project overrides it.
#[derive(Debug)]
pub struct RuleDefault {
    /// The rule's name, as `--rule` and the `rules` block spell it.
    pub name: &'static str,
    /// The severity the rule carries by default, as the config spells it.
    pub severity: &'static str,
}

/// What `spirolo init` found in a project.
#[derive(Debug, Default)]
pub struct Detection {
    /// The package manager the lockfile names, or `None` when there is no lockfile.
    pub package_manager: Option<&'static str>,
    /// The recognised test tools, in a fixed catalogue order, each with its declared version.
    pub tools: Vec<Tool>,
    /// The `tsconfig`/`jsconfig` path aliases, sorted by pattern.
    pub aliases: Vec<Alias>,
    /// The setup files the runner configs declare, deduplicated and sorted.
    pub setup_files: Vec<String>,
    /// Globs covering the directories the model found MSW/nock handlers in, sorted; a guess
    /// for the `handlers` key so a first run reads the same files the model did.
    pub handler_globs: Vec<String>,
    /// Modules whose folded constants or exported names look like a base URL, as
    /// `path#name`, sorted; commented `urlSources` candidates.
    pub url_sources: Vec<String>,
    /// The module specifiers module-mocked most often across the suite, sorted; commented
    /// `dataLayer` candidates, since a mock of your own client cannot be told from a mock of
    /// a third party by name alone.
    pub data_layer: Vec<String>,
    /// Markdown files that mention a gate, a threshold or a mutation score, sorted; commented
    /// `claims` candidates for `gate-truth` to diff against the real config.
    pub claims: Vec<String>,
    /// The mutation reports already kept under the tree at Stryker's default path, sorted;
    /// the live `inputs.mutation` value so a first run reads them with no configuration.
    pub mutation_reports: Vec<String>,
    /// Every rule's default severity, in registry order, for the `rules` block.
    pub rules: Vec<RuleDefault>,
}

/// A recognised tool and the version its manifest asked for.
#[derive(Debug)]
pub struct Tool {
    /// The package name, for example `@playwright/test`.
    pub name: String,
    /// The declared version, with a leading range operator stripped for display.
    pub version: String,
}

/// A path alias: the import pattern and the first path it maps to.
#[derive(Debug)]
pub struct Alias {
    /// The import pattern, for example `@/*`.
    pub pattern: String,
    /// The first target the pattern maps to, for example `./src/*`.
    pub target: String,
}

/// Detect the stack rooted at `root`, reading the tree through the already-built `model`.
///
/// Detection reads the model and the config files, never a second, divergent parse: the
/// handler globs come from the model's `handlers`, the URL sources from its folded
/// constants and exports, the data-layer candidates from the module mocks its test cases
/// installed. Markdown claims are the one file read here, since docs are not in the model.
pub fn run(root: &Path, model: &Model) -> Detection {
    Detection {
        package_manager: package_manager(root),
        tools: tools(root),
        aliases: aliases(root),
        setup_files: setup_files(&model.gates),
        handler_globs: handler_globs(model),
        url_sources: url_sources(model),
        data_layer: data_layer(model),
        claims: claims(root),
        mutation_reports: crate::inputs::find_reports(
            root,
            &[rc::DEFAULT_MUTATION_GLOB.to_owned()],
        ),
        rules: rule_defaults(),
    }
}

/// A `<dir>/**` glob for each directory the model found a handler in, sorted and
/// deduplicated, so a first run's `handlers` key reads the same files the model read.
///
/// A handler directly under the root (no parent directory) contributes `*` so the file is
/// still covered without a glob that reaches outside the project.
fn handler_globs(model: &Model) -> Vec<String> {
    let mut dirs: BTreeSet<String> = BTreeSet::new();
    for handler in &model.handlers {
        let dir = match handler.file.rsplit_once('/') {
            Some((parent, _)) => format!("{parent}/**"),
            None => "*".to_owned(),
        };
        dirs.insert(dir);
    }
    dirs.into_iter().collect()
}

/// Whether a folded constant value or an export name reads as a base URL: an absolute URL,
/// an API-versioned path, or a name that spells one of the base-URL idioms.
fn looks_like_base_url(value: &str) -> bool {
    let value = value.trim_start_matches(['`', '"', '\'']);
    value.starts_with("http://")
        || value.starts_with("https://")
        || value.starts_with("/api")
        || value.starts_with("/v")
}

/// Whether an exported binding's name spells a base-URL idiom (`urlApi`, `apiBaseUrl`,
/// `baseUrl`), matched case-insensitively and ignoring underscores so `BASE_URL`, `baseURL`
/// and `base_url` all count.
fn is_base_url_name(name: &str) -> bool {
    const IDIOMS: [&str; 6] = [
        "urlapi",
        "apiurl",
        "apibase",
        "apibaseurl",
        "baseurl",
        "apiroot",
    ];
    let normalised: String = name
        .chars()
        .filter(|c| *c != '_')
        .map(|c| c.to_ascii_lowercase())
        .collect();
    IDIOMS.contains(&normalised.as_str())
}

/// The modules whose folded constants look like a base URL, or which export a base-URL-named
/// binding, as `path#name`, sorted and deduplicated.
///
/// A constant that folded to a URL (`const API = "https://…"`) is caught by its value; a
/// base URL built from `process.env` folds opaque, so a binding *named* like a base URL is
/// caught by its name, which is how a `urlApi` — a `process.env`-interpolated
/// template exported by destructuring — is proposed at all.
fn url_sources(model: &Model) -> Vec<String> {
    let mut sources: BTreeSet<String> = BTreeSet::new();
    for module in &model.modules {
        // A base URL declared in a test is test fixture, not production's URL source; the
        // rule folds handlers against what production emits, so skip test files.
        if is_test_file(&module.path) {
            continue;
        }
        let stem = module.path.strip_suffix(".ts").unwrap_or(&module.path);
        let stem = stem.strip_suffix(".js").unwrap_or(stem);
        for constant in &module.constants {
            if constant.value.as_deref().is_some_and(looks_like_base_url) {
                sources.insert(format!("{stem}#{}", constant.name));
            }
        }
        for export in &module.exports {
            if is_base_url_name(&export.name) {
                sources.insert(format!("{stem}#{}", export.name));
            }
        }
    }
    sources.into_iter().collect()
}

/// Whether a repo-relative path is a test file, by the `.test.` / `.spec.` infix or a
/// `__tests__` directory — the same shapes the recognisers pair as tests.
fn is_test_file(path: &str) -> bool {
    path.contains(".test.")
        || path.contains(".spec.")
        || path.split('/').any(|segment| segment == "__tests__")
}

/// The module specifiers module-mocked most often across the suite, the top
/// [`DATA_LAYER_CANDIDATES`] by count, ties broken by the specifier, so the list is
/// deterministic and the same two runs write the same bytes.
///
/// Reads `Module.mocks`, not `TestCase.mocks`: `vi.mock` is hoisted above the tests, so a
/// module mock is a fact about the file, not a test case, and the file-scoped view is the
/// only one that sees the Supabase-client mocks a spec declares once at its top.
fn data_layer(model: &Model) -> Vec<String> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for module in &model.modules {
        for mock in &module.mocks {
            if mock.kind == spirolo_model::MockKind::Module {
                if let Some(target) = &mock.target {
                    *counts.entry(target.clone()).or_default() += 1;
                }
            }
        }
    }
    let mut ranked: Vec<(String, usize)> = counts.into_iter().collect();
    // Most-mocked first; ties settled by the specifier so the order never wobbles.
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ranked.truncate(DATA_LAYER_CANDIDATES);
    let mut specifiers: Vec<String> = ranked.into_iter().map(|(target, _)| target).collect();
    specifiers.sort();
    specifiers
}

/// The gate vocabulary a claims document is recognised by: a line mentioning one of these,
/// lower-cased, is enough to propose the file for the `claims` key.
const CLAIM_KEYWORDS: [&str; 6] = [
    "enforced",
    "threshold",
    "mutation score",
    "coverage gate",
    "gates a merge",
    "break the build",
];

/// The markdown files under `root` whose text mentions a gate, a threshold or a mutation
/// score, repo-relative and sorted; the commented `claims` candidates.
///
/// This is the one detector that reads a file rather than the model: docs are not parsed
/// into the model. `node_modules` and dot-directories are skipped so a dependency's own
/// testing guide is never proposed as this project's claim.
fn claims(root: &Path) -> Vec<String> {
    let mut files: BTreeSet<String> = BTreeSet::new();
    collect_claims(root, root, &mut files);
    files.into_iter().collect()
}

/// Walk `dir` for markdown claim documents, recording each as a path relative to `root`.
fn collect_claims(root: &Path, dir: &Path, files: &mut BTreeSet<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') || name == "node_modules" {
            continue;
        }
        if path.is_dir() {
            collect_claims(root, &path, files);
        } else if is_markdown(&name) && mentions_a_gate(&path) {
            if let Some(relative) = spirolo_model::relative_slash(root, &path) {
                files.insert(relative);
            }
        }
    }
}

/// Whether a file name has a markdown extension, compared case-insensitively.
fn is_markdown(name: &str) -> bool {
    Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("md") || extension.eq_ignore_ascii_case("mdx")
        })
}

/// Whether a markdown file's text mentions one of the [`CLAIM_KEYWORDS`], read lower-cased.
fn mentions_a_gate(path: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    let lower = text.to_ascii_lowercase();
    CLAIM_KEYWORDS.iter().any(|keyword| lower.contains(keyword))
}

/// Every registered rule's name and default severity, in the registry's order, so the
/// `rules` block lists exactly the rules the binary runs and grows as rules ship.
fn rule_defaults() -> Vec<RuleDefault> {
    spirolo_rules::registry()
        .iter()
        .map(|rule| RuleDefault {
            name: rule.name(),
            severity: spirolo_rules::status::default_severity(rule.as_ref()).name(),
        })
        .collect()
}

/// The package manager a lockfile names, in the order a project usually settles ties.
fn package_manager(root: &Path) -> Option<&'static str> {
    [
        ("pnpm-lock.yaml", "pnpm"),
        ("package-lock.json", "npm"),
        ("yarn.lock", "yarn"),
        ("bun.lockb", "bun"),
        ("bun.lock", "bun"),
    ]
    .into_iter()
    .find(|(file, _)| root.join(file).exists())
    .map(|(_, name)| name)
}

/// The recognised test tools, in the order they are reported.
///
/// Each family is one or more exact package names, the first present naming the tool;
/// Testing Library is matched by the `@testing-library/` prefix, since the family has one
/// package per framework.
fn tools(root: &Path) -> Vec<Tool> {
    let Some(deps) = dependencies(root) else {
        return Vec::new();
    };
    let mut tools = Vec::new();
    tools.extend(first_present(&deps, &["vitest"]));
    tools.extend(first_present(&deps, &["jest"]));
    tools.extend(first_present(&deps, &["@playwright/test"]));
    tools.extend(prefix_present(&deps, "@testing-library/"));
    tools.extend(first_present(&deps, &["msw"]));
    tools.extend(first_present(&deps, &["nock"]));
    tools.extend(first_present(&deps, &["redux-saga"]));
    tools.extend(first_present(
        &deps,
        &["@tanstack/react-query", "react-query"],
    ));
    tools.extend(first_present(&deps, &["@stryker-mutator/core"]));
    tools
}

/// The first of `names` present in `deps`, as a tool with its cleaned version.
fn first_present(deps: &BTreeMap<String, String>, names: &[&str]) -> Option<Tool> {
    names.iter().find_map(|name| {
        deps.get(*name).map(|version| Tool {
            name: (*name).to_owned(),
            version: clean_version(version),
        })
    })
}

/// The first dependency whose name starts with `prefix` (keys are sorted, so the choice is
/// deterministic), as a tool with its cleaned version.
fn prefix_present(deps: &BTreeMap<String, String>, prefix: &str) -> Option<Tool> {
    deps.iter()
        .find(|(name, _)| name.starts_with(prefix))
        .map(|(name, version)| Tool {
            name: name.clone(),
            version: clean_version(version),
        })
}

/// A version spec with a leading range operator stripped, so `^2.1.9` reads as `2.1.9`. A
/// spec with no operator (`workspace:*`, `latest`) is left as it is.
fn clean_version(spec: &str) -> String {
    spec.trim_start_matches(['^', '~', '>', '<', '=', 'v', ' '])
        .to_owned()
}

/// Every `dependencies` and `devDependencies` entry of `package.json`, or `None` when
/// there is no readable manifest.
fn dependencies(root: &Path) -> Option<BTreeMap<String, String>> {
    let source = std::fs::read_to_string(root.join("package.json")).ok()?;
    let value = parse_to_value(&source, &ParseOptions::default()).ok()??;
    let JsonValue::Object(mut object) = value else {
        return Some(BTreeMap::new());
    };
    let mut deps = BTreeMap::new();
    for section in ["dependencies", "devDependencies"] {
        let Some(entries) = object.take_object(section) else {
            continue;
        };
        for (name, spec) in entries {
            if let JsonValue::String(version) = spec {
                deps.insert(name.into_owned(), version.into_owned());
            }
        }
    }
    Some(deps)
}

/// The path aliases the first present of `tsconfig.json` / `jsconfig.json` declares.
fn aliases(root: &Path) -> Vec<Alias> {
    for file in ["tsconfig.json", "jsconfig.json"] {
        if let Some(aliases) = aliases_from(root.join(file)) {
            if !aliases.is_empty() {
                return aliases;
            }
        }
    }
    Vec::new()
}

/// The `compilerOptions.paths` of one config as aliases, sorted by pattern, or `None` when
/// the file is unreadable, unparsable, or declares no paths.
fn aliases_from(path: PathBuf) -> Option<Vec<Alias>> {
    let source = std::fs::read_to_string(path).ok()?;
    let value = parse_to_value(&source, &ParseOptions::default()).ok()??;
    let JsonValue::Object(mut root) = value else {
        return None;
    };
    let paths = root.take_object("compilerOptions")?.take_object("paths")?;
    let mut aliases = Vec::new();
    for (pattern, targets) in paths {
        if let Some(target) = first_target(targets) {
            aliases.push(Alias {
                pattern: pattern.into_owned(),
                target,
            });
        }
    }
    aliases.sort_by(|a, b| a.pattern.cmp(&b.pattern));
    Some(aliases)
}

/// The first target a `paths` entry maps to: the first string of its array, or the value
/// itself when it is a lone string.
fn first_target(targets: JsonValue<'_>) -> Option<String> {
    match targets {
        JsonValue::Array(array) => array.into_iter().find_map(|value| match value {
            JsonValue::String(target) => Some(target.into_owned()),
            _ => None,
        }),
        JsonValue::String(target) => Some(target.into_owned()),
        _ => None,
    }
}

/// The setup files every gate declares, deduplicated and sorted for a stable summary.
fn setup_files(gates: &[Gate]) -> Vec<String> {
    let mut files: Vec<String> = gates
        .iter()
        .flat_map(|gate| gate.setup_files.iter().cloned())
        .collect();
    files.sort();
    files.dedup();
    files
}

#[cfg(test)]
mod tests {
    use spirolo_model::{
        Constant, Export, ExportKind, Handler, HandlerLibrary, Language, Mock, MockKind, Module,
        ParseStatus, Span,
    };

    use super::{
        clean_version, data_layer, handler_globs, is_base_url_name, is_test_file,
        looks_like_base_url, url_sources,
    };

    #[test]
    fn a_caret_or_tilde_range_is_stripped_for_display() {
        assert_eq!(clean_version("^2.1.9"), "2.1.9");
        assert_eq!(clean_version("~2.4.11"), "2.4.11");
        assert_eq!(clean_version(">=1.0.0"), "1.0.0");
        assert_eq!(clean_version("1.47.0"), "1.47.0");
    }

    #[test]
    fn a_non_semver_spec_is_left_alone() {
        assert_eq!(clean_version("workspace:*"), "workspace:*");
        assert_eq!(clean_version("latest"), "latest");
    }

    #[test]
    fn a_base_url_is_an_absolute_url_or_an_api_path() {
        assert!(looks_like_base_url("http://localhost:5000/v1"));
        assert!(looks_like_base_url("https://api.example.com"));
        assert!(looks_like_base_url("/api/v1"));
        assert!(looks_like_base_url("/v1/users"));
        assert!(!looks_like_base_url("just a string"));
        assert!(!looks_like_base_url("./relative/path"));
    }

    #[test]
    fn a_base_url_name_is_matched_case_insensitively() {
        assert!(is_base_url_name("urlApi"));
        assert!(is_base_url_name("BASE_URL"));
        assert!(is_base_url_name("baseURL"));
        assert!(is_base_url_name("apiBaseUrl"));
        assert!(!is_base_url_name("handlers"));
        assert!(!is_base_url_name("urls"));
    }

    #[test]
    fn a_test_file_is_recognised_by_infix_or_tests_dir() {
        assert!(is_test_file("src/app.test.tsx"));
        assert!(is_test_file("src/app.spec.ts"));
        assert!(is_test_file("src/__tests__/app.ts"));
        assert!(!is_test_file("src/_core/config/app.ts"));
    }

    /// A model carrying one module with the given constants, exports and mocks, and the
    /// given handlers — enough for the detectors, which read only those tables.
    fn model(modules: Vec<Module>, handlers: Vec<Handler>) -> spirolo_model::Model {
        spirolo_model::Model {
            modules,
            handlers,
            ..spirolo_model::Model::default()
        }
    }

    fn module(
        path: &str,
        constants: Vec<Constant>,
        exports: Vec<Export>,
        mocks: Vec<Mock>,
    ) -> Module {
        Module {
            path: path.to_owned(),
            language: Language::TypeScript,
            parse_status: ParseStatus::Parsed,
            imports: Vec::new(),
            exports,
            constants,
            mocks,
            references: Vec::new(),
            spies: Vec::new(),
            loads: Vec::new(),
            complexity: 1,
            generated: false,
        }
    }

    fn constant(name: &str, value: &str) -> Constant {
        Constant {
            name: name.to_owned(),
            value: Some(value.to_owned()),
            opaque: false,
            reason: None,
            env_template: None,
            span: Span::default(),
        }
    }

    fn export(name: &str) -> Export {
        Export {
            name: name.to_owned(),
            kind: ExportKind::Named,
            origin: None,
            resolved: None,
            span: Span::default(),
        }
    }

    fn module_mock(target: &str) -> Mock {
        Mock {
            target: Some(target.to_owned()),
            resolved: None,
            reason: None,
            kind: MockKind::Module,
            factory: false,
            span: Span::default(),
        }
    }

    fn handler(file: &str) -> Handler {
        Handler {
            file: file.to_owned(),
            span: Span::default(),
            library: HandlerLibrary::Msw,
            method: None,
            pattern: None,
            pattern_kind: spirolo_model::PatternKind::Opaque,
        }
    }

    #[test]
    fn handler_globs_are_the_directories_the_model_found_handlers_in() {
        let model = model(
            Vec::new(),
            vec![
                handler("src/mocks/handlers/users.ts"),
                handler("src/mocks/handlers/auth.ts"),
                handler("test/msw/server.ts"),
            ],
        );
        assert_eq!(
            handler_globs(&model),
            ["src/mocks/handlers/**", "test/msw/**"]
        );
    }

    #[test]
    fn url_sources_find_a_folded_url_constant_and_a_base_url_named_export() {
        let model = model(
            vec![
                module(
                    "src/config/api.ts",
                    vec![constant("API", "https://api.example.com")],
                    Vec::new(),
                    Vec::new(),
                ),
                module(
                    "src/_core/config/app.ts",
                    Vec::new(),
                    vec![export("urlApi")],
                    Vec::new(),
                ),
            ],
            Vec::new(),
        );
        assert_eq!(
            url_sources(&model),
            ["src/_core/config/app#urlApi", "src/config/api#API"]
        );
    }

    #[test]
    fn url_sources_skip_a_url_declared_in_a_test_file() {
        let model = model(
            vec![module(
                "src/app.test.tsx",
                vec![constant("NON_FILE_URL", "/api/sops/x")],
                Vec::new(),
                Vec::new(),
            )],
            Vec::new(),
        );
        assert!(url_sources(&model).is_empty());
    }

    #[test]
    fn data_layer_ranks_the_most_module_mocked_specifiers() {
        // supabase mocked 3×, axios 2×, a rare one once: the shortlist keeps the frequent
        // two, in sorted order, and the whole thing is capped, so a wide suite stays a list.
        let model = model(
            vec![
                module(
                    "a.test.ts",
                    Vec::new(),
                    Vec::new(),
                    vec![module_mock("@/supabaseClient"), module_mock("axios")],
                ),
                module(
                    "b.test.ts",
                    Vec::new(),
                    Vec::new(),
                    vec![module_mock("@/supabaseClient"), module_mock("axios")],
                ),
                module(
                    "c.test.ts",
                    Vec::new(),
                    Vec::new(),
                    vec![module_mock("@/supabaseClient"), module_mock("rare-thing")],
                ),
            ],
            Vec::new(),
        );
        assert_eq!(
            data_layer(&model),
            ["@/supabaseClient", "axios", "rare-thing"]
        );
    }
}
