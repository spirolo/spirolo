export const ROWS = ['en', 'fr', 'de'] as const

const openers = [{ label: 'a' }, { label: 'b' }]
const closers = [{ label: 'c' }]

export const PACK = {
  locale: 'fr',
  phrases: [...openers, ...closers, { label: 'd' }],
}

export const CASES = [
  { id: 'one', hard: true },
  { id: 'two' },
  { id: 'three', hard: false },
  { id: 'four', kind: 'x' },
]

export const LEXICAL = CASES.filter((c) => !c.hard)

export const ALIASES: Record<string, string> = { embed: 'inline', block: 'cta' }
