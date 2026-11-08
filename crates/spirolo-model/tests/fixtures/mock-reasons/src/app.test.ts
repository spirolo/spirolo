import { it, expect, vi } from 'vitest'

vi.mock('pkg')
vi.mock('@/nowhere')
vi.mock('node:fs')

it('runs', () => {
  expect(1).toBe(1)
})
