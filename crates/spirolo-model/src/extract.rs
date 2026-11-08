//! One file in, owned facts out.
//!
//! This is the seam the parallel phase runs: the arena and the AST live entirely
//! inside [`extract_file`] and are dropped before it returns, so nothing borrowed from
//! a parse ever reaches the model. Later slices add facts *inside* this function
//! rather than around it.

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;

use crate::recognise::{FileContext, Recogniser};
use crate::{
    Diagnostic, Gate, Handler, Language, Module, ParseStatus, SagaWalk, Severity, Span, TestCase,
};

/// Everything one file contributed to the model.
pub struct FileFacts {
    /// The file's row in the `Module` table. Present whenever the file has a path the
    /// model can spell, even when it could not be read or parsed.
    pub module: Option<Module>,
    /// The test, suite, hook and benchmark rows the recognisers claimed in the file.
    pub test_cases: Vec<TestCase>,
    /// The MSW/nock handler rows the recognisers claimed in the file.
    pub handlers: Vec<Handler>,
    /// The saga step-walk sites the recognisers claimed in the file.
    pub saga_walks: Vec<SagaWalk>,
    /// The flake mechanisms and guards written in the file.
    pub flake: crate::flake::FlakeFacts,
    /// The `Gate` rows the recognisers read from the file when it is a config.
    pub gates: Vec<Gate>,
    /// The captured constants and binding maps the whole-project fold runs over. Empty
    /// for a file that never reached the parser.
    pub fold: crate::constants::FoldFacts,
    /// The `package.json` this file is, when it is one: read as data, never a module.
    pub manifest: Option<crate::manifest::Manifest>,
    /// What could not be decided about the file.
    pub diagnostics: Vec<Diagnostic>,
}

impl FileFacts {
    /// A runner-selected file whose language has no case extractor.
    fn unsupported(path: &str, language: Language) -> Self {
        let mut facts = Self::failed(
            path.to_owned(),
            language,
            "language_unsupported",
            "file inventoried without case extraction: only JavaScript and TypeScript are parsed"
                .to_owned(),
        );
        facts.diagnostics[0].severity = Severity::Warning;
        facts
    }

    /// A file that never reached the parser: a [`ParseStatus::Failed`] row and one
    /// diagnostic saying why.
    pub fn failed(path: String, language: Language, code: &str, message: String) -> Self {
        Self {
            diagnostics: vec![Diagnostic {
                code: code.to_owned(),
                severity: Severity::Error,
                file: Some(path.clone()),
                span: None,
                message,
            }],
            module: Some(Module {
                path,
                language,
                parse_status: ParseStatus::Failed,
                imports: Vec::new(),
                exports: Vec::new(),
                constants: Vec::new(),
                mocks: Vec::new(),
                references: Vec::new(),
                spies: Vec::new(),
                loads: Vec::new(),
                complexity: 0,
                generated: false,
            }),
            ..Self::empty()
        }
    }

    /// A file that contributed nothing yet: every table empty, no module row.
    pub fn empty() -> Self {
        Self {
            module: None,
            test_cases: Vec::new(),
            handlers: Vec::new(),
            saga_walks: Vec::new(),
            flake: crate::flake::FlakeFacts::default(),
            gates: Vec::new(),
            fold: crate::constants::FoldFacts::default(),
            manifest: None,
            diagnostics: Vec::new(),
        }
    }

    /// A data file the walk found: a `package.json`, read for its name and directory, or
    /// a runner/mutation config written as JSON. Not a module, so the `Module` table never
    /// carries it.
    ///
    /// A JSON document is a JavaScript expression, so the config recogniser reads it with
    /// the same object-literal reader it uses for `jest.config.js`: the document is handed
    /// to every recogniser's `gates` as the program `module.exports = <json>;` with the
    /// language marked [`Language::Json`]. A `package.json` that is not JSON is a
    /// `manifest_invalid` diagnostic; any other data file that does not parse is
    /// `parse_failed`, as a module would be.
    pub fn data(path: &str, source: &str, recognisers: &[&dyn Recogniser]) -> Self {
        let mut facts = Self::empty();
        if path.rsplit('/').next() == Some("package.json") {
            match crate::manifest::read(path, source) {
                Ok(manifest) => facts.manifest = Some(manifest),
                Err(diagnostic) => {
                    facts.diagnostics.push(diagnostic);
                    return facts;
                }
            }
        }

        let wrapped = format!("module.exports = {source};");
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, &wrapped, SourceType::cjs()).parse();
        if parsed.fatal_error || parsed.diagnostics.has_errors() {
            facts.diagnostics.push(Diagnostic {
                code: "parse_failed".to_owned(),
                severity: Severity::Error,
                file: Some(path.to_owned()),
                span: None,
                message: parsed.diagnostics.errors().next().map_or_else(
                    || "not valid JSON".to_owned(),
                    |error| error.message.trim_end_matches('.').to_owned(),
                ),
            });
            return facts;
        }
        let file = FileContext {
            path,
            language: Language::Json,
            source: &wrapped,
            program: &parsed.program,
            handler_globs: None,
        };
        facts.gates = recognisers
            .iter()
            .flat_map(|recogniser| recogniser.gates(&file))
            .collect();
        facts
    }
}

/// The markers a generator leaves in a file's head, compared case-insensitively.
const GENERATED_MARKERS: [&str; 4] = [
    "@generated",
    "do not edit",
    "generated from",
    "auto-generated",
];

/// Whether `source` says it is generated: a [`GENERATED_MARKERS`] entry in its first 20
/// lines. A warning further down is the author's, not a generator's.
fn is_generated(source: &str) -> bool {
    source.lines().take(20).any(|line| {
        let line = line.to_lowercase();
        GENERATED_MARKERS.iter().any(|marker| line.contains(marker))
    })
}

/// Parse `source` and extract the facts the model keeps about it.
///
/// `path` is the model's spelling of the file (repo-relative, slash-separated); it is
/// used for the row and for the diagnostics, never to touch the disk again.
///
/// A file that does not parse still gets a row — a broken test file is a fact about
/// the suite, and dropping it would silently shrink every count downstream. Its
/// `parse_status` says how much of the row to trust, and one diagnostic carries the
/// parser's first complaint with its span.
pub fn extract_file(
    path: &str,
    language: Language,
    source: &str,
    recognisers: &[&dyn Recogniser],
    handler_globs: Option<&globset::GlobSet>,
) -> FileFacts {
    if language == Language::Unknown {
        return FileFacts::unsupported(path, language);
    }
    let allocator = Allocator::default();
    // React runners commonly transform JSX in .js as well as .jsx. Keep the
    // module language/path, but let the parser read those bodies faithfully.
    let source_type = SourceType::from_path(path)
        .unwrap_or_default()
        .with_jsx(matches!(
            language,
            Language::JavaScript | Language::Jsx | Language::Tsx
        ));
    let parsed = Parser::new(&allocator, source, source_type).parse();

    let parse_status = if parsed.fatal_error {
        ParseStatus::Failed
    } else if !parsed.diagnostics.has_errors() {
        ParseStatus::Parsed
    } else {
        ParseStatus::Recovered
    };

    let diagnostics = parsed
        .diagnostics
        .errors()
        .next()
        .map(|error| Diagnostic {
            code: "parse_failed".to_owned(),
            severity: Severity::Error,
            file: Some(path.to_owned()),
            span: error.labels.first().and_then(|label| {
                let start = label.offset();
                let end = start.checked_add(label.len())?;
                Some(Span::locate(source, start, end))
            }),
            message: error.message.trim_end_matches('.').to_owned(),
        })
        .into_iter()
        .collect();

    // A file that panicked the parser has no trustworthy tree to read; one that only
    // recovered still does, so both the recognisers and the import/export walk see it and
    // later rules can decide what a partial parse is worth.
    let (test_cases, handlers, saga_walks, flake, gates, module_facts, fold) =
        if parse_status == ParseStatus::Failed {
            (
                Vec::new(),
                Vec::new(),
                Vec::new(),
                crate::flake::FlakeFacts::default(),
                Vec::new(),
                crate::modules::ModuleFacts::default(),
                crate::constants::FoldFacts::default(),
            )
        } else {
            let file = FileContext {
                path,
                language,
                source,
                program: &parsed.program,
                handler_globs,
            };
            let test_cases = recognisers
                .iter()
                .flat_map(|recogniser| recogniser.recognise(&file))
                .collect();
            let handlers = recognisers
                .iter()
                .flat_map(|recogniser| recogniser.handlers(&file))
                .collect();
            let saga_walks = recognisers
                .iter()
                .flat_map(|recogniser| recogniser.saga_walks(&file))
                .collect();
            let gates = recognisers
                .iter()
                .flat_map(|recogniser| recogniser.gates(&file))
                .collect();
            (
                test_cases,
                handlers,
                saga_walks,
                crate::flake::collect(path, source, &parsed.program),
                gates,
                crate::modules::collect(source, &parsed.program),
                crate::constants::collect(source, &parsed.program),
            )
        };

    FileFacts {
        module: Some(module_row(
            path,
            language,
            parse_status,
            module_facts,
            source,
        )),
        test_cases,
        handlers,
        saga_walks,
        flake,
        gates,
        fold,
        manifest: None,
        diagnostics,
    }
}

/// The [`Module`] row for a parsed file, from its import/export walk.
fn module_row(
    path: &str,
    language: Language,
    parse_status: ParseStatus,
    facts: crate::modules::ModuleFacts,
    source: &str,
) -> Module {
    Module {
        path: path.to_owned(),
        language,
        parse_status,
        imports: facts.imports,
        exports: facts.exports,
        constants: Vec::new(),
        mocks: facts.mocks,
        references: facts.references,
        spies: facts.spies,
        loads: facts.loads,
        complexity: facts.complexity,
        generated: is_generated(source),
    }
}
