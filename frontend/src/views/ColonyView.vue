<script setup lang="ts">
/**
 * Colony dashboard — shows population, stability, commodity stockpile,
 * active buildings with labour assignment, construction queue,
 * and an event log panel.  Interactive controls are wired through
 * both the REST path (gameStore.sendCommand) and the WebSocket send.
 */

import { computed, onMounted, ref } from 'vue'
import { useWorldStore } from '@/stores/worldStore'
import { useGameStore } from '@/stores/game'
import { useGameSocket } from '@/composables/useGameSocket'
import CommandPanel from '@/components/CommandPanel.vue'
import type { ColonyState } from '@/worldModel/model'

const worldStore = useWorldStore()
const gameStore = useGameStore()
const { send } = useGameSocket()

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

// ─── Stability helpers ────────────────────────────────────────────────────────

function stabilityClass(stability: number): string {
  if (stability > 0.6) return 'stability-high'
  if (stability >= 0.3) return 'stability-mid'
  return 'stability-low'
}

function stabilityLabel(stability: number): string {
  const pct = (stability * 100).toFixed(0)
  if (stability > 0.6) return `${pct}% — Stable`
  if (stability >= 0.3) return `${pct}% — Uncertain`
  return `${pct}% — Critical`
}

// ─── Sparkline helpers ────────────────────────────────────────────────────────

/**
 * Produce a tiny SVG polyline path from an array of population samples.
 * Uses the last 10 notifications of type needs_resolved to approximate trend.
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
  // If no data, return flat line.
  if (samples.length === 0) return [0, 0, 0]
  return samples
})

function sparklinePath(values: number[]): string {
  if (values.length < 2) return ''
  const min = Math.min(...values)
  const max = Math.max(...values)
  const range = max - min || 1
  const w = 80
  const h = 20
  const pts = values.map((v, i) => {
    const x = (i / (values.length - 1)) * w
    const y = h - ((v - min) / range) * h
    return `${x.toFixed(1)},${y.toFixed(1)}`
  })
  return `M ${pts.join(' L ')}`
}

// ─── Commodity net colour ─────────────────────────────────────────────────────

function netClass(net: number): string {
  if (net > 0) return 'net-positive'
  if (net < 0) return 'net-negative'
  return 'net-zero'
}

function formatNet(net: number): string {
  if (net > 0) return `+${net.toFixed(2)}`
  return net.toFixed(2)
}

// ─── Construction queue ───────────────────────────────────────────────────────

/** Building type to queue — user types it in or it is set by a quick-queue button. */
const queueBuildingType = ref('')
const queueSlotCost = ref(1)
const queueLabourPerTurn = ref(1)
const queueTurns = ref(5)
const queueBusy = ref(false)

async function queueConstruction(): Promise<void> {
  const col = selectedColony.value
  if (!col || !queueBuildingType.value.trim()) return
  queueBusy.value = true
  try {
    const colonyId = col.id
    const buildingType = queueBuildingType.value.trim()
    // Send over WebSocket for immediate feedback.
    const seq = Date.now()
    send({
      type: 'command',
      seq,
      command: {
        kind: 'queue_construction',
        colony_id: colonyId,
        building_type: buildingType,
        slot_cost: queueSlotCost.value,
        labor_per_turn: queueLabourPerTurn.value,
        construction_cost: [],
        construction_turns: queueTurns.value,
      },
    })
    // Also flush via REST so the response updates state synchronously.
    await gameStore.sendCommand({
      kind: 'queue_construction',
      colony_id: colonyId,
      building_type: buildingType,
      slot_cost: queueSlotCost.value,
      labor_per_turn: queueLabourPerTurn.value,
      construction_cost: [],
      construction_turns: queueTurns.value,
    })
    queueBuildingType.value = ''
  } finally {
    queueBusy.value = false
  }
}

// ─── Labour assignment ────────────────────────────────────────────────────────

/** Temporary labour values being edited (keyed by building_type). */
const labourDraft = ref<Record<string, number>>({})

function labourForBuilding(buildingType: string, current: number): number {
  return labourDraft.value[buildingType] ?? current
}

async function assignLabour(buildingType: string): Promise<void> {
  const col = selectedColony.value
  if (!col) return
  const labour = labourDraft.value[buildingType]
  if (labour === undefined) return
  const colonyId = col.id
  const seq = Date.now()
  // Send over WebSocket.
  send({
    type: 'command',
    seq,
    command: { kind: 'assign_labour', colony_id: colonyId, slot: buildingType, labour },
  })
  // Also flush via REST so state reflects the change.
  await gameStore.sendCommand({
    kind: 'assign_labour',
    colony_id: colonyId,
    slot: buildingType,
    labour,
  })
  delete labourDraft.value[buildingType]
}

// ─── Event log ────────────────────────────────────────────────────────────────

const eventLog = computed(() => worldStore.eventLog)

function formatEventKind(kind: string): string {
  return kind.replace(/_/g, ' ')
}

function eventLogClass(kind: string): string {
  if (kind.includes('shortfall') || kind.includes('cancel')) return 'log-warn'
  if (kind.includes('founded') || kind.includes('constructed') || kind.includes('advanced'))
    return 'log-info'
  return 'log-default'
}
</script>

<template>
  <div class="colony-view">
    <div class="layout">
      <!-- ── Left panel: dashboard ───────────────────────────────────────── -->
      <div class="dashboard">
        <h2 class="section-title">Colony Dashboard</h2>

        <div v-if="colonies.length === 0" class="empty-state" data-testid="no-colonies">
          No colonies founded yet. Use the command panel to found one.
        </div>

        <template v-else>
          <!-- Colony selector tabs -->
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

          <!-- Selected colony details -->
          <div v-if="selectedColony" class="colony-detail" :data-testid="`colony-detail-${selectedColony.id}`">
            <!-- Population + sparkline -->
            <div class="stat-row" data-testid="population-section">
              <div class="stat-block">
                <span class="stat-label">Population</span>
                <span class="stat-value" data-testid="population-count">
                  {{ selectedColony.population.toFixed(0) }}
                </span>
              </div>
              <div class="sparkline-wrap" title="Population trend (last sessions)">
                <svg
                  width="80"
                  height="20"
                  class="sparkline"
                  data-testid="population-sparkline"
                  aria-hidden="true"
                >
                  <path
                    v-if="populationTrend.length >= 2"
                    :d="sparklinePath(populationTrend)"
                    fill="none"
                    stroke="#4c8"
                    stroke-width="1.5"
                  />
                  <line
                    v-else
                    x1="0" y1="10" x2="80" y2="10"
                    stroke="#444"
                    stroke-width="1"
                    stroke-dasharray="4 3"
                  />
                </svg>
              </div>
            </div>

            <!-- Stability bar -->
            <div class="stat-block stability-section" data-testid="stability-section">
              <span class="stat-label">Stability</span>
              <div
                class="stability-bar-track"
                role="progressbar"
                :aria-valuenow="Math.round(selectedColony.stability * 100)"
                aria-valuemin="0"
                aria-valuemax="100"
                data-testid="stability-bar"
              >
                <div
                  class="stability-bar-fill"
                  :class="stabilityClass(selectedColony.stability)"
                  :style="{ width: `${(selectedColony.stability * 100).toFixed(1)}%` }"
                />
              </div>
              <span
                class="stability-label"
                :class="stabilityClass(selectedColony.stability)"
                data-testid="stability-label"
              >
                {{ stabilityLabel(selectedColony.stability) }}
              </span>
            </div>

            <!-- Labour -->
            <div class="stat-row">
              <div class="stat-block">
                <span class="stat-label">Labour</span>
                <span class="stat-value">{{ selectedColony.available_labour }}</span>
              </div>
            </div>

            <!-- Commodity stock table -->
            <div class="section" data-testid="commodity-section">
              <h4 class="sub-title">Commodities</h4>
              <div v-if="!gameStore.colonyScreen || gameStore.colonyScreen.colony_id !== selectedColony.id"
                   class="hint">
                Advance a turn to load commodity data.
              </div>
              <table v-else class="stock-table" data-testid="commodity-table">
                <thead>
                  <tr>
                    <th>Commodity</th>
                    <th class="num">Amount</th>
                    <th class="num">Capacity</th>
                    <th class="num">Net/Sol</th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="row in gameStore.colonyScreen.stockpile"
                    :key="row.commodity_id"
                    :data-testid="`stock-row-${row.commodity_id}`"
                  >
                    <td>{{ row.commodity_id }}</td>
                    <td class="num">{{ row.amount.toFixed(1) }}</td>
                    <td class="num">{{ row.capacity != null ? row.capacity.toFixed(1) : '∞' }}</td>
                    <td class="num" :class="netClass(row.net_per_turn)">
                      {{ formatNet(row.net_per_turn) }}
                    </td>
                  </tr>
                  <tr v-if="gameStore.colonyScreen.stockpile.length === 0">
                    <td colspan="4" class="empty-row">No commodities tracked yet.</td>
                  </tr>
                </tbody>
              </table>
            </div>

            <!-- Active buildings + labour assignment -->
            <div class="section" data-testid="directives-section">
              <h4 class="sub-title">Active Buildings &amp; Labour</h4>
              <ul
                v-if="gameStore.colonyScreen && gameStore.colonyScreen.colony_id === selectedColony.id"
                class="directive-list"
                data-testid="directive-list"
              >
                <li
                  v-for="b in gameStore.colonyScreen.buildings"
                  :key="b.building_type"
                  class="directive-item"
                  :data-testid="`building-row-${b.building_type}`"
                >
                  <span class="building-name">{{ b.building_type }}</span>
                  <span class="directive-meta">{{ b.slot_cost }} slot{{ b.slot_cost !== 1 ? 's' : '' }}</span>
                  <div class="labour-controls">
                    <input
                      class="labour-input"
                      type="number"
                      min="0"
                      :value="labourForBuilding(b.building_type, b.labour_assigned)"
                      :data-testid="`labour-input-${b.building_type}`"
                      @input="labourDraft[b.building_type] = +($event.target as HTMLInputElement).value"
                    />
                    <button
                      class="btn-assign"
                      :disabled="labourDraft[b.building_type] === undefined"
                      :data-testid="`assign-labour-${b.building_type}`"
                      @click="assignLabour(b.building_type)"
                    >
                      Assign
                    </button>
                  </div>
                  <span class="directive-meta">{{ b.full_capacity ? 'full' : 'partial' }}</span>
                </li>
                <li v-if="gameStore.colonyScreen.buildings.length === 0" class="empty-row">
                  No active buildings/directives.
                </li>
              </ul>
              <div v-else class="hint">No building data loaded.</div>

              <!-- Build slots summary -->
              <div
                v-if="gameStore.colonyScreen && gameStore.colonyScreen.colony_id === selectedColony.id"
                class="slots-summary"
                data-testid="slots-summary"
              >
                Build slots: {{ gameStore.colonyScreen.slots_used }} / {{ gameStore.colonyScreen.slot_capacity }}
                &nbsp;|&nbsp;
                Labour: {{ gameStore.colonyScreen.labour_available }} / {{ gameStore.colonyScreen.labour_total }} free
              </div>
            </div>

            <!-- Construction queue -->
            <div class="section" data-testid="construction-queue-section">
              <h4 class="sub-title">Construction Queue</h4>
              <div
                v-if="gameStore.colonyScreen && gameStore.colonyScreen.colony_id === selectedColony.id && gameStore.colonyScreen.construction_queue.length > 0"
              >
                <ul class="queue-list" data-testid="construction-queue-list">
                  <li
                    v-for="proj in gameStore.colonyScreen.construction_queue"
                    :key="proj.project_id"
                    class="queue-item"
                    :data-testid="`queue-item-${proj.project_id}`"
                  >
                    <span class="building-name">{{ proj.building_type }}</span>
                    <span class="queue-progress">
                      {{ proj.turns_completed }}/{{ proj.turns_total }} turns
                    </span>
                    <span class="directive-meta">{{ proj.slot_cost }} slot{{ proj.slot_cost !== 1 ? 's' : '' }}</span>
                  </li>
                </ul>
              </div>
              <div v-else class="hint">No projects in queue.</div>

              <!-- Queue new construction form -->
              <form
                class="queue-form"
                data-testid="queue-construction-form"
                @submit.prevent="queueConstruction"
              >
                <input
                  v-model="queueBuildingType"
                  class="input-sm"
                  placeholder="Building type (e.g. mine)"
                  data-testid="queue-building-type-input"
                />
                <div class="queue-params">
                  <label class="param-label">Slots
                    <input v-model.number="queueSlotCost" class="input-xs" type="number" min="1" data-testid="queue-slot-cost" />
                  </label>
                  <label class="param-label">Labour/turn
                    <input v-model.number="queueLabourPerTurn" class="input-xs" type="number" min="0" data-testid="queue-labour-per-turn" />
                  </label>
                  <label class="param-label">Turns
                    <input v-model.number="queueTurns" class="input-xs" type="number" min="1" data-testid="queue-turns" />
                  </label>
                </div>
                <button
                  type="submit"
                  class="btn-queue"
                  :disabled="queueBusy || !queueBuildingType.trim()"
                  data-testid="btn-queue-construction"
                >
                  Queue Construction
                </button>
              </form>
            </div>
          </div>
        </template>

        <!-- Notifications -->
        <div
          v-if="worldStore.notifications.length > 0"
          class="notifications"
          data-testid="notifications"
        >
          <h4 class="sub-title">Alerts</h4>
          <ul>
            <li
              v-for="n in worldStore.notifications"
              :key="n.id"
              :class="`notification tier-${n.tier}`"
            >
              {{ n.message }}
            </li>
          </ul>
        </div>

        <div class="research-total" data-testid="research-total">
          System research: {{ worldStore.researchTotal.toFixed(1) }} RP
        </div>
      </div>

      <!-- ── Right panel: command panel + event log ──────────────────────── -->
      <aside class="command-sidebar">
        <CommandPanel />

        <!-- Event log panel -->
        <div class="event-log" data-testid="event-log">
          <div class="event-log-header">
            <h4 class="sub-title" style="margin:0">Event Log</h4>
            <button
              class="btn-clear-log"
              data-testid="btn-clear-event-log"
              @click="worldStore.clearEventLog()"
            >
              Clear
            </button>
          </div>
          <div v-if="eventLog.length === 0" class="hint" style="padding:0.4rem 0">
            No events yet.
          </div>
          <ul v-else class="log-list" data-testid="event-log-list">
            <li
              v-for="(ev, idx) in [...eventLog].reverse()"
              :key="idx"
              class="log-item"
              :class="eventLogClass(ev.kind)"
              :data-testid="`log-item-${ev.kind}`"
            >
              {{ formatEventKind(ev.kind) }}
            </li>
          </ul>
        </div>
      </aside>
    </div>
  </div>
</template>

<style scoped>
.colony-view { width: 100%; }

.layout {
  display: grid;
  grid-template-columns: 1fr 240px;
  gap: 1.25rem;
  align-items: start;
}

@media (max-width: 700px) {
  .layout { grid-template-columns: 1fr; }
}

.dashboard { min-width: 0; }

.section-title { color: #8cf; font-size: 1rem; margin-bottom: 0.75rem; }
.sub-title { color: #668; font-size: 0.78rem; letter-spacing: 0.06em; text-transform: uppercase; margin: 0.75rem 0 0.4rem; }

.empty-state { color: #666; font-style: italic; margin: 1rem 0; }

/* Colony tabs */
.colony-tabs { display: flex; flex-wrap: wrap; gap: 0.4rem; margin-bottom: 0.75rem; }
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

/* Stats */
.stat-row { display: flex; gap: 1.5rem; margin-bottom: 0.5rem; align-items: flex-end; }
.stat-block { display: flex; flex-direction: column; }
.stat-label { font-size: 0.7rem; color: #668; margin-bottom: 0.1rem; }
.stat-value { font-size: 1.1rem; color: #dde; }

/* Sparkline */
.sparkline-wrap { align-self: flex-end; }
.sparkline { display: block; }

/* Stability bar */
.stability-section { margin-bottom: 0.75rem; }
.stability-bar-track {
  height: 8px;
  background: #1a1a2a;
  border: 1px solid #334;
  border-radius: 4px;
  overflow: hidden;
  margin: 0.25rem 0;
  width: 100%;
  max-width: 300px;
}
.stability-bar-fill {
  height: 100%;
  border-radius: 4px;
  transition: width 0.3s ease;
}
.stability-high { background: #3a8; color: #4c9; }
.stability-mid  { background: #a82; color: #ca6; }
.stability-low  { background: #a33; color: #c55; }
.stability-label { font-size: 0.75rem; }

/* Commodity table */
.section { margin-bottom: 1rem; }
.stock-table { width: 100%; border-collapse: collapse; font-size: 0.8rem; }
.stock-table th { color: #668; font-weight: normal; text-align: left; padding: 0.2rem 0.4rem; border-bottom: 1px solid #222; }
.stock-table th.num, .stock-table td.num { text-align: right; }
.stock-table td { padding: 0.25rem 0.4rem; border-bottom: 1px solid #1a1a24; color: #aab; }
.stock-table tbody tr:hover td { background: #13131e; }
.net-positive { color: #4c9; }
.net-negative { color: #c55; }
.net-zero     { color: #667; }
.empty-row { color: #445; font-style: italic; }

/* Buildings + labour */
.directive-list { list-style: none; display: flex; flex-direction: column; gap: 0.35rem; }
.directive-item {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  background: #13131e;
  border: 1px solid #223;
  border-radius: 3px;
  padding: 0.3rem 0.5rem;
  font-size: 0.8rem;
  color: #aab;
  flex-wrap: wrap;
}
.building-name { flex: 1 0 100px; }
.directive-meta { color: #668; font-size: 0.72rem; }
.labour-controls { display: flex; align-items: center; gap: 0.3rem; }
.labour-input {
  width: 54px;
  background: #0d0d15;
  border: 1px solid #334;
  border-radius: 3px;
  color: #cdd;
  padding: 0.2rem 0.35rem;
  font-family: monospace;
  font-size: 0.8rem;
}
.btn-assign {
  background: #1a2030;
  border: 1px solid #336;
  border-radius: 3px;
  color: #89b;
  padding: 0.15rem 0.4rem;
  font-size: 0.75rem;
  cursor: pointer;
}
.btn-assign:disabled { opacity: 0.4; cursor: not-allowed; }
.btn-assign:hover:not(:disabled) { background: #1e2840; }

/* Slots summary */
.slots-summary {
  font-size: 0.72rem;
  color: #558;
  margin-top: 0.35rem;
}

/* Construction queue list */
.queue-list { list-style: none; display: flex; flex-direction: column; gap: 0.3rem; margin-bottom: 0.5rem; }
.queue-item {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  background: #111120;
  border: 1px solid #223;
  border-radius: 3px;
  padding: 0.25rem 0.5rem;
  font-size: 0.8rem;
  color: #aab;
}
.queue-progress { font-size: 0.72rem; color: #668; }

/* Queue form */
.queue-form { display: flex; flex-direction: column; gap: 0.4rem; margin-top: 0.5rem; }
.input-sm {
  background: #0d0d15;
  border: 1px solid #334;
  border-radius: 3px;
  color: #cdd;
  padding: 0.3rem 0.5rem;
  font-family: monospace;
  font-size: 0.82rem;
}
.input-sm:focus { outline: none; border-color: #558; }
.queue-params { display: flex; gap: 0.75rem; flex-wrap: wrap; }
.param-label {
  font-size: 0.72rem;
  color: #668;
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
}
.input-xs {
  width: 52px;
  background: #0d0d15;
  border: 1px solid #334;
  border-radius: 3px;
  color: #cdd;
  padding: 0.2rem 0.3rem;
  font-family: monospace;
  font-size: 0.8rem;
}
.btn-queue {
  background: #1a2030;
  border: 1px solid #468;
  border-radius: 3px;
  color: #8cf;
  padding: 0.35rem 0.7rem;
  font-family: monospace;
  font-size: 0.82rem;
  cursor: pointer;
  align-self: flex-start;
}
.btn-queue:disabled { opacity: 0.45; cursor: not-allowed; }
.btn-queue:hover:not(:disabled) { background: #182030; }

/* Notifications */
.notifications { margin-top: 1rem; }
.notifications ul { list-style: none; }
.notification { padding: 0.2rem 0.5rem; font-size: 0.78rem; border-left: 3px solid #555; margin-bottom: 0.2rem; }
.notification.tier-notable  { border-color: #c84; color: #ca8; }
.notification.tier-urgent   { border-color: #c44; color: #c66; }
.notification.tier-blocking { border-color: #f44; color: #f66; }
.notification.tier-ambient  { border-color: #446; color: #778; }

.research-total { margin-top: 0.75rem; font-size: 0.78rem; color: #6a8; }
.hint { font-size: 0.75rem; color: #446; font-style: italic; }

/* Event log */
.event-log {
  margin-top: 1rem;
  background: #111118;
  border: 1px solid #334;
  border-radius: 4px;
  padding: 0.75rem;
}
.event-log-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.4rem;
}
.btn-clear-log {
  background: transparent;
  border: 1px solid #334;
  border-radius: 3px;
  color: #556;
  padding: 0.1rem 0.4rem;
  font-size: 0.7rem;
  cursor: pointer;
}
.btn-clear-log:hover { color: #889; border-color: #446; }
.log-list {
  list-style: none;
  max-height: 200px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
}
.log-item {
  font-size: 0.74rem;
  padding: 0.15rem 0.35rem;
  border-left: 2px solid #334;
  color: #778;
}
.log-info    { border-color: #468; color: #8ab; }
.log-warn    { border-color: #853; color: #b86; }
.log-default { border-color: #334; color: #667; }
</style>
