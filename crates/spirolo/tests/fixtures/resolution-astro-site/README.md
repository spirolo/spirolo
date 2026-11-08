A small synthetic Astro project that reproduces a run checked before `npm ci`: its
`tsconfig.json` extends `astro/tsconfigs/strict`, which is not installed, so every import
under it is left unknown.

`project/` holds a `package.json` with the two dependencies the run reads (`astro`,
`vitest`), a `tsconfig.json`, a `vitest.config.ts`, one module (`src/lib/glossary.ts`) and
one test that imports it (`tests/content/glossary-index.test.ts`). The module and the test
are written for this fixture. `installed/astro/` holds what the install puts under
`node_modules/astro/` that resolution reads: `tsconfigs/base.json` and
`tsconfigs/strict.json` verbatim from astro 6.3.8 (MIT), and its `package.json` cut to the
`tsconfigs` exports.

Without `installed/` copied to `node_modules/`, `check --all` reports one orphan-test row for
`../../src/lib/glossary` and one untested-hotspot row for `src/lib/glossary.ts`, both marked
unreliable; with it, neither.
