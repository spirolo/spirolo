import { describe, it, expect } from 'vitest'
import { ROWS, PACK, LEXICAL, ALIASES } from './data'
import { compute } from './compute'

describe('imported tables', () => {
  it.each(ROWS)('handles %s', (locale) => { expect(locale).toBeTruthy() })

  for (const rule of PACK.phrases) {
    it(`flags ${rule.label}`, () => { expect(rule.label).toBeTruthy() })
  }

  it.each(LEXICAL.map((c) => [c.id, c] as const))('%s', (_id, c) => { expect(c).toBeTruthy() })

  it.each(Object.keys(ALIASES))('maps %s', (alias) => { expect(alias).toBeTruthy() })
})

describe('local tables', () => {
  const local = [1, 2, 3] as const
  it.each(local)('local %i', (n) => { expect(n).toBeGreaterThan(0) })

  const typed: Array<[string, number]> = [['a', 1], ['b', 2]]
  for (const [name, value] of typed) {
    it(`${name} is ${value}`, () => { expect(value).toBeGreaterThan(0) })
  }

  ;['x', 'y'].forEach((letter) => {
    it(`letter ${letter}`, () => { expect(letter).toBeTruthy() })
  })
})

describe.each(ROWS)('suite for %s', (locale) => {
  it('inherits the suite table, not a second copy of it', () => { expect(locale).toBeTruthy() })
})

describe('an uncountable table', () => {
  const rows = compute()
  it.each(rows)('runtime %s', (row) => { expect(row).toBeTruthy() })
})
