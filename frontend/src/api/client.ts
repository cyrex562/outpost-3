/**
 * Typed HTTP client wrappers for the Outpost 3 Axum REST endpoints.
 *
 * All functions return `Promise<T>` and throw on HTTP error responses.
 * The WebSocket channel handles live state updates; these REST helpers
 * are used for session management and one-off command dispatch.
 */

import type { Command } from '@/types/commands'
import type { GameEvent } from '@/types/gameEvents'
import type { ColonyScreenData } from '@/types/screen'
import { clientId } from './clientId'

const BASE = ''

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(BASE + path, {
    // `...init` first: spreading it *after* `headers` let a caller-supplied
    // `headers` replace the merged object wholesale rather than extend it,
    // silently dropping `Content-Type` and getting the request rejected. No
    // caller passed headers until issue #452 needed one, so the bug had never
    // fired.
    ...init,
    headers: { 'Content-Type': 'application/json', ...init?.headers },
  })
  if (!res.ok) {
    const body = await res.text()
    throw new Error(`HTTP ${res.status}: ${body}`)
  }
  return res.json() as Promise<T>
}

// ─── Session endpoints ────────────────────────────────────────────────────────

/** Create a new game session. Returns the new session ID. */
export async function createSession(): Promise<string> {
  const data = await request<{ session_id: string }>('/sessions', { method: 'POST' })
  return data.session_id
}

/** Delete a session by ID. */
export async function deleteSession(sessionId: string): Promise<void> {
  await request<void>(`/sessions/${sessionId}`, { method: 'DELETE' })
}

/** Apply a command to the session engine. Returns the events produced. */
export async function applyCommand(sessionId: string, cmd: Command): Promise<GameEvent[]> {
  return request<GameEvent[]>(`/sessions/${sessionId}/commands`, {
    method: 'POST',
    body: JSON.stringify(cmd),
  })
}

/**
 * Apply a command to the shared browser-mode engine (issue #220) — the same
 * engine the WebSocket `new_game` flow bootstraps content/planet/colony
 * onto. Returns the events produced.
 */
export async function applySharedCommand(cmd: Command): Promise<GameEvent[]> {
  return request<GameEvent[]>('/api/command', {
    method: 'POST',
    // Identifies this tab so the server does not broadcast these same events
    // back to it over the WebSocket — they are in this response already, and
    // receiving both is what made every command-issued event log twice
    // (issue #452).
    headers: { 'X-Client-Id': clientId() },
    body: JSON.stringify(cmd),
  })
}

/** Get the full game state snapshot for a session. */
export async function getSessionState(sessionId: string): Promise<unknown> {
  return request<unknown>(`/sessions/${sessionId}/state`)
}

// ─── Colony endpoints ─────────────────────────────────────────────────────────

/** List all colonies in the shared engine. */
export async function listColonies(): Promise<{ id: string; name: string; population: number }[]> {
  const result = await request<{ colonies: { id: string; name: string; population: number }[] }>(
    '/api/colonies',
  )
  return result.colonies ?? []
}

/** Get the current sol counter from the shared engine. */
export async function getCurrentSol(): Promise<number> {
  const result = await request<{ value: number }>('/api/sol')
  return result.value ?? 0
}

// ─── WebSocket query helpers (sent through the WS, typed here for reference) ─

/** Build a ColonyScreen query message. */
export function buildColonyScreenQuery(
  colonyId: string,
  seq: number,
): { type: 'query'; seq: number; query: { kind: 'colony_screen'; colony_id: string } } {
  return {
    type: 'query',
    seq,
    query: { kind: 'colony_screen', colony_id: colonyId },
  }
}

export type { ColonyScreenData }

// ─── Snapshot type (shared with Tauri IPC) ───────────────────────────────────

export interface ColonySummary {
  id: string
  name: string
  sol: number
  population: number
  stability: number
  commodities: Record<string, number>
}

export interface Snapshot {
  sol: number
  month: number
  colonies: ColonySummary[]
  events: string[]
}

// ─── httpApi adapter (mirrors tauriApi shape) ─────────────────────────────────

export const httpApi = {
  isTauri: false as const,

  async newGame(): Promise<Snapshot> {
    const sessionId = await createSession()
    sessionStorage.setItem('session_id', sessionId)
    return { sol: 0, month: 0, colonies: [], events: [] }
  },

  async advanceSol(): Promise<Snapshot> {
    const sessionId = sessionStorage.getItem('session_id') ?? ''
    const events = await applyCommand(sessionId, { type: 'AdvanceColonySol' } as never)
    const state = (await getSessionState(sessionId)) as Snapshot | null
    return state ?? { sol: 0, month: 0, colonies: [], events: events.map((e) => JSON.stringify(e)) }
  },

  async getSnapshot(): Promise<Snapshot> {
    const sessionId = sessionStorage.getItem('session_id') ?? ''
    const state = (await getSessionState(sessionId)) as Snapshot | null
    return state ?? { sol: 0, month: 0, colonies: [], events: [] }
  },

  async saveGame(path: string): Promise<void> {
    const sessionId = sessionStorage.getItem('session_id') ?? ''
    await request<void>(`/sessions/${sessionId}/save`, { method: 'POST', body: JSON.stringify({ path }) })
  },

  async loadGame(path: string): Promise<Snapshot> {
    const sessionId = sessionStorage.getItem('session_id') ?? ''
    const state = (await request<Snapshot>(`/sessions/${sessionId}/load`, {
      method: 'POST',
      body: JSON.stringify({ path }),
    }))
    return state
  },
}
