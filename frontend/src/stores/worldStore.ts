/**
 * Pinia store that owns the live reactive world state.
 *
 * The store receives typed server messages via the WebSocket composable,
 * applies them through the pure reducer, and exposes derived getters so
 * components never access raw state fields directly.
 */

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { WorldState } from '@/worldModel/model'
import { EMPTY_WORLD_STATE } from '@/worldModel/model'
import { applyEvent, hydrateFromSnapshot } from '@/worldModel/reducer'
import type { ServerMessage } from '@/types/api'

/** Connection status. */
export type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'error'

export const useWorldStore = defineStore('world', () => {
  // ─── State ──────────────────────────────────────────────────────────────────
  const world = ref<WorldState>({ ...EMPTY_WORLD_STATE })
  const connectionStatus = ref<ConnectionStatus>('disconnected')
  const lastError = ref<string | null>(null)

  // ─── Getters ────────────────────────────────────────────────────────────────
  const sol = computed(() => world.value.sol)
  const month = computed(() => world.value.month)
  const colonies = computed(() => Object.values(world.value.colonies))
  const researchTotal = computed(() => world.value.research_total)
  const notifications = computed(() => world.value.notifications)
  const isConnected = computed(() => connectionStatus.value === 'connected')

  // ─── Actions ────────────────────────────────────────────────────────────────

  /**
   * Process a typed server message received over the WebSocket.
   * This is the single entry-point for all server-pushed state changes.
   */
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
        // Acknowledged — no state change needed here; callers may await acks.
        break
      case 'new_game_snapshot':
        // Full snapshot returned after NewGame init — treat the same as snapshot.
        world.value = hydrateFromSnapshot(msg.state)
        break
      case 'query_result':
        // Route colony_screen results to the game store.
        if (msg.result.kind === 'colony_screen') {
          // Lazy import to avoid circular dependency.
          import('@/stores/game').then(({ useGameStore }) => {
            useGameStore().setColonyScreen(msg.result.kind === 'colony_screen' ? msg.result.data : null!)
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

  return {
    // State (readonly refs exposed for components)
    world,
    connectionStatus,
    lastError,
    // Getters
    sol,
    month,
    colonies,
    researchTotal,
    notifications,
    isConnected,
    // Actions
    handleServerMessage,
    setConnectionStatus,
    clearNotifications,
  }
})
