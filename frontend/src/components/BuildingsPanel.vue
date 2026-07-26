<script setup lang="ts">
/**
 * Active buildings panel.
 *
 * Status comes from the building's **actual production last turn** (`scale`,
 * with `shortfall_reason` explaining any shortfall) — issue #303. It used to
 * be derived from `labour_assigned === 0`, but per-building labour assignment
 * has no backing state (it's always 0; production is gated by a colony-wide
 * labour ratio), so *every* building reported as "Idle" regardless of what it
 * was doing. Buildings whose recipes are all always-on carry `always_on` so
 * the absent recipe picker can be explained rather than looking broken.
 *
 * **No per-building labour control here (issue #307).** This panel used to
 * offer a number input and an Assign button per building. They did nothing:
 * `Command::AssignLabour` validates its argument, emits an event, and persists
 * no state, so the input's value was discarded and the displayed figure was
 * always the hardcoded `0`. Labour is already allocated automatically, as a
 * colony-wide ratio. A real per-building override belongs in the building
 * details page once #307 gives labour actual backing state; until then no
 * control is better than one that lies.
 */

import type { BuildingRow, IngredientRow } from '@/types/screen'

const props = defineProps<{
  /** `null` when the colony screen hasn't loaded yet for the selected colony. */
  buildings: BuildingRow[] | null
  slotsUsed: number
  slotCapacity: number
  labourAvailable: number
  labourTotal: number
}>()

const emit = defineEmits<{
  (e: 'view-details', buildingType: string): void
}>()

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

/**
 * The building's merged flows, at **full output** (issue #272).
 *
 * The row used to show only the pick-one recipe, so a multi-function building
 * like `colony_hq` — whose recipes are *all* always-on — read as having no
 * function at all. This is the merged nominal I/O of every recipe it runs.
 *
 * These are *rated* figures, not last turn's actual throughput (that is
 * `nominal × scale`). The line is labelled accordingly, because showing an
 * unscaled `24 water` next to a "water short" badge would otherwise look like a
 * contradiction.
 */
function outputSummary(b: BuildingRow): string {
  return joinFlows(b.outputs)
}

/** Same, for what the building consumes. */
function inputSummary(b: BuildingRow): string {
  return joinFlows(b.inputs)
}

/**
 * `?? []` guards: these fields are `#[serde(default)]` on the Rust side, so a
 * host on an older build can legitimately omit them from a payload a newer
 * frontend bundle receives. Dereferencing `.length` on the absent value would
 * take the whole panel down.
 */
function joinFlows(flows: IngredientRow[] | undefined): string {
  return (flows ?? []).map((f) => `${formatQty(f.quantity)} ${f.commodity_id}`).join(', ')
}

/** How many flow lines a row has, absent-safe. */
function flowCount(b: BuildingRow): number {
  return (b.inputs ?? []).length + (b.outputs ?? []).length
}

/** Recipes running in this building, absent-safe. */
function runningRecipes(b: BuildingRow): string[] {
  return b.running_recipe_ids ?? []
}

/** Trim trailing zeroes so "24" doesn't render as "24.0". */
function formatQty(q: number): string {
  return Number.isInteger(q) ? String(q) : q.toFixed(1)
}

/** Tooltip listing every recipe running in this building. */
function recipeTooltip(b: BuildingRow): string {
  const ids = runningRecipes(b)
  if (ids.length === 0) return 'No recipes — this is a storage or habitat structure.'
  const rated = 'Figures are rated output at full capacity, not last turn\'s actual.'
  if (ids.length === 1) return `Running: ${ids[0]}. ${rated}`
  return `Running ${ids.length} recipes at once: ${ids.join(', ')}. ${rated}`
}

</script>

<template>
  <div class="panel" data-testid="buildings-panel">
    <h4 class="panel-title">Buildings</h4>

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
        <span
          v-if="runningRecipes(b).length > 1"
          class="building-badge badge-multi"
          :data-testid="`building-recipe-count-${b.building_type}`"
          :title="recipeTooltip(b)"
        >{{ runningRecipes(b).length }} recipes</span>
        <div
          v-if="flowCount(b) > 0"
          class="building-io"
          :data-testid="`building-io-${b.building_type}`"
          :title="recipeTooltip(b)"
        >
          <span
            v-if="(b.inputs ?? []).length > 0"
            class="io-in"
            :data-testid="`building-inputs-${b.building_type}`"
          >← {{ inputSummary(b) }}</span>
          <span
            v-if="(b.outputs ?? []).length > 0"
            class="io-out"
            :data-testid="`building-outputs-${b.building_type}`"
          >→ {{ outputSummary(b) }}</span>
          <span class="io-rated" :data-testid="`building-io-rated-${b.building_type}`">rated</span>
        </div>
      </li>
      <li v-if="props.buildings.length === 0" class="empty-row">No active buildings.</li>
    </ul>
    <div v-else class="hint">No building data loaded.</div>

    <div v-if="props.buildings !== null" class="slots-summary" data-testid="slots-summary">
      Build slots: {{ props.slotsUsed }} / {{ props.slotCapacity }}
      &nbsp;|&nbsp;
      Labour: {{ props.labourAvailable.toFixed(0) }} of {{ props.labourTotal.toFixed(0) }} able to work
    </div>
  </div>
</template>

<style scoped>
.panel { padding: 0.75rem; height: 100%; overflow-y: auto; box-sizing: border-box; }
.building-io {
  flex-basis: 100%;
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
  font-size: 0.7rem;
  font-family: monospace;
  margin-top: 0.15rem;
}
.io-in { color: #c98; }
.io-out { color: #8c9; }
.io-rated { color: #667; font-style: italic; }
.badge-multi { border-color: #685; color: #ac9; }
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

.empty-row { color: #445; font-style: italic; }

.slots-summary { font-size: 0.72rem; color: #558; margin-top: 0.35rem; }
</style>
