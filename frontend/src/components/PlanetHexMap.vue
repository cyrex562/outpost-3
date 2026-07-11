<script setup lang="ts">
import { computed } from 'vue'
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

const bounds = computed(() => {
  let minX = Infinity
  let minY = Infinity
  let maxX = -Infinity
  let maxY = -Infinity
  for (const h of props.map.hexes) {
    const { x, y } = axialToPixel(h.q, h.r)
    if (x < minX) minX = x
    if (y < minY) minY = y
    if (x > maxX) maxX = x
    if (y > maxY) maxY = y
  }
  const pad = HEX_SIZE + 8
  return {
    x: minX - pad,
    y: minY - pad,
    w: (maxX - minX) + pad * 2,
    h: (maxY - minY) + pad * 2,
  }
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
  if (!hex.habitable) return BIOME_COLOR.Ocean // ocean = only unhabitable
  return BIOME_COLOR[hex.biome] ?? '#556'
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

// Top-N habitable sites for the "recommended" highlight ring.
const topSiteIds = computed<Set<string>>(() => {
  const n = props.highlightTopN ?? 3
  const habitable = positioned.value
    .filter((h) => h.habitable)
    .sort((a, b) => b.suitability - a.suitability)
    .slice(0, n)
  return new Set(habitable.map((h) => h.site_id))
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

function onClick(hex: Positioned): void {
  if (!hex.habitable || hex.occupied_by !== null) return
  emit('select', hex)
}
</script>

<template>
  <svg
    :viewBox="`${bounds.x} ${bounds.y} ${bounds.w} ${bounds.h}`"
    class="planet-map"
    data-testid="planet-hex-map"
  >
    <g
      v-for="h in positioned"
      :key="h.site_id || `${h.q}-${h.r}`"
      :class="classes(h)"
      @click="onClick(h)"
    >
      <polygon
        :points="hexPoints(h.cx, h.cy)"
        :fill="biomeColor(h)"
      />
      <circle
        v-if="h.deposits.length > 0"
        :cx="h.cx"
        :cy="h.cy - 3"
        r="3.5"
        fill="#fda"
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
</template>

<style scoped>
.planet-map {
  width: 100%;
  height: 100%;
  background: #05050b;
  border: 1px solid #223;
  border-radius: 6px;
}

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
</style>
