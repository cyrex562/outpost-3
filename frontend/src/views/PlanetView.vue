<script setup lang="ts">
/**
 * Planet map (map/nav plan) — a persistent, routed view of the founding
 * planet's hex map showing every colony as a node on its hex.
 *
 * - Navigate mode (default): clicking a colony node opens its dashboard
 *   (phase A1/A2 — this is the colony-switching hub).
 * - Build mode (phase A3b): pick two colony nodes and an infrastructure type
 *   to lay a road / rail / pipeline between them; existing edges can be
 *   demolished. The edges themselves are drawn by `PlanetHexMap` (phase A3a).
 */

import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import PlanetHexMap from '@/components/PlanetHexMap.vue'
import FloatingWindow from '@/components/FloatingWindow.vue'
import { useGameStore } from '@/stores/game'
import { getPlanetMap, type InfraEdge, type PlanetHex, type PlanetMap } from '@/services/tauriBridge'

const router = useRouter()
const gameStore = useGameStore()

const planetMap = ref<PlanetMap | null>(null)
const error = ref<string | null>(null)
const loading = ref(false)

type Mode = 'navigate' | 'build'
const mode = ref<Mode>('navigate')
const fromColonyId = ref<string | null>(null)
const toColonyId = ref<string | null>(null)
const infraType = ref<'road' | 'rail' | 'pipeline' | 'powerline'>('road')
const busy = ref(false)

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

/** Colony id → display name, from the occupied hexes. */
const colonyNameById = computed(() => {
  const m = new Map<string, string>()
  for (const h of planetMap.value?.hexes ?? []) {
    if (h.occupant_colony_id && h.occupied_by) m.set(h.occupant_colony_id, h.occupied_by)
  }
  return m
})

function nameOf(id: string | null): string {
  return id ? (colonyNameById.value.get(id) ?? id) : ''
}

const edges = computed<InfraEdge[]>(() => planetMap.value?.edges ?? [])

/** Highlight the chosen `from` endpoint's hex on the map while building. */
const selectedSite = computed<string | null>(() => {
  if (mode.value !== 'build' || !fromColonyId.value) return null
  const hex = planetMap.value?.hexes.find((h) => h.occupant_colony_id === fromColonyId.value)
  return hex?.site_id ?? null
})

function toggleMode(): void {
  mode.value = mode.value === 'build' ? 'navigate' : 'build'
  fromColonyId.value = null
  toColonyId.value = null
}

/** In build mode, clicking colony nodes fills the two endpoint slots. */
function pickEndpoint(cid: string): void {
  if (fromColonyId.value === cid) {
    fromColonyId.value = null
  } else if (toColonyId.value === cid) {
    toColonyId.value = null
  } else if (!fromColonyId.value) {
    fromColonyId.value = cid
  } else {
    toColonyId.value = cid
  }
}

function onHexSelect(hex: PlanetHex): void {
  const cid = hex.occupant_colony_id
  if (!cid) return
  if (mode.value === 'navigate') {
    void router.push({ name: 'colony', params: { colonyId: cid } })
  } else {
    pickEndpoint(cid)
  }
}

const canBuild = computed(
  () => !!fromColonyId.value && !!toColonyId.value && fromColonyId.value !== toColonyId.value && !busy.value,
)

// build/demolish always emit an event on success (InfrastructureBuilt /
// InfrastructureDemolished), and sendCommand returns [] only when the engine
// rejected the command (it surfaces the reason on gameStore.toastMessage) —
// so an empty result here means "rejected", not "accepted with no effect".
async function build(): Promise<void> {
  if (!canBuild.value || !fromColonyId.value || !toColonyId.value) return
  busy.value = true
  try {
    const events = await gameStore.sendCommand({
      kind: 'build_infrastructure',
      from_colony: fromColonyId.value,
      to_colony: toColonyId.value,
      infra_type: infraType.value,
    })
    if (events.length > 0) {
      fromColonyId.value = null
      toColonyId.value = null
      await refresh()
    } else {
      error.value = gameStore.toastMessage ?? 'Build rejected.'
    }
  } finally {
    busy.value = false
  }
}

async function demolish(edge: InfraEdge): Promise<void> {
  if (busy.value) return
  busy.value = true
  try {
    const events = await gameStore.sendCommand({
      kind: 'demolish_infrastructure',
      from_colony: edge.from_colony_id,
      to_colony: edge.to_colony_id,
    })
    if (events.length > 0) {
      await refresh()
    } else {
      error.value = gameStore.toastMessage ?? 'Demolish rejected.'
    }
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="planet-view" data-testid="planet-view">
    <div class="toolbar">
      <h2>Planet Map</h2>
      <span class="hint">
        {{ mode === 'build' ? 'Pick two colonies to connect.' : 'Click a colony to open its dashboard.' }}
      </span>
      <button class="btn" data-testid="btn-toggle-build" :class="{ active: mode === 'build' }" @click="toggleMode">
        {{ mode === 'build' ? 'Done' : 'Build Infrastructure' }}
      </button>
    </div>

    <p v-if="error" class="err" data-testid="planet-error">{{ error }}</p>
    <p v-else-if="loading" class="hint">Loading…</p>

    <template v-else-if="planetMap">
      <div v-if="mode === 'build'" class="build-panel" data-testid="build-panel">
        <div class="endpoints">
          <span class="slot" :class="{ filled: fromColonyId }">From: {{ fromColonyId ? nameOf(fromColonyId) : '—' }}</span>
          <span class="slot" :class="{ filled: toColonyId }">To: {{ toColonyId ? nameOf(toColonyId) : '—' }}</span>
          <select v-model="infraType" class="type-select" data-testid="infra-type-select">
            <option value="road">Road</option>
            <option value="rail">Rail</option>
            <option value="pipeline">Pipeline</option>
            <option value="powerline">Powerline</option>
          </select>
          <button class="btn primary" data-testid="btn-build" :disabled="!canBuild" @click="build">Build</button>
        </div>

        <div v-if="edges.length" class="edge-list" data-testid="edge-list">
          <div v-for="e in edges" :key="`${e.from_colony_id}-${e.to_colony_id}-${e.infra_type}`" class="edge-row">
            <span class="edge-desc">
              {{ nameOf(e.from_colony_id) }} ↔ {{ nameOf(e.to_colony_id) }} · {{ e.infra_type }}
              ({{ e.throughput.toFixed(0) }}/turn, {{ (e.loss_pct * 100).toFixed(0) }}% loss)
            </span>
            <button class="btn danger" :disabled="busy" @click="demolish(e)">Demolish</button>
          </div>
        </div>
        <p v-else class="hint">No infrastructure built yet.</p>
      </div>

      <!-- The planet map is a resizable, moveable window (UI-rework PR6):
           drag its title bar to reposition, drag the corner to resize, or use
           the title-bar button to maximise; its geometry persists per-player.
           `.map-host` is the positioned area it floats within.

           `fill-host` (issue #320) makes it open filling that host rather than
           at 760×520, which left most of a large display empty. The initial
           width/height are the fallback for hosts that can't be measured. -->
      <div class="map-host">
        <FloatingWindow
          title="Planet Map"
          storage-key="outpost3.planet-window"
          fill-host
          :initial-x="16"
          :initial-y="16"
          :initial-width="760"
          :initial-height="520"
        >
          <PlanetHexMap
            :map="planetMap"
            :selected-site="selectedSite"
            :highlight-top-n="0"
            selectable-occupied
            @select="onHexSelect"
          />
        </FloatingWindow>
      </div>
    </template>
  </div>
</template>

<style scoped>
.planet-view { display: flex; flex-direction: column; gap: 0.75rem; height: 100%; }
.toolbar { display: flex; align-items: center; gap: 1rem; }
.toolbar h2 { color: var(--accent); margin: 0; }
.hint { color: var(--text-faint); font-style: italic; font-size: 0.85rem; }
.err { color: var(--danger); font-size: 0.8rem; }
/* Positioned host the floating planet-map window lives inside (UI-rework
   PR6). `position: relative` anchors the window's absolute coordinates; it
   fills the remaining space so the window has room to move/resize. */
.map-host { flex: 1; min-height: 0; position: relative; overflow: hidden; }

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
.btn.active { border-color: var(--accent); color: var(--accent); }
.btn.primary { border-color: var(--border-accent); color: var(--accent); }
.btn.danger { border-color: var(--danger-border); color: var(--warn); }
.btn:disabled { opacity: 0.45; cursor: not-allowed; }
/* Push the mode-toggle button to the right edge of the toolbar. */
.toolbar > .btn { margin-left: auto; }

.build-panel {
  background: var(--surface-1);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 0.75rem;
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
}
.endpoints { display: flex; align-items: center; gap: 0.75rem; flex-wrap: wrap; }
.slot { color: var(--text-dim); font-size: 0.82rem; }
.slot.filled { color: var(--text); }
.type-select {
  background: var(--surface-3);
  border: 1px solid var(--border);
  border-radius: 3px;
  color: var(--text-bright);
  padding: 0.3rem 0.4rem;
  font-family: monospace;
  font-size: 0.8rem;
}
.edge-list { display: flex; flex-direction: column; gap: 0.35rem; }
.edge-row { display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; font-size: 0.8rem; color: var(--text); }
.edge-desc { font-family: monospace; }
</style>
