<script setup lang="ts">
/** Floating-window wrapper for `ConstructionQueuePanel` (colony details multi-window redesign). */
import { inject } from 'vue'
import ConstructionQueuePanel from '@/components/ConstructionQueuePanel.vue'
import { COLONY_WINDOW_CONTEXT_KEY } from '@/windows/colonyWindows'

const ctx = inject(COLONY_WINDOW_CONTEXT_KEY)
if (!ctx) throw new Error('ConstructionQueueWindowPanel must be mounted with ColonyWindowContext provided')
</script>

<template>
  <ConstructionQueuePanel
    :queue="ctx.constructionQueue"
    :canceling-ids="ctx.cancelingIds"
    @open-build="ctx.openBuildDialog"
    @cancel="ctx.cancelConstruction"
  />
</template>
