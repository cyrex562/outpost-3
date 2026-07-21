<script setup lang="ts">
/**
 * Installations list (navigation rework #7 phase 3) — every outpost
 * established anywhere in the system, in one place. `OutpostsView.vue`
 * already fetches the same unfiltered `listOutposts()` data but then
 * scopes its display to one colony at a time (for the establish/queue/
 * promote workflow, which needs a "current colony" context); this view is
 * the system-wide read + decommission counterpart — decommissioning
 * doesn't need a colony in context, so it's the one action kept here.
 */

import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useWorldStore } from '@/stores/worldStore'
import { useGameStore } from '@/stores/game'
import { listOutposts, type Outpost } from '@/services/tauriBridge'

const router = useRouter()
const worldStore = useWorldStore()
const gameStore = useGameStore()

const outposts = ref<Outpost[]>([])
const error = ref<string | null>(null)
const loading = ref(false)

/** Colony name lookup, falling back to the raw id if the colony isn't (yet) loaded. */
function colonyName(colonyId: string): string {
  return worldStore.world.colonies[colonyId]?.name ?? colonyId
}

async function refresh(): Promise<void> {
  loading.value = true
  error.value = null
  try {
    outposts.value = await listOutposts()
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}

onMounted(refresh)

const decommissioning = ref<string | null>(null)

async function decommission(outpostId: string): Promise<void> {
  decommissioning.value = outpostId
  try {
    const events = await gameStore.sendCommand({ kind: 'decommission_outpost', outpost_id: outpostId })
    if (events.length > 0) {
      error.value = null
      await refresh()
    } else {
      error.value = gameStore.toastMessage ?? 'Decommission rejected.'
    }
  } finally {
    decommissioning.value = null
  }
}

const sortedOutposts = computed(() => [...outposts.value].sort((a, b) => a.name.localeCompare(b.name)))

/** Navigation rework #7 phase 4: drill down into this outpost's own page. */
function openOutpost(outpostId: string): void {
  void router.push({ name: 'outpost', params: { outpostId } })
}
</script>

<template>
  <div class="installations-view" data-testid="installations-view">
    <div class="toolbar">
      <h2>Installations</h2>
      <span class="count">{{ outposts.length }} outpost{{ outposts.length === 1 ? '' : 's' }} system-wide</span>
    </div>

    <p v-if="error" class="err">{{ error }}</p>
    <p v-if="loading" class="hint">Loading…</p>
    <p v-else-if="outposts.length === 0" class="hint">No outposts established anywhere yet.</p>

    <ul class="installation-list" data-testid="installation-list">
      <li
        v-for="o in sortedOutposts"
        :key="o.id"
        class="installation-card"
        :data-testid="`installation-${o.id}`"
      >
        <div class="installation-header installation-header--link" @click="openOutpost(o.id)">
          <h3>{{ o.name }}</h3>
          <span class="body-tag">{{ o.body_name }}</span>
        </div>
        <dl class="stats">
          <dt>Colony</dt>
          <dd>{{ colonyName(o.parent_colony_id) }}</dd>
          <dt>Slots</dt>
          <dd>{{ o.slots_used }} / {{ o.slot_capacity }}</dd>
          <dt>Buildings</dt>
          <dd>{{ o.buildings.length === 0 ? 'none' : o.buildings.join(', ') }}</dd>
        </dl>
        <div class="card-actions">
          <button
            class="btn danger"
            :disabled="decommissioning === o.id"
            @click="decommission(o.id)"
          >
            {{ decommissioning === o.id ? 'Decommissioning…' : 'Decommission' }}
          </button>
        </div>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.installations-view { display: flex; flex-direction: column; gap: 0.75rem; }
.toolbar { display: flex; align-items: center; gap: 1rem; }
.toolbar h2 { color: #8cf; margin: 0; }
.count { color: #779; font-size: 0.8rem; }

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
.btn:disabled { opacity: 0.5; cursor: not-allowed; }
.btn.danger { border-color: #632; color: #d86; }

.installation-list { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 0.75rem; }
.installation-card { background: #101018; border: 1px solid #334; border-radius: 6px; padding: 0.75rem 1rem; }
.installation-header { display: flex; align-items: center; gap: 0.5rem; }
.installation-header--link { cursor: pointer; }
.installation-header--link:hover h3 { color: #adf; }
.installation-header h3 { color: #8cf; margin: 0; }
.body-tag { color: #779; font-size: 0.78rem; }

.stats { display: grid; grid-template-columns: 100px 1fr; gap: 0.3rem 0.6rem; font-size: 0.8rem; margin: 0.5rem 0; }
.stats dt { color: #668; }
.stats dd { color: #aab; margin: 0; }

.card-actions { display: flex; gap: 0.5rem; margin-top: 0.5rem; }

.hint { color: #557; font-style: italic; font-size: 0.85rem; }
.err { color: #d66; font-size: 0.8rem; }
</style>
