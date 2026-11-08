`base-rows.json` holds five findings (weak-assertion, fail-closed-setup, orphan-test,
flake-candidate and untested-hotspot) and the model rows they need, in the shape of an
`spirolo check --format json` capture. Project names, package names and paths are neutral
placeholders.

The tests clone these rows into hand-built populations to exercise the formula-v1
boundaries.

`rule-status.json` is a verbatim `spirolo rule-status --format json` capture (five rules lean
true, none proven). The score tests pass this table explicitly, so future corpus status
refreshes do not change their expectations.

`handler-drift-reach/` is a small project written for the handler-drift score test:
`src/mocks/handlers/orders.ts` with `BASE = "*/api/v1"`, `mocks/server.ts`, `test/setup.ts`,
an MSW spec importing the server, a spec with a drifted inline override and one that loads
no handler. `spirolo check` on it with `urlSources` set to
`http://localhost:5000/${API_VERSION}` and `API_VERSION=v1` reports two decided
handler-drift rows, both with no test path; the score test builds the model and runs the
rule the same way.
