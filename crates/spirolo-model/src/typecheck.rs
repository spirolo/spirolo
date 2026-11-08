//! Which type tests (`*.test-d.ts`) a package's scripts type-check.
//!
//! A type test registers `describe`/`it` like any suite, but it runs only under a type
//! check: `vitest --typecheck` (or `vitest typecheck`), whose default typecheck include is
//! `**/*.{test,spec}-d.?(c|m)[jt]s?(x)`, or `tsc` over a tsconfig whose `include` names
//! it. A type test such a script covers is not a test that never runs, so `orphan-test`
//! asks here before calling one never collected. A tsconfig the reader cannot read whole —
//! not literal JSON, or an `include` it would inherit through `extends` — covers an unknown
//! set, read as everything under its directory rather than as nothing.

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;

use crate::config_reader::{export_parts, literal_strings, prop, ConfigPart};
use crate::manifest::Manifest;
use crate::scope::invokes;

/// Vitest's default `typecheck.include`, spelled for [`globset`].
pub const VITEST_TYPECHECK_INCLUDE: [&str; 1] = ["**/*.{test,spec}-d.?(c|m)[jt]s?(x)"];

/// One type check a script runs: the directory its globs are relative to, and the globs.
pub struct TypeCheck {
    /// Repo-relative directory the globs are relative to.
    pub dir: String,
    /// The globs the check covers.
    pub include: Vec<String>,
    /// The globs it leaves out.
    pub exclude: Vec<String>,
}

/// Whether a repo-relative path is a type test by Vitest's naming.
pub fn is_type_test(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.contains(".test-d.") || name.contains(".spec-d.")
}

/// The type checks every manifest's scripts run, the tsconfigs read from under `root`.
pub fn checks(root: &std::path::Path, manifests: &[Manifest]) -> Vec<TypeCheck> {
    let mut checks = Vec::new();
    for manifest in manifests {
        for script in &manifest.scripts {
            if vitest_typecheck(script) {
                checks.push(TypeCheck {
                    dir: manifest.dir.clone(),
                    include: VITEST_TYPECHECK_INCLUDE.map(str::to_owned).to_vec(),
                    exclude: Vec::new(),
                });
            }
            if let Some(project) = tsc_project(script) {
                checks.push(tsc_check(root, &manifest.dir, &project));
            }
        }
    }
    checks
}

/// Whether a script runs Vitest's type check.
fn vitest_typecheck(script: &str) -> bool {
    invokes(script, "vitest")
        && script.split_whitespace().any(|word| {
            word == "--typecheck" || word.starts_with("--typecheck=") || word == "typecheck"
        })
}

/// The tsconfig a script's `tsc` reads, relative to its package: the `-p` / `--project` /
/// `-b` / `--build` argument (a file, or a directory holding `tsconfig.json`), else
/// `tsconfig.json`. `None` when the script runs no `tsc`.
fn tsc_project(script: &str) -> Option<String> {
    let mut words = script
        .split(|c: char| c.is_whitespace() || c == '&' || c == ';' || c == '|')
        .filter(|word| !word.is_empty())
        .skip_while(|word| *word != "tsc" && !word.ends_with("/tsc"));
    words.next()?;
    let mut project = "tsconfig.json".to_owned();
    while let Some(word) = words.next() {
        if matches!(word, "-p" | "--project" | "-b" | "--build") {
            match words.next() {
                Some(next) if !next.starts_with('-') => next.clone_into(&mut project),
                _ => break,
            }
        } else if !word.starts_with('-') {
            break;
        }
    }
    let project = project.trim_start_matches("./");
    Some(
        if std::path::Path::new(project)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
        {
            project.to_owned()
        } else {
            format!("{}/tsconfig.json", project.trim_end_matches('/'))
        },
    )
}

/// The check `tsc` makes from `project` (relative to `dir`): the tsconfig's `include`
/// (TypeScript's `**/*` when it sets none and extends nothing) less its `exclude`. A config
/// that cannot be read whole covers everything under its directory.
fn tsc_check(root: &std::path::Path, dir: &str, project: &str) -> TypeCheck {
    let path = if dir.is_empty() {
        project.to_owned()
    } else {
        format!("{dir}/{project}")
    };
    let config_dir = path
        .rfind('/')
        .map_or("", |slash| &path[..slash])
        .to_owned();
    let (include, exclude) = std::fs::read_to_string(root.join(&path))
        .ok()
        .and_then(|source| tsconfig_globs(&source))
        .unwrap_or_else(|| (vec!["**/*".to_owned()], Vec::new()));
    TypeCheck {
        dir: config_dir,
        include,
        exclude,
    }
}

/// A tsconfig's `include` and `exclude`, a bare directory in `include` read as everything
/// under it; `None` when either is not literal or `include` would come from `extends`.
fn tsconfig_globs(source: &str) -> Option<(Vec<String>, Vec<String>)> {
    // A tsconfig is JSON with comments and trailing commas, which a JavaScript object
    // literal accepts as written.
    let wrapped = format!("module.exports = {source}");
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &wrapped, SourceType::cjs()).parse();
    let parts = export_parts(&parsed.program);
    let [ConfigPart::Object(config)] = parts.as_slice() else {
        return None;
    };
    let include = match prop(config, "include") {
        Some(include) => literal_strings(include)?,
        None if prop(config, "extends").is_some() => return None,
        None => vec!["**/*".to_owned()],
    };
    let exclude = match prop(config, "exclude") {
        Some(exclude) => literal_strings(exclude)?,
        None => Vec::new(),
    };
    let include = include
        .into_iter()
        .map(|glob| {
            let glob = glob.trim_start_matches("./").trim_end_matches('/');
            let last = glob.rsplit('/').next().unwrap_or(glob);
            if last.contains('*') || last.contains('.') {
                glob.to_owned()
            } else {
                format!("{glob}/**/*")
            }
        })
        .collect();
    Some((include, exclude))
}
