<script setup lang="ts">
/** Floating-window wrapper for `UtilitiesPanel` (colony details multi-window redesign). */
import { inject } from 'vue'
import UtilitiesPanel from '@/components/UtilitiesPanel.vue'
import { COLONY_WINDOW_CONTEXT_KEY } from '@/windows/colonyWindows'

const ctx = inject(COLONY_WINDOW_CONTEXT_KEY)
if (!ctx) throw new Error('UtilitiesWindowPanel must be mounted with ColonyWindowContext provided')
</script>

<template>
  <UtilitiesPanel :resources="ctx.resources" />
</template>
