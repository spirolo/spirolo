//! The seam where framework recognisers turn one parsed file into test-case rows.
//!
//! The recognisers live in `spirolo-frameworks`, which depends on this crate. The model
//! is the leaf of the workspace and cannot name them, so it defines the trait and the
//! context here and lets a caller inject the set: the binary wires in
//! `spirolo-frameworks`'s recognisers via [`Model::build_with`], and the model itself
//! stays free of any framework knowledge.
//!
//! [`Model::build_with`]: crate::Model::build_with

use oxc_ast::ast::Program;

use crate::{Gate, Handler, Language, SagaWalk, TestCase};

/// One parsed file, handed to every recogniser while its arena is still alive.
///
/// Everything here is borrowed, never owned: the AST lives in the per-file arena inside
/// the extractor and is dropped the moment extraction returns, so a recogniser reads it
/// and hands back owned rows rather than holding on to a node.
pub struct FileContext<'a> {
    /// Repo-relative, slash-separated path of the file.
    pub path: &'a str,
    /// The dialect the file was read as.
    pub language: Language,
    /// The file's source text, so a recogniser can lift a condition's exact spelling
    /// out of a span rather than re-print the AST.
    pub source: &'a str,
    /// The parsed program.
    pub program: &'a Program<'a>,
    /// The configured handler globs, when `.spirolorc.jsonc` supplied any. `None` means no
    /// config opinion, so a handler recogniser falls back to its own default set of files;
    /// `Some(set)` replaces that default, and a file is a handler file when it matches.
    pub handler_globs: Option<&'a globset::GlobSet>,
}

/// Turns one parsed file into the rows it declares.
///
/// A recogniser is a pure function: same file in, same rows out, no state and no I/O.
/// It is [`Sync`] because the extractor runs it on the work-stealing pool, one file to
/// a task. A recogniser overrides the tables it claims and leaves the rest empty: the
/// Vitest/Jest and Playwright families produce test cases, the MSW/nock family produces
/// handlers, the config family produces gates, and none is obliged to fill a table it
/// knows nothing about.
pub trait Recogniser: Sync {
    /// The test, suite, hook and benchmark rows this recogniser claims in `file`.
    /// Defaults to none, for a recogniser that only contributes handlers.
    fn recognise(&self, file: &FileContext<'_>) -> Vec<TestCase> {
        let _ = file;
        Vec::new()
    }

    /// The MSW/nock request-interceptor rows this recogniser claims in `file`.
    /// Defaults to none, for a recogniser that only contributes test cases.
    fn handlers(&self, file: &FileContext<'_>) -> Vec<Handler> {
        let _ = file;
        Vec::new()
    }

    /// The saga step-walk sites this recogniser claims in `file`: `gen.next().value`
    /// assertions against effect creators and `testSaga` chain steps. Defaults to none, for
    /// a recogniser that reads tests, handlers or gates instead.
    fn saga_walks(&self, file: &FileContext<'_>) -> Vec<SagaWalk> {
        let _ = file;
        Vec::new()
    }

    /// The `Gate` rows this recogniser reads from `file` when it is a runner or mutation
    /// config. Defaults to none, for a recogniser that reads tests or handlers instead.
    ///
    /// A JSON config (`jest.config.json`, `stryker.conf.json`, a `package.json` with a
    /// `jest` key) arrives with [`FileContext::language`] set to [`Language::Json`] and
    /// its program being `module.exports = <json>;`, so the same object-literal reader
    /// serves both spellings.
    fn gates(&self, file: &FileContext<'_>) -> Vec<Gate> {
        let _ = file;
        Vec::new()
    }
}
