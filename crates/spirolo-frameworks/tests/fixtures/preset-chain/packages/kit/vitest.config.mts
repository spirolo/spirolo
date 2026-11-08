import { defineConfig, mergeConfig } from 'vitest/config'
import reactConfig from '@fx/config-vitest/react'

export default mergeConfig(
  reactConfig,
  defineConfig({
    test: {
      setupFiles: ['./src/setup.ts'],
      exclude: ['**/*.unit.tsx'],
    },
  }),
)
