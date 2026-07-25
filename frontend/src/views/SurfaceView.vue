<script setup lang="ts">
/**
 * Surface preview (map/nav plan) — a read-only, procedurally-generated hex
 * map of any system body's surface (planet or moon), reachable from the
 * system map's "View Surface" action. Unlike `PlanetView`, this carries no
 * colonies or infrastructure and offers no build/select interactions; it's a
 * scouting look at a body you may not have settled.
 */

import { onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import PlanetHexMap from '@/components/PlanetHexMap.vue'
import { getBodySurface, type PlanetMap } from '@/services/tauriBridge'

const route = useRoute()
const router = useRouter()

const surface = ref<PlanetMap | null>(null)
const error = ref<string | null>(null)
const loading = ref(false)

// The system map passes the body's display name along as a query param so we
// can title the page without a second round trip; fall back to the id.
const bodyName = ref<string>('')

async function load(bodyId: string): Promise<void> {
  loading.value = true
  error.value = null
  surface.value = null
  try {
    surface.value = await getBodySurface(bodyId)
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}

function syncFromRoute(): void {
  const id = String(route.params.bodyId ?? '')
  const nameParam = route.query.name
  bodyName.value = typeof nameParam === 'string' && nameParam.length > 0 ? nameParam : id
  if (id) void load(id)
}

onMounted(syncFromRoute)
// Re-fetch when navigating between bodies without leaving the route.
watch(() => route.params.bodyId, syncFromRoute)

function backToSystem(): void {
  void router.push({ name: 'system' })
}
</script>

<template>
  <div class="surface-view" data-testid="surface-view">
    <div class="toolbar">
      <h2>Surface — {{ bodyName }}</h2>
      <span class="hint">Read-only preview. Found a colony from the system map to settle here.</span>
      <button class="btn" data-testid="btn-back-system" @click="backToSystem">Back to System</button>
    </div>

    <p v-if="error" class="err" data-testid="surface-error">{{ error }}</p>
    <p v-else-if="loading" class="hint">Loading…</p>

    <div v-else-if="surface" class="map-host">
      <PlanetHexMap :map="surface" :selected-site="null" :highlight-top-n="0" />
    </div>
  </div>
</template>

<style scoped>
.surface-view { display: flex; flex-direction: column; gap: 0.75rem; height: 100%; }
.toolbar { display: flex; align-items: center; gap: 1rem; }
.toolbar h2 { color: #8cf; margin: 0; }
.hint { color: #557; font-style: italic; font-size: 0.85rem; }
.err { color: #d66; font-size: 0.8rem; }
/* Fill the shell rather than reserving a fixed 70vh (issue #320): `min-height:
   0` lets the flex item shrink to the space `.app-main` actually gives it, so
   the map grows on a large display and stops pushing the app into a scrollbar
   on a short one. `PlanetHexMap` fills this host and fits its own viewBox. */
.map-host { flex: 1; min-height: 0; }

.btn {
  background: #1a1a28;
  border: 1px solid #446;
  border-radius: 3px;
  color: #aac;
  padding: 0.35rem 0.7rem;
  font-family: monospace;
  font-size: 0.8rem;
  cursor: pointer;
}
.btn:hover { background: #22223a; }
/* Push the back button to the right edge of the toolbar. */
.toolbar > .btn { margin-left: auto; }
</style>
