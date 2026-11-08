# Absence-anchor corpus excerpts

These are static parser fixtures. The test bodies are copied unchanged from the
pinned corpus checkouts; imports and enclosing suites are trimmed. The components
and their dependencies are not executed.

| Fixture | Source | Recorded verdict |
| --- | --- | --- |
| `rjsf.test.tsx`, DaisyUI test | `rjsf-team/react-jsonschema-form` at `ad0ea07428f8fde1ed1a958a2383861e242f14d7`, `packages/daisyui/test/DaisyUIForm.test.tsx:108` | Round 3 false: optional-chain element result |
| `rjsf.test.tsx`, LayoutGridField test | Same commit, `packages/core/test/LayoutGridField.test.tsx:1396` | Round 2 wording: valid absence-only finding, intended empty render |
| `carbon.test.jsx`, Dialog test | `carbon-design-system/carbon` at `40f8d5cfaed733feefa2e7c39b8e9eea1795d97d`, `packages/react/src/components/Dialog/Dialog-test.js:47` | Round 3 false: nullable `firstChild` |
| `carbon.test.jsx`, AddSelect test | Same commit, `packages/react/src/components/AddSelect/__tests__/AddSelect-test.js:1587` | Round 1 true: absence-only queries |

Both projects license these excerpts under Apache-2.0. Their license texts are
included as `LICENSE-rjsf.md` and `LICENSE-carbon`. Carbon's copyright notice is
preserved in the fixture.
