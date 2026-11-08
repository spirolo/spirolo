import { describe, it, expect } from 'vitest'
describe('Plain', () => {
  it('renders', () => { expect('<h1/>').toMatchSnapshot() })
})
