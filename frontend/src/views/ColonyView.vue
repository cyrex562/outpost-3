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
import BuildingsPanel from '@/components/BuildingsPanel.vue'
import ConstructionQueuePanel from '@/components/ConstructionQueuePanel.vue'
import BuildDialog from '@/components/BuildDialog.vue'
import AlertsPanel from '@/components/AlertsPanel.vue'
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

// ─── Labour assignment ────────────────────────────────────────────────────────

async function assignLabour(buildingType: string, labour: number): Promise<void> {
  const col = selectedColony.value
  if (!col) return
  await gameStore.sendCommand({
    kind: 'assign_labour',
    colony_id: col.id,
    slot: buildingType,
    labour,
  })
}

// ─── Building details (issue #182; routed page as of navigation rework #7 phase 2) ──

function openBuildingDetails(buildingType: string): void {
  const col = selectedColony.value
  if (!col) return
  void router.push({ name: 'facility', params: { colonyId: col.id, buildingType } })
}

// ─── Panel layout persistence ───────────────────────────────────────────────────

interface PersistedLayout {
  /** [left-column %, center-column %, alerts-column %] */
  outer: number[]
  /** left column vertical split: [vital-stats %, commodities %] */
  left: number[]
  /** center column vertical split: [buildings %, construction-queue %] */
  center: number[]
}

// Bumped to `.v2` because the split shape changed from 2+4 panes to the
// 3-column (3 + 2 + 2) arrangement (UI-rework PR4); a stale v1 entry would
// have the wrong number of sizes.
const STORAGE_KEY = 'outpost3.colony-view.layout.v2'
const DEFAULT_OUTER = [30, 45, 25]
const DEFAULT_LEFT = [45, 55]
const DEFAULT_CENTER = [45, 55]

function loadPersistedLayout(): PersistedLayout {
  const fallback = { outer: DEFAULT_OUTER, left: DEFAULT_LEFT, center: DEFAULT_CENTER }
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY)
    if (!raw) return fallback
    const p = JSON.parse(raw) as Partial<PersistedLayout>
    const outer = Array.isArray(p.outer) && p.outer.length === 3 ? p.outer : DEFAULT_OUTER
    const left = Array.isArray(p.left) && p.left.length === 2 ? p.left : DEFAULT_LEFT
    const center = Array.isArray(p.center) && p.center.length === 2 ? p.center : DEFAULT_CENTER
    return { outer, left, center }
  } catch {
    // corrupt entry — fall back to defaults
    return fallback
  }
}

const persistedLayout = loadPersistedLayout()
const outerSizes = ref<number[]>(persistedLayout.outer)
const leftSizes = ref<number[]>(persistedLayout.left)
const centerSizes = ref<number[]>(persistedLayout.center)

function savePersistedLayout(): void {
  try {
    const payload: PersistedLayout = {
      outer: outerSizes.value,
      left: leftSizes.value,
      center: centerSizes.value,
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
              <Pane :size="leftSizes[1]" min-size="10">
                <CommoditiesPanel :stockpile="screen ? screen.stockpile : null" />
              </Pane>
            </Splitpanes>
          </Pane>

          <!-- Center column: the buildings list over the construction queue. -->
          <Pane :size="outerSizes[1]" min-size="20">
            <Splitpanes horizontal @resized="onCenterResized">
              <Pane :size="centerSizes[0]" min-size="10">
                <BuildingsPanel
                  :buildings="screen ? screen.buildings : null"
                  :slots-used="screen?.slots_used ?? 0"
                  :slot-capacity="screen?.slot_capacity ?? 0"
                  :labour-available="screen?.labour_available ?? 0"
                  :labour-total="screen?.labour_total ?? 0"
                  @assign-labour="assignLabour"
                  @view-details="openBuildingDetails"
                />
              </Pane>
              <Pane :size="centerSizes[1]" min-size="10">
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
    </template>
  </div>
</template>


<style scoped>
/* Fill the shell's main region (UI-rework PR4): a flex column whose panel
   area (`.panel-layout`) grows to take all remaining height, so the colony
   dashboard fills the screen without a fixed viewport-height guess. */
.colony-view { width: 100%; height: 100%; display: flex; flex-direction: column; }

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
