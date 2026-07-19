/**
 * API response shapes and WebSocket message types for Outpost 3.
 *
 * All server→client WebSocket messages are discriminated by a `type` field.
 * Client→server messages are discriminated by `type` and carry a `command`
 * or `query` sub-object discriminated by `kind`.
 */

import type { ServerEvent } from './events'

/** Difficulty preset matching the Rust `DifficultyPreset` enum. */
export type DifficultyPreset = 'Sandbox' | 'Easy' | 'Normal' | 'Hard' | 'Brutal'

// ─── Shared domain types ─────────────────────────────────────────────────────

/** Lightweight summary of a colony returned by ListColonies. */
export interface ColonySummary {
  id: string
  name: string
  population: number
  stability: number
  available_labour: number
  /** Commodity pool snapshot: [commodity_id, amount] pairs. */
  commodity_pool: [string, number][]
  buildings: string[]
  active_construction: string[]
}

/** Detailed colony status returned by ColonyStatus query. */
export interface ColonyStatus {
  id: string
  name: string
  population: number
  stability: number
  available_labour: number
}

/** Full world snapshot delivered on WebSocket connect. */
export interface WorldSnapshot {
  sol: number
  month: number
  colonies: ColonySummary[]
}

// ─── Server → Client messages ────────────────────────────────────────────────

export interface SnapshotMessage {
  type: 'snapshot'
  state: WorldSnapshot
}

export interface EventMessage {
  type: 'event'
  event: ServerEvent
}

export interface ErrorMessage {
  type: 'error'
  message: string
}

export interface AckMessage {
  type: 'ack'
  seq: number
}

export interface QueryResultMessage {
  type: 'query_result'
  seq: number
  result: QueryResultPayload
}

export type QueryResultPayload =
  | { kind: 'counter'; value: number }
  | { kind: 'colonies'; colonies: ColonySummary[] }
  | { kind: 'colony_status'; status: ColonyStatus }
  | { kind: 'research_total'; total: number }
  | { kind: 'labour'; labour: number }
  | { kind: 'colony_screen'; data: import('@/types/screen').ColonyScreenData }

export interface NewGameSnapshotMessage {
  type: 'new_game_snapshot'
  seq: number
  state: WorldSnapshot
}

/** Union of all server→client WebSocket messages. */
export type ServerMessage =
  | SnapshotMessage
  | EventMessage
  | ErrorMessage
  | AckMessage
  | QueryResultMessage
  | NewGameSnapshotMessage

// ─── Client → Server messages ────────────────────────────────────────────────

export type ClientCommand =
  | { kind: 'advance_sol' }
  | { kind: 'found_colony'; name: string; starting_population: number }
  | {
      kind: 'queue_construction'
      colony_id: string
      building_type: string
      slot_cost: number
      labor_per_turn: number
      construction_cost: [string, number][]
      construction_turns: number
    }
  | { kind: 'assign_labour'; colony_id: string; slot: string; labour: number }
  | {
      kind: 'new_game'
      difficulty: DifficultyPreset
      planet_seed: number
      /** Independent seed for star-system generation (issue #199); defaults to `planet_seed` server-side when omitted. */
      system_seed?: number
      /** Star-system generation tuning (playtest feedback: New Game sliders); each field defaults server-side when omitted. */
      habitable_zone_center_au?: number
      min_inner_planets?: number
      max_inner_planets?: number
      abundance_scalar?: number
    }

export type ClientQuery =
  | { kind: 'current_sol' }
  | { kind: 'current_month' }
  | { kind: 'list_colonies' }
  | { kind: 'colony_status'; colony_id: string }
  | { kind: 'colony_screen'; colony_id: string }

export interface ClientCommandMessage {
  type: 'command'
  seq: number
  command: ClientCommand
}

export interface ClientQueryMessage {
  type: 'query'
  seq: number
  query: ClientQuery
}

export type ClientMessage = ClientCommandMessage | ClientQueryMessage
