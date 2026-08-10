<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import type { PlanetHex, PlanetMap } from '@/services/tauriBridge'
import { usePlanetMapWrap } from '@/composables/usePlanetMapWrap'

// Shared across every mounted map, and persisted — see the composable.
const { wrapEnabled, toggleWrap } = usePlanetMapWrap()

const props = defineProps<{
  map: PlanetMap
  selectedSite: string | null
  highlightTopN?: number
  /**
   * When true (persistent map browse mode, phase A1), clicking an occupied
   * hex emits `select` so the parent can drill into that colony. Default
   * false keeps the founding wizard's behavior, where occupied hexes are
   * inert (you can't found on top of an existing colony).
   */
  selectableOccupied?: boolean
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

// ── Infrastructure edges (map/nav plan phase A3) ───────────────────────────
//
// Each edge connects two colonies; endpoints are resolved to hex centers via
// each colony's occupant_colony_id (surfaced in phase A1). Colonies occupy
// exactly one hex, so this lookup is unambiguous.
const colonyCenters = computed<Map<string, { cx: number; cy: number }>>(() => {
  const m = new Map<string, { cx: number; cy: number }>()
  for (const h of positioned.value) {
    if (h.occupant_colony_id) m.set(h.occupant_colony_id, { cx: h.cx, cy: h.cy })
  }
  return m
})

/** Occupied hexes, for the top colony-marker layer (drawn above edges). */
const occupiedHexes = computed<Positioned[]>(() =>
  positioned.value.filter((h) => h.occupied_by !== null),
)

const INFRA_COLOR: Record<string, string> = {
  road: '#b8a06a',
  rail: '#d8c85a',
  pipeline: '#6ab0d8',
  powerline: '#e0c848',
}

interface RenderedEdge {
  key: string
  x1: number
  y1: number
  x2: number
  y2: number
  color: string
  width: number
  infra_type: string
  /** Tooltip text summarizing throughput and transmission loss (issue #383). */
  title: string
}

// Pixel-space width of one full map wrap: shifting a hex's `q` by the map's
// `width` shifts its pixel x by exactly this (the `y` term only depends on
// `r`, so this shift is uniform regardless of which row the hex is on).
const mapPixelWidth = computed(() => HEX_SIZE * SQRT3 * props.map.width)

const infraEdges = computed<RenderedEdge[]>(() => {
  const out: RenderedEdge[] = []
  const wrapW = mapPixelWidth.value
  for (const e of props.map.edges ?? []) {
    const from = colonyCenters.value.get(e.from_colony_id)
    const to = colonyCenters.value.get(e.to_colony_id)
    if (!from || !to) continue // an endpoint colony isn't on the map (yet)
    const width = 1.2 + Math.min(4, e.throughput / 60)
    const color = INFRA_COLOR[e.infra_type] ?? '#889'
    const key = `${e.from_colony_id}-${e.to_colony_id}-${e.infra_type}`
    const title = `${e.infra_type} · ${e.throughput.toFixed(0)}/turn · ${(e.loss_pct * 100).toFixed(0)}% loss`

    // The engine's `edge_cost` already routes construction cost through the
    // seam when that's shorter (issue #315); rendering must match, or a
    // cheap seam-adjacent edge would draw as a long line across the whole
    // map interior. Pick whichever unwrapped copy of `to` is nearest `from`
    // in pixel space, then draw that segment *and* its mirror shifted by one
    // full map width — whichever half is actually within the viewBox shows,
    // giving a continuous wrap-around line without any special clipping.
    let toX = to.cx
    if (Math.abs(to.cx - wrapW - from.cx) < Math.abs(toX - from.cx)) toX = to.cx - wrapW
    if (Math.abs(to.cx + wrapW - from.cx) < Math.abs(toX - from.cx)) toX = to.cx + wrapW

    out.push({ key, x1: from.cx, y1: from.cy, x2: toX, y2: to.cy, color, width, infra_type: e.infra_type, title })
    if (toX !== to.cx) {
      // Mirror copy, shifted back by one map width, so the portion that
      // exits one side of the visible map re-enters on the other.
      const shift = toX - to.cx
      out.push({
        key: `${key}-wrap`,
        x1: from.cx - shift,
        y1: from.cy,
        x2: to.cx,
        y2: to.cy,
        color,
        width,
        infra_type: e.infra_type,
        title,
      })
    }
  }
  return out
})

/** Unpadded pixel extent of the hex grid, or `null` when there are no hexes. */
const rawExtent = computed<{ minX: number; minY: number; maxX: number; maxY: number } | null>(() => {
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
  return isFinite(minX) ? { minX, minY, maxX, maxY } : null
})

const contentBounds = computed(() => {
  const e = rawExtent.value
  if (!e) {
    return { x: -HEX_SIZE, y: -HEX_SIZE, w: HEX_SIZE * 2, h: HEX_SIZE * 2 }
  }
  const pad = HEX_SIZE + 8
  return {
    x: e.minX - pad,
    y: e.minY - pad,
    w: e.maxX - e.minX + pad * 2,
    h: e.maxY - e.minY + pad * 2,
  }
})

/**
 * Horizontal pixel shifts at which the whole map (hexes, deposits, infra
 * edges, colony markers) must be redrawn so panning west/east never runs off
 * into empty space — `PlanetMap.width` "wraps east-west" (see
 * `tauriBridge.ts`), but until now only infra-edge line segments accounted
 * for that (via the nearest-copy trick above); the tiles themselves were
 * drawn exactly once and simply ended at the map's edge.
 *
 * Computed from the current viewBox so exactly enough repeats render to
 * cover what's visible, at any zoom level — not a fixed "3 copies" that
 * would run out at low zoom (a small body's map can need a dozen+ repeats to
 * fill `MAX_VIEW_W`) or waste render cost at high zoom (where 1 is enough).
 */
const wrapShifts = computed<number[]>(() => {
  // Wrapping off → a single unshifted copy, i.e. exactly the pre-wrap
  // rendering. Every wrapped layer (tiles, deposits, markers, infra edges,
  // seam) derives from this list, so gating it here is the whole toggle.
  if (!wrapEnabled.value) return [0]
  const period = mapPixelWidth.value
  const e = rawExtent.value
  if (!e || !Number.isFinite(period) || period <= 1) return [0]
  const vb = viewBox.value
  // Copy k occupies [minX + k*period, maxX + k*period]; it overlaps the
  // viewBox when minX+k*period <= vb.x+vb.w (bounds k from above → floor)
  // and maxX+k*period >= vb.x (bounds k from below → ceil).
  const kLow = Math.ceil((vb.x - e.maxX) / period)
  const kHigh = Math.floor((vb.x + vb.w - e.minX) / period)
  // Safety ceiling, not a realistic limit — MAX_VIEW_W / the narrowest real
  // map width (`BodySize::Tiny`, 10 columns) needs well under this many.
  const MAX_COPIES = 41
  const shifts: number[] = []
  for (let k = kLow; k <= kHigh && shifts.length < MAX_COPIES; k++) shifts.push(k * period)
  return shifts.length ? shifts : [0]
})

// Pre-flattened (shift × item) views of the three renderable layers, each
// with cx offset by its shift. Kept as plain sibling arrays (not an extra
// wrapping `<g transform>` per shift) so the rendered DOM stays exactly the
// structure it was before wrapping existed whenever wrapShifts is just
// `[0]` (the common case, and every existing test's case) — a `<g>`
// wrapper would shift every `find('g')`/`findAll('g')[i]` index in ways
// unrelated to this feature.
interface WrappedHex extends Positioned {
  wrapKey: string
}

const wrappedPositioned = computed<WrappedHex[]>(() => {
  const out: WrappedHex[] = []
  for (const shift of wrapShifts.value) {
    for (const h of positioned.value) {
      out.push({ ...h, cx: h.cx + shift, wrapKey: `${shift}:${h.site_id || `${h.q}-${h.r}`}` })
    }
  }
  return out
})

const wrappedOccupiedHexes = computed<WrappedHex[]>(() => {
  const out: WrappedHex[] = []
  for (const shift of wrapShifts.value) {
    for (const h of occupiedHexes.value) {
      out.push({ ...h, cx: h.cx + shift, wrapKey: `${shift}:marker-${h.site_id}` })
    }
  }
  return out
})

interface WrappedEdge extends RenderedEdge {
  wrapKey: string
}

const wrappedInfraEdges = computed<WrappedEdge[]>(() => {
  const out: WrappedEdge[] = []
  for (const shift of wrapShifts.value) {
    for (const e of infraEdges.value) {
      out.push({ ...e, x1: e.x1 + shift, x2: e.x2 + shift, wrapKey: `${shift}:${e.key}` })
    }
  }
  return out
})

// ── Wrap-seam indicator ──────────────────────────────────────────────────
//
// A visible marker for where the map's east-west wrap actually falls
// (`PlanetMap.width` "wraps east-west" — see `tauriBridge.ts`) so panning
// into a wrapped repeat doesn't read as "am I still on the same map?".
// Traced along the west-facing edge of every q=0 hex — q=0 is adjacent to
// q=width-1 once wrapped, so this single line is the seam; drawing
// q=width-1's east edge too would just double it. Because axial (q, r)
// pixel position depends on both coordinates (`axialToPixel`'s x term has
// an `r` component), a constant-q column isn't a vertical line — it's
// diagonal, so this deliberately traces the hexes' own edge-by-edge
// perimeter rather than drawing one straight line, which would cut across
// the actual hex boundary instead of following it.
interface SeamSegment {
  key: string
  x1: number
  y1: number
  x2: number
  y2: number
}

// The single straight edge of a pointy-top hexagon that faces due west:
// the segment between the bottom-left vertex (150°) and top-left vertex
// (210°) in `hexPoints`' vertex ordering.
const WEST_EDGE_ANGLE_1 = (Math.PI / 3) * 2 + Math.PI / 6
const WEST_EDGE_ANGLE_2 = (Math.PI / 3) * 3 + Math.PI / 6

const seamSegments = computed<SeamSegment[]>(() => {
  const out: SeamSegment[] = []
  for (const h of positioned.value) {
    if (h.q !== 0) continue
    for (const shift of wrapShifts.value) {
      out.push({
        key: `${h.site_id || `${h.q}-${h.r}`}:${shift}`,
        x1: h.cx + shift + HEX_SIZE * Math.cos(WEST_EDGE_ANGLE_1),
        y1: h.cy + HEX_SIZE * Math.sin(WEST_EDGE_ANGLE_1),
        x2: h.cx + shift + HEX_SIZE * Math.cos(WEST_EDGE_ANGLE_2),
        y2: h.cy + HEX_SIZE * Math.sin(WEST_EDGE_ANGLE_2),
      })
    }
  }
  return out
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

// Terrain is the primary color signal — the physical landform (rock, soil,
// water, ash) rather than what's growing on it. Mirrors `outpost_core::map::Terrain`.
const TERRAIN_COLOR: Record<string, string> = {
  Plains: '#8a7f5a',
  Hills: '#9c8a66',
  Mountains: '#7d7468',
  Wetlands: '#5c6e58',
  Ocean: '#2c4665',
  Volcanic: '#5a3324',
}

// Vegetation is layered on top of terrain + water as a green tint (issue
// #316's layer 3). `vegetation_density` (0–1, `outpost_core::map::HexCell`)
// is the primary signal — a real per-cell gradient, zero everywhere on a
// body whose archetype has no vegetation story at all
// (`PlanetarySubtype::has_vegetation`). Older/fixture data that lacks the
// field falls back to a biome-derived approximation so existing snapshots
// and tests keep rendering sensibly.
const VEGETATION_STRENGTH_BY_BIOME: Record<string, number> = {
  Jungle: 0.7,
  Forest: 0.55,
  Grassland: 0.25,
}
const VEGETATION_COLOR = '#2f6b2f'

// Water/ice is its own overlay (issue #316's layer 2), independent of
// terrain's base color. `water_coverage` (0–1) sets the overlay's alpha;
// whether it reads as liquid or frozen is decided here from `temperature`
// rather than a separate core field — a `Frozen`/`Extreme` cell tints white
// (ice), everything else tints blue (liquid water). Terrain without an
// explicit `water_coverage` falls back to a terrain-derived guess (full
// coverage for Ocean, moderate for Wetlands) for older/fixture data.
const LIQUID_WATER_COLOR = '#3a78b0'
const ICE_COLOR = '#eaf3fb'
const FROZEN_BANDS = new Set(['Frozen', 'Extreme'])

function terrainColor(hex: PlanetHex): string {
  const base = TERRAIN_COLOR[hex.terrain] ?? '#556'
  const watered = applyWaterIce(base, hex)
  const vegetated = applyVegetation(watered, hex)
  const elevated = applyElevationShading(vegetated, hex.elevation)
  const tinted = applyTemperatureTint(elevated, hex.temperature)
  return applyContamination(tinted, hex)
}

/** This hex's contamination severity in `[0, 1]` (issue #387). */
function contamination(hex: PlanetHex): number {
  return typeof hex.contamination === 'number' ? hex.contamination : 0
}

// Sickly yellow-green, applied last (on top of every other layer) so
// contamination always reads as an overlay warning rather than competing
// with terrain/water/vegetation for the base color.
const CONTAMINATION_COLOR = '#9acd1a'

/** Blend the contamination warning tint over a color, by severity. */
function applyContamination(rgbOrHex: string, hex: PlanetHex): string {
  const severity = contamination(hex)
  if (severity <= 0) return rgbOrHex
  const base = parseRgb(rgbOrHex)
  const target = parseHex(CONTAMINATION_COLOR)
  if (!base || !target) return rgbOrHex
  // Capped below full-strength so even a maximally contaminated hex still
  // shows its underlying terrain, rather than reading as a flat color swatch.
  const strength = Math.min(severity, 1) * 0.6
  const r = clamp255(base.r + (target.r - base.r) * strength)
  const g = clamp255(base.g + (target.g - base.g) * strength)
  const b = clamp255(base.b + (target.b - base.b) * strength)
  return `rgb(${r}, ${g}, ${b})`
}

/** This hex's water/ice surface coverage in `[0, 1]`, real or derived. */
function waterCoverage(hex: PlanetHex): number {
  if (typeof hex.water_coverage === 'number') return hex.water_coverage
  if (hex.terrain === 'Ocean') return 1.0
  if (hex.terrain === 'Wetlands') return 0.35
  return 0.0
}

/** This hex's vegetation density in `[0, 1]`, real or derived from biome. */
function vegetationDensity(hex: PlanetHex): number {
  if (typeof hex.vegetation_density === 'number') return hex.vegetation_density
  return VEGETATION_STRENGTH_BY_BIOME[hex.biome] ?? 0
}

/** Blend the water/ice overlay over a terrain base color, by coverage. */
function applyWaterIce(rgbOrHex: string, hex: PlanetHex): string {
  const coverage = waterCoverage(hex)
  if (coverage <= 0) return rgbOrHex
  const base = parseRgb(rgbOrHex)
  const target = parseHex(FROZEN_BANDS.has(hex.temperature) ? ICE_COLOR : LIQUID_WATER_COLOR)
  if (!base || !target) return rgbOrHex
  const r = clamp255(base.r + (target.r - base.r) * coverage)
  const g = clamp255(base.g + (target.g - base.g) * coverage)
  const b = clamp255(base.b + (target.b - base.b) * coverage)
  return `rgb(${r}, ${g}, ${b})`
}

/** Blend the vegetation tint over a color, by this hex's vegetation density. */
function applyVegetation(rgbOrHex: string, hex: PlanetHex): string {
  const strength = vegetationDensity(hex)
  if (strength <= 0) return rgbOrHex
  const base = parseRgb(rgbOrHex)
  const target = parseHex(VEGETATION_COLOR)
  if (!base || !target) return rgbOrHex
  const r = clamp255(base.r + (target.r - base.r) * strength)
  const g = clamp255(base.g + (target.g - base.g) * strength)
  const b = clamp255(base.b + (target.b - base.b) * strength)
  return `rgb(${r}, ${g}, ${b})`
}

/**
 * Modulate a hex-fill colour by elevation to create subtle relief shading.
 * Low basins darken, high peaks brighten — kept modest so the terrain/
 * vegetation colour still dominates the read.
 */
function applyElevationShading(color: string, elevation: number): string {
  const rgb = parseRgb(color)
  if (!rgb) return color
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
 * Kept weak enough that terrain/vegetation colour and elevation shading
 * still dominate.
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

// Per-commodity colour + two-letter code for deposit boxes. Covers
// `outpost_core::map::VEIN_COMMODITIES` (the only commodities that ever
// actually appear as a hex deposit) explicitly; anything else falls back to
// a neutral grey box with its first two letters, uppercased — the full id
// also appears in the hover tooltip so the player can always identify it.
const DEPOSIT_STYLE: Record<string, { color: string; code: string }> = {
  structural_ore: { color: '#c88a4a', code: 'SO' },
  conductive_ore: { color: '#d4703a', code: 'CO' },
  precious_ore: { color: '#e6c04a', code: 'PO' },
  refractory_ore: { color: '#a85a4a', code: 'RO' },
  semiconductor_ore: { color: '#8d6ac8', code: 'SI' },
  fissile_ore: { color: '#b8e64a', code: 'FI' },
  silicates: { color: '#c9b8a5', code: 'SL' },
  hydrocarbons: { color: '#4a3a2a', code: 'HC' },
  biomass: { color: '#6ac26a', code: 'BM' },
}

function depositColor(commodity_id: string): string {
  return DEPOSIT_STYLE[commodity_id]?.color ?? '#aab'
}

function depositCode(commodity_id: string): string {
  return DEPOSIT_STYLE[commodity_id]?.code ?? commodity_id.slice(0, 2).toUpperCase()
}

// Layout multiple deposit boxes inside a hex, evenly spaced around the top
// half so they don't overlap the colony star underneath.
function depositPositions(
  hex: Positioned,
): { cx: number; cy: number; size: number; color: string; code: string; commodity_id: string; richness: number }[] {
  const n = hex.deposits.length
  if (n === 0) return []
  const orbit = HEX_SIZE * 0.45
  const startAngle = -Math.PI / 2 // straight up
  return hex.deposits.map((d, i) => {
    const angle = n === 1 ? startAngle : startAngle + (i - (n - 1) / 2) * (Math.PI / 3)
    return {
      cx: hex.cx + orbit * Math.cos(angle),
      cy: hex.cy + orbit * Math.sin(angle),
      // Richness is [0,1]; map to a modest box side so dense hexes don't clutter.
      size: 9 + Math.max(0.3, d.richness) * 5,
      color: depositColor(d.commodity_id),
      code: depositCode(d.commodity_id),
      commodity_id: d.commodity_id,
      richness: d.richness,
    }
  })
}

// Legend entries: only show commodities that actually appear on the map.
const legend = computed<{ commodity_id: string; color: string; code: string }[]>(() => {
  const seen = new Set<string>()
  for (const h of props.map.hexes) {
    for (const d of h.deposits) seen.add(d.commodity_id)
  }
  return Array.from(seen)
    .sort()
    .map((id) => ({ commodity_id: id, color: depositColor(id), code: depositCode(id) }))
})

// Terrain is the base fill color every hex gets before the water/vegetation/
// elevation/temperature/contamination overlays blend on top of it (see
// `terrainColor()`), so it's the one tile property worth a swatch legend —
// those other layers are continuous tints, not discrete categories, and
// biome doesn't drive its own color at all (only a vegetation-strength
// fallback). Both are still readable per-hex from the hover tooltip. Only
// terrain values actually present on this map get a row, matching Deposits.
const terrainLegend = computed<{ terrain: string; color: string }[]>(() => {
  const seen = new Set<string>()
  for (const h of props.map.hexes) seen.add(h.terrain)
  return Array.from(seen)
    .sort()
    .map((terrain) => ({ terrain, color: TERRAIN_COLOR[terrain] ?? '#556' }))
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
    'occupied-clickable': hex.occupied_by !== null && !!props.selectableOccupied,
  }
}

function onHexClick(hex: Positioned): void {
  // Suppress click when the mouse was actually dragging the map.
  if (dragMoved) return
  if (hex.occupied_by !== null) {
    // Browse mode: an occupied hex is a colony node — let the parent react
    // (e.g. route to that colony). Wizard mode leaves it inert.
    if (props.selectableOccupied) emit('select', hex)
    return
  }
  if (!hex.habitable) return
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

/**
 * A hex's geothermal gradient (issue #412), or `null` when the backend
 * predates the layer.
 */
function geothermal(h: PlanetHex): number | null {
  return h.geothermal_gradient ?? null
}

/**
 * The gradient as a percentage plus a word, because the number alone means
 * nothing to a player deciding where to land. The bands match the thresholds
 * the engine actually uses: below `DEEP_DRILLING_GRADIENT` (0.2) reaching
 * heat needs drilling tech, and at or above `VOLCANIC_MIN_GRADIENT` (0.6) the
 * ground is volcanic.
 */
function geothermalLabel(h: PlanetHex): string {
  const g = geothermal(h)
  if (g === null) return ''
  const pct = (g * 100).toFixed(0)
  if (g >= 0.6) return `${pct}% — shallow magma`
  if (g >= 0.4) return `${pct}% — warm crust`
  if (g >= 0.2) return `${pct}% — cool crust`
  return `${pct}% — deep, needs drilling tech`
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
      <!-- `wrappedPositioned` repeats every hex at each entry in `wrapShifts`
           (west/east panning must never run into empty space — the
           underlying map wraps east-west); in the common case wrapShifts is
           just `[0]`, so this renders identically to a plain `positioned`
           loop. Click/hover handlers receive the wrapped-and-shifted copy,
           which carries the same site_id/q/r as the real hex either way. -->
      <g
        v-for="h in wrappedPositioned"
        :key="h.wrapKey"
        :class="classes(h)"
        @click="onHexClick(h)"
        @mouseenter="onHexEnter(h, $event)"
        @mousemove="onHexMove"
        @mouseleave="onHexLeave"
      >
        <polygon :points="hexPoints(h.cx, h.cy)" :fill="terrainColor(h)" />
        <g v-for="d in depositPositions(h)" :key="`${h.wrapKey}-${d.commodity_id}`">
          <rect
            :x="d.cx - d.size / 2"
            :y="d.cy - d.size / 2"
            :width="d.size"
            :height="d.size"
            rx="1.5"
            :fill="d.color"
            stroke="#000"
            stroke-width="0.5"
          />
          <text
            :x="d.cx"
            :y="d.cy"
            text-anchor="middle"
            dominant-baseline="central"
            class="deposit-code"
          >
            {{ d.code }}
          </text>
        </g>
      </g>

      <!-- Wrap-seam indicator: where the map's east-west wrap actually
           falls, drawn above terrain like the infra/marker layers below. -->
      <g class="seam-layer" data-testid="seam-layer">
        <line
          v-for="seg in seamSegments"
          :key="seg.key"
          :x1="seg.x1"
          :y1="seg.y1"
          :x2="seg.x2"
          :y2="seg.y2"
          class="seam-line"
          data-testid="seam-line"
        ><title>Map wraps here — the west edge continues from the east edge</title></line>
      </g>

      <!-- Infrastructure edges (phase A3): drawn above terrain but beneath
           the colony markers below, so nodes stay legible. -->
      <g class="infra-layer" data-testid="infra-layer">
        <line
          v-for="edge in wrappedInfraEdges"
          :key="edge.wrapKey"
          :x1="edge.x1"
          :y1="edge.y1"
          :x2="edge.x2"
          :y2="edge.y2"
          :stroke="edge.color"
          :stroke-width="edge.width"
          stroke-linecap="round"
          class="infra-edge"
          :data-testid="`infra-edge-${edge.key}`"
        ><title>{{ edge.title }}</title></line>
      </g>

      <!-- Colony markers, on top of the infrastructure layer. The labels are
           pointer-events:none, so clicks fall through to the occupied hex
           group below (which owns selection) — no marker click handler needed. -->
      <g v-for="h in wrappedOccupiedHexes" :key="h.wrapKey" class="colony-marker">
        <text :x="h.cx" :y="h.cy + 4" text-anchor="middle" class="colony-label">★</text>
        <text
          :x="h.cx"
          :y="h.cy + HEX_SIZE * 0.72"
          text-anchor="middle"
          class="colony-name"
          :data-testid="`colony-node-label-${h.occupant_colony_id ?? h.site_id}`"
        >
          {{ h.occupied_by }}
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
      <div class="tt-row" v-if="geothermal(hoveredHex) !== null">
        <span class="tt-label">Geothermal</span>
        <span>{{ geothermalLabel(hoveredHex) }}</span>
      </div>
      <div class="tt-row">
        <span class="tt-label">Suitability</span>
        <span>{{ hoveredHex.suitability.toFixed(1) }}</span>
      </div>
      <div class="tt-row" v-if="contamination(hoveredHex) > 0">
        <span class="tt-warn">Contaminated · {{ (contamination(hoveredHex) * 100).toFixed(0) }}%</span>
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
          <span class="tt-box" :style="{ background: depositColor(d.commodity_id) }">{{
            depositCode(d.commodity_id)
          }}</span>
          {{ d.commodity_id }} · {{ (d.richness * 100).toFixed(0) }}%
        </div>
      </div>
    </div>

    <!-- Wrap toggle. Sits opposite the legend (which is bottom-right and
         click-through) so it never covers it, and re-enables pointer events
         the overlay layer otherwise suppresses. -->
    <button
      type="button"
      class="wrap-toggle"
      data-testid="planet-map-wrap-toggle"
      :aria-pressed="wrapEnabled"
      :title="
        wrapEnabled
          ? 'Wrapping on — the map repeats east-west as you pan. Click to show it as a single flat rectangle.'
          : 'Wrapping off — the map ends at its edges. Click to repeat it east-west as you pan.'
      "
      @click="toggleWrap"
    >
      Wrap: {{ wrapEnabled ? 'on' : 'off' }}
    </button>

    <div
      v-if="terrainLegend.length || legend.length"
      class="legend"
      data-testid="planet-map-legend"
    >
      <template v-if="terrainLegend.length">
        <div class="legend-title" data-testid="planet-map-legend-terrain-title">Terrain</div>
        <div v-for="t in terrainLegend" :key="t.terrain" class="legend-row">
          <span class="legend-swatch" :style="{ background: t.color }" />
          <span class="legend-label">{{ t.terrain }}</span>
        </div>
      </template>
      <template v-if="legend.length">
        <div class="legend-title" :class="{ 'legend-title-spaced': terrainLegend.length }">Deposits</div>
        <div v-for="e in legend" :key="e.commodity_id" class="legend-row">
          <span class="legend-box" :style="{ background: e.color }">{{ e.code }}</span>
          <span class="legend-label">{{ e.commodity_id }}</span>
        </div>
      </template>
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
  background: var(--map-bg-inner);
  border: 1px solid var(--border-subtle);
  border-radius: 6px;
  cursor: grab;
}
.planet-map.dragging { cursor: grabbing; }

.hex polygon {
  stroke: var(--surface-2);
  stroke-width: 1;
  cursor: pointer;
  transition: stroke 0.1s, stroke-width 0.1s;
}
.hex.not-habitable polygon { cursor: not-allowed; opacity: 0.6; }
.hex.occupied polygon { cursor: not-allowed; }
.hex.occupied-clickable polygon { cursor: pointer; }
.hex:not(.not-habitable):not(.occupied):hover polygon {
  stroke: var(--text);
  stroke-width: 2;
}
.hex.occupied-clickable:hover polygon {
  stroke: var(--accent);
  stroke-width: 2;
}
.hex.selected polygon {
  stroke: var(--accent);
  stroke-width: 3;
}
.hex.recommended polygon {
  stroke: var(--good-dim);
  stroke-width: 2;
  stroke-dasharray: 3 2;
}

.colony-label {
  fill: var(--map-marker);
  font-family: monospace;
  font-size: 12px;
  pointer-events: none;
}
.colony-name {
  fill: var(--accent-soft);
  font-family: monospace;
  font-size: 7px;
  pointer-events: none;
  paint-order: stroke;
  stroke: var(--map-bg-inner);
  stroke-width: 2px;
}

/* Wrap-seam indicator — bright and thick so it reads clearly at a glance
   against any terrain color underneath, but decorative (doesn't intercept
   clicks meant for the hexes it crosses over). */
.seam-layer { pointer-events: none; }
.seam-line { stroke: var(--map-seam); stroke-width: 3; stroke-linecap: round; }

/* Infrastructure edges are decorative in phase A3 — don't let them intercept
   clicks meant for the hexes they cross over. */
.infra-layer { pointer-events: none; }
.infra-edge { opacity: 0.85; }

.hex-tooltip {
  position: absolute;
  pointer-events: none;
  background: var(--surface-1);
  border: 1px solid var(--border-strong);
  border-radius: 4px;
  padding: 0.4rem 0.6rem;
  font-family: monospace;
  font-size: 0.75rem;
  color: var(--text);
  min-width: 140px;
  box-shadow: 0 2px 8px var(--shadow-strong);
  z-index: 5;
}
.tt-title { color: var(--accent); font-weight: bold; margin-bottom: 0.25rem; }
.tt-row { display: flex; justify-content: space-between; gap: 0.5rem; }
.tt-label { color: var(--text-dim); }
.tt-warn { color: var(--warn); }
.tt-deposits { margin-top: 0.3rem; }
.tt-deposit { display: flex; align-items: center; gap: 0.35rem; color: var(--text); }
.tt-box {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 14px;
  border-radius: 2px;
  border: 1px solid #000;
  color: #000;
  font-size: 0.6rem;
  font-weight: bold;
}

.wrap-toggle {
  position: absolute;
  top: 0.5rem;
  right: 0.5rem;
  background: var(--overlay-panel);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 0.2rem 0.45rem;
  font-family: monospace;
  font-size: 0.7rem;
  color: var(--text);
  cursor: pointer;
  z-index: 2;
}
.wrap-toggle:hover { border-color: var(--border-strong); color: var(--text-bright); }
.wrap-toggle[aria-pressed='true'] { border-color: var(--border-accent); color: var(--accent); }

.legend {
  position: absolute;
  bottom: 0.5rem;
  right: 0.5rem;
  background: var(--overlay-panel);
  border: 1px solid var(--border-subtle);
  border-radius: 4px;
  padding: 0.35rem 0.5rem;
  font-family: monospace;
  font-size: 0.7rem;
  color: var(--text);
  pointer-events: none;
  max-width: 130px;
}
.legend-title { color: var(--text-dim); margin-bottom: 0.2rem; }
.legend-title-spaced { margin-top: 0.4rem; }
.legend-row { display: flex; align-items: center; gap: 0.35rem; }
.legend-box {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 12px;
  border-radius: 2px;
  border: 1px solid #000;
  color: #000;
  font-size: 0.55rem;
  font-weight: bold;
}
.legend-swatch {
  display: inline-block;
  width: 16px;
  height: 12px;
  border-radius: 2px;
  border: 1px solid #000;
  flex-shrink: 0;
}
.legend-label { white-space: nowrap; }

.deposit-code {
  font-family: monospace;
  font-size: 5px;
  font-weight: bold;
  fill: #000;
  pointer-events: none;
  user-select: none;
}
</style>
