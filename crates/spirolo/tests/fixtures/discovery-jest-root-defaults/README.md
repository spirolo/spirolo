Unmodified config, manifests and tests from midwayjs/midway at
bd306bb74b025168b4a7386cff78aacf7862a81e (MIT, LICENSE), except that
the `author` email lines of the two package manifests are removed so
the fixture passes the public snapshot's email check. Spirolo does not
read `author`.

runner-list.json is real Jest stdout captured on 2026-10-29 with
`node_modules/.bin/jest --listTests --json --runInBand --cache=false`
at the repository root, with only the absolute checkout prefix removed
and the paths sorted. It contains 304 files. This cut retains the two
hackernews tests that root invocation lists. Midway's real `npm test`
is `lerna run test`: each package runs Jest with its own config, and
packages/web ignores `<rootDir>/test/fixtures`, so the real run
collects neither. Spirolo follows the real run; the root listing is
the comparator's view, not the truth for this repository.

The api-bridge ESM smoke file is retained as a negative example:
Jest 29 defaults do not collect .mjs, and it is absent from the listing.
