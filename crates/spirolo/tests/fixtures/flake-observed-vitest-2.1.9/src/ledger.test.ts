import { describe, expect, it } from 'vitest'

// SPIROLO_FLIP is set on the second run only, so one test passes then fails.
describe('ledger', () => {
  it('flips between runs', () => {
    expect(process.env.SPIROLO_FLIP).toBeUndefined()
  })
  it('is always red', () => {
    expect(1 + 1).toBe(3)
  })
  it('is always green', () => {
    expect(1 + 1).toBe(2)
  })
  it.skipIf(process.env.SPIROLO_FLIP)('is red or skipped', () => {
    expect(1 + 1).toBe(3)
  })
})
