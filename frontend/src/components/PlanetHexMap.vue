<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import type { PlanetHex, PlanetMap } from '@/services/tauriBridge'

const props = defineProps<{
  map: PlanetMap
  selectedSite: string | null
  highlightTopN?: number
}>()
const emit = defineEmits<{
  (e: 'select', hex: PlanetHex): void
}>()

// ── Layout ─────────────────────────────────────────────────────────────────

const HEX_SIZE = 22 // radius from center to a vertex
const SQRT3 = Math.sqrt(3)

/**
 * Convert axial (q, r) to pixel (x, y) for pointy-top hex layout.
 * Origin (0,0) sits at pixel (offsetX, offsetY).
 */
function axialToPixel(q: number, r: number): { x: number; y: number } {
  return {
    x: HEX_SIZE * (SQRT3 * q + (SQRT3 / 2) * r),
    y: HEX_SIZE * ((3 / 2) * r),
  }
}

interface Positioned extends PlanetHex {
  cx: number
  cy: number
}

const positioned = computed<Positioned[]>(() =>
  props.map.hexes.map((h) => {
    const { x, y } = axialToPixel(h.q, h.r)
    return { ...h, cx: x, cy: y }
  }),
)

const contentBounds = computed(() => {
  let minX = Infinity
  let minY = Infinity
  let maxX = -Infinity
  let maxY = -Infinity
  for (const h of positioned.value) {
    if (h.cx < minX) minX = h.cx
    if (h.cy < minY) minY = h.cy
    if (h.cx > maxX) maxX = h.cx
    if (h.cy > maxY) maxY = h.cy
  }
  if (!isFinite(minX)) {
    return { x: -HEX_SIZE, y: -HEX_SIZE, w: HEX_SIZE * 2, h: HEX_SIZE * 2 }
  }
  const pad = HEX_SIZE + 8
  return {
    x: minX - pad,
    y: minY - pad,
    w: maxX - minX + pad * 2,
    h: maxY - minY + pad * 2,
  }
})

// ── ViewBox pan/zoom state ─────────────────────────────────────────────────

interface ViewBox {
  x: number
  y: number
  w: number
  h: number
}

const viewBox = ref<ViewBox>({ x: -1, y: -1, w: 2, h: 2 })

function fitToContent(): void {
  const b = contentBounds.value
  viewBox.value = { x: b.x, y: b.y, w: b.w, h: b.h }
}

// Fit on mount and whenever the underlying map changes (new body picked).
onMounted(fitToContent)
watch(() => props.map, fitToContent)

const viewBoxStr = computed(
  () => `${viewBox.value.x} ${viewBox.value.y} ${viewBox.value.w} ${viewBox.value.h}`,
)

const svgRef = ref<SVGSVGElement | null>(null)

const ZOOM_STEP = 1.15
const ZOOM_STEP_INV = 1 / ZOOM_STEP
const MIN_VIEW_W = 80
const MAX_VIEW_W = 5000

function onWheel(e: WheelEvent): void {
  e.preventDefault()
  const svg = svgRef.value
  if (!svg) return
  const rect = svg.getBoundingClientRect()
  const factor = e.deltaY > 0 ? ZOOM_STEP : ZOOM_STEP_INV
  const newW = viewBox.value.w * factor
  const newH = viewBox.value.h * factor
  if (newW < MIN_VIEW_W || newW > MAX_VIEW_W) return
  const mouseRatioX = (e.clientX - rect.left) / rect.width
  const mouseRatioY = (e.clientY - rect.top) / rect.height
  viewBox.value = {
    x: viewBox.value.x - (newW - viewBox.value.w) * mouseRatioX,
    y: viewBox.value.y - (newH - viewBox.value.h) * mouseRatioY,
    w: newW,
    h: newH,
  }
}

let isDragging = false
let dragStartX = 0
let dragStartY = 0
let dragStartVbX = 0
let dragStartVbY = 0
let dragMoved = false

function onMouseDown(e: MouseEvent): void {
  if (e.button !== 0) return
  isDragging = true
  dragMoved = false
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
  const dx = e.clientX - dragStartX
  const dy = e.clientY - dragStartY
  if (Math.abs(dx) > 2 || Math.abs(dy) > 2) dragMoved = true
  viewBox.value = {
    ...viewBox.value,
    x: dragStartVbX - dx * scaleX,
    y: dragStartVbY - dy * scaleY,
  }
}

function onMouseUp(): void {
  isDragging = false
}

onMounted(() => {
  window.addEventListener('mousemove', onMouseMove)
  window.addEventListener('mouseup', onMouseUp)
})
onUnmounted(() => {
  window.removeEventListener('mousemove', onMouseMove)
  window.removeEventListener('mouseup', onMouseUp)
})

// ── Rendering helpers ─────────────────────────────────────────────────────

function hexPoints(cx: number, cy: number): string {
  const pts: string[] = []
  for (let i = 0; i < 6; i++) {
    const angle = (Math.PI / 3) * i + Math.PI / 6 // pointy-top offset
    const px = cx + HEX_SIZE * Math.cos(angle)
    const py = cy + HEX_SIZE * Math.sin(angle)
    pts.push(`${px.toFixed(2)},${py.toFixed(2)}`)
  }
  return pts.join(' ')
}

// Biome color palette
const BIOME_COLOR: Record<string, string> = {
  Desert: '#c9a566',
  Tundra: '#a9c0c8',
  Polar: '#e2ecef',
  Forest: '#3d7a3d',
  Jungle: '#276b28',
  Grassland: '#8ab558',
  Barren: '#8a7f6a',
  Ocean: '#2c4665',
  Geothermal: '#b04a2a',
}

function biomeColor(hex: PlanetHex): string {
  const base = hex.habitable ? BIOME_COLOR[hex.biome] ?? '#556' : BIOME_COLOR.Ocean
  const elevated = applyElevationShading(base, hex.elevation)
  return applyTemperatureTint(elevated, hex.temperature)
}

/**
 * Modulate a hex-fill colour by elevation to create subtle relief shading.
 * Low basins darken, high peaks brighten — kept modest so the biome palette
 * still dominates the read.
 */
function applyElevationShading(hex: string, elevation: number): string {
  const rgb = parseHex(hex)
  if (!rgb) return hex
  // elevation 0.5 -> 1.0 (neutral); 0.0 -> 0.78 (dark valley); 1.0 -> 1.15 (bright peak).
  const t = Math.max(0, Math.min(1, elevation))
  const factor = 0.78 + t * 0.37
  const r = clamp255(rgb.r * factor)
  const g = clamp255(rgb.g * factor)
  const b = clamp255(rgb.b * factor)
  return `rgb(${r}, ${g}, ${b})`
}

// Per-band tint colour + blend strength. `Temperate` is the neutral
// midpoint (no tint) — `TemperatureBand`'s ordinal scale (issue #187,
// `map.rs::cell_temperature`) runs Extreme(coldest) < Frozen < Cold <
// Temperate < Hot, so `Extreme` tints toward violet-blue rather than red.
const TEMPERATURE_TINT: Record<string, { color: string; strength: number }> = {
  Extreme: { color: '#3a1fb0', strength: 0.32 },
  Frozen: { color: '#3aa0e6', strength: 0.22 },
  Cold: { color: '#8fc8e6', strength: 0.1 },
  Temperate: { color: '#8fc8e6', strength: 0 },
  Hot: { color: '#e05a2a', strength: 0.22 },
}

/**
 * Blend a subtle cool→warm tint over an already elevation-shaded colour so
 * the thermal gradient reads at a glance without a hover (issue #191).
 * Kept weak enough that biome colour and elevation shading still dominate.
 */
function applyTemperatureTint(rgbColor: string, temperature: string): string {
  const tint = TEMPERATURE_TINT[temperature]
  if (!tint || tint.strength <= 0) return rgbColor
  const base = parseRgb(rgbColor)
  const target = parseHex(tint.color)
  if (!base || !target) return rgbColor
  const t = tint.strength
  const r = clamp255(base.r + (target.r - base.r) * t)
  const g = clamp255(base.g + (target.g - base.g) * t)
  const b = clamp255(base.b + (target.b - base.b) * t)
  return `rgb(${r}, ${g}, ${b})`
}

function parseHex(hex: string): { r: number; g: number; b: number } | null {
  const m = /^#([0-9a-f]{6})$/i.exec(hex)
  if (!m) return null
  const n = parseInt(m[1], 16)
  return { r: (n >> 16) & 0xff, g: (n >> 8) & 0xff, b: n & 0xff }
}

function parseRgb(rgb: string): { r: number; g: number; b: number } | null {
  const m = /^rgb\((\d+),\s*(\d+),\s*(\d+)\)$/.exec(rgb)
  if (m) return { r: Number(m[1]), g: Number(m[2]), b: Number(m[3]) }
  return parseHex(rgb)
}

function clamp255(v: number): number {
  return Math.max(0, Math.min(255, Math.round(v)))
}

// Per-commodity colour palette for deposit dots. Unknown commodities fall
// back to a neutral grey — the id also appears in the hover tooltip so the
// player can still identify it.
const DEPOSIT_COLOR: Record<string, string> = {
  iron: '#c88a4a',
  silicates: '#c9b8a5',
  rare_metals: '#e6c04a',
  water_ice: '#a8d6e6',
  water: '#5a9ac9',
  methane: '#8d6ac8',
  sulfur: '#e6d84a',
  geothermal_energy: '#e07a3a',
  organics: '#6ac26a',
}

function depositColor(commodity_id: string): string {
  return DEPOSIT_COLOR[commodity_id] ?? '#aab'
}

// Layout multiple deposit dots inside a hex, evenly spaced around the top
// half so they don't overlap the colony star underneath.
function depositPositions(hex: Positioned): { cx: number; cy: number; r: number; color: string; commodity_id: string; richness: number }[] {
  const n = hex.deposits.length
  if (n === 0) return []
  const orbit = HEX_SIZE * 0.45
  const startAngle = -Math.PI / 2 // straight up
  return hex.deposits.map((d, i) => {
    const angle =
      n === 1
        ? startAngle
        : startAngle + ((i - (n - 1) / 2) * (Math.PI / 3))
    return {
      cx: hex.cx + orbit * Math.cos(angle),
      cy: hex.cy + orbit * Math.sin(angle),
      // Richness is [0,1]; map to a modest radius so dense hexes don't clutter.
      r: 2 + Math.max(0.5, d.richness) * 3,
      color: depositColor(d.commodity_id),
      commodity_id: d.commodity_id,
      richness: d.richness,
    }
  })
}

// Legend entries: only show commodities that actually appear on the map.
const legend = computed<{ commodity_id: string; color: string }[]>(() => {
  const seen = new Set<string>()
  for (const h of props.map.hexes) {
    for (const d of h.deposits) seen.add(d.commodity_id)
  }
  return Array.from(seen)
    .sort()
    .map((id) => ({ commodity_id: id, color: depositColor(id) }))
})

/** Axial hex distance, matching the Rust `HexCoord::distance` cube-coordinate formula. */
function hexDistance(a: { q: number; r: number }, b: { q: number; r: number }): number {
  const dq = Math.abs(a.q - b.q)
  const dr = Math.abs(a.r - b.r)
  const ds = Math.abs(-a.q - a.r - (-b.q - b.r))
  return Math.max(dq, dr, ds)
}

// Minimum hex distance enforced between recommended sites so the top-N ring
// doesn't cluster into a single corner of the map (issue #188). Mirrors the
// greedy selection in `PlanetMap::top_landing_sites` on the Rust side.
const TOP_SITE_MIN_DISTANCE = 3

// Top-N habitable sites for the "recommended" highlight ring.
const topSiteIds = computed<Set<string>>(() => {
  const n = props.highlightTopN ?? 3
  const candidates = positioned.value
    .filter((h) => h.habitable && h.occupied_by === null)
    .sort((a, b) => b.suitability - a.suitability)

  const picked: Positioned[] = []
  for (const hex of candidates) {
    if (picked.length >= n) break
    if (picked.every((p) => hexDistance(p, hex) >= TOP_SITE_MIN_DISTANCE)) {
      picked.push(hex)
    }
  }
  return new Set(picked.map((h) => h.site_id))
})

function isSelected(hex: Positioned): boolean {
  return props.selectedSite === hex.site_id
}

function classes(hex: Positioned): Record<string, boolean> {
  return {
    hex: true,
    'not-habitable': !hex.habitable,
    selected: isSelected(hex),
    recommended: topSiteIds.value.has(hex.site_id) && !isSelected(hex),
    occupied: hex.occupied_by !== null,
  }
}

function onHexClick(hex: Positioned): void {
  // Suppress click when the mouse was actually dragging the map.
  if (dragMoved) return
  if (!hex.habitable || hex.occupied_by !== null) return
  emit('select', hex)
}

// Per-band tooltip copy for hexes whose temperature drags suitability down
// (issue #190) — mirrors the ordering in `temperature_suitability_factor`
// on the Rust side (Temperate is neutral; everything else is a penalty).
const HARSH_CLIMATE_WARNING: Record<string, string> = {
  Cold: 'Cold climate — reduced suitability',
  Hot: 'Hot climate — reduced suitability',
  Frozen: 'Frozen climate — much reduced suitability',
  Extreme: 'Extreme climate — severely reduced suitability',
}

function harshClimateWarning(temperature: string): string | null {
  return HARSH_CLIMATE_WARNING[temperature] ?? null
}

// ── Hover tooltip ─────────────────────────────────────────────────────────

const hoveredHex = ref<Positioned | null>(null)
const hoverPos = ref<{ x: number; y: number }>({ x: 0, y: 0 })

function onHexEnter(hex: Positioned, e: MouseEvent): void {
  hoveredHex.value = hex
  updateHoverPos(e)
}

function onHexMove(e: MouseEvent): void {
  if (hoveredHex.value) updateHoverPos(e)
}

function onHexLeave(): void {
  hoveredHex.value = null
}

function updateHoverPos(e: MouseEvent): void {
  const svg = svgRef.value
  if (!svg) return
  const rect = svg.getBoundingClientRect()
  hoverPos.value = { x: e.clientX - rect.left + 12, y: e.clientY - rect.top + 12 }
}

// ── Public API for the parent wizard ──────────────────────────────────────

/** Centre the viewBox on a specific site, keeping current zoom width. */
function focusSite(site_id: string): void {
  const hex = positioned.value.find((h) => h.site_id === site_id)
  if (!hex) return
  const w = viewBox.value.w
  const h = viewBox.value.h
  viewBox.value = { x: hex.cx - w / 2, y: hex.cy - h / 2, w, h }
}

/** Reset viewBox to fit all hexes. */
function resetView(): void {
  fitToContent()
}

defineExpose({ focusSite, resetView })
</script>

<template>
  <div class="planet-map-wrap">
    <svg
      ref="svgRef"
      :viewBox="viewBoxStr"
      preserveAspectRatio="xMidYMid meet"
      class="planet-map"
      :class="{ dragging: isDragging }"
      data-testid="planet-hex-map"
      @wheel.prevent="onWheel"
      @mousedown="onMouseDown"
    >
      <g
        v-for="h in positioned"
        :key="h.site_id || `${h.q}-${h.r}`"
        :class="classes(h)"
        @click="onHexClick(h)"
        @mouseenter="onHexEnter(h, $event)"
        @mousemove="onHexMove"
        @mouseleave="onHexLeave"
      >
        <polygon :points="hexPoints(h.cx, h.cy)" :fill="biomeColor(h)" />
        <circle
          v-for="d in depositPositions(h)"
          :key="`${h.site_id}-${d.commodity_id}`"
          :cx="d.cx"
          :cy="d.cy"
          :r="d.r"
          :fill="d.color"
          stroke="#000"
          stroke-width="0.5"
        />
        <text
          v-if="h.occupied_by"
          :x="h.cx"
          :y="h.cy + 4"
          text-anchor="middle"
          class="colony-label"
        >
          ★
        </text>
      </g>
    </svg>

    <div
      v-if="hoveredHex"
      class="hex-tooltip"
      :style="{ left: `${hoverPos.x}px`, top: `${hoverPos.y}px` }"
    >
      <div class="tt-title">
        {{ hoveredHex.terrain }} · {{ hoveredHex.biome }}
      </div>
      <div class="tt-row">
        <span class="tt-label">Temperature</span>
        <span>{{ hoveredHex.temperature }}</span>
      </div>
      <div class="tt-row">
        <span class="tt-label">Elevation</span>
        <span>{{ (hoveredHex.elevation * 100).toFixed(0) }}%</span>
      </div>
      <div class="tt-row">
        <span class="tt-label">Suitability</span>
        <span>{{ hoveredHex.suitability.toFixed(1) }}</span>
      </div>
      <div class="tt-row" v-if="!hoveredHex.habitable">
        <span class="tt-warn">Impassable</span>
      </div>
      <div class="tt-row" v-else-if="hoveredHex.occupied_by">
        <span class="tt-warn">Occupied by {{ hoveredHex.occupied_by }}</span>
      </div>
      <div class="tt-row" v-else-if="harshClimateWarning(hoveredHex.temperature)">
        <span class="tt-warn">{{ harshClimateWarning(hoveredHex.temperature) }}</span>
      </div>
      <div v-if="hoveredHex.deposits.length" class="tt-deposits">
        <div
          v-for="d in hoveredHex.deposits"
          :key="d.commodity_id"
          class="tt-deposit"
        >
          <span class="tt-dot" :style="{ background: depositColor(d.commodity_id) }" />
          {{ d.commodity_id }} · {{ (d.richness * 100).toFixed(0) }}%
        </div>
      </div>
    </div>

    <div v-if="legend.length" class="legend" data-testid="planet-map-legend">
      <div class="legend-title">Deposits</div>
      <div v-for="e in legend" :key="e.commodity_id" class="legend-row">
        <span class="legend-dot" :style="{ background: e.color }" />
        <span class="legend-label">{{ e.commodity_id }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.planet-map-wrap {
  position: relative;
  width: 100%;
  height: 100%;
}

.planet-map {
  width: 100%;
  height: 100%;
  background: #05050b;
  border: 1px solid #223;
  border-radius: 6px;
  cursor: grab;
}
.planet-map.dragging { cursor: grabbing; }

.hex polygon {
  stroke: #14141e;
  stroke-width: 1;
  cursor: pointer;
  transition: stroke 0.1s, stroke-width 0.1s;
}
.hex.not-habitable polygon { cursor: not-allowed; opacity: 0.6; }
.hex.occupied polygon { cursor: not-allowed; }
.hex:not(.not-habitable):not(.occupied):hover polygon {
  stroke: #aac;
  stroke-width: 2;
}
.hex.selected polygon {
  stroke: #8cf;
  stroke-width: 3;
}
.hex.recommended polygon {
  stroke: #ac6;
  stroke-width: 2;
  stroke-dasharray: 3 2;
}

.colony-label {
  fill: #fff;
  font-family: monospace;
  font-size: 12px;
  pointer-events: none;
}

.hex-tooltip {
  position: absolute;
  pointer-events: none;
  background: #10101a;
  border: 1px solid #446;
  border-radius: 4px;
  padding: 0.4rem 0.6rem;
  font-family: monospace;
  font-size: 0.75rem;
  color: #aac;
  min-width: 140px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.6);
  z-index: 5;
}
.tt-title { color: #8cf; font-weight: bold; margin-bottom: 0.25rem; }
.tt-row { display: flex; justify-content: space-between; gap: 0.5rem; }
.tt-label { color: #668; }
.tt-warn { color: #d86; }
.tt-deposits { margin-top: 0.3rem; }
.tt-deposit { display: flex; align-items: center; gap: 0.35rem; color: #aab; }
.tt-dot {
  display: inline-block;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  border: 1px solid #000;
}

.legend {
  position: absolute;
  bottom: 0.5rem;
  right: 0.5rem;
  background: rgba(16, 16, 26, 0.9);
  border: 1px solid #223;
  border-radius: 4px;
  padding: 0.35rem 0.5rem;
  font-family: monospace;
  font-size: 0.7rem;
  color: #aab;
  pointer-events: none;
  max-width: 130px;
}
.legend-title { color: #668; margin-bottom: 0.2rem; }
.legend-row { display: flex; align-items: center; gap: 0.35rem; }
.legend-dot {
  display: inline-block;
  width: 7px;
  height: 7px;
  border-radius: 50%;
  border: 1px solid #000;
}
.legend-label { white-space: nowrap; }
</style>
