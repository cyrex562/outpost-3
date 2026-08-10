<script setup lang="ts">
/** Floating-window wrapper for `BuildingsPanel` (colony details multi-window redesign). */
import { inject } from 'vue'
import BuildingsPanel from '@/components/BuildingsPanel.vue'
import { COLONY_WINDOW_CONTEXT_KEY } from '@/windows/colonyWindows'

const ctx = inject(COLONY_WINDOW_CONTEXT_KEY)
if (!ctx) throw new Error('BuildingsWindowPanel must be mounted with ColonyWindowContext provided')
</script>

<template>
  <BuildingsPanel
    :buildings="ctx.buildings"
    :slots-used="ctx.slotsUsed"
    :slot-capacity="ctx.slotCapacity"
    :labour-available="ctx.labourAvailable"
    :labour-total="ctx.labourTotal"
    @view-details="ctx.viewBuildingDetails"
    @set-priority="ctx.setBuildingPriority"
    @set-lock="ctx.setBuildingLock"
    @rename="ctx.renameBuilding"
    @set-paused="ctx.setBuildingPaused"
    @repair="ctx.repairBuilding"
  />
</template>
