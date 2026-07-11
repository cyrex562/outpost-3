/**
 * Typed server→client event contract for Outpost 3.
 *
 * These types mirror the Rust `ServerEvent` enum in `outpost_web::wsmsg`.
 * The discriminant field is `kind` (snake_case), matching the Rust serde tag.
 *
 * IMPORTANT: this file is the single source of truth for the client-side
 * event shape.  Never use `any` — add a new variant here if the server
 * gains a new event type.
 */

export interface ColonySolAdvancedEvent {
  kind: 'colony_sol_advanced'
  sol: number
}

export interface StrategicMonthAdvancedEvent {
  kind: 'strategic_month_advanced'
  month: number
}

export interface ColonyFoundedEvent {
  kind: 'colony_founded'
  colony_id: string
  name: string
  starting_population: number
}

export interface ConstructionQueuedEvent {
  kind: 'construction_queued'
  colony_id: string
  building_type: string
  project_id: string
}

export interface ConstructionCancelledEvent {
  kind: 'construction_cancelled'
  colony_id: string
  project_id: string
  refund: [string, number][]
}

export interface BuildingConstructedEvent {
  kind: 'building_constructed'
  colony_id: string
  building_type: string
}

export interface LabourAssignedEvent {
  kind: 'labour_assigned'
  colony_id: string
  slot: string
  labour: number
}

export interface NeedsResolvedEvent {
  kind: 'needs_resolved'
  colony_id: string
  composite_satisfaction: number
  stability_delta: number
  population_delta: number
}

export interface ResearchProducedEvent {
  kind: 'research_produced'
  colony_id: string
  amount: number
}

export interface DirectiveSetEvent {
  kind: 'directive_set'
  colony_id: string
  directive_id: string
}

export interface DirectiveRemovedEvent {
  kind: 'directive_removed'
  directive_id: string
}

export interface ManualOverrideChangedEvent {
  kind: 'manual_override_changed'
  colony_id: string
  enabled: boolean
}

export interface DirectiveFiredEvent {
  kind: 'directive_fired'
  colony_id: string
  directive_id: string
}

export interface ProductionShortfallEvent {
  kind: 'production_shortfall'
  colony_id: string
  building_type: string
  scale: number
  reason: string
}

export interface HazardOccurredEvent {
  kind: 'hazard_occurred'
  colony_id: string
  /** Hazard category, e.g. "DustStorm" */
  hazard_kind: string
  severity: number
  stability_delta: number
  commodity_losses: [string, number][]
  population_lost: number
}

export interface MigrationArrivedEvent {
  kind: 'migration_arrived'
  from_colony: string | null
  to_colony: string
  count: number
  overcrowding_stability_penalty: number
  forced_departure_stability_penalty: number
}

export interface VoluntaryEmigrationTriggeredEvent {
  kind: 'voluntary_emigration_triggered'
  from_colony: string
  to_colony: string
  count: number
}

export interface ExpeditionLaunchedEvent {
  kind: 'expedition_launched'
  expedition_id: string
  colony_id: string
  target_hex_q: number
  target_hex_r: number
}

export interface ExpeditionArrivedEvent {
  kind: 'expedition_arrived'
  expedition_id: string
}

export interface ExpeditionReturnedEvent {
  kind: 'expedition_returned'
  expedition_id: string
  colony_id: string
  deposits: [string, number][]
}

export interface ExpeditionLostEvent {
  kind: 'expedition_lost'
  expedition_id: string
}

export interface TechUnlockedEvent {
  kind: 'tech_unlocked'
  tech_id: string
}

export interface VictoryAchievedEvent {
  kind: 'victory_achieved'
  /** Debug representation of the VictoryCondition variant, e.g. "InterstellarExpeditionLaunched" */
  condition: string
}

export interface MenaceCriticalEvent {
  kind: 'menace_critical'
  /** Menace category, e.g. "EnvironmentalCollapse" */
  menace_kind: string
  level: number
  countdown_months: number
}

export interface CargoDeliveredEvent {
  kind: 'cargo_delivered'
  shipment_id: string
  colony_id: string
  commodity_id: string
  amount: number
}

export interface OrbitalStationCompletedEvent {
  kind: 'orbital_station_completed'
  station_id: string
  colony_id: string
  /** Station type, e.g. "Habitat" */
  station_type: string
  /** Orbit band, e.g. "Low" */
  orbit_type: string
  blueprint_id: string
}

/** A core event that the frontend does not need to act on. */
export interface IgnoredEvent {
  kind: 'ignored'
}

/** Union of all typed server events. */
export type ServerEvent =
  | ColonySolAdvancedEvent
  | StrategicMonthAdvancedEvent
  | ColonyFoundedEvent
  | ConstructionQueuedEvent
  | ConstructionCancelledEvent
  | BuildingConstructedEvent
  | LabourAssignedEvent
  | NeedsResolvedEvent
  | ResearchProducedEvent
  | DirectiveSetEvent
  | DirectiveRemovedEvent
  | ManualOverrideChangedEvent
  | DirectiveFiredEvent
  | ProductionShortfallEvent
  | HazardOccurredEvent
  | MigrationArrivedEvent
  | VoluntaryEmigrationTriggeredEvent
  | ExpeditionLaunchedEvent
  | ExpeditionArrivedEvent
  | ExpeditionReturnedEvent
  | ExpeditionLostEvent
  | TechUnlockedEvent
  | VictoryAchievedEvent
  | MenaceCriticalEvent
  | CargoDeliveredEvent
  | OrbitalStationCompletedEvent
  | IgnoredEvent
