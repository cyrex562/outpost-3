/**
 * Tauri IPC backend — used when running as a desktop app.
 * Falls back to HTTP when running in browser (dev server / web build).
 */

import type { Snapshot } from './client'

// Tauri's invoke is injected at runtime by the desktop shell.
// We detect it by checking for the global __TAURI__ object.
const isTauri = typeof window !== 'undefined' && '__TAURI__' in window

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri) {
    throw new Error(`Tauri not available — use HTTP client instead`)
  }
  // @ts-expect-error — __TAURI__ is injected at runtime
  return window.__TAURI__.core.invoke(cmd, args ?? {}) as Promise<T>
}

export const tauriApi = {
  isTauri,

  async newGame(contentDir: string, planetSeed: number, difficulty: string): Promise<Snapshot> {
    return invoke('new_game', { content_dir: contentDir, planet_seed: planetSeed, difficulty })
  },

  async advanceSol(): Promise<Snapshot> {
    return invoke('advance_sol')
  },

  async foundColony(name: string): Promise<Snapshot> {
    return invoke('found_colony', { name })
  },

  async queueConstruction(colonyId: string, buildingId: string): Promise<Snapshot> {
    return invoke('queue_construction', { colony_id: colonyId, building_id: buildingId })
  },

  async assignLabour(colonyId: string, buildingId: string, amount: number): Promise<Snapshot> {
    return invoke('assign_labour', { colony_id: colonyId, building_id: buildingId, amount })
  },

  async enqueueResearch(techId: string): Promise<Snapshot> {
    return invoke('enqueue_research', { tech_id: techId })
  },

  async getSnapshot(): Promise<Snapshot> {
    return invoke('get_snapshot')
  },

  async saveGame(path: string): Promise<void> {
    return invoke('save_game', { path })
  },

  async loadGame(path: string): Promise<Snapshot> {
    return invoke('load_game', { path })
  },
}
