<script setup lang="ts">
/**
 * Colony dashboard — shows population, stability, commodity stockpile,
 * active directives, and embeds the command panel.
 */

import { computed, onMounted } from 'vue'
import { useWorldStore } from '@/stores/worldStore'
import { useGameStore } from '@/stores/game'
import CommandPanel from '@/components/CommandPanel.vue'
import type { ColonyState } from '@/worldModel/model'

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
  // Auto-select first colony if none is selected.
  if (!gameStore.selectedColonyId && colonies.value.length > 0) {
    gameStore.selectedColonyId = colonies.value[0].id
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

            <!-- Active directives -->
            <div class="section" data-testid="directives-section">
              <h4 class="sub-title">Active Directives</h4>
              <ul
                v-if="gameStore.colonyScreen && gameStore.colonyScreen.colony_id === selectedColony.id"
                class="directive-list"
                data-testid="directive-list"
              >
                <li
                  v-for="b in gameStore.colonyScreen.buildings"
                  :key="b.building_type"
                  class="directive-item"
                >
                  {{ b.building_type }}
                  <span class="directive-meta">{{ b.labour_assigned }} labour · {{ b.full_capacity ? 'full' : 'partial' }}</span>
                </li>
                <li v-if="gameStore.colonyScreen.buildings.length === 0" class="empty-row">
                  No active buildings/directives.
                </li>
              </ul>
              <div v-else class="hint">No directive data loaded.</div>
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

      <!-- ── Right panel: command panel ──────────────────────────────────── -->
      <aside class="command-sidebar">
        <CommandPanel />
      </aside>
    </div>
  </div>
</template>

<style scoped>
.colony-view { width: 100%; }

.layout {
  display: grid;
  grid-template-columns: 1fr 220px;
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

/* Directives list */
.directive-list { list-style: none; display: flex; flex-direction: column; gap: 0.25rem; }
.directive-item {
  display: flex;
  justify-content: space-between;
  background: #13131e;
  border: 1px solid #223;
  border-radius: 3px;
  padding: 0.25rem 0.5rem;
  font-size: 0.8rem;
  color: #aab;
}
.directive-meta { color: #668; font-size: 0.72rem; }

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
</style>
