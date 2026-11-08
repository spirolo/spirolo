import { it, vi } from 'vitest'
import { query } from './db'
import { send } from '@/mail'

vi.mock('./db')
vi.mock('@/mail', () => ({ send: vi.fn() }))
vi.mock('axios')
vi.mock('./missing')

it('runs', () => { query(); send() })
