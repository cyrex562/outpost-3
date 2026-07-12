<script setup lang="ts">
import { ref, watch, onMounted, computed } from 'vue'
import {
  getDifficultyKnobs,
  listCustomPresets,
  saveCustomPreset,
  deleteCustomPreset,
  type DifficultyKnob,
  type CustomPreset,
} from '@/services/tauriBridge'

interface Props {
  /**
   * When true, the panel is being shown mid-game and value changes should be
   * dispatched to the engine immediately (parent hooks the `change` event).
   * When false, the parent will batch the payload and send it on Start.
   */
  live?: boolean
}
const props = withDefaults(defineProps<Props>(), { live: false })

const emit = defineEmits<{
  change: [payload: { scalars: Record<string, number>; menaceEnabled: boolean; hazardsEnabled: boolean }]
}>()

const knobs = ref<DifficultyKnob[]>([])
const scalars = ref<Record<string, number>>({})
const menaceEnabled = ref(true)
const hazardsEnabled = ref(true)
const busy = ref(false)
const err = ref<string | null>(null)

const savedPresets = ref<CustomPreset[]>([])
const showLoad = ref(false)
const savePromptOpen = ref(false)
const saveName = ref('')

const sliderKnobs = computed(() => knobs.value.filter((k) => k.kind === 'slider'))
const presetDefaults = computed<Record<string, number>>(() => {
  const map: Record<string, number> = {}
  for (const k of knobs.value) map[k.id] = k.preset_default_at_current_preset
  return map
})

async function refreshKnobs() {
  try {
    const data = await getDifficultyKnobs()
    knobs.value = data
    // Seed values from the engine's current state.
    for (const k of data) {
      if (k.kind === 'slider') scalars.value[k.id] = k.current_value
    }
    const menace = data.find((k) => k.id === 'menace_enabled')
    const hazards = data.find((k) => k.id === 'hazards_enabled')
    if (menace) menaceEnabled.value = menace.current_value > 0.5
    if (hazards) hazardsEnabled.value = hazards.current_value > 0.5
  } catch (e) {
    err.value = e instanceof Error ? e.message : String(e)
  }
}

async function refreshPresets() {
  try {
    savedPresets.value = await listCustomPresets()
  } catch (e) {
    err.value = e instanceof Error ? e.message : String(e)
  }
}

onMounted(async () => {
  await refreshKnobs()
  await refreshPresets()
  // Emit initial payload so the parent has current values on mount.
  emitCurrent()
})

function emitCurrent() {
  emit('change', {
    scalars: { ...scalars.value },
    menaceEnabled: menaceEnabled.value,
    hazardsEnabled: hazardsEnabled.value,
  })
}

// When live, propagate every value change to the parent immediately.
watch([scalars, menaceEnabled, hazardsEnabled], emitCurrent, { deep: true })

async function commitSave() {
  const name = saveName.value.trim()
  if (!name) return
  busy.value = true
  err.value = null
  try {
    await saveCustomPreset(name, { ...scalars.value }, menaceEnabled.value, hazardsEnabled.value)
    savePromptOpen.value = false
    saveName.value = ''
    await refreshPresets()
  } catch (e) {
    err.value = e instanceof Error ? e.message : String(e)
  } finally {
    busy.value = false
  }
}

function loadPreset(p: CustomPreset) {
  scalars.value = { ...p.scalars }
  menaceEnabled.value = p.menace_enabled
  hazardsEnabled.value = p.hazards_enabled
  showLoad.value = false
  emitCurrent()
}

async function removePreset(name: string) {
  try {
    await deleteCustomPreset(name)
    await refreshPresets()
  } catch (e) {
    err.value = e instanceof Error ? e.message : String(e)
  }
}

/** Snap slider values to the anchor step list so slider clicks land on ticks. */
function snapToStep(knob: DifficultyKnob, raw: number): number {
  let closest = knob.step[0]
  let bestDiff = Math.abs(raw - closest)
  for (const s of knob.step) {
    const d = Math.abs(raw - s)
    if (d < bestDiff) {
      bestDiff = d
      closest = s
    }
  }
  return closest
}

function onSliderInput(knob: DifficultyKnob, ev: Event) {
  const target = ev.target as HTMLInputElement
  const raw = parseFloat(target.value)
  scalars.value[knob.id] = snapToStep(knob, raw)
}

function ghostPercent(knob: DifficultyKnob): number {
  const min = knob.min
  const max = knob.max
  if (max === min) return 0
  return ((knob.preset_default_at_current_preset - min) / (max - min)) * 100
}
</script>

<template>
  <div class="cdp" data-testid="custom-difficulty-panel">
    <div v-if="err" class="cdp-err">{{ err }}</div>

    <div class="cdp-rows">
      <div v-for="k in sliderKnobs" :key="k.id" class="cdp-row">
        <label :for="'cdp-' + k.id" class="cdp-label">{{ k.label }}</label>
        <div class="cdp-slider-wrap">
          <div class="cdp-ghost" :style="{ left: ghostPercent(k) + '%' }" :title="'Preset default: ' + k.preset_default_at_current_preset.toFixed(2)"></div>
          <input
            :id="'cdp-' + k.id"
            type="range"
            :min="k.min"
            :max="k.max"
            step="0.01"
            :list="'cdp-steps-' + k.id"
            :value="scalars[k.id] ?? k.current_value"
            @input="onSliderInput(k, $event)"
          />
          <datalist :id="'cdp-steps-' + k.id">
            <option v-for="s in k.step" :key="s" :value="s"></option>
          </datalist>
        </div>
        <span class="cdp-value" :data-testid="'cdp-value-' + k.id">
          {{ (scalars[k.id] ?? k.current_value).toFixed(2) }}x
        </span>
        <span class="cdp-default">(preset {{ presetDefaults[k.id]?.toFixed(2) ?? '—' }}x)</span>
      </div>

      <div class="cdp-row cdp-toggle-row">
        <label class="cdp-label" for="cdp-menace">Menace Enabled</label>
        <input id="cdp-menace" type="checkbox" v-model="menaceEnabled" />
      </div>
      <div class="cdp-row cdp-toggle-row">
        <label class="cdp-label" for="cdp-hazards">Hazards Enabled</label>
        <input id="cdp-hazards" type="checkbox" v-model="hazardsEnabled" />
      </div>
    </div>

    <div class="cdp-actions">
      <button type="button" class="cdp-btn" @click="savePromptOpen = true">Save as preset…</button>
      <button type="button" class="cdp-btn" @click="() => { showLoad = true; refreshPresets() }">Load…</button>
    </div>

    <div v-if="savePromptOpen" class="cdp-inline-prompt">
      <input v-model="saveName" placeholder="Preset name" class="cdp-input" />
      <button type="button" class="cdp-btn" :disabled="busy" @click="commitSave">Save</button>
      <button type="button" class="cdp-btn" @click="savePromptOpen = false">Cancel</button>
    </div>

    <div v-if="showLoad" class="cdp-load-modal">
      <div v-if="savedPresets.length === 0" class="cdp-hint">No saved presets.</div>
      <ul v-else class="cdp-preset-list">
        <li v-for="p in savedPresets" :key="p.name">
          <button type="button" class="cdp-btn cdp-preset-btn" @click="loadPreset(p)">{{ p.name }}</button>
          <button type="button" class="cdp-btn cdp-preset-del" title="Delete preset" @click="removePreset(p.name)">×</button>
        </li>
      </ul>
      <button type="button" class="cdp-btn" @click="showLoad = false">Close</button>
    </div>

    <p v-if="props.live" class="cdp-hint">Changes apply immediately.</p>
  </div>
</template>

<style scoped>
.cdp {
  border: 1px solid #334;
  border-radius: 4px;
  background: #0d0d15;
  padding: 0.75rem;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  font-size: 0.85rem;
}
.cdp-rows { display: flex; flex-direction: column; gap: 0.35rem; }
.cdp-row {
  display: grid;
  grid-template-columns: 12rem 1fr 3.5rem 6rem;
  align-items: center;
  gap: 0.5rem;
}
.cdp-toggle-row { grid-template-columns: 12rem auto; }
.cdp-label { color: #aab; font-size: 0.82rem; }
.cdp-slider-wrap { position: relative; }
.cdp-slider-wrap input[type="range"] { width: 100%; }
.cdp-ghost {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 2px;
  background: #557;
  opacity: 0.55;
  pointer-events: none;
  transform: translateX(-1px);
}
.cdp-value { color: #8cf; font-family: monospace; text-align: right; }
.cdp-default { color: #556; font-size: 0.72rem; font-style: italic; }

.cdp-actions { display: flex; gap: 0.4rem; margin-top: 0.35rem; }
.cdp-btn {
  background: #1a1a28;
  border: 1px solid #446;
  color: #aac;
  padding: 0.3rem 0.65rem;
  border-radius: 3px;
  cursor: pointer;
  font-family: monospace;
  font-size: 0.8rem;
}
.cdp-btn:hover:not(:disabled) { background: #22223a; }
.cdp-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.cdp-input {
  background: #0a0a12;
  border: 1px solid #334;
  border-radius: 3px;
  color: #cdd;
  padding: 0.25rem 0.4rem;
  font-family: monospace;
  font-size: 0.8rem;
  min-width: 10rem;
}
.cdp-inline-prompt { display: flex; gap: 0.35rem; align-items: center; }

.cdp-load-modal {
  border: 1px solid #446;
  padding: 0.4rem;
  border-radius: 3px;
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  max-height: 12rem;
  overflow-y: auto;
}
.cdp-preset-list { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 0.2rem; }
.cdp-preset-list li { display: flex; gap: 0.3rem; align-items: center; }
.cdp-preset-btn { flex: 1; text-align: left; }
.cdp-preset-del { padding: 0.2rem 0.5rem; color: #c88; border-color: #a55; }
.cdp-hint { color: #667; font-size: 0.75rem; font-style: italic; }
.cdp-err { color: #d66; font-size: 0.8rem; }
</style>
