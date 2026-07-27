/**
 * Pure reducer: applies a ServerEvent to WorldState and returns a new WorldState.
 *
 * This is a plain function with no side-effects or Vue dependencies so it is
 * straightforward to unit-test with Vitest and portable to any renderer.
 */

import type { ServerEvent } from '@/types/events'
import type { WorldState, ColonyState } from './model'
import { EMPTY_WORLD_STATE } from './model'
import type { WorldSnapshot } from '@/types/api'

let _notificationSeq = 0

function nextNotificationId(): string {
  _notificationSeq += 1
  return `notif-${_notificationSeq}`
}

/**
 * Hydrate the world state from a full snapshot delivered on WebSocket connect.
 * Resets all local state so a reconnect is safe.
 */
export function hydrateFromSnapshot(snap: WorldSnapshot): WorldState {
  const colonies: Record<string, ColonyState> = {}
  for (const c of snap.colonies) {
    colonies[c.id] = {
      ...c,
      stability: c.stability,
      available_labour: Math.max(0, c.available_labour),
      buildings: c.buildings,
      active_projects: c.active_construction.map((building_type, i) => ({
        project_id: `${c.id}-proj-${i}`,
        building_type,
      })),
    }
  }
  return {
    ...EMPTY_WORLD_STATE,
    sol: snap.sol,
    month: snap.month,
    colonies,
    notifications: [],
  }
}

/**
 * Apply a single server event to the world state.
 *
 * Returns a new `WorldState` object (immutable update pattern).
 */
export function applyEvent(state: WorldState, event: ServerEvent): WorldState {
  switch (event.kind) {
    case 'colony_sol_advanced':
      return { ...state, sol: event.sol }

    case 'strategic_month_advanced':
      return { ...state, month: event.month }

    // Both founding paths add the colony to the world model; the site path
    // carries an extra site_id the model has no use for (issue #317).
    case 'colony_founded':
    case 'colony_founded_at_site': {
      const newColony: ColonyState = {
        id: event.colony_id,
        name: event.name,
        population: event.starting_population,
        stability: 1.0,
        available_labour: 0,
        buildings: [],
        active_projects: [],
        commodity_pool: [],
        active_construction: [],
      }
      return {
        ...state,
        colonies: { ...state.colonies, [event.colony_id]: newColony },
      }
    }

    case 'construction_queued': {
      const colony = state.colonies[event.colony_id]
      if (!colony) return state
      const updated: ColonyState = {
        ...colony,
        active_projects: [
          ...colony.active_projects,
          { project_id: event.project_id, building_type: event.building_type },
        ],
      }
      return {
        ...state,
        colonies: { ...state.colonies, [event.colony_id]: updated },
      }
    }

    case 'construction_cancelled': {
      const colony = state.colonies[event.colony_id]
      if (!colony) return state
      const updated: ColonyState = {
        ...colony,
        active_projects: colony.active_projects.filter(
          (p) => p.project_id !== event.project_id,
        ),
      }
      return {
        ...state,
        colonies: { ...state.colonies, [event.colony_id]: updated },
      }
    }

    case 'building_constructed': {
      const colony = state.colonies[event.colony_id]
      if (!colony) return state
      const updated: ColonyState = {
        ...colony,
        buildings: [...colony.buildings, event.building_type],
        // The completed project is removed from the queue by the cancel event
        // that may have been emitted before, or by matching building_type here.
        active_projects: colony.active_projects.filter(
          (p) => p.building_type !== event.building_type,
        ),
      }
      return {
        ...state,
        colonies: { ...state.colonies, [event.colony_id]: updated },
      }
    }

    case 'needs_resolved': {
      const colony = state.colonies[event.colony_id]
      if (!colony) return state
      const updated: ColonyState = {
        ...colony,
        stability: Math.max(0, Math.min(1, colony.stability + event.stability_delta)),
        population: Math.max(0, colony.population + event.population_delta),
      }
      const notification =
        event.composite_satisfaction < 0.3
          ? {
              id: nextNotificationId(),
              tier: 'notable' as const,
              message: `${colony.name}: low satisfaction (${(event.composite_satisfaction * 100).toFixed(0)}%)`,
              colony_id: event.colony_id,
              timestamp_sol: state.sol,
            }
          : null
      return {
        ...state,
        colonies: { ...state.colonies, [event.colony_id]: updated },
        notifications: notification
          ? [...state.notifications, notification]
          : state.notifications,
      }
    }

    case 'research_produced':
      return { ...state, research_total: state.research_total + event.amount }

    case 'labour_assigned': {
      const colony = state.colonies[event.colony_id]
      if (!colony) return state
      return {
        ...state,
        colonies: {
          ...state.colonies,
          [event.colony_id]: { ...colony, available_labour: colony.available_labour - event.labour },
        },
      }
    }

    case 'manual_override_changed':
    case 'directive_set':
    case 'directive_removed':
    case 'directive_fired':
      // These events affect directive / automation state.
      // The current world model does not yet track directives in detail;
      // return unchanged state so the reducer remains total over all event kinds.
      return state

    case 'colony_home_body_set':
    case 'active_recipe_set':
    case 'difficulty_changed':
    case 'outpost_established':
    case 'outpost_decommissioned':
    case 'outpost_construction_queued':
    case 'outpost_promoted':
      // The current world model does not yet track home-body/habitability,
      // per-building active recipe, difficulty, or outpost state in detail
      // (outposts are queried separately — see `services/tauriBridge.ts`'s
      // `listOutposts`). Return unchanged state rather than omitting these
      // cases: an omitted case still type-checks (TS can't see events a
      // wire layer forwards via an `as unknown as ServerEvent` cast) but
      // falls through with no matching arm, and a switch with no `default`
      // silently returns `undefined` — which is exactly what happened here
      // (issue: blank screen after founding, `world.value` became
      // `undefined` because `colony_home_body_set` — fired by the founding
      // wizard on nearly every real playthrough — had no case at all).
      return state

    case 'production_shortfall': {
      const colony = state.colonies[event.colony_id]
      const notification = {
        id: nextNotificationId(),
        tier: 'notable' as const,
        message: `${colony?.name ?? event.colony_id}: production shortfall on ${event.building_type} (scale ${(event.scale * 100).toFixed(0)}%)`,
        colony_id: event.colony_id,
        timestamp_sol: state.sol,
      }
      return { ...state, notifications: [...state.notifications, notification] }
    }

    case 'hazard_occurred': {
      const colony = state.colonies[event.colony_id]
      const notification = {
        id: nextNotificationId(),
        tier: 'urgent' as const,
        message: `${colony?.name ?? event.colony_id}: hazard — ${event.hazard_kind} (severity ${(event.severity * 100).toFixed(0)}%)`,
        colony_id: event.colony_id,
        timestamp_sol: state.sol,
      }
      return { ...state, notifications: [...state.notifications, notification] }
    }

    case 'migration_arrived': {
      const colony = state.colonies[event.to_colony]
      if (!colony) return state
      const updated = {
        ...colony,
        population: Math.max(0, colony.population + event.count),
        stability: Math.max(0, Math.min(1, colony.stability + event.overcrowding_stability_penalty)),
      }
      return { ...state, colonies: { ...state.colonies, [event.to_colony]: updated } }
    }

    case 'voluntary_emigration_triggered': {
      const colony = state.colonies[event.from_colony]
      if (!colony) return state
      const updated = {
        ...colony,
        population: Math.max(0, colony.population - event.count),
      }
      const notification = {
        id: nextNotificationId(),
        tier: 'notable' as const,
        message: `${colony.name}: voluntary emigration — ${event.count} colonists departed`,
        colony_id: event.from_colony,
        timestamp_sol: state.sol,
      }
      return {
        ...state,
        colonies: { ...state.colonies, [event.from_colony]: updated },
        notifications: [...state.notifications, notification],
      }
    }

    case 'expedition_launched': {
      const notification = {
        id: nextNotificationId(),
        tier: 'notable' as const,
        message: `Expedition launched from ${state.colonies[event.colony_id]?.name ?? event.colony_id} → hex (${event.target_hex_q}, ${event.target_hex_r})`,
        colony_id: event.colony_id,
        timestamp_sol: state.sol,
      }
      return { ...state, notifications: [...state.notifications, notification] }
    }

    case 'expedition_arrived':
    case 'expedition_returned':
      // Expedition lifecycle — no world-model mutation needed at this detail level.
      return state

    case 'expedition_lost': {
      const notification = {
        id: nextNotificationId(),
        tier: 'urgent' as const,
        message: `Expedition ${event.expedition_id.slice(0, 8)} was lost`,
        timestamp_sol: state.sol,
      }
      return { ...state, notifications: [...state.notifications, notification] }
    }

    case 'tech_unlocked': {
      const notification = {
        id: nextNotificationId(),
        tier: 'notable' as const,
        message: `Technology unlocked: ${event.tech_id}`,
        timestamp_sol: state.sol,
      }
      return { ...state, notifications: [...state.notifications, notification] }
    }

    case 'victory_achieved': {
      const notification = {
        id: nextNotificationId(),
        tier: 'urgent' as const,
        message: `Victory achieved: ${event.condition}`,
        timestamp_sol: state.sol,
      }
      return { ...state, notifications: [...state.notifications, notification] }
    }

    case 'menace_critical': {
      const notification = {
        id: nextNotificationId(),
        tier: 'urgent' as const,
        message: `MENACE CRITICAL: ${event.menace_kind} at ${(event.level * 100).toFixed(0)}% — ${event.countdown_months} months until collapse`,
        timestamp_sol: state.sol,
      }
      return { ...state, notifications: [...state.notifications, notification] }
    }

    case 'cargo_delivered':
    case 'orbital_station_completed':
      // These events update commodity pools / orbital state which the current
      // world model does not yet track in detail; return unchanged state.
      return state

    case 'ignored':
      // Explicitly mapped to indicate no frontend action required.
      return state

    default:
      // Defense in depth against the exact failure class above recurring:
      // both wire layers (`outpost_web`'s WS bridge and `outpost_tauri`'s
      // IPC bridge) can forward a `kind` this switch has no case for —
      // either a genuinely new core `Event` variant not yet given a typed
      // `ServerEvent` case anywhere, or `outpost_tauri`'s own `"unknown"`
      // catch-all for core events it hasn't typed yet either. A switch
      // with no matching case and no `default` returns `undefined` here,
      // silently corrupting the whole world state — which is exactly what
      // shipped for `colony_home_body_set` before this file added a case
      // for it. `console.warn` rather than silence, so a genuinely new
      // event type that DOES need modelling doesn't go unnoticed forever;
      // `state` unchanged is still the safe fallback either way.
      console.warn(`applyEvent: unrecognised event kind "${(event as { kind?: string }).kind}" — ignoring`)
      return state
  }
}
