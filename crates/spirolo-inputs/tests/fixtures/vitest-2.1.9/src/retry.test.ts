import { describe, expect, it } from 'vitest'

let attempts = 0

describe('retry', () => {
  it('passes on the second attempt', () => {
    attempts += 1
    expect(attempts).toBeGreaterThan(1)
  })
})
