<script setup lang="ts">
/**
 * Trade routes (issue #363) — create, view, and remove manually-added trade
 * routes between any two colonies, regardless of whether they share a body.
 *
 * `build_infrastructure` (see `PlanetView.vue`) only links colonies on the
 * same body — a road/rail/pipeline needs surface to run on. This screen is
 * the body-agnostic path: it drives `add_trade_route`/`remove_trade_route`
 * directly, which the engine derives convoy transit time for from body
 * separation exactly the way an infrastructure-linked route would.
 */

import { computed, onMounted, ref } from 'vue'
import { useGameStore } from '@/stores/game'
import { useWorldStore } from '@/stores/worldStore'
import { getTradeRoutes, type TradeRoute } from '@/services/tauriBridge'

const gameStore = useGameStore()
const worldStore = useWorldStore()

const routes = ref<TradeRoute[]>([])
const error = ref<string | null>(null)
const loading = ref(false)
const busy = ref(false)

const colonyA = ref<string>('')
const colonyB = ref<string>('')
const throughputCap = ref<number>(20)

async function refresh(): Promise<void> {
  loading.value = true
  error.value = null
  try {
    routes.value = await getTradeRoutes()
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}

onMounted(refresh)

function nameOf(colonyId: string): string {
  return worldStore.colonies.find((c) => c.id === colonyId)?.name ?? colonyId
}

const canAdd = computed(
  () =>
    !!colonyA.value &&
    !!colonyB.value &&
    colonyA.value !== colonyB.value &&
    throughputCap.value >= 0 &&
    !busy.value,
)

// A successful add/remove always emits an event (TradeRouteAdded /
// TradeRouteRemoved); sendCommand returns [] only when the engine rejected
// the command, surfacing the reason on gameStore.toastMessage.
async function addRoute(): Promise<void> {
  if (!canAdd.value) return
  busy.value = true
  try {
    const events = await gameStore.sendCommand({
      kind: 'add_trade_route',
      colony_a: colonyA.value,
      colony_b: colonyB.value,
      throughput_cap: throughputCap.value,
    })
    if (events.length > 0) {
      colonyA.value = ''
      colonyB.value = ''
      await refresh()
    } else {
      error.value = gameStore.toastMessage ?? 'Add route rejected.'
    }
  } finally {
    busy.value = false
  }
}

async function removeRoute(route: TradeRoute): Promise<void> {
  if (busy.value) return
  busy.value = true
  try {
    const events = await gameStore.sendCommand({
      kind: 'remove_trade_route',
      route_id: route.id,
    })
    if (events.length > 0) {
      await refresh()
    } else {
      error.value = gameStore.toastMessage ?? 'Remove route rejected.'
    }
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="trade-view" data-testid="trade-view">
    <div class="toolbar">
      <h2>Trade Routes</h2>
      <span class="hint">Connect any two colonies — no shared body required.</span>
    </div>

    <p v-if="error" class="err" data-testid="trade-error">{{ error }}</p>

    <div class="panel" data-testid="add-route-panel">
      <div class="endpoints">
        <select v-model="colonyA" class="colony-select" data-testid="colony-a-select">
          <option value="" disabled>Colony A</option>
          <option v-for="c in worldStore.colonies" :key="c.id" :value="c.id">{{ c.name }}</option>
        </select>
        <span class="arrow">↔</span>
        <select v-model="colonyB" class="colony-select" data-testid="colony-b-select">
          <option value="" disabled>Colony B</option>
          <option v-for="c in worldStore.colonies" :key="c.id" :value="c.id">{{ c.name }}</option>
        </select>
        <input
          v-model.number="throughputCap"
          type="number"
          min="0"
          step="1"
          class="cap-input"
          data-testid="throughput-cap-input"
        />
        <button class="btn primary" data-testid="btn-add-route" :disabled="!canAdd" @click="addRoute">
          Add Route
        </button>
      </div>
    </div>

    <p v-if="loading" class="hint">Loading…</p>
    <template v-else>
      <div v-if="routes.length" class="route-list" data-testid="route-list">
        <div v-for="r in routes" :key="r.id" class="route-row" :data-testid="`route-row-${r.id}`">
          <span class="route-desc">
            {{ nameOf(r.colony_a) }} ↔ {{ nameOf(r.colony_b) }}
            ({{ r.throughput_cap.toFixed(0) }}/turn, {{ r.transit_sols }} sol transit)
          </span>
          <button class="btn danger" :disabled="busy" @click="removeRoute(r)">Remove</button>
        </div>
      </div>
      <p v-else class="hint" data-testid="no-routes">No trade routes yet.</p>
    </template>
  </div>
</template>

<style scoped>
.trade-view { display: flex; flex-direction: column; gap: 0.75rem; height: 100%; padding: 1rem; }
.toolbar { display: flex; align-items: center; gap: 1rem; }
.toolbar h2 { color: var(--accent); margin: 0; }
.hint { color: var(--text-faint); font-style: italic; font-size: 0.85rem; }
.err { color: var(--danger); font-size: 0.8rem; }

.panel {
  background: var(--surface-1);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 0.75rem;
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
}
.endpoints { display: flex; align-items: center; gap: 0.6rem; flex-wrap: wrap; }
.arrow { color: var(--text-dim); }
.colony-select,
.cap-input {
  background: var(--surface-3);
  border: 1px solid var(--border);
  border-radius: 3px;
  color: var(--text-bright);
  padding: 0.3rem 0.4rem;
  font-family: monospace;
  font-size: 0.8rem;
}
.cap-input { width: 5rem; }

.btn {
  background: var(--surface-btn);
  border: 1px solid var(--border-strong);
  border-radius: 3px;
  color: var(--text);
  padding: 0.35rem 0.7rem;
  font-family: monospace;
  font-size: 0.8rem;
  cursor: pointer;
}
.btn:hover { background: var(--surface-btn-hover); }
.btn.primary { border-color: var(--border-accent); color: var(--accent); }
.btn.danger { border-color: var(--danger-border); color: var(--warn); }
.btn:disabled { opacity: 0.45; cursor: not-allowed; }

.route-list { display: flex; flex-direction: column; gap: 0.35rem; }
.route-row { display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; font-size: 0.8rem; color: var(--text); }
.route-desc { font-family: monospace; }
</style>
