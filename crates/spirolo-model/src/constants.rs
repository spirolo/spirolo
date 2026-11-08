//! Folding module-level string constants to a value, across module boundaries.
//!
//! Two halves, split by the arena. [`collect`] runs inside the per-file parse, while the
//! AST is alive, and lifts every module-level `const` with a string-shaped initializer
//! into an owned [`Init`] tree, together with the import and re-export bindings that a
//! reference in one file might resolve through. [`fold`] runs once afterwards, over the
//! whole file set, and evaluates each [`Init`] to a value — following identifiers to
//! local constants, imports to the exporting module, and re-export chains to the origin.
//!
//! Anything that cannot be decided from string literals alone — a member expression, a
//! call, a number, an import that leaves the project — folds to no value and is recorded
//! as opaque *with the reason it gave up*, so a later rule reports "could not decide"
//! rather than guessing a URL production code never emits.

use std::collections::{HashMap, HashSet};

use oxc_ast::ast::{
    BindingPattern, Declaration, Expression, ImportDeclarationSpecifier, Program, Statement,
    VariableDeclaration, VariableDeclarationKind,
};

use crate::{Constant, Model, Span};

/// A constant's initializer, captured as an owned tree so folding survives the arena.
enum Init {
    /// A string literal's value.
    Str(String),
    /// A template literal, in source order: text then interpolation then text.
    Template(Vec<Part>),
    /// `a + b` string concatenation.
    Concat(Box<Self>, Box<Self>),
    /// An identifier reference, resolved during folding against the module it appears in.
    Ref(String),
    /// A `process.env.NAME` read: a named environment hole. It never folds to a value —
    /// the model never reads the real environment — so a constant containing one is opaque,
    /// but the hole is kept as `${NAME}` in the constant's `env_template` so a rule holding
    /// assumed values (`handler-drift`'s `env` config) can complete it.
    Env(String),
    /// Something that is not a constant string expression, with the reason it is not.
    Opaque(&'static str),
}

/// One piece of a template literal.
enum Part {
    /// A literal text run between interpolations.
    Text(String),
    /// An `${…}` interpolation.
    Expr(Init),
}

/// A module-level `const` binding whose value folding will try to decide.
struct RawConstant {
    /// Binding name.
    name: String,
    /// Its initializer.
    init: Init,
    /// Location of the binding.
    span: Span,
}

/// A `import { imported as local } from "specifier"` binding.
pub struct Binding {
    /// The name the importing module uses.
    pub local: String,
    /// The name in the exporting module (`default` for a default import).
    pub imported: String,
    /// The specifier the name comes from.
    pub specifier: String,
}

/// A `export { imported as exported } from "specifier"` re-export.
pub struct Reexport {
    /// The name this module re-exports under.
    pub exported: String,
    /// The name in the module the specifier names.
    pub imported: String,
    /// The specifier the name comes from.
    pub specifier: String,
}

/// Everything one file contributes to the whole-project fold: its constants and the
/// binding maps a reference might resolve through.
#[derive(Default)]
pub struct FoldFacts {
    /// Module-level string `const` bindings.
    constants: Vec<RawConstant>,
    /// Module-level countable `const` bindings (arrays, objects), for the tables a
    /// parameterised test reads from another module. See [`crate::sizes`].
    pub sizes: Vec<crate::sizes::RawSize>,
    /// Named imports, so a reference to an imported constant can be followed.
    pub imports: Vec<Binding>,
    /// Named re-exports (`export { x } from "y"`), so a chain resolves to its origin.
    pub reexports: Vec<Reexport>,
    /// Bare `export * from "y"` specifiers, followed when a name is not found explicitly.
    pub stars: Vec<String>,
}

/// Lift the module-level constants and binding maps out of one parsed file.
///
/// `source` is used only to turn byte offsets into line and column, matching the
/// coordinates the parser's other rows carry. Only the top level is scanned: a `const`
/// inside a function is not a module constant.
pub fn collect(source: &str, program: &Program<'_>) -> FoldFacts {
    let mut facts = FoldFacts {
        sizes: crate::sizes::collect(program),
        ..FoldFacts::default()
    };
    for statement in &program.body {
        match statement {
            Statement::VariableDeclaration(declaration) => {
                collect_constants(source, declaration, &mut facts);
            }
            Statement::ImportDeclaration(import) => {
                if let Some(specifier) = import.source.value.as_str() {
                    collect_imports(specifier, import, &mut facts);
                }
            }
            Statement::ExportDeclaration(export) => {
                if let Declaration::VariableDeclaration(declaration) = &export.declaration {
                    collect_constants(source, declaration, &mut facts);
                }
            }
            Statement::ExportFromDeclaration(export) => {
                let Some(specifier) = export.source.value.as_str() else {
                    continue;
                };
                for spec in &export.specifiers {
                    facts.reexports.push(Reexport {
                        exported: spec.exported.name().to_string(),
                        imported: spec.local.name().to_string(),
                        specifier: specifier.to_owned(),
                    });
                }
            }
            // `export * from "y"` forwards every name; `export * as ns from "y"` binds a
            // namespace object, which is not a constant string, so only the bare form
            // contributes a chain to follow.
            Statement::ExportAllDeclaration(export) if export.exported.is_none() => {
                if let Some(specifier) = export.source.value.as_str() {
                    facts.stars.push(specifier.to_owned());
                }
            }
            _ => {}
        }
    }
    facts
}

/// Capture the string-shaped `const` declarators of one declaration.
fn collect_constants(source: &str, declaration: &VariableDeclaration<'_>, facts: &mut FoldFacts) {
    if declaration.kind != VariableDeclarationKind::Const {
        return;
    }
    for declarator in &declaration.declarations {
        let BindingPattern::BindingIdentifier(id) = &declarator.id else {
            continue;
        };
        let Some(init) = &declarator.init else {
            continue;
        };
        if let Some(init) = capture_toplevel(init) {
            facts.constants.push(RawConstant {
                name: id.name.to_string(),
                init,
                span: Span::locate(source, id.span.start, id.span.end),
            });
        }
    }
}

/// Capture the named import bindings of one `import` declaration.
fn collect_imports(
    specifier: &str,
    import: &oxc_ast::ast::ImportDeclaration<'_>,
    facts: &mut FoldFacts,
) {
    let Some(specifiers) = &import.specifiers else {
        return;
    };
    for spec in specifiers {
        let (local, imported) = match spec {
            ImportDeclarationSpecifier::ImportSpecifier(named) => {
                (named.local.name.as_str(), named.imported.name().to_string())
            }
            ImportDeclarationSpecifier::ImportDefaultSpecifier(default) => {
                (default.local.name.as_str(), "default".to_owned())
            }
            // A namespace binding is an object, never a constant string.
            ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => continue,
        };
        facts.imports.push(Binding {
            local: local.to_owned(),
            imported,
            specifier: specifier.to_owned(),
        });
    }
}

/// Capture a top-level initializer, keeping only the string-shaped ones: a literal, a
/// template, or a `+` concatenation. A `const` of any other shape is not a string
/// constant and gets no row.
fn capture_toplevel(expr: &Expression<'_>) -> Option<Init> {
    match expr {
        Expression::StringLiteral(_) | Expression::TemplateLiteral(_) => Some(capture(expr)),
        Expression::BinaryExpression(binary) if binary.operator.as_str() == "+" => {
            Some(capture(expr))
        }
        // `const API_VERSION = process.env.API_VERSION`: an env read is a string-shaped
        // binding a URL constant may reference, so it is captured (as an env hole) rather
        // than dropped, letting a template that interpolates it keep its `${NAME}` hole.
        Expression::StaticMemberExpression(_) if env_read(expr).is_some() => Some(capture(expr)),
        _ => None,
    }
}

/// Capture any expression as an [`Init`], mapping the non-string shapes to [`Init::Opaque`]
/// with the reason they cannot fold.
fn capture(expr: &Expression<'_>) -> Init {
    match expr {
        Expression::StringLiteral(literal) => literal
            .value
            .as_str()
            .map_or(Init::Opaque("not a constant string expression"), |value| {
                Init::Str(value.to_owned())
            }),
        Expression::TemplateLiteral(template) => {
            let mut parts = Vec::new();
            for (index, quasi) in template.quasis.iter().enumerate() {
                // `cooked` is `None` only when the chunk has an invalid escape sequence:
                // the raw source text is not the string the runtime would produce, so the
                // whole template is opaque rather than folded to a value it never emits.
                match quasi.value.cooked {
                    Some(cooked) => parts.push(cooked.as_str().map_or(
                        Part::Expr(Init::Opaque("not a constant string expression")),
                        |text| Part::Text(text.to_owned()),
                    )),
                    None => parts.push(Part::Expr(Init::Opaque(
                        "template literal has an invalid escape sequence",
                    ))),
                }
                if let Some(expression) = template.expressions.get(index) {
                    parts.push(Part::Expr(capture(expression)));
                }
            }
            Init::Template(parts)
        }
        Expression::BinaryExpression(binary) if binary.operator.as_str() == "+" => Init::Concat(
            Box::new(capture(&binary.left)),
            Box::new(capture(&binary.right)),
        ),
        Expression::Identifier(identifier) => Init::Ref(identifier.name.to_string()),
        Expression::BinaryExpression(_) => Init::Opaque("operator other than + is not a string"),
        Expression::NumericLiteral(_) => Init::Opaque("a number is not a string constant"),
        Expression::StaticMemberExpression(_) => {
            env_read(expr).map_or(Init::Opaque("not a constant string expression"), Init::Env)
        }
        _ => Init::Opaque("not a constant string expression"),
    }
}

/// The environment variable a `process.env.NAME` member expression reads, if that is what
/// `expr` is. Only the two-hop `process.env.NAME` shape is recognised, so an unrelated
/// member access stays opaque rather than being mistaken for an env read.
fn env_read(expr: &Expression<'_>) -> Option<String> {
    let Expression::StaticMemberExpression(outer) = expr else {
        return None;
    };
    let Expression::StaticMemberExpression(inner) = outer.object.get_inner_expression() else {
        return None;
    };
    let Expression::Identifier(root) = inner.object.get_inner_expression() else {
        return None;
    };
    if root.name == "process" && inner.property.name == "env" {
        Some(outer.property.name.to_string())
    } else {
        None
    }
}

/// Fold every captured constant into a value, filling each module's `constants` table.
///
/// Runs after resolution, so a module's `specifier → resolved path` map is available to
/// turn an import into the file that exports the name. `facts` is keyed by the same
/// repo-relative path the modules carry.
pub fn fold(model: &mut Model, facts: &HashMap<String, FoldFacts>) {
    // A module's specifier → resolved-path map, read once so the fold below borrows only
    // owned data and can write module.constants without aliasing the model.
    let specifiers: HashMap<&str, HashMap<&str, &str>> = model
        .modules
        .iter()
        .map(|module| {
            let map = module
                .imports
                .iter()
                .filter_map(|import| {
                    import
                        .resolved
                        .as_deref()
                        .map(|resolved| (import.specifier.as_str(), resolved))
                })
                .collect();
            (module.path.as_str(), map)
        })
        .collect();

    let resolver = Resolver {
        facts,
        specifiers: &specifiers,
    };

    let mut folded: HashMap<String, Vec<Constant>> = facts
        .iter()
        .map(|(path, module_facts)| {
            let mut constants: Vec<Constant> = module_facts
                .constants
                .iter()
                .map(|raw| {
                    let mut visited = HashSet::new();
                    match resolver.fold_init(&raw.init, path, &mut visited) {
                        Ok(value) => Constant {
                            name: raw.name.clone(),
                            value: Some(value),
                            opaque: false,
                            reason: None,
                            env_template: None,
                            span: raw.span,
                        },
                        Err(reason) => {
                            // Opaque: keep a `${NAME}` env template when the only thing that
                            // stopped the fold was a `process.env` read, so a rule with
                            // assumed values can complete it. A template with no hole means
                            // the opacity was not an env read, so it is discarded.
                            let mut env_visited = HashSet::new();
                            let env_template = resolver
                                .env_template_of(&raw.init, path, &mut env_visited)
                                .ok()
                                .filter(|template| template.contains("${"));
                            Constant {
                                name: raw.name.clone(),
                                value: None,
                                opaque: true,
                                reason: Some(reason),
                                env_template,
                                span: raw.span,
                            }
                        }
                    }
                })
                .collect();
            constants.sort_by_key(|constant| constant.span);
            (path.clone(), constants)
        })
        .collect();

    for module in &mut model.modules {
        if let Some(constants) = folded.remove(&module.path) {
            module.constants = constants;
        }
    }
}

/// The read-only view the fold recursion walks: every file's facts and its resolved
/// specifiers.
struct Resolver<'a> {
    facts: &'a HashMap<String, FoldFacts>,
    specifiers: &'a HashMap<&'a str, HashMap<&'a str, &'a str>>,
}

impl Resolver<'_> {
    /// Evaluate one initializer as seen from module `path`.
    fn fold_init(
        &self,
        init: &Init,
        path: &str,
        visited: &mut HashSet<String>,
    ) -> Result<String, String> {
        match init {
            Init::Str(value) => Ok(value.clone()),
            Init::Concat(left, right) => {
                Ok(self.fold_init(left, path, visited)? + &self.fold_init(right, path, visited)?)
            }
            Init::Template(parts) => {
                let mut out = String::new();
                for part in parts {
                    match part {
                        Part::Text(text) => out.push_str(text),
                        Part::Expr(expr) => out.push_str(&self.fold_init(expr, path, visited)?),
                    }
                }
                Ok(out)
            }
            Init::Ref(name) => self.value_of_name(name, path, visited),
            // An env read never folds to a value: the model does not read the environment.
            // The env-template pass (`env_template_of`) keeps it as a `${NAME}` hole instead.
            Init::Env(name) => Err(format!("`process.env.{name}` is read from the environment")),
            Init::Opaque(reason) => Err((*reason).to_owned()),
        }
    }

    /// Render `init` as a string with each `process.env.NAME` read left as a `${NAME}` hole,
    /// or `Err` when any part is opaque for a reason other than an env read (so a constant
    /// that is opaque for a genuine reason gets no misleading partial template). A constant
    /// with at least one env hole and no other opacity yields the template a rule completes.
    fn env_template_of(
        &self,
        init: &Init,
        path: &str,
        visited: &mut HashSet<String>,
    ) -> Result<String, ()> {
        match init {
            Init::Str(value) => Ok(value.clone()),
            Init::Concat(left, right) => Ok(self.env_template_of(left, path, visited)?
                + &self.env_template_of(right, path, visited)?),
            Init::Template(parts) => {
                let mut out = String::new();
                for part in parts {
                    match part {
                        Part::Text(text) => out.push_str(text),
                        Part::Expr(expr) => {
                            out.push_str(&self.env_template_of(expr, path, visited)?);
                        }
                    }
                }
                Ok(out)
            }
            // A ref may itself fold to a concrete value or to an env template; follow it.
            Init::Ref(name) => self.env_template_of_name(name, path, visited),
            Init::Env(name) => Ok(format!("${{{name}}}")),
            Init::Opaque(_) => Err(()),
        }
    }

    /// The env template of a referenced binding: its folded value when concrete, else its own
    /// env template followed through the module that defines it. `Err` when the reference is
    /// unresolvable or opaque for a non-env reason.
    fn env_template_of_name(
        &self,
        name: &str,
        path: &str,
        visited: &mut HashSet<String>,
    ) -> Result<String, ()> {
        if let Ok(value) = self.value_of_name(name, path, visited) {
            return Ok(value);
        }
        let key = format!("t\u{0}{path}\u{0}{name}");
        if !visited.insert(key.clone()) {
            return Err(());
        }
        let facts = self.facts.get(path).ok_or(())?;
        let result = facts
            .constants
            .iter()
            .find(|raw| raw.name == name)
            .ok_or(())
            .and_then(|raw| self.env_template_of(&raw.init, path, visited));
        visited.remove(&key);
        result
    }

    /// The value of a binding `name` referenced inside module `path`: a local constant,
    /// or an imported one followed to the module that exports it.
    ///
    /// The `visited` key is inserted for the duration of *this* resolution and removed
    /// before returning, so a genuine cycle (`A` folds through `A`) is caught while a name
    /// merely referenced twice in one initializer (`A + A`, a DAG) is not.
    fn value_of_name(
        &self,
        name: &str,
        path: &str,
        visited: &mut HashSet<String>,
    ) -> Result<String, String> {
        let key = format!("r\u{0}{path}\u{0}{name}");
        if !visited.insert(key.clone()) {
            return Err(format!("`{name}` is a cyclic constant reference"));
        }
        let result = self.resolve_name(name, path, visited);
        visited.remove(&key);
        result
    }

    /// The body of [`Self::value_of_name`], guarded by its cycle check.
    fn resolve_name(
        &self,
        name: &str,
        path: &str,
        visited: &mut HashSet<String>,
    ) -> Result<String, String> {
        let facts = self
            .facts
            .get(path)
            .ok_or_else(|| format!("`{name}` is not a constant"))?;
        if let Some(raw) = facts.constants.iter().find(|raw| raw.name == name) {
            return self.fold_init(&raw.init, path, visited);
        }
        if let Some(binding) = facts.imports.iter().find(|binding| binding.local == name) {
            let Some(target) = self.target_of(path, &binding.specifier) else {
                return Err(format!(
                    "`{name}` is imported from an unresolved or external module"
                ));
            };
            return self.value_of_export(&binding.imported, target, visited);
        }
        Err(format!("`{name}` is not a module-level constant"))
    }

    /// The value module `path` exports under `name`: a re-export followed to its origin, a
    /// local constant, or a name reached through a bare `export *`. Guarded like
    /// [`Self::value_of_name`], so a diamond re-export is not mistaken for a cycle.
    fn value_of_export(
        &self,
        name: &str,
        path: &str,
        visited: &mut HashSet<String>,
    ) -> Result<String, String> {
        let key = format!("e\u{0}{path}\u{0}{name}");
        if !visited.insert(key.clone()) {
            return Err(format!("`{name}` is a cyclic re-export"));
        }
        let result = self.resolve_export(name, path, visited);
        visited.remove(&key);
        result
    }

    /// The body of [`Self::value_of_export`], guarded by its cycle check.
    fn resolve_export(
        &self,
        name: &str,
        path: &str,
        visited: &mut HashSet<String>,
    ) -> Result<String, String> {
        let facts = self
            .facts
            .get(path)
            .ok_or_else(|| format!("`{name}` is exported by a file the model did not parse"))?;
        if let Some(reexport) = facts
            .reexports
            .iter()
            .find(|reexport| reexport.exported == name)
        {
            let Some(target) = self.target_of(path, &reexport.specifier) else {
                return Err(format!(
                    "`{name}` is re-exported from an unresolved or external module"
                ));
            };
            return self.value_of_export(&reexport.imported, target, visited);
        }
        if let Some(raw) = facts.constants.iter().find(|raw| raw.name == name) {
            return self.fold_init(&raw.init, path, visited);
        }
        for star in &facts.stars {
            if let Some(target) = self.target_of(path, star) {
                if let Ok(value) = self.value_of_export(name, target, visited) {
                    return Ok(value);
                }
            }
        }
        Err(format!("`{name}` is not exported as a constant by {path}"))
    }

    /// The in-project module a specifier resolves to from `path`, if any.
    fn target_of(&self, path: &str, specifier: &str) -> Option<&str> {
        let resolved = *self.specifiers.get(path)?.get(specifier)?;
        self.facts.contains_key(resolved).then_some(resolved)
    }
}
