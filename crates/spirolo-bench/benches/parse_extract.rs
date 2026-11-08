//! Criterion bench for the parse-and-extract phase, tracked over time.
//!
//! `spirolo list`'s cost is dominated by walking, parsing and extracting every file; the
//! milestone budget (`spirolo-bench` asserts it against `truth.json`) is the ceiling, this
//! is the trend line under it. It runs [`Model::build_with`] — the same call the binary
//! makes, recognisers and all — over the largest committed fixture, so the number moves
//! only when the phase itself does, never because a corpus repo is or isn't cloned.
//!
//! "Largest" is chosen by parseable bytes at run time rather than hard-coded, so adding a
//! bigger fixture re-points the bench without an edit here; the chosen fixture is printed.

// `criterion_group!`/`criterion_main!` expand to undocumented public items; a bench target
// has no public API to document, so silence `missing_docs` for this file alone.
#![allow(missing_docs)]

use std::path::{Path, PathBuf};

use criterion::{criterion_group, criterion_main, Criterion};
use spirolo_model::{BuildOptions, Model};

/// Source extensions the walk parses — the bytes that make a fixture "large" for this phase.
const SOURCE_EXTENSIONS: &[&str] = &["ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs"];

fn parse_extract(criterion: &mut Criterion) {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../spirolo-model/tests/fixtures");
    let (name, dir, bytes) = largest_fixture(&fixtures)
        .expect("the committed fixtures directory holds at least one parseable project");
    println!("parse_extract: largest fixture is {name} ({bytes} source bytes)");

    let recognisers = spirolo_frameworks::recognisers();
    let options = BuildOptions::default();
    criterion.bench_function("parse_extract/largest_fixture", |bencher| {
        bencher.iter(|| Model::build_with(std::hint::black_box(&dir), &options, &recognisers));
    });
}

/// The fixture directory holding the most parseable source, as `(name, path, bytes)`.
fn largest_fixture(fixtures: &Path) -> Option<(String, PathBuf, u64)> {
    std::fs::read_dir(fixtures)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .map(|dir| {
            let name = dir
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_owned();
            let bytes = source_bytes(&dir);
            (name, dir, bytes)
        })
        .max_by_key(|(_, _, bytes)| *bytes)
}

/// Total bytes of parseable source under `dir`, recursively.
fn source_bytes(dir: &Path) -> u64 {
    let mut total = 0;
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return 0;
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.is_dir() {
            total += source_bytes(&path);
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| SOURCE_EXTENSIONS.contains(&extension))
        {
            total += entry.metadata().map_or(0, |meta| meta.len());
        }
    }
    total
}

criterion_group!(benches, parse_extract);
criterion_main!(benches);
