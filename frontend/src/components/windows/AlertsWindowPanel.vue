<script setup lang="ts">
/** Floating-window wrapper for `AlertsPanel` (colony details multi-window redesign). */
import { inject } from 'vue'
import AlertsPanel from '@/components/AlertsPanel.vue'
import { COLONY_WINDOW_CONTEXT_KEY } from '@/windows/colonyWindows'

const ctx = inject(COLONY_WINDOW_CONTEXT_KEY)
if (!ctx) throw new Error('AlertsWindowPanel must be mounted with ColonyWindowContext provided')
</script>

<template>
  <AlertsPanel
    :notifications="ctx.notifications"
    :event-log="ctx.eventLog"
    @clear-log="ctx.clearEventLog"
  />
</template>
