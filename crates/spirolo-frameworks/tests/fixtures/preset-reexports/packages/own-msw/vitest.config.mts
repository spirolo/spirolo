import { defineConfig, mergeConfig } from 'vitest/config'
import baseConfig from '@fx/config-vitest/base'

export default mergeConfig(baseConfig, defineConfig({ test: { setupFiles: ['./setup.ts'] } }))
