/**
 * Typed game event shapes returned by the REST command endpoint.
 *
 * These mirror the Rust `Event` enum variants serialised by serde.
 * The discriminant is `kind` (snake_case).
 */

export interface ColonySolAdvancedEvent {
  kind: 'colony_sol_advanced'
  sol: number
}

export interface ColonyFoundedEvent {
  kind: 'colony_founded'
  colony_id: string
  name: string
  starting_population: number
}

export interface StrategicMonthAdvancedEvent {
  kind: 'strategic_month_advanced'
  month: number
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

export interface OutpostEstablishedEvent {
  kind: 'outpost_established'
  outpost_id: string
  colony_id: string
  body_id: string
}

export interface OutpostDecommissionedEvent {
  kind: 'outpost_decommissioned'
  outpost_id: string
}

export interface OutpostConstructionQueuedEvent {
  kind: 'outpost_construction_queued'
  outpost_id: string
  building_type: string
  project_id: string
}

export interface OutpostPromotedEvent {
  kind: 'outpost_promoted'
  outpost_id: string
  colony_id: string
  name: string
}

export interface OtherEvent {
  kind: string
  [key: string]: unknown
}

/** An inter-colony trade convoy landed and was credited to a pool (issue #332). */
export interface TradeConvoyArrivedEvent {
  kind: 'trade_convoy_arrived'
  convoy_id: string
  from_colony: string
  to_colony: string
  commodity_id: string
  amount: number
}

/**
 * A fast-forward run finished (issue #332).
 *
 * The UI's cue to stop its play timer and, when `halted`, open the digest panel.
 */
export interface FastForwardEndedEvent {
  kind: 'fast_forward_ended'
  sol: number
  sols_advanced: number
  halted: boolean
  halting_reason: string | null
}

/** Union of all known game event types. */
export type GameEvent =
  | ColonySolAdvancedEvent
  | ColonyFoundedEvent
  | StrategicMonthAdvancedEvent
  | NeedsResolvedEvent
  | ResearchProducedEvent
  | DirectiveFiredEvent
  | ProductionShortfallEvent
  | OutpostEstablishedEvent
  | OutpostDecommissionedEvent
  | OutpostConstructionQueuedEvent
  | OutpostPromotedEvent
  | TradeConvoyArrivedEvent
  | FastForwardEndedEvent
  | OtherEvent
