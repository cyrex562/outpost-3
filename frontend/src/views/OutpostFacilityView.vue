<script setup lang="ts">
/**
 * Outpost facility management page (navigation rework #7 phase 4) — the
 * outpost-scoped counterpart to `FacilityView.vue`. Reached by clicking a
 * building in `OutpostView.vue`'s building list.
 */

import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import BuildingDetailsHud from '@/components/BuildingDetailsHud.vue'

const route = useRoute()
const router = useRouter()

const outpostId = computed((): string | null => {
  const raw = route.params.outpostId
  return typeof raw === 'string' && raw.length > 0 ? raw : null
})

const buildingType = computed((): string | null => {
  const raw = route.params.buildingType
  return typeof raw === 'string' && raw.length > 0 ? raw : null
})

function backToOutpost(): void {
  if (outpostId.value) {
    router.push({ name: 'outpost', params: { outpostId: outpostId.value } })
  } else {
    router.push({ name: 'installations' })
  }
}
</script>

<template>
  <BuildingDetailsHud
    owner-type="outpost"
    :owner-id="outpostId"
    :building-type="buildingType"
    as-page
    @close="backToOutpost"
  />
</template>
