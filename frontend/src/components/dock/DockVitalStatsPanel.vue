<script setup lang="ts">
/**
 * `dockview-vue` panel wrapper for `VitalStatsPanel` (issue #321). Reads
 * colony data from the injected `ColonyDockContext` rather than dockview's
 * own `params` mechanism — see `colonyDock.ts` for why.
 */
import { inject } from 'vue'
import VitalStatsPanel from '@/components/VitalStatsPanel.vue'
import { COLONY_DOCK_CONTEXT_KEY } from '@/dock/colonyDock'

defineProps<{ params?: unknown }>()

const ctx = inject(COLONY_DOCK_CONTEXT_KEY)
if (!ctx) throw new Error('DockVitalStatsPanel must be mounted inside a colony dockview with ColonyDockContext provided')
</script>

<template>
  <VitalStatsPanel
    :population="ctx.population"
    :stability="ctx.stability"
    :available-labour="ctx.availableLabour"
    :population-trend="ctx.populationTrend"
    :slots-used="ctx.slotsUsed"
    :slot-capacity="ctx.slotCapacity"
    :labour-employed="ctx.labourEmployed"
    :labour-unemployed="ctx.labourUnemployed"
  />
</template>
