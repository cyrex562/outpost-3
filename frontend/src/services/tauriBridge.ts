/**
 * Tauri IPC bridge — the single entry point for talking to the Rust backend
 * when the app is running as a desktop shell. When running in a plain browser
 * (dev server / web build), `isTauri` is false and callers should fall back
 * to HTTP + WebSocket.
 */

import type { Command } from '@/types/commands'
import type { GameEvent } from '@/types/gameEvents'

const globalWindow = typeof window === 'undefined' ? null : (window as unknown as { __TAURI_INTERNALS__?: unknown })
export const isTauri = globalWindow !== null && '__TAURI_INTERNALS__' in globalWindow

type InvokeFn = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>

async function invokeFn<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const mod = await import('@tauri-apps/api/core')
  return mod.invoke<T>(cmd, args ?? {})
}

const invoke: InvokeFn = invokeFn

// ── Payload types (mirror commands.rs) ────────────────────────────────────────

export interface ColonyWire {
  id: string
  name: string
  population: number
}

export interface SnapshotPayload {
  sol: number
  month: number
  colonies: ColonyWire[]
  research_total: number
}

/** A raw query result — shape varies by `kind`. */
export interface QueryResult {
  kind: string
  [key: string]: unknown
}

// ── Command translation ──────────────────────────────────────────────────────

/** Translate a frontend `Command` into the payload the Rust `apply_command` accepts. */
function translateCommand(cmd: Command): Record<string, unknown> {
  // The Rust side deserializes the same discriminated-union format the frontend
  // already emits (`{ kind: 'advance_sol', ... }`), so we forward as-is EXCEPT
  // that `research_tech` → `research_tech` maps to a core variant of the same
  // name, and `advance_sol` → `advance_sol` is the flat AdvanceColonySol variant.
  return cmd as unknown as Record<string, unknown>
}

// ── Public API ───────────────────────────────────────────────────────────────

export async function bootstrap(
  contentDir: string,
  planetSeed: number,
  difficulty: string,
): Promise<SnapshotPayload> {
  return invoke<SnapshotPayload>('bootstrap', {
    contentDir,
    planetSeed,
    difficulty,
  })
}

export async function isReady(): Promise<boolean> {
  return invoke<boolean>('is_ready')
}

export async function snapshot(): Promise<SnapshotPayload> {
  return invoke<SnapshotPayload>('snapshot')
}

export async function apply(command: Command): Promise<GameEvent[]> {
  return invoke<GameEvent[]>('apply_command', {
    command: translateCommand(command),
  })
}

export async function query(q: { kind: string; [key: string]: unknown }): Promise<QueryResult> {
  return invoke<QueryResult>('run_query', { query: q })
}

export async function resetEngine(): Promise<void> {
  return invoke<void>('reset_engine')
}

export async function saveGame(path: string): Promise<void> {
  return invoke<void>('save_game', { path })
}

export async function loadGame(path: string): Promise<SnapshotPayload> {
  return invoke<SnapshotPayload>('load_game', { path })
}

export async function listSaves(dir: string): Promise<string[]> {
  return invoke<string[]>('list_saves', { dir })
}

// ── Custom, high-level queries ───────────────────────────────────────────────

export interface SystemBody {
  id: string
  name: string
  kind: string
  role: string
  distance_au: number
  colonizable: boolean
}

export async function getSystemBodies(): Promise<SystemBody[]> {
  return invoke<SystemBody[]>('get_system_bodies')
}

export interface TechNode {
  id: string
  name: string
  category: string
  description: string
  cost: number
  prerequisites: string[]
  state: 'researched' | 'in_progress' | 'available' | 'locked'
  progress: number
}

export async function getTechTree(): Promise<TechNode[]> {
  return invoke<TechNode[]>('get_tech_tree')
}

export interface ColonizeTarget {
  body_id: string
  body_name: string
  kind: string
  distance_au: number
}

export async function getColonizeTargets(): Promise<ColonizeTarget[]> {
  return invoke<ColonizeTarget[]>('get_colonize_targets')
}

export interface BuildingOption {
  id: string
  name: string
  description: string
  category: string
  slot_cost: number
  labor_per_turn: number
  construction_turns: number
  construction_cost: [string, number][]
  tech_prerequisite: string | null
}

export async function listBuildings(): Promise<BuildingOption[]> {
  return invoke<BuildingOption[]>('list_buildings')
}

export interface PlanetHex {
  q: number
  r: number
  site_id: string
  terrain: string
  biome: string
  deposits: { commodity_id: string; richness: number }[]
  habitable: boolean
  suitability: number
  occupied_by: string | null
}

export interface PlanetMap {
  seed: number
  radius: number
  hexes: PlanetHex[]
}

export async function getPlanetMap(): Promise<PlanetMap> {
  return invoke<PlanetMap>('get_planet_map')
}
