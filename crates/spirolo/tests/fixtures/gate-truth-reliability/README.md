# gate-truth reliability fixtures

Sources are pinned corpus checkouts, copied without execution. Package manifests retain
their original name and scripts; workflow files are unchanged unless noted.

- `react-native-web`: `necolas/react-native-web` at `f1b53224b242d9d8dad4b647e0b6f0395a0533ae`,
  `package.json` (name/scripts) and `.github/workflows/tests.yml`. Round 1 falsely
  reported no CI test step; `npm run unit` reaches Jest through `npm-run-all unit:*`.

- `react-native-web-lint`: same source, retaining only format, type-check and lint jobs.
- `gatsby-circleci`: `gatsbyjs/gatsby` at `8999a2ed4dd8b40bab6571f7d23c7d196ebdb83e`. Root scripts,
  stale GitHub workflow, and CircleCI aliases plus `unit_tests_node18` (other jobs omitted).

- `nivo-make`: `plouc/nivo` at `0dae2c32052a573f9f1d66ec1b453b572119b265`. Root scripts,
  CI `test` job and the `pkgs-test` Makefile target; other targets/jobs omitted.

- `nivo-cypress`: same Nivo source, retaining only the `end_to_end` job.
- `nivo-cypress-install`: same job, ending before `Cypress run`, retaining the
  `runTests: false` install action. Installing a runner must not count as running tests.

- `carbon-playwright`: `carbon-design-system/carbon` at `40f8d5cfaed733feefa2e7c39b8e9eea1795d97d`.
  Original root scripts, both Playwright configs and the CI `avt-runner` job.
  CI loads the root config; the web-components config is unused.

- `instantsearch-playwright`: `algolia/instantsearch` at `3892320b3aa642121c1b610441017f41ef947b2f`.
  Root/workspace scripts, e2e Playwright config and its CI job. The recorded retry
  finding stays when the config is selected through a named workspace and `-c`.

- `react-playground`: `facebook/react` at `7c6ac13e19fef500b7f669a16bbd01ecc95965ca`. Root/playground
  scripts, playground config and workflow, including its default working directory.

- `kaneo-playwright`: `usekaneo/kaneo` at `afe97be2473e0cd97fe4a63e526fef9ac4df85b0`. Root/e2e
  scripts, e2e config and browser-tests workflow. CI reaches the config with `pnpm --dir`.

- `react-devtools`: same React pin. Root/inline scripts, inline Playwright config,
  e2e workflow job and its Node wrapper. The wrapper spawns `yarn test:e2e` in the inline package.

- `tududi-local-playwright`: `chrisvel/tududi` at `1781c4c170b8e06f0d9d78e774bde9c95b3be1a8`.
  Root scripts, e2e config and CI workflow; CI runs lint/backend/build, not e2e.

Original upstream license and copyright notices are copied under `licenses/`, one
per source project. These fixtures retain their upstream licenses. The
extension.js notice is the upstream notice with the author's contact email
omitted; the name, year and URL are kept.

- `radix-playwright`: `radix-ui/primitives` at `f7ecd5ab16f5e1e820eb5786a1419a98a2d594ae` (the manifest pin).
  Root scripts, Playwright config and build workflow. Contrary to the round-1 review,
  `test:ci` calls `e2e`, so its retry finding stays.

- `extension-playwright`: `extension-js/extension.js` at `8143b84d990a50225a94ea6532acfa1e32c423e1`.
  Root name/scripts, root Playwright config, the `e2e` workflow and `scripts/run-e2e.mjs`.
  CI runs `xvfb-run -a timeout … pnpm test:e2e`; the wrapper spawns `pnpm exec playwright test`
  with `cwd` the repository root, so the root config's retries are a CI finding.
