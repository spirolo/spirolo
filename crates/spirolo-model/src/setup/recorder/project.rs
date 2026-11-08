//! Reach recorder checks through statically resolved ES module bindings. Modules are parsed
//! on demand and cached per setup analysis; an unrelated import cannot install a guard.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use oxc_allocator::Allocator;
use oxc_ast_visit::Visit;
use oxc_parser::Parser;
use oxc_span::SourceType;

use super::{reads_console_mock, Binding, Exported, HookRoot, Ref, Replacement, Walk};
use crate::resolve::WorkspaceResolver;

#[derive(Clone, PartialEq, Eq, Hash)]
struct Target {
    module: usize,
    binding: Binding,
    path: Vec<String>,
    /// How many trailing path members come after a value copied at load: `const n =
    /// calls.length` makes `n` the number `calls.length` was, not the live recorder.
    tail: Option<usize>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct ParameterBinding {
    module: usize,
    declaration: u32,
    target: Target,
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct Frame {
    module: usize,
    key: u32,
    parameters: Vec<ParameterBinding>,
    /// Whether a failure in this function fails the hook: not caught, not deferred.
    fails: bool,
}

struct Module {
    file: PathBuf,
    walk: Walk,
}

struct Project<'a> {
    imports: Option<(&'a Path, &'a WorkspaceResolver)>,
    modules: Vec<Module>,
    files: HashMap<PathBuf, usize>,
}

pub(super) fn guard(walk: Walk, file: &Path, imports: Option<(&Path, &WorkspaceResolver)>) -> bool {
    let file = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
    let mut project = Project {
        imports,
        files: HashMap::from([(file.clone(), 0)]),
        modules: vec![Module { file, walk }],
    };
    project.guard()
}

impl Project<'_> {
    fn load(&mut self, from: usize, source: &str) -> Option<usize> {
        let (root, resolver) = self.imports?;
        let file = resolver
            .resolve_file(&self.modules[from].file, source)
            .ok()?
            .full_path();
        let file = file.canonicalize().ok()?;
        // Match the model's project boundary: installed packages and files outside the root
        // do not become local helpers merely because their source happens to be on disk.
        if !file.starts_with(root) || file.components().any(|c| c.as_os_str() == "node_modules") {
            return None;
        }
        if let Some(module) = self.files.get(&file) {
            return Some(*module);
        }
        let source = std::fs::read_to_string(&file).ok()?;
        let source_type = SourceType::from_path(&file).ok()?;
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, &source, source_type).parse();
        if !parsed.diagnostics.is_empty() {
            return None;
        }
        let mut walk = Walk::default();
        walk.visit_program(&parsed.program);
        let module = self.modules.len();
        self.files.insert(file.clone(), module);
        self.modules.push(Module { file, walk });
        Some(module)
    }

    fn exported(
        &mut self,
        module: usize,
        name: &str,
        seen: &mut HashSet<(usize, String)>,
    ) -> Option<Target> {
        if !seen.insert((module, name.to_owned())) {
            return None;
        }
        match self.modules[module].walk.exports.get(name)?.clone() {
            Exported::Local(binding, path) => Some(Target {
                module,
                binding,
                path,
                tail: None,
            }),
            Exported::From(source, name) => {
                let module = self.load(module, &source)?;
                self.exported(module, &name, seen)
            }
        }
    }

    fn target(
        &mut self,
        module: usize,
        binding: &Binding,
        path: &[String],
        parameters: &[ParameterBinding],
    ) -> Option<Target> {
        let mut target = Target {
            module,
            binding: binding.clone(),
            path: path.to_vec(),
            tail: None,
        };
        let mut seen = HashSet::new();
        loop {
            let (binding, path) = self.modules[target.module]
                .walk
                .unalias(&target.binding, &target.path);
            target.binding = binding;
            if path.len() > target.path.len() && target.tail.is_none() {
                target.tail = Some(target.path.len());
            }
            target.path = path;
            if !seen.insert((target.module, target.binding.clone())) {
                return None;
            }
            if let Binding::Decl(decl) = target.binding {
                if let Some(parameter) = parameters.iter().find(|parameter| {
                    parameter.module == target.module && parameter.declaration == decl
                }) {
                    // An argument is evaluated at the call, so only a copy made before it
                    // (`const n = calls.length; check(n)`) is a snapshot.
                    let (suffix, tail) = (target.path, target.tail);
                    target = parameter.target.clone();
                    target.tail = tail.or_else(|| target.tail.map(|t| t + suffix.len()));
                    target.path.extend(suffix);
                    continue;
                }
            }
            let Some(import) = self.modules[target.module]
                .walk
                .imports
                .get(&target.binding)
                .cloned()
            else {
                return Some(target);
            };
            // A module the walk cannot follow (outside the root, unresolved, CommonJS, a name
            // reached only through `export *`) ends at the import binding itself: a push and a
            // read through the same binding still meet there.
            let Some(module) = self.load(target.module, &import.source) else {
                return Some(target);
            };
            let name = match &import.name {
                Some(name) => name.clone(),
                None if !target.path.is_empty() => target.path[0].clone(),
                None => return None,
            };
            let Some(mut exported) = self.exported(module, &name, &mut HashSet::new()) else {
                return Some(target);
            };
            if import.name.is_none() {
                target.path.remove(0);
            }
            // `export default calls.length` exports the value it had at load.
            exported.tail = target.tail.map(|t| t.min(target.path.len()));
            if !exported.path.is_empty() && exported.tail.is_none() {
                exported.tail = Some(target.path.len());
            }
            exported.path.extend(target.path);
            target = exported;
        }
    }

    fn unit(
        &mut self,
        module: usize,
        binding: &Binding,
        path: &[String],
        parameters: &[ParameterBinding],
    ) -> Option<(usize, u32)> {
        let target = self.target(module, binding, path, parameters)?;
        self.unit_at(&target)
    }

    /// The function a resolved name denotes; `check.call` and `check.apply` denote `check`.
    fn unit_at(&self, target: &Target) -> Option<(usize, u32)> {
        let walk = &self.modules[target.module].walk;
        let lookup = |path: &[String]| match (&target.binding, path.is_empty()) {
            (Binding::Decl(decl), true) => walk.fn_decls.get(decl).copied(),
            (_, true) => None,
            (binding, false) => walk.fn_members.get(binding)?.get(path).copied(),
        };
        let key = lookup(&target.path).or_else(|| match target.path.split_last() {
            Some((last, rest)) if matches!(last.as_str(), "call" | "apply") => lookup(rest),
            _ => None,
        })?;
        Some((target.module, key))
    }

    fn reads(target: &Target, asserted: bool, recorders: &[Target], mock: bool) -> bool {
        recorders.iter().any(|recorder| {
            target.module == recorder.module
                && target.binding == recorder.binding
                && target.path.starts_with(&recorder.path)
                && target
                    .tail
                    .is_none_or(|tail| target.path.len() - tail <= recorder.path.len())
        }) || (mock
            && reads_console_mock(&Ref {
                binding: target.binding.clone(),
                path: target.path.clone(),
                read: true,
                asserted,
                escapes: true,
            }))
    }

    /// Bind simple arguments per call, so `check(calls)` can assert its parameter without
    /// conflating it with a call of the same helper on an unrelated array.
    fn call_frame(
        &mut self,
        caller: &Frame,
        arguments: &[Option<(Binding, Vec<String>)>],
        (module, key): (usize, u32),
        fails: bool,
    ) -> Frame {
        let mut parameters = caller.parameters.clone();
        let declarations = self.modules[module].walk.units[&key].parameters.clone();
        for (index, declaration) in declarations.into_iter().enumerate() {
            let Some(declaration) = declaration else {
                continue;
            };
            parameters.retain(|parameter| {
                parameter.module != module || parameter.declaration != declaration
            });
            if let Some((binding, path)) = arguments.get(index).and_then(Option::as_ref) {
                if let Some(target) = self.target(caller.module, binding, path, &caller.parameters)
                {
                    parameters.push(ParameterBinding {
                        module,
                        declaration,
                        target,
                    });
                }
            }
        }
        parameters.sort_by_key(|parameter| (parameter.module, parameter.declaration));
        Frame {
            module,
            key,
            parameters,
            fails,
        }
    }

    /// Longer paths cannot become a known method or change a recorder-prefix match by
    /// appending more members. Equate those contexts so `check(rec.next)` recursion cannot
    /// grow the visited-state key forever; retain the full path while evaluating a frame.
    fn context_key(&self, frame: &Frame, recorders: &[Target]) -> Frame {
        let mut context = frame.clone();
        for parameter in &mut context.parameters {
            let target = &mut parameter.target;
            let method_length = self.modules[target.module]
                .walk
                .fn_members
                .get(&target.binding)
                .into_iter()
                .flat_map(|members| members.keys())
                .map(Vec::len)
                .max()
                .unwrap_or(0);
            let recorder_length = recorders
                .iter()
                .filter(|recorder| {
                    recorder.module == target.module && recorder.binding == target.binding
                })
                .map(|recorder| recorder.path.len())
                .max()
                .unwrap_or(0);
            target.path.truncate(method_length.max(recorder_length) + 1);
        }
        context
    }

    fn guard(&mut self) -> bool {
        let mut recorders = Vec::new();
        let mut mock = false;
        for replacement in self.modules[0].walk.replacements.clone() {
            let unit = match replacement {
                Replacement::Unit(key) => Some((0, key)),
                Replacement::Binding(binding) => self.unit(0, &binding, &[], &[]),
                Replacement::Mock => {
                    mock = true;
                    None
                }
            };
            if let Some((module, key)) = unit {
                let pushes = self.modules[module]
                    .walk
                    .units
                    .get(&key)
                    .map(|unit| unit.pushes.clone())
                    .unwrap_or_default();
                for (binding, path) in pushes {
                    if let Some(target) = self.target(module, &binding, &path, &[]) {
                        recorders.push(target);
                    }
                }
            }
        }
        if recorders.is_empty() && !mock {
            return false;
        }
        for hook in self.modules[0].walk.hooks.clone() {
            let start = match hook {
                HookRoot::Unit(key) => Some((0, key)),
                HookRoot::Binding(binding, path) => self.unit(0, &binding, &path, &[]),
            };
            let Some(start) = start else {
                continue;
            };
            let mut seen = HashSet::new();
            let mut stack = vec![Frame {
                module: start.0,
                key: start.1,
                parameters: Vec::new(),
                fails: true,
            }];
            let (mut reads, mut throws) = (false, false);
            while let Some(frame) = stack.pop() {
                let (module, key) = (frame.module, frame.key);
                if !seen.insert(self.context_key(&frame, &recorders))
                    || self.modules[module].walk.deferred.contains(&key)
                {
                    continue;
                }
                let Some(unit) = self.modules[module].walk.units.get(&key).cloned() else {
                    continue;
                };
                throws |= frame.fails && unit.throws;
                for reference in unit.refs.iter().filter(|reference| reference.read) {
                    let Some(target) = self.target(
                        module,
                        &reference.binding,
                        &reference.path,
                        &frame.parameters,
                    ) else {
                        continue;
                    };
                    let read = Self::reads(&target, reference.asserted, &recorders, mock);
                    reads |= read;
                    throws |= frame.fails && read && reference.asserted;
                    // Any other mention of a function is followed too (`[check].forEach(…)`):
                    // its reads count, its failures only when nothing local swallows them.
                    let called = unit.calls.iter().any(|call| {
                        call.binding == reference.binding
                            && call.path == reference.path
                            && (call.escapes || !reference.escapes)
                    });
                    if !called {
                        if let Some(callee) = self.unit_at(&target) {
                            let fails = frame.fails && reference.escapes;
                            stack.push(self.call_frame(&frame, &[], callee, fails));
                        }
                    }
                }
                for call in &unit.calls {
                    if let Some(callee) =
                        self.unit(module, &call.binding, &call.path, &frame.parameters)
                    {
                        let fails = frame.fails && call.escapes;
                        stack.push(self.call_frame(&frame, &call.arguments, callee, fails));
                    }
                }
                stack.extend(unit.children.iter().map(|(key, escapes)| Frame {
                    module,
                    key: *key,
                    parameters: frame.parameters.clone(),
                    fails: frame.fails && *escapes,
                }));
            }
            if reads && throws {
                return true;
            }
        }
        false
    }
}
