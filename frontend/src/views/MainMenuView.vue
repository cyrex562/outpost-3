<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue'
import { useRouter } from 'vue-router'
import {
  exitApp as tauriExit,
  isTauri,
  bootstrap,
  listSaves,
  loadGame,
  listCustomPresets,
  deleteCustomPreset,
  type CustomPreset,
} from '@/services/tauriBridge'
import { useWorldStore } from '@/stores/worldStore'
import CustomDifficultyPanel from '@/components/CustomDifficultyPanel.vue'

const router = useRouter()
const worldStore = useWorldStore()

const mode = ref<'root' | 'new' | 'load' | 'settings'>('root')
const busy = ref(false)
const error = ref<string | null>(null)

// New game form
const contentDir = ref('embedded')
const seed = ref(0)
/** Independent seed for star-system generation (issue #199) — defaults to
 * `seed` (the planet seed) server-side when omitted, but has its own
 * Randomise control so the system can be rerolled without rerolling the
 * founding planet's hex map (playtest feedback round 3). */
const systemSeed = ref<number>(Math.floor(Math.random() * 0xffffffff))
// Star-system generation tuning (playtest feedback round 3): mirrors
// NewGameView.vue's (browser-mode) sliders for the Tauri desktop New Game
// panel.
const habitableZoneCenterAu = ref<number>(1.0)
const innerPlanetCount = ref<number>(3)
const abundanceScalar = ref<number>(1.0)
// System-composition options (issue #318): gas giants/asteroid belts pick an
// exact count (sent as min=max, mirroring innerPlanetCount above); the
// cometary belt is a presence toggle (0 or 1, never a count of belts); giant
// moons is a real min/max spread (each giant in the system rolls
// independently within it) so it gets paired sliders instead of one.
const gasGiantCount = ref<number>(2)
const asteroidBeltCount = ref<number>(2)
const cometaryBeltPresent = ref<boolean>(true)
const giantMoonsMin = ref<number>(2)
const giantMoonsMax = ref<number>(20)
const rockyMoonsMax = ref<number>(3)
// Keep the two independent moon-count sliders from silently crossing —
// dragging min above max (or max below min) pulls the other bound along,
// so the pair always reads as a coherent range (the engine would normalize
// an inverted range safely, but silently swapping meaning is confusing UX).
watch(giantMoonsMin, (v) => {
  if (v > giantMoonsMax.value) giantMoonsMax.value = v
})
watch(giantMoonsMax, (v) => {
  if (v < giantMoonsMin.value) giantMoonsMin.value = v
})
// Difficulty selection: built-in preset name, "Custom", or "custom:<name>" for a saved preset.
const difficulty = ref<string>('Normal')
const savedPresets = ref<CustomPreset[]>([])

// Buffered payload from CustomDifficultyPanel (both for "Custom…" and named-custom selections).
const customPayload = ref<{
  scalars: Record<string, number>
  menaceEnabled: boolean
  hazardsEnabled: boolean
  maintenanceEnabled: boolean
} | null>(null)

const isCustomSelection = computed(
  () => difficulty.value === 'Custom' || difficulty.value.startsWith('custom:'),
)

// Load game
const availableSaves = ref<string[]>([])

async function refreshPresets() {
  if (!isTauri) return
  try {
    savedPresets.value = await listCustomPresets()
  } catch {
    savedPresets.value = []
  }
}

onMounted(refreshPresets)

// When a named custom preset is picked, seed customPayload so bootstrap sends
// the saved values (the panel will pre-populate from its own onMounted knob fetch).
watch(difficulty, () => {
  if (difficulty.value.startsWith('custom:')) {
    const name = difficulty.value.slice('custom:'.length)
    const p = savedPresets.value.find((sp) => sp.name === name)
    if (p) {
      customPayload.value = {
        scalars: { ...p.scalars },
        menaceEnabled: p.menace_enabled,
        hazardsEnabled: p.hazards_enabled,
        // Pre-#180 preset files may omit maintenance_enabled; default to on.
        maintenanceEnabled: p.maintenance_enabled ?? true,
      }
    }
  } else if (difficulty.value !== 'Custom') {
    customPayload.value = null
  }
})

async function removeSavedPreset(name: string) {
  await deleteCustomPreset(name)
  await refreshPresets()
  if (difficulty.value === `custom:${name}`) difficulty.value = 'Normal'
}

function onPanelChange(payload: typeof customPayload.value) {
  customPayload.value = payload
}

async function startNewGame(): Promise<void> {
  busy.value = true
  error.value = null
  try {
    if (!isTauri) {
      // Browser mode bootstraps through the WebSocket `new_game` flow
      // against the shared engine instead of the Tauri `bootstrap` IPC
      // command — see NewGameView.vue.
      router.push('/new-game')
      return
    }
    // For "Custom" and "custom:*" selections, pass the buffered scalars payload.
    // For built-in presets, don't send custom_scalars — the engine picks the grade table.
    const presetName = isCustomSelection.value ? 'Custom' : difficulty.value
    const cp = isCustomSelection.value ? customPayload.value : null
    const snap = await bootstrap(
      contentDir.value,
      seed.value,
      presetName,
      cp?.scalars,
      cp?.menaceEnabled,
      cp?.hazardsEnabled,
      cp?.maintenanceEnabled,
      systemSeed.value,
      {
        habitableZoneCenterAu: habitableZoneCenterAu.value,
        minInnerPlanets: innerPlanetCount.value,
        maxInnerPlanets: innerPlanetCount.value,
        abundanceScalarOverride: abundanceScalar.value,
        minGasGiants: gasGiantCount.value,
        maxGasGiants: gasGiantCount.value,
        minAsteroidBelts: asteroidBeltCount.value,
        maxAsteroidBelts: asteroidBeltCount.value,
        minCometaryBelts: cometaryBeltPresent.value ? 1 : 0,
        maxCometaryBelts: cometaryBeltPresent.value ? 1 : 0,
        minGiantMoons: giantMoonsMin.value,
        maxGiantMoons: giantMoonsMax.value,
        maxRockyMoons: rockyMoonsMax.value,
      },
    )
    worldStore.hydrate({ sol: snap.sol, month: snap.month, colonies: snap.colonies })
    router.push('/system')
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    busy.value = false
  }
}

function randomiseSeed(): void {
  seed.value = Math.floor(Math.random() * 0xffffffff)
}

function randomiseSystemSeed(): void {
  systemSeed.value = Math.floor(Math.random() * 0xffffffff)
}

async function openLoad(): Promise<void> {
  mode.value = 'load'
  error.value = null
  try {
    availableSaves.value = isTauri ? await listSaves('.') : []
  } catch {
    availableSaves.value = []
  }
}

async function loadSelected(name: string): Promise<void> {
  busy.value = true
  error.value = null
  try {
    const snap = await loadGame(name)
    worldStore.hydrate({ sol: snap.sol, month: snap.month, colonies: snap.colonies })
    router.push('/system')
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    busy.value = false
  }
}

async function exitApp(): Promise<void> {
  if (isTauri) {
    await tauriExit()
  } else {
    window.close()
  }
}
</script>

<template>
  <div class="menu-view" data-testid="main-menu">
    <h1 class="menu-title">OUTPOST 3</h1>
    <p class="menu-subtitle">Turn-based grand strategy — colony survival at star-system scale</p>

    <div v-if="mode === 'root'" class="menu-buttons">
      <button class="btn primary" data-testid="btn-new-game" @click="mode = 'new'">New Game</button>
      <button class="btn" data-testid="btn-load-game" @click="openLoad">Load Game</button>
      <button class="btn" data-testid="btn-settings" @click="mode = 'settings'">Settings</button>
      <button class="btn danger" data-testid="btn-exit" @click="exitApp">Exit</button>
    </div>

    <div v-else-if="mode === 'new'" class="panel" data-testid="new-game-panel">
      <h2>New Game</h2>
      <label>
        Content pack
        <input v-model="contentDir" class="input" placeholder="content/sample" />
      </label>
      <label>
        Planet Seed
        <div class="seed-row">
          <input v-model.number="seed" class="input" type="number" data-testid="planet-seed-input" />
          <button type="button" class="btn small" data-testid="randomise-seed" @click="randomiseSeed">
            Randomise
          </button>
        </div>
      </label>
      <label>
        Star System Seed
        <div class="seed-row">
          <input
            v-model.number="systemSeed"
            class="input"
            type="number"
            data-testid="system-seed-input"
          />
          <button
            type="button"
            class="btn small"
            data-testid="randomise-system-seed"
            @click="randomiseSystemSeed"
          >
            Randomise
          </button>
        </div>
        <span class="hint">Independent of the planet seed — reroll the system without rerolling the founding planet.</span>
      </label>
      <div class="slider-group">
        <label class="slider-row">
          <span class="slider-label">Habitable Zone Center</span>
          <input
            v-model.number="habitableZoneCenterAu"
            type="range"
            min="0.5"
            max="2.5"
            step="0.05"
            class="slider"
            data-testid="hz-center-slider"
          />
          <span class="slider-value" data-testid="hz-center-value">{{ habitableZoneCenterAu.toFixed(2) }} AU</span>
        </label>
        <label class="slider-row">
          <span class="slider-label">Inner Planets</span>
          <input
            v-model.number="innerPlanetCount"
            type="range"
            min="2"
            max="6"
            step="1"
            class="slider"
            data-testid="inner-planet-count-slider"
          />
          <span class="slider-value" data-testid="inner-planet-count-value">{{ innerPlanetCount }}</span>
        </label>
        <label class="slider-row">
          <span class="slider-label">Resource Abundance</span>
          <input
            v-model.number="abundanceScalar"
            type="range"
            min="0.3"
            max="3.0"
            step="0.1"
            class="slider"
            data-testid="abundance-slider"
          />
          <span class="slider-value" data-testid="abundance-value">{{ abundanceScalar.toFixed(1) }}x</span>
        </label>
        <label class="slider-row">
          <span class="slider-label">Gas Giants</span>
          <input
            v-model.number="gasGiantCount"
            type="range"
            min="1"
            max="4"
            step="1"
            class="slider"
            data-testid="gas-giant-count-slider"
          />
          <span class="slider-value" data-testid="gas-giant-count-value">{{ gasGiantCount }}</span>
        </label>
        <label class="slider-row">
          <span class="slider-label">Asteroid Belts</span>
          <input
            v-model.number="asteroidBeltCount"
            type="range"
            min="1"
            max="3"
            step="1"
            class="slider"
            data-testid="asteroid-belt-count-slider"
          />
          <span class="slider-value" data-testid="asteroid-belt-count-value">{{ asteroidBeltCount }}</span>
        </label>
        <label class="slider-row checkbox-row">
          <span class="slider-label">Cometary Belt</span>
          <input
            v-model="cometaryBeltPresent"
            type="checkbox"
            data-testid="cometary-belt-checkbox"
          />
        </label>
        <label class="slider-row">
          <span class="slider-label">Giant Moons (min)</span>
          <input
            v-model.number="giantMoonsMin"
            type="range"
            min="0"
            max="20"
            step="1"
            class="slider"
            data-testid="giant-moons-min-slider"
          />
          <span class="slider-value" data-testid="giant-moons-min-value">{{ giantMoonsMin }}</span>
        </label>
        <label class="slider-row">
          <span class="slider-label">Giant Moons (max)</span>
          <input
            v-model.number="giantMoonsMax"
            type="range"
            min="0"
            max="20"
            step="1"
            class="slider"
            data-testid="giant-moons-max-slider"
          />
          <span class="slider-value" data-testid="giant-moons-max-value">{{ giantMoonsMax }}</span>
        </label>
        <label class="slider-row">
          <span class="slider-label">Rocky Moons (max)</span>
          <input
            v-model.number="rockyMoonsMax"
            type="range"
            min="0"
            max="6"
            step="1"
            class="slider"
            data-testid="rocky-moons-max-slider"
          />
          <span class="slider-value" data-testid="rocky-moons-max-value">{{ rockyMoonsMax }}</span>
        </label>
      </div>
      <label>
        Difficulty
        <select v-model="difficulty" class="input" data-testid="difficulty-select">
          <option>Sandbox</option>
          <option>Easy</option>
          <option>Normal</option>
          <option>Hard</option>
          <option>Brutal</option>
          <option value="Custom">Custom…</option>
          <optgroup v-if="savedPresets.length" label="Saved">
            <option
              v-for="p in savedPresets"
              :key="p.name"
              :value="'custom:' + p.name"
            >★ {{ p.name }}</option>
          </optgroup>
        </select>
      </label>
      <div v-if="difficulty.startsWith('custom:')" class="row named-preset-row">
        <span class="hint">Selected saved preset: {{ difficulty.slice('custom:'.length) }}</span>
        <button
          type="button"
          class="btn danger small"
          title="Delete preset"
          @click="removeSavedPreset(difficulty.slice('custom:'.length))"
        >×</button>
      </div>
      <CustomDifficultyPanel
        v-if="isCustomSelection"
        :live="false"
        @change="onPanelChange"
      />
      <div class="row">
        <button class="btn primary" data-testid="btn-start" :disabled="busy" @click="startNewGame">
          {{ busy ? 'Starting…' : 'Start' }}
        </button>
        <button class="btn" @click="mode = 'root'">Back</button>
      </div>
      <p v-if="error" class="err">{{ error }}</p>
    </div>

    <div v-else-if="mode === 'load'" class="panel" data-testid="load-game-panel">
      <h2>Load Game</h2>
      <div v-if="availableSaves.length === 0" class="hint">
        No save files found.
      </div>
      <ul v-else class="save-list">
        <li v-for="name in availableSaves" :key="name">
          <button class="btn wide" :disabled="busy" @click="loadSelected(name)">{{ name }}</button>
        </li>
      </ul>
      <div class="row">
        <button class="btn" @click="mode = 'root'">Back</button>
      </div>
      <p v-if="error" class="err">{{ error }}</p>
    </div>

    <div v-else-if="mode === 'settings'" class="panel" data-testid="settings-panel">
      <h2>Settings</h2>
      <p class="hint">No settings yet.</p>
      <button class="btn" @click="mode = 'root'">Back</button>
    </div>
  </div>
</template>

<style scoped>
.menu-view {
  min-height: calc(100vh - 2rem);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 1rem;
  padding: 2rem;
}
.menu-title { color: #8cf; font-size: 2.5rem; letter-spacing: 0.3em; }
.menu-subtitle { color: #557; margin-bottom: 2rem; font-size: 0.9rem; }

.menu-buttons {
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
  min-width: 240px;
}

.panel {
  background: #101018;
  border: 1px solid #334;
  border-radius: 6px;
  padding: 1.5rem;
  min-width: 320px;
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
}
.panel h2 { color: #8cf; }
.panel label { display: flex; flex-direction: column; gap: 0.2rem; font-size: 0.8rem; color: #778; }
.input {
  background: #0d0d15;
  border: 1px solid #334;
  border-radius: 3px;
  color: #cdd;
  padding: 0.35rem 0.5rem;
  font-family: monospace;
  font-size: 0.85rem;
}

.seed-row { display: flex; gap: 0.4rem; }
.seed-row .input { flex: 1; }

.slider-group {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  border: 1px solid #223;
  border-radius: 3px;
  padding: 0.5rem;
}

.slider-row {
  display: grid;
  grid-template-columns: 9rem 1fr 4.5rem;
  align-items: center;
  gap: 0.5rem;
}

.slider-label { font-size: 0.8rem; color: #aab; }
.slider { width: 100%; accent-color: #4af; }
.slider-value { color: #8cf; font-family: monospace; font-size: 0.85rem; text-align: right; }

.btn {
  background: #1a1a28;
  border: 1px solid #446;
  border-radius: 3px;
  color: #aac;
  padding: 0.55rem 1rem;
  font-family: monospace;
  font-size: 0.9rem;
  cursor: pointer;
}
.btn:hover:not(:disabled) { background: #22223a; }
.btn:disabled { opacity: 0.45; cursor: not-allowed; }
.btn.primary { border-color: #468; color: #8cf; }
.btn.danger { border-color: #a55; color: #c88; }
.btn.wide { width: 100%; text-align: left; }

.row { display: flex; gap: 0.6rem; margin-top: 0.5rem; align-items: center; }
.named-preset-row { justify-content: space-between; }
.btn.small { padding: 0.2rem 0.5rem; font-size: 0.75rem; }
.err { color: #d66; font-size: 0.8rem; }
.hint { color: #558; font-style: italic; font-size: 0.85rem; }
.save-list { list-style: none; display: flex; flex-direction: column; gap: 0.3rem; }
</style>
