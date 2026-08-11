/**
 * Pinia store for command dispatch.
 *
 * In desktop (Tauri) mode all commands go through the Tauri bridge — no
 * WebSocket, no session concept. In browser mode we still support the
 * legacy REST + WebSocket path for `outpost_web`.
 */

import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import type { Command } from '@/types/commands'
import type { GameEvent } from '@/types/gameEvents'
import type { ColonyScreenData } from '@/types/screen'
import { createSession, applySharedCommand, deleteSession } from '@/api/client'
import { isTauri, apply as tauriApply, getColonyScreen } from '@/services/tauriBridge'
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
   * Fetch fresh `colony_screen` data for the given colony (or the currently
   * selected one) and store it on `colonyScreen`.
   *
   * Works in **both** hosts since #307 stage 4 — it used to bail out in browser
   * mode, which left `colonyScreen` permanently null there and every panel driven
   * by it (buildings, stockpile, colony resources) rendering empty.
   *
   * Errors are swallowed on purpose — a missing colony_screen shouldn't crash
   * the UI (may happen briefly after founding while the engine catches up).
   */
  async function refreshColonyScreen(colonyId?: string | null): Promise<void> {
    const id = colonyId ?? selectedColonyId.value
    if (!id) return
    try {
      colonyScreen.value = await getColonyScreen(id)
    } catch {
      // ignore — see doc comment
    }
  }

  // Auto-refresh whenever the selection changes in Tauri mode. This covers
  // tab clicks in ColonyView and the wizard's `selectedColonyId = founded.id`
  // handoff, so consumers never have to remember to fetch.
  watch(selectedColonyId, (next) => {
    if (next) void refreshColonyScreen(next)
  })

  /**
   * Enqueue a command and dispatch it.
   *
   * In Tauri mode this goes over IPC; the returned events are pushed straight
   * into the world store via `applyEvent` so state stays in sync without
   * needing a WebSocket round-trip. After every command we also refresh the
   * currently-selected colony's screen so per-turn state (commodity net,
   * new buildings, etc.) shows without a manual refetch.
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
      } else {
        // Browser mode dispatches against the shared engine (the same one
        // the WebSocket `new_game` flow bootstraps content/planet/colony
        // onto), not the unbootstrapped per-session engine (issue #220).
        //
        // These are fed into the reducer directly, same as Tauri mode, so the
        // UI reacts without waiting on a round trip. The server broadcasts the
        // same events over the WebSocket to every *other* client but skips
        // this one, keyed on the `X-Client-Id` header `applySharedCommand`
        // sends — see `api/clientId.ts`.
        //
        // (The comment previously here claimed the WS "only pushes events for
        // commands it itself issued". It never did: the server fanned out to
        // everyone including the issuer, so every command-issued event was
        // applied and logged twice — issue #452.)
        events = await applySharedCommand(cmd)
        for (const ev of events) {
          worldStore.handleServerMessage({ type: 'event', event: ev as unknown as import('@/types/events').ServerEvent })
        }
      }
      // Post-command refresh, both hosts: even if selection didn't change, this
      // catches per-turn commodity movement, construction progress, and
      // per-building staffing changes (#307).
      if (selectedColonyId.value) {
        await refreshColonyScreen(selectedColonyId.value)
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
    refreshColonyScreen,
  }
})

function summariseEvents(events: GameEvent[]): string {
  if (events.length === 0) return 'Command accepted — no events.'
  const lines = events
    .map((e) => {
      switch (e.kind) {
        case 'colony_sol_advanced':
          return `Sol ${e.sol} begins.`
        // The month is a calendar label with nothing in the UI showing it
        // anymore (issue #338, following #333's cadence unification) — a
        // toast for it would be month-flavoured copy with no month display
        // left to explain it, so this event is intentionally silent (filtered
        // out below).
        case 'strategic_month_advanced':
          return null
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
        case 'outpost_established':
          return `Outpost established.`
        case 'outpost_decommissioned':
          return `Outpost decommissioned.`
        case 'outpost_construction_queued':
          return `Outpost queued ${(e as import('@/types/gameEvents').OutpostConstructionQueuedEvent).building_type}.`
        case 'outpost_promoted':
          return `Outpost promoted to colony "${(e as import('@/types/gameEvents').OutpostPromotedEvent).name}".`
        default:
          return e.kind.replace(/_/g, ' ')
      }
    })
    .filter((line): line is string => line !== null)
  if (lines.length === 0) return 'Command accepted — no events.'
  return lines.slice(0, 4).join('  |  ') + (lines.length > 4 ? ` (+${lines.length - 4} more)` : '')
}
