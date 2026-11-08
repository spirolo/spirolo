import { expect, test } from 'vitest'
import { cents } from './price'

test('rounds a whole amount', () => {
  expect(cents(1)).toBe(100)
})
