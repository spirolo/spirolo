//! The imported calls of a test file: functions it imports from a module other than the
//! runner's and calls where the runner collects tests.
//!
//! `formTests(Form)` at the top of a file, or `describeWithPointerEvent('…', (p) => { test(…) })`,
//! registers tests whose names the file does not spell: the imported function chooses them
//! (a shared suite, a `describe.each` with a computed title). `stale-snapshot` reads these
//! calls so an entry such a call may write is never reported stale. A call is counted at the
//! top level and inside the callbacks of the calls there (a `describe`, a `forEach`, an
//! imported wrapper), never inside a test or hook body, where an imported call (`render`)
//! runs a test rather than registering one.

use std::collections::HashSet;

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Argument, ArrowFunctionBody, CallExpression, Expression, ImportOrExportKind, Statement,
};
use oxc_parser::Parser;
use oxc_span::SourceType;

use crate::vitest::{base_info, claimed_framework};
use spirolo_model::TestKind;

/// The names of the imported functions the test file at `path` calls where tests are
/// registered, sorted and deduplicated. Empty for a file that does not parse.
#[must_use]
pub fn imported_calls(path: &str, source: &str) -> Vec<String> {
    let Ok(source_type) = SourceType::from_path(path) else {
        return Vec::new();
    };
    // React runners transform JSX in `.js` as well, as the model's own parse assumes.
    let source_type = source_type.with_jsx(source_type.is_jsx() || source_type.is_javascript());
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();
    let imported = imported_names(&parsed.program.body);
    let mut calls = Vec::new();
    collect(&parsed.program.body, &imported, &mut calls);
    calls.sort();
    calls.dedup();
    calls
}

/// The local names a file's value imports bind, leaving out the runner's own packages.
fn imported_names<'a>(statements: &'a [Statement<'a>]) -> HashSet<&'a str> {
    let mut names = HashSet::new();
    for statement in statements {
        let Statement::ImportDeclaration(import) = statement else {
            continue;
        };
        if import.import_kind == ImportOrExportKind::Type
            || import
                .source
                .value
                .as_str()
                .and_then(claimed_framework)
                .is_some()
        {
            continue;
        }
        for specifier in import.specifiers.iter().flatten() {
            names.insert(specifier.local().name.as_str());
        }
    }
    names
}

/// Add to `calls` the imported functions `statements` call, descending into the callbacks
/// of every call except a test's, a hook's or a benchmark's.
fn collect(statements: &[Statement<'_>], imported: &HashSet<&str>, calls: &mut Vec<String>) {
    for statement in statements {
        if let Statement::ExpressionStatement(statement) = statement {
            collect_call(&statement.expression, imported, calls);
        }
    }
}

/// [`collect`] for one expression evaluated where tests are registered.
fn collect_call(expression: &Expression<'_>, imported: &HashSet<&str>, calls: &mut Vec<String>) {
    let Expression::CallExpression(call) = expression.without_parentheses() else {
        return;
    };
    let root = callee_root(call);
    if root
        .and_then(base_info)
        .is_some_and(|info| info.kind != TestKind::Suite)
    {
        return;
    }
    if let Some(root) = root.filter(|root| imported.contains(root)) {
        calls.push(root.to_owned());
    }
    for argument in &call.arguments {
        match argument {
            Argument::ArrowFunctionExpression(arrow) => match arrow.body.as_expression() {
                Some(body) => collect_call(body, imported, calls),
                None => {
                    if let ArrowFunctionBody::FunctionBody(body) = &arrow.body {
                        collect(&body.statements, imported, calls);
                    }
                }
            },
            Argument::FunctionExpression(function) => {
                if let Some(body) = &function.body {
                    collect(&body.statements, imported, calls);
                }
            }
            _ => {}
        }
    }
}

/// The identifier a call's callee chain roots in: `f` of `f(…)`, `describe.each(…)(…)` or
/// `a.b(…)`; `None` when it roots in anything else (`[1].forEach(…)`).
fn callee_root<'a>(call: &'a CallExpression<'a>) -> Option<&'a str> {
    let mut callee = call.callee.without_parentheses();
    loop {
        match callee {
            Expression::Identifier(identifier) => return Some(identifier.name.as_str()),
            Expression::StaticMemberExpression(member) => callee = &member.object,
            Expression::ComputedMemberExpression(member) => callee = &member.object,
            Expression::CallExpression(inner) => callee = &inner.callee,
            _ => return None,
        }
    }
}
