/**
 * Renderer-agnostic world model for Outpost 3.
 *
 * This module defines the client-side world state that is derived entirely from
 * the server event stream.  It is intentionally free of Vue reactivity so it
 * can be used by any renderer (Vue, WebGL, native) without modification.
 *
 * The Pinia store owns a reactive copy of `WorldState`; this module provides
 * the plain types and the initial value.
 */

import type { ColonySummary } from '@/types/api'
import type { ServerEvent } from '@/types/events'

/** Per-colony mutable state. */
export interface ColonyState extends ColonySummary {
  stability: number
  available_labour: number
  buildings: string[]
  active_projects: ProjectState[]
}

/** A queued construction project. */
export interface ProjectState {
  project_id: string
  building_type: string
}

/** The full client-side world state. */
export interface WorldState {
  /** Current colony-sol counter. */
  sol: number
  /**
   * Derived calendar label (sol ÷ sols-per-month) — no longer a real cadence
   * since #333 unified everything onto the sol, and no longer shown in the
   * UI (issue #338). Kept only because the engine still emits it.
   */
  month: number
  /** All known colonies, keyed by colony UUID. */
  colonies: Record<string, ColonyState>
  /** System-wide accumulated research total. */
  research_total: number
  /** Alerts and notable events accumulated this session. */
  notifications: Notification[]
}

/** A player-facing notification derived from an engine event. */
export interface Notification {
  id: string
  tier: 'blocking' | 'urgent' | 'notable' | 'ambient'
  message: string
  colony_id?: string
  timestamp_sol: number
}

/**
 * One row in the colony log (colony details multi-window redesign's
 * events/alerts unification) — every server event gets exactly one entry,
 * not two competing lists. `tier`/`message` come straight from the
 * reducer's own curated `Notification` for the minority of event kinds it
 * classifies as alert-worthy; every other event still gets an entry here
 * (built by `worldStore`, not the reducer — see its `handleServerMessage`),
 * just tagged `'ambient'` with a generic message. `event` is the raw
 * `ServerEvent` this entry came from, carrying every kind-specific field —
 * the log's click-to-expand detail view reads it directly rather than
 * needing a bespoke detail template per event kind.
 */
export interface LogEntry extends Notification {
  event: ServerEvent
}

/** The initial (empty) world state before a snapshot arrives. */
export const EMPTY_WORLD_STATE: WorldState = {
  sol: 0,
  month: 0,
  colonies: {},
  research_total: 0,
  notifications: [],
}
