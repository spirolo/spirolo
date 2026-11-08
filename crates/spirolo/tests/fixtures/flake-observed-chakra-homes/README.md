# Chakra runner-home regression (#587)

Pinned source: chakra-ui/chakra-ui at
`961161428b8c59157ad921dd23303b73c294d73f`,
`packages/codemod/__tests__/as-props.test.ts` (test at line 507).
The source is trimmed to the recorded test and its enclosing suites.

Reports and sidecars are excerpts of the saved round-3 corpus run history,
`run-01-idle` and `run-02-load`. Each report retains the original file result
and selected assertion unchanged; unrelated file results and assertions are
removed. Counters and timing metadata remain as recorded. The root home
passes this test; the codemod home never loads its file. Neither home flips.

The `true/` fixture retains the recorded true row in `cross-file.test.ts`
(test at line 7) with its original source and enclosing suite, from
`run-01-idle` (passes in both homes) and `run-10-load` (times out in both).
It must remain one finding, with evidence from both comparable homes.

The source excerpts are MIT licensed; the upstream notice is retained in `LICENSE`.
