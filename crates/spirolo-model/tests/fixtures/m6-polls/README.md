# Polling source

`do-poll.ts` is the verbatim `pollUntil` function from the OSS dev entry
`expo__expo`, `apps/test-suite/tests/AppMetrics.ts`, at its manifest pin.
It returns when the awaited state meets a predicate; its clock comparison caps the wait.
The fixture is parsed, never executed.

`while-poll.ts` is the verbatim `waitForNangoConnection` function from dev entry
`fastrepl__anarlog`, `apps/desktop/src/imports/connected-import.ts`, at its manifest
pin. It returns the awaited connection and throws on its deadline.

`action-poll.ts` is copied verbatim from Expo's `packages/@expo/cli/src/utils/delay.ts`.
Its `do…while` reads state in the condition and breaks on a deadline in the body.

`retry-after.ts` is the verbatim `packages/client/src/plugins/retry-after.ts` from
OSS dev entry `middleapi__orpc`, commit
`63b20537dbb6ee91a4d68c6d81fa1f53ab114964`. Its `while (true)` retry loop exits on
response state, attempt count or a deadline computed through `elapsedTime` in the
body. oRPC's MIT licence is copied unchanged in `orpc.LICENSE`.
