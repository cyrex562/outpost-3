<script setup lang="ts">
/** `dockview-vue` panel wrapper for `AlertsPanel` (issue #321). */
import { inject } from 'vue'
import AlertsPanel from '@/components/AlertsPanel.vue'
import { COLONY_DOCK_CONTEXT_KEY } from '@/dock/colonyDock'

defineProps<{ params?: unknown }>()

const ctx = inject(COLONY_DOCK_CONTEXT_KEY)
if (!ctx) throw new Error('DockAlertsPanel must be mounted inside a colony dockview with ColonyDockContext provided')
</script>

<template>
  <AlertsPanel
    :notifications="ctx.notifications"
    :event-log="ctx.eventLog"
    @clear-log="ctx.clearEventLog"
  />
</template>
