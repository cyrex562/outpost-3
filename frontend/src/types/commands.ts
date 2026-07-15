/**
 * Typed client command shapes that map directly to the Rust `Command` enum.
 *
 * These are submitted via `POST /sessions/{id}/commands` or over the WebSocket
 * channel as `{ type: "command", seq: N, command: Command }`.
 */

export type Command =
  | { kind: 'advance_sol' }
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
  | { kind: 'assign_labour'; colony_id: string; slot: string; labour: number }
  | { kind: 'cancel_construction'; colony_id: string; project_id: string }
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
  | {
      kind: 'found_colony_at_site'
      name: string
      starting_population: number
      site_id: string
      focus: string | null
      supplies_id?: string | null
      /** Star-system body this site belongs to (issue #183). Omitting it skips the habitability gate. */
      body_id?: string | null
    }
  | { kind: 'assign_colony_home_body'; colony_id: string; body_id: string }

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
