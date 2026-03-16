<template>
  <div class="p-2 text-xs select-none h-full flex flex-col">
    <svg
      :viewBox="`0 0 ${SIZE} ${SIZE}`"
      class="flex-1 min-h-0 w-full"
      preserveAspectRatio="xMidYMid meet"
    >
      <!-- Background grid -->
      <defs>
        <pattern id="grid" width="40" height="40" patternUnits="userSpaceOnUse">
          <path d="M 40 0 L 0 0 0 40" fill="none" stroke="#0f1a2a" stroke-width="0.5" />
        </pattern>
      </defs>
      <rect width="100%" height="100%" fill="url(#grid)" />

      <!-- Distance rings from home (5, 10, 15, 20, 25 ly) -->
      <circle
        v-for="r in rings"
        :key="'ring-' + r"
        :cx="CENTER" :cy="CENTER"
        :r="r * scale"
        fill="none" stroke="#1a2a3a" stroke-width="0.5" stroke-dasharray="3,3"
      />
      <!-- Ring labels -->
      <text
        v-for="r in rings"
        :key="'rlabel-' + r"
        :x="CENTER + r * scale + 2" :y="CENTER - 2"
        fill="#445" font-size="8"
      >{{ r }} ly</text>

      <!-- Travel line (home → destination) -->
      <line
        v-if="destination"
        :x1="CENTER" :y1="CENTER"
        :x2="destination.x" :y2="destination.y"
        stroke="#0f3460" stroke-width="1" stroke-dasharray="4,4"
      />

      <!-- Home system -->
      <circle :cx="CENTER" :cy="CENTER" r="4" fill="#e94560" />
      <text :x="CENTER + 7" :y="CENTER + 3" fill="#e94560" font-size="9" font-weight="bold">Home</text>

      <!-- Discovered systems -->
      <g v-for="sys in systemPositions" :key="sys.name">
        <circle
          :cx="sys.x" :cy="sys.y"
          :r="sys.selected ? 5 : 3"
          :fill="sys.selected ? '#22c55e' : habColor(sys.hab)"
          :stroke="sys.selected ? '#22c55e' : 'none'"
          :stroke-width="sys.selected ? 2 : 0"
          :opacity="sys.selected ? 1 : 0.8"
        />
        <text
          :x="sys.x + 7" :y="sys.y + 3"
          :fill="sys.selected ? '#22c55e' : '#8899aa'"
          font-size="8"
        >{{ sys.name }} ({{ sys.hab.toFixed(2) }})</text>
      </g>

      <!-- Ship position during transit -->
      <g v-if="shipPos">
        <polygon
          :points="shipTriangle(shipPos.x, shipPos.y)"
          fill="#3b82f6"
        />
        <circle :cx="shipPos.x" :cy="shipPos.y" r="8" fill="none" stroke="#3b82f6" stroke-width="0.5" opacity="0.4">
          <animate attributeName="r" values="6;12;6" dur="2s" repeatCount="indefinite" />
          <animate attributeName="opacity" values="0.4;0.1;0.4" dur="2s" repeatCount="indefinite" />
        </circle>
      </g>
    </svg>

    <!-- Info footer -->
    <div v-if="gs.transit" class="shrink-0 mt-1 text-[10px] text-panel-muted">
      En route to {{ gs.transit.destination }} · {{ gs.transit.pct_complete }}% · {{ gs.transit.transit_phase }}
    </div>
    <div v-else-if="gs.phase === 'search'" class="shrink-0 mt-1 text-[10px] text-panel-muted">
      Scanning for candidate systems...
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import { useGameStateStore } from '../stores/gameState'

const gs = useGameStateStore()

const SIZE = 300
const CENTER = SIZE / 2
const MAX_DISTANCE = 30 // ly to fit in view

const scale = computed(() => (SIZE * 0.42) / MAX_DISTANCE)
const rings = [5, 10, 15, 20, 25]

// Deterministic angle from system name (hash-like)
function nameToAngle(name) {
  let hash = 0
  for (let i = 0; i < name.length; i++) {
    hash = ((hash << 5) - hash + name.charCodeAt(i)) | 0
  }
  return (hash % 360) * (Math.PI / 180)
}

const systemPositions = computed(() => {
  return gs.starSystems.map(sys => {
    const angle = nameToAngle(sys.name)
    const r = sys.distance_ly * scale.value
    return {
      name: sys.name,
      x: CENTER + Math.cos(angle) * r,
      y: CENTER + Math.sin(angle) * r,
      hab: sys.habitability_score,
      selected: sys.selected,
      distance: sys.distance_ly,
    }
  })
})

const destination = computed(() => {
  return systemPositions.value.find(s => s.selected) || null
})

const shipPos = computed(() => {
  if (!gs.transit || !destination.value) return null
  const pct = gs.transit.pct_complete / 100
  return {
    x: CENTER + (destination.value.x - CENTER) * pct,
    y: CENTER + (destination.value.y - CENTER) * pct,
  }
})

function habColor(hab) {
  if (hab > 0.6) return '#22c55e'  // green
  if (hab > 0.3) return '#eab308'  // yellow
  return '#6b7280'                  // gray
}

function shipTriangle(x, y) {
  // Small triangle pointing toward destination
  const size = 4
  return `${x},${y - size} ${x - size * 0.7},${y + size * 0.5} ${x + size * 0.7},${y + size * 0.5}`
}
</script>
