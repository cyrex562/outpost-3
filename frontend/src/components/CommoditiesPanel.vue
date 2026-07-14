<script setup lang="ts">
/** Colony commodity stockpile panel: amount, capacity, and net/sol per commodity. */

import type { StockpileRow } from '@/types/screen'

const props = defineProps<{
  /** `null` when the colony screen hasn't loaded yet for the selected colony. */
  stockpile: StockpileRow[] | null
}>()

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
        </tr>
      </thead>
      <tbody>
        <tr v-for="row in props.stockpile" :key="row.commodity_id" :data-testid="`stock-row-${row.commodity_id}`">
          <td>{{ row.commodity_id }}</td>
          <td class="num">{{ row.amount.toFixed(1) }}</td>
          <td class="num">{{ row.capacity != null ? row.capacity.toFixed(1) : '∞' }}</td>
          <td class="num" :class="netClass(row.net_per_turn)">{{ formatNet(row.net_per_turn) }}</td>
        </tr>
        <tr v-if="props.stockpile.length === 0">
          <td colspan="4" class="empty-row">No commodities tracked yet.</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<style scoped>
.panel { padding: 0.75rem; height: 100%; overflow-y: auto; box-sizing: border-box; }
.panel-title { color: #8cf; font-size: 0.9rem; margin: 0 0 0.6rem; }
.hint { font-size: 0.75rem; color: #446; font-style: italic; }

.stock-table { width: 100%; border-collapse: collapse; font-size: 0.8rem; }
.stock-table th { color: #668; font-weight: normal; text-align: left; padding: 0.2rem 0.4rem; border-bottom: 1px solid #222; }
.stock-table th.num, .stock-table td.num { text-align: right; }
.stock-table td { padding: 0.25rem 0.4rem; border-bottom: 1px solid #1a1a24; color: #aab; }
.stock-table tbody tr:hover td { background: #13131e; }
.net-positive { color: #4c9; }
.net-negative { color: #c55; }
.net-zero     { color: #667; }
.empty-row { color: #445; font-style: italic; }
</style>
