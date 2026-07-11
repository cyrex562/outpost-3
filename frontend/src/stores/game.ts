/**
 * Pinia store for command dispatch.
 *
 * In desktop (Tauri) mode all commands go through the Tauri bridge — no
 * WebSocket, no session concept. In browser mode we still support the
 * legacy REST + WebSocket path for `outpost_web`.
 */

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { Command } from '@/types/commands'
import type { GameEvent } from '@/types/gameEvents'
import type { ColonyScreenData } from '@/types/screen'
import { createSession, applyCommand, deleteSession } from '@/api/client'
import { isTauri, apply as tauriApply, query as tauriQuery } from '@/services/tauriBridge'
import { useWorldStore } from '@/stores/worldStore'

export interface PendingCommand {
  seq: number
  command: Command
}

export const useGameStore = defineStore('game', () => {
  // ─── State ─────────────────────────────────────────────────────────────────

  const sessionId = ref<string | null>(null)
  const pendingCommands = ref<PendingCommand[]>([])
  const nextSeq = ref(1)
  const lastEvents = ref<GameEvent[]>([])
  const toastMessage = ref<string | null>(null)
  const busy = ref(false)
  const selectedColonyId = ref<string | null>(null)
  const colonyScreen = ref<ColonyScreenData | null>(null)

  // ─── Getters ───────────────────────────────────────────────────────────────

  const hasSession = computed(() => sessionId.value !== null || isTauri)
  const pendingCount = computed(() => pendingCommands.value.length)

  // ─── Session (browser mode only) ───────────────────────────────────────────

  async function openSession(): Promise<void> {
    if (isTauri || sessionId.value !== null) return
    const id = await createSession()
    sessionId.value = id
  }

  async function closeSession(): Promise<void> {
    if (isTauri || sessionId.value === null) return
    try {
      await deleteSession(sessionId.value)
    } finally {
      sessionId.value = null
      pendingCommands.value = []
      lastEvents.value = []
    }
  }

  /**
   * Enqueue a command and dispatch it.
   *
   * In Tauri mode this goes over IPC; the returned events are pushed straight
   * into the world store via `applyEvent` so state stays in sync without
   * needing a WebSocket round-trip.
   */
  async function sendCommand(cmd: Command): Promise<GameEvent[]> {
    const worldStore = useWorldStore()
    const seq = nextSeq.value++
    pendingCommands.value.push({ seq, command: cmd })
    busy.value = true
    toastMessage.value = null

    try {
      let events: GameEvent[]
      if (isTauri) {
        events = await tauriApply(cmd)
        // Feed events into the world reducer directly.
        for (const ev of events) {
          worldStore.handleServerMessage({ type: 'event', event: ev as unknown as import('@/types/events').ServerEvent })
        }
        // Refresh colony screen if a colony is selected (so tables repopulate).
        if (selectedColonyId.value) {
          try {
            const q = await tauriQuery({ kind: 'colony_screen', colony_id: selectedColonyId.value })
            if (q.kind === 'colony_screen' && q.data) {
              colonyScreen.value = q.data as ColonyScreenData
            }
          } catch {
            // colony_screen may not be available for every colony; ignore.
          }
        }
      } else {
        if (sessionId.value === null) await openSession()
        events = await applyCommand(sessionId.value!, cmd)
      }
      lastEvents.value = events
      toastMessage.value = summariseEvents(events)
      pendingCommands.value = pendingCommands.value.filter((p) => p.seq !== seq)
      return events
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err)
      toastMessage.value = `Error: ${msg}`
      pendingCommands.value = pendingCommands.value.filter((p) => p.seq !== seq)
      return []
    } finally {
      busy.value = false
    }
  }

  function dismissToast(): void {
    toastMessage.value = null
  }

  function setColonyScreen(data: ColonyScreenData): void {
    colonyScreen.value = data
    selectedColonyId.value = data.colony_id
  }

  return {
    sessionId,
    pendingCommands,
    lastEvents,
    toastMessage,
    busy,
    selectedColonyId,
    colonyScreen,
    hasSession,
    pendingCount,
    openSession,
    closeSession,
    sendCommand,
    dismissToast,
    setColonyScreen,
  }
})

function summariseEvents(events: GameEvent[]): string {
  if (events.length === 0) return 'Command accepted — no events.'
  const lines = events.map((e) => {
    switch (e.kind) {
      case 'colony_sol_advanced':
        return `Sol ${e.sol} begins.`
      case 'strategic_month_advanced':
        return `Month ${e.month} begins.`
      case 'colony_founded':
        return `Colony "${e.name}" founded.`
      case 'needs_resolved': {
        const delta = (e as import('@/types/gameEvents').NeedsResolvedEvent).stability_delta
        return `Needs resolved (Δstability ${delta >= 0 ? '+' : ''}${delta.toFixed(2)}).`
      }
      case 'research_produced': {
        const amt = (e as import('@/types/gameEvents').ResearchProducedEvent).amount
        return `Research +${amt.toFixed(1)} RP.`
      }
      case 'directive_fired':
        return `Directive fired.`
      case 'production_shortfall':
        return `Shortfall: ${e.building_type} — ${e.reason}.`
      default:
        return e.kind.replace(/_/g, ' ')
    }
  })
  return lines.slice(0, 4).join('  |  ') + (lines.length > 4 ? ` (+${lines.length - 4} more)` : '')
}
