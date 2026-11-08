//! Snippet-to-rows tests for the saga recogniser: one idiom, one snapshot.
//!
//! Every case reads the `SagaWalk` rows the recogniser returns — the same table the
//! `generator-walk` rule counts. The recogniser claims a file only when it imports
//! `redux-saga/effects` or `redux-saga-test-plan`, so a snippet that steps a generator
//! without those imports produces nothing, the way recognition is by import elsewhere.

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use spirolo_frameworks::Saga;
use spirolo_model::{FileContext, Language, Recogniser, SagaStepKind, SagaWalk};

/// Run the saga recogniser over one snippet at a test-file path.
fn walks_at(path: &str, source: &str) -> Vec<SagaWalk> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::from_path(path).unwrap()).parse();
    let file = FileContext {
        path,
        language: Language::TypeScript,
        source,
        program: &parsed.program,
        handler_globs: None,
    };
    Saga.saga_walks(&file)
}

/// Run the recogniser over a snippet as a saga test file.
fn walks(source: &str) -> Vec<SagaWalk> {
    walks_at("src/sagas.test.ts", source)
}

// ---------------------------------------------------------------- the acceptance snippets

#[test]
fn gen_next_value_against_an_effect_creator_is_one_step() {
    let walks = walks(
        "import { call } from 'redux-saga/effects'\n\
         it('walks', () => {\n\
         \x20 const gen = saga(1)\n\
         \x20 expect(gen.next().value).toEqual(call(api, 1))\n\
         })\n",
    );
    assert_eq!(walks.len(), 1, "one gen.next().value step");
    assert_eq!(walks[0].kind, SagaStepKind::GenNext);
    assert_eq!(walks[0].effect.as_deref(), Some("call"));
    insta::assert_json_snapshot!("gen_next_effect_creator", walks);
}

#[test]
fn a_test_saga_chain_counts_one_step_per_next() {
    let walks = walks(
        "import { testSaga } from 'redux-saga-test-plan'\n\
         it('walks', () => {\n\
         \x20 testSaga(s).next().put(a).next().isDone()\n\
         })\n",
    );
    assert_eq!(walks.len(), 2, "two .next() steps in the chain");
    assert!(walks.iter().all(|w| w.kind == SagaStepKind::TestSaga));
    insta::assert_json_snapshot!("test_saga_chain", walks);
}

#[test]
fn a_to_equal_against_a_plain_object_is_no_step() {
    // The matcher argument is a plain object, not a call to an effect creator, and the
    // subject is not a `.next().value`: this is an ordinary assertion, never a step-walk.
    let walks = walks(
        "import { call } from 'redux-saga/effects'\n\
         it('asserts a value', () => {\n\
         \x20 expect(result).toEqual({ ok: true })\n\
         })\n",
    );
    assert!(walks.is_empty(), "a plain-object assertion is not a step");
}

// ------------------------------------------------------------------ recognition by import

#[test]
fn without_a_saga_import_a_gen_next_assertion_is_not_a_step() {
    // The same `gen.next().value` shape, but no `redux-saga/effects` import: `call` here is
    // some other function, so the recogniser stays silent rather than guess.
    let walks = walks(
        "it('walks', () => {\n\
         \x20 expect(gen.next().value).toEqual(call(api, 1))\n\
         })\n",
    );
    assert!(walks.is_empty(), "recognition is by import, never by shape");
}

#[test]
fn a_gen_next_value_against_a_non_effect_call_is_not_a_step() {
    // `redux-saga/effects` is imported, but the assertion is against `helper(x)`, a call to
    // something that is not an imported effect creator: not a yield-order step-walk.
    let walks = walks(
        "import { call } from 'redux-saga/effects'\n\
         it('walks', () => {\n\
         \x20 expect(gen.next().value).toEqual(helper(x))\n\
         })\n",
    );
    assert!(walks.is_empty(), "only a call to an effect creator counts");
}

#[test]
fn an_aliased_effect_creator_is_recognised() {
    // `import { call as callEffect }`: the local alias is what appears in the assertion, so
    // the recogniser folds the import alias, not the original name.
    let walks = walks(
        "import { call as callEffect } from 'redux-saga/effects'\n\
         it('walks', () => {\n\
         \x20 expect(gen.next().value).toEqual(callEffect(api, 1))\n\
         })\n",
    );
    assert_eq!(walks.len(), 1);
    assert_eq!(walks[0].effect.as_deref(), Some("callEffect"));
}

#[test]
fn several_effect_creators_across_steps_each_count() {
    let walks = walks(
        "import { call, put, take } from 'redux-saga/effects'\n\
         it('walks the whole saga', () => {\n\
         \x20 expect(gen.next().value).toEqual(take('GO'))\n\
         \x20 expect(gen.next().value).toEqual(call(api, 1))\n\
         \x20 expect(gen.next(result).value).toEqual(put(done()))\n\
         })\n",
    );
    assert_eq!(walks.len(), 3, "three step-walk assertion sites");
    let effects: Vec<&str> = walks.iter().filter_map(|w| w.effect.as_deref()).collect();
    assert_eq!(effects, vec!["take", "call", "put"]);
}

#[test]
fn a_test_saga_step_records_its_fluent_effect() {
    let walks = walks(
        "import { testSaga } from 'redux-saga-test-plan'\n\
         it('walks', () => {\n\
         \x20 testSaga(saga, action)\n\
         \x20   .next().call(api, 1)\n\
         \x20   .next(result).put(succeeded(result))\n\
         \x20   .next().isDone()\n\
         })\n",
    );
    assert_eq!(walks.len(), 3, "three .next() steps");
    let effects: Vec<&str> = walks.iter().filter_map(|w| w.effect.as_deref()).collect();
    assert_eq!(effects, vec!["call", "put", "isDone"]);
}

#[test]
fn a_gen_next_result_read_through_a_binding_is_a_step() {
    // The common `const step = gen.next(); expect(step.value).toEqual(all(...))` idiom, where
    // the yield is read through a binding rather than inline: still one step.
    let walks = walks(
        "import { all, takeEvery } from 'redux-saga/effects'\n\
         it('walks', () => {\n\
         \x20 const result = gen.next()\n\
         \x20 expect(result.value).toEqual(all([takeEvery('CALL', handler)]))\n\
         })\n",
    );
    assert_eq!(walks.len(), 1, "a bound .next() result is a step");
    assert_eq!(walks[0].effect.as_deref(), Some("all"));
}

#[test]
fn a_deeper_read_of_the_yield_is_not_a_step() {
    // `expect(step.value.type).toBe('SELECT')` reads *into* the yielded value and asserts a
    // literal, not a whole effect: it is not a yield-order assertion against an effect creator.
    let walks = walks(
        "import { call } from 'redux-saga/effects'\n\
         it('inspects the yield', () => {\n\
         \x20 const step = gen.next()\n\
         \x20 expect(step.value.type).toBe('SELECT')\n\
         \x20 expect(step.done).toBe(false)\n\
         })\n",
    );
    assert!(walks.is_empty(), "a deeper read is not an effect step-walk");
}

#[test]
fn rows_are_sorted_by_span() {
    let walks = walks(
        "import { call, put } from 'redux-saga/effects'\n\
         it('walks', () => {\n\
         \x20 expect(gen.next().value).toEqual(call(api))\n\
         \x20 expect(gen.next().value).toEqual(put(done()))\n\
         })\n",
    );
    assert_eq!(walks.len(), 2);
    assert!(walks[0].span.start < walks[1].span.start, "sorted by span");
}
