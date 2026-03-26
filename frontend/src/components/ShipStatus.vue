<script setup>
import { computed } from 'vue'
import { useWorldStore } from '../stores/world'

const world = useWorldStore()

// Format large numbers with commas
function fmt(n) {
  if (n === undefined || n === null) return '—'
  return Number(n).toLocaleString()
}

// Integrity bar color
const integrityColor = computed(() => {
  const v = world.ship.integrity ?? 100
  if (v >= 80) return 'bg-green-500'
  if (v >= 50) return 'bg-yellow-400'
  return 'bg-orange-500'
})

const phaseLabel = computed(() => {
  const labels = {
    loadout: 'LOADOUT',
    search: 'SEARCH',
    transit: 'TRANSIT',
    survey: 'SURVEY',
    founding: 'FOUNDING',
    unknown: '—',
  }
  return labels[world.phase] ?? world.phase.toUpperCase()
})
</script>

<template>
  <div class="h-full overflow-y-auto p-2 text-xs font-mono space-y-3">

    <!-- Loading state -->
    <div v-if="!world.loaded" class="text-panel-muted">Awaiting ship data...</div>

    <template v-else>

      <!-- Ship name + phase -->
      <div class="space-y-1">
        <div class="text-panel-accent font-semibold truncate">
          {{ world.ship.name || '—' }}
        </div>
        <div class="flex gap-3 text-panel-muted">
          <span>Phase: <span class="text-panel-text">{{ phaseLabel }}</span></span>
          <span>Pop: <span class="text-panel-text">{{ fmt(world.population.count) }}</span></span>
        </div>
      </div>

      <!-- Hull integrity -->
      <div class="space-y-1">
        <div class="text-panel-muted uppercase tracking-wider text-[10px]">Hull Integrity</div>
        <div class="flex items-center gap-2">
          <div class="flex-1 h-1.5 bg-panel-border rounded-full overflow-hidden">
            <div
              class="h-full rounded-full transition-all"
              :class="integrityColor"
              :style="{ width: (world.ship.integrity ?? 100) + '%' }"
            />
          </div>
          <span class="text-panel-text w-8 text-right">{{ (world.ship.integrity ?? 100).toFixed(0) }}%</span>
        </div>
      </div>

      <!-- Resources -->
      <div class="space-y-1">
        <div class="text-panel-muted uppercase tracking-wider text-[10px]">Resources</div>
        <table class="w-full">
          <tbody>
            <tr v-for="[key, label] in [
              ['food', 'Food'],
              ['water', 'Water'],
              ['medicine', 'Medicine'],
              ['fuel', 'Fuel'],
              ['spare_parts', 'Parts'],
            ]" :key="key">
              <td class="text-panel-muted pr-2 py-0.5 w-20">{{ label }}</td>
              <td class="text-panel-text text-right tabular-nums">{{ fmt(world.resources[key]) }}</td>
            </tr>
          </tbody>
        </table>
      </div>

      <!-- Notables roster -->
      <div class="space-y-1">
        <div class="text-panel-muted uppercase tracking-wider text-[10px]">
          Mission Officers ({{ world.notables.length }})
        </div>
        <div
          v-for="n in world.notables"
          :key="n.name"
          class="border border-panel-border rounded px-2 py-1 space-y-0.5"
          :class="{ 'opacity-40 line-through': !n.alive }"
        >
          <div class="flex justify-between gap-2">
            <span class="text-panel-accent truncate">{{ n.name }}</span>
            <span class="text-panel-muted shrink-0">{{ n.role }}</span>
          </div>
          <div class="text-panel-muted text-[10px]">{{ n.traits.join(' · ') }}</div>
        </div>
      </div>

    </template>
  </div>
</template>
