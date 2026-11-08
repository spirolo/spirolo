//! Commands spawned by a checked-in Node wrapper, read without evaluating JavaScript.
//!
//! Follow literal spawn arguments and cwd paths built from top-level string bindings and
//! path.join/resolve. Dynamic commands and directories contribute no invented invocation.

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::ast::{BindingPattern, CallExpression, Expression, Statement};
use oxc_ast_visit::{walk, Visit};
use oxc_parser::Parser;
use oxc_span::SourceType;

use crate::config_reader::prop;
use crate::paths::join_relative;

pub(super) fn spawned(source: &str, path: &str, directory: &str) -> Vec<(String, String)> {
    let allocator = Allocator::default();
    let parsed = Parser::new(
        &allocator,
        source,
        SourceType::from_path(path).unwrap_or_default(),
    )
    .parse();
    if parsed.fatal_error || parsed.diagnostics.has_errors() {
        return Vec::new();
    }
    let mut values = BTreeMap::from([
        ("__dirname".to_owned(), parent(path).to_owned()),
        ("__filename".to_owned(), path.to_owned()),
    ]);
    for statement in &parsed.program.body {
        let Statement::VariableDeclaration(declaration) = statement else {
            continue;
        };
        for declarator in &declaration.declarations {
            if let (BindingPattern::BindingIdentifier(id), Some(init)) =
                (&declarator.id, &declarator.init)
            {
                if let Some(value) = value(init, &values) {
                    values.insert(id.name.to_string(), value);
                }
            }
        }
    }
    let mut reader = Spawns {
        values,
        directory,
        commands: Vec::new(),
    };
    reader.visit_program(&parsed.program);
    reader.commands
}

fn value(expression: &Expression<'_>, values: &BTreeMap<String, String>) -> Option<String> {
    match expression.get_inner_expression() {
        Expression::StringLiteral(literal) => literal.value.as_str().map(str::to_owned),
        Expression::Identifier(id) => values.get(id.name.as_str()).cloned(),
        // `import.meta.dirname` and `import.meta.filename` (Node 20.11+).
        Expression::StaticMemberExpression(member) => {
            if !matches!(
                member.object.get_inner_expression(),
                Expression::ImportMeta(_)
            ) {
                return None;
            }
            match member.property.name.as_str() {
                "dirname" => values.get("__dirname").cloned(),
                "filename" => values.get("__filename").cloned(),
                _ => None,
            }
        }
        Expression::CallExpression(call) => {
            let name = match call.callee.get_inner_expression() {
                Expression::Identifier(id) => id.name.as_str(),
                Expression::StaticMemberExpression(member) => member.property.name.as_str(),
                _ => return None,
            };
            match name {
                // `fileURLToPath(import.meta.url)` is the script's own path.
                "fileURLToPath" => {
                    let url = call.arguments.first()?.as_expression()?;
                    let Expression::StaticMemberExpression(member) = url.get_inner_expression()
                    else {
                        return None;
                    };
                    return (matches!(
                        member.object.get_inner_expression(),
                        Expression::ImportMeta(_)
                    ) && member.property.name == "url")
                        .then(|| values.get("__filename").cloned())
                        .flatten();
                }
                "dirname" => {
                    let path = value(call.arguments.first()?.as_expression()?, values)?;
                    return Some(parent(&path).to_owned());
                }
                "join" | "resolve" => {}
                _ => return None,
            }
            let mut args = call.arguments.iter();
            let mut path = value(args.next()?.as_expression()?, values)?;
            for argument in args {
                path = join_relative(&path, &value(argument.as_expression()?, values)?)?;
            }
            Some(path)
        }
        _ => None,
    }
}

struct Spawns<'a> {
    values: BTreeMap<String, String>,
    directory: &'a str,
    commands: Vec<(String, String)>,
}

impl<'a> Visit<'a> for Spawns<'_> {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if matches!(&call.callee, Expression::Identifier(id) if id.name == "spawn" || id.name == "spawnSync")
        {
            if let Some(command) = self.command(call) {
                self.commands.push(command);
            }
        }
        walk::walk_call_expression(self, call);
    }
}

impl Spawns<'_> {
    fn command(&self, call: &CallExpression<'_>) -> Option<(String, String)> {
        let program = value(call.arguments.first()?.as_expression()?, &self.values)?;
        let Expression::ArrayExpression(args) = call.arguments.get(1)?.as_expression()? else {
            return None;
        };
        let mut words = vec![program];
        for arg in &args.elements {
            // An output-path template after the script name does not change which script
            // runs. Stop at the first dynamic argument rather than guessing its value.
            let Some(word) = arg.as_expression().and_then(|arg| value(arg, &self.values)) else {
                break;
            };
            words.push(word);
        }
        if words.len() < 2 {
            return None;
        }
        let cwd = match call.arguments.get(2).and_then(|arg| arg.as_expression()) {
            Some(Expression::ObjectExpression(options)) => prop(options, "cwd").map_or_else(
                || Some(self.directory.to_owned()),
                |cwd| value(cwd, &self.values),
            )?,
            None => self.directory.to_owned(),
            _ => return None,
        };
        Some((cwd, words.join(" ")))
    }
}

/// The directory part of a repository-relative path.
fn parent(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(dir, _)| dir)
}
