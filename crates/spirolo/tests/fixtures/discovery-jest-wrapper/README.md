Unmodified wrapper, configs, manifests and tests from react/react at
7c6ac13e19fef500b7f669a16bbd01ecc95965ca (MIT, LICENSE).

runner-list.json is the 328-file JSON listing from
`node scripts/jest/jest-cli.js --listTests --json --runInBand --cache=false`
captured on 2026-10-29. Only the wrapper log prefix and the absolute
checkout prefix were removed, and paths sorted.

This cut retains two listed tests, under scripts/shared and packages/react. The direct-Jest
guard and the devtools events test are negative examples: neither is in
the default wrapper listing. The wrapper selects config.source.js; its
config.base.js supplies testRegex, roots and process.cwd() as rootDir.

The CoffeeScript test is inventoried without static case extraction; its
body and the React manifest are also unmodified upstream files.
