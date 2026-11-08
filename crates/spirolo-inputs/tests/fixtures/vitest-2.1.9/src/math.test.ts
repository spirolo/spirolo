import { describe, expect, it } from 'vitest'

describe('math', () => {
  describe('add', () => {
    it('adds two numbers', () => {
      expect(1 + 1).toBe(2)
    })
    it('is wrong on purpose', () => {
      expect(1 + 1).toBe(3)
    })
  })
  it.skip('is parked', () => {
    expect(true).toBe(true)
  })
  it.todo('is not written yet')
})

it('sits at the top level', () => {
  expect(0).toBe(0)
})
