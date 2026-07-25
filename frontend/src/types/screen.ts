/**
 * UI view-model types for the colony management screen.
 *
 * These mirror the Rust `ui::ColonyScreenData` struct and its sub-types
 * returned by the `Query::ColonyScreen` WebSocket query.
 */

/** A single building row shown on the colony screen. */
export interface BuildingRow {
  building_type: string
  /**
   * Always 0 — per-building labour assignment has no backing state yet
   * (production is gated by a colony-wide labour ratio). Do not infer working
   * status from this; use `scale` (issue #303). Real assignment lands with
   * automatic labour assignment (#307).
   */
  labour_assigned: number
  slot_cost: number
  full_capacity: boolean
  /** Production scale achieved last turn, 0.0–1.0. `0` = produced nothing. */
  scale: number
  /** Why output fell short last turn, if it did (e.g. `"input short: water"`). */
  shortfall_reason: string | null
  /** Building has only always-on recipes, so there is no recipe to choose. */
  always_on: boolean
  /**
   * Ids of every recipe this building actually runs — the resolved pick-one
   * recipe plus all always-on ones (issue #272).
   */
  running_recipe_ids: string[]
  /** Commodities consumed per cycle, summed across every running recipe. */
  inputs: IngredientRow[]
  /**
   * Commodities produced per cycle, summed across every running recipe
   * (issue #272) — the "producing power + water + oxygen" line that makes a
   * consolidated building legible at a glance.
   */
  outputs: IngredientRow[]
}

/** A commodity id + per-cycle quantity pair. */
export interface IngredientRow {
  commodity_id: string
  quantity: number
}

/**
 * A single **tradeable** commodity row in the stockpile table.
 *
 * Since issue #304 this never includes power, housing, or research — those are
 * colony resources (see `ResourceRow`) and are structurally unshippable.
 */
export interface StockpileRow {
  /** Content-pack commodity identifier. */
  commodity_id: string
  /** Current amount in the colony pool. */
  amount: number
  /** Maximum storable amount (null = unlimited). */
  capacity: number | null
  /** Net change last turn (positive = surplus, negative = deficit). */
  net_per_turn: number
}

/**
 * A colony-local resource row (issue #304).
 *
 * Not a commodity: these are produced and consumed in place and can never be
 * traded or shipped. There is no capacity or net/turn because the amount is
 * this sol's throughput (or standing capacity), cleared before the next sol.
 */
export interface ResourceRow {
  resource_id: string
  /** Display name from the content pack. */
  name: string
  /** Amount produced/available this sol. */
  amount: number
  /** `'flow'` = surplus is lost each sol; `'capacity'` = standing capability. */
  kind: 'flow' | 'capacity' | string
  /** Unit label for display (`'MW'`, `'slots'`, `'RP'`). */
  unit: string
}

/** A single in-progress construction project. */
export interface ConstructionQueueRow {
  project_id: string
  building_type: string
  turns_completed: number
  turns_total: number
  slot_cost: number
}

/** Complete data bundle for the colony management screen. */
export interface ColonyScreenData {
  colony_id: string
  name: string
  population: number
  stability: number
  slots_used: number
  slot_capacity: number
  labour_available: number
  labour_total: number
  /** Worker slots the colony's operational buildings are asking for. */
  labour_demanded: number
  /** Workforce taken up by those jobs: min(demanded, available). */
  labour_employed: number
  /** Workforce with no job to go to: available - employed. */
  labour_unemployed: number
  /**
   * Colony-local resources this sol (issue #304). Disjoint from `stockpile`,
   * which is tradeable cargo only.
   */
  resources: ResourceRow[]
  buildings: BuildingRow[]
  stockpile: StockpileRow[]
  construction_queue: ConstructionQueueRow[]
  manual_override: boolean
}
