<script setup lang="ts">
/** `dockview-vue` panel wrapper for `BuildingsPanel` (issue #321). */
import { inject } from 'vue'
import BuildingsPanel from '@/components/BuildingsPanel.vue'
import { COLONY_DOCK_CONTEXT_KEY } from '@/dock/colonyDock'

defineProps<{ params?: unknown }>()

const ctx = inject(COLONY_DOCK_CONTEXT_KEY)
if (!ctx) throw new Error('DockBuildingsPanel must be mounted inside a colony dockview with ColonyDockContext provided')
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
  />
</template>
