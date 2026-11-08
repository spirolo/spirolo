import { describe, it, test, expect } from 'vitest'
const name = 'dyn'
describe('Widget', () => {
  it('renders', () => { expect({ a: 1 }).toMatchSnapshot('with hint') })
  it.each(['one', 'two'])('case %s', (v) => { expect(v).toMatchSnapshot() })
  it(`title ${name}`, () => { expect(2).toMatchSnapshot() })
  test.for([[1, 2]])('sum %i + %i', ([a, b]) => { expect(a + b).toMatchSnapshot() })
  it('a > b', () => { expect('gt').toMatchSnapshot() })
})
