<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import {
  getColonizeTargets,
  listBuildings,
  type ColonizeTarget,
  type BuildingOption,
} from '@/services/tauriBridge'
import { useGameStore } from '@/stores/game'

const router = useRouter()
const gameStore = useGameStore()

const step = ref<1 | 2 | 3 | 4>(1)
const error = ref<string | null>(null)

// Step 1: pick a body
const bodies = ref<ColonizeTarget[]>([])
const chosenBody = ref<ColonizeTarget | null>(null)

// Step 2: pick a landing site (placeholder: 5 site slots per body)
const chosenSite = ref<number | null>(null)

// Step 3: choose starting buildings
const buildings = ref<BuildingOption[]>([])
const chosenBuildings = ref<Set<string>>(new Set())
const supplyLevel = ref<'lean' | 'standard' | 'stockpile'>('standard')

// Step 4: name + population
const colonyName = ref('Alpha Base')
const startingPop = ref(100)

onMounted(async () => {
  try {
    bodies.value = await getColonizeTargets()
    buildings.value = await listBuildings()
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  }
})

const canAdvance = computed(() => {
  switch (step.value) {
    case 1:
      return chosenBody.value !== null
    case 2:
      return chosenSite.value !== null
    case 3:
      return chosenBuildings.value.size > 0
    case 4:
      return colonyName.value.trim().length > 0 && startingPop.value > 0
    default:
      return false
  }
})

function next(): void {
  if (!canAdvance.value) return
  if (step.value < 4) step.value = (step.value + 1) as 1 | 2 | 3 | 4
}

function back(): void {
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
  const events = await gameStore.sendCommand({
    kind: 'found_colony',
    name: colonyName.value.trim(),
    starting_population: startingPop.value,
  })
  if (events.length === 0) {
    error.value = 'Colony founding rejected — check engine state.'
    return
  }
  const founded = events.find((e) => e.kind === 'colony_founded') as
    | { kind: 'colony_founded'; colony_id: string }
    | undefined
  if (founded) {
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
    gameStore.selectedColonyId = founded.colony_id
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

    <!-- Step 2: site placeholder -->
    <section v-else-if="step === 2" class="panel">
      <p class="hint">
        Choose a landing site on <strong>{{ chosenBody?.body_name }}</strong>.
        (Placeholder — real surface-hex selection comes with the planet-map view.)
      </p>
      <div class="site-grid">
        <button
          v-for="i in 6"
          :key="`site-${i}`"
          class="site-card"
          :class="{ selected: chosenSite === i }"
          @click="chosenSite = i"
        >
          Site {{ i }}
          <span class="site-meta">
            biome: {{ ['tundra','desert','ocean','plains','highland','crater'][i - 1] }}
          </span>
        </button>
      </div>
    </section>

    <!-- Step 3: loadout -->
    <section v-else-if="step === 3" class="panel">
      <p class="hint">Choose starting buildings and initial supply level.</p>

      <div class="supply-row">
        <label>
          <input type="radio" v-model="supplyLevel" value="lean" />
          Lean
        </label>
        <label>
          <input type="radio" v-model="supplyLevel" value="standard" />
          Standard
        </label>
        <label>
          <input type="radio" v-model="supplyLevel" value="stockpile" />
          Stockpile
        </label>
      </div>

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
        <div>Site: <strong>{{ chosenSite }}</strong></div>
        <div>Supply: <strong>{{ supplyLevel }}</strong></div>
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

.body-grid, .site-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 0.5rem; }
.body-card, .site-card {
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
.body-card:hover, .site-card:hover { background: #1a1a2a; }
.body-card.selected, .site-card.selected { border-color: #468; background: #182030; color: #8cf; }
.body-name { font-weight: bold; }
.body-meta, .site-meta { font-size: 0.75rem; color: #667; }

.supply-row { display: flex; gap: 1rem; font-size: 0.85rem; color: #aab; }
.supply-row label { display: flex; gap: 0.3rem; align-items: center; }

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
