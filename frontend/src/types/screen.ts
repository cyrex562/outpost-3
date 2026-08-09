/**
 * UI view-model types for the colony management screen.
 *
 * These mirror the Rust `ui::ColonyScreenData` struct and its sub-types
 * returned by the `Query::ColonyScreen` WebSocket query.
 */

/** A single building row shown on the colony screen. */
export interface BuildingRow {
  /**
   * Stable id of this placed instance (issue #307) — what the per-building
   * commands are addressed to, and the right key for a `v-for`. Two buildings of
   * one type are distinct rows, so `building_type` is **not** unique.
   */
  building_id: string
  /** Display name: the player's, else `"<Type Name> <n>"`. */
  name: string
  building_type: string
  /**
   * Labour actually assigned last sol, read from the plan production used
   * (issue #307). Real now — it was hardcoded 0 before.
   *
   * Still not the "is it working?" signal: a fully-staffed building can be idle
   * for want of inputs. Use `scale` for that (issue #303).
   */
  labour_assigned: number
  /**
   * Workers wanted, gated on whether the building could run at all. Less than
   * `labour_assigned` is impossible; `labour_assigned < labour_demand` means
   * understaffed. `0` for a building with no jobs to offer.
   */
  labour_demand: number
  /** Staffing priority: 1 is staffed first, 9 last (issue #307). */
  priority: number
  /** Workers pinned by the player, or `null` when automatic (issue #307). */
  labour_lock: number | null
  /**
   * Whether this building is paused (issue #309) — excluded from production
   * entirely, so it draws no labour, power, or commodity inputs and produces
   * nothing, but still occupies its build slot.
   */
  paused: boolean
  slot_cost: number
  full_capacity: boolean
  /** Production scale achieved last turn, 0.0–1.0. `0` = produced nothing. */
  scale: number
  /** Why output fell short last turn, if it did (e.g. `"input short: water"`). */
  shortfall_reason: string | null
  /**
   * The same shortfall's machine-readable category (issue #308): one of
   * `input_short`, `awaiting_upstream`, `power_brownout`, `labor_short`,
   * `maintenance_short`, `deposit_short`.
   *
   * `awaiting_upstream` is **transient** — the input is produced in this colony
   * and the chain is still filling, which resolves on its own. Everything else
   * wants the player to do something. Styling them alike is what made a
   * brand-new production chain look broken.
   */
  shortfall_kind: string | null
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
  /**
   * Amount the player has withheld from industry (issue #308); `0` if none.
   *
   * A floor *within* `amount`, not a separate quantity — reserved stock is
   * included in `amount` and stays visible. Recipe inputs and maintenance cannot
   * draw below it; colonist needs still can.
   */
  reserved: number
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

/** One site requirement of one building, as it stands at a given colony. */
export interface SiteRequirementRow {
  building_type: string
  /** The condition, phrased as a requirement rather than a failure. */
  label: string
  met: boolean
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
  /** Morale scalar in [0, 1] (issue #382) — separate from stability. */
  morale: number
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
  /**
   * Site requirements for the buildings that declare any, already evaluated
   * against *this* colony's site (issue #410).
   *
   * Comes from the colony screen rather than the building catalogue because
   * the catalogue is a global, colony-agnostic list — "is there ocean nearby"
   * has no answer without knowing which colony is asking. Only buildings with
   * at least one authored requirement appear.
   */
  site_requirements?: SiteRequirementRow[]
  manual_override: boolean
}
