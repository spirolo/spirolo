import { readFile } from 'node:fs/promises'
import path from 'path'

export const load = (file: string) => readFile(path.join('.', file))
