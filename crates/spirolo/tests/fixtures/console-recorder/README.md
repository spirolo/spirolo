# Console recorder reproductions

These sources extend the reproductions reported in issue #394. They were executed
unchanged with Vitest 5.0.1 on Node 22, outside the corpus checkouts. The
`helper-*`, `caught-*`, `*-snapshot*`, `argument-property-live` and
`hook-local-property` cases answer the review of #394 and ran on Node 20.20.2. The
`import-export-all` and `import-commonjs` cases answer a later review of #394 and ran
the same way on Node 20.20.2. `cases.json` records each setup, its helper modules,
and the observed runner exit code.

Each project used `globals: true`, `setupFiles: ['./setup.js']`, one worker, and no
cache. The probe file ran a passing assertion first, then logged both a console
error and a warning and ran the same assertion. Exit 1 means the console probe
failed; exit 0 means both tests passed. Deferred callbacks use a 60 second timer
and `unref()` so they cannot become a synchronous hook failure.

The integration tests feed these exact sources to the real Spirolo binary through
a Jest setup project. No corpus source, verdict, or adjudicator is changed.
