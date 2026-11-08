//! Which named code each part of a file is written in, and which names each one references:
//! the raw material `flake-candidate` follows from a test into a test-only module, so that a
//! mechanism in a helper's method counts only for the tests whose code calls into it.
//!
//! **Enclosing code.** Each function is named by its own identifier or by what it is bound
//! to (see [`Enclosing`]); a class's constructor and instance field initialisers by the
//! class. A function with no name takes its enclosing function's, and at module scope the
//! name of the declarator whose initialiser holds it. At module scope, a callback passed to a
//! hook (`beforeEach(() => …)`) or a module mock (`vi.mock(…, () => …)`), and an immediately
//! invoked function, run at load and count as module scope; any other unnamed callback there
//! is [`Enclosing::Anonymous`].
//!
//! **Names.** Every identifier reference and every member property (`this.search`,
//! `provider.getById`, `this['search']`), each under the innermost enclosing code, and with
//! its byte offset. A reference to an import also names what the import names in its
//! module (`makeId` for `import { makeId as id }`, `default` for a default import), and a
//! renamed destructuring names the property it takes (`search` in `{ search: s }`). A call
//! through a computed member whose key is not a literal (`this[k]()`), or an iteration of an
//! import's values (`Object.values(mock)`), names [`FlakeNames::DYNAMIC`]: the code may call
//! anything. A default export is named `default`, or links `default` to the name it
//! declares; a renaming export (`export { a as b }`) links the exported name to the local.
//! Export lists name nothing at module scope: exporting a function does not run it. Names
//! are kept as written: the graph is by name, never by binding.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use oxc_ast::ast::{
    Argument, ArrowFunctionExpression, AssignmentExpression, AssignmentTarget, BindingPattern,
    BindingProperty, CallExpression, Class, ComputedMemberExpression, ExportDefaultDeclaration,
    ExportDefaultDeclarationKind, ExportNamedDeclaration, Expression, Function,
    IdentifierReference, ImportDeclarationSpecifier, MethodDefinition, MethodDefinitionKind,
    ObjectProperty, Program, PropertyDefinition, Statement, StaticMemberExpression,
    VariableDeclarator,
};
use oxc_ast::StaticPropertyName;
use oxc_ast_visit::{walk, Visit};
use oxc_span::GetSpan;
use oxc_syntax::scope::ScopeFlags;

use crate::flake::hook_of;
use crate::{Enclosing, FlakeNames};

/// A file's enclosing-code ranges and its names.
pub struct Holders {
    /// Each function (or class initialiser) range with the code it is, in walk order, so a
    /// range nested in another comes after it.
    ranges: Vec<(u32, u32, Enclosing)>,
    /// The file's names, by enclosing code, with their offsets.
    pub names: FlakeNames,
}

impl Holders {
    /// The code the byte at `offset` is written in: the innermost range holding it, or
    /// module scope.
    pub fn enclosing(&self, offset: u32) -> Enclosing {
        self.ranges
            .iter()
            .filter(|(start, end, _)| *start <= offset && offset < *end)
            .max_by_key(|(start, _, _)| *start)
            .map_or(Enclosing::Module, |(_, _, enclosing)| enclosing.clone())
    }
}

/// Walk `program` and record its enclosing-code ranges and names.
pub fn collect(path: &str, program: &Program<'_>) -> Holders {
    let mut walker = Walker {
        current: Enclosing::Module,
        pending: None,
        declarator: None,
        classes: Vec::new(),
        at_load: HashSet::new(),
        imports: imports(program),
        holders: Holders {
            ranges: Vec::new(),
            names: FlakeNames {
                file: path.to_owned(),
                ..FlakeNames::default()
            },
        },
    };
    walker.visit_program(program);
    walker.holders
}

struct Walker {
    /// The code being walked.
    current: Enclosing,
    /// The name the next function or class takes from what it is bound to.
    pending: Option<String>,
    /// At module scope, the declarator whose initialiser is being walked.
    declarator: Option<String>,
    /// The enclosing classes' codes, innermost last.
    classes: Vec<Enclosing>,
    /// The starts of module-scope callbacks that run at load.
    at_load: HashSet<u32>,
    /// Each import's local name, with what it names in its module (`None` for a namespace).
    imports: HashMap<String, Option<String>>,
    holders: Holders,
}

/// Each import's local name, with the name it imports: the imported name, `default` for a
/// default import, `None` for a namespace import.
fn imports(program: &Program<'_>) -> HashMap<String, Option<String>> {
    let mut imports = HashMap::new();
    for statement in &program.body {
        let Statement::ImportDeclaration(declaration) = statement else {
            continue;
        };
        for specifier in declaration.specifiers.iter().flatten() {
            let (local, imported) = match specifier {
                ImportDeclarationSpecifier::ImportSpecifier(s) => {
                    (&s.local, Some(s.imported.name().to_string()))
                }
                ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                    (&s.local, Some("default".to_owned()))
                }
                ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => (&s.local, None),
            };
            imports.insert(local.name.to_string(), imported);
        }
    }
    imports
}

impl Walker {
    /// Whether `call` may call anything: through a computed member whose key is not a
    /// literal (`this[k]()`), or over an import's values (`Object.values(mock)`).
    fn dynamic(&self, call: &CallExpression<'_>) -> bool {
        match call.callee.get_inner_expression() {
            Expression::ComputedMemberExpression(member) => {
                literal_key(&member.expression).is_none()
            }
            Expression::StaticMemberExpression(member) => {
                matches!(member.property.name.as_str(), "values" | "entries")
                    && matches!(member.object.get_inner_expression(),
                        Expression::Identifier(id) if id.name == "Object")
                    && call.arguments.first().and_then(Argument::as_expression).is_some_and(
                        |argument| matches!(argument.get_inner_expression(),
                            Expression::Identifier(id) if self.imports.contains_key(id.name.as_str())),
                    )
            }
            _ => false,
        }
    }

    /// The code an unnamed function starting at `start` is.
    fn unnamed(&self, start: u32) -> Enclosing {
        match &self.current {
            Enclosing::Module if self.at_load.contains(&start) => Enclosing::Module,
            Enclosing::Module => self
                .declarator
                .clone()
                .map_or(Enclosing::Anonymous, Enclosing::Named),
            other => other.clone(),
        }
    }

    /// Walk a range of code that is `enclosing`.
    fn enter(&mut self, start: u32, end: u32, enclosing: Enclosing, walk: impl FnOnce(&mut Self)) {
        self.holders.ranges.push((start, end, enclosing.clone()));
        let current = std::mem::replace(&mut self.current, enclosing);
        let declarator = self.declarator.take();
        walk(self);
        self.current = current;
        self.declarator = declarator;
    }

    /// Record a reference to `name` at `offset` under the current code.
    fn name(&mut self, offset: u32, name: &str) {
        self.holders
            .names
            .calls
            .entry(self.current.clone())
            .or_default()
            .insert(name.to_owned());
        self.holders.names.uses.push((offset, name.to_owned()));
    }

    /// Link the named code `from` to `to`, as though it referenced it.
    fn link(&mut self, from: &str, to: &str) {
        self.holders
            .names
            .calls
            .entry(Enclosing::Named(from.to_owned()))
            .or_default()
            .insert(to.to_owned());
    }

    /// The code a class's constructor and instance initialisers are: the innermost class.
    fn class(&self) -> Enclosing {
        self.classes.last().cloned().unwrap_or(Enclosing::Anonymous)
    }
}

/// Whether `expr` is a function or class a binding can name.
fn is_nameable(expr: &Expression<'_>) -> bool {
    matches!(
        expr.get_inner_expression(),
        Expression::FunctionExpression(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::ClassExpression(_)
    )
}

/// The name a computed key spells as a literal: `'search'`, or a template with no
/// substitution.
fn literal_key(key: &Expression<'_>) -> Option<String> {
    match key.get_inner_expression() {
        Expression::StringLiteral(literal) => literal.value.as_str().map(str::to_owned),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => template
            .quasis
            .first()
            .and_then(|quasi| quasi.value.cooked?.as_str())
            .map(str::to_owned),
        _ => None,
    }
}

/// Whether `call`, at module scope, runs the callbacks it is passed at load: a hook, a
/// `vi`/`jest` module mock, or an immediately invoked function.
fn runs_at_load(call: &CallExpression<'_>) -> bool {
    let callee = call.callee.get_inner_expression();
    hook_of(callee).is_some()
        || matches!(
            callee,
            Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_)
        )
        || matches!(callee, Expression::StaticMemberExpression(member)
            if matches!(member.property.name.as_str(), "mock" | "doMock" | "unstable_mockModule")
                && matches!(member.object.get_inner_expression(),
                    Expression::Identifier(id) if id.name == "vi" || id.name == "jest"))
}

impl<'a> Visit<'a> for Walker {
    fn visit_function(&mut self, func: &Function<'a>, flags: ScopeFlags) {
        let named = self
            .pending
            .take()
            .or_else(|| func.id.as_ref().map(|id| id.name.to_string()));
        let enclosing = named.map_or_else(|| self.unnamed(func.span.start), Enclosing::Named);
        self.enter(func.span.start, func.span.end, enclosing, |walker| {
            walk::walk_function(walker, func, flags);
        });
    }

    fn visit_arrow_function_expression(&mut self, arrow: &ArrowFunctionExpression<'a>) {
        let enclosing = self
            .pending
            .take()
            .map_or_else(|| self.unnamed(arrow.span.start), Enclosing::Named);
        self.enter(arrow.span.start, arrow.span.end, enclosing, |walker| {
            walk::walk_arrow_function_expression(walker, arrow);
        });
    }

    fn visit_class(&mut self, class: &Class<'a>) {
        let named = self
            .pending
            .take()
            .or_else(|| class.id.as_ref().map(|id| id.name.to_string()));
        let enclosing = named.map_or_else(|| self.unnamed(class.span.start), Enclosing::Named);
        self.classes.push(enclosing);
        walk::walk_class(self, class);
        self.classes.pop();
    }

    fn visit_method_definition(&mut self, method: &MethodDefinition<'a>) {
        self.pending = if method.kind == MethodDefinitionKind::Constructor {
            match self.class() {
                Enclosing::Named(name) => Some(name),
                _ => None,
            }
        } else if method.computed {
            None
        } else {
            method
                .key
                .static_name()
                .and_then(StaticPropertyName::into_cow_str)
                .map(Cow::into_owned)
        };
        if self.pending.is_none() && method.kind == MethodDefinitionKind::Constructor {
            // An unnamed class's constructor is the class's code.
            let class = self.class();
            let value = &method.value;
            self.enter(value.span.start, value.span.end, class, |walker| {
                walk::walk_function(walker, value, ScopeFlags::Constructor);
            });
            return;
        }
        walk::walk_method_definition(self, method);
        self.pending = None;
    }

    fn visit_property_definition(&mut self, property: &PropertyDefinition<'a>) {
        let Some(value) = &property.value else {
            walk::walk_property_definition(self, property);
            return;
        };
        if is_nameable(value) && !property.computed {
            self.pending = property
                .key
                .static_name()
                .and_then(StaticPropertyName::into_cow_str)
                .map(Cow::into_owned);
            walk::walk_property_definition(self, property);
            self.pending = None;
        } else if property.r#static {
            // A static initialiser runs where the class is defined.
            walk::walk_property_definition(self, property);
        } else {
            // An instance initialiser runs on `new`, as the class's code.
            let span = value.span();
            let class = self.class();
            self.enter(span.start, span.end, class, |walker| {
                walk::walk_property_definition(walker, property);
            });
        }
    }

    fn visit_object_property(&mut self, property: &ObjectProperty<'a>) {
        if is_nameable(&property.value) && !property.computed {
            self.pending = property
                .key
                .static_name()
                .and_then(StaticPropertyName::into_cow_str)
                .map(Cow::into_owned);
        }
        walk::walk_object_property(self, property);
        self.pending = None;
    }

    fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
        let name = match &declarator.id {
            BindingPattern::BindingIdentifier(id) => Some(id.name.to_string()),
            _ => None,
        };
        let Some(init) = &declarator.init else {
            walk::walk_variable_declarator(self, declarator);
            return;
        };
        if is_nameable(init) {
            self.pending.clone_from(&name);
        }
        let outer = if self.current == Enclosing::Module {
            std::mem::replace(&mut self.declarator, name)
        } else {
            self.declarator.clone()
        };
        walk::walk_variable_declarator(self, declarator);
        self.declarator = outer;
        self.pending = None;
    }

    fn visit_assignment_expression(&mut self, assignment: &AssignmentExpression<'a>) {
        if is_nameable(&assignment.right) {
            if let AssignmentTarget::StaticMemberExpression(member) = &assignment.left {
                let module_exports = member.property.name == "exports"
                    && matches!(&member.object, Expression::Identifier(id) if id.name == "module");
                if !module_exports {
                    self.pending = Some(member.property.name.to_string());
                }
            }
        }
        walk::walk_assignment_expression(self, assignment);
        self.pending = None;
    }

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if self.dynamic(call) {
            self.name(call.span.start, FlakeNames::DYNAMIC);
        }
        if self.current == Enclosing::Module && runs_at_load(call) {
            self.at_load
                .insert(call.callee.get_inner_expression().span().start);
            for argument in &call.arguments {
                if let Some(expr) = argument.as_expression() {
                    if !matches!(argument, Argument::SpreadElement(_)) {
                        self.at_load
                            .insert(expr.get_inner_expression().span().start);
                    }
                }
            }
        }
        walk::walk_call_expression(self, call);
    }

    fn visit_identifier_reference(&mut self, id: &IdentifierReference<'a>) {
        self.name(id.span.start, id.name.as_str());
        if let Some(Some(imported)) = self.imports.get(id.name.as_str()) {
            if imported != id.name.as_str() {
                let imported = imported.clone();
                self.name(id.span.start, &imported);
            }
        }
    }

    fn visit_computed_member_expression(&mut self, member: &ComputedMemberExpression<'a>) {
        walk::walk_computed_member_expression(self, member);
        if let Some(key) = literal_key(&member.expression) {
            self.name(member.expression.span().start, &key);
        }
    }

    fn visit_binding_property(&mut self, property: &BindingProperty<'a>) {
        if !property.computed {
            if let Some(key) = property
                .key
                .static_name()
                .and_then(StaticPropertyName::into_cow_str)
            {
                self.name(property.key.span().start, &key);
            }
        }
        walk::walk_binding_property(self, property);
    }

    fn visit_export_default_declaration(&mut self, export: &ExportDefaultDeclaration<'a>) {
        match &export.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(func) => match &func.id {
                Some(id) => self.link("default", id.name.as_str()),
                None => self.pending = Some("default".to_owned()),
            },
            ExportDefaultDeclarationKind::ClassDeclaration(class) => match &class.id {
                Some(id) => self.link("default", id.name.as_str()),
                None => self.pending = Some("default".to_owned()),
            },
            ExportDefaultDeclarationKind::Identifier(id) => {
                // Exporting a binding runs nothing: link the name, record no reference.
                self.link("default", id.name.as_str());
                return;
            }
            declaration => {
                if declaration.as_expression().is_some_and(is_nameable) {
                    self.pending = Some("default".to_owned());
                }
            }
        }
        walk::walk_export_default_declaration(self, export);
        self.pending = None;
    }

    fn visit_export_named_declaration(&mut self, export: &ExportNamedDeclaration<'a>) {
        // An export list runs nothing: link renamed names, walk nothing.
        for specifier in &export.specifiers {
            let (local, exported) = (specifier.local.name(), specifier.exported.name());
            if local != exported {
                self.link(&exported, &local);
            }
        }
    }

    fn visit_static_member_expression(&mut self, member: &StaticMemberExpression<'a>) {
        walk::walk_static_member_expression(self, member);
        self.name(member.property.span.start, member.property.name.as_str());
    }
}
