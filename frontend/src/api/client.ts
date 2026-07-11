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

const BASE = ''

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(BASE + path, {
    headers: { 'Content-Type': 'application/json', ...init?.headers },
    ...init,
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
