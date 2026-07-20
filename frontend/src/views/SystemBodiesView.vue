<script setup lang="ts">
/**
 * System bodies list (navigation rework #7 phase 3) — every body in the
 * system in a scannable table, since `SystemMapView.vue`'s orbital-diagram
 * layout is great for spatial context but awkward for comparing many
 * bodies' stats side by side. Clicking a row jumps to `/system?body=<id>`,
 * which `SystemMapView.vue` reads on mount to preselect that body's info
 * panel.
 */

import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { getSystemBodies, type SystemBody } from '@/services/tauriBridge'

const router = useRouter()

const bodies = ref<SystemBody[]>([])
const error = ref<string | null>(null)
const loading = ref(false)

onMounted(async () => {
  loading.value = true
  try {
    bodies.value = await getSystemBodies()
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
})

const sortedBodies = computed(() => [...bodies.value].sort((a, b) => a.distance_au - b.distance_au))

function viewOnMap(body: SystemBody): void {
  void router.push({ path: '/system', query: { body: body.id } })
}
</script>

<template>
  <div class="bodies-view" data-testid="system-bodies-view">
    <div class="toolbar">
      <h2>System Bodies</h2>
      <span class="count">{{ bodies.length }} bod{{ bodies.length === 1 ? 'y' : 'ies' }}</span>
    </div>

    <p v-if="error" class="err">{{ error }}</p>
    <p v-if="loading" class="hint">Loading…</p>

    <table v-else class="bodies-table" data-testid="bodies-table">
      <thead>
        <tr>
          <th>Name</th>
          <th>Kind</th>
          <th>Distance (AU)</th>
          <th>Habitability</th>
          <th>Colonizable</th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="b in sortedBodies"
          :key="b.id"
          class="body-row"
          :data-testid="`body-row-${b.id}`"
          @click="viewOnMap(b)"
        >
          <td>{{ b.name }}</td>
          <td>{{ b.kind }}</td>
          <td>{{ b.distance_au.toFixed(2) }}</td>
          <td>{{ (b.habitability_effective * 100).toFixed(0) }}%</td>
          <td>{{ b.colonizable ? 'Yes' : 'No' }}</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<style scoped>
.bodies-view { display: flex; flex-direction: column; gap: 0.75rem; }
.toolbar { display: flex; align-items: center; gap: 1rem; }
.toolbar h2 { color: #8cf; margin: 0; }
.count { color: #779; font-size: 0.8rem; }

.bodies-table { width: 100%; border-collapse: collapse; font-size: 0.82rem; }
.bodies-table th {
  text-align: left;
  color: #668;
  font-weight: normal;
  padding: 0.4rem 0.6rem;
  border-bottom: 1px solid #334;
}
.bodies-table td { padding: 0.4rem 0.6rem; color: #aab; border-bottom: 1px solid #223; }
.body-row { cursor: pointer; }
.body-row:hover { background: #16162a; }

.hint { color: #557; font-style: italic; font-size: 0.85rem; }
.err { color: #d66; font-size: 0.8rem; }
</style>
