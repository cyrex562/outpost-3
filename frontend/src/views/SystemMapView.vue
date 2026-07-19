<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
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
    // After the body list arrives, fit the viewBox to the data if the user
    // hasn't already customised it (persisted state overrides fit-all).
    if (!persistedLoaded.value) resetView()
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  }
})

// ── World-space layout math ──────────────────────────────────────────────
//
// The SVG uses viewBox coordinates (world units); actual pixel size comes
// from the container's width + the user-adjustable `mapHeight`. Bodies are
// placed at their orbital distance from the origin (the star at 0,0) using
// a deterministic id-hashed angle so the layout is stable across reloads.

const WORLD_SCALE = 100 // 1 AU = 100 world units

/** Deterministic angle per body (radians) so orbits don't reshuffle. */
function angleFor(id: string): number {
  let h = 0
  for (let i = 0; i < id.length; i++) h = (h * 31 + id.charCodeAt(i)) | 0
  return ((h >>> 0) % 360) * (Math.PI / 180)
}

function bodyPos(b: SystemBody): { x: number; y: number } {
  const r = b.distance_au * WORLD_SCALE
  const a = angleFor(b.id)
  return { x: r * Math.cos(a), y: r * Math.sin(a) }
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

/** `"FoodYield"` -> `"Food Yield"` — category_modifiers ships Rust Debug-formatted enum names (issue #184). */
function formatYieldCategory(category: string): string {
  return category.replace(/([a-z])([A-Z])/g, '$1 $2')
}

const orbitRadii = computed(() =>
  bodies.value.map((b) => b.distance_au * WORLD_SCALE),
)

// ── Persistence ──────────────────────────────────────────────────────────

interface Persisted {
  x: number
  y: number
  w: number
  h: number
  panelHeight: number
}

const STORAGE_KEY = 'outpost3.system-map.layout'
const persistedLoaded = ref(false)

function loadPersisted(): Persisted | null {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY)
    if (!raw) return null
    const p = JSON.parse(raw) as Persisted
    if (
      typeof p.x === 'number' &&
      typeof p.y === 'number' &&
      typeof p.w === 'number' &&
      typeof p.h === 'number' &&
      typeof p.panelHeight === 'number'
    ) {
      return p
    }
  } catch {
    // corrupt entry — ignore and re-fit
  }
  return null
}

function savePersisted(): void {
  try {
    const payload: Persisted = {
      x: viewBox.value.x,
      y: viewBox.value.y,
      w: viewBox.value.w,
      h: viewBox.value.h,
      panelHeight: mapHeight.value,
    }
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(payload))
  } catch {
    // storage full or blocked — non-fatal
  }
}

// ── ViewBox state (pan + zoom) ───────────────────────────────────────────

const DEFAULT_VIEW = { x: -600, y: -600, w: 1200, h: 1200 }
const viewBox = ref({ ...DEFAULT_VIEW })

const mapHeight = ref(720)

const persisted = loadPersisted()
if (persisted) {
  viewBox.value = { x: persisted.x, y: persisted.y, w: persisted.w, h: persisted.h }
  mapHeight.value = persisted.panelHeight
  persistedLoaded.value = true
}

const viewBoxStr = computed(
  () => `${viewBox.value.x} ${viewBox.value.y} ${viewBox.value.w} ${viewBox.value.h}`,
)

/**
 * Recompute the viewBox so every body + a padded margin fits. Also clears
 * the persisted entry so the next mount will re-fit if bodies change.
 */
function resetView(): void {
  const maxAu = bodies.value.length
    ? Math.max(1, ...bodies.value.map((b) => b.distance_au))
    : 5
  const worldMax = maxAu * WORLD_SCALE * 1.2
  viewBox.value = { x: -worldMax, y: -worldMax, w: worldMax * 2, h: worldMax * 2 }
  savePersisted()
}

// Track the SVG element so wheel/mouse handlers can compute cursor
// coordinates against its bounding rect.
const svgRef = ref<SVGSVGElement | null>(null)

const ZOOM_STEP = 1.15
const ZOOM_STEP_INV = 1 / ZOOM_STEP
const MIN_VIEW = 80 // don't allow zooming past 80 world units wide
const MAX_VIEW = 20000 // or further out than 20 000

function onWheel(e: WheelEvent): void {
  e.preventDefault()
  const svg = svgRef.value
  if (!svg) return
  const rect = svg.getBoundingClientRect()
  const factor = e.deltaY > 0 ? ZOOM_STEP : ZOOM_STEP_INV
  const newW = viewBox.value.w * factor
  const newH = viewBox.value.h * factor
  // Clamp so we don't over-zoom.
  if (newW < MIN_VIEW || newW > MAX_VIEW) return
  const mouseRatioX = (e.clientX - rect.left) / rect.width
  const mouseRatioY = (e.clientY - rect.top) / rect.height
  viewBox.value = {
    x: viewBox.value.x - (newW - viewBox.value.w) * mouseRatioX,
    y: viewBox.value.y - (newH - viewBox.value.h) * mouseRatioY,
    w: newW,
    h: newH,
  }
  savePersisted()
}

// ── Pan drag ─────────────────────────────────────────────────────────────

let isDragging = false
let dragStartX = 0
let dragStartY = 0
let dragStartVbX = 0
let dragStartVbY = 0

function onMouseDown(e: MouseEvent): void {
  // Only left-drag pans; ignore body clicks (they're handled by <g @click>).
  if (e.button !== 0) return
  isDragging = true
  dragStartX = e.clientX
  dragStartY = e.clientY
  dragStartVbX = viewBox.value.x
  dragStartVbY = viewBox.value.y
}

function onMouseMove(e: MouseEvent): void {
  if (!isDragging) return
  const svg = svgRef.value
  if (!svg) return
  const rect = svg.getBoundingClientRect()
  const scaleX = viewBox.value.w / rect.width
  const scaleY = viewBox.value.h / rect.height
  viewBox.value = {
    ...viewBox.value,
    x: dragStartVbX - (e.clientX - dragStartX) * scaleX,
    y: dragStartVbY - (e.clientY - dragStartY) * scaleY,
  }
}

function onMouseUp(): void {
  if (isDragging) {
    isDragging = false
    savePersisted()
  }
}

// ── Panel resize (bottom-right grip drags height only) ───────────────────

const MIN_PANEL_H = 320
const MAX_PANEL_H = 1400
let isResizing = false
let resizeStartY = 0
let resizeStartH = 0

function onResizeStart(e: MouseEvent): void {
  isResizing = true
  resizeStartY = e.clientY
  resizeStartH = mapHeight.value
  e.preventDefault()
}

function onResizeMove(e: MouseEvent): void {
  if (!isResizing) return
  const delta = e.clientY - resizeStartY
  const next = Math.min(MAX_PANEL_H, Math.max(MIN_PANEL_H, resizeStartH + delta))
  mapHeight.value = next
}

function onResizeEnd(): void {
  if (isResizing) {
    isResizing = false
    savePersisted()
  }
}

// Global mouse listeners keep pan + resize drags working even when the
// cursor leaves the SVG bounds mid-gesture (matches harsh_realm's UX).
onMounted(() => {
  window.addEventListener('mousemove', onMouseMove)
  window.addEventListener('mouseup', onMouseUp)
  window.addEventListener('mousemove', onResizeMove)
  window.addEventListener('mouseup', onResizeEnd)
})
onUnmounted(() => {
  window.removeEventListener('mousemove', onMouseMove)
  window.removeEventListener('mouseup', onMouseUp)
  window.removeEventListener('mousemove', onResizeMove)
  window.removeEventListener('mouseup', onResizeEnd)
})

// Also persist whenever the user selects a different body — not strictly
// required, but keeps the layout write cadence low without silent drops.
watch(mapHeight, savePersisted)

// ── Zoom-aware sizing ────────────────────────────────────────────────────

/**
 * A label at world-size N appears at N * (viewBox.w / pixelWidth) pixels.
 * To keep labels around a target pixel size, scale the world font size
 * inversely with zoom. We derive the "zoom" ratio from viewBox width vs
 * the default view width so labels stay legible across the zoom range.
 */
const labelScale = computed(() => viewBox.value.w / DEFAULT_VIEW.w)
const labelFontSize = computed(() => Math.max(6, Math.min(18, 10 * labelScale.value)))
const labelOffsetY = computed(() => 12 * labelScale.value)
const strokeScale = computed(() => Math.max(0.5, Math.min(2, 1 * labelScale.value)))

// ── Navigation ───────────────────────────────────────────────────────────

function goToColony(): void {
  router.push('/colony')
}

function foundColony(body?: SystemBody | null): void {
  if (body) {
    router.push({ path: '/found', query: { body: body.id } })
  } else {
    router.push('/found')
  }
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
        <button class="btn" data-testid="btn-reset-view" @click="resetView">
          Reset View
        </button>
        <button class="btn" @click="foundColony()">Found Colony</button>
        <button class="btn" @click="goToColony">Colony Dashboard</button>
      </div>
    </div>

    <div class="content">
      <div class="map-wrap" :style="{ height: `${mapHeight}px` }">
        <svg
          ref="svgRef"
          :viewBox="viewBoxStr"
          preserveAspectRatio="xMidYMid meet"
          class="map"
          :class="{ dragging: isDragging }"
          data-testid="system-map-svg"
          @wheel.passive="false"
          @wheel="onWheel"
          @mousedown="onMouseDown"
        >
          <!-- Star at origin -->
          <circle
            cx="0"
            cy="0"
            r="14"
            fill="#fda"
            stroke="#ffd"
            stroke-width="1"
          />
          <text
            x="0"
            :y="-(14 + labelOffsetY)"
            text-anchor="middle"
            fill="#aac"
            :font-size="labelFontSize"
            font-family="monospace"
            class="body-label"
          >
            KEPLER
          </text>

          <!-- Orbit tracks -->
          <circle
            v-for="(r, i) in orbitRadii"
            :key="`orbit-${i}`"
            cx="0"
            cy="0"
            :r="r"
            fill="none"
            stroke="#334"
            :stroke-width="strokeScale"
            :stroke-dasharray="`${2 * strokeScale} ${3 * strokeScale}`"
          />

          <!-- Bodies -->
          <g
            v-for="b in bodies"
            :key="b.id"
            class="body-group"
            :class="{ selected: selected?.id === b.id }"
            @click.stop="selected = b"
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
              :y="bodyPos(b).y + bodyRadius(b) + labelOffsetY"
              text-anchor="middle"
              fill="#aac"
              :font-size="labelFontSize"
              font-family="monospace"
              class="body-label"
            >
              {{ b.name }}
            </text>
          </g>
        </svg>
        <!-- Bottom-right corner grip for panel height resize. -->
        <div
          class="resize-grip"
          data-testid="map-resize-grip"
          :class="{ active: isResizing }"
          @mousedown="onResizeStart"
        />
      </div>

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
          <dt>Atmosphere</dt>
          <dd>{{ selected.atmosphere_density }}<template v-if="selected.atmosphere_hazard !== 'None'"> · {{ selected.atmosphere_hazard }}</template></dd>
          <dt>Temperature</dt>
          <dd>{{ selected.temperature }}</dd>
          <dt>Gravity</dt>
          <dd>{{ selected.gravity_g.toFixed(2) }} g</dd>
          <dt>Radiation</dt>
          <dd>{{ selected.radiation }}</dd>
          <dt>Habitability</dt>
          <dd data-testid="habitability-value">
            <template v-if="selected.habitability_effective !== selected.habitability">
              {{ selected.habitability }} → <strong>{{ selected.habitability_effective }}</strong> / 100
            </template>
            <template v-else>
              {{ selected.habitability }} / 100
            </template>
            <span class="modifier" :class="habitabilityTone(selected.habitability_modifier_effective)">
              ({{ formatModifier(selected.habitability_modifier_effective) }} productivity)
            </span>
            <span v-if="selected.habitability_effective !== selected.habitability" class="mitigation-hint">
              tech mitigation applied
            </span>
          </dd>
          <template v-if="selected.category_modifiers.length > 0">
            <dt>Yield modifiers</dt>
            <dd data-testid="category-modifiers">
              <span
                v-for="[category, multiplier] in selected.category_modifiers"
                :key="category"
                class="modifier-chip"
                :class="habitabilityTone(multiplier)"
              >
                {{ formatYieldCategory(category) }} {{ formatModifier(multiplier) }}
              </span>
            </dd>
          </template>
          <dt>Subtype</dt>
          <dd>{{ selected.subtype }}</dd>
          <dt v-if="selected.parent_body_name">Orbits</dt>
          <dd v-if="selected.parent_body_name">{{ selected.parent_body_name }}</dd>
          <dt>Rotation</dt>
          <dd>
            {{ selected.tidally_locked ? 'Tidally locked' : `${selected.rotation_period_hours.toFixed(1)}h period` }},
            {{ selected.axial_tilt_deg.toFixed(1) }}° tilt
          </dd>
          <dt v-if="selected.moon_count > 0">Moons</dt>
          <dd v-if="selected.moon_count > 0">{{ selected.moon_count }}</dd>
        </dl>
        <button
          v-if="selected.colonizable"
          class="btn primary"
          @click="foundColony(selected)"
        >
          Found Colony Here
        </button>
      </aside>
      <aside v-else class="side-panel hint">
        Scroll to zoom, drag to pan, corner grip to resize.<br />
        Click a body to inspect.
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

.map-wrap {
  position: relative;
  flex: 1;
  min-width: 0;
  border: 1px solid #223;
  border-radius: 6px;
  overflow: hidden;
  background: radial-gradient(ellipse at center, #05050b 0%, #000 80%);
}
.map {
  display: block;
  width: 100%;
  height: 100%;
  cursor: grab;
  user-select: none;
  touch-action: none;
}
.map.dragging { cursor: grabbing; }

.resize-grip {
  position: absolute;
  right: 2px;
  bottom: 2px;
  width: 14px;
  height: 14px;
  cursor: nwse-resize;
  background: linear-gradient(
    135deg,
    transparent 45%,
    #446 45% 55%,
    transparent 55% 65%,
    #446 65% 75%,
    transparent 75%
  );
  border-radius: 2px;
  z-index: 2;
}
.resize-grip:hover, .resize-grip.active { filter: brightness(1.5); }

.body-label {
  pointer-events: none;
}

.body-group { cursor: pointer; }
.body-group.selected circle { stroke: #8cf; stroke-width: 2; }

.side-panel {
  min-width: 220px;
  max-width: 260px;
  background: #101018;
  border: 1px solid #334;
  border-radius: 6px;
  padding: 1rem;
  color: #aab;
}
.side-panel h3 { color: #8cf; margin-bottom: 0.5rem; }
.side-panel.hint { color: #557; font-style: italic; font-size: 0.85rem; }

.stats { display: grid; grid-template-columns: 100px 1fr; gap: 0.35rem 0.6rem; font-size: 0.8rem; margin-bottom: 0.75rem; }
.stats dt { color: #668; }
.stats dd { color: #aab; }
.modifier { font-size: 0.75rem; margin-left: 0.25rem; }
.modifier.bonus { color: #6c9; }
.modifier.neutral { color: #778; }
.modifier.penalty { color: #d86; }
.mitigation-hint { display: block; font-size: 0.68rem; color: #6c9; font-style: italic; margin-top: 0.1rem; }
.modifier-chip {
  display: inline-block;
  background: #14141e;
  border: 1px solid #334;
  border-radius: 3px;
  padding: 0.1rem 0.4rem;
  font-size: 0.7rem;
  margin: 0.1rem 0.25rem 0.1rem 0;
}
.modifier-chip.bonus { border-color: #365; color: #6c9; }
.modifier-chip.neutral { border-color: #334; color: #778; }
.modifier-chip.penalty { border-color: #632; color: #d86; }
.err { color: #d66; font-size: 0.8rem; }
</style>
