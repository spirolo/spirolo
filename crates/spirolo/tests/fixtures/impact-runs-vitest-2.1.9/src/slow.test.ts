import { expect, test } from 'vitest'
import { cents } from './price'

test('rounds after a wait', async () => {
  await new Promise((resolve) => setTimeout(resolve, 300))
  expect(cents(2)).toBe(200)
})
