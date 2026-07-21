<script setup lang="ts">
/**
 * Facility management page (navigation rework #7 phase 2) — the routed
 * home for a single building's detail view, reached by clicking a building
 * in `ColonyView`'s `BuildingsPanel`. Previously this only existed as
 * `BuildingDetailsHud`'s local modal (no back-stack position, no deep
 * link); this thin wrapper renders the same component in page mode
 * (`asPage`) and turns its `close` emit into a real navigation back to the
 * colony.
 */

import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import BuildingDetailsHud from '@/components/BuildingDetailsHud.vue'

const route = useRoute()
const router = useRouter()

const colonyId = computed((): string | null => {
  const raw = route.params.colonyId
  return typeof raw === 'string' && raw.length > 0 ? raw : null
})

const buildingType = computed((): string | null => {
  const raw = route.params.buildingType
  return typeof raw === 'string' && raw.length > 0 ? raw : null
})

function backToColony(): void {
  if (colonyId.value) {
    router.push({ name: 'colony', params: { colonyId: colonyId.value } })
  } else {
    router.push({ name: 'colony' })
  }
}
</script>

<template>
  <BuildingDetailsHud
    owner-type="colony"
    :owner-id="colonyId"
    :building-type="buildingType"
    as-page
    @close="backToColony"
  />
</template>
