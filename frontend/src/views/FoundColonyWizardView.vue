<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import {
  getColonizeTargets,
  getPlanetMap,
  getSystemBodies,
  listBuildings,
  listSupplyPackages,
  type ColonizeTarget,
  type BuildingOption,
  type PlanetHex,
  type PlanetMap,
  type SupplyPackage,
  type SystemBody,
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
const planetMapRef = ref<InstanceType<typeof PlanetHexMap> | null>(null)

// Full system-body attribute records (from get_system_bodies), keyed by id.
// Used to render the environmental header strip in step 2. Kept separate from
// the colonize-target list because that endpoint only carries the minimum
// needed for the body-picker cards.
const systemBodies = ref<SystemBody[]>([])
const chosenBodyAttributes = computed<SystemBody | null>(() => {
  if (!chosenBody.value) return null
  return systemBodies.value.find((b) => b.id === chosenBody.value?.body_id) ?? null
})

// Step 3: population, per-commodity supplies, per-building counts (issue #167)
const buildings = ref<BuildingOption[]>([])
/**
 * Buildings selectable at founding — tech-gated buildings can't be
 * researched yet at this point in a new game, so offering them here would
 * let the player queue construction they can never actually complete
 * (issue #166).
 */
const starterBuildings = computed(() => buildings.value.filter((b) => !b.tech_prerequisite))
/** Number of each building type to queue at founding. Keyed by building id. */
const buildingCounts = ref<Record<string, number>>({})
const supplyPackages = ref<SupplyPackage[]>([])
const chosenSupplyId = ref<string | null>(null)
/**
 * Per-commodity starting-supply amount, pre-filled from the chosen preset's
 * defaults (scaled to `startingPop`) and freely editable from there — the
 * preset becomes a starting point, not a fixed tier (issue #167).
 */
const supplyAmounts = ref<Record<string, number>>({})

/**
 * Colony starting build-slot budget. Mirrors `outpost_core::colony::BASE_SLOT_CAPACITY`
 * — every new colony starts with this many slots before any tech bonuses.
 * Used only for the wizard's real-time preview; the engine is the actual
 * source of truth and independently rejects over-budget `QueueConstruction`
 * calls with `EngineError::SlotCapacityExceeded`.
 */
const BASE_SLOT_CAPACITY = 5

// Colonist headcount, seeded into the new colony's population pool.
const colonyName = ref('Alpha Base')
const startingPop = ref(100)

/** (Re)fill `supplyAmounts` from a preset's per-100-colonist defaults, scaled to `startingPop`. */
function applyPresetDefaults(presetId: string | null): void {
  const pkg = supplyPackages.value.find((p) => p.id === presetId)
  const amounts: Record<string, number> = {}
  if (pkg) {
    for (const [commodityId, per100] of pkg.commodities) {
      amounts[commodityId] = Math.round((per100 * startingPop.value) / 100)
    }
  }
  supplyAmounts.value = amounts
}

function selectPreset(presetId: string): void {
  chosenSupplyId.value = presetId
  applyPresetDefaults(presetId)
}

/** Total build slots the currently-chosen building counts would consume. */
const totalSlotCost = computed(() =>
  starterBuildings.value.reduce(
    (sum, b) => sum + (buildingCounts.value[b.id] ?? 0) * b.slot_cost,
    0,
  ),
)
/** Total labor/turn the currently-chosen building counts would consume. */
const totalLaborPerTurn = computed(() =>
  starterBuildings.value.reduce(
    (sum, b) => sum + (buildingCounts.value[b.id] ?? 0) * b.labor_per_turn,
    0,
  ),
)
const overBudget = computed(() => totalSlotCost.value > BASE_SLOT_CAPACITY)
/** Number of buildings queued (sum of all per-building counts). */
const totalBuildingCount = computed(() =>
  Object.values(buildingCounts.value).reduce((sum, n) => sum + n, 0),
)

onMounted(async () => {
  try {
    bodies.value = await getColonizeTargets()
    systemBodies.value = await getSystemBodies()
    buildings.value = await listBuildings()
    planetMap.value = await getPlanetMap()
    supplyPackages.value = await listSupplyPackages()
    // Default the supply pick to a "Standard"-named package if present, else the first.
    const std = supplyPackages.value.find((p) => p.id === 'standard' || p.name.toLowerCase() === 'standard')
    const defaultPresetId = std?.id ?? supplyPackages.value[0]?.id ?? null
    if (defaultPresetId) selectPreset(defaultPresetId)

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
      return startingPop.value > 0 && totalBuildingCount.value > 0 && !overBudget.value
    case 4:
      // Re-check overBudget here too, not just on step 3's gate — step 3
      // is the only place buildingCounts is edited today, but defending
      // the actual submit action directly means this can't silently drift
      // out of sync if that ever changes.
      return colonyName.value.trim().length > 0 && !overBudget.value
    default:
      return false
  }
})

function pickHex(hex: PlanetHex): void {
  chosenHex.value = hex
}

/**
 * Find the highest-suitability habitable + unoccupied hex, select it, and
 * pan/zoom the planet map to focus it. No-op when the map hasn't loaded or
 * no habitable site exists.
 */
function jumpToBestSite(): void {
  const map = planetMap.value
  if (!map) return
  const candidates = map.hexes
    .filter((h) => h.habitable && h.occupied_by === null)
    .sort((a, b) => b.suitability - a.suitability)
  const best = candidates[0]
  if (!best) return
  chosenHex.value = best
  planetMapRef.value?.focusSite(best.site_id)
}

function formatModifier(mod: number): string {
  const pct = (mod - 1.0) * 100
  const sign = pct >= 0 ? '+' : ''
  return `${sign}${pct.toFixed(0)}%`
}

function habitabilityTone(mod: number): 'bonus' | 'neutral' | 'penalty' {
  if (mod > 1.001) return 'bonus'
  if (mod < 0.999) return 'penalty'
  return 'neutral'
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

/** Adjust a building's queued count by `delta`, clamped to zero. */
function adjustBuildingCount(id: string, delta: number): void {
  const current = buildingCounts.value[id] ?? 0
  const next = Math.max(0, current + delta)
  buildingCounts.value = { ...buildingCounts.value, [id]: next }
}

function setBuildingCount(id: string, value: number): void {
  const next = Number.isFinite(value) ? Math.max(0, Math.floor(value)) : 0
  buildingCounts.value = { ...buildingCounts.value, [id]: next }
}

function setSupplyAmount(commodityId: string, value: number): void {
  const next = Number.isFinite(value) ? Math.max(0, value) : 0
  supplyAmounts.value = { ...supplyAmounts.value, [commodityId]: next }
}

async function finish(): Promise<void> {
  error.value = null
  const site = chosenHex.value?.site_id ?? ''
  // Only forward amounts the player actually set (>0) as overrides — zeroed
  // commodities simply aren't seeded, same as a preset that omits them.
  const supplyOverrides: [string, number][] = Object.entries(supplyAmounts.value).filter(
    ([, qty]) => qty > 0,
  )
  const events = await gameStore.sendCommand(
    site
      ? {
          kind: 'found_colony_at_site',
          name: colonyName.value.trim(),
          starting_population: startingPop.value,
          site_id: site,
          focus: null,
          supplies_id: chosenSupplyId.value,
          supply_overrides: supplyOverrides.length > 0 ? supplyOverrides : null,
          body_id: chosenBody.value?.body_id ?? null,
        }
      : {
          kind: 'found_colony',
          name: colonyName.value.trim(),
          starting_population: startingPop.value,
        },
  )
  if (events.length === 0) {
    // gameStore.toastMessage carries the actual rejection reason (e.g. the
    // habitability-threshold message from issue #183) when the command fails.
    error.value = gameStore.toastMessage ?? 'Colony founding rejected — check engine state.'
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
    // Link the new colony to its star-system body so production picks up the
    // habitability modifier (issue #163). Silent failure is fine — a colony
    // without a home body just runs at the neutral 1.0 multiplier.
    if (chosenBody.value) {
      await gameStore.sendCommand({
        kind: 'assign_colony_home_body',
        colony_id: founded.colony_id,
        body_id: chosenBody.value.body_id,
      })
    }
    // Deploy the chosen starter buildings instantly in one "lander" batch —
    // the engine places them directly into `colony.buildings` rather than the
    // multi-turn `build_queue`, so they're operational the moment the colony
    // is founded. The engine validates the whole batch (tech gates, total
    // slot cost) atomically before placing anything.
    const kitBuildings: [string, number][] = []
    for (const [bid, count] of Object.entries(buildingCounts.value)) {
      if (count <= 0) continue
      const b = buildings.value.find((x) => x.id === bid)
      if (!b) continue
      for (let i = 0; i < count; i += 1) {
        kitBuildings.push([b.id, b.slot_cost])
      }
    }
    if (kitBuildings.length > 0) {
      // sendCommand() already sets gameStore.toastMessage with the failure
      // reason (or a success summary) — no separate fallback needed here.
      await gameStore.sendCommand({
        kind: 'deploy_starter_kit',
        colony_id: founded.colony_id,
        buildings: kitBuildings,
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
          :class="{ selected: chosenBody?.body_id === b.body_id, 'below-threshold': !b.can_found }"
          :data-testid="`body-card-${b.body_id}`"
          @click="chosenBody = b"
        >
          <div class="body-name">{{ b.body_name }}</div>
          <div class="body-meta">{{ b.kind }} · {{ b.distance_au.toFixed(2) }} AU · habitability {{ b.habitability }}/100</div>
          <div v-if="!b.can_found" class="body-warning" :data-testid="`body-warning-${b.body_id}`">
            Below habitability threshold — founding here will be rejected unless a harsh-world tech is researched.
          </div>
        </button>
      </div>
      <p v-if="bodies.length === 0" class="hint">No colonizable bodies detected.</p>
    </section>

    <!-- Step 2: planet hex map -->
    <section v-else-if="step === 2" class="panel">
      <p class="hint">
        Choose a landing site on <strong>{{ chosenBody?.body_name }}</strong>.
        Scroll to zoom, drag to pan. Ocean cells are impassable; dashed rings
        mark the top-3 suitability scores; stars mark occupied hexes.
      </p>
      <div v-if="chosenBodyAttributes" class="body-strip" data-testid="body-strip">
        <div class="strip-item">
          <span class="strip-label">Atmosphere</span>
          <span class="strip-value">
            {{ chosenBodyAttributes.atmosphere_density
            }}<template v-if="chosenBodyAttributes.atmosphere_hazard !== 'None'"> · {{ chosenBodyAttributes.atmosphere_hazard }}</template>
          </span>
        </div>
        <div class="strip-item">
          <span class="strip-label">Temperature</span>
          <span class="strip-value">{{ chosenBodyAttributes.temperature }}</span>
        </div>
        <div class="strip-item">
          <span class="strip-label">Gravity</span>
          <span class="strip-value">{{ chosenBodyAttributes.gravity_g.toFixed(2) }} g</span>
        </div>
        <div class="strip-item">
          <span class="strip-label">Radiation</span>
          <span class="strip-value">{{ chosenBodyAttributes.radiation }}</span>
        </div>
        <div class="strip-item">
          <span class="strip-label">Habitability</span>
          <span class="strip-value">
            {{ chosenBodyAttributes.habitability }} / 100
            <span
              class="modifier"
              :class="habitabilityTone(chosenBodyAttributes.habitability_modifier)"
            >
              ({{ formatModifier(chosenBodyAttributes.habitability_modifier) }})
            </span>
          </span>
        </div>
        <div class="strip-spacer" />
        <button
          class="btn map-btn"
          data-testid="jump-to-best-site"
          :disabled="!planetMap"
          @click="jumpToBestSite"
        >
          Jump to best site
        </button>
      </div>
      <div class="map-layout">
        <div class="map-wrap">
          <PlanetHexMap
            v-if="planetMap"
            ref="planetMapRef"
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
      <p class="hint">Set colonist headcount, tune starting supplies, and queue starting buildings.</p>

      <h4 class="sub-title">Colonists</h4>
      <div class="pop-row">
        <input
          v-model.number="startingPop"
          type="range"
          min="10"
          max="1000"
          step="10"
          class="pop-slider"
          data-testid="population-slider"
        />
        <input
          v-model.number="startingPop"
          type="number"
          min="1"
          class="input pop-input"
          data-testid="population-input"
        />
        <span class="pop-label">colonists</span>
      </div>

      <h4 class="sub-title">Starting supplies</h4>
      <div class="preset-row" v-if="supplyPackages.length > 0">
        <span class="preset-label">Preset:</span>
        <button
          v-for="pkg in supplyPackages"
          :key="pkg.id"
          type="button"
          class="preset-chip"
          :class="{ selected: chosenSupplyId === pkg.id }"
          :data-testid="`supply-preset-${pkg.id}`"
          @click="selectPreset(pkg.id)"
        >
          {{ pkg.name }}
        </button>
        <span class="hint preset-hint">(presets pre-fill the spinners below — tweak freely from there)</span>
      </div>
      <div class="supply-spinner-grid" v-if="Object.keys(supplyAmounts).length > 0">
        <div v-for="(qty, commodityId) in supplyAmounts" :key="commodityId" class="supply-spinner">
          <span class="supply-spinner-label">{{ commodityId }}</span>
          <input
            type="number"
            min="0"
            class="input supply-spinner-input"
            :value="qty"
            :data-testid="`supply-amount-${commodityId}`"
            @change="setSupplyAmount(commodityId, ($event.target as HTMLInputElement).valueAsNumber)"
          />
        </div>
      </div>
      <div v-else class="hint">No supply packages authored in this content pack.</div>

      <h4 class="sub-title">Starting buildings</h4>
      <div class="budget-preview" data-testid="budget-preview" :class="{ over: overBudget }">
        Build slots: <strong>{{ totalSlotCost }} / {{ BASE_SLOT_CAPACITY }}</strong>
        · Labor/turn: <strong>{{ totalLaborPerTurn }}</strong>
        · Buildings queued: <strong>{{ totalBuildingCount }}</strong>
        <span v-if="overBudget" class="budget-warning">— exceeds the starter build-slot budget</span>
      </div>

      <div class="building-grid">
        <div
          v-for="b in starterBuildings"
          :key="b.id"
          class="building-card"
          :class="{ selected: (buildingCounts[b.id] ?? 0) > 0 }"
        >
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
            <div class="building-count-row">
              <button
                type="button"
                class="count-btn"
                :data-testid="`building-minus-${b.id}`"
                @click="adjustBuildingCount(b.id, -1)"
              >
                −
              </button>
              <input
                type="number"
                min="0"
                class="input count-input"
                :value="buildingCounts[b.id] ?? 0"
                :data-testid="`building-count-${b.id}`"
                @change="setBuildingCount(b.id, ($event.target as HTMLInputElement).valueAsNumber)"
              />
              <button
                type="button"
                class="count-btn"
                :data-testid="`building-plus-${b.id}`"
                @click="adjustBuildingCount(b.id, 1)"
              >
                +
              </button>
            </div>
          </div>
        </div>
        <div v-if="starterBuildings.length === 0" class="hint">
          No starter buildings available in the loaded content pack.
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
      <div class="summary">
        <div>Body: <strong>{{ chosenBody?.body_name }}</strong></div>
        <div v-if="chosenHex">
          Site:
          <strong>
            {{ chosenHex.biome }} · {{ chosenHex.terrain }} ({{ chosenHex.q }}, {{ chosenHex.r }})
          </strong>
        </div>
        <div>Colonists: <strong>{{ startingPop }}</strong></div>
        <div>
          Supply preset:
          <strong>
            {{
              supplyPackages.find((p) => p.id === chosenSupplyId)?.name ?? 'custom'
            }}
          </strong>
        </div>
        <div>Buildings queued: <strong>{{ totalBuildingCount }}</strong> ({{ totalSlotCost }} / {{ BASE_SLOT_CAPACITY }} slots)</div>
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
.body-card.below-threshold { border-color: #632; }
.body-card.below-threshold.selected { border-color: #a53; background: #241417; color: #d88; }
.body-name { font-weight: bold; }
.body-meta { font-size: 0.75rem; color: #667; }
.body-warning { font-size: 0.7rem; color: #d88; font-style: italic; margin-top: 0.15rem; }

/* Step 2 layout */
.body-strip {
  display: flex;
  align-items: center;
  gap: 1rem;
  background: #14141e;
  border: 1px solid #223;
  border-radius: 4px;
  padding: 0.5rem 0.75rem;
  margin: 0.5rem 0 0.75rem;
  font-size: 0.78rem;
  color: #aab;
  flex-wrap: wrap;
}
.strip-item { display: flex; flex-direction: column; }
.strip-label {
  color: #668;
  font-size: 0.68rem;
  letter-spacing: 0.05em;
  text-transform: uppercase;
}
.strip-value { color: #aab; font-family: monospace; }
.strip-spacer { flex: 1; }
.map-btn { margin-left: auto; }
.modifier { font-size: 0.7rem; margin-left: 0.2rem; }
.modifier.bonus { color: #6c9; }
.modifier.neutral { color: #778; }
.modifier.penalty { color: #d86; }

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

.pop-row { display: flex; align-items: center; gap: 0.6rem; }
.pop-slider { flex: 1; accent-color: #468; }
.pop-input { width: 90px; }
.pop-label { color: #667; font-size: 0.8rem; }

.preset-row { display: flex; align-items: center; gap: 0.4rem; flex-wrap: wrap; }
.preset-label { color: #667; font-size: 0.78rem; }
.preset-chip {
  background: #14141e;
  border: 1px solid #334;
  border-radius: 3px;
  color: #aab;
  padding: 0.3rem 0.6rem;
  font-family: monospace;
  font-size: 0.78rem;
  cursor: pointer;
}
.preset-chip:hover { background: #1a1a2a; }
.preset-chip.selected { border-color: #468; background: #182030; color: #8cf; }
.preset-hint { margin-left: 0.3rem; }

.supply-spinner-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 0.4rem; }
.supply-spinner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.4rem;
  background: #14141e;
  border: 1px solid #223;
  border-radius: 3px;
  padding: 0.35rem 0.5rem;
}
.supply-spinner-label { color: #aab; font-size: 0.78rem; }
.supply-spinner-input { width: 80px; }

.budget-preview {
  background: #14141e;
  border: 1px solid #334;
  border-radius: 3px;
  padding: 0.4rem 0.6rem;
  color: #aab;
  font-size: 0.8rem;
}
.budget-preview.over { border-color: #a53; color: #d88; }
.budget-warning { color: #d66; margin-left: 0.4rem; }

.building-count-row { display: flex; align-items: center; gap: 0.35rem; margin-top: 0.35rem; }
.count-btn {
  background: #1a1a28;
  border: 1px solid #446;
  border-radius: 3px;
  color: #aac;
  width: 1.6rem;
  height: 1.6rem;
  font-family: monospace;
  cursor: pointer;
}
.count-btn:hover { background: #22223a; }
.count-input { width: 60px; }

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
