<script setup>
import { computed } from 'vue'
import { useWorldStore } from '../stores/world'

const world = useWorldStore()

// Sparkline config
const W = 200   // SVG width per sparkline
const H = 36    // SVG height per sparkline

// Resources to graph: [key, label, color]
const TRACKS = [
  { key: 'food',        label: 'Food',     color: '#4ade80' },
  { key: 'water',       label: 'Water',    color: '#38bdf8' },
  { key: 'medicine',    label: 'Medicine', color: '#f472b6' },
  { key: 'fuel',        label: 'Fuel',     color: '#a78bfa' },
]

// Build polyline points for a given resource key
function sparkPoints(key) {
  const snaps = world.resourceHistory
  if (!snaps || snaps.length < 2) return ''

  const values = snaps.map(s => s[key] ?? 0)
  const maxVal = Math.max(...values, 1)

  return snaps
    .map((_, i) => {
      const x = (i / (snaps.length - 1)) * W
      const y = H - (values[i] / maxVal) * (H - 2) - 1
      return `${x.toFixed(1)},${y.toFixed(1)}`
    })
    .join(' ')
}

// Current value for each track (from live resources, not history)
function currentVal(key) {
  const v = world.resources?.[key]
  if (v === undefined || v === null) return '—'
  if (v >= 1_000_000) return `${(v / 1_000_000).toFixed(1)}M`
  if (v >= 1_000) return `${(v / 1_000).toFixed(0)}k`
  return String(v)
}

// Percentage of the initial (max) value seen in history
function pctOfMax(key) {
  const snaps = world.resourceHistory
  if (!snaps || snaps.length < 2) return null
  const values = snaps.map(s => s[key] ?? 0)
  const maxVal = Math.max(...values, 1)
  const curVal = values[values.length - 1]
  return Math.round((curVal / maxVal) * 100)
}

function pctColor(pct) {
  if (pct === null) return '#6b7280'
  if (pct >= 60) return '#4ade80'
  if (pct >= 30) return '#facc15'
  return '#fb923c'
}

const hasHistory = computed(() => world.resourceHistory && world.resourceHistory.length >= 2)
</script>

<template>
  <div class="h-full overflow-y-auto p-2 text-xs font-mono space-y-3">

    <div v-if="!world.loaded" class="text-panel-muted">Awaiting telemetry data...</div>

    <template v-else>

      <div v-if="!hasHistory" class="text-panel-muted italic text-center py-6">
        Resource history accumulates during transit (one point per year).
      </div>

      <div v-else class="space-y-3">
        <div v-for="track in TRACKS" :key="track.key" class="space-y-0.5">

          <!-- Label + current value -->
          <div class="flex justify-between items-baseline">
            <span class="text-panel-muted">{{ track.label }}</span>
            <span class="tabular-nums" :style="{ color: pctColor(pctOfMax(track.key)) }">
              {{ currentVal(track.key) }}
              <span v-if="pctOfMax(track.key) !== null" class="text-[10px] text-panel-muted ml-1">
                ({{ pctOfMax(track.key) }}%)
              </span>
            </span>
          </div>

          <!-- Sparkline SVG -->
          <div class="border border-panel-border rounded overflow-hidden bg-black/30">
            <svg :width="W" :height="H" class="w-full" style="display:block">
              <!-- Zero-line -->
              <line :x1="0" :y1="H - 1" :x2="W" :y2="H - 1"
                stroke="#374151" stroke-width="0.5" />

              <!-- Sparkline polyline -->
              <polyline
                v-if="sparkPoints(track.key)"
                :points="sparkPoints(track.key)"
                fill="none"
                :stroke="track.color"
                stroke-width="1.5"
                stroke-linejoin="round"
                stroke-linecap="round"
                opacity="0.85"
              />

              <!-- Filled area under curve -->
              <polyline
                v-if="sparkPoints(track.key)"
                :points="`0,${H} ${sparkPoints(track.key)} ${W},${H}`"
                :fill="track.color"
                fill-opacity="0.08"
                stroke="none"
              />
            </svg>
          </div>

        </div>
      </div>

      <!-- Population mini-chart if history available -->
      <div v-if="hasHistory" class="space-y-0.5 pt-1 border-t border-panel-border">
        <div class="flex justify-between items-baseline">
          <span class="text-panel-muted">Population</span>
          <span class="text-panel-text tabular-nums">
            {{ world.population.count?.toLocaleString() ?? '—' }}
          </span>
        </div>
        <div class="border border-panel-border rounded overflow-hidden bg-black/30">
          <svg :width="W" :height="H" class="w-full" style="display:block">
            <line :x1="0" :y1="H - 1" :x2="W" :y2="H - 1"
              stroke="#374151" stroke-width="0.5" />
            <polyline
              v-if="sparkPoints('population')"
              :points="sparkPoints('population')"
              fill="none"
              stroke="#fbbf24"
              stroke-width="1.5"
              stroke-linejoin="round"
              stroke-linecap="round"
              opacity="0.85"
            />
            <polyline
              v-if="sparkPoints('population')"
              :points="`0,${H} ${sparkPoints('population')} ${W},${H}`"
              fill="#fbbf24"
              fill-opacity="0.08"
              stroke="none"
            />
          </svg>
        </div>
      </div>

      <!-- Legend note -->
      <div class="text-[10px] text-panel-muted text-center">
        {{ world.resourceHistory?.length ?? 0 }} year snapshot(s) · relative to mission peak
      </div>

    </template>
  </div>
</template>
