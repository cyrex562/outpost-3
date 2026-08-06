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
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 3px;
  color: var(--text-muted);
  padding: 0.3rem 0.6rem;
  font-family: monospace;
  font-size: 0.72rem;
  cursor: pointer;
  white-space: nowrap;
}
.palette-chip:hover { background: var(--surface-alt); border-color: var(--text-faint); color: var(--text); }
.palette-chip.open { border-color: var(--border-accent); background: var(--accent-bg); color: var(--accent); }
.palette-chip.open:hover { background: var(--accent-bg); }
</style>
