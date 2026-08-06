<script setup lang="ts">
/**
 * Live balance-tuning panel — playtesting tool.
 *
 * Balance numbers are explicitly open scalars (DESIGN.md §17 Q5: "all scalars,
 * to be tuned via the harness"). Until now, changing one meant editing content
 * and restarting, which makes it impossible to feel the effect of a dial in the
 * run you are already playing. This edits the live `DifficultyScalar` table
 * mid-game so a value can be nudged and its effect watched on the next sol.
 *
 * The dial list comes from the engine's canonical `TUNABLE` set rather than
 * being hardcoded here, so adding a quantity in the engine surfaces it in the
 * UI with no frontend change.
 *
 * Each edit is a `set_balance_scalar` command touching exactly one quantity —
 * unlike `set_custom_difficulty`, which replaces the whole table and also
 * toggles menace/hazards, and so would silently undo the rest of a tuning pass.
 */

import { onMounted, ref } from 'vue'
import { getBalanceScalars, type BalanceScalar } from '@/services/tauriBridge'
import { useGameStore } from '@/stores/game'

const gameStore = useGameStore()

const scalars = ref<BalanceScalar[]>([])
const error = ref<string | null>(null)
const loading = ref(false)

/** Turn `resource_consumption` into `Resource consumption` for display. */
function label(slug: string): string {
  const words = slug.replace(/_/g, ' ')
  return words.charAt(0).toUpperCase() + words.slice(1)
}

async function load(): Promise<void> {
  loading.value = true
  error.value = null
  try {
    scalars.value = await getBalanceScalars()
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}

/**
 * Apply one dial. The engine clamps, so re-read its answer rather than trusting
 * the local input — a value dragged past the bound must show where it landed.
 */
async function apply(row: BalanceScalar, raw: number): Promise<void> {
  if (!Number.isFinite(raw)) return
  error.value = null
  try {
    await gameStore.sendCommand({
      kind: 'set_balance_scalar',
      quantity: row.quantity,
      value: raw,
    })
    await load()
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  }
}

function onSlide(row: BalanceScalar, event: Event): void {
  const value = Number((event.target as HTMLInputElement).value)
  void apply(row, value)
}

function onNumber(row: BalanceScalar, event: Event): void {
  const value = Number((event.target as HTMLInputElement).value)
  void apply(row, value)
}

/** Put a single dial back to its unmodified 1.0. */
function reset(row: BalanceScalar): void {
  void apply(row, 1.0)
}

onMounted(load)
</script>

<template>
  <div class="panel" data-testid="balance-tuning-panel">
    <div class="panel-head">
      <h4 class="panel-title">Balance tuning</h4>
      <span class="hint">Playtesting — applies to the running game.</span>
    </div>

    <p v-if="error" class="err" data-testid="balance-error">{{ error }}</p>
    <p v-else-if="loading" class="hint">Loading…</p>

    <ul v-else class="dials">
      <li v-for="row in scalars" :key="row.quantity" class="dial" :data-testid="`dial-${row.quantity}`">
        <div class="dial-head">
          <span class="dial-label">{{ label(row.quantity) }}</span>
          <span
            class="dial-value"
            :class="{ modified: Math.abs(row.value - 1) > 1e-6 }"
            :data-testid="`dial-value-${row.quantity}`"
            >×{{ row.value.toFixed(2) }}</span
          >
        </div>
        <div class="dial-controls">
          <!-- Slider covers the common range; the number box reaches the extremes. -->
          <input
            class="slider"
            type="range"
            :min="row.min"
            :max="3"
            step="0.05"
            :value="Math.min(row.value, 3)"
            :data-testid="`slider-${row.quantity}`"
            @change="onSlide(row, $event)"
          />
          <input
            class="number"
            type="number"
            :min="row.min"
            :max="row.max"
            step="0.05"
            :value="row.value"
            :data-testid="`number-${row.quantity}`"
            @change="onNumber(row, $event)"
          />
          <button
            class="btn-reset"
            :disabled="Math.abs(row.value - 1) <= 1e-6"
            :data-testid="`reset-${row.quantity}`"
            @click="reset(row)"
          >
            Reset
          </button>
        </div>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.panel {
  background: var(--surface-3);
  border: 1px solid var(--border-subtle);
  border-radius: 6px;
  padding: 0.75rem;
}
.panel-head {
  display: flex;
  align-items: baseline;
  gap: 0.75rem;
  margin-bottom: 0.5rem;
}
.panel-title {
  color: var(--accent);
  margin: 0;
}
.hint {
  color: var(--text-muted);
  font-size: 0.8rem;
}
.err {
  color: var(--danger-strong);
  font-size: 0.85rem;
}
.dials {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
}
.dial-head {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
}
.dial-label {
  color: var(--accent-soft);
  font-size: 0.9rem;
}
.dial-value {
  color: var(--text-muted);
  font-variant-numeric: tabular-nums;
  font-size: 0.85rem;
}
/* A dial that is no longer at its default should be obvious at a glance. */
.dial-value.modified {
  color: var(--warn);
  font-weight: 600;
}
.dial-controls {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}
.slider {
  flex: 1;
  min-width: 0;
}
.number {
  width: 5rem;
  background: var(--bg);
  color: var(--accent-soft);
  border: 1px solid var(--border-subtle);
  border-radius: 4px;
  padding: 0.15rem 0.3rem;
}
.btn-reset {
  background: var(--accent-bg);
  color: var(--text);
  border: 1px solid var(--border-subtle);
  border-radius: 4px;
  padding: 0.15rem 0.5rem;
  cursor: pointer;
  font-size: 0.8rem;
}
.btn-reset:disabled {
  opacity: 0.4;
  cursor: default;
}
</style>
