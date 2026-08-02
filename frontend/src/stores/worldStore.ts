/**
 * Pinia store that owns the live reactive world state.
 */

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { WorldState, ColonyState, LogEntry } from '@/worldModel/model'
import { EMPTY_WORLD_STATE } from '@/worldModel/model'
import { applyEvent, hydrateFromSnapshot } from '@/worldModel/reducer'
import type { ServerMessage } from '@/types/api'
import { isTauri } from '@/services/tauriBridge'

/** Connection status. */
export type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'error'

/** Maximum number of recent entries to keep in the unified colony log. */
const MAX_LOG_ENTRIES = 200

interface HydrateInput {
  sol: number
  month: number
  colonies: { id: string; name: string; population: number }[]
}

let _logSeq = 0
function nextLogId(): string {
  _logSeq += 1
  return `log-${_logSeq}`
}

export const useWorldStore = defineStore('world', () => {
  const world = ref<WorldState>({ ...EMPTY_WORLD_STATE })
  // Desktop mode has no socket to connect to — declare connected up front.
  const connectionStatus = ref<ConnectionStatus>(isTauri ? 'connected' : 'disconnected')
  const lastError = ref<string | null>(null)
  /**
   * Rolling log of every server event — one entry each, whether or not the
   * reducer classified it as alert-worthy (colony details multi-window
   * redesign's events/alerts unification). The reducer's own `notifications`
   * (below) stay a separate, curated, alert-only list — used for
   * `SystemStatsBar`'s alert count and to decide when a new entry here also
   * pops a toast (see `alertToast`) — this array is the comprehensive one
   * the log panel actually renders.
   */
  const logEntries = ref<LogEntry[]>([])
  /** Most recent alert-tier ('notable'/'urgent'/'blocking') log entry that
   * hasn't been dismissed yet — the alert toast reads this. `null` when
   * there's nothing to show or the player already dismissed it. */
  const alertToast = ref<LogEntry | null>(null)

  const sol = computed(() => world.value.sol)
  const month = computed(() => world.value.month)
  const colonies = computed(() => Object.values(world.value.colonies))
  const researchTotal = computed(() => world.value.research_total)
  const notifications = computed(() => world.value.notifications)
  const isConnected = computed(() => connectionStatus.value === 'connected')

  /** Append a log entry, trimming from the front once past `MAX_LOG_ENTRIES`. */
  function pushLogEntry(entry: LogEntry): void {
    const next = [...logEntries.value, entry]
    logEntries.value = next.length > MAX_LOG_ENTRIES ? next.slice(-MAX_LOG_ENTRIES) : next
  }

  function handleServerMessage(msg: ServerMessage): void {
    switch (msg.type) {
      case 'snapshot':
        world.value = hydrateFromSnapshot(msg.state)
        break
      case 'event': {
        const prevNotifCount = world.value.notifications.length
        world.value = applyEvent(world.value, msg.event)
        const added = world.value.notifications.slice(prevNotifCount)
        if (added.length > 0) {
          // The reducer already classified this event as alert-worthy —
          // reuse its curated tier/message verbatim rather than re-deriving
          // a generic one, and pop the toast.
          for (const n of added) {
            const entry: LogEntry = { ...n, event: msg.event }
            pushLogEntry(entry)
            alertToast.value = entry
          }
        } else {
          // Not alert-worthy, but every event still gets a log row —
          // otherwise-silent events (construction queued, directives,
          // outposts, ...) are exactly what "otherwise be part of the log"
          // covers.
          pushLogEntry({
            id: nextLogId(),
            tier: 'ambient',
            message: msg.event.kind.replace(/_/g, ' '),
            timestamp_sol: world.value.sol,
            event: msg.event,
          })
        }
        break
      }
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

  function clearLog(): void {
    logEntries.value = []
  }

  function dismissAlertToast(): void {
    alertToast.value = null
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
    logEntries.value = []
    alertToast.value = null
  }

  return {
    world,
    connectionStatus,
    lastError,
    logEntries,
    alertToast,
    sol,
    month,
    colonies,
    researchTotal,
    notifications,
    isConnected,
    handleServerMessage,
    setConnectionStatus,
    clearNotifications,
    clearLog,
    dismissAlertToast,
    hydrate,
    reset,
  }
})
