//! Source-to-names tests for [`imported_calls`]: which imported functions a test file calls
//! where the runner registers tests.

use spirolo_frameworks::imported_calls;

#[test]
fn a_top_level_call_to_an_imported_suite_is_named() {
    let source = "import { formTests } from '@rjsf/snapshot-tests';\n\
                  import Form from '../src/index.ts';\n\
                  formTests(Form);\n";
    assert_eq!(imported_calls("test/Form.test.tsx", source), ["formTests"]);
}

#[test]
fn a_wrapper_inside_a_describe_and_a_concise_callback_is_named() {
    let source = "import { each } from './helpers';\n\
                  describe('suite', () => [1].forEach(() => each('x', () => {})));\n";
    assert_eq!(imported_calls("a.test.ts", source), ["each"]);
}

#[test]
fn the_runners_imports_and_calls_inside_a_test_or_hook_are_not_named() {
    let source = "import { describe, it, expect } from 'vitest';\n\
                  import { render, cleanup } from '@testing-library/react';\n\
                  import type { Props } from './props';\n\
                  describe('Widget', () => {\n\
                  \x20 beforeEach(() => { cleanup() });\n\
                  \x20 it('renders', () => { expect(render()).toMatchSnapshot() });\n\
                  });\n";
    assert!(imported_calls("widget.test.ts", source).is_empty());
}

#[test]
fn a_js_file_holding_jsx_is_read_as_its_runner_reads_it() {
    // Shaped on emotion's packages/jest/test/react-enzyme.test.js: JSX in a `.js` file, and
    // the imported `jestInCase` under a `describe` chosen by a conditional.
    let source = "import jestInCase from 'jest-in-case'\n\
                  const cases = { basic: { render() { return <div css={{ color: 'red' }}>hi</div> } } }\n\
                  ;(isReact16 ? describe : describe.skip)('enzyme', () => {\n\
                  \x20 jestInCase('mount', ({ render }) => { expect(render()).toMatchSnapshot() }, cases)\n\
                  })\n";
    assert_eq!(
        imported_calls("packages/jest/test/react-enzyme.test.js", source),
        ["jestInCase"]
    );
}
