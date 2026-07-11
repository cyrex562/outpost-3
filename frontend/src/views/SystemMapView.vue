<script setup lang="ts">
import { onMounted, ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { getSystemBodies, type SystemBody } from '@/services/tauriBridge'
import { useWorldStore } from '@/stores/worldStore'

const router = useRouter()
const worldStore = useWorldStore()

const bodies = ref<SystemBody[]>([])
const selected = ref<SystemBody | null>(null)
const error = ref<string | null>(null)

onMounted(async () => {
  try {
    bodies.value = await getSystemBodies()
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  }
})

// ── Layout math ────────────────────────────────────────────────────────────

const svgSize = 720
const center = svgSize / 2
// Scale distance_au → pixels so the farthest body lands at ~45% of half-width.
const maxAu = computed(() =>
  Math.max(1, ...bodies.value.map((b) => b.distance_au)),
)
const scale = computed(() => (center * 0.9) / maxAu.value)

// Distribute bodies around the star by hashing their id → angle.
function angleFor(id: string): number {
  let h = 0
  for (let i = 0; i < id.length; i++) h = (h * 31 + id.charCodeAt(i)) | 0
  return ((h >>> 0) % 360) * (Math.PI / 180)
}

function bodyPos(b: SystemBody): { x: number; y: number } {
  const r = b.distance_au * scale.value
  const a = angleFor(b.id)
  return { x: center + r * Math.cos(a), y: center + r * Math.sin(a) }
}

function bodyColor(b: SystemBody): string {
  switch (b.kind) {
    case 'InnerPlanet':
      return '#c96'
    case 'GasGiant':
      return '#8ab'
    case 'Moon':
      return '#aab'
    case 'AsteroidBelt':
      return '#665'
    case 'OrbitalStation':
      return '#4c8'
    default:
      return '#889'
  }
}

function bodyRadius(b: SystemBody): number {
  switch (b.kind) {
    case 'GasGiant':
      return 14
    case 'InnerPlanet':
      return 8
    case 'Moon':
      return 4
    case 'AsteroidBelt':
      return 3
    default:
      return 6
  }
}

const orbitRadii = computed(() =>
  bodies.value.map((b) => b.distance_au * scale.value),
)

function goToColony(): void {
  router.push('/colony')
}

function foundColony(): void {
  router.push('/found')
}
</script>

<template>
  <div class="system-view" data-testid="system-map">
    <div class="toolbar">
      <h2>Kepler System</h2>
      <div class="clock">
        Sol {{ worldStore.sol }} · Month {{ worldStore.month }}
      </div>
      <div class="actions">
        <button class="btn" @click="foundColony">Found Colony</button>
        <button class="btn" @click="goToColony">Colony Dashboard</button>
      </div>
    </div>

    <div class="content">
      <svg
        :width="svgSize"
        :height="svgSize"
        class="map"
        data-testid="system-map-svg"
      >
        <!-- Star at center -->
        <circle
          :cx="center"
          :cy="center"
          r="14"
          fill="#fda"
          stroke="#ffd"
          stroke-width="1"
        />
        <text
          :x="center"
          :y="center - 20"
          text-anchor="middle"
          fill="#fda"
          font-size="11"
          font-family="monospace"
        >
          KEPLER
        </text>

        <!-- Orbit tracks -->
        <circle
          v-for="(r, i) in orbitRadii"
          :key="`orbit-${i}`"
          :cx="center"
          :cy="center"
          :r="r"
          fill="none"
          stroke="#334"
          stroke-width="1"
          stroke-dasharray="2 3"
        />

        <!-- Bodies -->
        <g
          v-for="b in bodies"
          :key="b.id"
          class="body-group"
          :class="{ selected: selected?.id === b.id }"
          @click="selected = b"
        >
          <circle
            :cx="bodyPos(b).x"
            :cy="bodyPos(b).y"
            :r="bodyRadius(b)"
            :fill="bodyColor(b)"
            stroke="#000"
            stroke-width="1"
          />
          <text
            :x="bodyPos(b).x"
            :y="bodyPos(b).y + bodyRadius(b) + 12"
            text-anchor="middle"
            fill="#aac"
            font-size="10"
            font-family="monospace"
          >
            {{ b.name }}
          </text>
        </g>
      </svg>

      <aside class="side-panel" v-if="selected" data-testid="body-details">
        <h3>{{ selected.name }}</h3>
        <dl class="stats">
          <dt>Type</dt>
          <dd>{{ selected.kind }}</dd>
          <dt>Role</dt>
          <dd>{{ selected.role }}</dd>
          <dt>Distance</dt>
          <dd>{{ selected.distance_au.toFixed(2) }} AU</dd>
          <dt>Colonizable</dt>
          <dd>{{ selected.colonizable ? 'yes' : 'no' }}</dd>
        </dl>
        <button
          v-if="selected.colonizable"
          class="btn primary"
          @click="foundColony"
        >
          Found Colony Here
        </button>
      </aside>
      <aside v-else class="side-panel hint">
        Click a body to inspect it.
      </aside>
    </div>

    <p v-if="error" class="err">{{ error }}</p>
  </div>
</template>

<style scoped>
.system-view { display: flex; flex-direction: column; gap: 0.75rem; }
.toolbar { display: flex; align-items: center; gap: 1rem; }
.toolbar h2 { color: #8cf; }
.clock { color: #8a8; font-size: 0.85rem; }
.actions { margin-left: auto; display: flex; gap: 0.5rem; }

.btn {
  background: #1a1a28;
  border: 1px solid #446;
  border-radius: 3px;
  color: #aac;
  padding: 0.4rem 0.75rem;
  font-family: monospace;
  font-size: 0.82rem;
  cursor: pointer;
}
.btn:hover { background: #22223a; }
.btn.primary { border-color: #468; color: #8cf; }

.content { display: flex; gap: 1rem; align-items: flex-start; }
.map {
  background: radial-gradient(ellipse at center, #05050b 0%, #000 80%);
  border: 1px solid #223;
  border-radius: 6px;
}

.body-group { cursor: pointer; }
.body-group.selected circle { stroke: #8cf; stroke-width: 2; }

.side-panel {
  min-width: 220px;
  background: #101018;
  border: 1px solid #334;
  border-radius: 6px;
  padding: 1rem;
  color: #aab;
}
.side-panel h3 { color: #8cf; margin-bottom: 0.5rem; }
.side-panel.hint { color: #557; font-style: italic; }

.stats { display: grid; grid-template-columns: 90px 1fr; gap: 0.35rem 0.6rem; font-size: 0.8rem; margin-bottom: 0.75rem; }
.stats dt { color: #668; }
.stats dd { color: #aab; }
.err { color: #d66; font-size: 0.8rem; }
</style>
