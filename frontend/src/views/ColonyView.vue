<script setup lang="ts">
/**
 * Colony dashboard — a moveable, resizable, dockable multi-panel layout
 * (issue #169, redone as a real dock framework by issue #321) showing
 * population, commodities, buildings, construction queue, and alerts/event
 * log for the selected colony. Panels are laid out with `dockview-vue`
 * (drag-to-dock, tabs, splitters, serialisable layouts) — the previous
 * Splitpanes arrangement (resizable, but not moveable/tabbable/floatable)
 * is retired; see `docs/DESIGN.md` for the decision record.
 *
 * The six always-present panel components are wrapped by thin
 * `Dock*Panel.vue` components (registered with `DockviewVue`) that read
 * colony data via `provide`/`inject` (`colonyDock.ts`) rather than through
 * dockview's own per-panel `params`, since every one of them shows the same
 * single colony and there's nothing to parameterise per panel instance.
 *
 * Building details are deliberately *not* one of those dock panels (issue
 * #322, revised): they open in a `FloatingWindow` layered above the whole
 * dock instead, so the detail view can float over — rather than compete for
 * space inside — the docked arrangement, and closing it can't disturb
 * whatever layout the player has dragged/resized the six panels into.
 *
 * Turn control (Advance Turn) and system-wide stats now live in the app
 * shell's footer/stats bars (UI-rework PR3), not in this view.
 */

import { computed, onMounted, onUnmounted, provide, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { DockviewVue } from 'dockview-vue'
import type { DockviewApi, DockviewReadyEvent, VueComponent } from 'dockview-vue'
import 'dockview-vue/dist/styles/dockview.css'
import { useWorldStore } from '@/stores/worldStore'
import { useGameStore } from '@/stores/game'
import BuildDialog from '@/components/BuildDialog.vue'
import FloatingWindow from '@/components/FloatingWindow.vue'
import BuildingDetailsHud from '@/components/BuildingDetailsHud.vue'
import DockVitalStatsPanel from '@/components/dock/DockVitalStatsPanel.vue'
import DockUtilitiesPanel from '@/components/dock/DockUtilitiesPanel.vue'
import DockCommoditiesPanel from '@/components/dock/DockCommoditiesPanel.vue'
import DockBuildingsPanel from '@/components/dock/DockBuildingsPanel.vue'
import DockConstructionQueuePanel from '@/components/dock/DockConstructionQueuePanel.vue'
import DockAlertsPanel from '@/components/dock/DockAlertsPanel.vue'
import {
  COLONY_DOCK_COMPONENT,
  COLONY_DOCK_CONTEXT_KEY,
  buildDefaultColonyLayout,
  loadPersistedColonyLayout,
  savePersistedColonyLayout,
  type ColonyDockContext,
} from '@/dock/colonyDock'
import type { ColonyState } from '@/worldModel/model'
import {
  isTauri,
  listBuildings,
  getTechTree,
  type BuildingOption,
  type TechNode,
} from '@/services/tauriBridge'

const worldStore = useWorldStore()
const gameStore = useGameStore()
const route = useRoute()
const router = useRouter()

// ─── Colony selection ─────────────────────────────────────────────────────────
//
// The route's `:colonyId` param (issue #7 navigation rework, phase 1) is the
// source of truth for which colony is being viewed — this makes the URL
// itself a back/forward-navigable, deep-linkable representation of
// selection, rather than selection living only in `gameStore` (which had no
// way to distinguish "no colony selected yet" from "browser back button
// pressed"). `gameStore.selectedColonyId` is kept in sync below (via watch)
// purely so existing consumers — its own colonyScreen-refresh watcher, and
// any other view still reading it directly — keep working unchanged.

const colonies = computed(() => worldStore.colonies)

const routeColonyId = computed((): string | null => {
  const raw = route.params.colonyId
  return typeof raw === 'string' && raw.length > 0 ? raw : null
})

const selectedColony = computed((): ColonyState | null => {
  const id = routeColonyId.value ?? gameStore.selectedColonyId
  if (!id) return colonies.value[0] ?? null
  return worldStore.world.colonies[id] ?? colonies.value[0] ?? null
})

watch(
  () => selectedColony.value?.id ?? null,
  (id) => {
    if (!id) return
    gameStore.selectedColonyId = id
    // Keep the URL in sync with whatever actually resolved — covers the
    // bare `/colony` landing case (no param yet), colonies loading in
    // asynchronously after mount (worldStore starts empty and populates
    // reactively, so a mount-only check would miss this), and the routed
    // colony having disappeared from `worldStore` (its id fell through to
    // `colonies.value[0]` in the computed above) — in every case, `id` is
    // the source of truth and any mismatch gets corrected (replace, not
    // push: these are all "the URL was wrong," not user-initiated
    // navigation the user should be able to back out of).
    if (id !== routeColonyId.value) {
      void router.replace({ name: 'colony', params: { colonyId: id } })
    }
  },
  { immediate: true },
)

/** Return to the planet map — the hub for switching between colonies
 * (map/nav plan phase A2, which replaced the old per-colony tab bar with
 * map-driven navigation: click a colony node on `/planet` to open it). */
function goToPlanetMap(): void {
  void router.push({ name: 'planet' })
}

/** The colony screen data, but only when it matches the selected colony. */
const screen = computed(() => {
  const scr = gameStore.colonyScreen
  if (!scr || scr.colony_id !== selectedColony.value?.id) return null
  return scr
})

// ─── Population trend ─────────────────────────────────────────────────────────

/**
 * Produce population samples for the sparkline. Uses the last 10
 * notifications of type needs_resolved to approximate trend.
 */
const populationTrend = computed((): number[] => {
  const notifications = worldStore.notifications
  const samples: number[] = []
  for (let i = notifications.length - 1; i >= 0 && samples.length < 10; i--) {
    const n = notifications[i]
    if (n.colony_id === selectedColony.value?.id) {
      // Use a dummy value since we only have the message; sparkline is indicative.
      samples.unshift(0)
    }
  }
  if (samples.length === 0) return [0, 0, 0]
  return samples
})

// ─── Construction queue: catalog + queueing ────────────────────────────────────

const queueBusy = ref(false)

/** Whether the build dialog (catalog + quantity picker) is open. */
const showBuildDialog = ref(false)

/** Full building catalog from the loaded content pack. */
const buildingCatalog = ref<BuildingOption[]>([])

/** Tech node states — used to decide which buildings are tech-unlocked. */
const techNodes = ref<TechNode[]>([])

/** Set of tech ids the player has researched. */
const researchedTechs = computed<Set<string>>(
  () => new Set(techNodes.value.filter((t) => t.state === 'researched').map((t) => t.id)),
)

/** Slots not currently reserved by an active building or in-flight project. */
const slotsAvailable = computed<number | null>(() => (screen.value ? screen.value.slot_capacity - screen.value.slots_used : null))

async function loadCatalog(): Promise<void> {
  if (!isTauri) return
  try {
    const [buildings, techs] = await Promise.all([listBuildings(), getTechTree()])
    buildingCatalog.value = buildings
    techNodes.value = techs
  } catch {
    // catalog / tech tree may fail to load if engine isn't ready — ignore
  }
}

onMounted(loadCatalog)
// The catalog is pack-static but tech state changes over time. Refresh
// whenever the player switches colonies so a differently-selected colony
// sees the same tech gates (currently a system-wide pool anyway).
watch(() => gameStore.selectedColonyId, loadCatalog)

/**
 * Return `null` when the building can be queued, or a short human-readable
 * reason (used as button tooltip + disabled state) otherwise.
 */
function disabledReason(b: BuildingOption): string | null {
  if (b.tech_prerequisite && !researchedTechs.value.has(b.tech_prerequisite)) {
    return `Requires: ${b.tech_prerequisite}`
  }
  const free = slotsAvailable.value
  if (free !== null && b.slot_cost > free) {
    return `Needs ${b.slot_cost} slot${b.slot_cost === 1 ? '' : 's'}, ${free} free`
  }
  return null
}

/**
 * Queue `quantity` copies of a building. The build dialog lets the player
 * pick a count, so this dispatches one `queue_construction` per copy (there's
 * no batch command); each is a separate project the engine schedules in turn.
 */
async function queueBuilding(b: BuildingOption, quantity = 1): Promise<void> {
  const col = selectedColony.value
  if (!col || queueBusy.value || disabledReason(b) !== null) return
  const count = Math.max(1, Math.floor(quantity))
  queueBusy.value = true
  try {
    for (let i = 0; i < count; i++) {
      await gameStore.sendCommand({
        kind: 'queue_construction',
        colony_id: col.id,
        building_type: b.id,
        slot_cost: b.slot_cost,
        labor_per_turn: b.labor_per_turn,
        construction_cost: b.construction_cost,
        construction_turns: b.construction_turns,
      })
    }
  } finally {
    queueBusy.value = false
  }
}

// ─── Construction queue: cancelling ────────────────────────────────────────────

/** Project ids with a cancel request currently in flight. */
const cancelingIds = ref<Set<string>>(new Set())

async function cancelConstruction(projectId: string): Promise<void> {
  const col = selectedColony.value
  if (!col || cancelingIds.value.has(projectId)) return
  cancelingIds.value = new Set(cancelingIds.value).add(projectId)
  try {
    await gameStore.sendCommand({
      kind: 'cancel_construction',
      colony_id: col.id,
      project_id: projectId,
    })
  } finally {
    const next = new Set(cancelingIds.value)
    next.delete(projectId)
    cancelingIds.value = next
  }
}

// ─── Building details (issue #182; floating window per #339, revised #322) ─────
//
// Selecting a building opens `BuildingDetailsHud` in a `FloatingWindow`
// layered above the whole colony dashboard (dock panels included) rather
// than navigating to a routed page or opening as a dock panel of its own —
// a single reusable window that retargets to whichever building was last
// clicked, closeable without disturbing the docked layout underneath.
// `/colony/:colonyId/facility/:buildingType` stays live as a deep link — see
// the `routeBuildingType` watcher below — rather than routing to a separate
// page.

const selectedBuildingType = ref<string | null>(null)

function openBuildingDetails(buildingType: string): void {
  const col = selectedColony.value
  if (!col) return
  selectedBuildingType.value = buildingType
}

function closeBuildingDetails(): void {
  selectedBuildingType.value = null
}

// ─── Per-building staffing (issue #307) ────────────────────────────────────────
//
// The panel raises intents; dispatch lives here. `gameStore.sendCommand` already
// refreshes the colony screen and surfaces engine rejections as a toast, so a
// rejected priority or an over-sized pin tells the player why rather than
// failing silently.

async function setBuildingPriority(buildingId: string, priority: number): Promise<void> {
  const col = selectedColony.value
  if (!col) return
  await gameStore.sendCommand({
    kind: 'set_building_priority',
    colony_id: col.id,
    building_id: buildingId,
    priority,
  })
}

async function setBuildingLock(buildingId: string, lock: number | null): Promise<void> {
  const col = selectedColony.value
  if (!col) return
  await gameStore.sendCommand({
    kind: 'set_building_labour_lock',
    colony_id: col.id,
    building_id: buildingId,
    lock,
  })
}

/**
 * Withhold `amount` of a commodity from industry, or clear it with `0` (#308).
 *
 * Same intent-up/dispatch-here split as the staffing controls above, and the
 * same error path: `sendCommand` refreshes the screen and toasts an engine
 * rejection, so a bad amount tells the player rather than silently reverting.
 */
async function setCommodityReserve(commodityId: string, amount: number): Promise<void> {
  const col = selectedColony.value
  if (!col) return
  await gameStore.sendCommand({
    kind: 'set_commodity_reserve',
    colony_id: col.id,
    commodity_id: commodityId,
    amount,
  })
}

async function renameBuilding(buildingId: string, name: string | null): Promise<void> {
  const col = selectedColony.value
  if (!col) return
  await gameStore.sendCommand({
    kind: 'rename_building',
    colony_id: col.id,
    building_id: buildingId,
    name,
  })
}

/**
 * Pause or resume a building (issue #309). Same intent-up/dispatch-here split
 * as the staffing controls above.
 */
async function setBuildingPaused(buildingId: string, paused: boolean): Promise<void> {
  const col = selectedColony.value
  if (!col) return
  await gameStore.sendCommand({
    kind: 'set_building_paused',
    colony_id: col.id,
    building_id: buildingId,
    paused,
  })
}

// ─── Dock panel context (issue #321) ────────────────────────────────────────
//
// A single computed bag, provided to the whole dockview subtree, that every
// `Dock*Panel.vue` wrapper reads via `inject`. See `colonyDock.ts`'s
// `ColonyDockContext` doc comment for why provide/inject rather than
// dockview's own `params`.

const dockContext = computed<ColonyDockContext>(() => ({
  population: selectedColony.value?.population ?? 0,
  stability: selectedColony.value?.stability ?? 0,
  availableLabour: selectedColony.value?.available_labour ?? 0,
  populationTrend: populationTrend.value,
  slotsUsed: screen.value?.slots_used ?? 0,
  slotCapacity: screen.value?.slot_capacity ?? 0,
  labourAvailable: screen.value?.labour_available ?? 0,
  labourTotal: screen.value?.labour_total ?? 0,
  labourEmployed: screen.value?.labour_employed ?? 0,
  labourUnemployed: screen.value?.labour_unemployed ?? 0,
  resources: screen.value ? screen.value.resources : null,
  stockpile: screen.value ? screen.value.stockpile : null,
  buildings: screen.value ? screen.value.buildings : null,
  constructionQueue: screen.value ? screen.value.construction_queue : null,
  cancelingIds: cancelingIds.value,
  notifications: worldStore.notifications,
  eventLog: worldStore.eventLog,
  setCommodityReserve,
  viewBuildingDetails: openBuildingDetails,
  setBuildingPriority,
  setBuildingLock,
  renameBuilding,
  setBuildingPaused,
  cancelConstruction,
  openBuildDialog: () => {
    showBuildDialog.value = true
  },
  clearEventLog: () => worldStore.clearEventLog(),
}))

provide(COLONY_DOCK_CONTEXT_KEY, dockContext)

// ─── Dock layout wiring + persistence (issue #321) ──────────────────────────

const dockApi = ref<DockviewApi | null>(null)

// `dockview-vue`'s own `VueComponent` type isn't compatible with this
// project's `exactOptionalPropertyTypes: true` tsconfig setting (a type-only
// mismatch between the library and our stricter config, not a real runtime
// concern) — cast through `unknown` rather than loosening the project-wide
// compiler option for one third-party type.
const dockComponents = {
  [COLONY_DOCK_COMPONENT.vitalStats]: DockVitalStatsPanel,
  [COLONY_DOCK_COMPONENT.utilities]: DockUtilitiesPanel,
  [COLONY_DOCK_COMPONENT.commodities]: DockCommoditiesPanel,
  [COLONY_DOCK_COMPONENT.buildings]: DockBuildingsPanel,
  [COLONY_DOCK_COMPONENT.constructionQueue]: DockConstructionQueuePanel,
  [COLONY_DOCK_COMPONENT.alerts]: DockAlertsPanel,
} as unknown as Record<string, VueComponent>

/** Disposes the `onDidLayoutChange` subscription registered in `onDockReady`. */
let dockLayoutChangeSubscription: { dispose(): void } | null = null

function onDockReady(event: DockviewReadyEvent): void {
  dockApi.value = event.api
  // Registered before the layout is built so the *initial* default
  // arrangement gets persisted too — attaching this after
  // `buildDefaultColonyLayout` would mean nothing is saved until the
  // player makes their own change, leaving a first-launch persistence gap.
  dockLayoutChangeSubscription = event.api.onDidLayoutChange(() => {
    savePersistedColonyLayout(event.api.toJSON())
  })
  const saved = loadPersistedColonyLayout()
  let restored = false
  if (saved) {
    try {
      event.api.fromJSON(saved as Parameters<DockviewApi['fromJSON']>[0])
      restored = true
    } catch {
      // corrupt/incompatible persisted layout — fall through to the default
    }
  }
  if (!restored) {
    buildDefaultColonyLayout(event.api)
  }
}

/**
 * `/colony/:colonyId/facility/:buildingType` (also reachable via the
 * `facility` route name, e.g. from `BuildingsListView`) is kept as a
 * deep link into the floating building-details window rather than its own
 * routed page — both route names resolve to this same component, so
 * visiting either just opens the window. Re-fires when the route's
 * `buildingType` actually changes (not once at mount), so a second facility
 * link clicked while already on this colony retargets the window instead of
 * being a no-op.
 *
 * Deliberately watches only `routeBuildingType`, not `selectedColony` —
 * `selectedColony` is a computed over `worldStore.world`, which is replaced
 * wholesale on every server-pushed event (any sol advance, any command from
 * any client), so including it here would re-run this watcher — and
 * therefore reopen the window — on unrelated world ticks, even after the
 * player closed it via its own close button. `selectedColony` is still
 * checked, just inside the callback rather than as a tracked dependency, so
 * this stays a no-op until a colony resolves.
 */
const routeBuildingType = computed((): string | null => {
  const raw = route.params.buildingType
  return typeof raw === 'string' && raw.length > 0 ? raw : null
})

watch(
  routeBuildingType,
  (buildingType) => {
    if (!buildingType || !selectedColony.value) return
    selectedBuildingType.value = buildingType
  },
  { immediate: true },
)

/** Discard the current arrangement and rebuild the default 3-column layout
 * — the "remaining detail" question from the issue's decision comment about
 * a panel menu is left for a follow-up, but a reset affordance was called
 * out as worth having regardless. Doesn't call `clearPersistedColonyLayout`
 * directly: `api.clear()` and each `addPanel` below already fire
 * `onDidLayoutChange`, which persists the rebuilt default as the new saved
 * state once `buildDefaultColonyLayout` finishes — an explicit clear first
 * would only add a transient empty-layout write in between. */
function resetLayout(): void {
  if (!dockApi.value) return
  dockApi.value.clear()
  buildDefaultColonyLayout(dockApi.value)
}

onUnmounted(() => {
  dockLayoutChangeSubscription?.dispose()
})
</script>

<template>
  <div class="colony-view">
    <div v-if="colonies.length === 0" class="empty-state" data-testid="no-colonies">
      No colonies founded yet. Found one from the system map.
    </div>

    <template v-else>
      <div class="colony-header">
        <div class="colony-titlebar">
          <button
            class="btn-map"
            data-testid="btn-planet-map"
            title="Back to the planet map to switch colonies"
            @click="goToPlanetMap"
          >
            ← Planet Map
          </button>
          <h2 class="colony-title" data-testid="colony-title">{{ selectedColony?.name }}</h2>
        </div>
        <button
          class="btn-reset-layout"
          data-testid="btn-reset-layout"
          title="Discard your dragged/resized/floated panel arrangement and restore the default layout"
          @click="resetLayout"
        >
          Reset Layout
        </button>
      </div>

      <!-- `.colony-body` is `FloatingWindow`'s host (its own dedicated
           `position: relative` container per its own convention — see
           `PlanetView.vue`'s `.map-host` — rather than the whole
           `.colony-view`, which also holds `.colony-header`'s controls; a
           `fill-host` window would otherwise be free to overlap them) and
           traps dockview's own internal stacking context via `isolation:
           isolate` — dockview's *own* float-a-tab feature (a player
           dragging a dock tab loose, independent of this building-details
           window) uses z-index values in the high 900s internally, which
           would otherwise escape past this window's much lower z-index
           since nothing between them establishes a stacking context. -->
      <div class="colony-body">
        <!-- Moveable, resizable, dockable panel layout (issue #321) —
             replaces the old Splitpanes 3-column arrangement with a real
             dock framework: panels can be dragged to new positions,
             resized, stacked as tabs, or popped into floating groups, and
             the whole arrangement persists to localStorage (see
             `colonyDock.ts`). -->
        <div v-if="selectedColony" class="panel-layout" :data-testid="`colony-detail-${selectedColony.id}`">
          <DockviewVue
            class="dockview-theme-abyss colony-dockview"
            data-testid="colony-dockview"
            :components="dockComponents"
            @ready="onDockReady"
          />
        </div>

        <!-- Building details float above the whole dock (issue #322,
             revised) rather than living inside it, so inspecting a building
             never disturbs the dragged/resized dock arrangement
             underneath. -->
        <FloatingWindow
          v-if="selectedBuildingType && selectedColony"
          :title="selectedBuildingType"
          storage-key="outpost3.colony-view.building-details-window"
          closable
          fill-host
          :initial-x="40"
          :initial-y="40"
          :initial-width="520"
          :initial-height="480"
          @close="closeBuildingDetails"
        >
          <BuildingDetailsHud
            owner-type="colony"
            :owner-id="selectedColony.id"
            :building-type="selectedBuildingType"
            as-page
            @close="closeBuildingDetails"
          />
        </FloatingWindow>
      </div>

      <BuildDialog
        v-if="showBuildDialog"
        :catalog="buildingCatalog"
        :disabled-reason="disabledReason"
        :slots-available="slotsAvailable"
        :busy="queueBusy"
        @queue="queueBuilding"
        @close="showBuildDialog = false"
      />
    </template>
  </div>
</template>


<style scoped>
/* Fill the shell's main region (UI-rework PR4): a flex column whose panel
   area (`.panel-layout`) grows to take all remaining height, so the colony
   dashboard fills the screen without a fixed viewport-height guess. */
.colony-view { width: 100%; height: 100%; display: flex; flex-direction: column; position: relative; }

.empty-state { color: #666; font-style: italic; margin: 1rem 0; }

.colony-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 1rem;
  margin-bottom: 0.75rem;
  flex-wrap: wrap;
}

.colony-titlebar { display: flex; align-items: center; gap: 0.75rem; }
.colony-title { color: #8cf; font-size: 1.05rem; margin: 0; }
.btn-map {
  background: #151520;
  border: 1px solid #446;
  border-radius: 3px;
  color: #8cf;
  padding: 0.3rem 0.7rem;
  font-family: monospace;
  font-size: 0.8rem;
  cursor: pointer;
  white-space: nowrap;
}
.btn-map:hover { background: #1a1a2a; border-color: #558; }

.btn-reset-layout {
  background: #151520;
  border: 1px solid #446;
  border-radius: 3px;
  color: #8cf;
  padding: 0.3rem 0.7rem;
  font-family: monospace;
  font-size: 0.75rem;
  cursor: pointer;
  white-space: nowrap;
  align-self: flex-start;
}
.btn-reset-layout:hover { background: #1a1a2a; border-color: #558; }

/* `FloatingWindow`'s host (see the template comment above it): a dedicated
   `position: relative` container for the dock + the building-details
   window, excluding `.colony-header`'s controls. `isolation: isolate`
   traps dockview's own internal stacking context (its float-a-tab feature
   uses z-index values in the high 900s) so it can't escape past the
   building-details window's much lower z-index. */
.colony-body {
  flex: 1;
  min-height: 320px;
  display: flex;
  position: relative;
  isolation: isolate;
}

.panel-layout {
  flex: 1;
  border: 1px solid #223;
  border-radius: 4px;
  overflow: hidden;
}
</style>

<style>
/* Unscoped — `DockviewVue`'s own root element isn't part of this
   component's scoped-CSS output (it's a third-party component's root, not
   an element this template literally emits), so a `<style scoped>` sizing
   rule silently never applies to it and the host collapses to 0 height.
   Layered on top of the built-in `dockview-theme-abyss` theme rather than
   replacing it wholesale. */
.colony-dockview {
  width: 100%;
  height: 100%;
}
.colony-dockview.dockview-theme-abyss {
  --dv-background-color: #0d0d15;
  --dv-group-view-background-color: #0d0d15;
  --dv-tabs-and-actions-container-background-color: #14141e;
  --dv-activegroup-visiblepanel-tab-background-color: #1a1a2a;
  --dv-inactivegroup-visiblepanel-tab-background-color: #14141e;
  --dv-tab-divider-color: #223;
  --dv-separator-border: #223;
}
</style>
