/**
 * Pinia store for session management and command dispatch.
 *
 * Holds the active session ID, a typed queue of pending commands,
 * the last batch of events returned by the server, and a toast message
 * for command feedback.
 */

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { Command } from '@/types/commands'
import type { GameEvent } from '@/types/gameEvents'
import type { ColonyScreenData } from '@/types/screen'
import { createSession, applyCommand, deleteSession } from '@/api/client'

/** A pending command waiting to be flushed to the server. */
export interface PendingCommand {
  seq: number
  command: Command
}

export const useGameStore = defineStore('game', () => {
  // ─── State ─────────────────────────────────────────────────────────────────

  /** Active REST session ID (null when no session is open). */
  const sessionId = ref<string | null>(null)

  /** Queue of commands not yet sent to the server. */
  const pendingCommands = ref<PendingCommand[]>([])

  /** Sequence counter for correlating acks. */
  const nextSeq = ref(1)

  /** Events returned by the last command dispatch. */
  const lastEvents = ref<GameEvent[]>([])

  /** Toast message shown after a command completes. */
  const toastMessage = ref<string | null>(null)

  /** Whether a command is currently in flight. */
  const busy = ref(false)

  /** Selected colony ID for the command panel. */
  const selectedColonyId = ref<string | null>(null)

  /** Colony screen data for the currently selected colony. */
  const colonyScreen = ref<ColonyScreenData | null>(null)

  // ─── Getters ───────────────────────────────────────────────────────────────

  const hasSession = computed(() => sessionId.value !== null)
  const pendingCount = computed(() => pendingCommands.value.length)

  // ─── Actions ───────────────────────────────────────────────────────────────

  /** Open a new REST session with the server. */
  async function openSession(): Promise<void> {
    if (sessionId.value !== null) return
    const id = await createSession()
    sessionId.value = id
  }

  /** Close and discard the current session. */
  async function closeSession(): Promise<void> {
    if (sessionId.value === null) return
    try {
      await deleteSession(sessionId.value)
    } finally {
      sessionId.value = null
      pendingCommands.value = []
      lastEvents.value = []
    }
  }

  /**
   * Enqueue a command and immediately flush it to the server.
   *
   * Shows a toast summarising the returned events.  If no session is open
   * a new one is created automatically.
   */
  async function sendCommand(cmd: Command): Promise<GameEvent[]> {
    if (sessionId.value === null) {
      await openSession()
    }
    const seq = nextSeq.value++
    pendingCommands.value.push({ seq, command: cmd })
    busy.value = true
    toastMessage.value = null
    try {
      const id = sessionId.value!
      const events = await applyCommand(id, cmd)
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

  /** Dismiss the current toast. */
  function dismissToast(): void {
    toastMessage.value = null
  }

  /** Set the colony screen data (populated by a WS query result). */
  function setColonyScreen(data: ColonyScreenData): void {
    colonyScreen.value = data
    selectedColonyId.value = data.colony_id
  }

  return {
    // State
    sessionId,
    pendingCommands,
    lastEvents,
    toastMessage,
    busy,
    selectedColonyId,
    colonyScreen,
    // Getters
    hasSession,
    pendingCount,
    // Actions
    openSession,
    closeSession,
    sendCommand,
    dismissToast,
    setColonyScreen,
  }
})

// ─── Helpers ──────────────────────────────────────────────────────────────────

/** Produce a one-line human-readable summary of a batch of game events. */
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
