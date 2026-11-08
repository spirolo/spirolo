//! What a recognised leaf case *does*: its assertions, its mocks, and a body hash.
//!
//! These read the test's callback — the last function argument of the definition call —
//! and nothing else. They are shared by every recogniser that has a callback to read, so
//! the Vitest/Jest and (later) Playwright families report assertions and mocks the same
//! way. Everything here is a pure function of the callback and the file's source text.
//! [`mock_of`] is also what the file walk uses for a mock written outside any case.

use std::collections::{HashMap, HashSet};

use crate::kind_code;
use oxc_ast::ast::{
    Argument, ArrowFunctionBody, BindingPattern, CallExpression, ChainElement, Expression,
    TSInterfaceHeritage, TSTypeName, VariableDeclarator,
};
use oxc_ast::{AstKind, AstType};
use oxc_ast_visit::{walk, Visit};
use oxc_span::GetSpan;
use spirolo_model::{Assertion, LineIndex, Mock, MockKind, Span, ThrowingQuery, ValueLess};
use xxhash_rust::xxh3::xxh3_64_with_seed;

/// Fixed seed for the body hash, so two runs over the same tree hash equal (the
/// determinism the model promises). The value itself is arbitrary.
const BODY_HASH_SEED: u64 = 0x6173_7361_795f_6268;

/// The callback of a definition call: the last argument that is a function, so the
/// options-object form (`test("t", { retry: 2 }, () => {})`) and the `.each` form
/// (`test.each(table)("t", (a) => {})`) both find it. `None` for a bodyless case such
/// as `test.todo("later")`.
pub fn callback_of<'a>(call: &'a CallExpression<'a>) -> Option<&'a Expression<'a>> {
    call.arguments
        .iter()
        .rev()
        .find_map(|argument| match argument {
            Argument::ArrowFunctionExpression(_) | Argument::FunctionExpression(_) => {
                argument.as_expression()
            }
            _ => None,
        })
}

/// Whether a callback's body holds no statements — `() => {}`, `function () {}` — so the
/// case it defines runs nothing. A concise arrow (`() => x`) holds its expression.
pub fn is_empty_callback(callback: &Expression<'_>) -> bool {
    match callback.get_inner_expression() {
        Expression::ArrowFunctionExpression(arrow) => matches!(
            &arrow.body,
            ArrowFunctionBody::FunctionBody(body) if body.statements.is_empty()
        ),
        Expression::FunctionExpression(function) => function
            .body
            .as_ref()
            .is_some_and(|body| body.statements.is_empty()),
        _ => false,
    }
}

/// The assertions, mocks and throwing queries a callback installs in its own body, each
/// sorted by span.
pub struct BodyFacts {
    /// `expect` chains, `assert*` calls and `expectTypeOf` chains, in source order.
    pub assertions: Vec<Assertion>,
    /// `vi.mock` / `jest.mock`, `vi.spyOn`, `vi.fn`, fake timers and MSW `server.use`,
    /// in source order.
    pub mocks: Vec<Mock>,
    /// Throwing Testing Library queries (`getBy*`, `findBy*`, `within(...).getBy*`), in
    /// source order — each an implicit anchor on the render.
    pub throwing_queries: Vec<ThrowingQuery>,
}

/// Read a callback's own assertions, mocks and throwing queries. Descends the whole callback
/// subtree, so a helper closure's `expect` — or a query inside `waitFor(…)` — is still the
/// case's.
pub fn body_facts(lines: &LineIndex, source: &str, callback: &Expression<'_>) -> BodyFacts {
    let mut calls = CallBindings::default();
    calls.visit_expression(callback);
    let mut collector = BodyCollector {
        lines,
        source,
        calls: calls.names,
        assertions: Vec::new(),
        mocks: Vec::new(),
        throwing_queries: Vec::new(),
    };
    collector.visit_expression(callback);
    collector
        .assertions
        .sort_by_key(|row| (row.span.start, row.span.end));
    collector
        .mocks
        .sort_by_key(|row| (row.span.start, row.span.end));
    collector
        .throwing_queries
        .sort_by_key(|row| (row.span.start, row.span.end));
    BodyFacts {
        assertions: collector.assertions,
        mocks: collector.mocks,
        throwing_queries: collector.throwing_queries,
    }
}

/// A stable hash of the callback's shape: node kinds and kept literals, with locally
/// bound identifiers normalised to their declaration order so a consistent rename does
/// not change it, and the title excluded because it is not part of the callback.
///
/// Same seed, same bytes, same hash on every run — the body-dedup rule compares these.
/// Node kinds are written as the frozen codes in [`kind_code`], not oxc's own numbering,
/// and the shapes oxc reshaped since 0.110 are written as 0.110 read them, so an oxc bump
/// does not move a hash (and with it a finding id or a ratchet baseline).
pub fn body_hash(callback: &Expression<'_>) -> String {
    let mut bindings = BindingIndex::default();
    bindings.visit_expression(callback);

    let mut walker = HashWalker {
        bindings: &bindings.order,
        bytes: Vec::new(),
    };
    walker.visit_expression(callback);

    format!("{:016x}", xxh3_64_with_seed(&walker.bytes, BODY_HASH_SEED))
}

// ------------------------------------------------------------- assertions and mocks

/// Visits a callback body, collecting its assertion, mock and throwing-query calls.
struct BodyCollector<'a> {
    lines: &'a LineIndex,
    source: &'a str,
    /// The names a `const`/`let` in the callback binds to a call: see [`CallBindings`].
    calls: HashSet<String>,
    assertions: Vec<Assertion>,
    mocks: Vec<Mock>,
    throwing_queries: Vec<ThrowingQuery>,
}

impl<'a> Visit<'a> for BodyCollector<'a> {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if let Some(assertion) = self.assertion_of(call) {
            self.assertions.push(assertion);
        }
        if let Some(mock) = mock_of(self.lines, self.source, call) {
            self.mocks.push(mock);
        }
        if let Some(query) = throwing_query_of(self.lines, call) {
            self.throwing_queries.push(query);
        }
        walk::walk_call_expression(self, call);
    }
}

/// The throwing Testing Library query `call` is, if it is one: a `getBy*`, `getAllBy*`,
/// `findBy*` or `findAllBy*` call — bare (`getByRole(…)`), on `screen`
/// (`screen.getByText(…)`), or through `within(...)` (`within(row).getByTestId(…)`). The
/// `queryBy*` / `queryAllBy*` family returns `null` rather than throwing and is *not* a query
/// here, so a test built only on it is not treated as anchored. `None` for anything else.
pub fn throwing_query_of(lines: &LineIndex, call: &CallExpression<'_>) -> Option<ThrowingQuery> {
    let span = lines.locate(call.span.start, call.span.end);
    match call.callee.get_inner_expression() {
        // A bare query call — `getByRole(...)` — its callee a plain identifier.
        Expression::Identifier(id) if is_throwing_query(&id.name) => Some(ThrowingQuery {
            name: id.name.as_str().to_owned(),
            within: false,
            span,
        }),
        // A member query — `screen.getByText(...)` or `within(row).getByTestId(...)`.
        Expression::StaticMemberExpression(member) if is_throwing_query(&member.property.name) => {
            let within = matches!(
                member.object.get_inner_expression(),
                Expression::CallExpression(inner)
                    if matches!(
                        inner.callee.get_inner_expression(),
                        Expression::Identifier(object) if object.name == "within"
                    )
            );
            Some(ThrowingQuery {
                name: member.property.name.as_str().to_owned(),
                within,
                span,
            })
        }
        _ => None,
    }
}

/// Whether a method name is a *throwing* Testing Library query: the `getBy*` / `getAllBy*` /
/// `findBy*` / `findAllBy*` families, which raise when they match nothing. The `queryBy*` /
/// `queryAllBy*` families return `null`/`[]` and so are excluded — that is the exact line the
/// `absence-anchor` rule reasons about.
fn is_throwing_query(name: &str) -> bool {
    ThrowingQuery::is_named(name)
}

/// The mock `call` installs, if it is one: `vi`/`jest` module and function mocks and
/// fake timers, or an MSW/nock `server.use` / `worker.use`. Shared by the body collector
/// and the file walk, so a mock reads the same whether it sits in a case or outside one.
pub fn mock_of(lines: &LineIndex, source: &str, call: &CallExpression<'_>) -> Option<Mock> {
    let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
        return None;
    };
    let Expression::Identifier(object) = member.object.get_inner_expression() else {
        return None;
    };
    let span = lines.locate(call.span.start, call.span.end);
    let (kind, target, factory) = match (object.name.as_str(), member.property.name.as_str()) {
        ("vi" | "jest", "mock" | "doMock") => (
            MockKind::Module,
            string_literal(call.arguments.first()),
            call.arguments.len() > 1,
        ),
        ("vi" | "jest", "spyOn") => (MockKind::Function, spy_target(source, call), false),
        ("vi" | "jest", "fn") => (MockKind::Function, None, false),
        ("vi" | "jest", "useFakeTimers") => (MockKind::Timer, None, false),
        ("server" | "worker", "use") => (MockKind::Network, None, false),
        _ => return None,
    };
    Some(Mock {
        target,
        resolved: None,
        reason: None,
        kind,
        factory,
        span,
    })
}

/// How many comment lines directly above a definition [`comments_beside`] reads: the
/// window the `dead-skip` adjudicator reads (`COMMENT_LINES_ABOVE`).
const COMMENT_LINES_ABOVE: usize = 3;

/// The comment text written beside a definition call, for a skip's issue reference: up to
/// [`COMMENT_LINES_ABOVE`] comment lines (`//`, `/*` or ` *`, trimmed) directly above the
/// call's first line, in source order, stopping at the first blank or code line; then the
/// trailing `//` comment on the line the title ends on, after the title. A comment inside the
/// callback is not read.
pub fn comments_beside(source: &str, call: &CallExpression<'_>) -> Vec<String> {
    let start = (call.span.start as usize).min(source.len());
    let line_start = source[..start].rfind('\n').map_or(0, |i| i + 1);
    let mut comments: Vec<String> = source[..line_start]
        .lines()
        .rev()
        .map(str::trim)
        .take_while(|line| is_comment_line(line))
        .take(COMMENT_LINES_ABOVE)
        .map(str::to_owned)
        .collect();
    comments.reverse();
    if let Some(title) = call.arguments.first() {
        let title_end = (title.span().end as usize).min(source.len());
        let rest = &source[title_end..];
        let line = &rest[..rest.find('\n').unwrap_or(rest.len())];
        if let Some(at) = line.find("//") {
            comments.push(line[at..].trim().to_owned());
        }
    }
    comments
}

/// Whether a trimmed line is a comment line: a `//` line, a block-comment opener, or a
/// block-comment continuation or closer (a leading `*`).
fn is_comment_line(line: &str) -> bool {
    line.starts_with("//") || line.starts_with("/*") || line.starts_with('*')
}

/// `vi.spyOn(obj, "method")` as `obj.method`, or just the object's text when the method
/// name is not a literal.
fn spy_target(source: &str, call: &CallExpression<'_>) -> Option<String> {
    let object = first_argument_text(source, call)?;
    match call.arguments.get(1).and_then(Argument::as_expression) {
        Some(Expression::StringLiteral(method)) => Some(match method.value.as_str() {
            Some(method) => format!("{object}.{method}"),
            None => object,
        }),
        _ => Some(object),
    }
}

/// The exact source text of a call's first argument, for an assertion's subject or a
/// spy's object.
fn first_argument_text(source: &str, call: &CallExpression<'_>) -> Option<String> {
    let argument = call.arguments.first()?;
    let span = argument.span();
    source
        .get(span.start as usize..span.end as usize)
        .map(str::to_owned)
}

impl BodyCollector<'_> {
    /// The assertion `call` is, if it is one: an `expect`/`expectTypeOf` chain or an
    /// `assert*` call. The inner `expect(subject)` call is not itself matched (its callee
    /// is a plain identifier), so a chain counts once, at its outermost matcher.
    ///
    /// `carries_value` is the [`ValueLess`] taxonomy's answer, never the argument count: a
    /// bare `assert(x)` and `assert.ok(x)` are value-less, every other `assert.*` carries a
    /// value, and an `expect` chain is classified from its matcher, its negation and
    /// whether its subject is a `queryBy*` call (see [`subject_is_query`]). The query fact
    /// and the matcher's first argument are recorded on the row too, so a rule can read
    /// the absence idiom and a literal `0` count without re-parsing the source.
    fn assertion_of(&self, call: &CallExpression<'_>) -> Option<Assertion> {
        match call.callee.get_inner_expression() {
            Expression::Identifier(id) if id.name == "assert" => Some(Assertion {
                matcher: None,
                argument: None,
                subject: self.first_argument_text(call),
                subject_is_query: false,
                subject_is_call: false,
                negated: false,
                carries_value: ValueLess::classify(None, false, false).is_none(),
                span: self.span(call.span),
            }),
            Expression::StaticMemberExpression(member) => {
                if let Expression::Identifier(object) = member.object.get_inner_expression() {
                    if object.name == "assert" {
                        let matcher = member.property.name.as_str();
                        return Some(Assertion {
                            matcher: Some(matcher.to_owned()),
                            argument: self.argument_text(call, 1),
                            subject: self.first_argument_text(call),
                            subject_is_query: false,
                            subject_is_call: false,
                            negated: false,
                            carries_value: ValueLess::classify(Some(matcher), false, false)
                                .is_none(),
                            span: self.span(call.span),
                        });
                    }
                }
                self.expect_chain(call, member.property.name.as_str(), &member.object)
            }
            _ => None,
        }
    }

    /// Follow a matcher call's object chain down to its `expect(subject)` root, noting a
    /// `.not` on the way. Returns the assertion when the root is `expect`/`expectTypeOf`
    /// or one of the chain-opening statics — `expect.soft(x)`, `expect.poll(fn)`,
    /// `expect.element(locator)` — which are the same assertion with a different
    /// failure or retry policy. `expect.assertions(n)` and `expect.extend(…)` configure
    /// `expect` and are not chains, so they are not counted.
    fn expect_chain(
        &self,
        call: &CallExpression<'_>,
        matcher: &str,
        object: &Expression<'_>,
    ) -> Option<Assertion> {
        let mut negated = false;
        let mut node = object;
        loop {
            match node.get_inner_expression() {
                Expression::StaticMemberExpression(member) => {
                    // Toggle, not set: `expect(x).not.not.y()` is a double negation and
                    // reads as positive, the way the runner evaluates it.
                    negated ^= member.property.name == "not";
                    node = &member.object;
                }
                Expression::CallExpression(root) => {
                    if !is_expect_root(&root.callee) {
                        return None;
                    }
                    let subject_is_query = subject_is_query(root);
                    return Some(Assertion {
                        matcher: Some(matcher.to_owned()),
                        argument: self.argument_text(call, 0),
                        subject: self.first_argument_text(root),
                        subject_is_query,
                        subject_is_call: subject_is_element_read(root, &self.calls),
                        negated,
                        carries_value: ValueLess::classify(
                            Some(matcher),
                            negated,
                            subject_is_query,
                        )
                        .is_none(),
                        span: self.span(call.span),
                    });
                }
                _ => return None,
            }
        }
    }

    /// The exact source text of a call's first argument, for an assertion's subject.
    fn first_argument_text(&self, call: &CallExpression<'_>) -> Option<String> {
        first_argument_text(self.source, call)
    }

    /// The exact source text of the call's argument at `index`, for an assertion's matcher
    /// argument: index 0 of a matcher call (`toBe(1)`), index 1 of an `assert.*` call
    /// (`assert.equal(x, y)`), whose first argument is the subject.
    fn argument_text(&self, call: &CallExpression<'_>, index: usize) -> Option<String> {
        let span = call.arguments.get(index)?.span();
        self.source
            .get(span.start as usize..span.end as usize)
            .map(str::to_owned)
    }

    fn span(&self, span: oxc_span::Span) -> Span {
        self.lines.locate(span.start, span.end)
    }
}

/// Whether the subject of an `expect(...)` root is a non-throwing Testing Library query:
/// its first argument, after any `await`, parentheses or TS cast, is a call whose callee
/// is a bare `queryBy*` / `queryAllBy*` identifier or a member ending in one, on any
/// receiver (`screen`, `within(row)`, a render result). `queryBy*` returns `null` for a
/// missing element, so `toBeNull()` on it is the absence idiom the [`ValueLess`] table
/// treats as value-less. A query passed through another call (`f(screen.queryByText(…))`),
/// a member read off the result (`…?.textContent`), a throwing `getBy*` / `findBy*`, or
/// the bare function reference is not the idiom.
fn subject_is_query(root: &CallExpression<'_>) -> bool {
    let Some(subject) = root.arguments.first().and_then(Argument::as_expression) else {
        return false;
    };
    let mut node = subject.get_inner_expression();
    if let Expression::AwaitExpression(awaited) = node {
        node = awaited.argument.get_inner_expression();
    }
    let Expression::CallExpression(call) = node else {
        return false;
    };
    match call.callee.get_inner_expression() {
        Expression::Identifier(id) => is_non_throwing_query(&id.name),
        Expression::StaticMemberExpression(member) => is_non_throwing_query(&member.property.name),
        _ => false,
    }
}

/// Whether the subject is a potentially missing element: a call, a nullable DOM child
/// property, or a local bound to either. Persistent nodes (`container`, `document.body`)
/// are left out. The model retains the historical `subject_is_call` field name.
fn subject_is_element_read(root: &CallExpression<'_>, calls: &HashSet<String>) -> bool {
    let Some(subject) = root.arguments.first().and_then(Argument::as_expression) else {
        return false;
    };
    is_element_read(subject)
        || matches!(unawaited(subject), Expression::Identifier(id) if calls.contains(id.name.as_str()))
}

/// A call result (including an optional chain) or a nullable child property. An optional
/// call yields `undefined` on an empty render, which jest-dom rejects just like `null`.
fn is_element_read(expression: &Expression<'_>) -> bool {
    match unawaited(expression) {
        Expression::CallExpression(_) => true,
        Expression::StaticMemberExpression(member) => is_nullable_child(&member.property.name),
        Expression::ChainExpression(chain) => match &chain.expression {
            ChainElement::CallExpression(_) => true,
            ChainElement::StaticMemberExpression(member) => {
                is_nullable_child(&member.property.name)
            }
            _ => false,
        },
        _ => false,
    }
}

/// Child reads that can be null even though their container survives an empty render.
fn is_nullable_child(name: &str) -> bool {
    matches!(
        name,
        "firstChild" | "firstElementChild" | "lastChild" | "lastElementChild"
    )
}

/// An expression with any parentheses, TS cast and one `await` peeled off.
fn unawaited<'e, 'a>(expression: &'e Expression<'a>) -> &'e Expression<'a> {
    let node = expression.get_inner_expression();
    if let Expression::AwaitExpression(awaited) = node {
        return awaited.argument.get_inner_expression();
    }
    node
}

/// The names a callback's `const`/`let` declarations bind to an element read: a call,
/// an optional call chain or a nullable DOM child property. Matched by name within the
/// callback, never by scope.
#[derive(Default)]
struct CallBindings {
    names: HashSet<String>,
}

impl<'a> Visit<'a> for CallBindings {
    fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
        if let (Some(init), BindingPattern::BindingIdentifier(binding)) =
            (&declarator.init, &declarator.id)
        {
            if is_element_read(init) {
                self.names.insert(binding.name.as_str().to_owned());
            }
        }
        walk::walk_variable_declarator(self, declarator);
    }
}

/// Whether a method name is a *non-throwing* Testing Library query: `queryBy<Kind>` or
/// `queryAllBy<Kind>` with a capitalised kind, the families [`is_throwing_query`] leaves
/// out.
fn is_non_throwing_query(name: &str) -> bool {
    ["queryBy", "queryAllBy"].iter().any(|prefix| {
        name.strip_prefix(prefix)
            .and_then(|rest| rest.chars().next())
            .is_some_and(char::is_uppercase)
    })
}

/// Whether a callee opens an assertion chain: `expect`, `expectTypeOf`, or one of the
/// statics that open the same chain under another policy (`expect.soft`, `expect.poll`,
/// `expect.element`).
fn is_expect_root(callee: &Expression<'_>) -> bool {
    match callee.get_inner_expression() {
        Expression::Identifier(id) => id.name == "expect" || id.name == "expectTypeOf",
        Expression::StaticMemberExpression(member) => {
            matches!(member.property.name.as_str(), "soft" | "poll" | "element")
                && matches!(
                    member.object.get_inner_expression(),
                    Expression::Identifier(object) if object.name == "expect"
                )
        }
        _ => false,
    }
}

/// The value of a string-literal first argument (a mocked specifier); `None` when it is
/// anything else, so the row records "could not decide" rather than a guess.
fn string_literal(argument: Option<&Argument<'_>>) -> Option<String> {
    match argument.and_then(Argument::as_expression) {
        Some(Expression::StringLiteral(literal)) => literal.value.as_str().map(str::to_owned),
        _ => None,
    }
}

// --------------------------------------------------------------------- body hash

/// First pass over a callback: every locally bound name mapped to the order it was
/// declared in, so a reference to it can be written by position rather than by name.
#[derive(Default)]
struct BindingIndex {
    order: HashMap<String, u32>,
}

impl<'a> Visit<'a> for BindingIndex {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        if let AstKind::BindingIdentifier(id) = kind {
            if !self.order.contains_key(id.name.as_str()) {
                let next = u32::try_from(self.order.len()).unwrap_or(u32::MAX);
                self.order.insert(id.name.as_str().to_owned(), next);
            }
        }
    }
}

/// Second pass: append a byte per node kind and per kept literal, writing a locally bound
/// identifier as its declaration index and a free one (a global, an import) by name.
struct HashWalker<'m> {
    bindings: &'m HashMap<String, u32>,
    bytes: Vec<u8>,
}

impl HashWalker<'_> {
    /// Write an identifier: its binding index when it is local, its name when it is free.
    fn identifier(&mut self, name: &str) {
        if let Some(index) = self.bindings.get(name) {
            self.bytes.push(b'#');
            self.bytes.extend_from_slice(&index.to_le_bytes());
        } else {
            self.bytes.push(b'@');
            self.framed(name.as_bytes());
        }
    }

    /// Write a literal's kept text verbatim, so changing one literal changes the hash.
    fn literal(&mut self, text: &str) {
        self.bytes.push(b'=');
        self.framed(text.as_bytes());
    }

    /// Write an interface's `extends` target as the expression oxc 0.110 parsed it into.
    fn heritage_name(&mut self, name: &TSTypeName<'_>) {
        match name {
            TSTypeName::QualifiedName(qualified) => {
                self.bytes
                    .push(kind_code::code(AstType::StaticMemberExpression));
                self.heritage_name(&qualified.left);
                self.bytes.push(kind_code::code(AstType::IdentifierName));
                self.literal(qualified.right.name.as_str());
            }
            TSTypeName::IdentifierReference(_) | TSTypeName::ThisExpression(_) => {
                walk::walk_ts_type_name(self, name);
            }
        }
    }

    /// Write the two identifier-name children oxc 0.110 gave `import.meta`/`new.target`.
    fn meta_property(&mut self, meta: &str, property: &str) {
        for name in [meta, property] {
            self.bytes.push(kind_code::code(AstType::IdentifierName));
            self.literal(name);
        }
    }

    /// Append length-prefixed bytes, so variable-length content is self-delimiting: no
    /// run of names or literals can be re-cut into a different tree that hashes equal.
    fn framed(&mut self, content: &[u8]) {
        let length = u32::try_from(content.len()).unwrap_or(u32::MAX);
        self.bytes.extend_from_slice(&length.to_le_bytes());
        self.bytes.extend_from_slice(content);
    }
}

/// The text JSX renders for the text between two tags: each line trimmed of the
/// indentation around it (the first keeps its start, the last its end), blank lines dropped,
/// the rest joined by one space. Layout alone renders as the empty string.
fn jsx_text(raw: &str) -> String {
    let lines: Vec<&str> = raw.split('\n').collect();
    let last = lines.len() - 1;
    lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            let line = if index == 0 { *line } else { line.trim_start() };
            if index == last {
                line
            } else {
                line.trim_end()
            }
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

impl<'a> Visit<'a> for HashWalker<'_> {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        // The layout between tags renders as nothing, so it hashes as nothing.
        let jsx_text = match kind {
            AstKind::JSXText(text) => Some(jsx_text(text.value.as_str())),
            _ => None,
        };
        if jsx_text.as_deref() == Some("") {
            return;
        }
        self.bytes.push(kind_code::code(kind.ty()));
        match kind {
            AstKind::IdentifierReference(id) => self.identifier(id.name.as_str()),
            AstKind::BindingIdentifier(id) => self.identifier(id.name.as_str()),
            // A member/property name is meaning, not a rename target: keep it verbatim.
            AstKind::IdentifierName(id) => self.literal(id.name.as_str()),
            // So is a JSX attribute name, a lowercase element name and each part of a member
            // or namespaced name; the text between tags is a literal.
            AstKind::JSXIdentifier(id) => self.literal(id.name.as_str()),
            AstKind::JSXText(_) => self.literal(jsx_text.as_deref().unwrap_or_default()),
            // The value's exact bytes: its UTF-8 text, or the lossless WTF-8 of a lone surrogate.
            AstKind::StringLiteral(literal) => {
                self.bytes.push(b'=');
                self.framed(literal.value.as_bytes());
            }
            AstKind::NumericLiteral(literal) => {
                self.bytes.push(b'=');
                self.bytes
                    .extend_from_slice(&literal.value.to_bits().to_le_bytes());
            }
            AstKind::BooleanLiteral(literal) => self.bytes.push(u8::from(literal.value)),
            AstKind::BigIntLiteral(literal) => self.literal(literal.value.as_str()),
            AstKind::RegExpLiteral(literal) => {
                self.literal(literal.raw.as_deref().unwrap_or_default());
            }
            AstKind::TemplateLiteral(template) => {
                for quasi in &template.quasis {
                    self.literal(quasi.value.raw.as_str());
                }
            }
            // oxc 0.110 read both as a meta property with two identifier-name children.
            AstKind::ImportMeta(_) => self.meta_property("import", "meta"),
            AstKind::NewTarget(_) => self.meta_property("new", "target"),
            _ => {}
        }
    }

    // oxc 0.110 wrapped a concise arrow body (`() => x`) in a function body holding one
    // expression statement; later releases hold the expression bare. Write the wrapper's
    // two codes so the hash stays the one recorded under 0.110.
    // oxc 0.110 held an interface's `extends` target as an expression, so `ns.Base` was a
    // static member expression; later releases hold a type name.
    fn visit_ts_interface_heritage(&mut self, heritage: &TSInterfaceHeritage<'a>) {
        self.bytes
            .push(kind_code::code(AstType::TSInterfaceHeritage));
        self.heritage_name(&heritage.type_name);
        if let Some(arguments) = &heritage.type_arguments {
            self.visit_ts_type_parameter_instantiation(arguments);
        }
    }

    fn visit_arrow_function_body(&mut self, body: &ArrowFunctionBody<'a>) {
        if let Some(expression) = body.as_expression() {
            self.bytes.push(kind_code::code(AstType::FunctionBody));
            self.bytes
                .push(kind_code::code(AstType::ExpressionStatement));
            self.visit_expression(expression);
        } else {
            walk::walk_arrow_function_body(self, body);
        }
    }
}
