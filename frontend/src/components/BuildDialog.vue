<script setup lang="ts">
/**
 * Build dialog (UI-rework PR5) — a modal picker for queueing construction.
 * Lists the buildable catalog (with tech/slot gating reasons) and lets the
 * player choose a quantity per building before queueing, then emits `queue`
 * with the building and the chosen count. Opened from the construction-queue
 * panel's "Build…" button; the catalog used to live inline in that panel.
 */

import { reactive } from 'vue'
import type { BuildingOption } from '@/services/tauriBridge'

const props = defineProps<{
  catalog: BuildingOption[]
  disabledReason: (b: BuildingOption) => string | null
  slotsAvailable: number | null
  busy: boolean
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

      <div v-if="props.catalog.length === 0" class="hint">
        No buildings available in the loaded content pack.
      </div>
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
.dialog-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.65);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 150;
  padding: 1rem;
}
.dialog {
  background: #12121c;
  border: 1px solid #446;
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
.dialog-head h3 { color: #8cf; margin: 0; }
.slots { color: #778; font-size: 0.78rem; }
.btn-close {
  margin-left: auto;
  background: transparent;
  border: 1px solid #446;
  border-radius: 3px;
  color: #aac;
  cursor: pointer;
  padding: 0.15rem 0.45rem;
  font-family: monospace;
}
.btn-close:hover { background: #22223a; }
.hint { font-size: 0.8rem; color: #667; font-style: italic; }

.build-catalog { display: grid; grid-template-columns: repeat(auto-fill, minmax(240px, 1fr)); gap: 0.5rem; }
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
.build-card-foot { display: flex; justify-content: flex-end; align-items: center; gap: 0.5rem; margin-top: 0.25rem; }
.build-card-reason { color: #a86; font-size: 0.7rem; font-style: italic; margin-right: auto; }
.qty-label { color: #668; font-size: 0.78rem; display: flex; align-items: center; gap: 0.2rem; }
.qty-input {
  width: 3.2rem;
  background: #0d0d15;
  border: 1px solid #334;
  border-radius: 3px;
  color: #cdd;
  padding: 0.2rem 0.35rem;
  font-family: monospace;
  font-size: 0.78rem;
}
.btn-queue {
  background: #1a2030;
  border: 1px solid #468;
  border-radius: 3px;
  color: #8cf;
  padding: 0.3rem 0.7rem;
  font-family: monospace;
  font-size: 0.78rem;
  cursor: pointer;
}
.btn-queue:disabled { opacity: 0.45; cursor: not-allowed; }
.btn-queue:hover:not(:disabled) { background: #22293a; }
</style>
