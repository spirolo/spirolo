//! The modules each test file reaches, with the named exports of each that the test file
//! references by identifier: the weakness input of `untested-hotspot` (PRD 0005). Trees on
//! disk give real references and real resolution; the model crate has no recognisers, so
//! each test adds the `TestCase` row that makes a file a test file.

use std::path::{Path, PathBuf};

use spirolo_model::{
    BuildOptions, Model, Pairing, PairingReason, Span, TestCase, TestKind, TestReach,
};

/// A fresh tree under the target directory, prefixed so no other test shares it.
fn tree(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("test-reach-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    for (path, source) in files {
        let full = root.join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, source).unwrap();
    }
    root
}

/// A collected vitest test in `file`, which makes `file` a test file.
fn test_case(file: &str) -> TestCase {
    TestCase {
        file: file.to_owned(),
        span: Span::default(),
        framework: Some("vitest".to_owned()),
        title: Some("t".to_owned()),
        kind: TestKind::Test,
        suite_path: Vec::new(),
        assertions: Vec::new(),
        throwing_queries: Vec::new(),
        mocks: Vec::new(),
        skip: None,
        each: None,
        loops: Vec::new(),
        modifiers: Vec::new(),
        tags: Vec::new(),
        body_hash: None,
        empty: false,
        context: Vec::new(),
    }
}

/// Build the tree's model and mark `tests` as test files.
fn model(root: &Path, tests: &[&str]) -> Model {
    let mut model = Model::build(root, &BuildOptions::default());
    model
        .test_cases
        .extend(tests.iter().map(|file| test_case(file)));
    model
}

fn reach(test_file: &str, module: &str, referenced: &[&str]) -> TestReach {
    TestReach {
        test_file: test_file.to_owned(),
        module: module.to_owned(),
        referenced: referenced.iter().map(|name| (*name).to_owned()).collect(),
    }
}

#[test]
fn a_test_reaches_every_module_in_its_import_closure_with_the_exports_it_names() {
    let root = tree(
        "closure",
        &[
            (
                "src/math.ts",
                "import { round } from './round'\n\
                 export const add = (a: number, b: number) => round(a + b)\n\
                 export const sub = (a: number, b: number) => round(a - b)\n",
            ),
            (
                "src/round.ts",
                "export const round = (n: number) => Math.round(n)\n",
            ),
            (
                "src/math.test.ts",
                "import { add } from './math'\n\
                 it('adds', () => { expect(add(1, 2)).toBe(3) })\n",
            ),
        ],
    );
    let model = model(&root, &["src/math.test.ts"]);

    assert_eq!(
        model.test_reach(),
        vec![
            reach("src/math.test.ts", "src/math.ts", &["add"]),
            reach("src/math.test.ts", "src/round.ts", &[]),
        ]
    );
}

#[test]
fn a_module_paired_with_a_test_is_reached_even_through_an_import_that_did_not_resolve() {
    // `@/format` has no alias the model can follow, so the only link is the sibling pairing.
    let root = tree(
        "paired",
        &[
            (
                "src/format.ts",
                "import { pad } from './pad'\n\
                 export const money = (n: number) => pad(n.toFixed(2))\n\
                 export const percent = (n: number) => `${n}%`\n",
            ),
            (
                "src/pad.ts",
                "export const pad = (s: string) => s.padStart(8)\n",
            ),
            (
                "src/format.test.ts",
                "import { percent } from '@/format'\n\
                 it('formats', () => { expect(percent(5)).toBe('5%') })\n",
            ),
        ],
    );
    let mut model = model(&root, &["src/format.test.ts"]);
    model.pairings.push(Pairing {
        test_file: "src/format.test.ts".to_owned(),
        source_module: "src/format.ts".to_owned(),
        reason: PairingReason::SiblingName,
        ambiguous: false,
    });

    // The pairing reaches `format.ts` alone: its own imports are not the test's closure.
    assert_eq!(
        model.test_reach(),
        vec![reach("src/format.test.ts", "src/format.ts", &["percent"])]
    );
}

#[test]
fn only_named_exports_count_and_a_named_re_export_is_one() {
    let root = tree(
        "named",
        &[
            (
                "src/cart/index.ts",
                "export { total } from './total'\n\
                 export * from './tax'\n\
                 export type Line = { price: number }\n\
                 export default function cart() { return [] }\n",
            ),
            (
                "src/cart/total.ts",
                "export const total = (lines: { price: number }[]) =>\n  \
                   lines.reduce((sum, line) => sum + line.price, 0)\n",
            ),
            ("src/cart/tax.ts", "export const vat = (n: number) => n * 0.2\n"),
            (
                "src/cart/index.test.ts",
                "import cart, { total, vat, type Line } from './index'\n\
                 const lines: Line[] = [{ price: 2 }]\n\
                 it('totals', () => { expect(total(lines) + vat(1)).toBe(2.2); expect(cart()).toEqual([]) })\n",
            ),
        ],
    );
    let model = model(&root, &["src/cart/index.test.ts"]);

    // `cart` (default), `Line` (a type) and the star-exported `vat` are not named exports of
    // the barrel; `vat` is one of `tax.ts`, where the test's read of it counts.
    assert_eq!(
        model.test_reach(),
        vec![
            reach("src/cart/index.test.ts", "src/cart/index.ts", &["total"]),
            reach("src/cart/index.test.ts", "src/cart/tax.ts", &["vat"]),
            reach("src/cart/index.test.ts", "src/cart/total.ts", &["total"]),
        ]
    );
}

#[test]
fn a_member_read_through_a_namespace_import_is_not_an_identifier_reference() {
    // PRD 0005 counts an export as referenced when the test reads its name as an
    // identifier; `math.add` reads the property `add` of the binding `math`.
    let root = tree(
        "namespace",
        &[
            (
                "src/math.ts",
                "export const add = (a: number, b: number) => a + b\n",
            ),
            (
                "src/math.test.ts",
                "import * as math from './math'\n\
                 it('adds', () => { expect(math.add(1, 2)).toBe(3) })\n",
            ),
        ],
    );
    let model = model(&root, &["src/math.test.ts"]);

    assert_eq!(
        model.test_reach(),
        vec![reach("src/math.test.ts", "src/math.ts", &[])]
    );
}
