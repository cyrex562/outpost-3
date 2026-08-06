<script setup lang="ts">
/**
 * Colony utilities panel (issue #304) — power, housing capacity, and research
 * output for the current sol.
 *
 * These are deliberately *not* in `CommoditiesPanel`. They aren't cargo: they
 * are produced and consumed in place and can never be traded or shipped, so
 * showing them alongside the tradeable stockpile implied a hauler could carry
 * them away.
 *
 * There is no amount-in-store column because there is no store: a `flow`
 * resource's surplus is lost at the end of each sol, and a `capacity` one is a
 * standing figure its buildings re-establish. So every number here reads
 * "this sol", which the header states outright.
 */

import type { ResourceRow } from '@/types/screen'

const props = defineProps<{
  /** `null` when the colony screen hasn't loaded yet for the selected colony. */
  resources: ResourceRow[] | null
}>()

/** Human-readable gloss for the two temporal kinds. */
function kindLabel(kind: string): string {
  if (kind === 'capacity') return 'capacity'
  return 'per sol'
}

function kindTitle(kind: string): string {
  return kind === 'capacity'
    ? 'A standing capability re-established by its buildings each sol.'
    : 'Throughput this sol — any surplus is lost rather than stockpiled.'
}

function formatAmount(amount: number): string {
  // Sub-unit trickles (colony_hq's 1 RP/sol scaled down by a shortfall) would
  // otherwise render as a bare "0".
  return Math.abs(amount) < 10 ? amount.toFixed(1) : amount.toFixed(0)
}
</script>

<template>
  <div class="panel" data-testid="utilities-panel">
    <h4 class="panel-title">Utilities</h4>
    <p class="panel-note">Produced and used on-site — not tradeable.</p>

    <div v-if="props.resources === null" class="hint">
      Advance a turn to load utility data.
    </div>
    <ul v-else-if="props.resources.length > 0" class="util-list" data-testid="utility-list">
      <li
        v-for="r in props.resources"
        :key="r.resource_id"
        class="util-item"
        :data-testid="`utility-row-${r.resource_id}`"
      >
        <span class="util-name">{{ r.name }}</span>
        <span class="util-amount" :data-testid="`utility-amount-${r.resource_id}`">
          {{ formatAmount(r.amount) }}<span v-if="r.unit" class="util-unit">{{ r.unit }}</span>
        </span>
        <span class="util-kind" :title="kindTitle(r.kind)">{{ kindLabel(r.kind) }}</span>
      </li>
    </ul>
    <div v-else class="hint" data-testid="utilities-empty">
      No utilities produced this sol.
    </div>
  </div>
</template>

<style scoped>
.panel { padding: 0.75rem; height: 100%; overflow-y: auto; box-sizing: border-box; }
.panel-title { color: var(--accent); font-size: 0.9rem; margin: 0 0 0.15rem; }
.panel-note { color: var(--text-faint); font-size: 0.68rem; font-style: italic; margin: 0 0 0.6rem; }
.hint { font-size: 0.75rem; color: var(--border-strong); font-style: italic; }

.util-list { list-style: none; display: flex; flex-direction: column; gap: 0.35rem; margin: 0; padding: 0; }
.util-item {
  display: flex;
  align-items: baseline;
  gap: 0.5rem;
  background: var(--surface-2);
  border: 1px solid var(--border-subtle);
  border-radius: 3px;
  padding: 0.3rem 0.5rem;
  font-size: 0.8rem;
  color: var(--text);
}
.util-name { flex: 1 0 auto; }
.util-amount { font-family: monospace; color: var(--text-bright); font-size: 0.9rem; }
.util-unit { color: var(--text-dim); font-size: 0.68rem; margin-left: 0.15rem; }
.util-kind {
  color: var(--text-dim);
  font-size: 0.66rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  border: 1px solid var(--border);
  border-radius: 2px;
  padding: 0.02rem 0.28rem;
}
</style>
