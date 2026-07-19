<script setup lang="ts">
/**
 * New Game screen — lets the player choose difficulty and seed a planet before
 * the first sol is advanced.  Sends `ClientCommand::NewGame` over the WebSocket
 * and transitions to the colony view on success.
 */
import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useGameSocket } from '@/composables/useGameSocket'
import { useWorldStore } from '@/stores/worldStore'
import type { DifficultyPreset } from '@/types/api'

const DIFFICULTIES: { value: DifficultyPreset; label: string; description: string }[] = [
  { value: 'Sandbox', label: 'Sandbox', description: 'No penalties — free exploration.' },
  { value: 'Easy',    label: 'Easy',    description: 'Mild penalties, forgiving for new players.' },
  { value: 'Normal',  label: 'Normal',  description: 'Balanced challenge — the intended experience.' },
  { value: 'Hard',    label: 'Hard',    description: 'Significant penalties for experienced players.' },
  { value: 'Brutal',  label: 'Brutal',  description: 'Maximum pressure — near-unforgiving.' },
]

const router = useRouter()
const { send } = useGameSocket()
const store = useWorldStore()

const selectedDifficulty = ref<DifficultyPreset>('Normal')
const planetSeed = ref<number>(Math.floor(Math.random() * 0xffffffff))
/** Independent seed for star-system generation (issue #199) — defaults to
 * `planetSeed` server-side when left at its initial mirrored value and never
 * touched, but has its own Randomise control so the system can be rerolled
 * without rerolling the founding planet's hex map. */
const systemSeed = ref<number>(Math.floor(Math.random() * 0xffffffff))
const loading = ref(false)
const error = ref<string | null>(null)

// Star-system generation tuning (playtest feedback: expose the generator's
// tunable knobs as sliders instead of only the hardcoded defaults).
const habitableZoneCenterAu = ref<number>(1.0)
const innerPlanetCount = ref<number>(3)
const abundanceScalar = ref<number>(1.0)

const isConnected = computed(() => store.isConnected)

async function startGame() {
  if (!isConnected.value) {
    error.value = 'Not connected to server.'
    return
  }
  loading.value = true
  error.value = null
  try {
    send({
      type: 'command',
      seq: Date.now(),
      command: {
        kind: 'new_game',
        difficulty: selectedDifficulty.value,
        planet_seed: planetSeed.value,
        system_seed: systemSeed.value,
        habitable_zone_center_au: habitableZoneCenterAu.value,
        min_inner_planets: innerPlanetCount.value,
        max_inner_planets: innerPlanetCount.value,
        abundance_scalar: abundanceScalar.value,
      },
    })
    // Navigate to colony view — the store will receive new_game_snapshot from the server.
    await router.push('/colony')
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
  }
}

function randomiseSeed() {
  planetSeed.value = Math.floor(Math.random() * 0xffffffff)
}

function randomiseSystemSeed() {
  systemSeed.value = Math.floor(Math.random() * 0xffffffff)
}
</script>

<template>
  <div class="new-game">
    <div class="new-game-panel">
      <h2 class="panel-title">New Game</h2>

      <section class="field-group">
        <label class="field-label">Difficulty</label>
        <div class="difficulty-grid">
          <button
            v-for="d in DIFFICULTIES"
            :key="d.value"
            class="difficulty-btn"
            :class="{ selected: selectedDifficulty === d.value }"
            :data-testid="`difficulty-${d.value.toLowerCase()}`"
            @click="selectedDifficulty = d.value"
          >
            <span class="difficulty-name">{{ d.label }}</span>
            <span class="difficulty-desc">{{ d.description }}</span>
          </button>
        </div>
      </section>

      <section class="field-group">
        <label class="field-label">Planet Seed</label>
        <div class="seed-row">
          <input
            v-model.number="planetSeed"
            type="number"
            class="seed-input"
            data-testid="planet-seed-input"
            min="0"
            max="4294967295"
          />
          <button class="btn-secondary" data-testid="randomise-seed" @click="randomiseSeed">
            Randomise
          </button>
        </div>
      </section>

      <section class="field-group">
        <label class="field-label">Star System Seed</label>
        <div class="seed-row">
          <input
            v-model.number="systemSeed"
            type="number"
            class="seed-input"
            data-testid="system-seed-input"
            min="0"
            max="4294967295"
          />
          <button
            class="btn-secondary"
            data-testid="randomise-system-seed"
            @click="randomiseSystemSeed"
          >
            Randomise
          </button>
        </div>
        <p class="field-hint">
          Independent of the planet seed — reroll the star system without rerolling the founding planet.
        </p>
      </section>

      <section class="field-group">
        <label class="field-label">System Generation</label>
        <div class="slider-row">
          <label class="slider-label" for="hz-slider">Habitable Zone Center</label>
          <input
            id="hz-slider"
            v-model.number="habitableZoneCenterAu"
            type="range"
            min="0.5"
            max="2.5"
            step="0.05"
            class="slider"
            data-testid="hz-center-slider"
          />
          <span class="slider-value" data-testid="hz-center-value">{{ habitableZoneCenterAu.toFixed(2) }} AU</span>
        </div>
        <div class="slider-row">
          <label class="slider-label" for="planets-slider">Inner Planets</label>
          <input
            id="planets-slider"
            v-model.number="innerPlanetCount"
            type="range"
            min="2"
            max="6"
            step="1"
            class="slider"
            data-testid="inner-planet-count-slider"
          />
          <span class="slider-value" data-testid="inner-planet-count-value">{{ innerPlanetCount }}</span>
        </div>
        <div class="slider-row">
          <label class="slider-label" for="abundance-slider">Resource Abundance</label>
          <input
            id="abundance-slider"
            v-model.number="abundanceScalar"
            type="range"
            min="0.3"
            max="3.0"
            step="0.1"
            class="slider"
            data-testid="abundance-slider"
          />
          <span class="slider-value" data-testid="abundance-value">{{ abundanceScalar.toFixed(1) }}x</span>
        </div>
      </section>

      <p v-if="error" class="error-msg" data-testid="new-game-error">{{ error }}</p>

      <div class="actions">
        <button
          class="btn-primary"
          data-testid="start-game-btn"
          :disabled="!isConnected || loading"
          @click="startGame"
        >
          {{ loading ? 'Initialising…' : 'Start Game' }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.new-game {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 60vh;
}

.new-game-panel {
  background: #141420;
  border: 1px solid #334;
  border-radius: 4px;
  padding: 2rem;
  width: 100%;
  max-width: 540px;
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.panel-title {
  font-size: 1.4rem;
  color: #8cf;
  margin-bottom: 0.25rem;
}

.field-group {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.field-label {
  font-size: 0.8rem;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: #889;
}

.field-hint {
  font-size: 0.72rem;
  color: #667;
  margin: 0;
}

.slider-row {
  display: grid;
  grid-template-columns: 9rem 1fr 4.5rem;
  align-items: center;
  gap: 0.5rem;
}

.slider-label {
  font-size: 0.8rem;
  color: #aab;
}

.slider {
  width: 100%;
  accent-color: #4af;
}

.slider-value {
  color: #8cf;
  font-family: monospace;
  font-size: 0.85rem;
  text-align: right;
}

.difficulty-grid {
  display: grid;
  gap: 0.4rem;
}

.difficulty-btn {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  padding: 0.6rem 0.8rem;
  background: #1a1a28;
  border: 1px solid #334;
  border-radius: 3px;
  cursor: pointer;
  color: #cdd;
  text-align: left;
  transition: border-color 0.1s, background 0.1s;
}

.difficulty-btn:hover {
  background: #1e1e30;
  border-color: #556;
}

.difficulty-btn.selected {
  background: #1a2a3a;
  border-color: #4af;
}

.difficulty-name {
  font-weight: bold;
  font-size: 0.9rem;
}

.difficulty-desc {
  font-size: 0.75rem;
  color: #889;
  margin-top: 0.15rem;
}

.seed-row {
  display: flex;
  gap: 0.5rem;
}

.seed-input {
  flex: 1;
  background: #1a1a28;
  border: 1px solid #334;
  border-radius: 3px;
  padding: 0.4rem 0.6rem;
  color: #cdd;
  font-family: monospace;
  font-size: 0.9rem;
}

.seed-input:focus {
  outline: none;
  border-color: #4af;
}

.btn-primary {
  padding: 0.6rem 1.4rem;
  background: #1a4a7a;
  border: 1px solid #4af;
  border-radius: 3px;
  color: #cef;
  cursor: pointer;
  font-size: 0.95rem;
  transition: background 0.1s;
}

.btn-primary:hover:not(:disabled) {
  background: #1e5a8a;
}

.btn-primary:disabled {
  opacity: 0.4;
  cursor: default;
}

.btn-secondary {
  padding: 0.4rem 0.8rem;
  background: #1a1a28;
  border: 1px solid #334;
  border-radius: 3px;
  color: #cdd;
  cursor: pointer;
  font-size: 0.85rem;
}

.btn-secondary:hover {
  background: #1e1e30;
}

.actions {
  display: flex;
  justify-content: flex-end;
}

.error-msg {
  color: #f66;
  font-size: 0.85rem;
}
</style>
