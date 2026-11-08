import { describe, it, expect } from 'vitest'
const area = 'north'
describe.each(['alpha'])('Suite %s', () => {
  it('renders', () => { expect(1).toMatchSnapshot() })
})
describe(`Area ${area}`, () => {
  it('renders', () => { expect(2).toMatchSnapshot() })
})
