/**
 * UI view-model types for the colony management screen.
 *
 * These mirror the Rust `ui::ColonyScreenData` struct and its sub-types
 * returned by the `Query::ColonyScreen` WebSocket query.
 */

/** A single building row shown on the colony screen. */
export interface BuildingRow {
  building_type: string
  labour_assigned: number
  slot_cost: number
  full_capacity: boolean
}

/** A single commodity row in the stockpile table. */
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
  buildings: BuildingRow[]
  stockpile: StockpileRow[]
  construction_queue: ConstructionQueueRow[]
  manual_override: boolean
}
