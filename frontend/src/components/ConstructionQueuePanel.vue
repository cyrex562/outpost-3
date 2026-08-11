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
 *
 * Empty-queue collapse (issue #339): most of the early game has nothing
 * under construction, so a full-height panel here just pushes the building
 * list into a short scroll area. When the queue is empty this renders a
 * single compact row instead of the padded heading + hint block, so the
 * host layout (ColonyView's resizable split) can give the building list the
 * space by default. The host still owns and persists the actual split size —
 * this component only signals emptiness via `data-collapsed` for the host
 * (and tests) to key off of.
 */

import { computed } from 'vue'
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

/** Nothing in progress — drives the compact collapsed presentation below. */
const isEmpty = computed(() => props.queue === null || props.queue.length === 0)
</script>

<template>
  <div
    class="panel"
    :class="{ 'panel--collapsed': isEmpty }"
    data-testid="construction-queue-panel"
    :data-collapsed="isEmpty"
  >
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
        <!-- A repair shares this queue (issue #451) but is not a new
             building; saying so avoids reading the row as a duplicate. -->
        <span class="building-name">
          <span v-if="proj.is_repair" class="queue-kind" data-testid="queue-repair-tag">repair:</span>
          {{ proj.building_type }}
        </span>
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
    <div v-else class="hint" data-testid="construction-queue-empty-hint">
      Nothing under construction. Use “Build…” to queue a project.
    </div>
  </div>
</template>

<style scoped>
.panel { padding: 0.75rem; height: 100%; overflow-y: auto; box-sizing: border-box; }
/* Compact state (issue #339): a single tight row rather than a padded block,
   so the host pane reads as "collapsed" even when the split gives it more
   room than the content needs. */
.panel--collapsed {
  padding: 0.4rem 0.75rem;
  display: flex;
  align-items: center;
  overflow: hidden;
}
.panel--collapsed .queue-heading { margin-bottom: 0; flex: 1; }
.panel--collapsed .hint { margin: 0 0 0 0.6rem; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.queue-heading { display: flex; align-items: center; justify-content: space-between; margin-bottom: 0.6rem; }
.panel-title { color: var(--accent); font-size: 0.9rem; margin: 0; }
.hint { font-size: 0.75rem; color: var(--border-strong); font-style: italic; }

.btn-build {
  background: var(--accent-bg);
  border: 1px solid var(--border-accent);
  border-radius: 3px;
  color: var(--accent);
  padding: 0.25rem 0.7rem;
  font-family: monospace;
  font-size: 0.78rem;
  cursor: pointer;
}
.btn-build:hover { background: var(--accent-bg); }

.queue-list { list-style: none; display: flex; flex-direction: column; gap: 0.3rem; margin: 0; padding: 0; }
.queue-item {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  background: var(--surface-1);
  border: 1px solid var(--border-subtle);
  border-radius: 3px;
  padding: 0.25rem 0.5rem;
  font-size: 0.8rem;
  color: var(--text);
}
.building-name { flex: 1 0 100px; }
.queue-kind { color: var(--accent); margin-right: 0.25rem; }
.queue-progress { font-size: 0.72rem; color: var(--text-dim); }
.queue-meta { color: var(--text-dim); font-size: 0.72rem; }
.btn-cancel {
  background: var(--danger-bg);
  border: 1px solid var(--danger-border);
  border-radius: 3px;
  color: var(--danger-dim);
  padding: 0.15rem 0.4rem;
  font-size: 0.72rem;
  cursor: pointer;
  margin-left: auto;
}
.btn-cancel:disabled { opacity: 0.5; cursor: not-allowed; }
.btn-cancel:hover:not(:disabled) { background: var(--danger-bg); }
</style>
