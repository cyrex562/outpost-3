<script setup lang="ts">
/**
 * Colony commodity stockpile panel: amount, capacity, net/sol, and the reserve
 * the player has withheld from industry (issue #308).
 *
 * The reserve is a floor *within* `amount`, not a separate bucket — reserved
 * stock is still in the pool and still counted in `amount`. Recipe inputs and
 * building maintenance cannot draw the pool below it; colonist needs still can,
 * which is what makes reserving food useful rather than suicidal.
 *
 * Like `BuildingsPanel`, this raises an intent and lets the view dispatch, so
 * the panel stays free of store and transport concerns.
 */

import { ref } from 'vue'
import type { StockpileRow } from '@/types/screen'

const props = defineProps<{
  /** `null` when the colony screen hasn't loaded yet for the selected colony. */
  stockpile: StockpileRow[] | null
}>()

const emit = defineEmits<{
  /** Withhold `amount` of `commodityId` from industry; `0` clears the reserve. */
  (e: 'set-reserve', commodityId: string, amount: number): void
}>()

/** Commodity id currently being edited, or `null` when no row is in edit mode. */
const editing = ref<string | null>(null)
/**
 * The in-progress value, `string | number` because `v-model` on a
 * `type="number"` input hands back a **number** once the field parses and the
 * raw string only while it doesn't (empty, `"-"`, `"1e"`). Typing this as
 * `string` and calling `.trim()` is a crash, not a cast.
 */
const draft = ref<string | number>('')

function beginEdit(row: StockpileRow): void {
  editing.value = row.commodity_id
  // Start from the current reserve so adjusting is a tweak, not a retype.
  draft.value = row.reserved > 0 ? row.reserved : ''
}

function cancelEdit(): void {
  editing.value = null
  draft.value = ''
}

function saveEdit(commodityId: string): void {
  const raw = typeof draft.value === 'string' ? draft.value.trim() : draft.value
  // An empty field reads as "no reserve" rather than as a mistake — it is the
  // natural way to clear one, and matches how the rename control treats blank.
  const parsed = raw === '' ? 0 : Number(raw)
  if (!Number.isFinite(parsed) || parsed < 0) {
    // Leave the field open so the bad value is still visible and correctable;
    // the engine would reject it anyway, and a silent revert hides the typo.
    return
  }
  emit('set-reserve', commodityId, parsed)
  cancelEdit()
}

function netClass(net: number): string {
  if (net > 0) return 'net-positive'
  if (net < 0) return 'net-negative'
  return 'net-zero'
}

function formatNet(net: number): string {
  if (net > 0) return `+${net.toFixed(2)}`
  return net.toFixed(2)
}
</script>

<template>
  <div class="panel" data-testid="commodities-panel">
    <h4 class="panel-title">Commodities</h4>
    <div v-if="props.stockpile === null" class="hint">Advance a turn to load commodity data.</div>
    <table v-else class="stock-table" data-testid="commodity-table">
      <thead>
        <tr>
          <th>Commodity</th>
          <th class="num">Amount</th>
          <th class="num">Capacity</th>
          <th class="num">Net/Sol</th>
          <th class="num" title="Withheld from industry; colonists can still consume it">
            Reserved
          </th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="row in props.stockpile"
          :key="row.commodity_id"
          :data-testid="`stock-row-${row.commodity_id}`"
        >
          <td>{{ row.commodity_id }}</td>
          <td class="num">{{ row.amount.toFixed(1) }}</td>
          <td class="num">{{ row.capacity != null ? row.capacity.toFixed(1) : '∞' }}</td>
          <td class="num" :class="netClass(row.net_per_turn)">{{ formatNet(row.net_per_turn) }}</td>
          <td class="num reserve-cell">
            <template v-if="editing === row.commodity_id">
              <input
                v-model="draft"
                class="reserve-input"
                type="number"
                min="0"
                step="any"
                :data-testid="`reserve-input-${row.commodity_id}`"
                @keyup.enter="saveEdit(row.commodity_id)"
                @keyup.escape="cancelEdit"
              />
              <button
                class="mini"
                :data-testid="`reserve-save-${row.commodity_id}`"
                @click="saveEdit(row.commodity_id)"
              >
                ✓
              </button>
              <button class="mini" :data-testid="`reserve-cancel-${row.commodity_id}`" @click="cancelEdit">
                ✕
              </button>
            </template>
            <template v-else>
              <button
                class="reserve-value"
                :class="{ 'is-set': row.reserved > 0 }"
                :title="
                  row.reserved > 0
                    ? `${row.reserved} withheld from industry — click to change`
                    : 'Click to withhold some of this commodity from industry'
                "
                :data-testid="`reserve-edit-${row.commodity_id}`"
                @click="beginEdit(row)"
              >
                {{ row.reserved > 0 ? row.reserved.toFixed(1) : '—' }}
              </button>
            </template>
          </td>
        </tr>
        <tr v-if="props.stockpile.length === 0">
          <td colspan="5" class="empty-row">No commodities tracked yet.</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<style scoped>
.panel { padding: 0.75rem; height: 100%; overflow-y: auto; box-sizing: border-box; }
.panel-title { color: var(--accent); font-size: 0.9rem; margin: 0 0 0.6rem; }
.hint { font-size: 0.75rem; color: var(--border-strong); font-style: italic; }

.stock-table { width: 100%; border-collapse: collapse; font-size: 0.8rem; }
.stock-table th { color: var(--text-dim); font-weight: normal; text-align: left; padding: 0.2rem 0.4rem; border-bottom: 1px solid var(--border-subtle); }
.stock-table th.num, .stock-table td.num { text-align: right; }
.stock-table td { padding: 0.25rem 0.4rem; border-bottom: 1px solid var(--surface-btn); color: var(--text); }
.stock-table tbody tr:hover td { background: var(--surface-2); }
.net-positive { color: var(--good); }
.net-negative { color: var(--danger); }
.net-zero     { color: var(--text-dim); }
.empty-row { color: var(--border); font-style: italic; }

.reserve-cell { white-space: nowrap; }
.reserve-value {
  background: none;
  border: 1px dashed var(--surface-btn-hover);
  border-radius: 3px;
  color: var(--text-dim);
  cursor: pointer;
  font: inherit;
  padding: 0 0.3rem;
}
.reserve-value:hover { border-color: var(--accent-border); color: var(--accent); }
.reserve-value.is-set { border-style: solid; border-color: var(--warn-dim); color: var(--warn); }
.reserve-input {
  background: var(--surface-3);
  border: 1px solid var(--accent-border);
  border-radius: 3px;
  color: var(--accent-soft);
  font: inherit;
  padding: 0 0.2rem;
  text-align: right;
  width: 4.5rem;
}
.mini {
  background: none;
  border: 1px solid var(--surface-btn-hover);
  border-radius: 3px;
  color: var(--accent);
  cursor: pointer;
  font-size: 0.7rem;
  margin-left: 0.15rem;
  padding: 0 0.25rem;
}
.mini:hover { border-color: var(--accent-border); }
</style>
