/**
 * Typed client command shapes that map directly to the Rust `Command` enum.
 *
 * These are submitted via `POST /sessions/{id}/commands` or over the WebSocket
 * channel as `{ type: "command", seq: N, command: Command }`.
 */

/**
 * Interrupt severity tiers, lowest to highest (issue #332). Used as the
 * fast-forward halt threshold: a run stops on the first interrupt at or above
 * the chosen tier.
 */
export type InterruptTier = 'ambient' | 'notable' | 'urgent' | 'blocking'

export type Command =
  | { kind: 'advance_sol' }
  /**
   * Advance up to `max_sols` sols in one call, halting early on the first
   * interrupt at or above `threshold` (issue #332).
   */
  | { kind: 'fast_forward'; max_sols: number; threshold: InterruptTier }
  | { kind: 'found_colony'; name: string; starting_population: number }
  | {
      kind: 'queue_construction'
      colony_id: string
      building_type: string
      slot_cost: number
      labor_per_turn: number
      construction_cost: [string, number][]
      construction_turns: number
    }
  | {
      /**
       * Vestigial (issue #307) — the engine validates this and emits an event
       * but persists nothing, and production ignores it. The real per-building
       * override is `set_building_labour_lock`.
       */
      kind: 'assign_labour'
      colony_id: string
      slot: string
      labour: number
    }
  | {
      /**
       * Withhold a quantity of one commodity from industry (issue #308).
       *
       * A floor on the stockpile, not a transfer: the reserved stock stays in
       * the pool and stays visible, but recipe inputs and building maintenance
       * cannot draw the pool below it. `0` clears the reserve. Colonist needs
       * are exempt — reserving food is only useful if the colonists can still
       * eat it while the fuel plant cannot.
       *
       * A negative or non-finite amount is rejected, not clamped.
       */
      kind: 'set_commodity_reserve'
      colony_id: string
      commodity_id: string
      amount: number
    }
  | {
      /**
       * Set one building's staffing priority (issue #307). `1` is staffed
       * first, `9` last; the engine rejects anything outside that range rather
       * than clamping.
       *
       * Addressed to a placed **instance**, unlike `set_active_recipe`, which
       * applies to every building of a type.
       */
      kind: 'set_building_priority'
      colony_id: string
      building_id: string
      priority: number
    }
  | {
      /**
       * Pin workers to a building, or release it back to automatic assignment
       * with `null` (issue #307). A lock larger than the building's worker slots
       * is rejected, not clamped.
       */
      kind: 'set_building_labour_lock'
      colony_id: string
      building_id: string
      lock: number | null
    }
  | {
      /**
       * Name a building, or clear it back to the auto-numbered default with
       * `null` (issue #307). Names are trimmed; blank and overlong are rejected.
       */
      kind: 'rename_building'
      colony_id: string
      building_id: string
      name: string | null
    }
  | {
      /**
       * Pause or resume a placed building (issue #309). A paused building is
       * excluded from production entirely — no labour, power, or commodity
       * draw — but keeps its build slot; pausing is not demolition.
       */
      kind: 'set_building_paused'
      colony_id: string
      building_id: string
      paused: boolean
    }
  | {
      /**
       * Repair a broken building, returning it to service (issue #384).
       *
       * Charges the colony's stockpile. Rejected if the building is not
       * actually broken — repair is not a free top-up for a merely worn one —
       * or if the materials are not there, in which case nothing is spent.
       */
      kind: 'repair_building'
      colony_id: string
      building_id: string
    }
  | { kind: 'cancel_construction'; colony_id: string; project_id: string }
  | {
      kind: 'deploy_starter_kit'
      colony_id: string
      /** `[building_type, slot_cost]` pairs to place instantly, in order. */
      buildings: [string, number][]
    }
  | {
      kind: 'set_directive'
      directive: {
        id: string
        colony_id: string
        label: string
        priority: number
        condition: DirectiveCondition
        action: DirectiveAction
      }
    }
  | { kind: 'remove_directive'; directive_id: string }
  | { kind: 'set_manual_override'; colony_id: string; enabled: boolean }
  | { kind: 'research_tech'; tech_id: string }
  | { kind: 'enqueue_research'; tech_id: string }
  | { kind: 'cancel_research' }
  | {
      kind: 'set_balance_scalar'
      /** Stable slug of the quantity to tune. */
      quantity: string
      /** New multiplier; the engine clamps to its own bounds. */
      value: number
    }
  | {
      kind: 'found_colony_at_site'
      name: string
      starting_population: number
      site_id: string
      focus: string | null
      supplies_id?: string | null
      /**
       * Explicit per-commodity starter-supply amounts (issue #167). When
       * present, replaces the `supplies_id` package's per-100-colonist
       * scaling with these exact absolute quantities.
       */
      supply_overrides?: [string, number][] | null
      /**
       * Colony funding this settlement (issue #359). The sponsor pays the
       * `colony` cost profile out of its pool and gives up the colonists.
       * Omitting it means nobody pays — correct only for the game's first
       * colony, which arrives from off-system with no sponsor to bill.
       */
      sponsor_colony_id?: string | null
      /** Star-system body this site belongs to (issue #183). Omitting it skips the habitability gate. */
      body_id?: string | null
    }
  | { kind: 'assign_colony_home_body'; colony_id: string; body_id: string }
  | { kind: 'establish_outpost'; name: string; colony_id: string; body_id: string }
  | { kind: 'decommission_outpost'; outpost_id: string }
  | {
      kind: 'queue_outpost_construction'
      outpost_id: string
      building_type: string
      slot_cost: number
      labor_per_turn: number
      construction_cost: [string, number][]
      construction_turns: number
    }
  | {
      kind: 'promote_outpost_to_colony'
      outpost_id: string
      name: string
      starting_population: number
    }
  | {
      kind: 'build_infrastructure'
      from_colony: string
      to_colony: string
      infra_type: 'road' | 'rail' | 'pipeline' | 'powerline'
    }
  | { kind: 'demolish_infrastructure'; from_colony: string; to_colony: string }
  | {
      /**
       * Create a manually-added trade route between two colonies (issue
       * #363). Unlike `build_infrastructure`, this has no same-body
       * requirement — the engine derives convoy transit time from body
       * separation the same way either kind of route is created, falling
       * back to a 1-sol default when the endpoints share a body or lack
       * home-body context.
       */
      kind: 'add_trade_route'
      colony_a: string
      colony_b: string
      /** Maximum units per commodity that may transit per strategic turn. */
      throughput_cap: number
    }
  | { kind: 'remove_trade_route'; route_id: string }

/** Directive trigger condition shape. */
export interface DirectiveCondition {
  kind: string
  [key: string]: unknown
}

/** Directive action shape. */
export interface DirectiveAction {
  kind: string
  [key: string]: unknown
}
