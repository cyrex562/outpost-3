<script setup lang="ts">
/** Floating-window wrapper for `VitalStatsPanel` (colony details multi-window redesign). */
import { inject } from 'vue'
import VitalStatsPanel from '@/components/VitalStatsPanel.vue'
import { COLONY_WINDOW_CONTEXT_KEY } from '@/windows/colonyWindows'

const ctx = inject(COLONY_WINDOW_CONTEXT_KEY)
if (!ctx) throw new Error('VitalStatsWindowPanel must be mounted with ColonyWindowContext provided')
</script>

<template>
  <VitalStatsPanel
    :population="ctx.population"
    :stability="ctx.stability"
    :morale="ctx.morale"
    :productivity-modifier="ctx.productivityModifier"
    :productivity-note="ctx.productivityNote"
    :available-labour="ctx.availableLabour"
    :population-trend="ctx.populationTrend"
    :slots-used="ctx.slotsUsed"
    :slot-capacity="ctx.slotCapacity"
    :labour-employed="ctx.labourEmployed"
    :labour-unemployed="ctx.labourUnemployed"
  />
</template>
