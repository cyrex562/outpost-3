/**
 * Colony screen dock layout (issue #321): panel registry, default
 * arrangement, and localStorage persistence for the `dockview-vue` layout
 * that replaced the Splitpanes-based colony dashboard.
 *
 * Kept dependency-free from Vue/dockview types where practical so the
 * default-layout builder and persistence helpers can be unit tested without
 * mounting a real `DockviewApi` (most of dockview's own drag/drop/resize
 * behaviour isn't assertable in jsdom — see the issue's decision comment).
 */

import type { ComputedRef, InjectionKey } from 'vue'
import type { Notification } from '@/worldModel/model'
import type { ServerEvent } from '@/types/events'
import type { BuildingRow, ConstructionQueueRow, ResourceRow, StockpileRow } from '@/types/screen'

/** Component-registry keys used both by `DockviewVue`'s `:components` map
 * and by `buildDefaultColonyLayout`'s `addPanel` calls — kept as named
 * constants so the two never drift out of sync. */
export const COLONY_DOCK_COMPONENT = {
  vitalStats: 'vital-stats',
  utilities: 'utilities',
  commodities: 'commodities',
  buildings: 'buildings',
  constructionQueue: 'construction-queue',
  alerts: 'alerts',
} as const

export type ColonyDockPanelId = (typeof COLONY_DOCK_COMPONENT)[keyof typeof COLONY_DOCK_COMPONENT]

/** Human-readable tab titles for the default layout / any future panel menu. */
export const COLONY_DOCK_TITLES: Record<ColonyDockPanelId, string> = {
  [COLONY_DOCK_COMPONENT.vitalStats]: 'Vitals',
  [COLONY_DOCK_COMPONENT.utilities]: 'Utilities',
  [COLONY_DOCK_COMPONENT.commodities]: 'Commodities',
  [COLONY_DOCK_COMPONENT.buildings]: 'Buildings',
  [COLONY_DOCK_COMPONENT.constructionQueue]: 'Construction Queue',
  [COLONY_DOCK_COMPONENT.alerts]: 'Alerts',
}

/** Minimal shape `buildDefaultColonyLayout` needs from a `DockviewApi` —
 * just `addPanel`, so it can be exercised with a lightweight test double
 * instead of a real dockview instance. */
export interface DockPanelAdder {
  addPanel(options: {
    id: string
    title?: string
    component: string
    position?: { referencePanel: string; direction: 'left' | 'right' | 'above' | 'below' | 'within' }
  }): unknown
}

/**
 * Build the default colony-screen arrangement: a left column (vitals over
 * utilities over commodities), a center column (buildings over the
 * construction queue), and a right column (alerts) — the same 3-column
 * shape the pre-#321 Splitpanes layout used, just expressed as dockview
 * panels instead of splitter panes.
 */
export function buildDefaultColonyLayout(api: DockPanelAdder): void {
  const c = COLONY_DOCK_COMPONENT
  api.addPanel({ id: c.vitalStats, title: COLONY_DOCK_TITLES[c.vitalStats], component: c.vitalStats })
  api.addPanel({
    id: c.utilities,
    title: COLONY_DOCK_TITLES[c.utilities],
    component: c.utilities,
    position: { referencePanel: c.vitalStats, direction: 'below' },
  })
  api.addPanel({
    id: c.commodities,
    title: COLONY_DOCK_TITLES[c.commodities],
    component: c.commodities,
    position: { referencePanel: c.utilities, direction: 'below' },
  })
  api.addPanel({
    id: c.buildings,
    title: COLONY_DOCK_TITLES[c.buildings],
    component: c.buildings,
    position: { referencePanel: c.vitalStats, direction: 'right' },
  })
  api.addPanel({
    id: c.constructionQueue,
    title: COLONY_DOCK_TITLES[c.constructionQueue],
    component: c.constructionQueue,
    position: { referencePanel: c.buildings, direction: 'below' },
  })
  api.addPanel({
    id: c.alerts,
    title: COLONY_DOCK_TITLES[c.alerts],
    component: c.alerts,
    position: { referencePanel: c.buildings, direction: 'right' },
  })
}

// ─── Layout persistence ─────────────────────────────────────────────────────

/** `.v1` since this replaces the entirely-different Splitpanes-shaped
 * `outpost3.colony-view.layout.v4` key rather than migrating it — a
 * dockview serialized layout and a Splitpanes size-array share no fields. */
export const COLONY_DOCK_STORAGE_KEY = 'outpost3.colony-view.dockview-layout.v1'

/** Read the persisted serialized dockview layout, or `null` if there is
 * none / it's corrupt / storage is unavailable. Callers should fall back to
 * {@link buildDefaultColonyLayout} on `null`. */
export function loadPersistedColonyLayout(): unknown {
  try {
    const raw = window.localStorage.getItem(COLONY_DOCK_STORAGE_KEY)
    if (!raw) return null
    return JSON.parse(raw)
  } catch {
    return null
  }
}

/** Persist a dockview `api.toJSON()` result. Non-fatal on failure (storage
 * full or blocked), matching the rest of this app's layout-persistence
 * helpers. */
export function savePersistedColonyLayout(layout: unknown): void {
  try {
    window.localStorage.setItem(COLONY_DOCK_STORAGE_KEY, JSON.stringify(layout))
  } catch {
    // storage full or blocked — non-fatal
  }
}

/** Clear the persisted layout — used by "Reset Layout" so a fresh
 * `buildDefaultColonyLayout` becomes what's saved going forward. */
export function clearPersistedColonyLayout(): void {
  try {
    window.localStorage.removeItem(COLONY_DOCK_STORAGE_KEY)
  } catch {
    // storage full or blocked — non-fatal
  }
}

// ─── Panel data/handler context (provide/inject) ───────────────────────────

/**
 * Everything the six dock-panel wrapper components need, computed once in
 * `ColonyView.vue` and handed down via `provide`/`inject` rather than
 * through dockview's own `params` mechanism — `dockview-vue` teleports
 * panels so they stay true descendants of the host in the Vue tree
 * (framework features like `provide`/`inject` work natively), and every
 * one of these panels needs the *same* single colony's data, so there's no
 * per-panel-instance parameterisation to thread through `params` anyway.
 */
export interface ColonyDockContext {
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
  notifications: Notification[]
  eventLog: ServerEvent[]
  setCommodityReserve(commodityId: string, amount: number): void
  viewBuildingDetails(buildingType: string): void
  setBuildingPriority(buildingId: string, priority: number): void
  setBuildingLock(buildingId: string, lock: number | null): void
  renameBuilding(buildingId: string, name: string | null): void
  setBuildingPaused(buildingId: string, paused: boolean): void
  cancelConstruction(projectId: string): void
  openBuildDialog(): void
  clearEventLog(): void
}

export const COLONY_DOCK_CONTEXT_KEY: InjectionKey<ComputedRef<ColonyDockContext>> =
  Symbol('colonyDockContext')
