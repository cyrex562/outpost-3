<script setup>
import { computed } from 'vue'
import { useWorldStore } from '../stores/world'

const world = useWorldStore()

// SVG viewport: 220×220, home system at center (110, 110)
const CX = 110
const CY = 110
const MAX_LY = 25   // max display radius in ly
const SCALE = 95 / MAX_LY  // px per light-year

function systemCoords(system) {
  const rad = (system.angle_deg * Math.PI) / 180
  const r = Math.min(system.distance_ly, MAX_LY) * SCALE
  return {
    x: CX + r * Math.cos(rad),
    y: CY + r * Math.sin(rad),
  }
}

function habitabilityColor(hab) {
  if (hab >= 60) return '#4ade80'   // green
  if (hab >= 35) return '#facc15'   // yellow
  if (hab >= 15) return '#fb923c'   // orange
  return '#6b7280'                  // gray
}

// Ship position along the path to destination during transit
const shipPos = computed(() => {
  if (!world.transit?.destination_name || !world.systems?.length) return null
  const dest = world.systems.find((s) => s.selected)
  if (!dest) return null
  const progress = world.transit.progress ?? 0
  const coords = systemCoords(dest)
  return {
    x: CX + (coords.x - CX) * progress,
    y: CY + (coords.y - CY) * progress,
  }
})

// Years remaining / elapsed display
const transitLabel = computed(() => {
  if (!world.transit?.duration_days) return null
  const remaining = world.transit.days_remaining / 365
  const elapsed = world.transit.days_elapsed / 365
  return { remaining: remaining.toFixed(1), elapsed: elapsed.toFixed(1) }
})
</script>

<template>
  <div class="h-full overflow-y-auto p-2 text-xs font-mono space-y-3">

    <div v-if="!world.loaded" class="text-panel-muted">Awaiting navigation data...</div>

    <template v-else>

      <!-- SVG Star Map -->
      <div class="flex justify-center">
        <svg width="220" height="220" class="rounded border border-panel-border bg-black/40">

          <!-- Range rings -->
          <circle v-for="r in [5, 10, 15, 20, 25]" :key="r"
            :cx="CX" :cy="CY"
            :r="r * SCALE"
            fill="none" stroke="#374151" stroke-width="0.5"
          />

          <!-- Axis lines -->
          <line :x1="CX" y1="0" :x2="CX" y2="220" stroke="#374151" stroke-width="0.5" />
          <line x1="0" :y1="CY" x2="220" :y2="CY" stroke="#374151" stroke-width="0.5" />

          <!-- Range labels -->
          <text v-for="r in [10, 20]" :key="'lbl-'+r"
            :x="CX + r * SCALE + 2" :y="CY - 2"
            fill="#4b5563" font-size="7" font-family="monospace"
          >{{ r }}ly</text>

          <!-- Transit path to destination -->
          <template v-if="world.transit?.destination_name">
            <line
              v-for="s in world.systems.filter(s => s.selected)" :key="'path-'+s.name"
              :x1="CX" :y1="CY"
              :x2="systemCoords(s).x" :y2="systemCoords(s).y"
              stroke="#6366f1" stroke-width="1" stroke-dasharray="3 3" opacity="0.6"
            />
          </template>

          <!-- Candidate systems -->
          <g v-for="s in world.systems" :key="s.name">
            <!-- Selection ring -->
            <circle v-if="s.selected"
              :cx="systemCoords(s).x" :cy="systemCoords(s).y"
              r="8" fill="none"
              stroke="#6366f1" stroke-width="1.5"
            />
            <!-- System dot -->
            <circle
              :cx="systemCoords(s).x" :cy="systemCoords(s).y"
              r="4"
              :fill="habitabilityColor(s.best_habitability)"
              opacity="0.9"
            />
            <!-- Name label -->
            <text
              :x="systemCoords(s).x + 6" :y="systemCoords(s).y + 3"
              fill="#d1d5db" font-size="7" font-family="monospace"
            >{{ s.name.split(' ').slice(-1)[0] }}</text>
          </g>

          <!-- Ship position -->
          <g v-if="shipPos && !world.transit?.arrived">
            <polygon
              :points="`${shipPos.x},${shipPos.y - 5} ${shipPos.x - 3},${shipPos.y + 3} ${shipPos.x + 3},${shipPos.y + 3}`"
              fill="#a5b4fc"
            />
          </g>

          <!-- Home system (origin) -->
          <circle :cx="CX" :cy="CY" r="3" fill="#60a5fa" />
          <text :x="CX + 4" :y="CY - 4" fill="#60a5fa" font-size="7" font-family="monospace">Home</text>

        </svg>
      </div>

      <!-- Legend -->
      <div class="flex gap-3 text-[10px] text-panel-muted justify-center">
        <span><span class="text-green-400">●</span> High (&ge;60%)</span>
        <span><span class="text-yellow-400">●</span> Med (&ge;35%)</span>
        <span><span class="text-orange-400">●</span> Low (&ge;15%)</span>
        <span><span class="text-gray-500">●</span> Barren</span>
      </div>

      <!-- Transit progress -->
      <div v-if="world.transit?.duration_days" class="space-y-1">
        <div class="text-panel-muted uppercase tracking-wider text-[10px]">Transit to {{ world.transit.destination_name }}</div>

        <div v-if="world.transit.arrived" class="text-green-400 font-semibold">
          Arrived — survey operations underway
        </div>
        <template v-else>
          <div class="flex items-center gap-2">
            <div class="flex-1 h-1.5 bg-panel-border rounded-full overflow-hidden">
              <div
                class="h-full bg-indigo-400 rounded-full transition-all"
                :style="{ width: (world.transit.progress * 100) + '%' }"
              />
            </div>
            <span class="text-panel-text w-8 text-right">{{ (world.transit.progress * 100).toFixed(1) }}%</span>
          </div>
          <div class="text-panel-muted" v-if="transitLabel">
            {{ transitLabel.elapsed }} yr elapsed · {{ transitLabel.remaining }} yr remaining
          </div>
        </template>
      </div>

      <!-- System list -->
      <div class="space-y-1">
        <div class="text-panel-muted uppercase tracking-wider text-[10px]">Candidate Systems</div>
        <div
          v-for="s in world.systems"
          :key="s.name"
          class="border rounded px-2 py-1 space-y-0.5 text-[10px]"
          :class="s.selected ? 'border-indigo-500 bg-indigo-950/30' : 'border-panel-border'"
        >
          <div class="flex justify-between">
            <span class="font-semibold" :class="s.selected ? 'text-indigo-300' : 'text-panel-text'">
              {{ s.name }}
              <span v-if="s.selected" class="text-indigo-400 ml-1">★ destination</span>
            </span>
            <span class="text-panel-muted">{{ s.star_type }} · {{ s.distance_ly }} ly</span>
          </div>
          <div class="text-panel-muted">
            {{ s.planets.length }} world{{ s.planets.length !== 1 ? 's' : '' }} ·
            best habitability
            <span :style="{ color: habitabilityColor(s.best_habitability) }">
              {{ s.best_habitability.toFixed(0) }}%
            </span>
          </div>
        </div>
      </div>

    </template>
  </div>
</template>
