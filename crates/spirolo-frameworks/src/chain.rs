//! Resolving a `root.modifier.modifier(…)` callee into a base name and its modifiers.
//!
//! Every family whose vocabulary is reached through chained members — Vitest/Jest's
//! `it.skip.each`, Playwright's `test.describe.serial` — reads a call the same way: peel
//! the static member chain down to the identifier it roots in, keeping each `.name` link
//! and the little a modifier carries (a condition's span, a `.each` table's size). The
//! recognisers then decide what those names mean; this module only takes the callee apart.

use oxc_ast::ast::{Argument, Expression, TemplateLiteral};
use oxc_span::GetSpan;
use spirolo_model::Each;

pub use crate::countable::{each_from_argument, EMPTY_EACH};

/// A resolved `root.modifier.modifier(…)` callee: the base name and its modifiers in
/// source order. Everything it holds is a short-lived borrow of the AST or is `Copy`,
/// so it lives no longer than the call it was read from.
pub struct Chain<'e> {
    /// The identifier the chain roots in, for example `test` or `it`.
    pub root: &'e str,
    /// The `.name` links, outermost call last, in source order.
    pub modifiers: Vec<Modifier<'e>>,
}

/// One `.name` / `.name(args)` / `` .name`tpl` `` link in a chain. The arguments a
/// `.skipIf` / `.each` carries are reduced here to what a row needs — a condition span
/// and a table size — so nothing borrows the argument nodes.
pub struct Modifier<'e> {
    /// The property name of the link, for example `skip` or `each`.
    pub name: &'e str,
    /// Location of the property name.
    pub span: oxc_span::Span,
    /// The first argument's span, for lifting a `.skipIf` / `.runIf` condition's text.
    pub condition_span: Option<oxc_span::Span>,
    /// The table's size, for `.each` / `.for`.
    pub table: Option<Each>,
}

/// Resolve a callee expression into its base name and modifiers, or `None` when it is
/// not rooted in a plain identifier reached only through static member access.
pub fn resolve_chain<'e>(expr: &'e Expression<'e>) -> Option<Chain<'e>> {
    match expr.get_inner_expression() {
        Expression::Identifier(id) => Some(Chain {
            root: id.name.as_str(),
            modifiers: Vec::new(),
        }),
        Expression::StaticMemberExpression(member) => {
            let mut chain = resolve_chain(&member.object)?;
            chain.modifiers.push(Modifier {
                name: member.property.name.as_str(),
                span: member.property.span,
                condition_span: None,
                table: None,
            });
            Some(chain)
        }
        Expression::CallExpression(call) => {
            let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression()
            else {
                return None;
            };
            let mut chain = resolve_chain(&member.object)?;
            let first = call.arguments.first();
            chain.modifiers.push(Modifier {
                name: member.property.name.as_str(),
                span: member.property.span,
                condition_span: first.map(GetSpan::span),
                table: Some(first.map_or(EMPTY_EACH, each_from_argument)),
            });
            Some(chain)
        }
        Expression::TaggedTemplateExpression(tagged) => {
            let Expression::StaticMemberExpression(member) = tagged.tag.get_inner_expression()
            else {
                return None;
            };
            let mut chain = resolve_chain(&member.object)?;
            chain.modifiers.push(Modifier {
                name: member.property.name.as_str(),
                span: member.property.span,
                condition_span: None,
                table: Some(each_from_template(&tagged.quasi)),
            });
            Some(chain)
        }
        _ => None,
    }
}

/// A template table (`` it.each`a | b\n1 | 2` ``): rows are the data lines, params the
/// header columns. Interpolations are counted structurally — the newlines that separate
/// rows live in the literal text, not in the `${…}` values.
pub fn each_from_template(template: &TemplateLiteral<'_>) -> Each {
    let mut text = String::new();
    for (index, quasi) in template.quasis.iter().enumerate() {
        text.push_str(quasi.value.raw.as_str());
        if index < template.expressions.len() {
            text.push(' ');
        }
    }
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let Some(header) = lines.first() else {
        return EMPTY_EACH;
    };
    // Count non-empty cells so the leading/trailing-pipe idiom (`| a | b |`) reads as two
    // columns, not four, and agrees with how `each_from_array` counts an array row.
    let params = header
        .split('|')
        .map(str::trim)
        .filter(|cell| !cell.is_empty())
        .count();
    Each {
        cases: u32::try_from(lines.len() - 1).ok(),
        params: u32::try_from(params).ok(),
        source: None,
    }
}

/// The title argument, when it is a static string or a substitution-free template.
pub fn title_of(argument: &Argument<'_>) -> Option<String> {
    match argument {
        Argument::StringLiteral(literal) => literal.value.as_str().map(str::to_owned),
        Argument::TemplateLiteral(template) if template.expressions.is_empty() => template
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
