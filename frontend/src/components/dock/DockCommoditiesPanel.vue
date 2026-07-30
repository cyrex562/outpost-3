<script setup lang="ts">
/** `dockview-vue` panel wrapper for `CommoditiesPanel` (issue #321). */
import { inject } from 'vue'
import CommoditiesPanel from '@/components/CommoditiesPanel.vue'
import { COLONY_DOCK_CONTEXT_KEY } from '@/dock/colonyDock'

defineProps<{ params?: unknown }>()

const ctx = inject(COLONY_DOCK_CONTEXT_KEY)
if (!ctx) throw new Error('DockCommoditiesPanel must be mounted inside a colony dockview with ColonyDockContext provided')
</script>

<template>
  <CommoditiesPanel
    :stockpile="ctx.stockpile"
    @set-reserve="ctx.setCommodityReserve"
  />
</template>
