<template>
  <div class="p-2 text-xs select-none h-full flex flex-col">
    <!-- Toggle buttons -->
    <div class="flex gap-1 flex-wrap mb-2 shrink-0">
      <button
        v-for="key in allKeys"
        :key="key"
        class="px-1.5 py-0.5 rounded text-[10px] border transition-colors"
        :class="activeKeys.has(key) ? 'border-opacity-80 text-opacity-100' : 'border-gray-700 text-panel-muted'"
        :style="activeKeys.has(key) ? { borderColor: LINE_COLORS[key], color: LINE_COLORS[key] } : {}"
        @click="toggleKey(key)"
      >{{ formatKey(key) }}</button>
    </div>

    <!-- SVG chart -->
    <div ref="chartContainer" class="flex-1 min-h-0 relative">
      <svg
        v-if="history.length > 1"
        :viewBox="`0 0 ${W} ${H}`"
        preserveAspectRatio="none"
        class="w-full h-full"
      >
        <!-- Grid lines -->
        <line
          v-for="i in 4"
          :key="'grid-' + i"
          :x1="0" :y1="H * i / 5"
          :x2="W" :y2="H * i / 5"
          stroke="#1a2a3a" stroke-width="0.5"
        />

        <!-- Data lines -->
        <polyline
          v-for="key in visibleKeys"
          :key="key"
          :points="pathFor(key)"
          fill="none"
          :stroke="LINE_COLORS[key] || '#888'"
          stroke-width="1.5"
          stroke-linejoin="round"
          vector-effect="non-scaling-stroke"
        />
      </svg>

      <!-- No data placeholder -->
      <div v-else class="absolute inset-0 flex items-center justify-center text-panel-muted">
        Waiting for data...
      </div>

      <!-- Y-axis labels (overlay) -->
      <div v-if="history.length > 1" class="absolute top-0 left-0 h-full flex flex-col justify-between pointer-events-none py-0.5">
        <span
          v-for="(label, i) in yLabels"
          :key="i"
          class="text-[9px] text-panel-muted bg-panel-bg/80 px-0.5"
        >{{ label }}</span>
      </div>

      <!-- X-axis labels (overlay) -->
      <div v-if="history.length > 1" class="absolute bottom-0 left-0 w-full flex justify-between pointer-events-none px-1">
        <span class="text-[9px] text-panel-muted bg-panel-bg/80 px-0.5">Y{{ xStart }}</span>
        <span class="text-[9px] text-panel-muted bg-panel-bg/80 px-0.5">Y{{ xEnd }}</span>
      </div>
    </div>

    <!-- Legend -->
    <div v-if="visibleKeys.length" class="flex gap-2 flex-wrap mt-1 shrink-0">
      <div v-for="key in visibleKeys" :key="'leg-' + key" class="flex items-center gap-1">
        <span class="inline-block w-2 h-0.5 rounded" :style="{ background: LINE_COLORS[key] || '#888' }"></span>
        <span class="text-[10px] text-panel-muted">{{ formatKey(key) }}: {{ currentVal(key) }}</span>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, reactive } from 'vue'
import { useGameStateStore } from '../stores/gameState'

const gs = useGameStateStore()

const W = 400
const H = 200

const LINE_COLORS = {
  materials: '#a78bfa',    // purple
  fuel: '#f97316',         // orange
  food: '#22c55e',         // green
  water: '#3b82f6',        // blue
  spare_parts: '#eab308',  // yellow
}

const allKeys = ['materials', 'fuel', 'food', 'water', 'spare_parts']
const activeKeys = reactive(new Set(['fuel', 'food', 'water']))

const history = computed(() => gs.history)

const visibleKeys = computed(() => allKeys.filter(k => activeKeys.has(k)))

function toggleKey(key) {
  if (activeKeys.has(key)) {
    activeKeys.delete(key)
  } else {
    activeKeys.add(key)
  }
}

function formatKey(key) {
  return key.replace(/_/g, ' ').replace(/\b\w/g, c => c.toUpperCase())
}

// Compute min/max across all visible keys for scaling
const yRange = computed(() => {
  const h = history.value
  if (!h.length) return { min: 0, max: 100 }

  let min = Infinity
  let max = -Infinity
  for (const entry of h) {
    for (const key of visibleKeys.value) {
      const v = entry.resources?.[key]
      if (v != null) {
        if (v < min) min = v
        if (v > max) max = v
      }
    }
  }

  if (min === Infinity) return { min: 0, max: 100 }
  // Add 10% padding
  const range = max - min || 1
  return { min: Math.max(0, min - range * 0.05), max: max + range * 0.05 }
})

function pathFor(key) {
  const h = history.value
  if (h.length < 2) return ''

  const { min, max } = yRange.value
  const range = max - min || 1

  const points = []
  for (let i = 0; i < h.length; i++) {
    const x = (i / (h.length - 1)) * W
    const val = h[i].resources?.[key] ?? 0
    const y = H - ((val - min) / range) * H
    points.push(`${x.toFixed(1)},${y.toFixed(1)}`)
  }
  return points.join(' ')
}

const xStart = computed(() => {
  const h = history.value
  return h.length ? h[0].year : 1
})

const xEnd = computed(() => {
  const h = history.value
  return h.length ? h[h.length - 1].year : 1
})

const yLabels = computed(() => {
  const { min, max } = yRange.value
  const labels = []
  for (let i = 0; i <= 4; i++) {
    const val = max - (i / 4) * (max - min)
    labels.push(val >= 1000 ? Math.round(val / 1000) + 'k' : Math.round(val).toString())
  }
  return labels
})

function currentVal(key) {
  if (!gs.resources) return '—'
  const v = gs.resources[key]
  if (v == null) return '—'
  return v >= 1000 ? Math.round(v).toLocaleString() : v.toFixed(1)
}
</script>
