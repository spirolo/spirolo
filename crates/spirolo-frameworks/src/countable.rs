//! Reading the size of a table a test iterates, and tracking the loops a definition sits in.
//!
//! `it.each(rows)`, `describe.each(cases)`, `for (const x of xs) it(…)` and
//! `xs.forEach((x) => it(…))` all define one case per element of some table. When the
//! table is a literal in the call, its size is read here. When it is a binding, the
//! recogniser remembers the `const` declarations it has walked past — a block-scoped
//! `const rows = [...]` inside a `describe` is invisible to any whole-project pass — and
//! otherwise records the binding's name as the table's [`Each::source`], for the model to
//! count once every module is parsed.

use std::collections::HashMap;

use oxc_ast::ast::{
    Argument, ArrayExpressionElement, BindingPattern, CallExpression, Expression,
    ObjectPropertyKind, VariableDeclarator,
};
use spirolo_model::Each;

/// A table whose size could not be read and that names no binding.
pub const EMPTY_EACH: Each = Each {
    cases: None,
    params: None,
    source: None,
};

/// The table an expression denotes: a literal's size, or the binding it refers to.
///
/// TypeScript wrappers (`as const`, `satisfies`) and parentheses are looked through. An
/// array supplies one case per element and its row arity as params; an object supplies one
/// case per property (`Object.keys` / `values` / `entries` of it enumerate the same);
/// `.map(…)` keeps a length. A spread inside a literal, a `.filter`, or any other call
/// leaves the size open.
pub fn table_of(expr: &Expression<'_>) -> Each {
    match expr.get_inner_expression() {
        Expression::ArrayExpression(array) => {
            if array
                .elements
                .iter()
                .any(|element| matches!(element, ArrayExpressionElement::SpreadElement(_)))
            {
                return EMPTY_EACH;
            }
            let params = match array.elements.first() {
                Some(ArrayExpressionElement::Elision(_)) | None => None,
                Some(element) => {
                    match element
                        .as_expression()
                        .map(Expression::get_inner_expression)
                    {
                        Some(Expression::ArrayExpression(row)) => {
                            u32::try_from(row.elements.len()).ok()
                        }
                        _ => Some(1),
                    }
                }
            };
            Each {
                cases: u32::try_from(array.elements.len()).ok(),
                params,
                source: None,
            }
        }
        Expression::ObjectExpression(object) => {
            if object
                .properties
                .iter()
                .any(|property| matches!(property, ObjectPropertyKind::SpreadProperty(_)))
            {
                return EMPTY_EACH;
            }
            Each {
                cases: u32::try_from(object.properties.len()).ok(),
                params: Some(1),
                source: None,
            }
        }
        Expression::Identifier(_) | Expression::StaticMemberExpression(_) => Each {
            cases: None,
            params: None,
            source: dotted_path(expr),
        },
        Expression::CallExpression(call) => enumerated_by(call).map_or(EMPTY_EACH, table_of),
        _ => EMPTY_EACH,
    }
}

/// The table a call enumerates without changing its size: the argument of
/// `Object.keys(x)` / `Object.values(x)` / `Object.entries(x)`, or the receiver of
/// `x.map(…)`.
fn enumerated_by<'a>(call: &'a CallExpression<'a>) -> Option<&'a Expression<'a>> {
    let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
        return None;
    };
    let method = member.property.name.as_str();
    let receiver = member.object.get_inner_expression();
    let on_object = matches!(receiver, Expression::Identifier(id) if id.name == "Object");
    if on_object && matches!(method, "keys" | "values" | "entries") {
        return call.arguments.first().and_then(Argument::as_expression);
    }
    if method == "map" {
        return Some(&member.object);
    }
    None
}

/// The table a `.each` / `.for` argument denotes.
pub fn each_from_argument(argument: &Argument<'_>) -> Each {
    argument.as_expression().map_or(EMPTY_EACH, table_of)
}

/// `a.b.c` as `"a.b.c"`, when every link is a static member on an identifier root.
fn dotted_path(expr: &Expression<'_>) -> Option<String> {
    match expr.get_inner_expression() {
        Expression::Identifier(id) => Some(id.name.as_str().to_owned()),
        Expression::StaticMemberExpression(member) => {
            let head = dotted_path(&member.object)?;
            Some(format!("{head}.{}", member.property.name))
        }
        _ => None,
    }
}

/// The `const` bindings a walk has passed and the loops it is inside, so a table read
/// from a local name is counted on the spot and a definition inside a loop knows how many
/// times it runs.
#[derive(Default)]
pub struct Tables {
    /// Local `const NAME = <table>` bindings, by name, with their size or source.
    bindings: HashMap<String, Each>,
    /// The enclosing `for…of` / `forEach` loops, outermost first.
    loops: Vec<Each>,
    /// Where the loop stack stood when each enclosing suite was entered: a row reports
    /// only the loops opened since its suite, so a loop around a `describe` multiplies the
    /// suite once rather than the suite and every case in it.
    marks: Vec<usize>,
}

impl Tables {
    /// Remember `const NAME = …` when its initializer is a table or names one.
    pub fn register(&mut self, declarator: &VariableDeclarator<'_>) {
        let (Some(init), BindingPattern::BindingIdentifier(binding)) =
            (&declarator.init, &declarator.id)
        else {
            return;
        };
        let table = self.localise(table_of(init));
        if table.cases.is_some() || table.source.is_some() {
            self.bindings
                .insert(binding.name.as_str().to_owned(), table);
        }
    }

    /// Replace a table that names a binding this walk has seen with that binding's table.
    pub fn localise(&self, table: Each) -> Each {
        if table.cases.is_some() {
            return table;
        }
        let Some(source) = table.source.as_deref() else {
            return table;
        };
        // `rows.phrases` resolves through `rows` only when the binding itself was a
        // literal; a nested path into a local object is left for the model's fold.
        self.bindings.get(source).cloned().unwrap_or(table)
    }

    /// Enter a loop over `iterable` (`for (const x of iterable)`, `iterable.forEach(…)`).
    pub fn enter_loop(&mut self, iterable: &Expression<'_>) {
        let table = self.localise(table_of(iterable));
        self.loops.push(table);
    }

    /// Leave the innermost loop.
    pub fn leave_loop(&mut self) {
        self.loops.pop();
    }

    /// The loops opened since the enclosing suite, outermost first, for a row defined here.
    pub fn loops(&self) -> Vec<Each> {
        let since = self.marks.last().copied().unwrap_or(0);
        self.loops[since..].to_vec()
    }

    /// Enter a suite: the loops open now belong to it, not to the rows inside it.
    pub fn enter_suite(&mut self) {
        self.marks.push(self.loops.len());
    }

    /// Leave the innermost suite.
    pub fn leave_suite(&mut self) {
        self.marks.pop();
    }
}

/// The receiver of an `xs.forEach(fn)` call, when the call is one.
pub fn for_each_receiver<'a>(call: &'a CallExpression<'a>) -> Option<&'a Expression<'a>> {
    let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
        return None;
    };
    if member.property.name != "forEach" {
        return None;
    }
    let has_callback = call.arguments.iter().any(|argument| {
        matches!(
            argument,
            Argument::ArrowFunctionExpression(_) | Argument::FunctionExpression(_)
        )
    });
    has_callback.then_some(&member.object)
}
