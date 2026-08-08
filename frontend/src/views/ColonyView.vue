<script setup lang="ts">
/**
 * Colony dashboard — a multi-window HUD (colony details multi-window
 * redesign) showing population, commodities, buildings, construction queue,
 * and alerts/event log for the selected colony. Each of the six panels
 * opens in its own `FloatingWindow`, all sharing one `FloatingWindowRegistry`
 * (`floatingWindowRegistry.ts`) so they snap against each other's edges and
 * maintain a click-to-front z-order, floating above `.base-view` — a plain,
 * always-visible backdrop rather than a seventh panel — rather than tiling
 * the whole screen as the previous `dockview-vue` layout did (issue #321;
 * see `docs/DESIGN.md` for the decision record). Closing a window doesn't
 * lose it: `ColonyWindowPalette` in the header reopens (or refocuses) any
 * of the six by id.
 *
 * The six panel components are wrapped by thin `*WindowPanel.vue`
 * components that read colony data via `provide`/`inject`
 * (`colonyWindows.ts`) rather than through props, since every one of them
 * shows the same single colony and there's nothing to parameterise per
 * window instance.
 *
 * Building details use the same `FloatingWindow`/registry machinery but are
 * NOT one of the six palette panels (issue #322, revised): selecting a
 * building opens a window that retargets to whichever building was last
 * clicked, rather than living in the palette's fixed six.
 *
 * Turn control (Advance Turn) and system-wide stats now live in the app
 * shell's footer/stats bars (UI-rework PR3), not in this view.
 */

import { computed, onMounted, provide, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useWorldStore } from '@/stores/worldStore'
import { useGameStore } from '@/stores/game'
import BuildDialog from '@/components/BuildDialog.vue'
import FloatingWindow from '@/components/FloatingWindow.vue'
import BuildingDetailsHud from '@/components/BuildingDetailsHud.vue'
import ColonyWindowPalette from '@/components/windows/ColonyWindowPalette.vue'
import VitalStatsWindowPanel from '@/components/windows/VitalStatsWindowPanel.vue'
import UtilitiesWindowPanel from '@/components/windows/UtilitiesWindowPanel.vue'
import CommoditiesWindowPanel from '@/components/windows/CommoditiesWindowPanel.vue'
import BuildingsWindowPanel from '@/components/windows/BuildingsWindowPanel.vue'
import ConstructionQueueWindowPanel from '@/components/windows/ConstructionQueueWindowPanel.vue'
import AlertsWindowPanel from '@/components/windows/AlertsWindowPanel.vue'
import {
  FLOATING_WINDOW_REGISTRY_KEY,
  createFloatingWindowRegistry,
} from '@/composables/floatingWindowRegistry'
import {
  COLONY_WINDOW,
  COLONY_WINDOW_IDS,
  COLONY_WINDOW_TITLES,
  COLONY_WINDOW_DEFAULT_RECT,
  COLONY_WINDOW_CONTEXT_KEY,
  colonyWindowStorageKey,
  clearAllColonyWindowGeometry,
  loadPersistedOpenWindowIds,
  savePersistedOpenWindowIds,
  clearPersistedOpenWindowIds,
  type ColonyWindowId,
  type ColonyWindowContext,
} from '@/windows/colonyWindows'
import type { ColonyState } from '@/worldModel/model'
import {
  listBuildings,
  getTechTree,
  type BuildingOption,
  type TechNode,
} from '@/services/tauriBridge'

/** Vue component per window id — the same six wrappers previously
 * registered with `DockviewVue`'s `:components` map, now rendered directly. */
const WINDOW_COMPONENT: Record<ColonyWindowId, unknown> = {
  [COLONY_WINDOW.vitalStats]: VitalStatsWindowPanel,
  [COLONY_WINDOW.utilities]: UtilitiesWindowPanel,
  [COLONY_WINDOW.commodities]: CommoditiesWindowPanel,
  [COLONY_WINDOW.buildings]: BuildingsWindowPanel,
  [COLONY_WINDOW.constructionQueue]: ConstructionQueueWindowPanel,
  [COLONY_WINDOW.alerts]: AlertsWindowPanel,
}

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

/** Why the catalog is empty, when it failed rather than genuinely being so. */
const catalogError = ref<string | null>(null)

/** Set of tech ids the player has researched. */
const researchedTechs = computed<Set<string>>(
  () => new Set(techNodes.value.filter((t) => t.state === 'researched').map((t) => t.id)),
)

/** Slots not currently reserved by an active building or in-flight project. */
const slotsAvailable = computed<number | null>(() => (screen.value ? screen.value.slot_capacity - screen.value.slots_used : null))

/**
 * Why this isn't a `Promise.all` in a silent `try/catch` any more.
 *
 * It used to be, and that combination hid a real bug for a whole playtest
 * round: when the backend had no content registry, `listBuildings` rejected,
 * the catch swallowed it, and the build dialog rendered "No buildings
 * available in the loaded content pack" — indistinguishable from a genuinely
 * empty pack, with nothing in the console or the log to say otherwise.
 *
 * Two changes: the calls settle independently, so a failing tech tree no
 * longer costs the player the entire building catalog; and a failure is
 * recorded and shown rather than discarded.
 *
 * There is no `isTauri` guard: both calls have browser-mode paths of their
 * own (`/api/buildings`, `/api/tech-tree`).
 */
async function loadCatalog(): Promise<void> {
  catalogError.value = null
  const [buildings, techs] = await Promise.allSettled([listBuildings(), getTechTree()])

  if (buildings.status === 'fulfilled') {
    buildingCatalog.value = buildings.value
  } else {
    const message = buildings.reason instanceof Error ? buildings.reason.message : String(buildings.reason)
    catalogError.value = `Could not load the building catalog: ${message}`
    console.error('[colony] building catalog failed to load:', buildings.reason)
  }

  if (techs.status === 'fulfilled') {
    techNodes.value = techs.value
  } else {
    // Non-fatal on its own: without the tech list every tech-gated building
    // reads as locked, but the catalog itself is still usable.
    console.error('[colony] tech tree failed to load:', techs.reason)
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
  if (b.max_instances !== null && existingCount(b) >= b.max_instances) {
    return b.max_instances === 1
      ? 'Limit 1 per colony — already built'
      : `Limit ${b.max_instances} per colony — already built`
  }
  const free = slotsAvailable.value
  if (free !== null && b.slot_cost > free) {
    return `Needs ${b.slot_cost} slot${b.slot_cost === 1 ? '' : 's'}, ${free} free`
  }
  return null
}

/**
 * How many of `b` this colony already has, counting queued projects as well as
 * standing buildings — the same tally the engine uses for
 * `BuildingDef::max_instances`. Counting only completed ones would let the
 * player queue a second copy that then fails on completion.
 *
 * Reads the colony *screen* rather than `ColonyState`: the latter's
 * `buildings` comes from `/api/colonies`, which returns it empty, and its
 * entries would be instance ids rather than building types even when
 * populated. The screen's rows carry `building_type`, which is what the cap is
 * keyed on.
 */
function existingCount(b: BuildingOption): number {
  const scr = screen.value
  if (!scr) return 0
  const built = scr.buildings.filter((row) => row.building_type === b.id).length
  const queued = scr.construction_queue.filter((row) => row.building_type === b.id).length
  return built + queued
}

/**
 * Copies of `b` this colony may still queue, or `null` when unlimited. The
 * build dialog offers a quantity, and it dispatches one command per copy, so
 * without this a "build 5" on a capped building would succeed once and then
 * error four times.
 */
function remainingAllowance(b: BuildingOption): number | null {
  if (b.max_instances === null) return null
  return Math.max(0, b.max_instances - existingCount(b))
}

/**
 * Queue `quantity` copies of a building. The build dialog lets the player
 * pick a count, so this dispatches one `queue_construction` per copy (there's
 * no batch command); each is a separate project the engine schedules in turn.
 */
async function queueBuilding(b: BuildingOption, quantity = 1): Promise<void> {
  const col = selectedColony.value
  if (!col || queueBusy.value || disabledReason(b) !== null) return
  // Clamp to what the colony's instance cap still allows, so asking for five
  // of a capped building queues the one it can rather than erroring four times.
  const allowance = remainingAllowance(b)
  const requested = Math.max(1, Math.floor(quantity))
  const count = allowance === null ? requested : Math.min(requested, allowance)
  if (count < 1) return
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

// ─── Window panel context (colony details multi-window redesign) ───────────
//
// A single computed bag, provided to the whole view, that every
// `*WindowPanel.vue` wrapper reads via `inject`. See `colonyWindows.ts`'s
// `ColonyWindowContext` doc comment for why provide/inject rather than props.

const windowContext = computed<ColonyWindowContext>(() => ({
  population: selectedColony.value?.population ?? 0,
  stability: selectedColony.value?.stability ?? 0,
  // Morale has no event-stream plumbing yet (issue #382) — sourced from the
  // query-driven `screen` (ColonyScreenData), refreshed the same way
  // stability's siblings below (labour, resources, ...) already are.
  morale: screen.value?.morale ?? 0,
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
  logEntries: worldStore.logEntries,
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
  clearLog: () => worldStore.clearLog(),
}))

provide(COLONY_WINDOW_CONTEXT_KEY, windowContext)

// ─── Window open/close state + shared snap/z-order registry ────────────────
//
// One registry per mounted ColonyView, shared by every FloatingWindow in
// `.colony-body` (the six panel windows below, plus the building-details
// window) via provide/inject — see `floatingWindowRegistry.ts`.

provide(FLOATING_WINDOW_REGISTRY_KEY, createFloatingWindowRegistry())

/** Which of the six panel windows are currently open. Defaults to all six —
 * the pre-redesign dock's always-all-visible starting point. Reassigned
 * wholesale (never mutated in place) on every toggle, matching this file's
 * existing `cancelingIds` convention — Vue *does* track in-place `Set`
 * mutations here too, this is just consistency with that convention, not a
 * reactivity requirement. */
const openWindowIds = ref<Set<ColonyWindowId>>(
  new Set(loadPersistedOpenWindowIds() ?? COLONY_WINDOW_IDS),
)

watch(openWindowIds, (ids) => savePersistedOpenWindowIds(Array.from(ids)))

/** Toggled by the tool palette: opens a closed window, closes an open one. */
function toggleWindow(id: ColonyWindowId): void {
  const next = new Set(openWindowIds.value)
  if (next.has(id)) next.delete(id)
  else next.add(id)
  openWindowIds.value = next
}

/** A window's own close (×) button — always closes, never toggles-open. */
function closeWindow(id: ColonyWindowId): void {
  const next = new Set(openWindowIds.value)
  next.delete(id)
  openWindowIds.value = next
}

/**
 * Bumped on every "Reset Layout" and folded into each panel window's `:key`
 * — clearing a window's persisted geometry (below) doesn't do anything for
 * a window that's already mounted, since `FloatingWindow` only reads
 * persisted state at mount time. Changing `:key` forces Vue to destroy and
 * recreate every open window's `FloatingWindow` instance, so the fresh
 * mount reads the now-cleared (and thus default) geometry.
 */
const layoutResetNonce = ref(0)

/** Discard every panel window's dragged/resized/closed state and restore
 * the default arrangement, all open. */
function resetLayout(): void {
  clearPersistedOpenWindowIds()
  clearAllColonyWindowGeometry()
  openWindowIds.value = new Set(COLONY_WINDOW_IDS)
  layoutResetNonce.value += 1
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
        <ColonyWindowPalette :open-ids="openWindowIds" @toggle="toggleWindow" />
        <button
          class="btn-reset-layout"
          data-testid="btn-reset-layout"
          title="Discard every window's dragged/resized/closed state and restore the default arrangement"
          @click="resetLayout"
        >
          Reset Layout
        </button>
      </div>

      <!-- `.colony-body` is every FloatingWindow's shared host (its own
           dedicated `position: relative` container per `FloatingWindow`'s
           own convention — see `PlanetView.vue`'s `.map-host` — rather than
           the whole `.colony-view`, which also holds `.colony-header`'s
           controls; a `fill-host` window would otherwise be free to overlap
           them). `.base-view` is the plain backdrop the windows float
           above — colony details itself, not a seventh panel — painted
           first so it sits behind every window regardless of z-order
           (FloatingWindow's own z-index, from the shared registry, always
           beats an unset one). -->
      <div class="colony-body">
        <div v-if="selectedColony" class="base-view" :data-testid="`colony-detail-${selectedColony.id}`">
          <div class="base-view-name">{{ selectedColony.name }}</div>
          <div class="base-view-meta">
            Population {{ Math.round(selectedColony.population) }}
            · Stability {{ (selectedColony.stability * 100).toFixed(0) }}%
          </div>
        </div>

        <!-- The six panel windows — moveable, resizable, edge-snapping
             (colony details multi-window redesign). Only the currently-open
             ones mount; the tool palette above reopens a closed one. -->
        <FloatingWindow
          v-for="id in openWindowIds"
          :key="`${id}-${layoutResetNonce}`"
          :window-id="id"
          :title="COLONY_WINDOW_TITLES[id]"
          :storage-key="colonyWindowStorageKey(id)"
          :initial-x="COLONY_WINDOW_DEFAULT_RECT[id].x"
          :initial-y="COLONY_WINDOW_DEFAULT_RECT[id].y"
          :initial-width="COLONY_WINDOW_DEFAULT_RECT[id].w"
          :initial-height="COLONY_WINDOW_DEFAULT_RECT[id].h"
          closable
          @close="closeWindow(id)"
        >
          <component :is="WINDOW_COMPONENT[id]" />
        </FloatingWindow>

        <!-- Building details float above everything else (issue #322,
             revised): selecting a building opens this window rather than
             disturbing any panel window's own position, and it shares the
             same snap/z-order registry as the six above. -->
        <FloatingWindow
          v-if="selectedBuildingType && selectedColony"
          window-id="building-details"
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
        :catalog-error="catalogError"
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
   area (`.colony-body`) grows to take all remaining height, so the colony
   dashboard fills the screen without a fixed viewport-height guess. */
.colony-view { width: 100%; height: 100%; display: flex; flex-direction: column; position: relative; }

.empty-state { color: var(--text-muted); font-style: italic; margin: 1rem 0; }

.colony-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 1rem;
  margin-bottom: 0.75rem;
  flex-wrap: wrap;
}

.colony-titlebar { display: flex; align-items: center; gap: 0.75rem; }
.colony-title { color: var(--accent); font-size: 1.05rem; margin: 0; }
.btn-map {
  background: var(--surface-2);
  border: 1px solid var(--border-strong);
  border-radius: 3px;
  color: var(--accent);
  padding: 0.3rem 0.7rem;
  font-family: monospace;
  font-size: 0.8rem;
  cursor: pointer;
  white-space: nowrap;
}
.btn-map:hover { background: var(--surface-alt); border-color: var(--text-faint); }

.btn-reset-layout {
  background: var(--surface-2);
  border: 1px solid var(--border-strong);
  border-radius: 3px;
  color: var(--accent);
  padding: 0.3rem 0.7rem;
  font-family: monospace;
  font-size: 0.75rem;
  cursor: pointer;
  white-space: nowrap;
  align-self: flex-start;
}
.btn-reset-layout:hover { background: var(--surface-alt); border-color: var(--text-faint); }

/* `FloatingWindow`'s shared host (see the template comment above it): a
   dedicated `position: relative` container for `.base-view` and every
   floating window, excluding `.colony-header`'s controls. `isolation:
   isolate` gives this subtree its own stacking context so none of the
   windows' z-indices (from the shared registry) can escape past unrelated
   overlays elsewhere in the app shell. */
.colony-body {
  flex: 1;
  min-height: 320px;
  position: relative;
  isolation: isolate;
  border: 1px solid var(--border-subtle);
  border-radius: 4px;
  overflow: hidden;
  background: var(--surface-3);
}

/* The backdrop every floating window sits above — colony details itself,
   not a seventh panel. Deliberately sparse: with all six windows open by
   default it's mostly covered, and it only needs to read coherently in the
   gaps or once the player moves/closes windows around. */
.base-view {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  text-align: center;
  color: var(--border);
  user-select: none;
}
.base-view-name { font-size: 2.4rem; font-weight: bold; letter-spacing: 0.04em; color: var(--surface-btn-hover); }
.base-view-meta { font-size: 0.9rem; color: var(--surface-btn-hover); }
</style>
