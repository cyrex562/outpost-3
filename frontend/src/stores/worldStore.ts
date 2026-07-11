/**
 * Pinia store that owns the live reactive world state.
 */

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { WorldState, ColonyState } from '@/worldModel/model'
import { EMPTY_WORLD_STATE } from '@/worldModel/model'
import { applyEvent, hydrateFromSnapshot } from '@/worldModel/reducer'
import type { ServerMessage } from '@/types/api'
import { isTauri } from '@/services/tauriBridge'

/** Connection status. */
export type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'error'

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
        break
      case 'error':
        lastError.value = msg.message
        break
      case 'ack':
        break
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

  /** Hydrate the store from a Tauri `SnapshotPayload`. */
  function hydrate(input: HydrateInput): void {
    const colonies: Record<string, ColonyState> = {}
    for (const c of input.colonies) {
      colonies[c.id] = {
        id: c.id,
        name: c.name,
        population: c.population,
        stability: 1.0,
        available_labour: 0,
        buildings: [],
        active_projects: [],
      }
    }
    world.value = {
      ...EMPTY_WORLD_STATE,
      sol: input.sol,
      month: input.month,
      colonies,
    }
  }

  /** Reset everything back to the pre-game state. */
  function reset(): void {
    world.value = { ...EMPTY_WORLD_STATE }
    lastError.value = null
  }

  return {
    world,
    connectionStatus,
    lastError,
    sol,
    month,
    colonies,
    researchTotal,
    notifications,
    isConnected,
    handleServerMessage,
    setConnectionStatus,
    clearNotifications,
    hydrate,
    reset,
  }
})
