<script setup lang="ts">
/**
 * Construction queue panel (UI-rework PR5) — shows only what is *currently
 * under construction* (in-progress projects with their turn progress), plus a
 * "Build…" button that opens the build dialog. The build catalog itself moved
 * out to BuildDialog.vue so this panel is a focused queue view, not a mix of
 * "queued" and "available".
 *
 * Cancel wires through `Command::CancelConstruction` (issue #169) — the
 * caller owns the async send and passes back `cancelingIds` so this panel can
 * disable the right button while a cancel is in flight without duplicating
 * request-tracking state.
 */

import type { ConstructionQueueRow } from '@/types/screen'

const props = defineProps<{
  /** `null` when the colony screen hasn't loaded yet for the selected colony. */
  queue: ConstructionQueueRow[] | null
  cancelingIds: Set<string>
}>()

const emit = defineEmits<{
  (e: 'cancel', projectId: string): void
  (e: 'open-build'): void
}>()
</script>

<template>
  <div class="panel" data-testid="construction-queue-panel">
    <div class="queue-heading">
      <h4 class="panel-title">Under Construction</h4>
      <button class="btn-build" data-testid="btn-open-build" @click="emit('open-build')">
        Build…
      </button>
    </div>

    <ul v-if="props.queue !== null && props.queue.length > 0" class="queue-list" data-testid="construction-queue-list">
      <li
        v-for="proj in props.queue"
        :key="proj.project_id"
        class="queue-item"
        :data-testid="`queue-item-${proj.project_id}`"
      >
        <span class="building-name">{{ proj.building_type }}</span>
        <span class="queue-progress">{{ proj.turns_completed }}/{{ proj.turns_total }} turns</span>
        <span class="queue-meta">{{ proj.slot_cost }} slot{{ proj.slot_cost !== 1 ? 's' : '' }}</span>
        <button
          class="btn-cancel"
          :disabled="props.cancelingIds.has(proj.project_id)"
          :data-testid="`btn-cancel-${proj.project_id}`"
          @click="emit('cancel', proj.project_id)"
        >
          {{ props.cancelingIds.has(proj.project_id) ? 'Cancelling…' : 'Cancel' }}
        </button>
      </li>
    </ul>
    <div v-else class="hint">Nothing under construction. Use “Build…” to queue a project.</div>
  </div>
</template>

<style scoped>
.panel { padding: 0.75rem; height: 100%; overflow-y: auto; box-sizing: border-box; }
.queue-heading { display: flex; align-items: center; justify-content: space-between; margin-bottom: 0.6rem; }
.panel-title { color: #8cf; font-size: 0.9rem; margin: 0; }
.hint { font-size: 0.75rem; color: #446; font-style: italic; }

.btn-build {
  background: #1a2030;
  border: 1px solid #468;
  border-radius: 3px;
  color: #8cf;
  padding: 0.25rem 0.7rem;
  font-family: monospace;
  font-size: 0.78rem;
  cursor: pointer;
}
.btn-build:hover { background: #22293a; }

.queue-list { list-style: none; display: flex; flex-direction: column; gap: 0.3rem; margin: 0; padding: 0; }
.queue-item {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  background: #111120;
  border: 1px solid #223;
  border-radius: 3px;
  padding: 0.25rem 0.5rem;
  font-size: 0.8rem;
  color: #aab;
}
.building-name { flex: 1 0 100px; }
.queue-progress { font-size: 0.72rem; color: #668; }
.queue-meta { color: #668; font-size: 0.72rem; }
.btn-cancel {
  background: #241417;
  border: 1px solid #632;
  border-radius: 3px;
  color: #d88;
  padding: 0.15rem 0.4rem;
  font-size: 0.72rem;
  cursor: pointer;
  margin-left: auto;
}
.btn-cancel:disabled { opacity: 0.5; cursor: not-allowed; }
.btn-cancel:hover:not(:disabled) { background: #2e1a1e; }
</style>
