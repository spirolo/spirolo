Real vitest 3.2.7 output for issue #131, not hand-shaped.

Each test file first also held `it('was removed long ago', …)`; `npx vitest run` wrote the
two `.snap` files verbatim. That test was then deleted from both files and
`CI=true npx vitest run` reported `Snapshots 2 obsolete`, `Tests 7 passed`: in each file
only `… > was removed long ago 1` is obsolete. In `widget.test.ts` the hint, the `.each`
rows, the `test.for` row, the template-literal title and the title containing ` > ` are
all live.

`src/suites.test.ts` (a `describe.each` suite and a template-literal `describe` title) was
added afterwards and run the same way: `npx vitest run` wrote its `.snap`, and
`CI=true npx vitest run src/suites.test.ts` reported no obsolete snapshot.
