<script setup lang="ts">
/**
 * Build dialog (UI-rework PR5) — a modal picker for queueing construction.
 * Lists the buildable catalog (with tech/slot gating reasons) and lets the
 * player choose a quantity per building before queueing, then emits `queue`
 * with the building and the chosen count. Opened from the construction-queue
 * panel's "Build…" button; the catalog used to live inline in that panel.
 */

import { computed, reactive, ref, watch } from 'vue'
import type { BuildingOption } from '@/services/tauriBridge'

const props = defineProps<{
  catalog: BuildingOption[]
  disabledReason: (b: BuildingOption) => string | null
  slotsAvailable: number | null
  busy: boolean
  /**
   * Why the catalog failed to load, if it did. Distinguishes "the pack has no
   * buildings" from "we could not ask" — those rendered identically before,
   * which is how an empty build dialog caused by a missing content registry
   * went unexplained through a playtest round.
   */
  catalogError?: string | null
  /** Whether this building's tech prerequisite is unmet. */
  isTechLocked: (b: BuildingOption) => boolean
  /** Whether the colony could fund this building's construction right now. */
  isAffordable: (b: BuildingOption) => boolean
}>()

const emit = defineEmits<{
  (e: 'queue', building: BuildingOption, quantity: number): void
  (e: 'close'): void
}>()

/** Per-building selected quantity, defaulting to 1. Keyed by building id. */
const quantities = reactive<Record<string, number>>({})

function quantityFor(id: string): number {
  return quantities[id] ?? 1
}

/** Clamp to a sane minimum so a blank/zero input can't queue nothing. */
function setQuantity(id: string, value: number): void {
  quantities[id] = Number.isFinite(value) && value >= 1 ? Math.floor(value) : 1
}


// ── Catalogue filters ─────────────────────────────────────────────────────
//
// Two independent filters rather than one "only what I can build": they hide
// different things for different reasons, and a player usually wants one at a
// time. Hiding tech-locked entries is about browsing what is reachable now;
// hiding unaffordable ones is about deciding what to spend on. Collapsing them
// would make "what could I build if I had the metal?" unanswerable.
//
// Off by default — the full roster is the honest starting view, and a filter
// silently on from a previous session would look like missing content.

const HIDE_TECH_LOCKED_KEY = 'outpost3.build-dialog.hide-tech-locked'
const HIDE_UNAFFORDABLE_KEY = 'outpost3.build-dialog.hide-unaffordable'

function loadFlag(key: string): boolean {
  try {
    return window.localStorage.getItem(key) === 'true'
  } catch {
    // storage blocked (private mode, embedded webview) — non-fatal
    return false
  }
}

function persistFlag(key: string, on: boolean): void {
  try {
    window.localStorage.setItem(key, String(on))
  } catch {
    // storage blocked — the filter still works for this session
  }
}

const hideTechLocked = ref(loadFlag(HIDE_TECH_LOCKED_KEY))
const hideUnaffordable = ref(loadFlag(HIDE_UNAFFORDABLE_KEY))

watch(hideTechLocked, (on) => persistFlag(HIDE_TECH_LOCKED_KEY, on), { flush: 'sync' })
watch(hideUnaffordable, (on) => persistFlag(HIDE_UNAFFORDABLE_KEY, on), { flush: 'sync' })

const visibleCatalog = computed(() =>
  props.catalog.filter((b) => {
    if (hideTechLocked.value && props.isTechLocked(b)) return false
    if (hideUnaffordable.value && !props.isAffordable(b)) return false
    return true
  }),
)

/** How many entries the active filters are hiding — shown so an unexpectedly
 * short list reads as "filtered", not as "content is missing". */
const hiddenCount = computed(() => props.catalog.length - visibleCatalog.value.length)

function queue(b: BuildingOption): void {
  if (props.busy || props.disabledReason(b) !== null) return
  emit('queue', b, quantityFor(b.id))
}
</script>

<template>
  <div class="dialog-backdrop" data-testid="build-dialog" @click.self="emit('close')">
    <div class="dialog">
      <div class="dialog-head">
        <h3>Build</h3>
        <span v-if="props.slotsAvailable !== null" class="slots" data-testid="build-dialog-slots">
          {{ props.slotsAvailable }} slot{{ props.slotsAvailable === 1 ? '' : 's' }} free
        </span>
        <button class="btn-close" data-testid="btn-close-build" aria-label="Close" @click="emit('close')">✕</button>
      </div>

      <div v-if="props.catalog.length > 0" class="filters" data-testid="build-dialog-filters">
        <label class="filter">
          <input
            type="checkbox"
            data-testid="filter-hide-tech-locked"
            v-model="hideTechLocked"
          />
          Hide tech-locked
        </label>
        <label class="filter">
          <input
            type="checkbox"
            data-testid="filter-hide-unaffordable"
            v-model="hideUnaffordable"
          />
          Hide unaffordable
        </label>
        <span v-if="hiddenCount > 0" class="filter-count" data-testid="build-dialog-hidden-count">
          {{ hiddenCount }} hidden
        </span>
      </div>

      <div v-if="props.catalogError" class="err" data-testid="build-dialog-error">
        {{ props.catalogError }}
      </div>
      <div v-else-if="props.catalog.length === 0" class="hint">
        No buildings available in the loaded content pack.
      </div>
      <div v-else-if="visibleCatalog.length === 0" class="hint" data-testid="build-dialog-all-filtered">
        All {{ props.catalog.length }} buildings are hidden by the filters above.
      </div>
      <div v-else class="build-catalog">
        <div
          v-for="b in visibleCatalog"
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
            <label class="qty-label">
              ×
              <input
                class="qty-input"
                type="number"
                min="1"
                :value="quantityFor(b.id)"
                :data-testid="`qty-${b.id}`"
                @input="setQuantity(b.id, Number(($event.target as HTMLInputElement).value))"
              />
            </label>
            <button
              class="btn-queue"
              :disabled="props.busy || props.disabledReason(b) !== null"
              :data-testid="`btn-queue-${b.id}`"
              @click="queue(b)"
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
.filters {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 0.4rem 0;
  font-size: 0.75rem;
  color: var(--text-muted);
  border-bottom: 1px solid var(--border-subtle);
  margin-bottom: 0.5rem;
  flex-wrap: wrap;
}
.filter { display: flex; align-items: center; gap: 0.3rem; cursor: pointer; }
.filter input { cursor: pointer; accent-color: var(--accent); }
.filter-count { margin-left: auto; color: var(--text-faint); font-style: italic; }
.err { color: var(--danger); font-size: 0.8rem; padding: 0.5rem 0; }
.dialog-backdrop {
  position: fixed;
  inset: 0;
  background: var(--overlay-backdrop);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 150;
  padding: 1rem;
}
.dialog {
  background: var(--surface-2);
  border: 1px solid var(--border-strong);
  border-radius: 6px;
  padding: 1rem 1.25rem;
  width: min(920px, 100%);
  max-height: 85vh;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}
.dialog-head { display: flex; align-items: baseline; gap: 0.75rem; }
.dialog-head h3 { color: var(--accent); margin: 0; }
.slots { color: var(--text-muted); font-size: 0.78rem; }
.btn-close {
  margin-left: auto;
  background: transparent;
  border: 1px solid var(--border-strong);
  border-radius: 3px;
  color: var(--text);
  cursor: pointer;
  padding: 0.15rem 0.45rem;
  font-family: monospace;
}
.btn-close:hover { background: var(--surface-btn-hover); }
.hint { font-size: 0.8rem; color: var(--text-dim); font-style: italic; }

.build-catalog { display: grid; grid-template-columns: repeat(auto-fill, minmax(240px, 1fr)); gap: 0.5rem; }
.build-card {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 0.55rem 0.6rem;
  color: var(--text);
}
.build-card.is-disabled { opacity: 0.55; }
.build-card-head { display: flex; justify-content: space-between; align-items: baseline; gap: 0.5rem; }
.build-card-name { color: var(--accent); font-size: 0.86rem; font-weight: 600; }
.build-card-cat { color: var(--text-faint); font-size: 0.68rem; text-transform: uppercase; letter-spacing: 0.05em; }
.build-card-desc { color: var(--text-muted); font-size: 0.76rem; }
.build-card-stats { color: var(--text-dim); font-size: 0.72rem; }
.build-card-cost { display: flex; gap: 0.25rem; flex-wrap: wrap; }
.cost-chip {
  background: var(--surface-alt);
  border: 1px solid var(--border-subtle);
  border-radius: 2px;
  padding: 0.05rem 0.3rem;
  color: var(--good-dim);
  font-size: 0.7rem;
}
.build-card-foot { display: flex; justify-content: flex-end; align-items: center; gap: 0.5rem; margin-top: 0.25rem; }
.build-card-reason { color: var(--warn-dim); font-size: 0.7rem; font-style: italic; margin-right: auto; }
.qty-label { color: var(--text-dim); font-size: 0.78rem; display: flex; align-items: center; gap: 0.2rem; }
.qty-input {
  width: 3.2rem;
  background: var(--surface-3);
  border: 1px solid var(--border);
  border-radius: 3px;
  color: var(--text-bright);
  padding: 0.2rem 0.35rem;
  font-family: monospace;
  font-size: 0.78rem;
}
.btn-queue {
  background: var(--accent-bg);
  border: 1px solid var(--border-accent);
  border-radius: 3px;
  color: var(--accent);
  padding: 0.3rem 0.7rem;
  font-family: monospace;
  font-size: 0.78rem;
  cursor: pointer;
}
.btn-queue:disabled { opacity: 0.45; cursor: not-allowed; }
.btn-queue:hover:not(:disabled) { background: var(--accent-bg); }
</style>
