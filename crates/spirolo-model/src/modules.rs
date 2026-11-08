//! Pulling a file's imports and exports out of its AST.
//!
//! This runs inside [`extract_file`](crate::extract::extract_file), while the arena is
//! still alive: a [`Collector`] walks the program once and returns owned rows. Nothing
//! borrowed from the parse escapes. Specifiers are recorded here but not resolved —
//! resolution needs the whole file set and happens later, in [`crate::resolve`].

use std::collections::HashSet;

use oxc_ast::ast::{
    Argument, ArrowFunctionExpression, BindingPattern, CallExpression, CatchClause,
    ConditionalExpression, Declaration, DoWhileStatement, ExportAllDeclaration, ExportDeclaration,
    ExportDefaultDeclaration, ExportFromDeclaration, ExportNamedDeclaration, ExportSpecifier,
    Expression, ForInStatement, ForOfStatement, ForStatement, Function, IdentifierReference,
    IfStatement, ImportDeclaration, ImportExpression, ImportOrExportKind, LogicalExpression,
    ModuleDeclaration, Program, SwitchCase, VariableDeclarator, WhileStatement,
};
use oxc_ast_visit::{walk, Visit};
use oxc_span::GetSpan;
use oxc_syntax::scope::ScopeFlags;

use crate::{
    Export, ExportKind, IdentRef, Import, ImportKind, LineIndex, Load, Mock, MockKind, Span, Spy,
};

/// The imports, exports and module mocks one module declares, in source order.
#[derive(Default)]
pub struct ModuleFacts {
    /// Every specifier the module depends on, unresolved for now.
    pub imports: Vec<Import>,
    /// Every name the module makes available.
    pub exports: Vec<Export>,
    /// Every module mock the file declares (`vi.mock("x")`, `jest.mock("x")`).
    pub mocks: Vec<Mock>,
    /// Every identifier reference (a read) the file makes, in span order.
    pub references: Vec<IdentRef>,
    /// Every `vi.fn()` / `vi.spyOn(…)` result bound to a plain name, in span order.
    pub spies: Vec<Spy>,
    /// Every runner-registry load (`jest.requireActual("x")` and kin), in span order.
    pub loads: Vec<Load>,
    /// The module's cyclomatic complexity: see [`crate::Module::complexity`].
    pub complexity: u32,
}

/// Walk `program` and collect its imports and exports.
///
/// `source` is only used to turn byte offsets into line and column, so the rows carry
/// the same coordinates the parser's diagnostics do.
pub fn collect(source: &str, program: &oxc_ast::ast::Program<'_>) -> ModuleFacts {
    let mut collector = Collector {
        source,
        lines: LineIndex::new(source),
        imports: Vec::new(),
        exports: Vec::new(),
        mocks: Vec::new(),
        references: Vec::new(),
        spies: Vec::new(),
        loads: Vec::new(),
        complexity: 1,
        type_only: type_only_names(program),
    };
    collector.visit_program(program);
    collector.imports.sort_by_key(|i| i.span);
    collector.exports.sort_by_key(|e| e.span);
    collector.mocks.sort_by_key(|m| m.span);
    collector.references.sort_by_key(|r| r.span);
    collector.spies.sort_by_key(|s| s.span);
    collector.loads.sort_by_key(|l| l.span);
    ModuleFacts {
        imports: collector.imports,
        exports: collector.exports,
        mocks: collector.mocks,
        references: collector.references,
        spies: collector.spies,
        loads: collector.loads,
        complexity: collector.complexity,
    }
}

struct Collector<'s> {
    source: &'s str,
    /// The file's line starts, so every span is located without rescanning the source.
    lines: LineIndex,
    imports: Vec<Import>,
    exports: Vec<Export>,
    mocks: Vec<Mock>,
    references: Vec<IdentRef>,
    spies: Vec<Spy>,
    loads: Vec<Load>,
    /// The module top level's one, plus one per function and per decision point seen.
    complexity: u32,
    /// The top-level names declared only as a type or interface, so a local
    /// `export { Name }` of one is a type export.
    type_only: HashSet<String>,
}

/// The names `program`'s top level declares as a type alias or interface and not also as a
/// value (a `const`, function, class or enum of the same name).
fn type_only_names(program: &Program<'_>) -> HashSet<String> {
    let mut types = HashSet::new();
    let mut values = HashSet::new();
    for statement in &program.body {
        let declaration = match statement.as_module_declaration() {
            Some(ModuleDeclaration::ExportDeclaration(export)) => &export.declaration,
            Some(_) => continue,
            None => match statement.as_declaration() {
                Some(declaration) => declaration,
                None => continue,
            },
        };
        match declaration {
            Declaration::TSTypeAliasDeclaration(alias) => {
                types.insert(alias.id.name.to_string());
            }
            Declaration::TSInterfaceDeclaration(interface) => {
                types.insert(interface.id.name.to_string());
            }
            Declaration::VariableDeclaration(variable) => {
                for declarator in &variable.declarations {
                    for id in declarator.id.get_binding_identifiers() {
                        values.insert(id.name.to_string());
                    }
                }
            }
            Declaration::FunctionDeclaration(function) => {
                values.extend(function.id.iter().map(|id| id.name.to_string()));
            }
            Declaration::ClassDeclaration(class) => {
                values.extend(class.id.iter().map(|id| id.name.to_string()));
            }
            Declaration::TSEnumDeclaration(enumeration) => {
                values.insert(enumeration.id.name.to_string());
            }
            _ => {}
        }
    }
    types.retain(|name| !values.contains(name));
    types
}

impl Collector<'_> {
    fn span(&self, span: oxc_span::Span) -> Span {
        self.lines.locate(span.start, span.end)
    }

    fn push_import(&mut self, specifier: &str, kind: ImportKind, span: oxc_span::Span) {
        self.imports.push(Import {
            specifier: specifier.to_owned(),
            resolved: None,
            reason: None,
            kind,
            span: self.span(span),
        });
    }

    /// An `import(x)` / `require(x)` whose specifier is an expression, not a literal: the
    /// module it loads is decided at run time. Recorded with the expression as written and
    /// the reason `computed`, which the resolver leaves alone.
    fn push_computed(&mut self, span: oxc_span::Span, kind: ImportKind) {
        let written = &self.source[span.start as usize..span.end as usize];
        self.imports.push(Import {
            specifier: written.to_owned(),
            resolved: None,
            reason: Some("computed".to_owned()),
            kind,
            span: self.span(span),
        });
    }

    /// A name the module binds itself.
    fn push_export(&mut self, name: &str, kind: ExportKind, span: oxc_span::Span) {
        self.push_export_from(name, kind, None, span);
    }

    /// A name the module makes available, from `origin` when it is a re-export.
    fn push_export_from(
        &mut self,
        name: &str,
        kind: ExportKind,
        origin: Option<&str>,
        span: oxc_span::Span,
    ) {
        self.exports.push(Export {
            name: name.to_owned(),
            kind,
            origin: origin.map(str::to_owned),
            resolved: None,
            span: self.span(span),
        });
    }

    /// The names an `export const/function/class/type` declaration binds.
    fn export_declaration(&mut self, declaration: &Declaration<'_>) {
        match declaration {
            Declaration::VariableDeclaration(variable) => {
                for declarator in &variable.declarations {
                    for id in declarator.id.get_binding_identifiers() {
                        self.push_export(id.name.as_str(), ExportKind::Named, id.span);
                    }
                }
            }
            Declaration::FunctionDeclaration(function) => {
                if let Some(id) = &function.id {
                    self.push_export(id.name.as_str(), ExportKind::Named, id.span);
                }
            }
            Declaration::ClassDeclaration(class) => {
                if let Some(id) = &class.id {
                    self.push_export(id.name.as_str(), ExportKind::Named, id.span);
                }
            }
            Declaration::TSTypeAliasDeclaration(alias) => {
                self.push_export(alias.id.name.as_str(), ExportKind::Type, alias.id.span);
            }
            Declaration::TSInterfaceDeclaration(interface) => {
                self.push_export(
                    interface.id.name.as_str(),
                    ExportKind::Type,
                    interface.id.span,
                );
            }
            Declaration::TSEnumDeclaration(enumeration) => {
                self.push_export(
                    enumeration.id.name.as_str(),
                    ExportKind::Named,
                    enumeration.id.span,
                );
            }
            // `module`, `global`, `import x =` declarations do not add a plain export name.
            _ => {}
        }
    }

    /// The names an `export { … }` list binds; `origin` is the module a re-export
    /// points at, `None` for a local list.
    fn export_specifiers(
        &mut self,
        specifiers: &[ExportSpecifier<'_>],
        export_kind: ImportOrExportKind,
        origin: Option<&str>,
    ) {
        let whole_is_type = export_kind.is_type();
        for specifier in specifiers {
            let local_type =
                origin.is_none() && self.type_only.contains(specifier.local.name().as_str());
            let kind = if whole_is_type || specifier.export_kind.is_type() || local_type {
                ExportKind::Type
            } else {
                ExportKind::Named
            };
            self.push_export_from(
                specifier.exported.name().as_str(),
                kind,
                origin,
                specifier.span,
            );
        }
    }
}

impl<'a> Visit<'a> for Collector<'_> {
    fn visit_import_declaration(&mut self, it: &ImportDeclaration<'a>) {
        let kind = if it.import_kind.is_type() {
            ImportKind::Type
        } else {
            ImportKind::Static
        };
        if let Some(specifier) = it.source.value.as_str() {
            self.push_import(specifier, kind, it.source.span);
        }
    }

    fn visit_export_declaration(&mut self, it: &ExportDeclaration<'a>) {
        self.export_declaration(&it.declaration);
        // The exported declaration's body is code like any other: its `import()`,
        // `require()`, mocks and references count.
        walk::walk_export_declaration(self, it);
    }

    fn visit_export_named_declaration(&mut self, it: &ExportNamedDeclaration<'a>) {
        self.export_specifiers(&it.specifiers, it.export_kind, None);
        walk::walk_export_named_declaration(self, it);
    }

    fn visit_export_from_declaration(&mut self, it: &ExportFromDeclaration<'a>) {
        // `export { x } from "y"` and `export type { x } from "y"` re-export from another
        // module; the specifier is the origin those names point at.
        let source = &it.source;
        if let Some(specifier) = source.value.as_str() {
            self.push_import(specifier, ImportKind::ReExport, source.span);
            self.export_specifiers(&it.specifiers, it.export_kind, Some(specifier));
        }
        walk::walk_export_from_declaration(self, it);
    }

    fn visit_export_all_declaration(&mut self, it: &ExportAllDeclaration<'a>) {
        let Some(specifier) = it.source.value.as_str() else {
            return;
        };
        self.push_import(specifier, ImportKind::ReExport, it.source.span);
        let kind = if it.export_kind.is_type() {
            ExportKind::Type
        } else {
            ExportKind::Namespace
        };
        // `export * as ns from "y"` names the namespace; a bare `export *` has no name.
        let name = it
            .exported
            .as_ref()
            .map_or_else(|| "*".to_owned(), |e| e.name().to_string());
        self.push_export_from(&name, kind, Some(specifier), it.span);
    }

    fn visit_export_default_declaration(&mut self, it: &ExportDefaultDeclaration<'a>) {
        self.push_export("default", ExportKind::Default, it.span);
        walk::walk_export_default_declaration(self, it);
    }

    fn visit_import_expression(&mut self, it: &ImportExpression<'a>) {
        match &it.source {
            // A lone surrogate names no module: the literal is computed, like an expression.
            Expression::StringLiteral(literal) => match literal.value.as_str() {
                Some(specifier) => self.push_import(specifier, ImportKind::Dynamic, literal.span),
                None => self.push_computed(literal.span, ImportKind::Dynamic),
            },
            source => self.push_computed(source.span(), ImportKind::Dynamic),
        }
        walk::walk_import_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Expression::Identifier(callee) = &it.callee {
            if callee.name == "require" {
                match it.arguments.first() {
                    Some(Argument::StringLiteral(literal)) => match literal.value.as_str() {
                        Some(specifier) => {
                            self.push_import(specifier, ImportKind::Require, literal.span);
                        }
                        None => self.push_computed(literal.span, ImportKind::Require),
                    },
                    Some(argument) => self.push_computed(argument.span(), ImportKind::Require),
                    None => {}
                }
            }
        }
        if let Some(mock) = self.module_mock(it) {
            self.mocks.push(mock);
        }
        if let Some(load) = self.registry_load(it) {
            self.loads.push(load);
        }
        walk::walk_call_expression(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        // An overload signature or a `declare function` has no body: nothing runs.
        if it.body.is_some() {
            self.complexity += 1;
        }
        walk::walk_function(self, it, flags);
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.complexity += 1;
        walk::walk_arrow_function_expression(self, it);
    }

    // The decision points PRD 0005 counts: `if`, `?:`, `&&`, `||`, `??`, `case`, `for`,
    // `for…in/of`, `while`, `do` and `catch`, one each. `else` and `default` are not.

    fn visit_if_statement(&mut self, it: &IfStatement<'a>) {
        self.complexity += 1;
        walk::walk_if_statement(self, it);
    }

    fn visit_conditional_expression(&mut self, it: &ConditionalExpression<'a>) {
        self.complexity += 1;
        walk::walk_conditional_expression(self, it);
    }

    fn visit_logical_expression(&mut self, it: &LogicalExpression<'a>) {
        self.complexity += 1;
        walk::walk_logical_expression(self, it);
    }

    fn visit_switch_case(&mut self, it: &SwitchCase<'a>) {
        if it.test.is_some() {
            self.complexity += 1;
        }
        walk::walk_switch_case(self, it);
    }

    fn visit_for_statement(&mut self, it: &ForStatement<'a>) {
        self.complexity += 1;
        walk::walk_for_statement(self, it);
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        self.complexity += 1;
        walk::walk_for_in_statement(self, it);
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        self.complexity += 1;
        walk::walk_for_of_statement(self, it);
    }

    fn visit_while_statement(&mut self, it: &WhileStatement<'a>) {
        self.complexity += 1;
        walk::walk_while_statement(self, it);
    }

    fn visit_do_while_statement(&mut self, it: &DoWhileStatement<'a>) {
        self.complexity += 1;
        walk::walk_do_while_statement(self, it);
    }

    fn visit_catch_clause(&mut self, it: &CatchClause<'a>) {
        self.complexity += 1;
        walk::walk_catch_clause(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.references.push(IdentRef {
            name: it.name.as_str().to_owned(),
            span: self.span(it.span),
        });
        walk::walk_identifier_reference(self, it);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let Some(spy) = self.spy_binding(it) {
            self.spies.push(spy);
        }
        walk::walk_variable_declarator(self, it);
    }
}

impl Collector<'_> {
    /// A `vi.mock("x")` / `jest.mock("x")` (and the `doMock` variants) call as a module
    /// [`Mock`], or `None` for anything else. The target is the first argument's string
    /// literal; a mock with a non-literal target (`vi.mock(resolvePath())`) is recorded with
    /// no target rather than guessed at. A factory second argument is noted, matching the
    /// in-body collector's `factory` flag so the two views agree.
    fn module_mock(&self, call: &CallExpression<'_>) -> Option<Mock> {
        let Expression::StaticMemberExpression(member) = &call.callee else {
            return None;
        };
        let Expression::Identifier(object) = member.object.get_inner_expression() else {
            return None;
        };
        if !matches!(object.name.as_str(), "vi" | "jest")
            || !matches!(member.property.name.as_str(), "mock" | "doMock")
        {
            return None;
        }
        let target = match call.arguments.first() {
            Some(Argument::StringLiteral(literal)) => literal.value.as_str().map(str::to_owned),
            _ => None,
        };
        Some(Mock {
            target,
            resolved: None,
            reason: None,
            kind: MockKind::Module,
            factory: call.arguments.len() > 1,
            span: self.span(call.span),
        })
    }

    /// A `jest.requireActual("x")` / `jest.requireMock("x")` / `vi.importActual("x")` /
    /// `vi.importMock("x")` call as a [`Load`], or `None` for anything else, including a
    /// non-literal specifier.
    fn registry_load(&self, call: &CallExpression<'_>) -> Option<Load> {
        let Expression::StaticMemberExpression(member) = &call.callee else {
            return None;
        };
        let Expression::Identifier(object) = member.object.get_inner_expression() else {
            return None;
        };
        let registry = match object.name.as_str() {
            "jest" => ["requireActual", "requireMock"],
            "vi" => ["importActual", "importMock"],
            _ => return None,
        };
        if !registry.contains(&member.property.name.as_str()) {
            return None;
        }
        let Some(Argument::StringLiteral(literal)) = call.arguments.first() else {
            return None;
        };
        Some(Load {
            specifier: literal.value.as_str()?.to_owned(),
            resolved: None,
            reason: None,
            span: self.span(call.span),
        })
    }

    /// A `const X = vi.fn()` / `jest.fn()` / `vi.spyOn(…)` / `jest.spyOn(…)` declarator as a
    /// [`Spy`] bound to `X`, or `None` for anything else. Only a plain binding identifier is
    /// taken — a destructuring pattern binds no single spy name the unread kind could search.
    fn spy_binding(&self, declarator: &VariableDeclarator<'_>) -> Option<Spy> {
        let BindingPattern::BindingIdentifier(binding) = &declarator.id else {
            return None;
        };
        let Expression::CallExpression(call) = declarator.init.as_ref()? else {
            return None;
        };
        let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
            return None;
        };
        let Expression::Identifier(object) = member.object.get_inner_expression() else {
            return None;
        };
        if !matches!(object.name.as_str(), "vi" | "jest")
            || !matches!(member.property.name.as_str(), "fn" | "spyOn")
        {
            return None;
        }
        Some(Spy {
            binding: binding.name.as_str().to_owned(),
            span: self.span(call.span),
        })
    }
}

#[cfg(test)]
mod tests {
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;

    use crate::MockKind;

    /// The module mocks collected from a snippet, as `(target, factory)` pairs in span
    /// order — the file-scoped view `data-layer-mock` and `init` read.
    fn mocks(source: &str) -> Vec<(Option<String>, bool)> {
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
        super::collect(source, &parsed.program)
            .mocks
            .into_iter()
            .map(|mock| (mock.target, mock.factory))
            .collect()
    }

    /// The imports collected from a snippet, as `(specifier, kind, reason)` triples.
    fn imports(source: &str) -> Vec<(String, crate::ImportKind, Option<String>)> {
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
        super::collect(source, &parsed.program)
            .imports
            .into_iter()
            .map(|import| (import.specifier, import.kind, import.reason))
            .collect()
    }

    /// The exports collected from a snippet, as `(name, kind)` pairs.
    fn exports(source: &str) -> Vec<(String, crate::ExportKind)> {
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
        super::collect(source, &parsed.program)
            .exports
            .into_iter()
            .map(|export| (export.name, export.kind))
            .collect()
    }

    #[test]
    fn a_local_interface_or_type_in_an_export_list_is_a_type_export() {
        // The #457 review's scratch module: `Opts` is a type, so a test can never reference
        // it as a value, and `untested-hotspot` must not count it as an unreferenced one.
        let source = "interface Opts { a: number }\n\
                      type Alias = string\n\
                      export function run(x) { if (x > 1) { return x ? 1 : 2; } return 0; }\n\
                      export { Opts, Alias as Renamed };\n";
        assert_eq!(
            exports(source),
            [
                ("run".to_owned(), crate::ExportKind::Named),
                ("Opts".to_owned(), crate::ExportKind::Type),
                ("Renamed".to_owned(), crate::ExportKind::Type),
            ]
        );
    }

    #[test]
    fn a_local_name_that_is_both_a_type_and_a_value_is_a_named_export() {
        let source = "interface Point { x: number }\n\
                      const Point = (x: number): Point => ({ x })\n\
                      export { Point };\n";
        assert_eq!(
            exports(source),
            [("Point".to_owned(), crate::ExportKind::Named)]
        );
    }

    #[test]
    fn a_computed_dynamic_import_or_require_is_an_import_whose_target_is_computed() {
        // A kit's cli.test.ts that re-imports its CLI through a variable:
        // so the module it loads is decided at run time. It is still an import: recorded as
        // written, with the reason `computed`, never dropped as though nothing were loaded.
        let source =
            "const importCli = (specifier: string): Promise<unknown> => import(specifier)\n\
                      const load = (name: string) => require(name)\n";
        assert_eq!(
            imports(source),
            [
                (
                    "specifier".to_owned(),
                    crate::ImportKind::Dynamic,
                    Some("computed".to_owned())
                ),
                (
                    "name".to_owned(),
                    crate::ImportKind::Require,
                    Some("computed".to_owned())
                ),
            ]
        );
    }

    #[test]
    fn a_specifier_with_a_lone_surrogate_is_skipped_and_a_well_formed_one_is_recorded() {
        // `"\ud800"` holds a lone UTF-16 surrogate: no module path spells it and oxc gives
        // no `&str` for it, so it is not a static specifier. A static import or re-export of
        // one is skipped; a `require()` of one takes the non-literal path (computed, recorded
        // as written). A well-formed specifier beside them is recorded as before.
        let source = r#"import x from "\ud800"
export { a } from "\udc00"
import y from "./y"
const z = require("\ud800")
"#;
        assert_eq!(
            imports(source),
            [
                ("./y".to_owned(), crate::ImportKind::Static, None),
                (
                    r#""\ud800""#.to_owned(),
                    crate::ImportKind::Require,
                    Some("computed".to_owned())
                ),
            ]
        );
    }

    #[test]
    fn a_dynamic_import_inside_an_exported_function_is_an_import() {
        // An API route.ts that loads its manager lazily inside the exported
        // handler; the export's body is walked like any other code.
        let source = "export async function POST(req: NextRequest) {\n\
                      \x20 const { SimpleKeywordManager } = await import('@acme/keyword-manager')\n\
                      }\n\
                      export default async function handler() {\n\
                      \x20 return require('./legacy')\n\
                      }\n";
        assert_eq!(
            imports(source),
            [
                (
                    "@acme/keyword-manager".to_owned(),
                    crate::ImportKind::Dynamic,
                    None
                ),
                ("./legacy".to_owned(), crate::ImportKind::Require, None),
            ]
        );
    }

    #[test]
    fn a_top_level_vi_mock_is_a_module_mock_on_the_file() {
        let source = "import { vi } from 'vitest'\n\
                      vi.mock('@/supabaseClient')\n\
                      test('x', () => {})\n";
        assert_eq!(
            mocks(source),
            [(Some("@/supabaseClient".to_owned()), false)]
        );
    }

    #[test]
    fn jest_mock_and_do_mock_and_a_factory_are_all_module_mocks() {
        let source = "jest.mock('axios')\n\
                      vi.doMock('../client', () => ({ default: {} }))\n";
        assert_eq!(
            mocks(source),
            [
                (Some("axios".to_owned()), false),
                (Some("../client".to_owned()), true),
            ]
        );
    }

    #[test]
    fn the_runner_registry_loads_are_recorded_and_a_non_literal_one_is_not() {
        let source = "jest.requireActual('./a')\n\
                      jest.requireMock('./b')\n\
                      await vi.importActual<typeof import('./c')>('./c')\n\
                      vi.importMock('./d')\n\
                      jest.requireActual(name)\n\
                      vi.requireActual('./e')\n";
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let loads: Vec<String> = super::collect(source, &parsed.program)
            .loads
            .into_iter()
            .map(|load| load.specifier)
            .collect();
        assert_eq!(loads, ["./a", "./b", "./c", "./d"]);
    }

    #[test]
    fn a_non_literal_target_is_recorded_with_no_target_not_guessed() {
        let source = "vi.mock(resolvePath())\n";
        assert_eq!(mocks(source), [(None, false)]);
    }

    #[test]
    fn a_module_mock_carries_the_module_kind() {
        let allocator = Allocator::default();
        let source = "vi.mock('x')\n";
        let parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let collected = super::collect(source, &parsed.program);
        assert_eq!(collected.mocks.len(), 1);
        assert_eq!(collected.mocks[0].kind, MockKind::Module);
    }

    #[test]
    fn a_spy_or_fn_is_not_a_module_mock() {
        assert!(mocks("vi.spyOn(obj, 'method')\nvi.fn()\n").is_empty());
    }

    /// The identifier references (reads) collected from a snippet, as names in span order.
    fn references(source: &str) -> Vec<String> {
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
        super::collect(source, &parsed.program)
            .references
            .into_iter()
            .map(|r| r.name)
            .collect()
    }

    #[test]
    fn a_read_of_a_binding_is_an_identifier_reference() {
        let source = "const spy = fn()\nspy.mockReturnValue(1)\n";
        assert!(references(source).contains(&"spy".to_owned()));
    }

    /// The spy bindings collected from a snippet, as binding names in span order.
    fn spies(source: &str) -> Vec<String> {
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
        super::collect(source, &parsed.program)
            .spies
            .into_iter()
            .map(|s| s.binding)
            .collect()
    }

    #[test]
    fn a_vi_fn_bound_to_a_name_is_a_spy_binding() {
        assert_eq!(spies("const handler = vi.fn()\n"), ["handler".to_owned()]);
    }

    #[test]
    fn a_bound_spy_on_and_jest_fn_are_spy_bindings() {
        assert_eq!(
            spies("const read = vi.spyOn(fs, 'readFile')\nconst m = jest.fn()\n"),
            ["read".to_owned(), "m".to_owned()]
        );
    }

    #[test]
    fn an_unbound_fn_or_spy_on_is_not_a_spy_binding() {
        assert!(spies("vi.fn()\nvi.spyOn(obj, 'method')\n").is_empty());
    }

    /// A snippet's cyclomatic complexity, as PRD 0005 defines it for `untested-hotspot`.
    fn complexity(source: &str) -> u32 {
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
        super::collect(source, &parsed.program).complexity
    }

    #[test]
    fn a_module_with_no_function_and_no_decision_is_one() {
        assert_eq!(complexity("export const answer = 42\n"), 1);
    }

    #[test]
    fn every_function_arrow_and_method_counts_one_besides_the_top_level() {
        let source = "export function a() {}\n\
                      export const b = () => 1\n\
                      export const c = function () {}\n\
                      export class D { e() {} get f() { return 1 } }\n";
        assert_eq!(complexity(source), 1 + 5);
    }

    #[test]
    fn an_overload_signature_or_a_declared_function_has_no_body_and_counts_nothing() {
        let source = "export function parse(input: string): number\n\
                      export function parse(input: number): number\n\
                      export function parse(input: unknown) { return Number(input) }\n\
                      declare function external(): void\n";
        assert_eq!(complexity(source), 1 + 1);
    }

    #[test]
    fn each_decision_point_counts_one() {
        // One function around each, so every case is 1 (top level) + 1 (function) + 1.
        let cases = [
            ("if", "function f(a) { if (a) { return 1 } return 2 }"),
            ("?:", "const f = (a) => (a ? 1 : 2)"),
            ("&&", "const f = (a, b) => a && b"),
            ("||", "const f = (a, b) => a || b"),
            ("??", "const f = (a, b) => a ?? b"),
            ("case", "function f(a) { switch (a) { case 1: return 1 } }"),
            (
                "for",
                "function f(xs) { for (let i = 0; i < xs.length; i++) {} }",
            ),
            ("for…in", "function f(o) { for (const k in o) {} }"),
            ("for…of", "function f(xs) { for (const x of xs) {} }"),
            ("while", "function f(a) { while (a) { a-- } }"),
            ("do", "function f(a) { do { a-- } while (a) }"),
            (
                "catch",
                "function f(g) { try { g() } catch (e) { return e } }",
            ),
        ];
        for (point, source) in cases {
            assert_eq!(complexity(source), 3, "{point}: {source}");
        }
    }

    #[test]
    fn a_default_clause_and_an_else_are_not_decision_points() {
        let source = "function f(a) {\n\
                        if (a) { return 1 } else { return 2 }\n\
                        switch (a) { case 1: return 1; default: return 0 }\n\
                      }\n";
        // Top level, the function, the `if` and the one `case`.
        assert_eq!(complexity(source), 4);
    }
}
