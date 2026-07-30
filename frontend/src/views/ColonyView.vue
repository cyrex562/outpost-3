<script setup lang="ts">
/**
 * Colony dashboard — a draggable/resizable multi-panel layout (issue #169)
 * showing population, commodities, buildings, construction queue, and
 * alerts/event log for the selected colony. Panels are laid out with
 * Splitpanes (dockable, resizable splits) and the split sizes persist to
 * localStorage, mirroring the pattern SystemMapView.vue already uses for
 * its own layout state.
 *
 * Turn control (Advance Turn) and system-wide stats now live in the app
 * shell's footer/stats bars (UI-rework PR3), not in this view.
 */

import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { Splitpanes, Pane } from 'splitpanes'
import type { SplitpanesResizedPayload } from 'splitpanes'
import 'splitpanes/dist/splitpanes.css'
import { useWorldStore } from '@/stores/worldStore'
import { useGameStore } from '@/stores/game'
import VitalStatsPanel from '@/components/VitalStatsPanel.vue'
import CommoditiesPanel from '@/components/CommoditiesPanel.vue'
import UtilitiesPanel from '@/components/UtilitiesPanel.vue'
import BuildingsPanel from '@/components/BuildingsPanel.vue'
import ConstructionQueuePanel from '@/components/ConstructionQueuePanel.vue'
import BuildDialog from '@/components/BuildDialog.vue'
import AlertsPanel from '@/components/AlertsPanel.vue'
import FloatingWindow from '@/components/FloatingWindow.vue'
import BuildingDetailsHud from '@/components/BuildingDetailsHud.vue'
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

// ─── Building details (issue #182, floated as of #339) ─────────────────────────
//
// Selecting a building used to navigate to the routed `/facility/:type` page,
// replacing the whole colony view. That hid the stores/labour/building-list
// context a player usually wants while reading a building's lines and
// shortfalls, so it now opens as a floating window over the colony view
// instead (`BuildingDetailsHud` in its `asPage` presentation, which is the
// plain-content mode the routed page already used — only the container
// changes). The routed page itself stays for deep-linking (e.g. outposts).

/** Building type currently shown in the floating details window, or `null` when closed. */
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

// ─── Panel layout persistence ───────────────────────────────────────────────────

interface PersistedLayout {
  /** [left-column %, center-column %, alerts-column %] */
  outer: number[]
  /** left column vertical split: [vital-stats %, commodities %] */
  left: number[]
  /** center column vertical split: [buildings %, construction-queue %] */
  center: number[]
  /** Whether `center` reflects a deliberate drag rather than an emptiness-driven default. */
  centerTouched: boolean
}

// Bumped to `.v2` because the split shape changed from 2+4 panes to the
// 3-column (3 + 2 + 2) arrangement (UI-rework PR4); a stale entry would have
// the wrong number of sizes. Bumped to v3 when the left column gained a third
// pane for the utilities panel (issue #304) — a persisted v2 entry has only two
// left sizes and would leave the new pane unsized. Bumped to v4 when the
// center split gained emptiness-driven defaults (issue #339) — `center` is no
// longer meaningful on its own without the `centerTouched` flag.
const STORAGE_KEY = 'outpost3.colony-view.layout.v4'
const DEFAULT_OUTER = [30, 45, 25]
const DEFAULT_LEFT = [34, 26, 40]
// Building list gets the space by default (issue #339) — construction is
// empty most of the early game, so it no longer starts out larger than the
// building list the way `[45, 55]` used to.
const DEFAULT_CENTER_FILLED = [62, 38]
// Compact "nothing under construction" default: the queue panel collapses to
// just enough room for its heading row, handing the rest to the buildings.
const DEFAULT_CENTER_EMPTY = [88, 12]

function loadPersistedLayout(): PersistedLayout {
  const fallback = {
    outer: DEFAULT_OUTER,
    left: DEFAULT_LEFT,
    center: DEFAULT_CENTER_FILLED,
    centerTouched: false,
  }
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY)
    if (!raw) return fallback
    const p = JSON.parse(raw) as Partial<PersistedLayout>
    const outer = Array.isArray(p.outer) && p.outer.length === 3 ? p.outer : DEFAULT_OUTER
    const left = Array.isArray(p.left) && p.left.length === 3 ? p.left : DEFAULT_LEFT
    const center = Array.isArray(p.center) && p.center.length === 2 ? p.center : DEFAULT_CENTER_FILLED
    const centerTouched = p.centerTouched === true
    return { outer, left, center, centerTouched }
  } catch {
    // corrupt entry — fall back to defaults
    return fallback
  }
}

const persistedLayout = loadPersistedLayout()
const outerSizes = ref<number[]>(persistedLayout.outer)
const leftSizes = ref<number[]>(persistedLayout.left)
const centerSizes = ref<number[]>(persistedLayout.center)
/** Once the player drags the queue/building-list divider, their choice sticks
 * regardless of the queue's emptiness — only an untouched split auto-adjusts. */
const centerTouched = ref<boolean>(persistedLayout.centerTouched)

/** Nothing currently under construction — drives the collapsed default split. */
const constructionQueueEmpty = computed(
  () => !screen.value || screen.value.construction_queue.length === 0,
)

/** The center split actually handed to Splitpanes: the player's own drag once
 * they've made one, otherwise an emptiness-driven default so the building
 * list gets the space while the queue is empty and expands back out once
 * something is queued. */
const effectiveCenterSizes = computed<number[]>(() =>
  centerTouched.value
    ? centerSizes.value
    : constructionQueueEmpty.value
      ? DEFAULT_CENTER_EMPTY
      : DEFAULT_CENTER_FILLED,
)

function savePersistedLayout(): void {
  try {
    const payload: PersistedLayout = {
      outer: outerSizes.value,
      left: leftSizes.value,
      center: centerSizes.value,
      centerTouched: centerTouched.value,
    }
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(payload))
  } catch {
    // storage full or blocked — non-fatal
  }
}

function onOuterResized(payload: SplitpanesResizedPayload): void {
  outerSizes.value = payload.panes.map((p) => p.size)
  savePersistedLayout()
}

function onLeftResized(payload: SplitpanesResizedPayload): void {
  leftSizes.value = payload.panes.map((p) => p.size)
  savePersistedLayout()
}

function onCenterResized(payload: SplitpanesResizedPayload): void {
  centerSizes.value = payload.panes.map((p) => p.size)
  centerTouched.value = true
  savePersistedLayout()
}
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
      </div>

      <div v-if="selectedColony" class="panel-layout" :data-testid="`colony-detail-${selectedColony.id}`">
        <Splitpanes class="default-theme colony-splitpanes" @resized="onOuterResized">
          <!-- Left column: vital statistics over the commodity stockpile. -->
          <Pane :size="outerSizes[0]" min-size="15">
            <Splitpanes horizontal @resized="onLeftResized">
              <Pane :size="leftSizes[0]" min-size="10">
                <VitalStatsPanel
                  :population="selectedColony.population"
                  :stability="selectedColony.stability"
                  :available-labour="selectedColony.available_labour"
                  :population-trend="populationTrend"
                  :slots-used="screen?.slots_used ?? 0"
                  :slot-capacity="screen?.slot_capacity ?? 0"
                  :labour-employed="screen?.labour_employed ?? 0"
                  :labour-unemployed="screen?.labour_unemployed ?? 0"
                />
              </Pane>
              <!-- Utilities sit between vitals and commodities (issue #304):
                   power/housing/research are colony-local and unshippable, so
                   they must not read as stock a hauler could collect. -->
              <Pane :size="leftSizes[1]" min-size="10">
                <UtilitiesPanel :resources="screen ? screen.resources : null" />
              </Pane>
              <Pane :size="leftSizes[2]" min-size="10">
                <CommoditiesPanel
                  :stockpile="screen ? screen.stockpile : null"
                  @set-reserve="setCommodityReserve"
                />
              </Pane>
            </Splitpanes>
          </Pane>

          <!-- Center column: the buildings list over the construction queue. -->
          <Pane :size="outerSizes[1]" min-size="20">
            <!-- Resizable split (issue #339): the construction queue defaults
                 to a compact size while empty and expands back out once
                 something is queued, but a deliberate drag on this divider
                 always wins over that emptiness-driven default — see
                 `effectiveCenterSizes`. -->
            <Splitpanes horizontal @resized="onCenterResized">
              <Pane :size="effectiveCenterSizes[0]" min-size="15">
                <BuildingsPanel
                  :buildings="screen ? screen.buildings : null"
                  :slots-used="screen?.slots_used ?? 0"
                  :slot-capacity="screen?.slot_capacity ?? 0"
                  :labour-available="screen?.labour_available ?? 0"
                  :labour-total="screen?.labour_total ?? 0"
                  @view-details="openBuildingDetails"
                  @set-priority="setBuildingPriority"
                  @set-lock="setBuildingLock"
                  @rename="renameBuilding"
                  @set-paused="setBuildingPaused"
                />
              </Pane>
              <Pane :size="effectiveCenterSizes[1]" min-size="6">
                <ConstructionQueuePanel
                  :queue="screen ? screen.construction_queue : null"
                  :canceling-ids="cancelingIds"
                  @open-build="showBuildDialog = true"
                  @cancel="cancelConstruction"
                />
              </Pane>
            </Splitpanes>
          </Pane>

          <!-- Right column: alerts + event log. -->
          <Pane :size="outerSizes[2]" min-size="12">
            <AlertsPanel
              :notifications="worldStore.notifications"
              :event-log="worldStore.eventLog"
              @clear-log="worldStore.clearEventLog()"
            />
          </Pane>
        </Splitpanes>
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

      <!-- Building details float over the whole view (issue #339) rather than
           replacing it, so stores/labour/the rest of the building list stay
           visible while reading a building's lines and shortfalls. Reuses
           `BuildingDetailsHud`'s `asPage` presentation (plain content, no
           backdrop) since `FloatingWindow` already supplies the frame,
           drag/resize, and dismiss button. -->
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

.panel-layout {
  flex: 1;
  min-height: 320px;
  border: 1px solid #223;
  border-radius: 4px;
  overflow: hidden;
}
</style>

<style>
/* Splitpanes theme overrides — unscoped so they reach the library's own
   root elements, which render outside this component's scoped attribute. */
.colony-splitpanes.default-theme .splitpanes__pane {
  background: #0d0d15;
}
.colony-splitpanes.default-theme .splitpanes__splitter {
  background: #14141e;
  border-color: #223;
}
.colony-splitpanes.default-theme .splitpanes__splitter:hover {
  background: #1a1a2a;
}
</style>
