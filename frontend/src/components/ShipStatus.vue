<template>
  <div class="p-2 text-xs space-y-3 select-none">
    <!-- Phase badge -->
    <div class="flex items-center gap-2">
      <span v-if="gs.shipName" class="text-panel-text font-bold">{{ gs.shipName }}</span>
      <span
        class="px-2 py-0.5 rounded text-[10px] font-bold uppercase tracking-wider"
        :class="phaseBadgeClass"
      >{{ gs.phase || '—' }}</span>
    </div>

    <!-- Resources table -->
    <div v-if="gs.resources">
      <div class="text-panel-muted mb-1 uppercase tracking-wider text-[10px]">Resources</div>
      <table class="w-full">
        <thead>
          <tr class="text-panel-muted text-[10px]">
            <th class="text-left font-normal">Name</th>
            <th class="text-right font-normal">Amount</th>
            <th class="text-right font-normal">Rate/d</th>
            <th class="text-right font-normal w-6">⬤</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="key in gs.resourceKeys" :key="key">
            <td class="text-panel-text">{{ formatKey(key) }}</td>
            <td class="text-right tabular-nums">{{ fmt(gs.resources[key]) }}</td>
            <td class="text-right tabular-nums" :class="rateClass(gs.resourceRates[key])">
              {{ fmtRate(gs.resourceRates[key]) }}
            </td>
            <td class="text-right">
              <span :class="statusDot(gs.resources[key], key)">●</span>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Ship systems health bars -->
    <div v-if="gs.shipSystems">
      <div class="text-panel-muted mb-1 uppercase tracking-wider text-[10px]">Ship Systems</div>
      <div class="space-y-1">
        <div v-for="(val, key) in gs.shipSystems" :key="key" class="flex items-center gap-2">
          <span class="w-24 text-panel-text truncate">{{ formatKey(key) }}</span>
          <div class="flex-1 h-2 bg-gray-800 rounded overflow-hidden">
            <div
              class="h-full rounded transition-all duration-500"
              :class="barColor(val)"
              :style="{ width: (val * 100) + '%' }"
            ></div>
          </div>
          <span class="w-10 text-right tabular-nums" :class="barTextColor(val)">
            {{ Math.round(val * 100) }}%
          </span>
        </div>
      </div>
    </div>

    <!-- Population -->
    <div v-if="gs.population">
      <div class="text-panel-muted mb-1 uppercase tracking-wider text-[10px]">Population</div>
      <div class="flex items-center gap-3 flex-wrap">
        <span class="text-panel-text font-bold">{{ gs.population.total }}</span>
        <span class="text-panel-muted">|</span>
        <span title="Scientists">🔬 {{ gs.population.scientists }}</span>
        <span title="Engineers">⚙ {{ gs.population.engineers }}</span>
        <span title="Medical">🏥 {{ gs.population.medical }}</span>
        <span title="Security">🛡 {{ gs.population.military }}</span>
        <span title="Civilians">👥 {{ gs.population.civilians }}</span>
      </div>
      <div class="flex items-center gap-2 mt-1">
        <span class="text-panel-muted text-[10px]">Morale</span>
        <div class="flex-1 h-1.5 bg-gray-800 rounded overflow-hidden">
          <div
            class="h-full rounded transition-all duration-500"
            :class="moraleColor(gs.population.morale)"
            :style="{ width: (gs.population.morale * 100) + '%' }"
          ></div>
        </div>
        <span class="tabular-nums text-[10px]">{{ Math.round(gs.population.morale * 100) }}%</span>
      </div>
    </div>

    <!-- Transit info -->
    <div v-if="gs.transit">
      <div class="text-panel-muted mb-1 uppercase tracking-wider text-[10px]">Transit</div>
      <div class="space-y-0.5">
        <div>→ {{ gs.transit.destination }} ({{ gs.transit.distance_ly }} ly)</div>
        <div class="flex items-center gap-2">
          <div class="flex-1 h-1.5 bg-gray-800 rounded overflow-hidden">
            <div
              class="h-full bg-blue-500 rounded transition-all"
              :style="{ width: gs.transit.pct_complete + '%' }"
            ></div>
          </div>
          <span class="tabular-nums text-[10px]">{{ gs.transit.pct_complete }}%</span>
        </div>
        <div class="text-panel-muted text-[10px]">
          Phase: {{ gs.transit.transit_phase }} · {{ gs.transit.distance_traveled_ly }} / {{ gs.transit.distance_ly }} ly
        </div>
      </div>
    </div>

    <!-- Search info -->
    <div v-if="gs.search && gs.phase === 'search'">
      <div class="text-panel-muted mb-1 uppercase tracking-wider text-[10px]">Search</div>
      <div>Discovered: {{ gs.search.systems_discovered }} / {{ gs.search.target_discoveries }}</div>
    </div>

    <!-- Notable roster -->
    <div v-if="gs.livingNotables.length">
      <div class="text-panel-muted mb-1 uppercase tracking-wider text-[10px]">Notables</div>
      <div class="space-y-0.5">
        <div v-for="n in gs.livingNotables" :key="n.name" class="flex items-baseline gap-1">
          <span class="text-yellow-400 text-[10px]">★</span>
          <span class="text-panel-text">{{ n.name }}</span>
          <span class="text-panel-muted text-[10px]">{{ n.role }}, {{ n.age }}</span>
          <span class="text-panel-muted text-[10px] italic">{{ n.traits.join(', ') }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import { useGameStateStore } from '../stores/gameState'

const gs = useGameStateStore()

const PHASE_COLORS = {
  loadout: 'bg-purple-800 text-purple-200',
  search: 'bg-blue-800 text-blue-200',
  transit: 'bg-cyan-800 text-cyan-200',
  survey: 'bg-green-800 text-green-200',
  founding: 'bg-yellow-800 text-yellow-200',
}

const phaseBadgeClass = computed(() => PHASE_COLORS[gs.phase] || 'bg-gray-700 text-gray-300')

function formatKey(key) {
  return key.replace(/_/g, ' ').replace(/\b\w/g, c => c.toUpperCase())
}

function fmt(val) {
  if (val == null) return '—'
  if (val >= 10000) return Math.round(val).toLocaleString()
  if (val >= 100) return Math.round(val).toString()
  return val.toFixed(1)
}

function fmtRate(val) {
  if (val == null || val === undefined) return '—'
  const sign = val >= 0 ? '+' : ''
  return sign + val.toFixed(1)
}

function rateClass(val) {
  if (val == null) return 'text-panel-muted'
  if (val > 0.1) return 'text-green-400'
  if (val < -0.1) return 'text-red-400'
  return 'text-panel-muted'
}

function statusDot(val, key) {
  // Thresholds per resource type
  if (val > 500) return 'text-green-400'
  if (val > 100) return 'text-yellow-400'
  return 'text-red-400'
}

function barColor(val) {
  if (val > 0.7) return 'bg-green-500'
  if (val > 0.4) return 'bg-yellow-500'
  return 'bg-red-500'
}

function barTextColor(val) {
  if (val > 0.7) return 'text-green-400'
  if (val > 0.4) return 'text-yellow-400'
  return 'text-red-400'
}

function moraleColor(val) {
  if (val > 0.6) return 'bg-green-500'
  if (val > 0.35) return 'bg-yellow-500'
  return 'bg-red-500'
}
</script>
