<script setup lang="ts">
/**
 * Active buildings + labour assignment panel.
 *
 * Status comes from the building's **actual production last turn** (`scale`,
 * with `shortfall_reason` explaining any shortfall) — issue #303. It used to
 * be derived from `labour_assigned === 0`, but per-building labour assignment
 * has no backing state (it's always 0; production is gated by a colony-wide
 * labour ratio), so *every* building reported as "Idle" regardless of what it
 * was doing. Buildings whose recipes are all always-on carry `always_on` so
 * the absent recipe picker can be explained rather than looking broken.
 */

import { ref } from 'vue'
import type { BuildingRow } from '@/types/screen'

const props = defineProps<{
  /** `null` when the colony screen hasn't loaded yet for the selected colony. */
  buildings: BuildingRow[] | null
  slotsUsed: number
  slotCapacity: number
  labourAvailable: number
  labourTotal: number
}>()

const emit = defineEmits<{
  (e: 'assign-labour', buildingType: string, labour: number): void
  (e: 'view-details', buildingType: string): void
}>()

/** Temporary labour values being edited (keyed by building_type). */
const labourDraft = ref<Record<string, number>>({})

function labourForBuilding(buildingType: string, current: number): number {
  return labourDraft.value[buildingType] ?? current
}

function buildingStatus(b: BuildingRow): 'idle' | 'running' | 'partial' {
  if (b.full_capacity) return 'running'
  // Anything above zero produced *something* last turn — only a genuine zero
  // is idle.
  return b.scale > 0 ? 'partial' : 'idle'
}

function statusLabel(status: 'idle' | 'running' | 'partial'): string {
  return { idle: 'Idle', running: 'Running', partial: 'Partial' }[status]
}

/**
 * Tooltip/aside text explaining a non-running status — the engine's shortfall
 * reason when it has one, so "Partial" and "Idle" say *why*.
 */
function statusDetail(b: BuildingRow): string {
  if (b.full_capacity) return 'Running at full output'
  if (b.shortfall_reason) {
    return `${(b.scale * 100).toFixed(0)}% output — ${b.shortfall_reason}`
  }
  return b.scale > 0 ? `${(b.scale * 100).toFixed(0)}% output` : 'Produced nothing last turn'
}

function assignLabour(buildingType: string): void {
  const labour = labourDraft.value[buildingType]
  if (labour === undefined) return
  emit('assign-labour', buildingType, labour)
  delete labourDraft.value[buildingType]
}
</script>

<template>
  <div class="panel" data-testid="buildings-panel">
    <h4 class="panel-title">Buildings &amp; Labour</h4>

    <ul v-if="props.buildings !== null" class="building-list" data-testid="building-list">
      <li
        v-for="b in props.buildings"
        :key="b.building_type"
        class="building-item"
        :data-testid="`building-row-${b.building_type}`"
      >
        <button
          class="building-name building-name-btn"
          :data-testid="`view-details-${b.building_type}`"
          @click="emit('view-details', b.building_type)"
        >
          {{ b.building_type }}
        </button>
        <span
          class="building-status"
          :class="`status-${buildingStatus(b)}`"
          :data-testid="`building-status-${b.building_type}`"
          :title="statusDetail(b)"
        >
          {{ statusLabel(buildingStatus(b)) }}
        </span>
        <span
          v-if="b.shortfall_reason && !b.full_capacity"
          class="building-reason"
          :data-testid="`building-reason-${b.building_type}`"
        >{{ b.shortfall_reason }}</span>
        <span
          v-if="b.always_on"
          class="building-badge"
          :data-testid="`building-always-on-${b.building_type}`"
          title="This facility runs always-on recipes — there is no recipe to choose."
        >always-on</span>
        <span class="building-meta">{{ b.slot_cost }} slot{{ b.slot_cost !== 1 ? 's' : '' }}</span>
        <div class="labour-controls">
          <input
            class="labour-input"
            type="number"
            min="0"
            :value="labourForBuilding(b.building_type, b.labour_assigned)"
            :data-testid="`labour-input-${b.building_type}`"
            @input="labourDraft[b.building_type] = +($event.target as HTMLInputElement).value"
          />
          <button
            class="btn-assign"
            :disabled="labourDraft[b.building_type] === undefined"
            :data-testid="`assign-labour-${b.building_type}`"
            @click="assignLabour(b.building_type)"
          >
            Assign
          </button>
        </div>
      </li>
      <li v-if="props.buildings.length === 0" class="empty-row">No active buildings.</li>
    </ul>
    <div v-else class="hint">No building data loaded.</div>

    <div v-if="props.buildings !== null" class="slots-summary" data-testid="slots-summary">
      Build slots: {{ props.slotsUsed }} / {{ props.slotCapacity }}
      &nbsp;|&nbsp;
      Labour: {{ props.labourAvailable }} / {{ props.labourTotal }} free
    </div>
  </div>
</template>

<style scoped>
.panel { padding: 0.75rem; height: 100%; overflow-y: auto; box-sizing: border-box; }
.panel-title { color: #8cf; font-size: 0.9rem; margin: 0 0 0.6rem; }
.hint { font-size: 0.75rem; color: #446; font-style: italic; }

.building-list { list-style: none; display: flex; flex-direction: column; gap: 0.35rem; margin: 0; padding: 0; }
.building-item {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  background: #13131e;
  border: 1px solid #223;
  border-radius: 3px;
  padding: 0.3rem 0.5rem;
  font-size: 0.8rem;
  color: #aab;
  flex-wrap: wrap;
}
.building-name { flex: 1 0 100px; }
.building-name-btn {
  background: none;
  border: none;
  color: #aab;
  font-size: inherit;
  font-family: inherit;
  text-align: left;
  cursor: pointer;
  padding: 0;
  text-decoration: underline dotted;
}
.building-name-btn:hover { color: #cdd; }
.building-meta { color: #668; font-size: 0.72rem; }
.building-status { font-size: 0.7rem; font-weight: 600; letter-spacing: 0.03em; text-transform: uppercase; }
.status-running { color: #6adba5; }
.status-partial { color: #eab764; }
.status-idle    { color: #778; }

.building-reason { color: #a86; font-size: 0.7rem; font-style: italic; }
.building-badge {
  border: 1px solid #475;
  border-radius: 2px;
  color: #8a8;
  font-size: 0.64rem;
  padding: 0.02rem 0.28rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.labour-controls { display: flex; align-items: center; gap: 0.3rem; }
.labour-input {
  width: 54px;
  background: #0d0d15;
  border: 1px solid #334;
  border-radius: 3px;
  color: #cdd;
  padding: 0.2rem 0.35rem;
  font-family: monospace;
  font-size: 0.8rem;
}
.btn-assign {
  background: #1a2030;
  border: 1px solid #336;
  border-radius: 3px;
  color: #89b;
  padding: 0.15rem 0.4rem;
  font-size: 0.75rem;
  cursor: pointer;
}
.btn-assign:disabled { opacity: 0.4; cursor: not-allowed; }
.btn-assign:hover:not(:disabled) { background: #1e2840; }

.empty-row { color: #445; font-style: italic; }

.slots-summary { font-size: 0.72rem; color: #558; margin-top: 0.35rem; }
</style>
