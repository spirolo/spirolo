import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    include: ['**/*.msw.{ts,tsx}'],
    setupFiles: ['./msw-setup.ts'],
  },
})
