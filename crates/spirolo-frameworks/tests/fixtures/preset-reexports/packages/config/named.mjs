import { defineConfig } from 'vitest/config'

export const strict = defineConfig({
  test: { include: ['**/*.strict.ts'], restoreMocks: true },
})
