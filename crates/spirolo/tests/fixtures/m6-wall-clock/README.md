# Wall-clock rows of the flake-candidate visit (#449)

Each file is copied verbatim, keeping its path in the repo, from the OSS dev entry it is named
after, at the commit its corpus manifest pins (the corpus is not part of the source snapshot).
They are parsed only. grommet's `package.json` (its Jest config) and setup file come along,
without which the reader does not see `DateTimeInput-test.tsx` as a test.

- Elapsed budgets, read true: tududi `subtasks-completion.test.js:580` (round 2, row 1),
  react-hook-form `performance.test.tsx:373` (non-pack), fumadocs
  `packages/image-size/test/index.test.ts:225` (round 1, row 8).
- Calendar re-reads, read true: grommet `DateTimeInput-test.tsx:597` (round 2, row 10),
  react-day-picker `examples/InvalidMonthDefaultLimit.test.tsx:8` (round 1, row 11).
- A single read compared to itself, read false: rjsf `StringField.test.tsx:1972` (round 2,
  row 3), orpc `packages/ratelimit/src/handler-plugin.test.ts:15` (non-pack).
