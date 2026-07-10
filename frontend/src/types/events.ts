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
