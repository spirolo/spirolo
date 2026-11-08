import { defineConfig, mergeConfig } from 'vitest/config'

import baseConfig from './base.mjs'

export default mergeConfig(
  baseConfig,
  defineConfig({ test: { environment: 'jsdom' } }),
)
