<script setup lang="ts">
/**
 * Active buildings panel.
 *
 * Status comes from the building's **actual production last turn** (`scale`,
 * with `shortfall_reason` explaining any shortfall) — issue #303. It used to be
 * derived from `labour_assigned === 0`, which reported *every* building as
 * "Idle" because per-building labour had no backing state. Buildings whose
 * recipes are all always-on carry `always_on` so the absent recipe picker can be
 * explained rather than looking broken.
 *
 * # Per-building staffing (issue #307)
 *
 * This panel briefly had a labour control that did nothing —
 * `Command::AssignLabour` validated its input, emitted an event, and persisted
 * nothing — so #346 removed it on the principle that no control beats one that
 * lies. #307 gave labour real backing state, and these controls drive it:
 * priority, a manual pin, and a rename so instances can be told apart.
 *
 * Rows are **per placed instance**, not per type: a colony with three mines has
 * three rows. They are therefore keyed by `building_id`, not `building_type` —
 * keying by type made Vue treat sibling instances as one element and reuse the
 * wrong DOM node between them.
 *
 * The panel raises **intents** rather than calling the engine itself. The parent
 * owns the bridge call and the refresh, which keeps this component a pure
 * function of its props and testable without stubbing the transport.
 */

import { ref } from 'vue'
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
  /**
   * Open the building's detail page. Carries the **type**, not the instance —
   * that page is still type-scoped (#339 revisits it).
   */
  (e: 'view-details', buildingType: string): void
  (e: 'set-priority', buildingId: string, priority: number): void
  /** `null` releases the building back to automatic assignment. */
  (e: 'set-lock', buildingId: string, lock: number | null): void
  /** `null` reverts to the auto-numbered default name. */
  (e: 'rename', buildingId: string, name: string | null): void
}>()

/** Priority bands offered, matching the engine's `1..=MAX_BUILDING_PRIORITY`. */
const PRIORITIES = [1, 2, 3, 4, 5, 6, 7, 8, 9]

/** Instance id currently being renamed, if any. */
const renamingId = ref<string | null>(null)
const renameDraft = ref('')

function startRename(b: BuildingRow): void {
  renamingId.value = b.building_id
  // Seed with the current display name so a small edit doesn't mean retyping.
  renameDraft.value = b.name
}

function cancelRename(): void {
  renamingId.value = null
  renameDraft.value = ''
}

function commitRename(b: BuildingRow): void {
  const trimmed = renameDraft.value.trim()
  // Empty means "revert to the auto-numbered default" rather than an error —
  // clearing the box is the natural way to ask for that. The engine rejects a
  // blank *name*, so send null explicitly instead of an empty string.
  emit('rename', b.building_id, trimmed === '' ? null : trimmed)
  cancelRename()
}

function onPriorityChange(b: BuildingRow, value: string): void {
  const next = Number(value)
  if (!Number.isInteger(next) || next === b.priority) return
  emit('set-priority', b.building_id, next)
}

/** Pin the building at whatever it currently wants — the useful default. */
function pin(b: BuildingRow): void {
  // Pin to demand where there is one, else to a single worker: a building that
  // couldn't run this sol reports demand 0, and pinning 0 would be a no-op the
  // player didn't ask for.
  emit('set-lock', b.building_id, Math.max(1, b.labour_demand))
}

function unpin(b: BuildingRow): void {
  emit('set-lock', b.building_id, null)
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

/**
 * `true` when the building wanted workers and didn't get all of them.
 *
 * Guarded on `labour_demand > 0`: a building with no jobs to offer is not
 * understaffed, and reporting it as such would flag every storage silo.
 */
function isUnderstaffed(b: BuildingRow): boolean {
  return b.labour_demand > 0 && b.labour_assigned < b.labour_demand
}

/** Staffing summary, e.g. `"3/5 staffed"`. */
function staffingLabel(b: BuildingRow): string {
  if (b.labour_demand === 0) {
    return b.labour_assigned > 0 ? `${b.labour_assigned} staffed` : 'no jobs'
  }
  return `${b.labour_assigned}/${b.labour_demand} staffed`
}

function staffingDetail(b: BuildingRow): string {
  if (b.labour_demand === 0) {
    return 'Offers no jobs this sol — either it has no recipe, or it could not run at all.'
  }
  if (isUnderstaffed(b)) {
    return `Understaffed: wanted ${b.labour_demand} workers, got ${b.labour_assigned}. Raise its priority to staff it ahead of others.`
  }
  return `Fully staffed with ${b.labour_assigned} worker${b.labour_assigned === 1 ? '' : 's'}.`
}

/**
 * The building's merged flows, at **full output** (issue #272).
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

/**
 * Label for a row, falling back to the type when `name` is absent.
 *
 * `name` is a newer field, so a host on an older build may omit it — showing
 * `undefined` in the list would be worse than showing the type key.
 */
function displayName(b: BuildingRow): string {
  return b.name ?? b.building_type
}
</script>

<template>
  <div class="panel" data-testid="buildings-panel">
    <h4 class="panel-title">Buildings</h4>

    <ul v-if="props.buildings !== null" class="building-list" data-testid="building-list">
      <li
        v-for="b in props.buildings"
        :key="b.building_id"
        class="building-item"
        :data-testid="`building-row-${b.building_type}`"
        :data-building-id="b.building_id"
      >
        <div class="row-main">
          <button
            class="building-name building-name-btn"
            :data-testid="`view-details-${b.building_type}`"
            @click="emit('view-details', b.building_type)"
          >
            {{ displayName(b) }}
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
        </div>

        <div class="row-staffing" :data-testid="`building-staffing-${b.building_id}`">
          <span
            class="staffing"
            :class="{ understaffed: isUnderstaffed(b) }"
            :data-testid="`building-staffed-${b.building_id}`"
            :title="staffingDetail(b)"
          >{{ staffingLabel(b) }}</span>
          <span
            v-if="isUnderstaffed(b)"
            class="building-badge badge-warn"
            :data-testid="`building-understaffed-${b.building_id}`"
            :title="staffingDetail(b)"
          >understaffed</span>

          <label class="priority-label">
            <span class="priority-caption">Priority</span>
            <select
              class="priority-select"
              :value="b.priority"
              :data-testid="`building-priority-${b.building_id}`"
              title="1 is staffed first, 9 last."
              @change="onPriorityChange(b, ($event.target as HTMLSelectElement).value)"
            >
              <option v-for="p in PRIORITIES" :key="p" :value="p">{{ p }}</option>
            </select>
          </label>

          <button
            v-if="b.labour_lock === null"
            class="btn-small"
            :data-testid="`building-pin-${b.building_id}`"
            title="Pin workers here so automatic assignment can't reclaim them."
            @click="pin(b)"
          >Pin</button>
          <template v-else>
            <span
              class="building-badge badge-lock"
              :data-testid="`building-locked-${b.building_id}`"
              :title="`${b.labour_lock} worker(s) pinned here — automatic assignment will not reclaim them.`"
            >pinned {{ b.labour_lock }}</span>
            <button
              class="btn-small"
              :data-testid="`building-unpin-${b.building_id}`"
              title="Return this building to automatic assignment."
              @click="unpin(b)"
            >Unpin</button>
          </template>

          <template v-if="renamingId === b.building_id">
            <input
              v-model="renameDraft"
              class="rename-input"
              :data-testid="`building-rename-input-${b.building_id}`"
              placeholder="Leave blank to reset"
              @keyup.enter="commitRename(b)"
              @keyup.esc="cancelRename()"
            />
            <button
              class="btn-small"
              :data-testid="`building-rename-save-${b.building_id}`"
              @click="commitRename(b)"
            >Save</button>
            <button
              class="btn-small"
              :data-testid="`building-rename-cancel-${b.building_id}`"
              @click="cancelRename()"
            >Cancel</button>
          </template>
          <button
            v-else
            class="btn-small"
            :data-testid="`building-rename-${b.building_id}`"
            title="Give this building a name so you can tell it from its siblings."
            @click="startRename(b)"
          >Rename</button>
        </div>

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
.badge-warn { border-color: #a75; color: #eab764; }
.badge-lock { border-color: #578; color: #8bd; }
.panel-title { color: #8cf; font-size: 0.9rem; margin: 0 0 0.6rem; }
.hint { font-size: 0.75rem; color: #446; font-style: italic; }

.building-list { list-style: none; display: flex; flex-direction: column; gap: 0.35rem; margin: 0; padding: 0; }
.building-item {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  background: #13131e;
  border: 1px solid #223;
  border-radius: 3px;
  padding: 0.3rem 0.5rem;
  font-size: 0.8rem;
  color: #aab;
}
.row-main { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; }
.row-staffing { display: flex; align-items: center; gap: 0.4rem; flex-wrap: wrap; }
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

.staffing { font-size: 0.72rem; color: #789; font-family: monospace; }
.staffing.understaffed { color: #eab764; }

.priority-label { display: inline-flex; align-items: center; gap: 0.25rem; }
.priority-caption { font-size: 0.68rem; color: #668; text-transform: uppercase; letter-spacing: 0.04em; }
.priority-select {
  background: #0e0e16;
  border: 1px solid #334;
  color: #aab;
  font-size: 0.72rem;
  font-family: inherit;
  padding: 0.05rem 0.2rem;
  border-radius: 2px;
}

.btn-small {
  background: #1a1a28;
  border: 1px solid #334;
  border-radius: 2px;
  color: #9ab;
  font-size: 0.68rem;
  font-family: inherit;
  padding: 0.08rem 0.35rem;
  cursor: pointer;
}
.btn-small:hover { background: #23233a; color: #cdd; }

.rename-input {
  background: #0e0e16;
  border: 1px solid #445;
  border-radius: 2px;
  color: #cdd;
  font-size: 0.72rem;
  font-family: inherit;
  padding: 0.08rem 0.3rem;
  min-width: 9rem;
}

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
