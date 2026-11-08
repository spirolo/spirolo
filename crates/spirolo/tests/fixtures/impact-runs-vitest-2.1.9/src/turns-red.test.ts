import { expect, test } from 'vitest'
import { cents } from './price'

test('rounds half a cent', async () => {
  await new Promise((resolve) => setTimeout(resolve, 150))
  expect(cents(0.005)).toBe(process.env.SPIROLO_RED ? 2 : 1)
})
