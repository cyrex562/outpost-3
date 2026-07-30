<script setup lang="ts">
/**
 * `dockview-vue` panel wrapper for the building-details inspector (issue
 * #322). Unlike the other `Dock*Panel.vue` wrappers, this one *does* vary
 * per instance (which building it shows), so it reads `buildingType` from
 * dockview's own `params` mechanism instead of `ColonyDockContext` — the
 * mechanism the rest of this dock deliberately avoids (see `colonyDock.ts`)
 * because it's the one thing here that's genuinely per-panel.
 *
 * `dockview-vue` hands a panel component a single `params` prop shaped
 * `{params: <what was passed to addPanel/updateParameters>, api, containerApi,
 * tabLocation}` — the double nesting is the library's own wrapping, not a
 * mistake here.
 */
import { computed, inject } from 'vue'
import BuildingDetailsHud from '@/components/BuildingDetailsHud.vue'
import { COLONY_DOCK_CONTEXT_KEY, type BuildingDetailsPanelParams } from '@/dock/colonyDock'

interface DockviewPanelProps {
  params?: BuildingDetailsPanelParams
  api?: { close(): void }
}

const props = defineProps<{ params?: DockviewPanelProps }>()

const ctx = inject(COLONY_DOCK_CONTEXT_KEY)
if (!ctx) {
  throw new Error('DockBuildingDetailsPanel must be mounted inside a colony dockview with ColonyDockContext provided')
}

const buildingType = computed(() => props.params?.params?.buildingType ?? null)

/** The "← Back" button in `BuildingDetailsHud`'s `asPage` mode reads as a
 * close action here (there's no page to go back to) — close the dock tab. */
function close(): void {
  props.params?.api?.close()
}
</script>

<template>
  <BuildingDetailsHud
    owner-type="colony"
    :owner-id="ctx.colonyId"
    :building-type="buildingType"
    as-page
    @close="close"
  />
</template>
