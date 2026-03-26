<script setup>
import { computed } from 'vue'
import { useWorldStore } from '../stores/world'

const world = useWorldStore()

function fmt(n) {
  if (n === undefined || n === null) return '—'
  return Number(n).toLocaleString()
}

// Convert person-days of food/water to years at current population
function toYears(units, pop) {
  if (!pop || !units) return null
  return (units / (pop * 365)).toFixed(1)
}

const integrityColor = computed(() => {
  const v = world.ship.integrity ?? 100
  if (v >= 80) return 'bg-green-500'
  if (v >= 50) return 'bg-yellow-400'
  return 'bg-orange-500'
})

const phaseLabel = computed(() => {
  const labels = {
    loadout:  'LOADOUT',
    search:   'SEARCH',
    transit:  'TRANSIT',
    survey:   'SURVEY',
    founding: 'FOUNDING',
    unknown:  '—',
  }
  return labels[world.phase] ?? world.phase.toUpperCase()
})

// Resource rows: [key, label, unit, show_years]
const resourceRows = computed(() => {
  const pop = world.population.count
  const r = world.resources
  return [
    {
      key: 'food', label: 'Food',
      raw: fmt(r.food),
      suffix: toYears(r.food, pop) ? `${toYears(r.food, pop)} yr` : null,
      critical: r.food < pop * 365,
    },
    {
      key: 'water', label: 'Water',
      raw: fmt(r.water),
      suffix: toYears(r.water, pop) ? `${toYears(r.water, pop)} yr` : null,
      critical: r.water < pop * 365,
    },
    {
      key: 'medicine', label: 'Medicine',
      raw: fmt(r.medicine),
      suffix: null,
      critical: r.medicine < pop * 10,
    },
    {
      key: 'fuel', label: 'Fuel',
      raw: fmt(r.fuel),
      suffix: null,
      critical: r.fuel < 50_000,
    },
    {
      key: 'spare_parts', label: 'Parts',
      raw: fmt(r.spare_parts),
      suffix: null,
      critical: r.spare_parts < 5_000,
    },
  ]
})
</script>

<template>
  <div class="h-full overflow-y-auto p-2 text-xs font-mono space-y-3">

    <div v-if="!world.loaded" class="text-panel-muted">Awaiting ship data...</div>

    <template v-else>

      <!-- Ship name + status line -->
      <div class="space-y-1">
        <div class="text-panel-accent font-semibold truncate">
          {{ world.ship.name || '—' }}
        </div>
        <div class="flex gap-3 text-panel-muted flex-wrap">
          <span>Phase: <span class="text-panel-text">{{ phaseLabel }}</span></span>
          <span>Pop: <span class="text-panel-text">{{ fmt(world.population.count) }}</span></span>
          <span v-if="world.population.births_this_year > 0" class="text-green-400">
            +{{ world.population.births_this_year }} births
          </span>
          <span v-if="world.population.deaths_this_year > 0" class="text-red-400">
            −{{ world.population.deaths_this_year }} deaths
          </span>
        </div>
      </div>

      <!-- Hull integrity bar -->
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
          <span class="text-panel-text w-8 text-right">{{ (world.ship.integrity ?? 100).toFixed(1) }}%</span>
        </div>
      </div>

      <!-- Resources -->
      <div class="space-y-1">
        <div class="text-panel-muted uppercase tracking-wider text-[10px]">Resources</div>
        <table class="w-full">
          <tbody>
            <tr v-for="row in resourceRows" :key="row.key">
              <td class="text-panel-muted pr-2 py-0.5 w-16">{{ row.label }}</td>
              <td class="text-right tabular-nums py-0.5" :class="row.critical ? 'text-orange-400' : 'text-panel-text'">
                {{ row.raw }}
              </td>
              <td class="text-right text-panel-muted pl-2 py-0.5 w-12 tabular-nums">
                {{ row.suffix ?? '' }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <!-- Notable roster -->
      <div class="space-y-1">
        <div class="text-panel-muted uppercase tracking-wider text-[10px]">
          Officers ({{ world.notables.length }})
        </div>
        <div
          v-for="n in world.notables"
          :key="n.name"
          class="border border-panel-border rounded px-2 py-1 space-y-0.5"
          :class="{ 'opacity-40': !n.alive }"
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
