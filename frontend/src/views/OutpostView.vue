<script setup lang="ts">
/**
 * Outpost drill-down page (navigation rework #7 phase 4 — outpost
 * parity with colonies). Mirrors `ColonyView.vue`'s route-param-driven
 * selection shape, scaled down to what an outpost actually has: no
 * population/construction-queue-catalog concerns, just slot usage,
 * stockpile, and a clickable building list that opens the same kind of
 * per-building detail page `FacilityView.vue` gives colony buildings
 * (via `OutpostFacilityView.vue`, reusing `BuildingDetailsHud` with
 * `owner-type="outpost"`).
 *
 * There is no single-outpost fetch endpoint — `listOutposts()` returns
 * every outpost system-wide already (used unfiltered by
 * `InstallationsView.vue`), so this just filters client-side by the route
 * param, same tradeoff `ColonyView.vue` makes reading from `worldStore`.
 */

import { computed, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useWorldStore } from '@/stores/worldStore'
import { listOutposts, type Outpost } from '@/services/tauriBridge'

const route = useRoute()
const router = useRouter()
const worldStore = useWorldStore()

const outposts = ref<Outpost[]>([])
const error = ref<string | null>(null)
const loading = ref(false)

async function refresh(): Promise<void> {
  loading.value = true
  error.value = null
  try {
    outposts.value = await listOutposts()
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}

onMounted(refresh)

const outpostId = computed((): string | null => {
  const raw = route.params.outpostId
  return typeof raw === 'string' && raw.length > 0 ? raw : null
})

const outpost = computed((): Outpost | null => outposts.value.find((o) => o.id === outpostId.value) ?? null)

function colonyName(colonyId: string): string {
  return worldStore.world.colonies[colonyId]?.name ?? colonyId
}

function openBuildingDetails(buildingType: string): void {
  if (!outpost.value) return
  void router.push({ name: 'outpost-facility', params: { outpostId: outpost.value.id, buildingType } })
}
</script>

<template>
  <div class="outpost-view" data-testid="outpost-view">
    <p v-if="error" class="err">{{ error }}</p>
    <p v-if="loading" class="hint">Loading…</p>

    <template v-else-if="outpost">
      <div class="outpost-header">
        <h2>{{ outpost.name }}</h2>
        <span class="body-tag">{{ outpost.body_name }}</span>
      </div>

      <dl class="stats">
        <dt>Colony</dt>
        <dd>{{ colonyName(outpost.parent_colony_id) }}</dd>
        <dt>Slots</dt>
        <dd>{{ outpost.slots_used }} / {{ outpost.slot_capacity }}</dd>
      </dl>

      <section class="section">
        <h3>Buildings</h3>
        <p v-if="outpost.buildings.length === 0" class="hint">No buildings constructed yet.</p>
        <ul v-else class="building-list" data-testid="outpost-building-list">
          <li
            v-for="(buildingType, i) in outpost.buildings"
            :key="`${buildingType}-${i}`"
            class="building-row"
            :data-testid="`outpost-building-${buildingType}-${i}`"
            @click="openBuildingDetails(buildingType)"
          >
            {{ buildingType }}
          </li>
        </ul>
      </section>

      <section v-if="outpost.pool.length > 0" class="section">
        <h3>Stockpile</h3>
        <span v-for="[cid, amt] in outpost.pool" :key="cid" class="pool-chip">
          {{ cid }}: {{ amt.toFixed(1) }}
        </span>
      </section>
    </template>

    <p v-else class="hint" data-testid="outpost-not-found">Outpost not found.</p>
  </div>
</template>

<style scoped>
.outpost-view { display: flex; flex-direction: column; gap: 0.75rem; }

.outpost-header { display: flex; align-items: center; gap: 0.5rem; }
.outpost-header h2 { color: var(--accent); margin: 0; }
.body-tag { color: var(--text-muted); font-size: 0.78rem; }

.stats { display: grid; grid-template-columns: 100px 1fr; gap: 0.3rem 0.6rem; font-size: 0.8rem; }
.stats dt { color: var(--text-dim); }
.stats dd { color: var(--text); margin: 0; }

.section { margin-top: 0.25rem; }
.section h3 { color: var(--text-muted); font-size: 0.85rem; margin: 0 0 0.4rem; }

.building-list { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 0.4rem; }
.building-row {
  background: var(--surface-1);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 0.4rem 0.6rem;
  font-size: 0.85rem;
  color: var(--text);
  cursor: pointer;
}
.building-row:hover { background: var(--surface-2); border-color: var(--border-strong); }

.pool-chip { display: inline-block; background: var(--surface-2); border: 1px solid var(--border); border-radius: 3px; padding: 0.1rem 0.4rem; font-size: 0.72rem; margin: 0.1rem 0.25rem 0.1rem 0; color: var(--text); }

.hint { color: var(--text-faint); font-style: italic; font-size: 0.85rem; }
.err { color: var(--danger); font-size: 0.8rem; }
</style>
