<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import {
  getColonizeTargets,
  getPlanetMap,
  listBuildings,
  listSupplyPackages,
  type ColonizeTarget,
  type BuildingOption,
  type PlanetHex,
  type PlanetMap,
  type SupplyPackage,
} from '@/services/tauriBridge'
import PlanetHexMap from '@/components/PlanetHexMap.vue'
import { useGameStore } from '@/stores/game'

const router = useRouter()
const route = useRoute()
const gameStore = useGameStore()

const step = ref<1 | 2 | 3 | 4>(1)
const error = ref<string | null>(null)
/** True when the wizard was launched from the map with a body preselected. */
const bodyLocked = ref(false)

// Step 1: pick a body
const bodies = ref<ColonizeTarget[]>([])
const chosenBody = ref<ColonizeTarget | null>(null)

// Step 2: pick a landing site on the planet map
const planetMap = ref<PlanetMap | null>(null)
const chosenHex = ref<PlanetHex | null>(null)

// Step 3: choose starting buildings + supply package
const buildings = ref<BuildingOption[]>([])
const chosenBuildings = ref<Set<string>>(new Set())
const supplyPackages = ref<SupplyPackage[]>([])
const chosenSupplyId = ref<string | null>(null)

// Step 4: name + population
const colonyName = ref('Alpha Base')
const startingPop = ref(100)

onMounted(async () => {
  try {
    bodies.value = await getColonizeTargets()
    buildings.value = await listBuildings()
    planetMap.value = await getPlanetMap()
    supplyPackages.value = await listSupplyPackages()
    // Default the supply pick to a "Standard"-named package if present, else the first.
    const std = supplyPackages.value.find((p) => p.id === 'standard' || p.name.toLowerCase() === 'standard')
    chosenSupplyId.value = std?.id ?? supplyPackages.value[0]?.id ?? null

    // If the wizard was launched from a body's "Found Colony Here" button,
    // the id lands in the `body` query. Preselect and skip step 1.
    const preselectedId = typeof route.query.body === 'string' ? route.query.body : null
    if (preselectedId) {
      const match = bodies.value.find((b) => b.body_id === preselectedId)
      if (match) {
        chosenBody.value = match
        bodyLocked.value = true
        step.value = 2
      }
    }
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  }
})

const canAdvance = computed(() => {
  switch (step.value) {
    case 1:
      return chosenBody.value !== null
    case 2:
      return chosenHex.value !== null && chosenHex.value.habitable
    case 3:
      return chosenBuildings.value.size > 0
    case 4:
      return colonyName.value.trim().length > 0 && startingPop.value > 0
    default:
      return false
  }
})

function pickHex(hex: PlanetHex): void {
  chosenHex.value = hex
}

function next(): void {
  if (!canAdvance.value) return
  if (step.value < 4) step.value = (step.value + 1) as 1 | 2 | 3 | 4
}

function back(): void {
  // When the body was preselected from the star map, step 1 is skipped —
  // back-from-step-2 should return to the map, not surface a hidden step 1.
  if (bodyLocked.value && step.value === 2) {
    router.push('/system')
    return
  }
  if (step.value > 1) step.value = (step.value - 1) as 1 | 2 | 3 | 4
  else router.push('/system')
}

function toggleBuilding(id: string): void {
  const s = new Set(chosenBuildings.value)
  if (s.has(id)) s.delete(id)
  else s.add(id)
  chosenBuildings.value = s
}

async function finish(): Promise<void> {
  error.value = null
  const site = chosenHex.value?.site_id ?? ''
  const events = await gameStore.sendCommand(
    site
      ? {
          kind: 'found_colony_at_site',
          name: colonyName.value.trim(),
          starting_population: startingPop.value,
          site_id: site,
          focus: null,
          supplies_id: chosenSupplyId.value,
        }
      : {
          kind: 'found_colony',
          name: colonyName.value.trim(),
          starting_population: startingPop.value,
        },
  )
  if (events.length === 0) {
    error.value = 'Colony founding rejected — check engine state.'
    return
  }
  const founded = events.find((e) => e.kind === 'colony_founded') as
    | { kind: 'colony_founded'; colony_id: string }
    | undefined
  if (founded) {
    // Point the selection at the new colony BEFORE the construction commands
    // land, so the per-command colony_screen refresh reflects the queued
    // projects rather than a previous selection.
    gameStore.selectedColonyId = founded.colony_id
    for (const bid of chosenBuildings.value) {
      const b = buildings.value.find((x) => x.id === bid)
      if (!b) continue
      await gameStore.sendCommand({
        kind: 'queue_construction',
        colony_id: founded.colony_id,
        building_type: b.id,
        slot_cost: b.slot_cost,
        labor_per_turn: b.labor_per_turn,
        construction_cost: b.construction_cost,
        construction_turns: b.construction_turns,
      })
    }
    // If no starting buildings were queued, the selection change fires the
    // watcher which triggers a refresh. Await it explicitly here so that by
    // the time ColonyView mounts, its stockpile table has data.
    await gameStore.refreshColonyScreen(founded.colony_id)
  }
  router.push('/colony')
}
</script>

<template>
  <div class="wizard" data-testid="found-colony-wizard">
    <header class="head">
      <h2>Found Colony</h2>
      <ol class="steps">
        <li :class="{ active: step === 1, done: step > 1 }">1. Body</li>
        <li :class="{ active: step === 2, done: step > 2 }">2. Site</li>
        <li :class="{ active: step === 3, done: step > 3 }">3. Loadout</li>
        <li :class="{ active: step === 4 }">4. Founding</li>
      </ol>
    </header>

    <!-- Step 1: body -->
    <section v-if="step === 1" class="panel">
      <p class="hint">Choose the celestial body to colonize.</p>
      <div class="body-grid">
        <button
          v-for="b in bodies"
          :key="b.body_id"
          class="body-card"
          :class="{ selected: chosenBody?.body_id === b.body_id }"
          @click="chosenBody = b"
        >
          <div class="body-name">{{ b.body_name }}</div>
          <div class="body-meta">{{ b.kind }} · {{ b.distance_au.toFixed(2) }} AU</div>
        </button>
      </div>
      <p v-if="bodies.length === 0" class="hint">No colonizable bodies detected.</p>
    </section>

    <!-- Step 2: planet hex map -->
    <section v-else-if="step === 2" class="panel">
      <p class="hint">
        Choose a landing site on <strong>{{ chosenBody?.body_name }}</strong>.
        Ocean cells are impassable; dashed rings mark the top-3 suitability scores;
        stars mark occupied hexes.
      </p>
      <div class="map-layout">
        <div class="map-wrap">
          <PlanetHexMap
            v-if="planetMap"
            :map="planetMap"
            :selected-site="chosenHex?.site_id ?? null"
            :highlight-top-n="3"
            @select="pickHex"
          />
          <p v-else class="hint">Loading map…</p>
        </div>
        <aside class="site-details">
          <template v-if="chosenHex">
            <h4>Selected site</h4>
            <dl class="stats">
              <dt>Coord</dt>
              <dd>({{ chosenHex.q }}, {{ chosenHex.r }})</dd>
              <dt>Terrain</dt>
              <dd>{{ chosenHex.terrain }}</dd>
              <dt>Biome</dt>
              <dd>{{ chosenHex.biome }}</dd>
              <dt>Habitable</dt>
              <dd>{{ chosenHex.habitable ? 'yes' : 'no' }}</dd>
              <dt>Suitability</dt>
              <dd>{{ chosenHex.suitability.toFixed(1) }}</dd>
              <dt v-if="chosenHex.deposits.length">Deposits</dt>
              <dd v-if="chosenHex.deposits.length">
                <span
                  v-for="d in chosenHex.deposits"
                  :key="d.commodity_id"
                  class="deposit-chip"
                >
                  {{ d.commodity_id }} ({{ (d.richness * 100).toFixed(0) }}%)
                </span>
              </dd>
            </dl>
          </template>
          <p v-else class="hint">Click a habitable hex to select it.</p>
        </aside>
      </div>
    </section>

    <!-- Step 3: loadout -->
    <section v-else-if="step === 3" class="panel">
      <p class="hint">Choose starting buildings and a supply package.</p>

      <h4 class="sub-title">Supply package</h4>
      <div class="supply-grid" v-if="supplyPackages.length > 0">
        <label
          v-for="pkg in supplyPackages"
          :key="pkg.id"
          class="supply-card"
          :class="{ selected: chosenSupplyId === pkg.id }"
        >
          <input
            type="radio"
            name="supply-pkg"
            :checked="chosenSupplyId === pkg.id"
            @change="chosenSupplyId = pkg.id"
          />
          <div class="supply-info">
            <div class="supply-name">{{ pkg.name }}</div>
            <div class="supply-desc">{{ pkg.description || '—' }}</div>
            <div class="supply-cost">
              at {{ startingPop }} colonists:
              <span
                v-for="(c, i) in pkg.commodities"
                :key="i"
                class="cost-chip"
              >
                {{ (c[1] * startingPop / 100).toFixed(0) }} {{ c[0] }}
              </span>
            </div>
          </div>
        </label>
      </div>
      <div v-else class="hint">No supply packages authored in this content pack.</div>

      <h4 class="sub-title">Starting buildings</h4>

      <div class="building-grid">
        <label
          v-for="b in buildings"
          :key="b.id"
          class="building-card"
          :class="{ selected: chosenBuildings.has(b.id) }"
        >
          <input
            type="checkbox"
            :checked="chosenBuildings.has(b.id)"
            @change="toggleBuilding(b.id)"
          />
          <div class="building-info">
            <div class="building-name">{{ b.name }}</div>
            <div class="building-cat">{{ b.category }}</div>
            <div class="building-desc">{{ b.description || '—' }}</div>
            <div class="building-stats">
              {{ b.construction_turns }} sols · {{ b.labor_per_turn }} labor/turn · {{ b.slot_cost }} slot{{ b.slot_cost === 1 ? '' : 's' }}
            </div>
            <div v-if="b.construction_cost.length" class="building-cost">
              cost:
              <span v-for="(c, i) in b.construction_cost" :key="i" class="cost-chip">
                {{ c[1] }} {{ c[0] }}
              </span>
            </div>
            <div v-if="b.tech_prerequisite" class="building-tech">
              requires: {{ b.tech_prerequisite }}
            </div>
          </div>
        </label>
        <div v-if="buildings.length === 0" class="hint">
          No buildings available in the loaded content pack.
        </div>
      </div>
    </section>

    <!-- Step 4: name -->
    <section v-else-if="step === 4" class="panel">
      <p class="hint">Confirm colony details and found the colony.</p>
      <label class="field">
        Name
        <input v-model="colonyName" class="input" />
      </label>
      <label class="field">
        Starting population
        <input v-model.number="startingPop" class="input" type="number" min="1" />
      </label>
      <div class="summary">
        <div>Body: <strong>{{ chosenBody?.body_name }}</strong></div>
        <div v-if="chosenHex">
          Site:
          <strong>
            {{ chosenHex.biome }} · {{ chosenHex.terrain }} ({{ chosenHex.q }}, {{ chosenHex.r }})
          </strong>
        </div>
        <div>
          Supply:
          <strong>
            {{
              supplyPackages.find((p) => p.id === chosenSupplyId)?.name ?? 'none'
            }}
          </strong>
        </div>
        <div>Buildings: <strong>{{ chosenBuildings.size }}</strong></div>
      </div>
    </section>

    <footer class="foot">
      <button class="btn" @click="back">
        {{ step === 1 ? 'Cancel' : 'Back' }}
      </button>
      <button v-if="step < 4" class="btn primary" :disabled="!canAdvance" @click="next">
        Next
      </button>
      <button
        v-else
        class="btn primary"
        :disabled="!canAdvance || gameStore.busy"
        @click="finish"
      >
        {{ gameStore.busy ? 'Founding…' : 'Found Colony' }}
      </button>
    </footer>
    <p v-if="error" class="err">{{ error }}</p>
  </div>
</template>

<style scoped>
.wizard {
  max-width: 900px;
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}
.head { display: flex; flex-direction: column; gap: 0.5rem; }
.head h2 { color: #8cf; }

.steps {
  display: flex;
  gap: 0.5rem;
  list-style: none;
  padding: 0;
}
.steps li {
  padding: 0.3rem 0.6rem;
  background: #1a1a24;
  border: 1px solid #223;
  border-radius: 3px;
  color: #557;
  font-size: 0.78rem;
}
.steps li.active { color: #8cf; border-color: #446; }
.steps li.done { color: #6a8; border-color: #365; }

.panel {
  background: #101018;
  border: 1px solid #223;
  border-radius: 6px;
  padding: 1.25rem;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}
.hint { color: #667; font-size: 0.85rem; }

.body-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 0.5rem; }
.body-card {
  background: #14141e;
  border: 1px solid #334;
  border-radius: 4px;
  padding: 0.6rem;
  color: #aab;
  cursor: pointer;
  text-align: left;
  font-family: monospace;
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
}
.body-card:hover { background: #1a1a2a; }
.body-card.selected { border-color: #468; background: #182030; color: #8cf; }
.body-name { font-weight: bold; }
.body-meta { font-size: 0.75rem; color: #667; }

/* Step 2 layout */
.map-layout {
  display: grid;
  grid-template-columns: minmax(300px, 1fr) 220px;
  gap: 0.75rem;
  align-items: stretch;
  min-height: 480px;
}
.map-wrap { min-width: 0; }
.site-details {
  background: #14141e;
  border: 1px solid #223;
  border-radius: 4px;
  padding: 0.75rem;
  color: #aab;
}
.site-details h4 { color: #8cf; margin-bottom: 0.5rem; }
.stats { display: grid; grid-template-columns: 80px 1fr; gap: 0.3rem 0.5rem; font-size: 0.8rem; }
.stats dt { color: #668; }
.stats dd { color: #aab; }
.deposit-chip {
  display: inline-block;
  background: #1a1a2a;
  border: 1px solid #443;
  border-radius: 2px;
  padding: 0.05rem 0.3rem;
  color: #ca8;
  font-size: 0.72rem;
  margin-right: 0.25rem;
  margin-top: 0.15rem;
}

.sub-title {
  color: #668;
  font-size: 0.78rem;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  margin-top: 0.75rem;
}

.supply-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: 0.5rem;
}
.supply-card {
  display: flex;
  gap: 0.5rem;
  align-items: flex-start;
  background: #14141e;
  border: 1px solid #334;
  border-radius: 4px;
  padding: 0.5rem;
  cursor: pointer;
  color: #aab;
}
.supply-card:hover { background: #1a1a2a; }
.supply-card.selected { border-color: #468; background: #182030; }
.supply-info { display: flex; flex-direction: column; min-width: 0; }
.supply-name { color: #8cf; font-size: 0.85rem; font-weight: bold; }
.supply-desc { color: #778; font-size: 0.75rem; margin-top: 0.15rem; }
.supply-cost {
  color: #668;
  font-size: 0.72rem;
  margin-top: 0.35rem;
  display: flex;
  gap: 0.25rem;
  flex-wrap: wrap;
}

.building-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(240px, 1fr)); gap: 0.5rem; }
.building-card {
  display: flex;
  gap: 0.5rem;
  align-items: flex-start;
  background: #14141e;
  border: 1px solid #334;
  border-radius: 4px;
  padding: 0.5rem;
  cursor: pointer;
  color: #aab;
}
.building-card:hover { background: #1a1a2a; }
.building-card.selected { border-color: #468; background: #182030; }
.building-info { display: flex; flex-direction: column; }
.building-name { color: #8cf; font-size: 0.85rem; }
.building-cat { color: #557; font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.05em; }
.building-desc { color: #778; font-size: 0.75rem; margin-top: 0.15rem; }
.building-stats { color: #668; font-size: 0.72rem; margin-top: 0.25rem; }
.building-cost { color: #668; font-size: 0.72rem; margin-top: 0.15rem; display: flex; gap: 0.35rem; flex-wrap: wrap; }
.cost-chip {
  background: #1a1a2a;
  border: 1px solid #223;
  border-radius: 2px;
  padding: 0.05rem 0.3rem;
  color: #8a8;
}
.building-tech { color: #a86; font-size: 0.72rem; margin-top: 0.15rem; }

.field { display: flex; flex-direction: column; font-size: 0.8rem; color: #667; gap: 0.2rem; }
.input {
  background: #0d0d15;
  border: 1px solid #334;
  border-radius: 3px;
  color: #cdd;
  padding: 0.35rem 0.5rem;
  font-family: monospace;
  font-size: 0.85rem;
}
.summary { color: #aab; font-size: 0.85rem; display: flex; flex-direction: column; gap: 0.15rem; }

.foot { display: flex; justify-content: space-between; gap: 0.5rem; }
.btn {
  background: #1a1a28;
  border: 1px solid #446;
  border-radius: 3px;
  color: #aac;
  padding: 0.55rem 1rem;
  font-family: monospace;
  font-size: 0.82rem;
  cursor: pointer;
}
.btn:hover:not(:disabled) { background: #22223a; }
.btn:disabled { opacity: 0.45; cursor: not-allowed; }
.btn.primary { border-color: #468; color: #8cf; }
.err { color: #d66; font-size: 0.85rem; }
</style>
