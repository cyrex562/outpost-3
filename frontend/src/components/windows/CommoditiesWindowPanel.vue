<script setup lang="ts">
/** Floating-window wrapper for `CommoditiesPanel` (colony details multi-window redesign). */
import { inject } from 'vue'
import CommoditiesPanel from '@/components/CommoditiesPanel.vue'
import { COLONY_WINDOW_CONTEXT_KEY } from '@/windows/colonyWindows'

const ctx = inject(COLONY_WINDOW_CONTEXT_KEY)
if (!ctx) throw new Error('CommoditiesWindowPanel must be mounted with ColonyWindowContext provided')
</script>

<template>
  <CommoditiesPanel
    :stockpile="ctx.stockpile"
    @set-reserve="ctx.setCommodityReserve"
  />
</template>
