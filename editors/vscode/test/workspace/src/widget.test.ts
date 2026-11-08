import { it, expect, vi } from 'vitest'
import { widget } from './widget'
vi.mock('./missing')
it('renders the widget', () => { expect(widget()).toBeTruthy() })
it.skip('later', () => { expect(widget()).toBe(1) })
