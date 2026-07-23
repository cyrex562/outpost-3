<script setup lang="ts">
/**
 * Planet map (map/nav plan, phase A1) — a persistent, routed view of the
 * founding planet's hex map showing every colony as a node on its hex. This
 * promotes `PlanetHexMap` out of the founding wizard (where it's a one-time
 * site picker) into a standing strategic view: clicking a colony node drills
 * into that colony's dashboard.
 *
 * Read-only for now — infrastructure links between colonies and making this
 * the primary colony-navigation hub come in later phases (A2/A3).
 */

import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import PlanetHexMap from '@/components/PlanetHexMap.vue'
import { getPlanetMap, type PlanetHex, type PlanetMap } from '@/services/tauriBridge'

const router = useRouter()

const planetMap = ref<PlanetMap | null>(null)
const error = ref<string | null>(null)
const loading = ref(false)

async function refresh(): Promise<void> {
  loading.value = true
  error.value = null
  try {
    planetMap.value = await getPlanetMap()
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}

onMounted(refresh)

/** Clicking a colony node routes to that colony's dashboard. */
function onHexSelect(hex: PlanetHex): void {
  if (hex.occupant_colony_id) {
    void router.push({ name: 'colony', params: { colonyId: hex.occupant_colony_id } })
  }
}
</script>

<template>
  <div class="planet-view" data-testid="planet-view">
    <div class="toolbar">
      <h2>Planet Map</h2>
      <span class="hint">Click a colony to open its dashboard.</span>
    </div>

    <p v-if="error" class="err" data-testid="planet-error">{{ error }}</p>
    <p v-else-if="loading" class="hint">Loading…</p>

    <div v-else-if="planetMap" class="map-host">
      <PlanetHexMap
        :map="planetMap"
        :selected-site="null"
        :highlight-top-n="0"
        selectable-occupied
        @select="onHexSelect"
      />
    </div>
  </div>
</template>

<style scoped>
.planet-view { display: flex; flex-direction: column; gap: 0.75rem; height: 100%; }
.toolbar { display: flex; align-items: baseline; gap: 1rem; }
.toolbar h2 { color: #8cf; margin: 0; }
.hint { color: #557; font-style: italic; font-size: 0.85rem; }
.err { color: #d66; font-size: 0.8rem; }
.map-host { flex: 1; min-height: 70vh; }
</style>
