//! What a test body depends on outside its own text: [`TestCase::context`].
//!
//! The body hash reads the callback alone, so two bodies of equal text hash equal whatever
//! their free names are bound to: a file-level helper of each file, a `const` of each
//! `describe`. The recogniser keeps the declarations in scope while it walks — module scope,
//! then each enclosing function, body and block — and resolves every free name a test body
//! reads against them, innermost first. A name nothing in the file declares is a global
//! (`expect`, `window`) and is left out: its name is already in the hash.
//!
//! [`TestCase::context`]: spirolo_model::TestCase::context

use std::cell::Cell;
use std::collections::{BTreeSet, HashSet};

use oxc_ast::ast::{
    Argument, ArrowFunctionExpression, BindingIdentifier, BindingPattern, CallExpression,
    Expression, FormalParameters, Function, IdentifierReference, ImportDeclarationSpecifier,
    ImportExpression, Statement, VariableDeclaration, VariableDeclarator,
};
use oxc_ast::StaticPropertyName;
use oxc_ast_visit::{walk, Visit};
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use spirolo_model::{ContextRef, LineIndex};

use crate::vitest::claimed_framework;

/// What one name in scope is bound to.
#[derive(Debug, Clone)]
enum Declared {
    /// A declaration in the file: the binding identifier's location.
    Binding(spirolo_model::Span),
    /// An import: the specifier as written and the export it binds.
    Import { specifier: String, export: String },
}

/// One scope's declarations, by name.
#[derive(Debug, Default)]
pub struct Frame(Vec<(String, Declared)>);

/// The declarations in scope at the walk's current point, module scope first.
#[derive(Debug, Default)]
pub struct Scopes {
    frames: Vec<Frame>,
}

impl Scopes {
    /// Open a scope holding `frame`'s declarations.
    pub fn enter(&mut self, frame: Frame) {
        self.frames.push(frame);
    }

    /// Close the innermost scope.
    pub fn leave(&mut self) {
        self.frames.pop();
    }

    /// What each free name `callback` reads is bound to in the scopes open now, sorted and
    /// deduplicated.
    pub fn context_of(&self, callback: &Expression<'_>) -> Vec<ContextRef> {
        let mut free = FreeNames::default();
        free.visit_expression(callback);
        let mut context: BTreeSet<ContextRef> = free
            .specifiers
            .iter()
            .cloned()
            .map(|specifier| ContextRef::Specifier { specifier })
            .collect();
        for name in free.free_reads() {
            if let Some(declared) = self.resolve(&name) {
                context.insert(match declared {
                    Declared::Binding(span) => ContextRef::Binding {
                        name: name.clone(),
                        span: *span,
                    },
                    Declared::Import { specifier, export } => ContextRef::Import {
                        name: name.clone(),
                        specifier: specifier.clone(),
                        export: export.clone(),
                    },
                });
            }
        }
        context.into_iter().collect()
    }

    /// The innermost declaration of `name`, if the file declares it.
    fn resolve(&self, name: &str) -> Option<&Declared> {
        self.frames.iter().rev().find_map(|frame| {
            frame
                .0
                .iter()
                .rev()
                .find(|(declared, _)| declared == name)
                .map(|(_, declared)| declared)
        })
    }
}

/// The declarations `statements` make directly: imports, `var` / `let` / `const`, functions
/// and classes, not those of a nested block.
pub fn statements_frame(lines: &LineIndex, statements: &[Statement<'_>]) -> Frame {
    let mut names = Declarations::new(lines);
    for statement in statements {
        match statement {
            // The runner's own vocabulary is the injected globals by another route: a file
            // that imports `expect` reads what a file that does not reads.
            Statement::ImportDeclaration(import)
                if import
                    .source
                    .value
                    .as_str()
                    .and_then(claimed_framework)
                    .is_none() =>
            {
                // A lone surrogate names no module: like `const x = require(expr)`, each
                // local is a binding of the file.
                let Some(source) = import.source.value.as_str() else {
                    for specifier in import.specifiers.iter().flatten() {
                        names.visit_binding_identifier(specifier.local());
                    }
                    continue;
                };
                for specifier in import.specifiers.iter().flatten() {
                    let export = match specifier {
                        ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                            specifier.imported.name().as_str().to_owned()
                        }
                        ImportDeclarationSpecifier::ImportDefaultSpecifier(_) => {
                            "default".to_owned()
                        }
                        ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => "*".to_owned(),
                    };
                    names.frame.0.push((
                        specifier.local().name.as_str().to_owned(),
                        Declared::Import {
                            specifier: source.to_owned(),
                            export,
                        },
                    ));
                }
            }
            Statement::VariableDeclaration(declaration) => {
                for declarator in &declaration.declarations {
                    match required(declarator) {
                        Some(specifier) => names.require(specifier, &declarator.id),
                        None => names.visit_binding_pattern(&declarator.id),
                    }
                }
            }
            Statement::FunctionDeclaration(function) => {
                if let Some(id) = &function.id {
                    names.visit_binding_identifier(id);
                }
            }
            Statement::ClassDeclaration(class) => {
                if let Some(id) = &class.id {
                    names.visit_binding_identifier(id);
                }
            }
            _ => {}
        }
    }
    names.frame
}

/// The declarations a function's parameters make.
pub fn parameters_frame(lines: &LineIndex, params: &FormalParameters<'_>) -> Frame {
    let mut names = Declarations::new(lines);
    names.visit_formal_parameters(params);
    names.frame
}

/// Collects the binding identifiers it is walked over as declarations.
struct Declarations<'l> {
    lines: &'l LineIndex,
    frame: Frame,
}

impl<'l> Declarations<'l> {
    fn new(lines: &'l LineIndex) -> Self {
        Self {
            lines,
            frame: Frame::default(),
        }
    }
}

impl Declarations<'_> {
    /// Declare what `const pattern = require(specifier)` binds: the whole module as `*`, or
    /// each destructured property as that export. A property that is itself a pattern, or
    /// whose key is computed, is declared as a binding of the file.
    fn require(&mut self, specifier: &str, pattern: &BindingPattern<'_>) {
        let import = |export: &str| Declared::Import {
            specifier: specifier.to_owned(),
            export: export.to_owned(),
        };
        match pattern {
            BindingPattern::BindingIdentifier(id) => {
                self.frame
                    .0
                    .push((id.name.as_str().to_owned(), import("*")));
            }
            BindingPattern::ObjectPattern(object) => {
                for property in &object.properties {
                    match (
                        property
                            .key
                            .static_name()
                            .and_then(StaticPropertyName::into_cow_str),
                        &property.value,
                    ) {
                        (Some(export), BindingPattern::BindingIdentifier(id))
                            if !property.computed =>
                        {
                            self.frame
                                .0
                                .push((id.name.as_str().to_owned(), import(&export)));
                        }
                        _ => self.visit_binding_property(property),
                    }
                }
                if let Some(rest) = &object.rest {
                    self.visit_binding_rest_element(rest);
                }
            }
            _ => self.visit_binding_pattern(pattern),
        }
    }
}

/// The specifier of `require('…')` when a declarator binds one directly.
fn required<'a>(declarator: &'a VariableDeclarator<'a>) -> Option<&'a str> {
    let Some(Expression::CallExpression(call)) = declarator
        .init
        .as_ref()
        .map(Expression::get_inner_expression)
    else {
        return None;
    };
    let Expression::Identifier(callee) = &call.callee else {
        return None;
    };
    if callee.name != "require" || call.arguments.len() != 1 {
        return None;
    }
    match call.arguments.first().and_then(Argument::as_expression) {
        Some(Expression::StringLiteral(literal)) => literal.value.as_str(),
        _ => None,
    }
}

impl<'a> Visit<'a> for Declarations<'_> {
    fn visit_binding_identifier(&mut self, identifier: &BindingIdentifier<'a>) {
        let span = self
            .lines
            .locate(identifier.span.start, identifier.span.end);
        self.frame
            .0
            .push((identifier.name.as_str().to_owned(), Declared::Binding(span)));
    }
}

/// The names a callback reads that no scope inside it binds, and the relative module
/// specifiers it loads.
///
/// Each scope the walk opens — the callback's own, then each nested function, block, loop
/// and catch — keeps the names it binds, and each read keeps the scope it is made in. A read
/// is free when no scope from its own outward to the callback's binds its name, so a
/// parameter of an inner callback hides only the reads inside that callback. The check runs
/// after the walk, as a declaration later in the scope still binds an earlier read.
#[derive(Default)]
struct FreeNames {
    scopes: Vec<Scope>,
    /// The innermost open scope, `None` before the walk enters the callback.
    current: Option<usize>,
    reads: Vec<(String, Option<usize>)>,
    /// Inside a `var` declaration, whose names belong to the enclosing function.
    in_var: bool,
    specifiers: BTreeSet<String>,
}

/// One scope inside the callback: the scope it opens in, whether it is a function's, and
/// the names it binds.
struct Scope {
    parent: Option<usize>,
    function: bool,
    bound: HashSet<String>,
}

impl FreeNames {
    fn load(&mut self, source: Option<&Expression<'_>>) {
        if let Some(Expression::StringLiteral(literal)) = source {
            if let Some(specifier) = literal.value.as_str() {
                if specifier.starts_with('.') {
                    self.specifiers.insert(specifier.to_owned());
                }
            }
        }
    }

    /// Bind `name` in the current scope, or, for a `var`, in the function scope around it.
    fn bind(&mut self, name: &str, hoisted: bool) {
        let mut scope = self.current;
        if hoisted {
            while let Some(index) = scope.filter(|&index| !self.scopes[index].function) {
                scope = self.scopes[index].parent;
            }
        }
        if let Some(index) = scope {
            self.scopes[index].bound.insert(name.to_owned());
        }
    }

    /// The names read where no scope out to the callback's binds them, sorted.
    fn free_reads(&self) -> BTreeSet<String> {
        self.reads
            .iter()
            .filter(|(name, mut scope)| {
                while let Some(index) = scope {
                    if self.scopes[index].bound.contains(name) {
                        return false;
                    }
                    scope = self.scopes[index].parent;
                }
                true
            })
            .map(|(name, _)| name.clone())
            .collect()
    }
}

impl<'a> Visit<'a> for FreeNames {
    fn enter_scope(&mut self, flags: ScopeFlags, _: &Cell<Option<ScopeId>>) {
        self.scopes.push(Scope {
            parent: self.current,
            function: flags.is_function(),
            bound: HashSet::new(),
        });
        self.current = Some(self.scopes.len() - 1);
    }

    fn leave_scope(&mut self) {
        self.current = self.current.and_then(|index| self.scopes[index].parent);
    }

    fn visit_binding_identifier(&mut self, identifier: &BindingIdentifier<'a>) {
        self.bind(identifier.name.as_str(), self.in_var);
    }

    fn visit_identifier_reference(&mut self, identifier: &IdentifierReference<'a>) {
        self.reads
            .push((identifier.name.as_str().to_owned(), self.current));
    }

    fn visit_variable_declaration(&mut self, declaration: &VariableDeclaration<'a>) {
        let outer = std::mem::replace(&mut self.in_var, declaration.kind.is_var());
        walk::walk_variable_declaration(self, declaration);
        self.in_var = outer;
    }

    fn visit_function(&mut self, function: &Function<'a>, flags: ScopeFlags) {
        // A declaration's name is bound where it is declared; the walk binds it inside.
        if function.is_declaration() {
            if let Some(id) = &function.id {
                self.bind(id.name.as_str(), false);
            }
        }
        let outer = std::mem::replace(&mut self.in_var, false);
        walk::walk_function(self, function, flags);
        self.in_var = outer;
    }

    fn visit_arrow_function_expression(&mut self, arrow: &ArrowFunctionExpression<'a>) {
        let outer = std::mem::replace(&mut self.in_var, false);
        walk::walk_arrow_function_expression(self, arrow);
        self.in_var = outer;
    }

    fn visit_import_expression(&mut self, import: &ImportExpression<'a>) {
        self.load(Some(&import.source));
        walk::walk_import_expression(self, import);
    }

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if matches!(&call.callee, Expression::Identifier(callee) if callee.name == "require") {
            self.load(call.arguments.first().and_then(Argument::as_expression));
        }
        walk::walk_call_expression(self, call);
    }
}
