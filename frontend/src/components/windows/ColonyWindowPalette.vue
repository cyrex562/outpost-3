<script setup lang="ts">
/**
 * Tool palette (colony details multi-window redesign) — one toggle chip per
 * panel window, so closing a window from its own `×` isn't a dead end: the
 * chip for that window stays visible here and reopens it. A chip for an
 * already-open window closes it too, mirroring its own close button rather
 * than being a one-way "open" affordance.
 */
import { COLONY_WINDOW_IDS, COLONY_WINDOW_TITLES, type ColonyWindowId } from '@/windows/colonyWindows'

defineProps<{ openIds: Set<ColonyWindowId> }>()

const emit = defineEmits<{
  (e: 'toggle', id: ColonyWindowId): void
}>()
</script>

<template>
  <div class="window-palette" data-testid="window-palette">
    <button
      v-for="id in COLONY_WINDOW_IDS"
      :key="id"
      type="button"
      class="palette-chip"
      :class="{ open: openIds.has(id) }"
      :data-testid="`palette-toggle-${id}`"
      :aria-pressed="openIds.has(id)"
      :title="openIds.has(id) ? `Close ${COLONY_WINDOW_TITLES[id]}` : `Open ${COLONY_WINDOW_TITLES[id]}`"
      @click="emit('toggle', id)"
    >
      {{ COLONY_WINDOW_TITLES[id] }}
    </button>
  </div>
</template>

<style scoped>
.window-palette {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.35rem;
}

.palette-chip {
  background: #14141e;
  border: 1px solid #334;
  border-radius: 3px;
  color: #778;
  padding: 0.3rem 0.6rem;
  font-family: monospace;
  font-size: 0.72rem;
  cursor: pointer;
  white-space: nowrap;
}
.palette-chip:hover { background: #1a1a2a; border-color: #558; color: #aab; }
.palette-chip.open { border-color: #468; background: #182030; color: #8cf; }
.palette-chip.open:hover { background: #1c2c40; }
</style>
