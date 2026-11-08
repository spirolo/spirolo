import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    include: ['**/*.unit.{ts,tsx}'],
    exclude: ['**/node_modules/**'],
    setupFiles: ['./setup.ts'],
  },
})
