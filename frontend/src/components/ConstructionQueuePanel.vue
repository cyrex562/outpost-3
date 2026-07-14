<script setup lang="ts">
/**
 * Construction queue + build catalog panel.
 *
 * Cancel wires through `Command::CancelConstruction` (issue #169) — the
 * caller owns the async send and passes back `cancelingIds` so this panel
 * can disable the right button while a cancel is in flight without
 * duplicating request-tracking state.
 */

import type { ConstructionQueueRow } from '@/types/screen'
import type { BuildingOption } from '@/services/tauriBridge'

const props = defineProps<{
  /** `null` when the colony screen hasn't loaded yet for the selected colony. */
  queue: ConstructionQueueRow[] | null
  catalog: BuildingOption[]
  disabledReason: (b: BuildingOption) => string | null
  slotsAvailable: number | null
  queueBusy: boolean
  cancelingIds: Set<string>
}>()

const emit = defineEmits<{
  (e: 'queue', building: BuildingOption): void
  (e: 'cancel', projectId: string): void
}>()
</script>

<template>
  <div class="panel" data-testid="construction-queue-panel">
    <h4 class="panel-title">Construction Queue</h4>

    <div v-if="props.queue !== null && props.queue.length > 0">
      <ul class="queue-list" data-testid="construction-queue-list">
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
    </div>
    <div v-else class="hint">No projects in queue.</div>

    <div class="build-catalog-wrap" data-testid="build-catalog">
      <div class="catalog-heading">
        <h5 class="catalog-title">Build</h5>
        <span v-if="props.slotsAvailable !== null" class="catalog-slots" data-testid="catalog-slots">
          {{ props.slotsAvailable }} slot{{ props.slotsAvailable === 1 ? '' : 's' }} free
        </span>
      </div>
      <div v-if="props.catalog.length === 0" class="hint">No buildings available in the loaded content pack.</div>
      <div v-else class="build-catalog">
        <div
          v-for="b in props.catalog"
          :key="b.id"
          class="build-card"
          :class="{ 'is-disabled': props.disabledReason(b) !== null }"
          :title="props.disabledReason(b) ?? ''"
          :data-testid="`build-card-${b.id}`"
        >
          <div class="build-card-head">
            <span class="build-card-name">{{ b.name }}</span>
            <span class="build-card-cat">{{ b.category }}</span>
          </div>
          <p v-if="b.description" class="build-card-desc">{{ b.description }}</p>
          <div class="build-card-stats">
            {{ b.construction_turns }} sols · {{ b.labor_per_turn }} labor/turn ·
            {{ b.slot_cost }} slot{{ b.slot_cost === 1 ? '' : 's' }}
          </div>
          <div v-if="b.construction_cost.length" class="build-card-cost">
            <span v-for="(c, i) in b.construction_cost" :key="i" class="cost-chip">{{ c[1] }} {{ c[0] }}</span>
          </div>
          <div class="build-card-foot">
            <span v-if="props.disabledReason(b)" class="build-card-reason" :data-testid="`build-card-reason-${b.id}`">
              {{ props.disabledReason(b) }}
            </span>
            <button
              class="btn-queue"
              :disabled="props.queueBusy || props.disabledReason(b) !== null"
              :data-testid="`btn-queue-${b.id}`"
              @click="emit('queue', b)"
            >
              Queue
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.panel { padding: 0.75rem; height: 100%; overflow-y: auto; box-sizing: border-box; }
.panel-title { color: #8cf; font-size: 0.9rem; margin: 0 0 0.6rem; }
.hint { font-size: 0.75rem; color: #446; font-style: italic; }

.queue-list { list-style: none; display: flex; flex-direction: column; gap: 0.3rem; margin: 0 0 0.5rem; padding: 0; }
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

.build-catalog-wrap { margin-top: 0.75rem; }
.catalog-heading { display: flex; justify-content: space-between; align-items: baseline; margin-bottom: 0.4rem; }
.catalog-title { color: #668; font-size: 0.78rem; letter-spacing: 0.06em; text-transform: uppercase; }
.catalog-slots { color: #778; font-size: 0.72rem; }

.build-catalog { display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 0.5rem; }
.build-card {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  background: #14141e;
  border: 1px solid #334;
  border-radius: 4px;
  padding: 0.55rem 0.6rem;
  color: #aab;
}
.build-card.is-disabled { opacity: 0.55; }
.build-card-head { display: flex; justify-content: space-between; align-items: baseline; gap: 0.5rem; }
.build-card-name { color: #8cf; font-size: 0.86rem; font-weight: 600; }
.build-card-cat { color: #557; font-size: 0.68rem; text-transform: uppercase; letter-spacing: 0.05em; }
.build-card-desc { color: #889; font-size: 0.76rem; }
.build-card-stats { color: #668; font-size: 0.72rem; }
.build-card-cost { display: flex; gap: 0.25rem; flex-wrap: wrap; }
.cost-chip {
  background: #1a1a2a;
  border: 1px solid #223;
  border-radius: 2px;
  padding: 0.05rem 0.3rem;
  color: #8a8;
  font-size: 0.7rem;
}
.build-card-foot { display: flex; justify-content: space-between; align-items: center; gap: 0.5rem; margin-top: 0.25rem; }
.build-card-reason { color: #a86; font-size: 0.7rem; font-style: italic; }
.btn-queue {
  background: #1a2030;
  border: 1px solid #468;
  border-radius: 3px;
  color: #8cf;
  padding: 0.3rem 0.7rem;
  font-family: monospace;
  font-size: 0.78rem;
  cursor: pointer;
  margin-left: auto;
}
.btn-queue:disabled { opacity: 0.45; cursor: not-allowed; }
.btn-queue:hover:not(:disabled) { background: #22293a; }
</style>
