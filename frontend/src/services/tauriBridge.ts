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
  customScalars?: Record<string, number>,
  customMenaceEnabled?: boolean,
  customHazardsEnabled?: boolean,
  customMaintenanceEnabled?: boolean,
): Promise<SnapshotPayload> {
  return invoke<SnapshotPayload>('bootstrap', {
    contentDir,
    planetSeed,
    difficulty,
    customScalars: customScalars ?? null,
    customMenaceEnabled: customMenaceEnabled ?? null,
    customHazardsEnabled: customHazardsEnabled ?? null,
    customMaintenanceEnabled: customMaintenanceEnabled ?? null,
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

/** Terminate the desktop process. No-op in browser mode. */
export async function exitApp(): Promise<void> {
  if (!isTauri) return
  return invoke<void>('exit_app')
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
  /** Atmospheric thickness/density band (issue #197). */
  atmosphere_density: string
  /** Atmospheric chemical hazard band (issue #197). */
  atmosphere_hazard: string
  temperature: string
  gravity_g: number
  radiation: string
  habitability: number
  habitability_modifier: number
  /** Surface/composition archetype (issue #196) — flavor/authoring guidance, not a habitability input. */
  subtype: string
  tidally_locked: boolean
  axial_tilt_deg: number
  rotation_period_hours: number
  moon_count: number
  /** Display name of the body this one orbits, if any. */
  parent_body_name: string | null
}

export async function getSystemBodies(): Promise<SystemBody[]> {
  return invoke<SystemBody[]>('get_system_bodies')
}

export interface TechNode {
  id: string
  name: string
  category: string
  description: string
  tier: number
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
  /** Body habitability score (0-100), issue #183. */
  habitability: number
  /** Whether founding here is currently allowed (score clears the threshold, or the harsh-world capability is unlocked). */
  can_found: boolean
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
  elevation: number
  temperature: string
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

export interface SupplyPackage {
  id: string
  name: string
  description: string
  commodities: [string, number][]
}

export async function listSupplyPackages(): Promise<SupplyPackage[]> {
  return invoke<SupplyPackage[]>('list_supply_packages')
}

// ── #161 Custom difficulty ──────────────────────────────────────────────────

export interface DifficultyKnob {
  id: string
  label: string
  kind: 'slider' | 'toggle'
  step: number[]
  min: number
  max: number
  preset_default_at_current_preset: number
  current_value: number
}

export interface CustomPreset {
  name: string
  scalars: Record<string, number>
  menace_enabled: boolean
  hazards_enabled: boolean
  /** Master maintenance toggle (issue #180). Optional for pre-#180 preset files. */
  maintenance_enabled?: boolean
}

export async function getDifficultyKnobs(): Promise<DifficultyKnob[]> {
  return invoke<DifficultyKnob[]>('get_difficulty_knobs')
}

export async function listCustomPresets(): Promise<CustomPreset[]> {
  return invoke<CustomPreset[]>('list_custom_presets')
}

export async function saveCustomPreset(
  name: string,
  scalars: Record<string, number>,
  menaceEnabled: boolean,
  hazardsEnabled: boolean,
  maintenanceEnabled?: boolean,
): Promise<void> {
  return invoke<void>('save_custom_preset', {
    name,
    scalars,
    menaceEnabled,
    hazardsEnabled,
    maintenanceEnabled: maintenanceEnabled ?? null,
  })
}

export async function deleteCustomPreset(name: string): Promise<void> {
  return invoke<void>('delete_custom_preset', { name })
}

/** Fire `SetCustomDifficulty` on the engine in one round-trip. */
export async function setCustomDifficulty(
  scalars: Record<string, number>,
  menaceEnabled: boolean,
  hazardsEnabled: boolean,
  maintenanceEnabled?: boolean,
): Promise<GameEvent[]> {
  return invoke<GameEvent[]>('apply_command', {
    command: {
      kind: 'set_custom_difficulty',
      scalars,
      menace_enabled: menaceEnabled,
      hazards_enabled: hazardsEnabled,
      maintenance_enabled: maintenanceEnabled ?? null,
    },
  })
}
