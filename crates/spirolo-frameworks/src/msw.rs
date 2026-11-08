//! The MSW/nock recogniser: request interceptors into the `Handler` table.
//!
//! Two libraries, one table. MSW declares handlers as calls on an imported namespace —
//! `http.get(pattern, resolver)` (v2), `rest.post(pattern, resolver)` (v1),
//! `graphql.query(name, resolver)` — while nock chains a verb off a base:
//! `nock(base).get(path)`. The method is the verb, uppercased; the pattern is the
//! first argument, folded when it is a literal and recorded as opaque (`None`) otherwise,
//! so a computed URL is kept as "could not decide" rather than dropped.
//!
//! A handler is recognised only in a handler file — by default one under a `mocks` or
//! `msw` directory, or one the configured `handlers` globs name — and only when the file
//! imports the library, so the Node built-in `http` is never mistaken for MSW's.

use oxc_ast::ast::{Argument, CallExpression, Expression, Program, Statement};
use oxc_ast_visit::{walk, Visit};
use spirolo_model::{FileContext, Handler, HandlerLibrary, PatternKind, Recogniser, Span};

/// Recognises MSW and nock request handlers. See the [module docs](self).
#[derive(Debug, Default, Clone, Copy)]
pub struct MswNock;

impl Recogniser for MswNock {
    fn handlers(&self, file: &FileContext<'_>) -> Vec<Handler> {
        // Handlers live in handler files; elsewhere an `http.get` is almost never MSW.
        if !is_handler_file(file.path, file.handler_globs) {
            return Vec::new();
        }
        let libraries = read_libraries(file.program);
        if libraries.is_empty() {
            return Vec::new();
        }
        let mut collector = Collector {
            path: file.path,
            source: file.source,
            libraries,
            constants: local_string_constants(file.program),
            rows: Vec::new(),
        };
        collector.visit_program(file.program);
        collector.rows
    }
}

/// Which libraries a file imported, so recognition is by name but never by accident.
#[derive(Default)]
struct Libraries {
    /// `msw` is imported, so `http` / `rest` / `graphql` mean MSW here.
    msw: bool,
    /// `nock` is imported, so `nock(base).verb(path)` means a nock interceptor.
    nock: bool,
}

impl Libraries {
    const fn is_empty(&self) -> bool {
        !self.msw && !self.nock
    }
}

/// The visitor that walks a handler file and collects its interceptor rows.
struct Collector<'a> {
    path: &'a str,
    source: &'a str,
    libraries: Libraries,
    /// The file's top-level `const NAME = "string"` bindings, so a pattern built from a
    /// local base constant (the common `${BASE}/x` template idiom) folds instead of being
    /// dropped as opaque.
    constants: std::collections::HashMap<String, String>,
    rows: Vec<Handler>,
}

impl<'a> Visit<'a> for Collector<'a> {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if let Some(handler) = self.handler_of(call) {
            self.rows.push(handler);
        }
        walk::walk_call_expression(self, call);
    }
}

impl Collector<'_> {
    /// The handler `call` declares, if it is one.
    fn handler_of(&self, call: &CallExpression<'_>) -> Option<Handler> {
        let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
            return None;
        };
        let property = member.property.name.as_str();

        // MSW: a call on the imported `http` / `rest` / `graphql` namespace.
        if self.libraries.msw {
            if let Expression::Identifier(object) = member.object.get_inner_expression() {
                match object.name.as_str() {
                    "http" | "rest" => return Some(self.msw_http(call, property)),
                    "graphql" => return Some(self.msw_graphql(call, property)),
                    _ => {}
                }
            }
        }

        // nock: a verb call whose object chain roots in `nock(base)`.
        if self.libraries.nock {
            if let Some(method) = http_method(property) {
                if roots_in_nock(&member.object) {
                    return Some(self.handler(call, HandlerLibrary::Nock, Some(method)));
                }
            }
        }

        None
    }

    /// An `http.get` / `rest.post` handler: the verb is the method, `all` meaning any.
    fn msw_http(&self, call: &CallExpression<'_>, verb: &str) -> Handler {
        let method = http_method(verb);
        self.handler(call, HandlerLibrary::Msw, method)
    }

    /// A `graphql.query` / `graphql.mutation` handler: no HTTP method, and the operation
    /// name stands in for the pattern.
    fn msw_graphql(&self, call: &CallExpression<'_>, _operation: &str) -> Handler {
        let (pattern, pattern_kind) = self.pattern_of(call);
        Handler {
            file: self.path.to_owned(),
            span: Span::locate(self.source, call.span.start, call.span.end),
            library: HandlerLibrary::Msw,
            method: None,
            pattern,
            pattern_kind,
        }
    }

    /// Build a handler row with a decided method and the folded first-argument pattern.
    fn handler(
        &self,
        call: &CallExpression<'_>,
        library: HandlerLibrary,
        method: Option<String>,
    ) -> Handler {
        let (pattern, pattern_kind) = self.pattern_of(call);
        Handler {
            file: self.path.to_owned(),
            span: Span::locate(self.source, call.span.start, call.span.end),
            library,
            method,
            pattern,
            pattern_kind,
        }
    }

    /// The handler's URL pattern and how it was written: a string or a template folds to its
    /// value (folding `${IDENT}` and identifier operands against the file's local string
    /// constants, so the common `` `${BASE}/x` `` idiom decides); a regex literal is
    /// [`PatternKind::Regex`] with no folded value; anything the fold cannot decide is
    /// [`PatternKind::Opaque`], kept so a computed pattern is not dropped.
    fn pattern_of(&self, call: &CallExpression<'_>) -> (Option<String>, PatternKind) {
        match call.arguments.first() {
            Some(Argument::StringLiteral(literal)) => literal
                .value
                .as_str()
                .map_or((None, PatternKind::Opaque), |value| {
                    (Some(value.to_owned()), PatternKind::String)
                }),
            Some(Argument::TemplateLiteral(template)) => self
                .fold_template(template)
                .map_or((None, PatternKind::Opaque), |value| {
                    (Some(value), PatternKind::Template)
                }),
            Some(Argument::RegExpLiteral(_)) => (None, PatternKind::Regex),
            Some(argument) => {
                argument
                    .as_expression()
                    .map_or((None, PatternKind::Opaque), |expression| {
                        // A bare identifier (`http.get(URL, …)`) or a `+` concatenation of locals
                        // folds against the file's constants; anything else is opaque.
                        self.fold_expression(expression)
                            .map_or((None, PatternKind::Opaque), |value| {
                                (Some(value), PatternKind::String)
                            })
                    })
            }
            None => (None, PatternKind::Opaque),
        }
    }

    /// Fold a template literal against the file's local string constants, or `None` when any
    /// interpolation is not a foldable identifier reference.
    fn fold_template(&self, template: &oxc_ast::ast::TemplateLiteral<'_>) -> Option<String> {
        let mut out = String::new();
        for (index, quasi) in template.quasis.iter().enumerate() {
            out.push_str(quasi.value.cooked?.as_str()?);
            if let Some(expression) = template.expressions.get(index) {
                out.push_str(&self.fold_expression(expression)?);
            }
        }
        Some(out)
    }

    /// Fold an expression to a string against the file's local constants: a string literal, a
    /// substitution-free-or-foldable template, an identifier bound to a local string
    /// constant, or a `+` concatenation of any of these. `None` for anything else.
    fn fold_expression(&self, expression: &Expression<'_>) -> Option<String> {
        match expression.get_inner_expression() {
            Expression::StringLiteral(literal) => literal.value.as_str().map(str::to_owned),
            Expression::TemplateLiteral(template) => self.fold_template(template),
            Expression::Identifier(identifier) => {
                self.constants.get(identifier.name.as_str()).cloned()
            }
            Expression::BinaryExpression(binary) if binary.operator.as_str() == "+" => {
                Some(self.fold_expression(&binary.left)? + &self.fold_expression(&binary.right)?)
            }
            // `process.env.X ?? "default"`: take the left when it folds, else the default.
            Expression::LogicalExpression(logical)
                if matches!(logical.operator.as_str(), "??" | "||") =>
            {
                self.fold_expression(&logical.left)
                    .or_else(|| self.fold_expression(&logical.right))
            }
            _ => None,
        }
    }
}

/// The method a verb names, uppercased; `None` for MSW's `all` (any method).
fn http_method(verb: &str) -> Option<String> {
    match verb {
        "get" | "post" | "put" | "delete" | "patch" | "head" | "options" => {
            Some(verb.to_uppercase())
        }
        _ => None,
    }
}

/// The file's top-level `const NAME = <foldable string>` bindings, so a handler pattern
/// built from a local base constant folds. Only module-level `const` string constants are
/// collected (a `let` may be reassigned; a nested binding is not the module's contract), and
/// only ones whose value folds against the constants declared before them.
fn local_string_constants(program: &Program<'_>) -> std::collections::HashMap<String, String> {
    let mut constants: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for statement in &program.body {
        let Statement::VariableDeclaration(declaration) = statement else {
            continue;
        };
        if declaration.kind != oxc_ast::ast::VariableDeclarationKind::Const {
            continue;
        }
        for declarator in &declaration.declarations {
            let (Some(name), Some(init)) = (declarator.id.get_identifier_name(), &declarator.init)
            else {
                continue;
            };
            if let Some(value) = fold_const(init, &constants) {
                constants.insert(name.as_str().to_owned(), value);
            }
        }
    }
    constants
}

/// Fold a top-level constant's initializer against the constants seen before it: a string,
/// a template of foldable parts, a reference to an earlier constant, or a `+` concatenation.
fn fold_const(
    expression: &Expression<'_>,
    constants: &std::collections::HashMap<String, String>,
) -> Option<String> {
    match expression.get_inner_expression() {
        Expression::StringLiteral(literal) => literal.value.as_str().map(str::to_owned),
        Expression::TemplateLiteral(template) => {
            let mut out = String::new();
            for (index, quasi) in template.quasis.iter().enumerate() {
                out.push_str(quasi.value.cooked?.as_str()?);
                if let Some(inner) = template.expressions.get(index) {
                    out.push_str(&fold_const(inner, constants)?);
                }
            }
            Some(out)
        }
        Expression::Identifier(identifier) => constants.get(identifier.name.as_str()).cloned(),
        Expression::BinaryExpression(binary) if binary.operator.as_str() == "+" => {
            Some(fold_const(&binary.left, constants)? + &fold_const(&binary.right, constants)?)
        }
        // `process.env.X ?? "default"` / `... || "default"`: the left is an environment read
        // the model does not resolve, so the fold takes the default — the value that runs when
        // the variable is unset, which is the test and dev case MSW handlers assume.
        Expression::LogicalExpression(logical)
            if matches!(logical.operator.as_str(), "??" | "||") =>
        {
            fold_const(&logical.left, constants).or_else(|| fold_const(&logical.right, constants))
        }
        _ => None,
    }
}

/// Whether an expression chain bottoms out at a `nock(base)` call, so a verb hung off it
/// (directly, or after `.reply(…)` in a chain) is a nock interceptor.
fn roots_in_nock(expr: &Expression<'_>) -> bool {
    match expr.get_inner_expression() {
        Expression::CallExpression(call) => match call.callee.get_inner_expression() {
            Expression::Identifier(id) => id.name == "nock",
            Expression::StaticMemberExpression(member) => roots_in_nock(&member.object),
            _ => false,
        },
        Expression::StaticMemberExpression(member) => roots_in_nock(&member.object),
        _ => false,
    }
}

/// Whether a path is a handler file.
///
/// When `.spirolorc.jsonc` supplied `handlers` globs, they decide it and nothing else does:
/// a file is a handler file exactly when it matches one. With no config the default holds —
/// a file under a `mocks` or `msw` directory — so the common suite needs no configuration.
fn is_handler_file(path: &str, globs: Option<&globset::GlobSet>) -> bool {
    globs.map_or_else(
        || {
            path.split('/')
                .any(|segment| segment == "mocks" || segment == "msw")
        },
        |globs| globs.is_match(path),
    )
}

/// Read the file's imports into the set of handler libraries it uses.
fn read_libraries(program: &Program<'_>) -> Libraries {
    let mut libraries = Libraries::default();
    for statement in &program.body {
        let Statement::ImportDeclaration(import) = statement else {
            continue;
        };
        match import.source.value.as_str() {
            Some("msw" | "msw/node" | "msw/browser" | "msw/native") => libraries.msw = true,
            Some("nock") => libraries.nock = true,
            _ => {}
        }
    }
    libraries
}
