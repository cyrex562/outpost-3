/**
 * Tauri IPC bridge — the single entry point for talking to the Rust backend
 * when the app is running as a desktop shell. When running in a plain browser
 * (dev server / web build), `isTauri` is false and callers should fall back
 * to HTTP + WebSocket.
 */

import type { Command, InterruptTier } from '@/types/commands'
import type { ColonyScreenData } from '@/types/screen'
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
    /** Gas-giant/asteroid-belt/cometary-belt/moon count overrides (issue #318). */
    minGasGiants?: number
    maxGasGiants?: number
    minAsteroidBelts?: number
    maxAsteroidBelts?: number
    minCometaryBelts?: number
    maxCometaryBelts?: number
    minGiantMoons?: number
    maxGiantMoons?: number
    maxRockyMoons?: number
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
    minGasGiants: genParams?.minGasGiants ?? null,
    maxGasGiants: genParams?.maxGasGiants ?? null,
    minAsteroidBelts: genParams?.minAsteroidBelts ?? null,
    maxAsteroidBelts: genParams?.maxAsteroidBelts ?? null,
    minCometaryBelts: genParams?.minCometaryBelts ?? null,
    maxCometaryBelts: genParams?.maxCometaryBelts ?? null,
    minGiantMoons: genParams?.minGiantMoons ?? null,
    maxGiantMoons: genParams?.maxGiantMoons ?? null,
    maxRockyMoons: genParams?.maxRockyMoons ?? null,
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
  /**
   * Starlight reaching this body, where Sol at 1 AU is `1` (issue #413).
   *
   * A moon reports its parent's, since its own `distance_au` is measured
   * from the planet. Optional so a payload from a backend predating the
   * field still parses.
   */
  insolation?: number
  /**
   * How vigorously bulk water moves here, `0`–`1` (issue #440) — what an
   * ocean current plant is worth on this body.
   *
   * Derived, not raw: `tidally_locked` and `rotation_period_hours` are on
   * this payload too, but a moon's dominant term is tidal forcing from its
   * parent, which cannot be computed frontend-side. Optional so a payload
   * from a backend predating the field still parses.
   */
  ocean_circulation?: number
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

/**
 * The generated star system's display name (e.g. `"Vega"`), used to label the
 * system map and its star. Returns `''` for a system that predates
 * seed-derived naming — callers fall back to a generic label.
 *
 * The REST route wraps the name in an object (`{ name }`) per convention while
 * the Tauri command returns the bare string, so the shapes are normalized here.
 */
export async function getSystemName(): Promise<string> {
  if (!isTauri) {
    const res = await fetchJson<{ name: string }>('/api/system-name')
    return res.name ?? ''
  }
  return invoke<string>('get_system_name')
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

/** One accumulated interrupt from a fast-forward run (issue #332). */
export interface DigestItem {
  tier: InterruptTier
  message: string
  colony_id: string | null
  acknowledged: boolean
}

/** The return-from-fast-forward triage payload (issue #332). */
export interface InterruptDigest {
  stopped_at_sol: number
  sols_requested: number
  halting_message: string | null
  halting_tier: InterruptTier | null
  items: DigestItem[]
}

/**
 * Read what happened during the last fast-forward run.
 *
 * The `fast_forward` command reports only that a run ended and why; the
 * accumulated below-threshold interrupts live here.
 */
export async function getInterruptDigest(): Promise<InterruptDigest> {
  if (!isTauri) return fetchJson<InterruptDigest>('/api/interrupt-digest')
  return invoke<InterruptDigest>('get_interrupt_digest')
}

export interface ColonizeTarget {
  body_id: string
  body_name: string
  kind: string
  distance_au: number
  /** Body habitability score (0-100), issue #183. */
  habitability: number
  /**
   * Starlight reaching this body, where Sol at 1 AU is `1` (issue #413).
   *
   * Shown in the wizard because it decides what solar power is worth here
   * (issue #415) — a landing-site input, so it belongs in the comparison made
   * before founding rather than being discovered after.
   */
  insolation?: number
  /**
   * How vigorously bulk water moves here, `0`–`1` (issue #440).
   *
   * In the wizard for the same reason as `insolation`: it decides what an
   * ocean current plant is worth on this body, and that is a landing-site
   * input rather than something to discover afterwards.
   */
  ocean_circulation?: number
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
  /**
   * Whether this building is part of the engine's default landing kit (issue
   * #317). The founding wizard pre-selects these so the recommended loadout
   * matches what the engine would place on its own.
   */
  starter_kit: boolean
  /**
   * Most instances one colony may have, or `null` for unlimited.
   *
   * `colony_hq` is capped at 1: it is the colony's administrative core, not a
   * utility block, and stacking several was a way to exploit its slot
   * efficiency rather than a real decision. The engine rejects the command
   * either way; this lets the build UI grey the option out instead of
   * offering a button that always errors.
   */
  max_instances: number | null
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
  /**
   * Geothermal gradient in `[0, 1]` (issue #412) — how shallow magma sits
   * beneath this hex. `1` is a hotspot; `0` is cold cratonic crust where
   * reaching heat means drilling deeper than anyone has.
   *
   * Optional so a payload from a backend predating the layer still parses.
   */
  geothermal_gradient?: number
  temperature: string
  /**
   * Fraction of this cell covered by water/ice, in `[0.0, 1.0]` (issue
   * #316). Optional so older snapshots/fixtures without the field still
   * type-check — `PlanetHexMap.vue` derives a fallback from `terrain` when
   * absent.
   */
  water_coverage?: number
  /**
   * Vegetation density in this cell, in `[0.0, 1.0]` (issue #316). Optional
   * for the same reason as `water_coverage` — `PlanetHexMap.vue` falls back
   * to a biome-derived approximation when absent.
   */
  vegetation_density?: number
  /**
   * Contamination severity in `[0.0, 1.0]` from waste overflow (issue
   * #387). `0.0`/absent is pristine — optional for the same fixture/
   * older-snapshot reason as `water_coverage`.
   */
  contamination?: number
  deposits: { commodity_id: string; richness: number }[]
  habitable: boolean
  suitability: number
  occupied_by: string | null
  /** Id of the colony occupying this cell, if any (persistent planet map, phase A1) — for link-through to `/colony/:id`. */
  occupant_colony_id: string | null
}

/** An infrastructure edge between two colonies (map/nav plan phase A3). */
export interface InfraEdge {
  from_colony_id: string
  to_colony_id: string
  /** `road` | `rail` | `pipeline` | `powerline`. */
  infra_type: string
  /** Cargo (or, for a powerline, power) throughput per turn, before tech modifiers. */
  throughput: number
  /** Construction cost (abstract resource units). */
  cost: number
  /** Fraction of throughput lost in transit, in `[0.0, 1.0]` (issue #383). */
  loss_pct: number
}

export interface PlanetMap {
  seed: number
  /** Column count of the map (wraps east-west). */
  width: number
  /** Row count of the map (`r = 0` / `r = height - 1` are the poles). */
  height: number
  hexes: PlanetHex[]
  /** Infrastructure edges connecting colony nodes (map/nav plan phase A3). */
  edges: InfraEdge[]
}

/**
 * A trade route in the planetary trade network (issue #363) —
 * infrastructure-linked or manually added alike; the two are
 * indistinguishable once created.
 */
export interface TradeRoute {
  id: string
  colony_a: string
  colony_b: string
  /** Maximum units per commodity that may transit per strategic turn. */
  throughput_cap: number
  /** Sols a convoy spends in transit between the two endpoints. */
  transit_sols: number
}

/**
 * Fetch every trade route in the planetary trade network (issue #363).
 * The read side of the trade-route UI — `add_trade_route`/`remove_trade_route`
 * commands (sent via `gameStore.sendCommand`) are the write side.
 */
export async function getTradeRoutes(): Promise<TradeRoute[]> {
  if (!isTauri) return fetchJson<TradeRoute[]>('/api/trade-routes')
  return invoke<TradeRoute[]>('get_trade_routes')
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
  kind:
    | 'input_short'
    | 'awaiting_upstream'
    | 'power_brownout'
    | 'labor_short'
    | 'maintenance_short'
    | 'deposit_short'
  commodity_id: string | null
  effective_scale: number
  /** How much more of `commodity_id` full output needed; `0` when not applicable. */
  deficit: number
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
  /**
   * Per-sol upkeep **as actually charged at this owner's site** — the body's
   * atmospheric-hazard multiplier is already applied (issue #438), so this is
   * what the stockpile really loses, not the authored figure.
   */
  maintenance: IngredientRow[]
  /** Atmosphere's multiplier on maintenance; `1` is nominal (issue #438). */
  maintenance_multiplier?: number
  /** Hazard responsible for an elevated upkeep, or `null` when it costs nothing extra. */
  maintenance_hazard?: string | null
  /** The recipe this building actually runs right now (active selection, or the deterministic default). */
  recipe: RecipeRow | null
  /** Every recipe authored for this building type (issue #166). Empty unless there's a real choice (more than one). */
  available_recipes: RecipeRow[]
  /** Recipes that always run alongside `recipe` every turn (concurrent/multi-function buildings, issue #272). */
  concurrent_recipes: RecipeRow[]
  /**
   * The building's production lines (issue #272) — prefer this over
   * `available_recipes` for a picker.
   *
   * `available_recipes` is a flat list of every selectable recipe, which only
   * reads correctly for a single-line building. For a multi-line one it shows
   * recipes from *different* lines as if they were alternatives, when in fact
   * all of them run at once.
   */
  lines: RecipeLineRow[]
  last_run: BuildingRunRow | null
}

/** One production line on a building (issue #272). */
export interface RecipeLineRow {
  /** Authored line name; `null` is the building's default line. */
  line: string | null
  /** True when the line always runs and offers no choice. */
  always_on: boolean
  /** Recipe currently running on this line. */
  selected_recipe_id: string
  /** Every recipe on this line. Length 1 means there is nothing to choose. */
  alternatives: RecipeRow[]
}

/**
 * The colony management screen bundle for one colony.
 *
 * Browser mode had no way to fetch this, so every panel driven by it — buildings,
 * stockpile, colony resources — rendered empty there. `/api/colony-screen/:id`
 * closes that gap (issue #307 stage 4).
 */
export async function getColonyScreen(colonyId: string): Promise<ColonyScreenData> {
  if (!isTauri) return fetchJson<ColonyScreenData>(`/api/colony-screen/${colonyId}`)
  const q = await query({ kind: 'colony_screen', colony_id: colonyId })
  if (q.kind !== 'colony_screen' || !q.data) {
    throw new Error(`unexpected query result for colony_screen: ${JSON.stringify(q)}`)
  }
  return q.data as ColonyScreenData
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

/** One tunable balance dial, for the live playtesting editor. */
export interface BalanceScalar {
  quantity: string
  value: number
  min: number
  max: number
}

/**
 * Fetch every tunable balance scalar and its current value.
 *
 * The list comes from the engine's canonical `TUNABLE` set, so a dial added
 * there shows up here without a frontend change.
 */
export async function getBalanceScalars(): Promise<BalanceScalar[]> {
  if (!isTauri) return fetchJson<BalanceScalar[]>('/api/balance-scalars')
  return invoke<BalanceScalar[]>('get_balance_scalars')
}

export async function getPlanetMap(): Promise<PlanetMap> {
  if (!isTauri) return fetchJson<PlanetMap>('/api/planet-map')
  return invoke<PlanetMap>('get_planet_map')
}

/**
 * Fetch a read-only, procedurally-generated surface preview for any system
 * body (planet or moon), whether or not a colony has been founded there.
 * Used by the system map's "View Surface" action (map/nav plan). The preview
 * carries terrain/biome/deposit metadata but no colonies or infrastructure.
 */
export async function getBodySurface(bodyId: string): Promise<PlanetMap> {
  if (!isTauri) return fetchJson<PlanetMap>(`/api/body-surface/${bodyId}`)
  return invoke<PlanetMap>('get_body_surface', { bodyId })
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
