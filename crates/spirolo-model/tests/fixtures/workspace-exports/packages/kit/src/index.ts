import { get } from '@x/http'
import { client } from '@x/http/client'

export const kit = () => get(client)
