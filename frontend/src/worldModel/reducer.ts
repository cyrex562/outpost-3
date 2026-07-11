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

    case 'colony_founded': {
      const newColony: ColonyState = {
        id: event.colony_id,
        name: event.name,
        population: event.starting_population,
        stability: 1.0,
        available_labour: 0,
        buildings: [],
        active_projects: [],
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
  }
}
