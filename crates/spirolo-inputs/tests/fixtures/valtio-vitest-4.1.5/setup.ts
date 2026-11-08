import '@testing-library/jest-dom/vitest'

if (process.env.SPIROLO_M6_LOAD_FAILURE) throw new Error('controlled M6 load failure')
beforeAll(() => {
  if (process.env.SPIROLO_M6_HOOK_FAILURE) throw new Error('controlled M6 hook failure')
})
