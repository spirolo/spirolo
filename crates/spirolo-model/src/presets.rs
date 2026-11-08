//! Following a config's imported presets to what they declare.
//!
//! Every kit in a monorepo of shared kits inherits its config from a shared preset, in one
//! of a few shapes:
//!
//! ```js
//! import variantConfig from '@acme/config-vitest/react'
//! export default mergeConfig(variantConfig, defineConfig({ test: { setupFiles: […] } }))
//! ```
//!
//! ```js
//! import baseConfig from '@acme/config-vitest/base'
//! export default baseConfig                       // a bare re-export
//! ```
//!
//! The kit declares little or nothing of its own — its `include`, its `setupFiles`, its
//! coverage thresholds (or their absence) and its mock-reset policy all live in the preset,
//! which itself may be a `mergeConfig(baseConfig, …)` that ends at a `base` config. Read
//! only the kit's own file and all of that is invisible: the gate looks like it collects
//! nothing and sets nothing.
//!
//! This step runs after [`crate::resolve`] and before [`crate::setup`] and [`crate::scope`].
//! For every gate whose config imports a preset (recorded as written by the
//! config recogniser in [`Gate::preset_imports`]), it resolves each import through the
//! *same* resolver the rest of the model uses — honouring `package.json` `exports`,
//! workspace aliases and relative paths alike — reads the preset with the same
//! [`crate::config_reader`] the recogniser used on the kit's own file, follows the preset's
//! own imports in turn, and merges what the chain declares *under* the kit's own fields:
//! arrays concatenate (the preset's first), scalars keep the kit's own value when it sets
//! one. A preset's `setupFiles` are rewritten relative to the root so [`crate::setup`] opens
//! the file the preset meant, not one beside the kit. The chain of preset files is recorded
//! on [`Gate::preset_chain`] so a rule can lift a finding to the preset every inheritor
//! names.
//!
//! What cannot be followed is said, not guessed: a specifier that does not resolve, a
//! preset that cannot be read as source, a preset whose export is a function body, or one
//! that nests deeper than [`MAX_DEPTH`] each leave the gate opaque with the cause in
//! [`Gate::unread`], naming the preset. Nothing is executed: the presets are parsed as
//! source and read as object literals, exactly as the kit's own config is.

use std::path::{Path, PathBuf};

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_resolver::Resolver;
use oxc_span::SourceType;

use crate::config_reader::{export_parts, merge_into, named_export_parts, read_config, ConfigPart};
use crate::{Gate, GateTool, Model, PresetImport};

/// Preset chains in practice are two deep (`kit → variant → base`); this bound stops a
/// cyclic or pathological import graph from looping forever.
const MAX_DEPTH: usize = 8;

/// Fill every gate with what its imported presets declare, through the resolver
/// [`crate::resolve::resolve`] returned so a preset specifier resolves exactly as an import
/// would. Mutates `model` in place; callers sort afterwards.
pub fn resolve(model: &mut Model, resolver: &Resolver) {
    let root = model.root.clone();
    for gate in &mut model.gates {
        // A CI workflow is YAML and imports nothing; every other tool's config may.
        if gate.tool == GateTool::Ci {
            continue;
        }
        if !gate.preset_imports.is_empty() {
            follow_imports(&root, resolver, gate);
        }
        if gate.tool == GateTool::Jest {
            follow_jest_preset(&root, resolver, gate);
        }
    }
}

/// Follow the presets a gate's config imports, merging what they declare under the kit's own.
fn follow_imports(root: &Path, resolver: &Resolver, gate: &mut Gate) {
    {
        let path = root.join(&gate.source);
        let mut follower = Follower {
            root,
            resolver,
            tool: gate.tool,
            chain: Vec::new(),
        };
        // Everything the chain declares, read into one piece in inheritance order.
        let mut inherited = Gate::blank(&gate.source, gate.tool);
        for import in &gate.preset_imports {
            follower.follow(import, &path, 0, &mut inherited);
        }
        // The kit's own fields take precedence over the inherited ones: arrays concatenate
        // with the preset's first, scalars keep the kit's value when it sets one.
        let own = std::mem::replace(gate, inherited);
        let imports = own.preset_imports.clone();
        merge_into(gate, own);
        gate.preset_imports = imports;
        gate.preset_chain = follower.chain;
    }
}

/// Follow Jest's own `preset: 'name'` key ([`Gate::jest_preset`]) to the `jest-preset`
/// module it names, when that module resolves to a file of the project itself (a workspace
/// package, as carbon's `jest-config-carbon`). An installed preset (`ts-jest`, `react-native`) or one that
/// does not resolve, or one the reader cannot read whole, is left unread as before: the
/// reader neither follows nor guesses it.
///
/// Jest puts a preset's `setupFiles` and `setupFilesAfterEnv` before the config's own and
/// lets every other key the config sets replace the preset's. Only what the setup reader
/// judges is taken from the preset — its setup files and its mock-reset flags — so the
/// gate's collection keys stay the config's own.
fn follow_jest_preset(root: &Path, resolver: &Resolver, gate: &mut Gate) {
    let Some(name) = gate.jest_preset.clone() else {
        return;
    };
    let path = root.join(&gate.source);
    let specifier = if name.starts_with('.') || name.starts_with('/') {
        name
    } else {
        format!("{name}/jest-preset")
    };
    // A workspace package resolves through its `node_modules` link to its real path in the
    // tree; an installed package stays under `node_modules`, and is not followed.
    let in_root = resolve_specifier(resolver, &path, &specifier)
        .and_then(|preset| crate::relative_slash(root, &preset))
        .is_some_and(|preset| !preset.split('/').any(|segment| segment == "node_modules"));
    if !in_root {
        return;
    }
    let mut follower = Follower {
        root,
        resolver,
        tool: GateTool::Jest,
        chain: Vec::new(),
    };
    let mut inherited = Gate::blank(&gate.source, GateTool::Jest);
    let import = PresetImport {
        specifier,
        name: None,
    };
    follower.follow(&import, &path, 0, &mut inherited);
    // A preset the reader cannot see whole (expo's `withWatchPlugins({ ...require(…) })`)
    // is left unread as an installed one is, so the gate keeps what its own config says.
    if inherited.opaque {
        return;
    }
    // The preset's collection keys are not taken, so what the gate collects is not known
    // from its own config alone.
    gate.selection_unapplied |= !inherited.include.is_empty()
        || !inherited.test_regex.is_empty()
        || !inherited.collection_roots.is_empty()
        || inherited.collection_root.is_some();

    let own = std::mem::replace(&mut gate.setup_files, inherited.setup_files);
    for spelling in own {
        if !gate.setup_files.contains(&spelling) {
            gate.setup_files.push(spelling);
        }
    }
    for spelling in inherited.inherited_setup_files {
        if !gate.inherited_setup_files.contains(&spelling) {
            gate.inherited_setup_files.push(spelling);
        }
    }
    gate.restore_mocks = gate.restore_mocks.or(inherited.restore_mocks);
    gate.clear_mocks = gate.clear_mocks.or(inherited.clear_mocks);
    gate.reset_mocks = gate.reset_mocks.or(inherited.reset_mocks);
    for preset in follower.chain {
        if !gate.preset_chain.contains(&preset) {
            gate.preset_chain.push(preset);
        }
    }
}

/// The state one gate's chain is followed with: the root every path is spelled against,
/// the resolver, the tool the presets are read as, and the chain collected so far.
struct Follower<'a> {
    root: &'a Path,
    resolver: &'a Resolver,
    tool: GateTool,
    chain: Vec<String>,
}

impl Follower<'_> {
    /// Resolve `import` against the directory of `from`, read the preset it names, follow
    /// the preset's own imports, and merge what the preset declares into `into`. What
    /// cannot be followed is recorded on `into` with the preset named.
    fn follow(&mut self, import: &PresetImport, from: &Path, depth: usize, into: &mut Gate) {
        let specifier = import.specifier.as_str();
        let from_label = self.label(from);
        if depth > MAX_DEPTH {
            into.could_not_read(format!(
                "preset '{specifier}' from {from_label} nests deeper than {MAX_DEPTH} levels"
            ));
            return;
        }
        let Some(preset) = resolve_specifier(self.resolver, from, specifier) else {
            into.could_not_read(format!(
                "preset '{specifier}' could not be resolved from {from_label}"
            ));
            return;
        };
        let label = self.label(&preset);
        // A preset inside the root joins the chain (once); one outside it — an installed
        // package — is read but not named, since no finding could be located there.
        if let Some(relative) = crate::relative_slash(self.root, &preset) {
            if !self.chain.contains(&relative) {
                self.chain.push(relative);
            }
        }
        let Ok(source) = std::fs::read_to_string(&preset) else {
            into.could_not_read(format!("preset '{specifier}' ({label}) could not be read"));
            return;
        };
        let Ok(source_type) = SourceType::from_path(&preset) else {
            into.could_not_read(format!(
                "preset '{specifier}' ({label}) is not a module the reader parses"
            ));
            return;
        };
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, &source, source_type).parse();
        let program = &parsed.program;
        let parts = import.name.as_ref().map_or_else(
            || export_parts(program),
            |name| named_export_parts(program, name),
        );

        // The preset's parts in order: an object literal is read, an import is followed one
        // level deeper, and what cannot be read is recorded with the preset's name.
        let mut piece = Gate::blank(&into.source, self.tool);
        for part in &parts {
            match part {
                ConfigPart::Object(object) => {
                    let mut sub = Gate::blank(&into.source, self.tool);
                    read_config(self.tool, object, &mut sub);
                    merge_into(&mut piece, sub);
                }
                ConfigPart::Import(nested) => {
                    self.follow(nested, &preset, depth + 1, &mut piece);
                }
                ConfigPart::Unreadable(why) => {
                    piece.could_not_read(format!("preset '{specifier}' ({label}): {why}"));
                }
            }
        }
        // A relative setup file is relative to the preset, not to the kit that inherits it:
        // spell it from the root so the setup reader opens the file the preset meant.
        for spelling in &mut piece.setup_files {
            if let Some(from_root) = self.spell_from_root(&preset, spelling) {
                *spelling = from_root;
            }
        }
        // Everything the preset's setup declares is inherited by the gate that names it.
        for spelling in &piece.setup_files {
            if !piece.inherited_setup_files.contains(spelling) {
                piece.inherited_setup_files.push(spelling.clone());
            }
        }
        merge_into(into, piece);
    }

    /// The path as the model spells it, or as the filesystem does when it is outside the
    /// root — for a message.
    fn label(&self, path: &Path) -> String {
        crate::relative_slash(self.root, path).unwrap_or_else(|| path.display().to_string())
    }

    /// A relative setup-file spelling (`./setup.ts`) joined to the preset's directory and
    /// spelled from the root (`packages/config/setup.ts`), or as an absolute path when the
    /// preset sits outside the root. `None` for a spelling that is not relative.
    fn spell_from_root(&self, preset: &Path, spelling: &str) -> Option<String> {
        if !spelling.starts_with('.') {
            return None;
        }
        let joined = normalise(&preset.parent()?.join(spelling));
        Some(
            crate::relative_slash(self.root, &joined)
                .unwrap_or_else(|| joined.to_string_lossy().into_owned()),
        )
    }
}

/// Resolve `specifier` against the directory of `from`, returning the file it names.
fn resolve_specifier(resolver: &Resolver, from: &Path, specifier: &str) -> Option<PathBuf> {
    let directory = from.parent()?;
    resolver
        .resolve(directory, specifier)
        .ok()
        .map(|resolution| resolution.full_path())
}

/// Collapse `.` and `..` segments so a joined path names one file the reader can open.
fn normalise(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        use std::path::Component;
        match component {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}
