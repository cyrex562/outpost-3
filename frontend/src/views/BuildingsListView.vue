<script setup lang="ts">
/**
 * All-buildings list (navigation rework #7 phase 3) — every building in
 * every colony, flattened into one table. Uses only `worldStore.colonies`
 * (already loaded, no new query) — each `ColonyState.buildings` is just a
 * `string[]` of building types, so this table is necessarily type/colony
 * only, not full per-instance detail (recipe, power, etc.); click a row to
 * open that building's full detail on its facility page.
 */

import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { useWorldStore } from '@/stores/worldStore'

const router = useRouter()
const worldStore = useWorldStore()

interface BuildingRow {
  colonyId: string
  colonyName: string
  buildingType: string
}

const rows = computed((): BuildingRow[] => {
  const out: BuildingRow[] = []
  for (const colony of worldStore.colonies) {
    for (const buildingType of colony.buildings) {
      out.push({ colonyId: colony.id, colonyName: colony.name, buildingType })
    }
  }
  return out.sort(
    (a, b) => a.colonyName.localeCompare(b.colonyName) || a.buildingType.localeCompare(b.buildingType),
  )
})

function openFacility(row: BuildingRow): void {
  void router.push({
    name: 'facility',
    params: { colonyId: row.colonyId, buildingType: row.buildingType },
  })
}
</script>

<template>
  <div class="buildings-view" data-testid="buildings-list-view">
    <div class="toolbar">
      <h2>Buildings</h2>
      <span class="count">{{ rows.length }} building{{ rows.length === 1 ? '' : 's' }} across {{ worldStore.colonies.length }} colon{{ worldStore.colonies.length === 1 ? 'y' : 'ies' }}</span>
    </div>

    <p v-if="rows.length === 0" class="hint">No buildings constructed anywhere yet.</p>

    <table v-else class="buildings-table" data-testid="buildings-table">
      <thead>
        <tr>
          <th>Building</th>
          <th>Colony</th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="(row, i) in rows"
          :key="`${row.colonyId}-${row.buildingType}-${i}`"
          class="building-row"
          :data-testid="`building-row-${row.colonyId}-${row.buildingType}-${i}`"
          @click="openFacility(row)"
        >
          <td>{{ row.buildingType }}</td>
          <td>{{ row.colonyName }}</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<style scoped>
.buildings-view { display: flex; flex-direction: column; gap: 0.75rem; }
.toolbar { display: flex; align-items: center; gap: 1rem; }
.toolbar h2 { color: var(--accent); margin: 0; }
.count { color: var(--text-muted); font-size: 0.8rem; }

.buildings-table { width: 100%; border-collapse: collapse; font-size: 0.82rem; }
.buildings-table th {
  text-align: left;
  color: var(--text-dim);
  font-weight: normal;
  padding: 0.4rem 0.6rem;
  border-bottom: 1px solid var(--border);
}
.buildings-table td { padding: 0.4rem 0.6rem; color: var(--text); border-bottom: 1px solid var(--border-subtle); }
.building-row { cursor: pointer; }
.building-row:hover { background: var(--surface-2); }

.hint { color: var(--text-faint); font-style: italic; font-size: 0.85rem; }
</style>
