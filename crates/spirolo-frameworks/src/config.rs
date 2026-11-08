//! The config recogniser: runner and mutation configs by AST into the `Gate` table.
//!
//! A config file is recognised by its name — `vitest.config.*`, `vite.config.*`,
//! `jest.config.*`, `playwright.config.*`, `stryker.conf.*` — and its exported object
//! literal is read into a [`Gate`]. A `vite.config.*` is a build config first: it is a
//! gate only when it configures the runner — a readable part declares a `test` key, or the
//! file imports `vitest/config` — so a plain bundler config never inflates the count.
//!
//! The JSON spellings are read too: the model hands `jest.config.json`,
//! `stryker.conf.json` / `stryker.config.json` and every `package.json` over as the
//! program `module.exports = <json>;` with the language marked `Json`, so the same
//! reader serves them. A `package.json` is a gate only through its `jest` key, at the
//! manifest's own path.
//!
//! The reading itself — finding the export, unwrapping `defineConfig`, flattening
//! `mergeConfig`, reading each tool's fields and merging the parts — is the model's
//! [`spirolo_model::config_reader`], shared with the preset follower so the kit's own file and
//! the presets it inherits are read by one code path. This recogniser sees one file: a
//! part that is an imported preset is recorded as written in [`Gate::preset_imports`], and
//! the model follows it afterwards, through the resolver, deciding then whether the gate
//! was read whole.
//!
//! Nothing is executed.

use oxc_ast::ast::Statement;
use spirolo_model::config_reader::{
    export_parts, object_value, prop, read_config, read_parts, ConfigPart,
};
use spirolo_model::{FileContext, Gate, GateTool, Language, Recogniser};

/// Recognises runner and mutation config files. See the [module docs](self).
#[derive(Debug, Default, Clone, Copy)]
pub struct Configs;

impl Recogniser for Configs {
    fn gates(&self, file: &FileContext<'_>) -> Vec<Gate> {
        let Some(tool) = tool_of(file.path) else {
            if file.language == Language::Json && is_package_json(file.path) {
                return package_json_gate(file);
            }
            return Vec::new();
        };
        let parts = export_parts(file.program);
        if is_vite_config(file.path) && !configures_vitest(&parts, file.program) {
            return Vec::new();
        }
        let mut gate = Gate::blank(file.path, tool);
        read_parts(tool, &parts, &mut gate);
        vec![gate]
    }
}

/// The tool a config filename names, or `None` when the file is not a config.
fn tool_of(path: &str) -> Option<GateTool> {
    let name = path.rsplit('/').next().unwrap_or(path);
    let stem = name.split('.').next().unwrap_or(name);
    // `stryker.conf.js` and `stryker.config.js`; every other tool spells it `.config.`.
    let is_config = name.contains(".config.") || name.contains(".conf.");
    if !is_config {
        return None;
    }
    match stem {
        "vitest" | "vite" => Some(GateTool::Vitest),
        "jest" => Some(GateTool::Jest),
        "playwright" => Some(GateTool::Playwright),
        "stryker" => Some(GateTool::Stryker),
        _ => None,
    }
}

/// Whether the file is a `package.json`.
fn is_package_json(path: &str) -> bool {
    path.rsplit('/').next() == Some("package.json")
}

/// The Jest gate a `package.json` declares under its `jest` key, if it has one. Jest reads
/// this block exactly as it reads a `jest.config.*` export, so the row is a Jest gate at
/// the manifest's path; a manifest with no such key gates nothing.
fn package_json_gate(file: &FileContext<'_>) -> Vec<Gate> {
    let parts = export_parts(file.program);
    let [ConfigPart::Object(object)] = parts.as_slice() else {
        return Vec::new();
    };
    let Some(jest) = prop(object, "jest").and_then(object_value) else {
        return Vec::new();
    };
    let mut gate = Gate::blank(file.path, GateTool::Jest);
    read_config(GateTool::Jest, jest, &mut gate);
    vec![gate]
}

/// Whether the file is a `vite.config.*` — Vite's own config, which only gates tests when
/// it carries Vitest's `test` block.
fn is_vite_config(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.split('.').next() == Some("vite")
}

/// Whether a `vite.config.*` configures Vitest: some readable part of its export declares
/// a `test` key, or the file imports from `vitest/config` (which says so even when the
/// export itself is a function the reader does not evaluate). A config that does neither
/// — an opaque export written against `vite`'s `defineConfig` — is not decidable
/// statically and is left out rather than guessed at.
fn configures_vitest(parts: &[ConfigPart<'_>], program: &oxc_ast::ast::Program<'_>) -> bool {
    let has_test_key = parts.iter().any(|part| match part {
        ConfigPart::Object(object) => prop(object, "test").is_some(),
        ConfigPart::Import(_) | ConfigPart::Unreadable(_) => false,
    });
    has_test_key || imports_vitest_config(program)
}

/// Whether the program has an `import … from 'vitest/config'`.
fn imports_vitest_config(program: &oxc_ast::ast::Program<'_>) -> bool {
    program.body.iter().any(|statement| {
        matches!(
            statement,
            Statement::ImportDeclaration(import) if import.source.value == "vitest/config"
        )
    })
}
