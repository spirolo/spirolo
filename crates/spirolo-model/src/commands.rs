//! Static config selection in the root package's Node/yargs test wrapper.
//!
//! Some packages reserve their default Jest config for a guard, then pass --config
//! from a Node wrapper. Read literal argument arrays and their conditional pushes,
//! using literal yargs defaults and pure predicate helpers. Unknown expressions
//! leave the existing config in charge. No JavaScript is executed.

use std::collections::HashMap;

use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, Function, Program, Statement};
use oxc_parser::Parser;
use oxc_span::SourceType;

use crate::config_reader::{export_parts, object_value, prop, read_parts};
use crate::manifest::Manifest;
use crate::{Gate, GateTool, Model};

/// The package directory and the explicit config its primary test command selects.
pub type Selection = (String, String);

/// Discover wrapper-selected Jest configs among the files the model already walked.
pub fn discover(model: &mut Model, manifests: &[Manifest]) -> Vec<Selection> {
    let mut selected = Vec::new();
    for manifest in manifests {
        // process.cwd() in an inherited config is the project root for this run.
        // Nested package wrappers need their own invocation working-directory model.
        if !manifest.dir.is_empty() || manifest.runner != Some(GateTool::Jest) {
            continue;
        }
        let Some(script) = manifest.named_scripts.get("test").map(String::as_str) else {
            continue;
        };
        let mut words = script.split_whitespace();
        if words.next() != Some("node") {
            continue;
        }
        let Some(wrapper) = words.next() else {
            continue;
        };
        // Arguments and shell syntax require more than the wrapper's defaults.
        if words.next().is_some() {
            continue;
        }
        let Some(wrapper) = crate::paths::join_relative(&manifest.dir, wrapper) else {
            continue;
        };
        let Some(source) = wrapper_source(model, &wrapper) else {
            continue;
        };
        let Some(config) = selected_config(&wrapper, &source) else {
            continue;
        };
        let Some(config) = crate::paths::join_relative(&manifest.dir, &config) else {
            continue;
        };
        let Some(source) = walked_source(model, &config) else {
            continue;
        };
        let allocator = Allocator::default();
        let Some(source_type) = SourceType::from_path(&config).ok() else {
            continue;
        };
        let parsed = Parser::new(&allocator, &source, source_type).parse();
        if parsed.fatal_error || parsed.diagnostics.has_errors() {
            continue;
        }
        if !model
            .gates
            .iter()
            .any(|gate| gate.source == config && gate.tool == GateTool::Jest)
        {
            let mut gate = Gate::blank(&config, GateTool::Jest);
            read_parts(GateTool::Jest, &export_parts(&parsed.program), &mut gate);
            model.gates.push(gate);
        }
        selected.push((manifest.dir.clone(), config));
    }
    selected
}

fn wrapper_source(model: &Model, path: &str) -> Option<String> {
    let module = model.modules.iter().find(|module| module.path == path)?;
    let imports = |name| module.imports.iter().any(|import| import.specifier == name);
    if !imports("yargs") || !(imports("child_process") || imports("node:child_process")) {
        return None;
    }
    walked_source(model, path)
}

fn walked_source(model: &Model, path: &str) -> Option<String> {
    model
        .modules
        .iter()
        .any(|module| module.path == path)
        .then_some(())?;
    std::fs::read_to_string(model.root.join(path)).ok()
}

fn selected_config(path: &str, source: &str) -> Option<String> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::from_path(path).ok()?).parse();
    if parsed.fatal_error || parsed.diagnostics.has_errors() {
        return None;
    }
    let reader = Reader::new(&parsed.program)?;
    for function in reader.argument_builders(&parsed.program) {
        let body = function.body.as_ref()?;
        for statement in &body.statements {
            let Statement::VariableDeclaration(declaration) = statement else {
                continue;
            };
            for binding in &declaration.declarations {
                let Some(Expression::ArrayExpression(array)) = binding.init.as_ref() else {
                    continue;
                };
                let values: Vec<_> = array
                    .elements
                    .iter()
                    .map(|element| element.as_expression().and_then(string_value))
                    .collect::<Option<_>>()?;
                if values.get(1).map(String::as_str) != Some("--config")
                    || !values
                        .first()
                        .is_some_and(|runner| crate::scope::invokes(runner, "jest"))
                {
                    continue;
                }
                if let Some(config) = values.get(2) {
                    return Some(config.clone());
                }
                let name = binding.id.get_binding_identifier()?.name.as_str();
                for statement in &body.statements {
                    if let Some(config) = reader.pushed_config(statement, name).ok()? {
                        return Some(config);
                    }
                }
            }
        }
    }
    None
}

struct Unknown;

#[derive(Clone, PartialEq)]
enum Literal {
    Bool(bool),
    String(String),
}

struct Reader<'a> {
    functions: HashMap<&'a str, &'a Function<'a>>,
    constants: HashMap<&'a str, &'a Expression<'a>>,
    argv: &'a str,
    defaults: HashMap<String, Literal>,
}

impl<'a> Reader<'a> {
    // Follow an actually called entry point to spawn('node', args), then follow
    // that local args binding to its builder. Unused config helpers are not runs.
    fn argument_builders(&self, program: &'a Program<'a>) -> Vec<&'a Function<'a>> {
        let mut builders = Vec::new();
        for statement in &program.body {
            let Statement::ExpressionStatement(statement) = statement else {
                continue;
            };
            let Expression::CallExpression(call) = statement.expression.get_inner_expression()
            else {
                continue;
            };
            let Expression::Identifier(entry) = call.callee.get_inner_expression() else {
                continue;
            };
            let Some(body) = self
                .functions
                .get(entry.name.as_str())
                .and_then(|function| function.body.as_ref())
            else {
                continue;
            };
            let bindings: HashMap<_, _> = body
                .statements
                .iter()
                .filter_map(|statement| {
                    if let Statement::VariableDeclaration(declaration) = statement {
                        Some(declaration)
                    } else {
                        None
                    }
                })
                .flat_map(|declaration| &declaration.declarations)
                .filter_map(|binding| {
                    Some((
                        binding.id.get_binding_identifier()?.name.as_str(),
                        binding.init.as_ref()?,
                    ))
                })
                .collect();
            for init in bindings.values() {
                let Expression::CallExpression(spawn) = init.get_inner_expression() else {
                    continue;
                };
                if !matches!(spawn.callee.get_inner_expression(), Expression::Identifier(id) if id.name == "spawn")
                    || spawn
                        .arguments
                        .first()
                        .and_then(|arg| arg.as_expression())
                        .and_then(string_value)
                        .as_deref()
                        != Some("node")
                {
                    continue;
                }
                let Some(Expression::Identifier(args)) =
                    spawn.arguments.get(1).and_then(|arg| arg.as_expression())
                else {
                    continue;
                };
                let Some(Expression::CallExpression(builder)) = bindings
                    .get(args.name.as_str())
                    .map(|init| init.get_inner_expression())
                else {
                    continue;
                };
                let Expression::Identifier(name) = builder.callee.get_inner_expression() else {
                    continue;
                };
                if builder.arguments.is_empty() {
                    if let Some(function) = self.functions.get(name.name.as_str()) {
                        builders.push(*function);
                    }
                }
            }
        }
        // Multiple processes require an invocation union, which this reader cannot
        // represent as one replacement for the package's default config.
        if builders.len() == 1 {
            builders
        } else {
            Vec::new()
        }
    }

    fn new(program: &'a Program<'a>) -> Option<Self> {
        let mut reader = Self {
            functions: HashMap::new(),
            constants: HashMap::new(),
            argv: "",
            defaults: HashMap::new(),
        };
        for statement in &program.body {
            match statement {
                Statement::FunctionDeclaration(function) => {
                    if let Some(id) = &function.id {
                        reader.functions.insert(id.name.as_str(), function);
                    }
                }
                Statement::VariableDeclaration(declaration) => {
                    for binding in &declaration.declarations {
                        let (Some(id), Some(init)) =
                            (binding.id.get_binding_identifier(), &binding.init)
                        else {
                            continue;
                        };
                        reader.constants.insert(id.name.as_str(), init);
                        if let Some(options) = options_object(init) {
                            reader.argv = id.name.as_str();
                            for property in &options.properties {
                                let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(property) =
                                    property
                                else {
                                    return None;
                                };
                                let name = property.key.static_name()?.into_cow_str()?.into_owned();
                                let option = object_value(&property.value)?;
                                let value = if let Some(default) = prop(option, "default") {
                                    literal(default)?
                                } else if prop(option, "type").and_then(string_value).as_deref()
                                    == Some("boolean")
                                {
                                    Literal::Bool(false)
                                } else {
                                    continue;
                                };
                                reader.defaults.insert(name, value);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        (!reader.argv.is_empty()).then_some(reader)
    }

    fn eval(&self, expression: &Expression<'a>, depth: usize) -> Option<Literal> {
        if depth > 16 {
            return None;
        }
        let expression = expression.get_inner_expression();
        if let Some(value) = literal(expression) {
            return Some(value);
        }
        match expression {
            Expression::Identifier(id) => {
                self.eval(self.constants.get(id.name.as_str())?, depth + 1)
            }
            Expression::StaticMemberExpression(member) => {
                if matches!(member.object.get_inner_expression(), Expression::Identifier(id) if id.name == self.argv)
                {
                    self.defaults.get(member.property.name.as_str()).cloned()
                } else {
                    None
                }
            }
            Expression::UnaryExpression(unary) if unary.operator.as_str() == "!" => {
                Some(Literal::Bool(!self.boolean(&unary.argument, depth + 1)?))
            }
            Expression::BinaryExpression(binary) => {
                let left = self.eval(&binary.left, depth + 1)?;
                let right = self.eval(&binary.right, depth + 1)?;
                match binary.operator.as_str() {
                    "===" => Some(Literal::Bool(left == right)),
                    "!==" => Some(Literal::Bool(left != right)),
                    _ => None,
                }
            }
            Expression::LogicalExpression(logical) => {
                let left = self.boolean(&logical.left, depth + 1)?;
                match logical.operator.as_str() {
                    "&&" if !left => Some(Literal::Bool(false)),
                    "||" if left => Some(Literal::Bool(true)),
                    "&&" | "||" => Some(Literal::Bool(self.boolean(&logical.right, depth + 1)?)),
                    _ => None,
                }
            }
            Expression::CallExpression(call) if call.arguments.is_empty() => {
                let Expression::Identifier(id) = call.callee.get_inner_expression() else {
                    return None;
                };
                let body = self.functions.get(id.name.as_str())?.body.as_ref()?;
                let [Statement::ReturnStatement(returned)] = body.statements.as_slice() else {
                    return None;
                };
                self.eval(returned.argument.as_ref()?, depth + 1)
            }
            _ => None,
        }
    }

    fn boolean(&self, expression: &Expression<'a>, depth: usize) -> Option<bool> {
        match self.eval(expression, depth)? {
            Literal::Bool(value) => Some(value),
            Literal::String(value) => Some(!value.is_empty()),
        }
    }

    // Unknown control flow must not choose a different config branch.
    fn pushed_config(
        &self,
        statement: &Statement<'a>,
        args: &str,
    ) -> Result<Option<String>, Unknown> {
        match statement {
            Statement::IfStatement(branch) => {
                if self.boolean(&branch.test, 0).ok_or(Unknown)? {
                    self.pushed_config(&branch.consequent, args)
                } else {
                    branch
                        .alternate
                        .as_ref()
                        .map_or(Ok(None), |other| self.pushed_config(other, args))
                }
            }
            Statement::BlockStatement(block) => {
                for statement in &block.body {
                    if let Some(config) = self.pushed_config(statement, args)? {
                        return Ok(Some(config));
                    }
                }
                Ok(None)
            }
            Statement::ExpressionStatement(statement) => {
                if let Expression::CallExpression(call) =
                    statement.expression.get_inner_expression()
                {
                    if let Expression::StaticMemberExpression(member) =
                        call.callee.get_inner_expression()
                    {
                        if member.property.name == "push"
                            && matches!(member.object.get_inner_expression(), Expression::Identifier(id) if id.name == args)
                        {
                            let Literal::String(config) = self
                                .eval(
                                    call.arguments
                                        .first()
                                        .and_then(|argument| argument.as_expression())
                                        .ok_or(Unknown)?,
                                    0,
                                )
                                .ok_or(Unknown)?
                            else {
                                return Err(Unknown);
                            };
                            return Ok(Some(config));
                        }
                    }
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }
}

fn literal(expression: &Expression<'_>) -> Option<Literal> {
    match expression.get_inner_expression() {
        Expression::BooleanLiteral(value) => Some(Literal::Bool(value.value)),
        Expression::StringLiteral(value) => value
            .value
            .as_str()
            .map(|value| Literal::String(value.to_owned())),
        _ => None,
    }
}

fn options_object<'a>(
    expression: &'a Expression<'a>,
) -> Option<&'a oxc_ast::ast::ObjectExpression<'a>> {
    match expression.get_inner_expression() {
        Expression::StaticMemberExpression(member) => options_object(&member.object),
        Expression::CallExpression(call) => {
            let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression()
            else {
                return None;
            };
            if member.property.name == "options" {
                object_value(call.arguments.first()?.as_expression()?)
            } else {
                options_object(&member.object)
            }
        }
        _ => None,
    }
}

fn string_value(expression: &Expression<'_>) -> Option<String> {
    match literal(expression)? {
        Literal::String(value) => Some(value),
        Literal::Bool(_) => None,
    }
}
