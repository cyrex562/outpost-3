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

/**
 * Fetch a browser-mode REST endpoint against the shared `outpost_web`
 * engine. Used as the non-Tauri fallback for the founding wizard's
 * read-only data calls (issue #220) — see `outpost_web::query_routes`.
 */
async function fetchJson<T>(path: string): Promise<T> {
  const res = await fetch(path)
  if (!res.ok) {
    const body = await res.text().catch(() => '')
    throw new Error(`HTTP ${res.status} ${path}: ${body}`)
  }
  return res.json() as Promise<T>
}

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

/**
 * Forward an uncaught frontend error to the backend's log file — the
 * webview/browser has no error boundary of its own, so an exception during
 * a component's render/setup can leave the screen blank with nothing but
 * this call to explain it afterward. Wired up from `main.ts`'s global
 * handlers. Never throws — logging a failure must not itself crash further.
 */
export async function logFrontendError(source: string, message: string, stack?: string): Promise<void> {
  try {
    if (isTauri) {
      await invoke('log_frontend_error', { source, message, stack: stack ?? null })
    } else {
      await fetch('/api/log', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ source, message, stack: stack ?? null }),
      })
    }
  } catch {
    // best-effort — the backend log/HTTP endpoint may be unreachable
    // (e.g. before bootstrap); console still has the original error.
  }
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
  /** Independent seed for star-system generation (issue #199); defaults to `planetSeed` when omitted. */
  systemSeed?: number,
  /** Star-system generation tuning (playtest feedback: New Game sliders); each field defaults server-side when omitted. */
  genParams?: {
    habitableZoneCenterAu?: number
    minInnerPlanets?: number
    maxInnerPlanets?: number
    abundanceScalarOverride?: number
  },
): Promise<SnapshotPayload> {
  return invoke<SnapshotPayload>('bootstrap', {
    contentDir,
    planetSeed,
    difficulty,
    customScalars: customScalars ?? null,
    customMenaceEnabled: customMenaceEnabled ?? null,
    customHazardsEnabled: customHazardsEnabled ?? null,
    customMaintenanceEnabled: customMaintenanceEnabled ?? null,
    systemSeed: systemSeed ?? null,
    habitableZoneCenterAu: genParams?.habitableZoneCenterAu ?? null,
    minInnerPlanets: genParams?.minInnerPlanets ?? null,
    maxInnerPlanets: genParams?.maxInnerPlanets ?? null,
    abundanceScalarOverride: genParams?.abundanceScalarOverride ?? null,
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
  /** Habitability score after tech-driven mitigations are applied (issue #185); equals `habitability` when none apply. */
  habitability_effective: number
  /** Habitability modifier after tech-driven mitigations are applied (issue #185); equals `habitability_modifier` when none apply. */
  habitability_modifier_effective: number
  /** Surface/composition archetype (issue #196) — flavor/authoring guidance, not a habitability input. */
  subtype: string
  tidally_locked: boolean
  axial_tilt_deg: number
  rotation_period_hours: number
  moon_count: number
  /** Display name of the body this one orbits, if any. */
  parent_body_name: string | null
  /** Per-category production modifiers (issue #184) — category name to multiplier, e.g. `["PowerOutput", 1.35]`. Empty when unauthored. */
  category_modifiers: [string, number][]
  /** Density-zoned annulus profile for belt-kind bodies (system-screen fix B2). `null` for non-belt bodies. */
  belt_profile: BeltProfile | null
}

/** One angular zone of a belt's annulus (system-screen fix B2). */
export interface BeltZone {
  /** Angular start of the zone, in degrees `[0, 360)`. */
  start_deg: number
  /** Angular sweep of the zone, in degrees. */
  sweep_deg: number
  /** Fill density `[0, 1]` — drives the annulus fill opacity. */
  density: number
}

/** Radial/angular density profile of a belt, for annulus rendering (system-screen fix B2). */
export interface BeltProfile {
  /** Inner radius of the annulus, in AU. */
  inner_au: number
  /** Outer radius of the annulus, in AU. */
  outer_au: number
  /** Angular zones subdividing the annulus. */
  zones: BeltZone[]
}

export async function getSystemBodies(): Promise<SystemBody[]> {
  if (!isTauri) return fetchJson<SystemBody[]>('/api/system-bodies')
  return invoke<SystemBody[]>('get_system_bodies')
}

/** Mirrors `outpost_core::tech::TechEffect` (`#[serde(tag = "type", rename_all = "snake_case")]`). */
export type TechEffect =
  | { type: 'unlock_building'; building_id: string }
  | { type: 'unlock_commodity'; commodity_id: string }
  | { type: 'unlock_capability'; capability_id: string }
  | { type: 'bonus'; category: string; value: number }
  | { type: 'mitigate_attribute'; attribute: string }
  | { type: 'survey_modifier_bonus'; full_reveal_bonus: number; partial_reveal_bonus: number }
  | { type: 'reduce_transit_time'; fraction: number }
  | { type: 'extend_outpost_range'; bonus_au: number }

export interface TechNode {
  id: string
  name: string
  category: string
  description: string
  tier: number
  cost: number
  prerequisites: string[]
  state: 'researched' | 'in_progress' | 'queued' | 'available' | 'locked'
  progress: number
  effects: TechEffect[]
  /** Zero-based position in the actual FIFO research queue; only meaningful when `state === 'queued'`. */
  queue_position: number | null
}

export async function getTechTree(): Promise<TechNode[]> {
  if (!isTauri) return fetchJson<TechNode[]>('/api/tech-tree')
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
  if (!isTauri) return fetchJson<ColonizeTarget[]>('/api/colonize-targets')
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
  if (!isTauri) return fetchJson<BuildingOption[]>('/api/buildings')
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

export interface IngredientRow {
  commodity_id: string
  quantity: number
}

export interface RecipeRow {
  recipe_id: string
  name: string
  inputs: IngredientRow[]
  outputs: IngredientRow[]
  cycle_sols: number
}

export interface ShortfallRow {
  kind: 'input_short' | 'power_brownout' | 'labor_short' | 'maintenance_short'
  commodity_id: string | null
  effective_scale: number
}

export interface BuildingRunRow {
  scale: number
  is_full_production: boolean
  shortfalls: ShortfallRow[]
}

export interface BuildingDetail {
  building_type: string
  name: string
  description: string
  category: string
  slot_cost: number
  power_delta: number
  maintenance: IngredientRow[]
  /** The recipe this building actually runs right now (active selection, or the deterministic default). */
  recipe: RecipeRow | null
  /** Every recipe authored for this building type (issue #166). Empty unless there's a real choice (more than one). */
  available_recipes: RecipeRow[]
  /** Recipes that always run alongside `recipe` every turn (concurrent/multi-function buildings, issue #272). */
  concurrent_recipes: RecipeRow[]
  last_run: BuildingRunRow | null
}

/** Full detail for one building type within a colony (issue #182). */
export async function getBuildingDetail(colonyId: string, buildingType: string): Promise<BuildingDetail> {
  const q = await query({ kind: 'building_detail', colony_id: colonyId, building_type: buildingType })
  if (q.kind !== 'building_detail' || !q.data) {
    throw new Error(`unexpected query result for building_detail: ${JSON.stringify(q)}`)
  }
  return q.data as BuildingDetail
}

/** Select which recipe a building type runs in this colony (issue #166). */
export async function setActiveRecipe(colonyId: string, buildingType: string, recipeId: string): Promise<GameEvent[]> {
  return invoke<GameEvent[]>('apply_command', {
    command: {
      kind: 'set_active_recipe',
      colony_id: colonyId,
      building_type: buildingType,
      recipe_id: recipeId,
    },
  })
}

// ── Outposts (issue #233/#243) ──────────────────────────────────────────────

export interface Outpost {
  id: string
  name: string
  parent_colony_id: string
  body_id: string
  body_name: string
  slot_capacity: number
  slots_used: number
  buildings: string[]
  /** `[commodity_id, amount]` pairs — every commodity that has ever had a non-zero amount. */
  pool: [string, number][]
}

/** List every established outpost across all colonies. Frontend filters by `parent_colony_id`. */
export async function listOutposts(): Promise<Outpost[]> {
  if (!isTauri) return fetchJson<Outpost[]>('/api/outposts')
  return invoke<Outpost[]>('list_outposts')
}

/**
 * Full detail for one building type within an outpost (navigation rework #7
 * phase 4 — mirrors `getBuildingDetail` for colonies). Tauri-only for now,
 * same as `getBuildingDetail`/`setActiveRecipe` — browser mode has no
 * generic query endpoint yet; this is an existing gap, not new to outposts.
 */
export async function getOutpostBuildingDetail(outpostId: string, buildingType: string): Promise<BuildingDetail> {
  const q = await query({ kind: 'outpost_building_detail', outpost_id: outpostId, building_type: buildingType })
  if (q.kind !== 'building_detail' || !q.data) {
    throw new Error(`unexpected query result for outpost_building_detail: ${JSON.stringify(q)}`)
  }
  return q.data as BuildingDetail
}

/** Select which recipe a building type runs at an outpost (navigation rework #7 phase 4). */
export async function setOutpostActiveRecipe(
  outpostId: string,
  buildingType: string,
  recipeId: string,
): Promise<GameEvent[]> {
  return invoke<GameEvent[]>('apply_command', {
    command: {
      kind: 'set_outpost_active_recipe',
      outpost_id: outpostId,
      building_type: buildingType,
      recipe_id: recipeId,
    },
  })
}

export interface OutpostTarget {
  body_id: string
  body_name: string
  kind: string
  distance_au: number
  /** Distance from the parent colony's home body, in AU; `null` when the colony has no home body. */
  distance_from_home_au: number | null
  /** Whether `EstablishOutpost` would currently accept this body. */
  in_range: boolean
}

/** Bodies a given colony could establish an outpost on, annotated with range-gate status (issue #241). */
export async function getOutpostTargets(colonyId: string): Promise<OutpostTarget[]> {
  if (!isTauri) return fetchJson<OutpostTarget[]>(`/api/outpost-targets/${colonyId}`)
  return invoke<OutpostTarget[]>('get_outpost_targets', { colonyId })
}

export async function getPlanetMap(): Promise<PlanetMap> {
  if (!isTauri) return fetchJson<PlanetMap>('/api/planet-map')
  return invoke<PlanetMap>('get_planet_map')
}

export interface SupplyPackage {
  id: string
  name: string
  description: string
  commodities: [string, number][]
}

export async function listSupplyPackages(): Promise<SupplyPackage[]> {
  if (!isTauri) return fetchJson<SupplyPackage[]>('/api/supply-packages')
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
