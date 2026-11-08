//! Reading a runner or mutation config's exported value into a [`Gate`], by AST.
//!
//! Two readers need the same vocabulary: the config recogniser in `spirolo-frameworks`, which
//! reads a `vitest.config.*` / `jest.config.*` / `playwright.config.*` / `stryker.conf.*`
//! file the walk parsed, and the preset follower ([`crate::presets`]), which re-reads the
//! files a config inherits from (`mergeConfig(preset, …)`, `export default preset`) through
//! the resolver. Both call [`export_parts`] to find what the file exports and
//! [`read_config`] to read one object literal's fields, so a field the recogniser reads at
//! the kit and the preset follower reads at the preset are read by the same code and merged
//! by the same rule ([`merge_into`]: arrays concatenate, scalars override left to right).
//!
//! The export is found in either module system: an ES `export default {…}` (or
//! `export default defineConfig({…})`) or a `CommonJS` `module.exports = {…}`. Each tool
//! contributes its own vocabulary: Vitest and Jest name the tests the runner will collect and
//! how it resets mocks, Playwright names its projects and its `forbidOnly` gate, Stryker
//! names its mutation `break` score.
//!
//! A value imported from another module — `mergeConfig(preset, …)`, `export default
//! preset`, `export { default } from "preset"`, `defineConfig(preset)`, `make(…)` where
//! `make` is imported — is a [`ConfigPart::Import`]: the recogniser records it on the gate
//! as written and the preset follower resolves it. Nothing is executed. A value the reader
//! cannot decide statically — a spread, a function, a `require` — leaves its field empty
//! and is recorded on the gate with its reason ([`Gate::could_not_read`]), so a later rule
//! reports "could not decide" and says what, rather than guessing what the config would
//! have evaluated to at runtime.

use oxc_ast::ast::{
    Argument, BindingPattern, CallExpression, Declaration, ExportFromDeclaration, Expression,
    ImportDeclarationSpecifier, ObjectExpression, ObjectPropertyKind, Program, PropertyKey,
    Statement, VariableDeclaration,
};

use crate::{Gate, GateFlag, GateTool, PresetImport, Threshold};

// ------------------------------------------------------------- finding the export

/// One part of a config's exported value, in merge order.
#[derive(Debug)]
pub enum ConfigPart<'a> {
    /// An object literal, ready to read.
    Object(&'a ObjectExpression<'a>),
    /// A value imported from another module — a preset — that the caller resolves and
    /// reads: the recogniser records it on the gate, the preset follower follows it.
    Import(PresetImport),
    /// A part the reader cannot see into — a `require`, a function, a binding it does not
    /// follow — with the reason, worded for [`Gate::could_not_read`].
    Unreadable(String),
}

/// The parts of a config's exported value, in merge order.
///
/// The export is `export default X`, `module.exports = X` or `export { default } from "p"`,
/// with any `defineConfig(X)` wrapper unwrapped to its argument, a `mergeConfig(a, b, …)`
/// flattened into its arguments in order, an identifier followed to the import that binds
/// it ([`ConfigPart::Import`]) or to its top-level `const` initialiser, and a call to an
/// imported function taken as that import (`export default make(…)`: the preset is what
/// the reader can name; what the factory does with its argument it cannot). A file with no
/// export at all yields one [`ConfigPart::Unreadable`] part.
#[must_use]
pub fn export_parts<'a>(program: &'a Program<'a>) -> Vec<ConfigPart<'a>> {
    let mut parts = Vec::new();
    for statement in &program.body {
        match statement {
            Statement::ExportDefaultDeclaration(export) => {
                match export.declaration.as_expression() {
                    Some(expression) => parts_of_expression(expression, program, &mut parts),
                    None => parts.push(ConfigPart::Unreadable(
                        "`export default` declares a function or class the reader does not \
                         evaluate"
                            .to_owned(),
                    )),
                }
                return parts;
            }
            Statement::ExportFromDeclaration(export) => {
                // `export { default } from "p"` / `export { x as default } from "p"`: the
                // default export is another module's — an import of it.
                if let Some(import) = re_exported_default(export) {
                    parts.push(ConfigPart::Import(import));
                    return parts;
                }
            }
            Statement::ExpressionStatement(statement) => {
                if let Expression::AssignmentExpression(assignment) = &statement.expression {
                    if is_module_exports(&assignment.left) {
                        parts_of_expression(&assignment.right, program, &mut parts);
                        return parts;
                    }
                }
            }
            _ => {}
        }
    }
    vec![ConfigPart::Unreadable(
        "no `export default` or `module.exports`".to_owned(),
    )]
}

/// The parts of a named export `name` of a preset module.
///
/// `export const name = X` gives `X`'s parts; `export { name }` of a top-level `const name
/// = X` the same; `export { name } from "p"` is an import of it. A module without that
/// export yields one [`ConfigPart::Unreadable`] part.
#[must_use]
pub fn named_export_parts<'a>(program: &'a Program<'a>, name: &str) -> Vec<ConfigPart<'a>> {
    let mut parts = Vec::new();
    for statement in &program.body {
        let (specifiers, source) = match statement {
            // `export const name = X`.
            Statement::ExportDeclaration(export) => {
                if let Declaration::VariableDeclaration(declaration) = &export.declaration {
                    if let Some(init) = declarator_init(declaration, name) {
                        parts_of_expression(init, program, &mut parts);
                        return parts;
                    }
                }
                continue;
            }
            Statement::ExportNamedDeclaration(export) => (&export.specifiers, None),
            Statement::ExportFromDeclaration(export) => (&export.specifiers, Some(&export.source)),
            _ => continue,
        };
        for specifier in specifiers {
            if specifier.exported.name() != name {
                continue;
            }
            let local = specifier.local.name().as_str().to_owned();
            if let Some(source) = source {
                // `export { local as name } from "p"`.
                let Some(source) = source.value.as_str() else {
                    continue;
                };
                parts.push(ConfigPart::Import(PresetImport {
                    specifier: source.to_owned(),
                    name: (local != "default").then_some(local),
                }));
            } else {
                parts_of_binding(&local, program, &mut parts);
            }
            return parts;
        }
    }
    vec![ConfigPart::Unreadable(format!("no export named `{name}`"))]
}

/// The import an `export { default } from "p"` / `export { x as default } from "p"`
/// statement re-exports as the default, if it is one.
fn re_exported_default(export: &ExportFromDeclaration<'_>) -> Option<PresetImport> {
    let source = &export.source;
    let specifier = export
        .specifiers
        .iter()
        .find(|specifier| specifier.exported.name() == "default")?;
    let local = specifier.local.name().as_str().to_owned();
    Some(PresetImport {
        specifier: source.value.as_str()?.to_owned(),
        name: (local != "default").then_some(local),
    })
}

/// Peel a `defineConfig(…)` (or any single-call) wrapper off an exported value and collect
/// the parts underneath, flattening a `mergeConfig(…)` into its arguments and following an
/// identifier to what binds it.
fn parts_of_expression<'a>(
    expression: &'a Expression<'a>,
    program: &'a Program<'a>,
    parts: &mut Vec<ConfigPart<'a>>,
) {
    match expression.get_inner_expression() {
        Expression::ObjectExpression(object) => parts.push(ConfigPart::Object(object)),
        Expression::CallExpression(call) => parts_of_call(call, program, parts),
        Expression::Identifier(id) => parts_of_binding(id.name.as_str(), program, parts),
        Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_) => {
            parts.push(ConfigPart::Unreadable(
                "the export is a function the reader does not evaluate".to_owned(),
            ));
        }
        _ => parts.push(ConfigPart::Unreadable(
            "the export is not an object literal".to_owned(),
        )),
    }
}

/// The parts a call contributes: `mergeConfig(a, b, …)` is every argument's; a call to an
/// imported function is that import (a preset factory, whose arguments are not read); any
/// other wrapper (`defineConfig(x)`) is its first argument's.
fn parts_of_call<'a>(
    call: &'a CallExpression<'a>,
    program: &'a Program<'a>,
    parts: &mut Vec<ConfigPart<'a>>,
) {
    if is_named_call(call, "require") && call.arguments.len() == 1 {
        if let Some(specifier) = call.arguments[0].as_expression().and_then(string_value) {
            parts.push(ConfigPart::Import(PresetImport {
                specifier,
                name: None,
            }));
            return;
        }
    }
    if is_named_call(call, "mergeConfig") {
        for argument in &call.arguments {
            match argument.as_expression() {
                Some(expression) => parts_of_expression(expression, program, parts),
                None => parts.push(ConfigPart::Unreadable(
                    "a spread argument to `mergeConfig(…)`".to_owned(),
                )),
            }
        }
        return;
    }
    // `make(…)` where `make` is imported: the preset is the module `make` comes from. The
    // framework's own wrappers are unwrapped instead, whatever they are imported from.
    if let Expression::Identifier(callee) = call.callee.get_inner_expression() {
        let name = callee.name.as_str();
        if !is_framework_wrapper(name) {
            if let Some(import) = import_binding(program, name) {
                parts.push(ConfigPart::Import(import));
                return;
            }
        }
    }
    // `defineConfig({…})`, `defineConfig(preset)`, `defineConfig(mergeConfig(…))`: a wrapper
    // returns what it is given, merged left to right when given several (Playwright's
    // `defineConfig(base, overrides)`), so read every argument. A `require('x')` or a wrapper
    // passed a function is not evaluated.
    let readable = call.arguments.iter().all(|argument| {
        matches!(
            argument
                .as_expression()
                .map(Expression::get_inner_expression),
            Some(
                Expression::ObjectExpression(_)
                    | Expression::CallExpression(_)
                    | Expression::Identifier(_)
            )
        )
    });
    if call.arguments.is_empty() || !readable {
        parts.push(ConfigPart::Unreadable(unreadable_call(call)));
        return;
    }
    for argument in &call.arguments {
        if let Some(expression) = argument.as_expression() {
            parts_of_expression(expression, program, parts);
        }
    }
}

/// The wrappers Vitest, Vite and Playwright ship, which return their argument: unwrapped
/// rather than taken as a preset factory even though they are imported.
fn is_framework_wrapper(name: &str) -> bool {
    matches!(
        name,
        "defineConfig" | "defineProject" | "defineWorkspace" | "mergeConfig"
    )
}

/// The parts a bare identifier contributes: the import that binds it (a preset), or the
/// initialiser of the top-level `const` that declares it; anything else is a binding the
/// reader does not follow.
fn parts_of_binding<'a>(name: &str, program: &'a Program<'a>, parts: &mut Vec<ConfigPart<'a>>) {
    if let Some(import) = import_binding(program, name) {
        parts.push(ConfigPart::Import(import));
        return;
    }
    if let Some(init) = local_binding(program, name) {
        parts_of_expression(init, program, parts);
        return;
    }
    parts.push(ConfigPart::Unreadable(format!(
        "`{name}` names a binding the reader does not follow"
    )));
}

/// The import that binds `name` at the top level, as a [`PresetImport`]: a default import
/// (`import name from "p"`) takes the default export; a named import
/// (`import { x as name } from "p"`) takes `x`. A namespace import is not followed.
fn import_binding(program: &Program<'_>, name: &str) -> Option<PresetImport> {
    for statement in &program.body {
        let Statement::ImportDeclaration(import) = statement else {
            continue;
        };
        let Some(specifiers) = &import.specifiers else {
            continue;
        };
        for specifier in specifiers {
            match specifier {
                ImportDeclarationSpecifier::ImportDefaultSpecifier(default)
                    if default.local.name == name =>
                {
                    return Some(PresetImport {
                        specifier: import.source.value.as_str()?.to_owned(),
                        name: None,
                    });
                }
                ImportDeclarationSpecifier::ImportSpecifier(named) if named.local.name == name => {
                    let imported = named.imported.name().as_str().to_owned();
                    return Some(PresetImport {
                        specifier: import.source.value.as_str()?.to_owned(),
                        name: (imported != "default").then_some(imported),
                    });
                }
                _ => {}
            }
        }
    }
    None
}

/// The initialiser of the top-level `const`/`let`/`var` that declares `name`, if any.
fn local_binding<'a>(program: &'a Program<'a>, name: &str) -> Option<&'a Expression<'a>> {
    for statement in &program.body {
        let declaration = match statement {
            Statement::VariableDeclaration(declaration) => declaration,
            Statement::ExportDeclaration(export) => match &export.declaration {
                Declaration::VariableDeclaration(declaration) => declaration,
                _ => continue,
            },
            _ => continue,
        };
        if let Some(init) = declarator_init(declaration, name) {
            return Some(init);
        }
    }
    None
}

/// The initialiser of the declarator named `name` in a variable declaration, if any.
fn declarator_init<'a>(
    declaration: &'a VariableDeclaration<'a>,
    name: &str,
) -> Option<&'a Expression<'a>> {
    declaration.declarations.iter().find_map(|declarator| {
        let BindingPattern::BindingIdentifier(id) = &declarator.id else {
            return None;
        };
        (id.name == name)
            .then_some(declarator.init.as_ref())
            .flatten()
    })
}

/// Why a call whose first argument is not an object literal could not be read: a
/// `require('x')` is never evaluated; a wrapper passed a function (`defineConfig(() =>
/// …)`) is not evaluated either; anything else is a call whose value the reader cannot see.
fn unreadable_call(call: &CallExpression<'_>) -> String {
    let callee = match call.callee.get_inner_expression() {
        Expression::Identifier(id) => id.name.as_str().to_owned(),
        Expression::StaticMemberExpression(member) => member.property.name.as_str().to_owned(),
        _ => "(…)".to_owned(),
    };
    if callee == "require" {
        let specifier = call
            .arguments
            .first()
            .and_then(Argument::as_expression)
            .and_then(string_value)
            .unwrap_or_default();
        return format!("`require('{specifier}')` is a call the reader does not evaluate");
    }
    match call
        .arguments
        .first()
        .and_then(Argument::as_expression)
        .map(Expression::get_inner_expression)
    {
        Some(Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)) => {
            format!("`{callee}(…)` is passed a function the reader does not evaluate")
        }
        Some(_) => format!("`{callee}(…)` is passed a value that is not an object literal"),
        None => format!("`{callee}()` is a call the reader does not evaluate"),
    }
}

/// Whether a call's callee is the bare identifier `name`.
fn is_named_call(call: &CallExpression<'_>, name: &str) -> bool {
    matches!(call.callee.get_inner_expression(), Expression::Identifier(id) if id.name == name)
}

/// Whether an assignment target is `module.exports` or a bare `exports`.
fn is_module_exports(target: &oxc_ast::ast::AssignmentTarget<'_>) -> bool {
    use oxc_ast::ast::AssignmentTarget;
    match target {
        AssignmentTarget::AssignmentTargetIdentifier(id) => id.name == "exports",
        AssignmentTarget::StaticMemberExpression(member) => {
            member.property.name == "exports"
                && matches!(
                    member.object.get_inner_expression(),
                    Expression::Identifier(object) if object.name == "module"
                )
        }
        _ => false,
    }
}

// ------------------------------------------------------------- reading the parts

/// Read every part of an export into `gate`.
///
/// Each object literal is read on its own as a piece of the same tool and merged in
/// ([`merge_into`]); an import is recorded as written in [`Gate::preset_imports`] for the
/// preset follower to resolve; an unreadable part is recorded on the gate with its reason
/// ([`Gate::could_not_read`]).
pub fn read_parts(tool: GateTool, parts: &[ConfigPart<'_>], gate: &mut Gate) {
    for part in parts {
        match part {
            ConfigPart::Object(object) => {
                let mut piece = Gate::blank(&gate.source, tool);
                read_config(tool, object, &mut piece);
                merge_into(gate, piece);
            }
            ConfigPart::Import(import) => gate.preset_imports.push(import.clone()),
            ConfigPart::Unreadable(why) => gate.could_not_read(why.clone()),
        }
    }
}

/// Read the object literal into the gate, dispatching on the tool.
///
/// A spread at the top level (`{ ...require('./base'), … }`) mixes in a value the reader
/// cannot see, so the gate is opaque even though the fields written alongside it are read.
pub fn read_config(tool: GateTool, object: &ObjectExpression<'_>, gate: &mut Gate) {
    note_spreads(object, gate);
    match tool {
        GateTool::Vitest => read_vitest(object, gate),
        GateTool::Jest => read_jest(object, gate),
        GateTool::Playwright => read_playwright(object, gate),
        GateTool::Stryker => read_stryker(object, gate),
        GateTool::Ci | GateTool::Unknown => {}
    }
}

/// Merge one `mergeConfig` part into the gate: arrays concatenate (deduplicated, in
/// order), scalars take the later value when it is set, and opacity accumulates.
pub fn merge_into(gate: &mut Gate, part: Gate) {
    fn append(into: &mut Vec<String>, more: Vec<String>) {
        for item in more {
            if !into.contains(&item) {
                into.push(item);
            }
        }
    }
    append(&mut gate.include, part.include);
    append(&mut gate.test_regex, part.test_regex);
    gate.collection_projects.extend(part.collection_projects);
    append(&mut gate.collection_roots, part.collection_roots);
    if part.collection_root.is_some() {
        gate.collection_root = part.collection_root;
    }
    append(&mut gate.exclude, part.exclude);
    if !part.module_name_mapper.is_empty() {
        gate.module_name_mapper = part.module_name_mapper;
    }
    append(&mut gate.setup_files, part.setup_files);
    append(&mut gate.inherited_setup_files, part.inherited_setup_files);
    append(&mut gate.projects, part.projects);
    append(&mut gate.mutate, part.mutate);
    append(&mut gate.project_globs, part.project_globs);
    for threshold in part.thresholds {
        gate.thresholds
            .retain(|existing| existing.metric != threshold.metric);
        gate.thresholds.push(threshold);
    }
    gate.thresholds.sort_by(|a, b| a.metric.cmp(&b.metric));
    gate.retries = part.retries.or(gate.retries);
    gate.restore_mocks = part.restore_mocks.or(gate.restore_mocks);
    gate.clear_mocks = part.clear_mocks.or(gate.clear_mocks);
    gate.reset_mocks = part.reset_mocks.or(gate.reset_mocks);
    gate.unstub_globals = part.unstub_globals.or(gate.unstub_globals);
    gate.unstub_envs = part.unstub_envs.or(gate.unstub_envs);
    gate.forbid_only = part.forbid_only.or(gate.forbid_only);
    gate.test_dir = part.test_dir.or_else(|| gate.test_dir.take());
    gate.test_runner = part.test_runner.or_else(|| gate.test_runner.take());
    gate.mutation_break = part.mutation_break.or(gate.mutation_break);
    for what in part.unread {
        gate.could_not_read(what);
    }
    gate.opaque |= part.opaque;
    gate.snapshot_resolver |= part.snapshot_resolver;
    gate.selection_unapplied |= part.selection_unapplied;
    gate.jest_preset = part.jest_preset.or_else(|| gate.jest_preset.take());
}

/// Mark the gate opaque when an object mixes in a spread (`...x`), whose value the reader
/// cannot resolve without evaluating the module it came from.
fn note_spreads(object: &ObjectExpression<'_>, gate: &mut Gate) {
    if object
        .properties
        .iter()
        .any(|property| matches!(property, ObjectPropertyKind::SpreadProperty(_)))
    {
        gate.could_not_read("a spread (`...`) in the config object");
    }
}

/// Vitest keeps its runner options under a `test` key; a config without one gates nothing.
fn read_vitest(object: &ObjectExpression<'_>, gate: &mut Gate) {
    if prop(object, "root").is_some() {
        gate.selection_unapplied = true;
    }
    let Some(test) = prop(object, "test").and_then(object_value) else {
        return;
    };
    // `test.root` moves `include` exactly as a top-level `root` does.
    if prop(test, "root").is_some() {
        gate.selection_unapplied = true;
    }
    note_spreads(test, gate);
    read_runner_common(test, gate);
    read_collection_projects(GateTool::Vitest, test, gate);
    if let Some(unstub) = prop(test, "unstubGlobals") {
        gate.unstub_globals =
            bool_value(unstub).or_else(|| unread_scalar(gate, "unstubGlobals", "boolean"));
    }
    if let Some(unstub) = prop(test, "unstubEnvs") {
        gate.unstub_envs =
            bool_value(unstub).or_else(|| unread_scalar(gate, "unstubEnvs", "boolean"));
    }
    if let Some(coverage) = prop(test, "coverage").and_then(object_value) {
        if let Some(thresholds) = prop(coverage, "thresholds").and_then(object_value) {
            gate.thresholds = numeric_thresholds(thresholds, gate);
        }
    }
    if let Some(retry) = prop(test, "retry") {
        gate.retries = number_value(under_ci(retry))
            .and_then(u32_of)
            .or_else(|| unread_scalar(gate, "retry", "number"));
    }
}

/// Record a scalar the reader could not decide (`forbidOnly: !!process.env.CI`) and yield
/// `None` for it: the field is unread, not unset, and the gate says so.
fn unread_scalar<T>(gate: &mut Gate, key: &str, literal: &str) -> Option<T> {
    gate.could_not_read(format!("`{key}` is not a {literal} literal"));
    None
}

/// Jest keeps its options at the top level; `testMatch` names its tests,
/// `testPathIgnorePatterns` (regular expressions, kept verbatim in `exclude`) removes
/// some, `moduleNameMapper` redirects specifiers, and `coverageThreshold.global` carries
/// its thresholds.
fn read_jest(object: &ObjectExpression<'_>, gate: &mut Gate) {
    read_runner_common(object, gate);
    if let Some(preset) = prop(object, "preset") {
        gate.jest_preset = string_value(preset);
    }
    if let Some(roots) = prop(object, "roots") {
        gate.collection_roots = string_array(roots, "roots", gate);
    }
    if let Some(root) = prop(object, "rootDir") {
        let dir = gate.source.rsplit_once('/').map_or("", |(dir, _)| dir);
        gate.collection_root = match root.get_inner_expression() {
            Expression::Identifier(id) if id.name == "__dirname" => Some(dir.to_owned()),
            Expression::CallExpression(call) if is_process_cwd(call) => Some(String::new()),
            _ => string_value(root).and_then(|root| crate::paths::join_relative(dir, &root)),
        };
        if gate.collection_root.is_none() {
            gate.could_not_read("`rootDir` is not a static project-relative path".to_owned());
        }
    }
    read_collection_projects(GateTool::Jest, object, gate);
    if let Some(pattern) = prop(object, "testRegex") {
        gate.test_regex = string_value(pattern).map_or_else(
            || string_array(pattern, "testRegex", gate),
            |pattern| vec![pattern],
        );
    }
    if let Some(matches) = prop(object, "testMatch") {
        gate.include = string_array(matches, "testMatch", gate);
    }
    if let Some(ignored) = prop(object, "testPathIgnorePatterns") {
        gate.exclude = string_array(ignored, "testPathIgnorePatterns", gate);
    }
    // Jest excludes these modules before it searches for tests as well.
    if let Some(ignored) = prop(object, "modulePathIgnorePatterns") {
        let ignored = string_array(ignored, "modulePathIgnorePatterns", gate);
        gate.exclude.extend(ignored);
    }
    if let Some(mapper) = prop(object, "moduleNameMapper").and_then(object_value) {
        gate.module_name_mapper = mapper
            .properties
            .iter()
            .filter_map(|property| {
                let ObjectPropertyKind::ObjectProperty(property) = property else {
                    return None;
                };
                Some((key_name(&property.key)?, literal_strings(&property.value)?))
            })
            .collect();
    }
    if let Some(after) = prop(object, "setupFilesAfterEnv") {
        let extra = values_array(after, "setupFilesAfterEnv", gate, setup_path);
        gate.setup_files.extend(extra);
    }
    if let Some(global) = prop(object, "coverageThreshold")
        .and_then(object_value)
        .and_then(|threshold| prop(threshold, "global"))
        .and_then(object_value)
    {
        gate.thresholds = numeric_thresholds(global, gate);
    }
}

fn is_process_cwd(call: &CallExpression<'_>) -> bool {
    matches!(call.callee.get_inner_expression(),
        Expression::StaticMemberExpression(member)
            if member.property.name == "cwd"
                && matches!(member.object.get_inner_expression(), Expression::Identifier(id) if id.name == "process")
    ) && call.arguments.is_empty()
}

fn read_collection_projects(tool: GateTool, object: &ObjectExpression<'_>, gate: &mut Gate) {
    let Some(projects) = prop(object, "projects").map(Expression::get_inner_expression) else {
        return;
    };
    let Expression::ArrayExpression(projects) = projects else {
        gate.could_not_read("`projects` is not an array literal");
        return;
    };
    for element in &projects.elements {
        if let Some(glob) = element.as_expression().and_then(string_value) {
            gate.project_globs.push(glob);
            continue;
        }
        if element.is_elision() {
            continue;
        }
        let Some(project) = element.as_expression().and_then(object_value) else {
            gate.could_not_read("a `projects` entry that is neither a path nor an object literal");
            continue;
        };
        let mut part = Gate::blank(&gate.source, tool);
        read_config(tool, project, &mut part);
        gate.collection_projects.push(crate::CollectionProject {
            root: part.collection_root,
            roots: part.collection_roots,
            include: part.include,
            exclude: part.exclude,
            test_regex: part.test_regex,
        });
    }
}

/// The fields Vitest and Jest share: `include`/`exclude`, `setupFiles`, mock resets.
fn read_runner_common(object: &ObjectExpression<'_>, gate: &mut Gate) {
    if let Some(include) = prop(object, "include") {
        gate.include = string_array(include, "include", gate);
    }
    if let Some(exclude) = prop(object, "exclude") {
        gate.exclude = string_array(exclude, "exclude", gate);
    }
    if let Some(setup) = prop(object, "setupFiles") {
        gate.setup_files = values_array(setup, "setupFiles", gate, setup_path);
    }
    if let Some(restore) = prop(object, "restoreMocks") {
        gate.restore_mocks =
            bool_value(restore).or_else(|| unread_scalar(gate, "restoreMocks", "boolean"));
    }
    if let Some(clear) = prop(object, "clearMocks") {
        gate.clear_mocks =
            bool_value(clear).or_else(|| unread_scalar(gate, "clearMocks", "boolean"));
    }
    let reset_key = if gate.tool == GateTool::Vitest {
        "mockReset"
    } else {
        "resetMocks"
    };
    if let Some(reset) = prop(object, reset_key) {
        gate.reset_mocks = bool_value(reset).or_else(|| unread_scalar(gate, reset_key, "boolean"));
    }
    // A custom `resolveSnapshotPath` moves the `.snap` files off the `__snapshots__`
    // convention `stale-snapshot` follows; its mere presence, whatever it computes, makes
    // the rule defer for the whole project.
    if prop(object, "resolveSnapshotPath").is_some() {
        gate.snapshot_resolver = true;
    }
}

/// Playwright names its `testDir`, its `retries`, its `forbidOnly` gate, and the projects
/// the run fans out over.
fn read_playwright(object: &ObjectExpression<'_>, gate: &mut Gate) {
    if let Some(dir) = prop(object, "testDir") {
        gate.test_dir = string_value(dir)
            .or_else(|| dirname_join(dir))
            .or_else(|| unread_scalar(gate, "testDir", "string"));
    }
    // `testMatch` is a glob, a RegExp, or an array of either; a RegExp is matched against
    // the file's path, like Jest's `testRegex`.
    if let Some(matches) = prop(object, "testMatch") {
        let items: Vec<&Expression<'_>> = match matches.get_inner_expression() {
            Expression::ArrayExpression(array) => array
                .elements
                .iter()
                .filter_map(|element| element.as_expression())
                .collect(),
            other => vec![other],
        };
        for item in items {
            match item.get_inner_expression() {
                Expression::RegExpLiteral(regex) => {
                    gate.test_regex
                        .push(regex.regex.pattern.text.as_str().to_owned());
                }
                other => match string_value(other) {
                    Some(glob) => gate.include.push(glob),
                    None => gate.could_not_read(
                        "an entry of `testMatch` that is not a string or RegExp literal",
                    ),
                },
            }
        }
    }
    if let Some(retries) = prop(object, "retries") {
        gate.retries = number_value(under_ci(retries))
            .and_then(u32_of)
            .or_else(|| unread_scalar(gate, "retries", "number"));
    }
    if let Some(forbid) = prop(object, "forbidOnly") {
        gate.forbid_only =
            flag_value(forbid).or_else(|| unread_scalar(gate, "forbidOnly", "boolean"));
    }
    if let Some(projects) = prop(object, "projects") {
        gate.projects = project_names(projects, gate);
    }
}

/// A directory spelled `path.join(__dirname, 'e2e')` / `path.resolve(__dirname, …)` (or
/// the bare `join` / `resolve`), as the path relative to the config it names.
fn dirname_join(value: &Expression<'_>) -> Option<String> {
    let Expression::CallExpression(call) = value.get_inner_expression() else {
        return None;
    };
    let callee = match call.callee.get_inner_expression() {
        Expression::StaticMemberExpression(member) => member.property.name.as_str(),
        Expression::Identifier(id) => id.name.as_str(),
        _ => return None,
    };
    if !matches!(callee, "join" | "resolve") {
        return None;
    }
    let mut arguments = call.arguments.iter();
    let first = arguments.next()?.as_expression()?;
    if !matches!(first.get_inner_expression(), Expression::Identifier(id) if id.name == "__dirname")
    {
        return None;
    }
    let parts: Option<Vec<String>> = arguments
        .map(|argument| argument.as_expression().and_then(string_value))
        .collect();
    let joined = parts?
        .iter()
        .map(|part| part.trim_start_matches("./").trim_end_matches('/'))
        .filter(|part| !part.is_empty() && *part != ".")
        .collect::<Vec<_>>()
        .join("/");
    Some(if joined.is_empty() {
        ".".to_owned()
    } else {
        joined
    })
}

/// Stryker names the files it will `mutate`, its `testRunner`, and the mutation-score
/// `break` under `thresholds`.
fn read_stryker(object: &ObjectExpression<'_>, gate: &mut Gate) {
    if let Some(mutate) = prop(object, "mutate") {
        gate.mutate = string_array(mutate, "mutate", gate);
    }
    if let Some(runner) = prop(object, "testRunner") {
        gate.test_runner =
            string_value(runner).or_else(|| unread_scalar(gate, "testRunner", "string"));
    }
    if let Some(thresholds) = prop(object, "thresholds").and_then(object_value) {
        // `break: null` is Stryker's own "no gate" and is read as unset, not unread.
        if let Some(break_score) = prop(thresholds, "break") {
            gate.mutation_break = number_value(break_score).or_else(|| {
                if matches!(
                    break_score.get_inner_expression(),
                    Expression::NullLiteral(_)
                ) {
                    None
                } else {
                    unread_scalar(gate, "thresholds.break", "number")
                }
            });
        }
    }
}

/// Every numeric entry of a thresholds object as a [`Threshold`], sorted by metric. A
/// non-numeric entry (Vitest's `'100': true`) marks the gate opaque rather than inventing
/// a value.
fn numeric_thresholds(object: &ObjectExpression<'_>, gate: &mut Gate) -> Vec<Threshold> {
    let mut thresholds = Vec::new();
    for property in &object.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            gate.could_not_read("a spread (`...`) in the coverage thresholds");
            continue;
        };
        let Some(metric) = key_name(&property.key) else {
            gate.could_not_read("a coverage threshold whose key is not a plain name");
            continue;
        };
        match number_value(&property.value) {
            Some(value) => thresholds.push(Threshold { metric, value }),
            None => gate.could_not_read(format!(
                "a coverage threshold (`{metric}`) that is not a number"
            )),
        }
    }
    thresholds.sort_by(|a, b| a.metric.cmp(&b.metric));
    thresholds
}

/// The `name` of each project in a `projects` array; an element that is not an object with
/// a readable string name marks the gate opaque.
fn project_names(value: &Expression<'_>, gate: &mut Gate) -> Vec<String> {
    let Expression::ArrayExpression(array) = value.get_inner_expression() else {
        gate.could_not_read("`projects` is not an array literal");
        return Vec::new();
    };
    let mut names = Vec::new();
    for element in &array.elements {
        match element
            .as_expression()
            .map(Expression::get_inner_expression)
        {
            Some(Expression::ObjectExpression(object)) => {
                match prop(object, "name").and_then(string_value) {
                    Some(name) => names.push(name),
                    None => gate.could_not_read("a `projects` entry with no string `name`"),
                }
            }
            _ => gate.could_not_read("a `projects` entry that is not an object literal"),
        }
    }
    names
}

// ---------------------------------------------------------------- small value readers

/// The value of an object property named `name`, when it is a plain data property.
#[must_use]
pub fn prop<'a>(object: &'a ObjectExpression<'a>, name: &str) -> Option<&'a Expression<'a>> {
    object.properties.iter().find_map(|property| {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return None;
        };
        (key_name(&property.key).as_deref() == Some(name)).then_some(&property.value)
    })
}

/// A property key's plain name: a static identifier or a string literal.
fn key_name(key: &PropertyKey<'_>) -> Option<String> {
    match key {
        PropertyKey::StaticIdentifier(id) => Some(id.name.as_str().to_owned()),
        PropertyKey::StringLiteral(literal) => literal.value.as_str().map(str::to_owned),
        _ => None,
    }
}

/// The object literal an expression is, if it is one.
#[must_use]
pub fn object_value<'a>(value: &'a Expression<'a>) -> Option<&'a ObjectExpression<'a>> {
    match value.get_inner_expression() {
        Expression::ObjectExpression(object) => Some(object),
        _ => None,
    }
}

/// The string value of an expression: a literal or a substitution-free template.
/// A setup-file entry: a string literal, or a path built from the config's own directory —
/// `resolve(__dirname, './setup.js')`, `path.join(__dirname, 'setup', 'a.js')` — spelled as
/// the relative path it names (`./setup.js`), which the setup reader joins to that directory.
fn setup_path(value: &Expression<'_>) -> Option<String> {
    string_value(value).or_else(|| dirname_path(value))
}

/// `resolve|join(__dirname, 'a', 'b')` (bare or on `path`) as the relative path `./a/b`.
fn dirname_path(value: &Expression<'_>) -> Option<String> {
    let Expression::CallExpression(call) = value.get_inner_expression() else {
        return None;
    };
    let callee = match call.callee.get_inner_expression() {
        Expression::Identifier(id) => id.name.as_str(),
        Expression::StaticMemberExpression(member) => member.property.name.as_str(),
        _ => return None,
    };
    if !matches!(callee, "resolve" | "join") {
        return None;
    }
    let mut arguments = call.arguments.iter();
    let first = arguments.next()?.as_expression()?;
    if !matches!(first.get_inner_expression(), Expression::Identifier(id) if id.name == "__dirname")
    {
        return None;
    }
    let mut path = String::from(".");
    for argument in arguments {
        let segment = string_value(argument.as_expression()?)?;
        let segment = segment.trim_start_matches("./");
        if segment.starts_with('/') {
            return None;
        }
        path.push('/');
        path.push_str(segment);
    }
    (path != ".").then_some(path)
}

fn string_value(value: &Expression<'_>) -> Option<String> {
    match value.get_inner_expression() {
        Expression::StringLiteral(literal) => literal.value.as_str().map(str::to_owned),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => template
            .quasis
            .first()
            .and_then(|quasi| {
                quasi
                    .value
                    .cooked
                    .unwrap_or_else(|| quasi.value.raw.into())
                    .as_str()
            })
            .map(str::to_owned),
        _ => None,
    }
}

/// The strings of an array literal of string literals, or a lone string literal, as a list;
/// `None` when any part is not a literal, so a computed glob reads as unread, not as absent.
#[must_use]
pub fn literal_strings(value: &Expression<'_>) -> Option<Vec<String>> {
    if let Expression::ArrayExpression(array) = value.get_inner_expression() {
        return array
            .elements
            .iter()
            .map(|element| element.as_expression().and_then(string_value))
            .collect();
    }
    string_value(value).map(|text| vec![text])
}

/// The boolean value of an expression, when it is a literal `true` / `false`.
fn bool_value(value: &Expression<'_>) -> Option<bool> {
    match value.get_inner_expression() {
        Expression::BooleanLiteral(literal) => Some(literal.value),
        _ => None,
    }
}

/// A gate flag's value: a literal, or [`GateFlag::CiConditional`] for an expression that
/// folds to `true` once CI is assumed set (`!!process.env.CI`). `None` for anything else,
/// including an expression that folds to `false` under CI (`!process.env.CI`): that is a
/// non-literal the reader does not represent, so the field is unread rather than guessed.
fn flag_value(value: &Expression<'_>) -> Option<GateFlag> {
    if let Some(literal) = bool_value(value) {
        return Some(if literal {
            GateFlag::True
        } else {
            GateFlag::False
        });
    }
    (truth_under_ci(value) == Some(true)).then_some(GateFlag::CiConditional)
}

/// The truth of an expression once `process.env.CI` is assumed set to a non-empty string,
/// or `None` when it depends on anything else.
///
/// Folds the forms a config writes around the CI variable: the read itself, `!` and `!!`,
/// `Boolean(x)`, a comparison of the read with a literal (`=== "true"` is true, `===
/// undefined` false, `!==` the inverse), `||` and `&&` where one side decides, a `?:` whose
/// test decides, and the boolean literals. Nothing else is evaluated.
fn truth_under_ci(value: &Expression<'_>) -> Option<bool> {
    use oxc_ast::ast::{BinaryOperator, LogicalOperator, UnaryOperator};
    match value.get_inner_expression() {
        Expression::BooleanLiteral(literal) => Some(literal.value),
        expression if is_ci_read(expression) => Some(true),
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
            truth_under_ci(&unary.argument).map(|truth| !truth)
        }
        // `Boolean(x)` is the truth of `x`; any other call is not evaluated.
        Expression::CallExpression(call) => {
            let is_boolean =
                matches!(&call.callee, Expression::Identifier(name) if name.name == "Boolean");
            if !is_boolean || call.arguments.len() != 1 {
                return None;
            }
            call.arguments
                .first()
                .and_then(Argument::as_expression)
                .and_then(truth_under_ci)
        }
        Expression::BinaryExpression(binary) => {
            let (read, other) = if is_ci_read(&binary.left) {
                (true, &binary.right)
            } else if is_ci_read(&binary.right) {
                (true, &binary.left)
            } else {
                (false, &binary.left)
            };
            if !read {
                return None;
            }
            // Under CI the variable is a non-empty string: equal to a non-empty string
            // literal it is taken to spell, unequal to `undefined`, `null` and `""`.
            let equal = match other.get_inner_expression() {
                Expression::StringLiteral(literal) => !literal.value.is_empty(),
                Expression::NullLiteral(_) => false,
                Expression::Identifier(name) if name.name == "undefined" => false,
                _ => return None,
            };
            match binary.operator {
                BinaryOperator::Equality | BinaryOperator::StrictEquality => Some(equal),
                BinaryOperator::Inequality | BinaryOperator::StrictInequality => Some(!equal),
                _ => None,
            }
        }
        Expression::LogicalExpression(logical) => {
            let left = truth_under_ci(&logical.left);
            let right = truth_under_ci(&logical.right);
            match logical.operator {
                LogicalOperator::Or => match (left, right) {
                    (Some(true), _) | (_, Some(true)) => Some(true),
                    (Some(false), Some(false)) => Some(false),
                    _ => None,
                },
                LogicalOperator::And => match (left, right) {
                    (Some(false), _) | (_, Some(false)) => Some(false),
                    (Some(true), Some(true)) => Some(true),
                    _ => None,
                },
                LogicalOperator::Coalesce => None,
            }
        }
        Expression::ConditionalExpression(conditional) => {
            let branch = if truth_under_ci(&conditional.test)? {
                &conditional.consequent
            } else {
                &conditional.alternate
            };
            truth_under_ci(branch)
        }
        _ => None,
    }
}

/// The branch of a `?:` an expression takes once CI is assumed set — the scaffold's
/// `retries: process.env.CI ? 2 : 0` is `2` in the build the gate is judged for — followed
/// through nested ternaries; the expression itself when it is not a ternary whose test the
/// reader can decide under CI.
fn under_ci<'a>(value: &'a Expression<'a>) -> &'a Expression<'a> {
    let inner = value.get_inner_expression();
    let Expression::ConditionalExpression(conditional) = inner else {
        return inner;
    };
    match truth_under_ci(&conditional.test) {
        Some(true) => under_ci(&conditional.consequent),
        Some(false) => under_ci(&conditional.alternate),
        None => inner,
    }
}

/// Whether an expression reads the CI variable: `process.env.CI` or `process.env["CI"]`.
fn is_ci_read(value: &Expression<'_>) -> bool {
    let Some(member) = value.get_inner_expression().as_member_expression() else {
        return false;
    };
    let Some(name) = member.static_property_name() else {
        return false;
    };
    if name != "CI" {
        return false;
    }
    let Some(env) = member
        .object()
        .get_inner_expression()
        .as_member_expression()
    else {
        return false;
    };
    env.static_property_name().is_some_and(|name| name == "env")
        && matches!(env.object().get_inner_expression(), Expression::Identifier(root) if root.name == "process")
}

/// The numeric value of an expression, when it is a numeric literal.
fn number_value(value: &Expression<'_>) -> Option<f64> {
    match value.get_inner_expression() {
        Expression::NumericLiteral(literal) => Some(literal.value),
        _ => None,
    }
}

/// A count read from a numeric literal, when it is a whole non-negative number.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn u32_of(value: f64) -> Option<u32> {
    (value.fract() == 0.0 && value >= 0.0 && value <= f64::from(u32::MAX)).then_some(value as u32)
}

/// The strings of an array literal (or a lone string), in source order. A non-string
/// element, or a value that is not an array or string, is recorded on the gate as unread
/// (naming the `key` it sits under) and skipped, so a computed glob is "could not decide"
/// rather than a guess.
fn string_array(value: &Expression<'_>, key: &str, gate: &mut Gate) -> Vec<String> {
    values_array(value, key, gate, string_value)
}

/// An array (or lone value) of entries each read by `read`; an entry `read` cannot decide
/// is recorded on the gate as unread.
fn values_array(
    value: &Expression<'_>,
    key: &str,
    gate: &mut Gate,
    read: fn(&Expression<'_>) -> Option<String>,
) -> Vec<String> {
    if let Expression::ArrayExpression(array) = value.get_inner_expression() {
        let mut out = Vec::new();
        for element in &array.elements {
            match element.as_expression().and_then(read) {
                Some(text) => out.push(text),
                None => {
                    gate.could_not_read(format!(
                        "an entry of `{key}` that is not a string literal"
                    ));
                }
            }
        }
        return out;
    }
    // A lone string is a one-element list (Vitest allows `setupFiles: './setup.ts'`).
    read(value).map_or_else(
        || {
            gate.could_not_read(format!("`{key}` is not an array or a string literal"));
            Vec::new()
        },
        |text| vec![text],
    )
}
