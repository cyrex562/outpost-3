/**
 * Pinia store that owns the live reactive world state.
 */

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { WorldState, ColonyState } from '@/worldModel/model'
import { EMPTY_WORLD_STATE } from '@/worldModel/model'
import { applyEvent, hydrateFromSnapshot } from '@/worldModel/reducer'
import type { ServerMessage } from '@/types/api'
import type { ServerEvent } from '@/types/events'
import { isTauri } from '@/services/tauriBridge'

/** Connection status. */
export type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'error'

/** Maximum number of recent server events to keep in the event log. */
const MAX_EVENT_LOG = 50

interface HydrateInput {
  sol: number
  month: number
  colonies: { id: string; name: string; population: number }[]
}

export const useWorldStore = defineStore('world', () => {
  const world = ref<WorldState>({ ...EMPTY_WORLD_STATE })
  // Desktop mode has no socket to connect to — declare connected up front.
  const connectionStatus = ref<ConnectionStatus>(isTauri ? 'connected' : 'disconnected')
  const lastError = ref<string | null>(null)
  /** Rolling log of recent server events for the event log panel. */
  const eventLog = ref<ServerEvent[]>([])

  const sol = computed(() => world.value.sol)
  const month = computed(() => world.value.month)
  const colonies = computed(() => Object.values(world.value.colonies))
  const researchTotal = computed(() => world.value.research_total)
  const notifications = computed(() => world.value.notifications)
  const isConnected = computed(() => connectionStatus.value === 'connected')

  function handleServerMessage(msg: ServerMessage): void {
    switch (msg.type) {
      case 'snapshot':
        world.value = hydrateFromSnapshot(msg.state)
        break
      case 'event':
        world.value = applyEvent(world.value, msg.event)
        eventLog.value = [...eventLog.value.slice(-(MAX_EVENT_LOG - 1)), msg.event]
        break
      case 'error':
        lastError.value = msg.message
        break
      case 'ack':
        break
      case 'new_game_snapshot': {
        // Full snapshot returned after NewGame init — treat the same as snapshot.
        world.value = hydrateFromSnapshot(msg.state)
        // Authoritatively select the newly founded colony from this direct,
        // request-scoped response rather than leaving it to a later "pick
        // colonies[0] if unset" fallback (ColonyView/OutpostsView) — that
        // fallback races against any other event this shared connection
        // happens to receive first (e.g. a broadcasted event from another
        // browser tab/session hitting the same shared engine), which can
        // latch onto the wrong colony entirely.
        const firstColonyId = Object.keys(world.value.colonies)[0]
        if (firstColonyId) {
          import('@/stores/game').then(({ useGameStore }) => {
            useGameStore().selectedColonyId = firstColonyId
          })
        }
        break
      }
      case 'query_result':
        if (msg.result.kind === 'colony_screen') {
          import('@/stores/game').then(({ useGameStore }) => {
            useGameStore().setColonyScreen(
              msg.result.kind === 'colony_screen' ? msg.result.data : (null as never),
            )
          })
        }
        break
    }
  }

  function setConnectionStatus(status: ConnectionStatus): void {
    connectionStatus.value = status
    if (status === 'connected') {
      lastError.value = null
    }
  }

  function clearNotifications(): void {
    world.value = { ...world.value, notifications: [] }
  }

  function clearEventLog(): void {
    eventLog.value = []
  }

  /** Hydrate the store from a Tauri `SnapshotPayload`. */
  function hydrate(input: HydrateInput): void {
    const nextColonies: Record<string, ColonyState> = {}
    for (const c of input.colonies) {
      nextColonies[c.id] = {
        id: c.id,
        name: c.name,
        population: c.population,
        stability: 1.0,
        available_labour: 0,
        buildings: [],
        active_projects: [],
        commodity_pool: [],
        active_construction: [],
      }
    }
    world.value = {
      ...EMPTY_WORLD_STATE,
      sol: input.sol,
      month: input.month,
      colonies: nextColonies,
    }
  }

  /** Reset everything back to the pre-game state. */
  function reset(): void {
    world.value = { ...EMPTY_WORLD_STATE }
    lastError.value = null
    eventLog.value = []
  }

  return {
    world,
    connectionStatus,
    lastError,
    eventLog,
    sol,
    month,
    colonies,
    researchTotal,
    notifications,
    isConnected,
    handleServerMessage,
    setConnectionStatus,
    clearNotifications,
    clearEventLog,
    hydrate,
    reset,
  }
})
