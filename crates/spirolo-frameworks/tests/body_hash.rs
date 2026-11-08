//! The body hash: equal for the same behaviour under a different name, different when a
//! literal changes. Reads only the `TestCase` rows the recogniser returns.

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use spirolo_frameworks::VitestJest;
use spirolo_model::{FileContext, Language, Recogniser, TestCase, TestKind};

/// Run the recogniser over one TypeScript snippet, as if it were `src/example.test.ts`.
fn recognise(source: &str) -> Vec<TestCase> {
    recognise_at("src/example.test.ts", source)
}

/// Run the recogniser over one snippet as if it were the file at `path`.
fn recognise_at(path: &str, source: &str) -> Vec<TestCase> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::from_path(path).unwrap()).parse();
    let file = FileContext {
        path,
        language: Language::TypeScript,
        source,
        program: &parsed.program,
        handler_globs: None,
    };
    VitestJest.recognise(&file)
}

/// The body hash of the one test in a snippet.
fn hash_of(source: &str) -> String {
    hash_at("src/example.test.ts", source)
}

/// The body hash of the one test in a snippet read as the file at `path`.
fn hash_at(path: &str, source: &str) -> String {
    recognise_at(path, source)
        .into_iter()
        .find(|row| row.kind == TestKind::Test)
        .expect("a test")
        .body_hash
        .expect("a body hash")
}

#[test]
fn same_body_different_title_hashes_equal() {
    let one =
        hash_of("import { it } from 'vitest'\nit('adds', () => { expect(add(1, 2)).toBe(3) })\n");
    let two =
        hash_of("import { it } from 'vitest'\nit('sums', () => { expect(add(1, 2)).toBe(3) })\n");
    assert_eq!(one, two, "the title is not part of the hash");
}

#[test]
fn changing_one_literal_changes_the_hash() {
    let three =
        hash_of("import { it } from 'vitest'\nit('t', () => { expect(add(1, 2)).toBe(3) })\n");
    let four =
        hash_of("import { it } from 'vitest'\nit('t', () => { expect(add(1, 2)).toBe(4) })\n");
    assert_ne!(three, four, "a changed literal changes the hash");
}

#[test]
fn a_consistent_local_rename_does_not_change_the_hash() {
    let sum = hash_of(
        "import { it } from 'vitest'\n\
         it('t', () => { const sum = add(1, 2); expect(sum).toBe(3) })\n",
    );
    let total = hash_of(
        "import { it } from 'vitest'\n\
         it('t', () => { const total = add(1, 2); expect(total).toBe(3) })\n",
    );
    assert_eq!(sum, total, "renaming a local consistently keeps the hash");
}

#[test]
fn renaming_a_free_reference_does_change_the_hash() {
    // `add` is not bound in the body, so it is behaviour, not a local: swapping it for a
    // different function is a different test, and the hash must say so.
    let add = hash_of("import { it } from 'vitest'\nit('t', () => { expect(add(1)).toBe(1) })\n");
    let sub = hash_of("import { it } from 'vitest'\nit('t', () => { expect(sub(1)).toBe(1) })\n");
    assert_ne!(add, sub, "a different free callee is a different body");
}

#[test]
fn whitespace_and_formatting_do_not_change_the_hash() {
    let tight = hash_of("import { it } from 'vitest'\nit('t', () => {expect(x).toBe(1)})\n");
    let loose = hash_of(
        "import { it } from 'vitest'\n\
         it('t', () => {\n    expect(x).toBe(1)\n})\n",
    );
    assert_eq!(tight, loose, "whitespace is not part of the hash");
}

#[test]
fn a_bodyless_todo_has_no_hash() {
    let rows = recognise("import { test } from 'vitest'\ntest.todo('later')\n");
    assert_eq!(rows[0].body_hash, None, "nothing to hash");
}

#[test]
fn a_suite_carries_no_body_hash() {
    let rows = recognise(
        "import { describe, it } from 'vitest'\n\
         describe('s', () => { it('a', () => {}) })\n",
    );
    let suite = rows.iter().find(|row| row.kind == TestKind::Suite).unwrap();
    assert_eq!(suite.body_hash, None, "a describe body is not a test body");
}

#[test]
fn the_hash_is_a_stable_snapshot() {
    // Pins the exact digest so an accidental change to how bodies are hashed is caught.
    insta::assert_json_snapshot!(recognise(
        "import { it } from 'vitest'\nit('adds', () => { expect(add(1, 2)).toBe(3) })\n"
    ));
}

/// Hashes recorded by the pre-bump binary (oxc 0.110, commit 4eecaf4) on these exact
/// sources: a parser bump must not move a body hash, because `duplicate-test` finding
/// ids and ratchet baselines are built from it.
#[test]
fn an_oxc_bump_keeps_the_hashes_recorded_under_oxc_0_110() {
    let recorded = [
        (
            "it('reads import.meta', () => {\n  expect(import.meta.url).toContain('shapes')\n})\n",
            "c6a7bb4447c9ed08",
        ),
        (
            "it('reads new.target', () => {\n  function F() { return new.target }\n  expect(F()).toBeUndefined()\n})\n",
            "9e4e011010f79a35",
        ),
        (
            "it('declares an interface extending a name', () => {\n  interface A extends Base { x: number }\n  interface B extends ns.Base<string> { y: number }\n  const a: A = { x: 1 } as A\n  expect(a.x).toBe(1)\n})\n",
            "6c9b626bd3755047",
        ),
        ("it('concise', () => expect(1).toBe(1))\n", "d41a872fe9653b94"),
    ];
    for (body, hash) in recorded {
        let source = format!("import {{ it, expect }} from 'vitest'\n\n{body}");
        assert_eq!(hash_of(&source), hash, "{body}");
    }
}

/// A JSX body as the carbon `ComboBox` suite writes it (`ComboBox-test.js:587` and `:605`),
/// rendering the component with `prop` set.
fn combobox_body(prop: &str) -> String {
    format!(
        "it('restores the label on blur', async () => {{\n\
         \x20 render(\n\
         \x20   <ComboBox\n\
         \x20     {{...mockProps}}\n\
         \x20     {prop}={{mockProps.items[1]}}\n\
         \x20     allowCustomValue={{false}}\n\
         \x20   />\n\
         \x20 );\n\
         \x20 expect(findInputNode()).toHaveDisplayValue('Item 1');\n\
         \x20 await userEvent.keyboard('[Tab]');\n\
         \x20 expect(findInputNode()).toHaveDisplayValue('Item 1');\n\
         }});\n"
    )
}

#[test]
fn a_different_jsx_attribute_name_changes_the_hash() {
    let path = "src/ComboBox-test.jsx";
    let initial = hash_at(path, &combobox_body("initialSelectedItem"));
    let controlled = hash_at(path, &combobox_body("selectedItem"));
    assert_ne!(
        initial, controlled,
        "an attribute name is meaning, not a rename target"
    );
    assert_eq!(
        initial,
        hash_at(path, &combobox_body("initialSelectedItem")),
        "identical JSX bodies still hash equal"
    );
}

#[test]
fn every_jsx_name_part_and_jsx_text_is_part_of_the_hash() {
    let path = "src/example.test.tsx";
    let body = |jsx: &str| hash_at(path, &format!("it('t', () => {{ render({jsx}) }})\n"));
    let pairs = [
        ("<div />", "<span />"),
        ("<Foo.Bar />", "<Foo.Baz />"),
        ("<svg:rect />", "<svg:circle />"),
        ("<a xlink:href=\"x\" />", "<a xlink:role=\"x\" />"),
        ("<p>yes</p>", "<p>no</p>"),
    ];
    for (one, two) in pairs {
        assert_ne!(body(one), body(two), "{one} vs {two}");
    }
}
