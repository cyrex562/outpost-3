<script setup lang="ts">
/** `dockview-vue` panel wrapper for `UtilitiesPanel` (issue #321). */
import { inject } from 'vue'
import UtilitiesPanel from '@/components/UtilitiesPanel.vue'
import { COLONY_DOCK_CONTEXT_KEY } from '@/dock/colonyDock'

defineProps<{ params?: unknown }>()

const ctx = inject(COLONY_DOCK_CONTEXT_KEY)
if (!ctx) throw new Error('DockUtilitiesPanel must be mounted inside a colony dockview with ColonyDockContext provided')
</script>

<template>
  <UtilitiesPanel :resources="ctx.resources" />
</template>
