import { defineConfig } from 'vitest/config'

// A function export: nothing about it is statically readable.
export default defineConfig(() => ({ test: { include: ['**/*.unit.ts'] } }))
