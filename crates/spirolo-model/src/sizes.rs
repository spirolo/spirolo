//! Counting the tables a parameterised test reads from another binding.
//!
//! `it.each(SUPPORTED_CURRENCIES)(…)` and `for (const rule of pack.phrases) it(…)` define
//! one test per element of something declared elsewhere — at the top of the file, or in a
//! source module the file imports. The recogniser cannot see past its own file, so it
//! records the binding it read the table from ([`Each::source`]) and leaves the count
//! open. This module closes it: [`collect`] lifts every module-level `const` whose
//! initializer is an array or object literal (or a reference to one) into a countable
//! shape while the AST is alive, and [`resolve`] runs once over the whole model, following
//! a source through local constants, imports and re-export chains to the literal that
//! declares it, and writes the count back onto the row.
//!
//! What counts: an array literal has one case per element, a spread of another countable
//! adds its elements, an object literal has one per property (`Object.keys` / `values` /
//! `entries` of it enumerate the same), `.map(…)` keeps a length, and `.filter(…)` keeps
//! the elements whose literal property satisfies a plain predicate (`(c) => !c.hard`,
//! `(c) => c.kind === 'x'`). Anything else — a computed predicate, a call, a value decided
//! at runtime — stays uncounted, so a rule reports "could not decide" rather than a number
//! the runner never produced.

use std::collections::{HashMap, HashSet};

use oxc_ast::ast::{
    Argument, ArrayExpressionElement, BindingPattern, Declaration, Expression, ObjectPropertyKind,
    Program, PropertyKey, Statement, VariableDeclaration, VariableDeclarationKind,
};

use crate::constants::FoldFacts;
use crate::{Diagnostic, Each, Model, Severity, TestKind};

/// A countable initializer, captured as an owned tree so it survives the arena.
#[derive(Debug, Clone)]
pub enum Size {
    /// An array or object literal: its items, and the arity of a row when the literal is
    /// an array whose elements are themselves arrays.
    Items(Vec<Item>, Option<u32>),
    /// A reference to another binding (`ROWS`, `pack.phrases`), resolved during the fold.
    Ref(String),
    /// `inner.filter(predicate)`: the elements of `inner` the predicate keeps.
    Filter(Box<Self>, Predicate),
    /// Something whose length is decided at runtime.
    Opaque,
}

/// One element of a literal.
#[derive(Debug, Clone)]
pub enum Item {
    /// A plain element or property, with the literal properties it carries when it is an
    /// object literal (so a filter predicate can read them).
    One(Props),
    /// `...other`: as many cases as `other` has.
    Spread(Size),
}

/// The literal-valued properties of an object element, by key.
pub type Props = Vec<(String, Lit)>;

/// A literal an element property holds, as far as a predicate needs to know.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lit {
    /// `true` / `false`.
    Bool(bool),
    /// A string or substitution-free template.
    Str(String),
    /// A number, kept as written.
    Num(String),
    /// `null`.
    Null,
    /// `undefined`.
    Undefined,
    /// Anything else — an object, an array, a call, an identifier — whose truthiness or
    /// equality a predicate cannot decide.
    Other,
}

/// A `.filter((c) => …)` predicate over one literal property of each element.
#[derive(Debug, Clone)]
pub struct Predicate {
    /// The property the predicate reads (`hard` in `(c) => !c.hard`).
    key: String,
    /// What it asks of the property.
    test: Test,
}

/// The shapes of predicate the fold understands.
#[derive(Debug, Clone)]
enum Test {
    /// `(c) => c.key`
    Truthy,
    /// `(c) => !c.key`
    Falsy,
    /// `(c) => c.key === lit` (or `==`)
    Eq(Lit),
    /// `(c) => c.key !== lit` (or `!=`)
    Ne(Lit),
}

impl Predicate {
    /// Whether an element with `props` passes; `None` when a property's value cannot be
    /// decided (or is not a literal at all).
    fn keeps(&self, props: &Props) -> Option<bool> {
        let value = props
            .iter()
            .find(|(key, _)| *key == self.key)
            .map_or(&Lit::Undefined, |(_, value)| value);
        match &self.test {
            Test::Truthy => truthy(value),
            Test::Falsy => truthy(value).map(|t| !t),
            Test::Eq(expected) => equal(value, expected),
            Test::Ne(expected) => equal(value, expected).map(|e| !e),
        }
    }
}

/// JavaScript truthiness of a literal, when known.
fn truthy(value: &Lit) -> Option<bool> {
    match value {
        Lit::Bool(b) => Some(*b),
        Lit::Str(s) => Some(!s.is_empty()),
        Lit::Num(n) => Some(!matches!(n.as_str(), "0" | "0.0" | "-0" | "NaN")),
        Lit::Null | Lit::Undefined => Some(false),
        Lit::Other => None,
    }
}

/// Strict equality of two literals, when both are known.
fn equal(value: &Lit, expected: &Lit) -> Option<bool> {
    match (value, expected) {
        (Lit::Other, _) | (_, Lit::Other) => None,
        _ => Some(value == expected),
    }
}

/// A module-level countable binding, keyed by its dotted path (`ROWS`, `pack.phrases`).
pub struct RawSize {
    /// `NAME`, or `NAME.key.key` for a literal nested in an object property.
    pub path: String,
    /// Its countable shape.
    pub size: Size,
}

/// Capture every module-level `const` with a countable initializer, including the
/// literals nested in an object's properties, keyed by dotted path.
pub fn collect(program: &Program<'_>) -> Vec<RawSize> {
    let mut sizes = Vec::new();
    for statement in &program.body {
        match statement {
            Statement::VariableDeclaration(declaration) => {
                collect_declaration(declaration, &mut sizes);
            }
            Statement::ExportDeclaration(export) => {
                if let Declaration::VariableDeclaration(declaration) = &export.declaration {
                    collect_declaration(declaration, &mut sizes);
                }
            }
            _ => {}
        }
    }
    sizes
}

fn collect_declaration(declaration: &VariableDeclaration<'_>, sizes: &mut Vec<RawSize>) {
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
        capture_into(init, id.name.as_str(), sizes);
    }
}

/// Record `expr` under `path`, and each object property that is itself a literal under
/// `path.key`, so a test that iterates `pack.phrases` finds the array it means.
fn capture_into(expr: &Expression<'_>, path: &str, sizes: &mut Vec<RawSize>) {
    let size = capture(expr);
    if matches!(size, Size::Opaque) {
        return;
    }
    if let Expression::ObjectExpression(object) = expr.get_inner_expression() {
        for property in &object.properties {
            if let ObjectPropertyKind::ObjectProperty(property) = property {
                if let Some(key) = key_name(&property.key) {
                    capture_into(&property.value, &format!("{path}.{key}"), sizes);
                }
            }
        }
    }
    sizes.push(RawSize {
        path: path.to_owned(),
        size,
    });
}

/// The countable shape of an expression: a literal, a reference, a filter, or opaque.
///
/// TypeScript wrappers (`as const`, `satisfies`, `!`) and parentheses are looked through,
/// since the runtime sees the literal underneath.
pub fn capture(expr: &Expression<'_>) -> Size {
    match expr.get_inner_expression() {
        Expression::ArrayExpression(array) => {
            let items = array
                .elements
                .iter()
                .map(|element| match element {
                    ArrayExpressionElement::SpreadElement(spread) => {
                        Item::Spread(capture(&spread.argument))
                    }
                    _ => Item::One(element.as_expression().map(props_of).unwrap_or_default()),
                })
                .collect();
            Size::Items(items, row_arity(array.elements.first()))
        }
        Expression::ObjectExpression(object) => {
            let items = object
                .properties
                .iter()
                .map(|property| match property {
                    ObjectPropertyKind::SpreadProperty(spread) => {
                        Item::Spread(capture(&spread.argument))
                    }
                    ObjectPropertyKind::ObjectProperty(property) => {
                        Item::One(props_of(&property.value))
                    }
                })
                .collect();
            Size::Items(items, None)
        }
        Expression::Identifier(id) => Size::Ref(id.name.as_str().to_owned()),
        Expression::StaticMemberExpression(_) => dotted_path(expr).map_or(Size::Opaque, Size::Ref),
        Expression::CallExpression(call) => {
            // `Object.keys(x)` / `values` / `entries` enumerate x's properties, `x.map(…)`
            // keeps x's length, and `x.filter(p)` keeps what a readable `p` keeps.
            let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression()
            else {
                return Size::Opaque;
            };
            let method = member.property.name.as_str();
            let receiver = member.object.get_inner_expression();
            let on_object = matches!(receiver, Expression::Identifier(id) if id.name == "Object");
            if on_object && matches!(method, "keys" | "values" | "entries") {
                return call
                    .arguments
                    .first()
                    .and_then(Argument::as_expression)
                    .map_or(Size::Opaque, capture);
            }
            match method {
                "map" => capture(&member.object),
                "filter" => call
                    .arguments
                    .first()
                    .and_then(Argument::as_expression)
                    .and_then(predicate_of)
                    .map_or(Size::Opaque, |predicate| {
                        Size::Filter(Box::new(capture(&member.object)), predicate)
                    }),
                _ => Size::Opaque,
            }
        }
        _ => Size::Opaque,
    }
}

/// The literal-valued properties of an object literal; empty for any other expression.
fn props_of(expr: &Expression<'_>) -> Props {
    let Expression::ObjectExpression(object) = expr.get_inner_expression() else {
        return Vec::new();
    };
    object
        .properties
        .iter()
        .filter_map(|property| match property {
            ObjectPropertyKind::ObjectProperty(property) => {
                Some((key_name(&property.key)?, lit_of(&property.value)))
            }
            ObjectPropertyKind::SpreadProperty(_) => None,
        })
        .collect()
}

/// The literal an expression is, as far as a predicate can compare it.
fn lit_of(expr: &Expression<'_>) -> Lit {
    match expr.get_inner_expression() {
        Expression::BooleanLiteral(literal) => Lit::Bool(literal.value),
        Expression::StringLiteral(literal) => literal
            .value
            .as_str()
            .map_or(Lit::Other, |value| Lit::Str(value.to_owned())),
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
            .map_or(Lit::Other, |value| Lit::Str(value.to_owned())),
        Expression::NumericLiteral(literal) => Lit::Num(literal.value.to_string()),
        Expression::NullLiteral(_) => Lit::Null,
        Expression::Identifier(id) if id.name == "undefined" => Lit::Undefined,
        _ => Lit::Other,
    }
}

/// The predicate of a `.filter(fn)` argument, when `fn` is an arrow over one parameter
/// whose body reads one property of it: `(c) => c.key`, `(c) => !c.key`,
/// `(c) => c.key === 'x'`, `(c) => c.key !== 'x'`, with or without a `return` block.
fn predicate_of(expr: &Expression<'_>) -> Option<Predicate> {
    let Expression::ArrowFunctionExpression(arrow) = expr.get_inner_expression() else {
        return None;
    };
    let param = match arrow.params.items.as_slice() {
        [only] => match &only.pattern {
            BindingPattern::BindingIdentifier(id) => id.name.as_str(),
            _ => return None,
        },
        _ => return None,
    };
    let body = match arrow.get_expression() {
        Some(expression) => expression,
        None => match arrow.get_function_body()?.statements.as_slice() {
            [Statement::ReturnStatement(statement)] => statement.argument.as_ref()?,
            _ => return None,
        },
    };
    predicate_expr(body, param)
}

fn predicate_expr(expr: &Expression<'_>, param: &str) -> Option<Predicate> {
    match expr.get_inner_expression() {
        Expression::UnaryExpression(unary) if unary.operator.as_str() == "!" => {
            let inner = predicate_expr(&unary.argument, param)?;
            let test = match inner.test {
                Test::Truthy => Test::Falsy,
                Test::Falsy => Test::Truthy,
                Test::Eq(lit) => Test::Ne(lit),
                Test::Ne(lit) => Test::Eq(lit),
            };
            Some(Predicate {
                key: inner.key,
                test,
            })
        }
        Expression::BinaryExpression(binary) => {
            let (key, other) = match (
                param_key(&binary.left, param),
                param_key(&binary.right, param),
            ) {
                (Some(key), None) => (key, &binary.right),
                (None, Some(key)) => (key, &binary.left),
                _ => return None,
            };
            let lit = lit_of(other);
            if lit == Lit::Other {
                return None;
            }
            let test = match binary.operator.as_str() {
                "===" | "==" => Test::Eq(lit),
                "!==" | "!=" => Test::Ne(lit),
                _ => return None,
            };
            Some(Predicate { key, test })
        }
        _ => Some(Predicate {
            key: param_key(expr, param)?,
            test: Test::Truthy,
        }),
    }
}

/// `param.key` → `key`.
fn param_key(expr: &Expression<'_>, param: &str) -> Option<String> {
    let Expression::StaticMemberExpression(member) = expr.get_inner_expression() else {
        return None;
    };
    match member.object.get_inner_expression() {
        Expression::Identifier(id) if id.name == param => {
            Some(member.property.name.as_str().to_owned())
        }
        _ => None,
    }
}

/// The arity of a table row: an array row supplies one argument per element, anything
/// else is passed whole as a single argument. A spread, a hole or an empty table cannot
/// say.
fn row_arity(first: Option<&ArrayExpressionElement<'_>>) -> Option<u32> {
    match first {
        Some(ArrayExpressionElement::SpreadElement(_) | ArrayExpressionElement::Elision(_))
        | None => None,
        Some(element) => match element
            .as_expression()
            .map(Expression::get_inner_expression)
        {
            Some(Expression::ArrayExpression(row)) => u32::try_from(row.elements.len()).ok(),
            _ => Some(1),
        },
    }
}

/// `a.b.c` as `"a.b.c"`, when every link is a static member on an identifier root.
pub fn dotted_path(expr: &Expression<'_>) -> Option<String> {
    match expr.get_inner_expression() {
        Expression::Identifier(id) => Some(id.name.as_str().to_owned()),
        Expression::StaticMemberExpression(member) => {
            let head = dotted_path(&member.object)?;
            Some(format!("{head}.{}", member.property.name))
        }
        _ => None,
    }
}

/// A property key's plain name: a static identifier or a string literal.
fn key_name(key: &PropertyKey<'_>) -> Option<String> {
    match key {
        PropertyKey::StaticIdentifier(id) => Some(id.name.as_str().to_owned()),
        PropertyKey::StringLiteral(literal) => literal.value.as_str().map(str::to_owned),
        _ => None,
    }
}

// ------------------------------------------------------------------ the whole-project pass

/// Fill the count of every table and loop a test case reads from a named binding.
///
/// Runs after resolution, so an import can be followed to the module that declares the
/// binding. A source that does not lead to a literal leaves the count open.
pub fn resolve(model: &mut Model, facts: &HashMap<String, FoldFacts>) {
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

    for case in &mut model.test_cases {
        let file = case.file.clone();
        if let Some(each) = &mut case.each {
            fill(each, &file, &resolver);
        }
        for each in &mut case.loops {
            fill(each, &file, &resolver);
        }
    }

    // A table still open after the fold is decided at runtime — a call, a filesystem
    // walk, a filtered value — so how many times the runner defines the case cannot be
    // known here. Say so, per table, rather than let the row read as "defined once".
    let mut diagnostics = Vec::new();
    for case in &model.test_cases {
        if !matches!(case.kind, TestKind::Test | TestKind::Suite) {
            continue;
        }
        let what = if case.kind == TestKind::Suite {
            "suite"
        } else {
            "test"
        };
        for table in case.each.iter().chain(case.loops.iter()) {
            if table.cases.is_some() {
                continue;
            }
            let message = table.source.as_ref().map_or_else(
                || {
                    format!(
                        "the table this {what} is defined over is not a literal the model can \
                         count, so how many times it is defined is undecidable statically"
                    )
                },
                |source| {
                    format!(
                        "`{source}` is not a table the model can count (it is computed at \
                         runtime), so how many times this {what} is defined is undecidable \
                         statically"
                    )
                },
            );
            diagnostics.push(Diagnostic {
                code: "test_count_undecidable".to_owned(),
                severity: Severity::Warning,
                file: Some(case.file.clone()),
                span: Some(case.span),
                message,
            });
        }
    }
    model.diagnostics.extend(diagnostics);
}

/// Count one table from its source, when it has one and is not yet counted.
fn fill(each: &mut Each, file: &str, resolver: &Resolver<'_>) {
    if each.cases.is_some() {
        return;
    }
    let Some(source) = each.source.as_deref() else {
        return;
    };
    let mut visited = HashSet::new();
    if let Some((elements, params)) = resolver.elements_of_path(source, file, &mut visited) {
        each.cases = u32::try_from(elements.len()).ok();
        if each.params.is_none() {
            each.params = params;
        }
    }
}

/// The elements a size resolves to, with the row arity when it is an array of rows.
type Elements = (Vec<Props>, Option<u32>);

/// The read-only view the fold recursion walks.
struct Resolver<'a> {
    facts: &'a HashMap<String, FoldFacts>,
    specifiers: &'a HashMap<&'a str, HashMap<&'a str, &'a str>>,
}

impl Resolver<'_> {
    /// Evaluate a captured size as seen from module `path`.
    fn elements(&self, size: &Size, path: &str, visited: &mut HashSet<String>) -> Option<Elements> {
        match size {
            Size::Items(items, params) => {
                let mut elements = Vec::new();
                for item in items {
                    match item {
                        Item::One(props) => elements.push(props.clone()),
                        Item::Spread(inner) => {
                            elements.extend(self.elements(inner, path, visited)?.0);
                        }
                    }
                }
                Some((elements, *params))
            }
            Size::Ref(name) => self.elements_of_path(name, path, visited),
            Size::Filter(inner, predicate) => {
                let (elements, params) = self.elements(inner, path, visited)?;
                let mut kept = Vec::new();
                for props in elements {
                    if predicate.keeps(&props)? {
                        kept.push(props);
                    }
                }
                Some((kept, params))
            }
            Size::Opaque => None,
        }
    }

    /// The elements of the dotted `name` referenced inside module `path`: a local constant,
    /// or an import followed to its exporting module. Guarded against cycles.
    fn elements_of_path(
        &self,
        name: &str,
        path: &str,
        visited: &mut HashSet<String>,
    ) -> Option<Elements> {
        let key = format!("r\u{0}{path}\u{0}{name}");
        if !visited.insert(key.clone()) {
            return None;
        }
        let result = self.resolve_path(name, path, visited);
        visited.remove(&key);
        result
    }

    fn resolve_path(
        &self,
        name: &str,
        path: &str,
        visited: &mut HashSet<String>,
    ) -> Option<Elements> {
        let facts = self.facts.get(path)?;
        if let Some(raw) = facts.sizes.iter().find(|raw| raw.path == name) {
            return self.elements(&raw.size, path, visited);
        }
        let (head, rest) = split_head(name);
        let binding = facts.imports.iter().find(|binding| binding.local == head)?;
        let target = self.target_of(path, &binding.specifier)?;
        let exported = join_path(&binding.imported, rest);
        self.elements_of_export(&exported, target, visited)
    }

    /// The elements module `path` exports under the dotted `name`: a re-export followed to
    /// its origin, a local constant, or a name reached through a bare `export *`.
    fn elements_of_export(
        &self,
        name: &str,
        path: &str,
        visited: &mut HashSet<String>,
    ) -> Option<Elements> {
        let key = format!("e\u{0}{path}\u{0}{name}");
        if !visited.insert(key.clone()) {
            return None;
        }
        let result = self.resolve_export(name, path, visited);
        visited.remove(&key);
        result
    }

    fn resolve_export(
        &self,
        name: &str,
        path: &str,
        visited: &mut HashSet<String>,
    ) -> Option<Elements> {
        let facts = self.facts.get(path)?;
        let (head, rest) = split_head(name);
        if let Some(reexport) = facts
            .reexports
            .iter()
            .find(|reexport| reexport.exported == head)
        {
            let target = self.target_of(path, &reexport.specifier)?;
            return self.elements_of_export(&join_path(&reexport.imported, rest), target, visited);
        }
        if let Some(raw) = facts.sizes.iter().find(|raw| raw.path == name) {
            return self.elements(&raw.size, path, visited);
        }
        for star in &facts.stars {
            if let Some(target) = self.target_of(path, star) {
                if let Some(elements) = self.elements_of_export(name, target, visited) {
                    return Some(elements);
                }
            }
        }
        None
    }

    /// The in-project module a specifier resolves to from `path`, if any.
    fn target_of(&self, path: &str, specifier: &str) -> Option<&str> {
        let resolved = *self.specifiers.get(path)?.get(specifier)?;
        self.facts.contains_key(resolved).then_some(resolved)
    }
}

/// Split the first segment off a dotted path: `a.b.c` gives `a` and `b.c`; a bare `a`
/// gives `a` and an empty rest.
fn split_head(name: &str) -> (&str, &str) {
    name.split_once('.').unwrap_or((name, ""))
}

fn join_path(head: &str, rest: &str) -> String {
    if rest.is_empty() {
        head.to_owned()
    } else {
        format!("{head}.{rest}")
    }
}
