import { writable } from 'svelte/store'
import { api } from './api'
import type { Info } from './api'

export const serverInfo = writable<Info|undefined>(undefined)

export async function reloadServerInfo (): Promise<void> {
    serverInfo.set(await api.getInfo())
}
