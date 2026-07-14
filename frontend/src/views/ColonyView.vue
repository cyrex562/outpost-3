<script setup lang="ts">
/**
 * Colony dashboard — a draggable/resizable multi-panel layout (issue #169)
 * showing population, commodities, buildings, construction queue, and
 * alerts/event log for the selected colony. Panels are laid out with
 * Splitpanes (dockable, resizable splits) and the split sizes persist to
 * localStorage, mirroring the pattern SystemMapView.vue already uses for
 * its own layout state.
 *
 * CommandPanel stays outside the resizable panel area — it's a colony-level
 * action bar (advance turn, found colony, ...), not a data-display panel.
 */

import { computed, onMounted, ref, watch } from 'vue'
import { Splitpanes, Pane } from 'splitpanes'
import type { SplitpanesResizedPayload } from 'splitpanes'
import 'splitpanes/dist/splitpanes.css'
import { useWorldStore } from '@/stores/worldStore'
import { useGameStore } from '@/stores/game'
import CommandPanel from '@/components/CommandPanel.vue'
import PopulationPanel from '@/components/PopulationPanel.vue'
import CommoditiesPanel from '@/components/CommoditiesPanel.vue'
import BuildingsPanel from '@/components/BuildingsPanel.vue'
import ConstructionQueuePanel from '@/components/ConstructionQueuePanel.vue'
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

// ─── Colony selection ─────────────────────────────────────────────────────────

const colonies = computed(() => worldStore.colonies)

const selectedColony = computed((): ColonyState | null => {
  const id = gameStore.selectedColonyId
  if (!id) return colonies.value[0] ?? null
  return worldStore.world.colonies[id] ?? null
})

onMounted(() => {
  // Auto-select first colony if none is selected. The `watch` on
  // `selectedColonyId` in the game store will fetch the colony_screen in
  // Tauri mode; nothing else to do here.
  if (!gameStore.selectedColonyId && colonies.value.length > 0) {
    gameStore.selectedColonyId = colonies.value[0].id
    return
  }
  // Selection was already set (e.g. after founding, then navigating back).
  // Ensure the screen is populated for the current selection.
  const id = gameStore.selectedColonyId
  if (id && (!gameStore.colonyScreen || gameStore.colonyScreen.colony_id !== id)) {
    void gameStore.refreshColonyScreen(id)
  }
})

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

async function queueBuilding(b: BuildingOption): Promise<void> {
  const col = selectedColony.value
  if (!col || queueBusy.value || disabledReason(b) !== null) return
  queueBusy.value = true
  try {
    await gameStore.sendCommand({
      kind: 'queue_construction',
      colony_id: col.id,
      building_type: b.id,
      slot_cost: b.slot_cost,
      labor_per_turn: b.labor_per_turn,
      construction_cost: b.construction_cost,
      construction_turns: b.construction_turns,
    })
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

// ─── Panel layout persistence ───────────────────────────────────────────────────

interface PersistedLayout {
  /** [main-column %, alerts %] */
  outer: number[]
  /** [population %, commodities %, buildings %, construction-queue %] */
  left: number[]
}

const STORAGE_KEY = 'outpost3.colony-view.layout'
const DEFAULT_OUTER = [70, 30]
const DEFAULT_LEFT = [22, 22, 28, 28]

function loadPersistedLayout(): PersistedLayout {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY)
    if (!raw) return { outer: DEFAULT_OUTER, left: DEFAULT_LEFT }
    const p = JSON.parse(raw) as Partial<PersistedLayout>
    const outer = Array.isArray(p.outer) && p.outer.length === 2 ? p.outer : DEFAULT_OUTER
    const left = Array.isArray(p.left) && p.left.length === 4 ? p.left : DEFAULT_LEFT
    return { outer, left }
  } catch {
    // corrupt entry — fall back to defaults
    return { outer: DEFAULT_OUTER, left: DEFAULT_LEFT }
  }
}

const persistedLayout = loadPersistedLayout()
const outerSizes = ref<number[]>(persistedLayout.outer)
const leftSizes = ref<number[]>(persistedLayout.left)

function savePersistedLayout(): void {
  try {
    const payload: PersistedLayout = { outer: outerSizes.value, left: leftSizes.value }
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
</script>

<template>
  <div class="colony-view">
    <div v-if="colonies.length === 0" class="empty-state" data-testid="no-colonies">
      No colonies founded yet. Use the command panel to found one.
    </div>

    <template v-else>
      <div class="colony-header">
        <div class="colony-tabs" data-testid="colony-tabs">
          <button
            v-for="col in colonies"
            :key="col.id"
            class="tab"
            :class="{ active: selectedColony?.id === col.id }"
            @click="gameStore.selectedColonyId = col.id"
          >
            {{ col.name }}
          </button>
        </div>
        <CommandPanel />
      </div>

      <div v-if="selectedColony" class="panel-layout" :data-testid="`colony-detail-${selectedColony.id}`">
        <Splitpanes class="default-theme colony-splitpanes" @resized="onOuterResized">
          <Pane :size="outerSizes[0]" min-size="40">
            <Splitpanes horizontal @resized="onLeftResized">
              <Pane :size="leftSizes[0]" min-size="10">
                <PopulationPanel
                  :population="selectedColony.population"
                  :stability="selectedColony.stability"
                  :available-labour="selectedColony.available_labour"
                  :population-trend="populationTrend"
                />
              </Pane>
              <Pane :size="leftSizes[1]" min-size="10">
                <CommoditiesPanel :stockpile="screen ? screen.stockpile : null" />
              </Pane>
              <Pane :size="leftSizes[2]" min-size="10">
                <BuildingsPanel
                  :buildings="screen ? screen.buildings : null"
                  :slots-used="screen?.slots_used ?? 0"
                  :slot-capacity="screen?.slot_capacity ?? 0"
                  :labour-available="screen?.labour_available ?? 0"
                  :labour-total="screen?.labour_total ?? 0"
                  @assign-labour="assignLabour"
                />
              </Pane>
              <Pane :size="leftSizes[3]" min-size="10">
                <ConstructionQueuePanel
                  :queue="screen ? screen.construction_queue : null"
                  :catalog="buildingCatalog"
                  :disabled-reason="disabledReason"
                  :slots-available="slotsAvailable"
                  :queue-busy="queueBusy"
                  :canceling-ids="cancelingIds"
                  @queue="queueBuilding"
                  @cancel="cancelConstruction"
                />
              </Pane>
            </Splitpanes>
          </Pane>
          <Pane :size="outerSizes[1]" min-size="15">
            <AlertsPanel
              :notifications="worldStore.notifications"
              :event-log="worldStore.eventLog"
              @clear-log="worldStore.clearEventLog()"
            />
          </Pane>
        </Splitpanes>
      </div>

      <div class="research-total" data-testid="research-total">
        System research: {{ worldStore.researchTotal.toFixed(1) }} RP
      </div>
    </template>
  </div>
</template>

<style scoped>
.colony-view { width: 100%; }

.empty-state { color: #666; font-style: italic; margin: 1rem 0; }

.colony-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 1rem;
  margin-bottom: 0.75rem;
  flex-wrap: wrap;
}

.colony-tabs { display: flex; flex-wrap: wrap; gap: 0.4rem; }
.tab {
  background: #151520;
  border: 1px solid #334;
  border-radius: 3px;
  color: #889;
  padding: 0.25rem 0.6rem;
  font-family: monospace;
  font-size: 0.8rem;
  cursor: pointer;
}
.tab.active { border-color: #558; color: #aac; background: #1a1a2a; }
.tab:hover:not(.active) { border-color: #446; color: #aab; }

.panel-layout {
  height: 70vh;
  min-height: 420px;
  border: 1px solid #223;
  border-radius: 4px;
  overflow: hidden;
}

.research-total { margin-top: 0.75rem; font-size: 0.78rem; color: #6a8; }
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
