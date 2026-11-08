import { defineConfig } from 'vitest/config'

export default function make(overrides = {}) {
  return defineConfig({ test: { include: ['**/*.made.ts'], ...overrides } })
}
