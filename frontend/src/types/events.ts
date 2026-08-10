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

/**
 * A colony founded through the planet-map site path — what the founding wizard
 * uses (issue #317). Carries the same fields as `colony_founded` plus the site.
 */
export interface ColonyFoundedAtSiteEvent {
  kind: 'colony_founded_at_site'
  colony_id: string
  name: string
  starting_population: number
  site_id: string
}

/**
 * A colony founding was launched and is in transit (issue #359).
 *
 * The founding wizard always names a sponsor colony, so foundings are
 * *deferred* — the colony arrives after a distance-derived delay rather than
 * appearing at once. Before this had a wire representation the event was
 * dropped, so the player clicked Found and got no confirmation of any kind,
 * then saw the colony appear some sols later with nothing having announced it
 * (issue #403).
 */
export interface ColonyFoundingLaunchedEvent {
  kind: 'colony_founding_launched'
  /** Pending-colony id, distinct from the eventual colony id. */
  pending_id: string
  name: string
  body_id: string
  /** Sols until it arrives. */
  sols_remaining: number
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

/** A site-preparation project widened the colony's build-slot capacity (#306). */
export interface SlotCapacityExpandedEvent {
  kind: 'slot_capacity_expanded'
  colony_id: string
  building_type: string
  added: number
  slot_capacity: number
}

/** An outpost's site preparation widened its build-slot capacity. */
export interface OutpostSlotCapacityExpandedEvent {
  kind: 'outpost_slot_capacity_expanded'
  outpost_id: string
  building_type: string
  added: number
  slot_capacity: number
}

/**
 * Construction blocked by the player's own commodity reserve (issue #355).
 *
 * Distinct from `construction_stalled` because the advice is opposite: the stock
 * is present, behind a floor the player set. A shortage message would send them
 * hunting for materials they already have.
 */
export interface ConstructionStalledByReserveEvent {
  kind: 'construction_stalled_by_reserve'
  colony_id: string
  project_id: string
  building_type: string
  /** Per-commodity amount the reserve withheld from this sol's instalment. */
  withheld: [string, number][]
}

/** Construction made no progress for want of materials (issue #306). */
export interface ConstructionStalledEvent {
  kind: 'construction_stalled'
  colony_id: string
  project_id: string
  building_type: string
  /** Per-commodity amount still needed to fund this sol. */
  missing: [string, number][]
}

/** An outpost's construction made no progress for want of materials. */
export interface OutpostConstructionStalledEvent {
  kind: 'outpost_construction_stalled'
  outpost_id: string
  project_id: string
  building_type: string
  missing: [string, number][]
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

/** A colony's home body was recorded (issue #163). Fired by the founding
 * wizard right after `found_colony`/`found_colony_at_site` whenever the
 * player picked a body — i.e. on nearly every real playthrough. This event
 * kind has its own typed `ServerEvent` variant on both wire layers
 * (`outpost_web`'s WS bridge and `outpost_tauri`'s IPC bridge) — it just
 * wasn't declared here, so a Tauri session forwarding it hit a frontend
 * type/reducer that couldn't recognise the `kind` string at all. */
export interface ColonyHomeBodySetEvent {
  kind: 'colony_home_body_set'
  colony_id: string
  body_id: string
  habitability_modifier: number
}

/** A building type's active (pick-one) recipe was changed (issue #166). */
export interface ActiveRecipeSetEvent {
  kind: 'active_recipe_set'
  colony_id: string
  building_type: string
  recipe_id: string
}

/** The active difficulty preset changed (issue #161). */
export interface DifficultyChangedEvent {
  kind: 'difficulty_changed'
  preset: string
}

/** A new outpost was established (issue #233/#243). */
export interface OutpostEstablishedEvent {
  kind: 'outpost_established'
  outpost_id: string
  colony_id: string
  body_id: string
}

/** An outpost was decommissioned (issue #233/#243). */
export interface OutpostDecommissionedEvent {
  kind: 'outpost_decommissioned'
  outpost_id: string
}

/** An outpost queued a construction project (issue #233/#243). */
export interface OutpostConstructionQueuedEvent {
  kind: 'outpost_construction_queued'
  outpost_id: string
  building_type: string
  project_id: string
}

/** An outpost was promoted into a full colony (issue #242/#243). */
export interface OutpostPromotedEvent {
  kind: 'outpost_promoted'
  outpost_id: string
  colony_id: string
  name: string
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
  | ColonyFoundedAtSiteEvent
  | ConstructionQueuedEvent
  | ConstructionCancelledEvent
  | BuildingConstructedEvent
  | SlotCapacityExpandedEvent
  | OutpostSlotCapacityExpandedEvent
  | ConstructionStalledEvent
  | ConstructionStalledByReserveEvent
  | OutpostConstructionStalledEvent
  | LabourAssignedEvent
  | NeedsResolvedEvent
  | ResearchProducedEvent
  | DirectiveSetEvent
  | DirectiveRemovedEvent
  | ManualOverrideChangedEvent
  | DirectiveFiredEvent
  | ProductionShortfallEvent
  | ColonyFoundingLaunchedEvent
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
  | ColonyHomeBodySetEvent
  | ActiveRecipeSetEvent
  | DifficultyChangedEvent
  | OutpostEstablishedEvent
  | OutpostDecommissionedEvent
  | OutpostConstructionQueuedEvent
  | OutpostPromotedEvent
  | IgnoredEvent
