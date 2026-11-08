# Named reset source

`agentService.test.ts` is copied verbatim from the OSS dev entry `hyperdxio__hyperdx`,
`packages/api/src/opamp/services/__tests__/agentService.test.ts`, at the commit the development
corpus manifest pins (the corpus is not part of the source snapshot). It is parsed only: that
workspace is never executed. Its named arrow callback resets two module-scope objects inside a
describe's beforeEach.

`useSidePanelStack.test.tsx` is copied verbatim from the same entry's
`packages/app/src/hooks/__tests__/useSidePanelStack.test.tsx`. Its callback is a
function declaration. This workspace is also parsed only, never executed.
