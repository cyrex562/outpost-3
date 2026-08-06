<script setup lang="ts">
/**
 * Outposts management view (issue #243) — list/manage a colony's established
 * outposts, establish new ones on in-range bodies (#241), queue construction,
 * and promote an outpost into a full colony (#242).
 */

import { computed, onMounted, ref, watch } from 'vue'
import { useWorldStore } from '@/stores/worldStore'
import { useGameStore } from '@/stores/game'
import {
  listOutposts,
  getOutpostTargets,
  listBuildings,
  type Outpost,
  type OutpostTarget,
  type BuildingOption,
} from '@/services/tauriBridge'

const worldStore = useWorldStore()
const gameStore = useGameStore()

const colonies = computed(() => worldStore.colonies)
const selectedColonyId = computed({
  get: () => gameStore.selectedColonyId ?? colonies.value[0]?.id ?? null,
  set: (id: string | null) => {
    gameStore.selectedColonyId = id
  },
})

const allOutposts = ref<Outpost[]>([])
const outposts = computed(() =>
  allOutposts.value.filter((o) => o.parent_colony_id === selectedColonyId.value),
)

const error = ref<string | null>(null)
const loading = ref(false)

async function refreshOutposts(): Promise<void> {
  loading.value = true
  error.value = null
  try {
    allOutposts.value = await listOutposts()
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  if (!gameStore.selectedColonyId && colonies.value.length > 0) {
    gameStore.selectedColonyId = colonies.value[0].id
  }
  void refreshOutposts()
})

// ─── Establish flow ─────────────────────────────────────────────────────────

const showEstablish = ref(false)
const targets = ref<OutpostTarget[]>([])
const targetsLoading = ref(false)
const selectedBodyId = ref<string | null>(null)
const newOutpostName = ref('')

async function openEstablish(): Promise<void> {
  showEstablish.value = true
  selectedBodyId.value = null
  const colonyId = selectedColonyId.value
  if (!colonyId) return
  targetsLoading.value = true
  try {
    targets.value = await getOutpostTargets(colonyId)
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    targetsLoading.value = false
  }
}

function closeEstablish(): void {
  showEstablish.value = false
}

async function submitEstablish(): Promise<void> {
  const colonyId = selectedColonyId.value
  const bodyId = selectedBodyId.value
  const name = newOutpostName.value.trim()
  if (!colonyId || !bodyId || !name) return
  const events = await gameStore.sendCommand({
    kind: 'establish_outpost',
    name,
    colony_id: colonyId,
    body_id: bodyId,
  })
  if (events.length > 0) {
    newOutpostName.value = ''
    showEstablish.value = false
    error.value = null
    await refreshOutposts()
  } else {
    error.value = gameStore.toastMessage ?? 'Outpost establishment rejected.'
  }
}

// ─── Decommission / promote ─────────────────────────────────────────────────

async function decommission(outpostId: string): Promise<void> {
  const events = await gameStore.sendCommand({ kind: 'decommission_outpost', outpost_id: outpostId })
  if (events.length > 0) {
    error.value = null
    await refreshOutposts()
  } else {
    error.value = gameStore.toastMessage ?? 'Decommission rejected.'
  }
}

const promoting = ref<string | null>(null)
const promoteName = ref('')
const promotePopulation = ref(50)

function openPromote(outpost: Outpost): void {
  promoting.value = outpost.id
  promoteName.value = outpost.name
  promotePopulation.value = 50
}

function closePromote(): void {
  promoting.value = null
}

async function submitPromote(): Promise<void> {
  const outpostId = promoting.value
  const name = promoteName.value.trim()
  if (!outpostId || !name) return
  const events = await gameStore.sendCommand({
    kind: 'promote_outpost_to_colony',
    outpost_id: outpostId,
    name,
    starting_population: Math.max(1, Math.floor(promotePopulation.value)),
  })
  if (events.length > 0) {
    promoting.value = null
    error.value = null
    await refreshOutposts()
  } else {
    error.value = gameStore.toastMessage ?? 'Promotion rejected.'
  }
}

// ─── Construction queue ─────────────────────────────────────────────────────

const buildOptions = ref<BuildingOption[]>([])
const queuingFor = ref<string | null>(null)
const selectedBuilding = ref<string | null>(null)

async function openQueue(outpostId: string): Promise<void> {
  queuingFor.value = outpostId
  selectedBuilding.value = null
  if (buildOptions.value.length === 0) {
    try {
      buildOptions.value = await listBuildings()
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    }
  }
}

function closeQueue(): void {
  queuingFor.value = null
}

async function submitQueue(): Promise<void> {
  const outpostId = queuingFor.value
  const buildingId = selectedBuilding.value
  const option = buildOptions.value.find((b) => b.id === buildingId)
  if (!outpostId || !option) return
  const events = await gameStore.sendCommand({
    kind: 'queue_outpost_construction',
    outpost_id: outpostId,
    building_type: option.id,
    slot_cost: option.slot_cost,
    labor_per_turn: option.labor_per_turn,
    construction_cost: option.construction_cost,
    construction_turns: option.construction_turns,
  })
  if (events.length > 0) {
    queuingFor.value = null
    error.value = null
    await refreshOutposts()
  } else {
    error.value = gameStore.toastMessage ?? 'Construction queueing rejected.'
  }
}

watch(selectedColonyId, () => {
  showEstablish.value = false
  promoting.value = null
  queuingFor.value = null
})
</script>

<template>
  <div class="outposts-view" data-testid="outposts-view">
    <div class="toolbar">
      <h2>Outposts</h2>
      <select
        v-if="colonies.length > 1"
        v-model="selectedColonyId"
        class="colony-select"
        data-testid="outposts-colony-select"
      >
        <option v-for="c in colonies" :key="c.id" :value="c.id">{{ c.name }}</option>
      </select>
      <div class="actions">
        <button class="btn primary" data-testid="btn-establish-outpost" @click="openEstablish">
          Establish Outpost
        </button>
      </div>
    </div>

    <p v-if="error" class="err">{{ error }}</p>
    <p v-if="loading" class="hint">Loading…</p>
    <p v-else-if="outposts.length === 0" class="hint">
      No outposts established for this colony yet.
    </p>

    <ul class="outpost-list" data-testid="outpost-list">
      <li v-for="o in outposts" :key="o.id" class="outpost-card" :data-testid="`outpost-${o.id}`">
        <div class="outpost-header">
          <h3>{{ o.name }}</h3>
          <span class="body-tag">{{ o.body_name }}</span>
        </div>
        <dl class="stats">
          <dt>Slots</dt>
          <dd>{{ o.slots_used }} / {{ o.slot_capacity }}</dd>
          <dt>Buildings</dt>
          <dd>{{ o.buildings.length === 0 ? 'none' : o.buildings.join(', ') }}</dd>
          <dt v-if="o.pool.length > 0">Stockpile</dt>
          <dd v-if="o.pool.length > 0">
            <span v-for="[cid, amt] in o.pool" :key="cid" class="pool-chip">
              {{ cid }}: {{ amt.toFixed(1) }}
            </span>
          </dd>
        </dl>
        <div class="card-actions">
          <button class="btn" @click="openQueue(o.id)">Queue Building</button>
          <button class="btn" @click="openPromote(o)">Promote to Colony</button>
          <button class="btn danger" @click="decommission(o.id)">Decommission</button>
        </div>

        <div v-if="queuingFor === o.id" class="inline-form" data-testid="queue-form">
          <select v-model="selectedBuilding">
            <option :value="null" disabled>Select building…</option>
            <option v-for="b in buildOptions" :key="b.id" :value="b.id">{{ b.name }}</option>
          </select>
          <button class="btn primary" :disabled="!selectedBuilding" @click="submitQueue">Queue</button>
          <button class="btn" @click="closeQueue">Cancel</button>
        </div>

        <div v-if="promoting === o.id" class="inline-form" data-testid="promote-form">
          <input v-model="promoteName" placeholder="Colony name" />
          <input v-model.number="promotePopulation" type="number" min="1" placeholder="Starting population" />
          <button class="btn primary" data-testid="btn-confirm-promote" @click="submitPromote">Promote</button>
          <button class="btn" @click="closePromote">Cancel</button>
        </div>
      </li>
    </ul>

    <div v-if="showEstablish" class="establish-panel" data-testid="establish-panel">
      <h3>Establish Outpost</h3>
      <input v-model="newOutpostName" placeholder="Outpost name" data-testid="outpost-name-input" />
      <p v-if="targetsLoading" class="hint">Loading targets…</p>
      <ul v-else class="target-list" data-testid="target-list">
        <li
          v-for="t in targets"
          :key="t.body_id"
          class="target-row"
          :class="{ selected: selectedBodyId === t.body_id, 'out-of-range': !t.in_range }"
          :data-testid="`target-${t.body_id}`"
          @click="t.in_range && (selectedBodyId = t.body_id)"
        >
          <span class="target-name">{{ t.body_name }}</span>
          <span class="target-dist">{{ t.distance_au.toFixed(2) }} AU</span>
          <span v-if="!t.in_range" class="target-oor">out of range</span>
        </li>
      </ul>
      <div class="establish-actions">
        <button
          class="btn primary"
          data-testid="btn-confirm-establish"
          :disabled="!selectedBodyId || !newOutpostName.trim()"
          @click="submitEstablish"
        >
          Establish
        </button>
        <button class="btn" @click="closeEstablish">Cancel</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.outposts-view { display: flex; flex-direction: column; gap: 0.75rem; }
.toolbar { display: flex; align-items: center; gap: 1rem; }
.toolbar h2 { color: var(--accent); }
.colony-select { background: var(--surface-btn); border: 1px solid var(--border-strong); color: var(--text); padding: 0.3rem; }
.actions { margin-left: auto; }

.btn {
  background: var(--surface-btn);
  border: 1px solid var(--border-strong);
  border-radius: 3px;
  color: var(--text);
  padding: 0.4rem 0.75rem;
  font-family: monospace;
  font-size: 0.82rem;
  cursor: pointer;
}
.btn:hover { background: var(--surface-btn-hover); }
.btn:disabled { opacity: 0.5; cursor: not-allowed; }
.btn.primary { border-color: var(--border-accent); color: var(--accent); }
.btn.danger { border-color: var(--danger-border); color: var(--warn); }

.outpost-list { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 0.75rem; }
.outpost-card { background: var(--surface-1); border: 1px solid var(--border); border-radius: 6px; padding: 0.75rem 1rem; }
.outpost-header { display: flex; align-items: center; gap: 0.5rem; }
.outpost-header h3 { color: var(--accent); margin: 0; }
.body-tag { color: var(--text-muted); font-size: 0.78rem; }

.stats { display: grid; grid-template-columns: 100px 1fr; gap: 0.3rem 0.6rem; font-size: 0.8rem; margin: 0.5rem 0; }
.stats dt { color: var(--text-dim); }
.stats dd { color: var(--text); margin: 0; }
.pool-chip { display: inline-block; background: var(--surface-2); border: 1px solid var(--border); border-radius: 3px; padding: 0.1rem 0.4rem; font-size: 0.72rem; margin: 0.1rem 0.25rem 0.1rem 0; }

.card-actions { display: flex; gap: 0.5rem; margin-top: 0.5rem; }

.inline-form { display: flex; gap: 0.5rem; margin-top: 0.5rem; align-items: center; }
.inline-form input, .inline-form select {
  background: var(--surface-3); border: 1px solid var(--border-strong); color: var(--text); padding: 0.3rem 0.5rem; font-family: monospace; font-size: 0.8rem;
}

.establish-panel { background: var(--surface-1); border: 1px solid var(--border); border-radius: 6px; padding: 1rem; max-width: 420px; }
.establish-panel h3 { color: var(--accent); margin-top: 0; }
.establish-panel input { width: 100%; box-sizing: border-box; margin-bottom: 0.5rem; background: var(--surface-3); border: 1px solid var(--border-strong); color: var(--text); padding: 0.4rem; font-family: monospace; }

.target-list { list-style: none; padding: 0; margin: 0 0 0.75rem 0; max-height: 220px; overflow-y: auto; }
.target-row { display: flex; gap: 0.5rem; padding: 0.35rem 0.5rem; cursor: pointer; border-radius: 3px; font-size: 0.8rem; }
.target-row:hover { background: var(--surface-2); }
.target-row.selected { background: var(--accent-bg); }
.target-row.out-of-range { opacity: 0.5; cursor: not-allowed; }
.target-name { flex: 1; color: var(--text); }
.target-dist { color: var(--text-muted); }
.target-oor { color: var(--warn); font-size: 0.72rem; }

.establish-actions { display: flex; gap: 0.5rem; }

.hint { color: var(--text-faint); font-style: italic; font-size: 0.85rem; }
.err { color: var(--danger); font-size: 0.8rem; }
</style>
