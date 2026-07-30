<script setup lang="ts">
/** `dockview-vue` panel wrapper for `ConstructionQueuePanel` (issue #321). */
import { inject } from 'vue'
import ConstructionQueuePanel from '@/components/ConstructionQueuePanel.vue'
import { COLONY_DOCK_CONTEXT_KEY } from '@/dock/colonyDock'

defineProps<{ params?: unknown }>()

const ctx = inject(COLONY_DOCK_CONTEXT_KEY)
if (!ctx) throw new Error('DockConstructionQueuePanel must be mounted inside a colony dockview with ColonyDockContext provided')
</script>

<template>
  <ConstructionQueuePanel
    :queue="ctx.constructionQueue"
    :canceling-ids="ctx.cancelingIds"
    @open-build="ctx.openBuildDialog"
    @cancel="ctx.cancelConstruction"
  />
</template>
