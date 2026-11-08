import { beforeAll } from 'vitest'
import { setupServer } from 'msw/node'
export const server = setupServer()
beforeAll(() => { server.listen() })
