<script setup>
import { computed, ref } from 'vue'
import { useWorldStore } from '../stores/world'
import { useApi } from '../composables/useApi'

const world = useWorldStore()
const api = useApi()
const busy = ref(false)
const apiError = ref('')

async function setDestination(systemName) {
  busy.value = true
  apiError.value = ''
  try {
    await api.selectDestination(systemName)
  } catch (e) {
    apiError.value = e.message
  } finally {
    busy.value = false
  }
}

async function surveyChoice(decision) {
  busy.value = true
  apiError.value = ''
  try {
    await api.surveyDecision(decision)
  } catch (e) {
    apiError.value = e.message
  } finally {
    busy.value = false
  }
}

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
      <div v-if="world.transit?.duration_days && world.phase === 'transit'" class="space-y-1">
        <div class="text-panel-muted uppercase tracking-wider text-[10px]">Transit to {{ world.transit.destination_name }}</div>
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
      </div>

      <!-- Survey progress -->
      <div v-if="world.phase === 'survey' && world.survey?.target_system_name" class="space-y-1">
        <div class="text-panel-muted uppercase tracking-wider text-[10px]">Surveying {{ world.survey.target_system_name }}</div>
        <div class="flex items-center gap-2">
          <div class="flex-1 h-1.5 bg-panel-border rounded-full overflow-hidden">
            <div
              class="h-full bg-teal-400 rounded-full transition-all"
              :style="{ width: (world.survey.progress * 100) + '%' }"
            />
          </div>
          <span class="text-panel-text w-8 text-right">{{ (world.survey.progress * 100).toFixed(0) }}%</span>
        </div>
        <div class="text-panel-muted text-[10px]">
          Day {{ world.survey.days_elapsed }} of {{ world.survey.survey_duration_days }} · verdict at 90%
        </div>
      </div>

      <!-- Founding complete -->
      <div v-if="world.phase === 'founding' && world.colony?.planet_name" class="space-y-1">
        <div class="text-panel-muted uppercase tracking-wider text-[10px]">Colony Established</div>
        <div class="text-teal-300 font-semibold">{{ world.colony.planet_name }}</div>
        <div class="text-panel-muted text-[10px]">{{ world.colony.system_name }} system · Year {{ world.colony.founded_year }}</div>
      </div>

      <!-- Survey verdict action buttons -->
      <div v-if="world.phase === 'survey' && world.survey?.verdict" class="space-y-1.5">
        <div class="text-panel-muted uppercase tracking-wider text-[10px]">Survey Decision Required</div>
        <div class="border border-yellow-700/60 rounded px-2 py-2 bg-yellow-950/20 space-y-2">
          <div class="text-yellow-300 text-[11px]">
            Crew recommends:
            <span class="font-semibold uppercase">{{ world.survey.verdict }}</span>
          </div>
          <div class="flex gap-2">
            <button
              class="flex-1 py-1 rounded border text-[11px] font-semibold transition-colors"
              :class="busy ? 'opacity-40 cursor-not-allowed border-panel-border text-panel-muted' : 'border-teal-600 text-teal-300 hover:bg-teal-900/30'"
              :disabled="busy"
              @click="surveyChoice('accept')"
            >
              ✓ Settle Here
            </button>
            <button
              class="flex-1 py-1 rounded border text-[11px] font-semibold transition-colors"
              :class="busy ? 'opacity-40 cursor-not-allowed border-panel-border text-panel-muted' : 'border-orange-600 text-orange-300 hover:bg-orange-900/20'"
              :disabled="busy"
              @click="surveyChoice('reject')"
            >
              ✗ Keep Searching
            </button>
          </div>
          <div v-if="world.survey.rejections_remaining !== undefined" class="text-[10px] text-panel-muted">
            {{ world.survey.rejections_remaining }} rejection(s) remaining before forced settlement
          </div>
        </div>
        <div v-if="apiError" class="text-orange-400 text-[10px]">{{ apiError }}</div>
      </div>

      <!-- System list -->
      <div class="space-y-1">
        <div class="text-panel-muted uppercase tracking-wider text-[10px]">
          {{ world.phase === 'search' ? 'Select Destination' : 'Candidate Systems' }}
        </div>
        <div v-if="apiError && world.phase === 'search'" class="text-orange-400 text-[10px]">{{ apiError }}</div>
        <div
          v-for="s in world.systems"
          :key="s.name"
          class="border rounded px-2 py-1 space-y-0.5 text-[10px]"
          :class="s.selected ? 'border-indigo-500 bg-indigo-950/30' : 'border-panel-border'"
        >
          <div class="flex justify-between items-start gap-1">
            <span class="font-semibold" :class="s.selected ? 'text-indigo-300' : 'text-panel-text'">
              {{ s.name }}
              <span v-if="s.selected" class="text-indigo-400 ml-1">★ destination</span>
            </span>
            <span class="text-panel-muted shrink-0">{{ s.star_type }} · {{ s.distance_ly }} ly</span>
          </div>
          <div class="flex items-center justify-between gap-2">
            <span class="text-panel-muted">
              {{ s.planets.length }} world{{ s.planets.length !== 1 ? 's' : '' }} ·
              best habitability
              <span :style="{ color: habitabilityColor(s.best_habitability) }">
                {{ s.best_habitability.toFixed(0) }}%
              </span>
            </span>
            <!-- Set Course button — only visible in search phase -->
            <button
              v-if="world.phase === 'search'"
              class="shrink-0 px-2 py-0.5 rounded border text-[10px] font-semibold transition-colors"
              :class="busy
                ? 'opacity-40 cursor-not-allowed border-panel-border text-panel-muted'
                : 'border-indigo-600 text-indigo-300 hover:bg-indigo-900/30'"
              :disabled="busy"
              @click="setDestination(s.name)"
            >
              Set Course →
            </button>
          </div>
        </div>
      </div>

    </template>
  </div>
</template>
