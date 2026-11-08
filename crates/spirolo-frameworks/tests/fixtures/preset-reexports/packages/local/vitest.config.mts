import { defineConfig } from 'vitest/config'

const config = defineConfig({ test: { include: ['**/*.local.ts'] } })

export default config
