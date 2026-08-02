/**
 * Colony screen window registry (colony details multi-window redesign):
 * panel identity, default arrangement, and localStorage persistence for the
 * `FloatingWindow`-based layout that replaced the `dockview-vue` tiling
 * dashboard. Six panels — Vitals, Utilities, Commodities, Buildings,
 * Construction Queue, Alerts — each open in their own floating, draggable,
 * resizable, edge-snapping window (see `FloatingWindow.vue` and
 * `floatingWindowRegistry.ts`) rather than tiling the whole screen.
 *
 * This module owns two independent pieces of persisted state:
 * - *which* windows are open (`loadPersistedOpenWindowIds`/`savePersistedOpenWindowIds`)
 * - *where* each open window sits (`FloatingWindow`'s own per-window
 *   `storageKey`, built here by `colonyWindowStorageKey`)
 * "Reset Layout" clears both.
 */

import type { ComputedRef, InjectionKey } from 'vue'
import type { LogEntry } from '@/worldModel/model'
import type { BuildingRow, ConstructionQueueRow, ResourceRow, StockpileRow } from '@/types/screen'

/** Stable identifiers for the six colony panel windows. */
export const COLONY_WINDOW = {
  vitalStats: 'vital-stats',
  utilities: 'utilities',
  commodities: 'commodities',
  buildings: 'buildings',
  constructionQueue: 'construction-queue',
  alerts: 'alerts',
} as const

export type ColonyWindowId = (typeof COLONY_WINDOW)[keyof typeof COLONY_WINDOW]

/** Every window id, in the tool palette's display order. */
export const COLONY_WINDOW_IDS: ColonyWindowId[] = Object.values(COLONY_WINDOW)

/** Human-readable titles — window title bars and the tool palette. */
export const COLONY_WINDOW_TITLES: Record<ColonyWindowId, string> = {
  [COLONY_WINDOW.vitalStats]: 'Vitals',
  [COLONY_WINDOW.utilities]: 'Utilities',
  [COLONY_WINDOW.commodities]: 'Commodities',
  [COLONY_WINDOW.buildings]: 'Buildings',
  [COLONY_WINDOW.constructionQueue]: 'Construction Queue',
  [COLONY_WINDOW.alerts]: 'Alerts',
}

export interface WindowDefaultRect {
  x: number
  y: number
  w: number
  h: number
}

/**
 * Default arrangement: a left column (vitals/utilities/commodities stacked),
 * a center column (buildings/construction queue stacked), and a right
 * column (alerts) — the same 3-column shape the dockview layout (and the
 * Splitpanes layout before it) used, just as absolute window rects instead
 * of relative dock positions. Purely a starting point: every window is
 * freely moveable/resizable/snappable from here, and its own geometry
 * persists independently once touched.
 */
export const COLONY_WINDOW_DEFAULT_RECT: Record<ColonyWindowId, WindowDefaultRect> = {
  [COLONY_WINDOW.vitalStats]: { x: 16, y: 16, w: 360, h: 220 },
  [COLONY_WINDOW.utilities]: { x: 16, y: 248, w: 360, h: 220 },
  [COLONY_WINDOW.commodities]: { x: 16, y: 480, w: 360, h: 280 },
  [COLONY_WINDOW.buildings]: { x: 392, y: 16, w: 520, h: 460 },
  [COLONY_WINDOW.constructionQueue]: { x: 392, y: 488, w: 520, h: 200 },
  [COLONY_WINDOW.alerts]: { x: 928, y: 16, w: 420, h: 672 },
}

// ─── Which windows are open ─────────────────────────────────────────────────

const OPEN_WINDOWS_STORAGE_KEY = 'outpost3.colony-view.open-windows.v1'

function isColonyWindowId(v: unknown): v is ColonyWindowId {
  return typeof v === 'string' && (COLONY_WINDOW_IDS as string[]).includes(v)
}

/** Read the persisted set of open window ids, or `null` if there is none /
 * it's corrupt / storage is unavailable — callers should default to "all
 * six open" on `null`, matching the pre-redesign dock's always-all-visible
 * default. Unknown ids (an old build's panel that no longer exists) are
 * silently dropped rather than surfaced as ghost windows. */
export function loadPersistedOpenWindowIds(): ColonyWindowId[] | null {
  try {
    const raw = window.localStorage.getItem(OPEN_WINDOWS_STORAGE_KEY)
    if (!raw) return null
    const parsed: unknown = JSON.parse(raw)
    if (!Array.isArray(parsed)) return null
    const ids = parsed.filter(isColonyWindowId)
    return ids
  } catch {
    return null
  }
}

/** Persist the current open-window set. Non-fatal on failure (storage full
 * or blocked), matching the rest of this app's layout-persistence helpers. */
export function savePersistedOpenWindowIds(ids: ColonyWindowId[]): void {
  try {
    window.localStorage.setItem(OPEN_WINDOWS_STORAGE_KEY, JSON.stringify(ids))
  } catch {
    // storage full or blocked — non-fatal
  }
}

/** Clears the persisted open-window set — part of "Reset Layout". */
export function clearPersistedOpenWindowIds(): void {
  try {
    window.localStorage.removeItem(OPEN_WINDOWS_STORAGE_KEY)
  } catch {
    // storage full or blocked — non-fatal
  }
}

// ─── Where each window sits ─────────────────────────────────────────────────

/** `FloatingWindow`'s own `storageKey` prop for a given colony window —
 * one independent localStorage entry per window id, so moving one panel
 * never disturbs another's persisted geometry. */
export function colonyWindowStorageKey(id: ColonyWindowId): string {
  return `outpost3.colony-view.window.${id}`
}

/** Clears every colony window's persisted geometry — the other half of
 * "Reset Layout", alongside {@link clearPersistedOpenWindowIds}. */
export function clearAllColonyWindowGeometry(): void {
  for (const id of COLONY_WINDOW_IDS) {
    try {
      window.localStorage.removeItem(colonyWindowStorageKey(id))
    } catch {
      // storage full or blocked — non-fatal
    }
  }
}

// ─── Panel data/handler context (provide/inject) ───────────────────────────

/**
 * Everything the six panel wrapper components need, computed once in
 * `ColonyView.vue` and handed down via `provide`/`inject` — every one of
 * these panels shows the same single colony's data, so there's no
 * per-panel-instance parameterisation to thread through props for.
 */
export interface ColonyWindowContext {
  population: number
  stability: number
  /** Morale scalar in [0, 1] (issue #382) — separate from stability. */
  morale: number
  availableLabour: number
  populationTrend: number[]
  slotsUsed: number
  slotCapacity: number
  labourAvailable: number
  labourTotal: number
  labourEmployed: number
  labourUnemployed: number
  resources: ResourceRow[] | null
  stockpile: StockpileRow[] | null
  buildings: BuildingRow[] | null
  constructionQueue: ConstructionQueueRow[] | null
  cancelingIds: Set<string>
  /** The unified colony log (colony details multi-window redesign) —
   * every event, alert-tier ones colored/toasted, the rest ambient. */
  logEntries: LogEntry[]
  setCommodityReserve(commodityId: string, amount: number): void
  viewBuildingDetails(buildingType: string): void
  setBuildingPriority(buildingId: string, priority: number): void
  setBuildingLock(buildingId: string, lock: number | null): void
  renameBuilding(buildingId: string, name: string | null): void
  setBuildingPaused(buildingId: string, paused: boolean): void
  cancelConstruction(projectId: string): void
  openBuildDialog(): void
  clearLog(): void
}

export const COLONY_WINDOW_CONTEXT_KEY: InjectionKey<ComputedRef<ColonyWindowContext>> =
  Symbol('colonyWindowContext')
