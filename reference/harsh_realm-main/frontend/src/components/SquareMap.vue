<script setup lang="ts">
import { ref, computed, watch, onMounted } from "vue";
import { useMapStore } from "../stores/map";
import { useWorldStore } from "../stores/world";
import { useGameStore } from "../stores/game";
import { useWebSocket } from "../composables/useWebSocket";
import MapLegend from "./MapLegend.vue";
import {
  BIOME_COLORS,
  DESERT_COLORS,
  ENCOUNTER_COLORS,
  ENCOUNTER_FEATURES,
  LOOT_COLORS,
  cellLootValue,
  lootGemPoints,
  lootTier,
  type LootTier,
  MOUNTAIN_COLORS,
  OBJECTIVE_COLORS,
  OBJECTIVE_FEATURES,
  ROAD_COLORS,
  RUIN_COLORS,
  SETTLEMENT_COLORS,
  WASTELAND_COLORS,
  WATER_COLORS,
  barrenKind,
  desertDunes,
  forestBiome,
  forestNeighborCount,
  forestTrees,
  hasAnyFeature,
  hashUnit,
  isRoadAt,
  mountainPeaks,
  neighborTerrains,
  orthoRoad,
  orthoWater,
  ruinFragments,
  settlementHouses,
  settlementSize,
  waterKind,
  wastelandCracks,
  type BarrenKind,
  type Crack,
  type Dune,
  type ForestBiome,
  type HousePlacement,
  type OrthoFlags,
  type PeakPlacement,
  type RuinFragment,
  type TreePlacement,
  type WaterKind,
} from "../utils/mapGraphics";

const mapStore = useMapStore();
const worldStore = useWorldStore();
const gameStore = useGameStore();
const { send } = useWebSocket();

const CELL_SIZE = 24;
const showLegend = ref(false);

const svgRef = ref<SVGSVGElement | null>(null);

// --- Right-click cell menu (HR-408) ---
const TRAVEL_SKILLS = [
  { id: "notice", label: "Notice" },
  { id: "survive", label: "Survive" },
  { id: "know", label: "Know" },
];

const menu = ref<{ open: boolean; x: number; y: number; q: number; r: number }>({
  open: false,
  x: 0,
  y: 0,
  q: 0,
  r: 0,
});
const skillsOpen = ref(false);

const menuCell = computed(() =>
  mapStore.cells.get(mapStore.cellKey(menu.value.q, menu.value.r)),
);
const menuHasSettlement = computed(() =>
  (menuCell.value?.features ?? []).includes("settlement"),
);

function onContextMenu(e: MouseEvent): void {
  e.preventDefault();
  const svg = svgRef.value;
  if (!svg) return;
  // Map screen → SVG user coordinates via the element's CTM (this correctly
  // accounts for the viewBox and preserveAspectRatio letterboxing).
  const ctm = svg.getScreenCTM();
  if (!ctm) return;
  const inv = ctm.inverse();
  const worldX = inv.a * e.clientX + inv.c * e.clientY + inv.e;
  const worldY = inv.b * e.clientX + inv.d * e.clientY + inv.f;
  const q = Math.floor(worldX / CELL_SIZE);
  const r = Math.floor(worldY / CELL_SIZE);
  // Only offer the menu for cells that exist on the map (known to the store,
  // including not-yet-explored fog cells).
  if (!mapStore.cells.has(mapStore.cellKey(q, r))) {
    menu.value.open = false;
    return;
  }
  skillsOpen.value = false;
  menu.value = { open: true, x: e.clientX, y: e.clientY, q, r };
}

function closeMenu(): void {
  menu.value.open = false;
  skillsOpen.value = false;
}

function travel(action: "" | "look" | "enter"): void {
  const suffix = action ? ` ${action}` : "";
  send(`travel ${menu.value.q} ${menu.value.r}${suffix}`);
  closeMenu();
}

function travelSkill(skillId: string): void {
  send(`travel ${menu.value.q} ${menu.value.r} skill ${skillId}`);
  closeMenu();
}

function resumeTravel(): void {
  send("resume");
}
function cancelTravel(): void {
  send("cancel");
}

const TERRAIN_COLORS: Record<string, string> = {
    // World map terrains
    plains: "#4a5d3a",
    forest: "#2d4a2d",
    hills: "#6b6b4f",
    mountains: "#5a5a5a",
    water: "#2a3d5a",
    swamp: "#3d4a3a",
    desert: "#7a6a4a",
    wasteland: "#5a4a3a",
    ruins: "#4a4a5a",
    // Dungeon terrains
    corridor: "#5a5040",
    room_floor: "#6a6050",
    wall: "#2a2a2a",
    door: "#8a7040",
    stairs_up: "#50706a",
    stairs_down: "#705060",
    trap_floor: "#6a3030",
    // Town terrains
    road: "#6a6a5a",
    building: "#4a4040",
    shop: "#5a6a40",
    tavern: "#6a5a30",
    temple: "#4a4a6a",
    open_ground: "#4a5a3a",
    plaza: "#6a6a6a",
};
const FOG_COLOR = "#1e1e2a";
const PLAYER_COLOR = "#f0c040";

const FEATURE_LABELS: Record<string, { label: string; color: string }> = {
    settlement: { label: "S", color: "white" },
    ruins: { label: "R", color: "#888" },
    lair: { label: "L", color: "#f44" },
    landmark: { label: "\u2605", color: "#ff0" },
};

// Features that render a dedicated SVG stamp (below) instead of a text glyph.
const STAMPED_FEATURES = new Set<string>([
    "settlement",
    "ruins",
    ...OBJECTIVE_FEATURES,
    ...ENCOUNTER_FEATURES,
]);

// ViewBox state for pan/zoom
const viewBox = ref({ x: 0, y: 0, w: 600, h: 500 });

// Drag state
let isDragging = false;
let dragStartX = 0;
let dragStartY = 0;
let dragStartVbX = 0;
let dragStartVbY = 0;

function cellToPixel(q: number, r: number): { x: number; y: number } {
    return { x: q * CELL_SIZE, y: r * CELL_SIZE };
}

function terrainColor(terrain: string): string {
    return TERRAIN_COLORS[terrain] ?? "#3a3a3a";
}

function centerOnPlayer() {
    const q = mapStore.playerQ;
    const r = mapStore.playerR;
    if (q !== null && r !== null) {
        const { x, y } = cellToPixel(q, r);
        viewBox.value = {
            x: x - viewBox.value.w / 2 + CELL_SIZE / 2,
            y: y - viewBox.value.h / 2 + CELL_SIZE / 2,
            w: viewBox.value.w,
            h: viewBox.value.h,
        };
    }
}

watch(
    () => worldStore.activeWorld,
    async (world) => {
        if (world) {
            await mapStore.loadMap();
            centerOnPlayer();
        }
    },
);

watch(
    () =>
        [mapStore.playerQ, mapStore.playerR] as [number | null, number | null],
    () => {
        if (mapStore.playerQ !== null && mapStore.playerR !== null) {
            centerOnPlayer();
        }
    },
);

onMounted(async () => {
    if (worldStore.activeWorld) {
        await mapStore.loadMap();
        centerOnPlayer();
    }
});

// --- Pan / Zoom ---

function onWheel(e: WheelEvent) {
    e.preventDefault();
    const zoomFactor = e.deltaY > 0 ? 1.15 : 0.87;
    const newW = viewBox.value.w * zoomFactor;
    const newH = viewBox.value.h * zoomFactor;
    const svgEl = (e.currentTarget as SVGElement).getBoundingClientRect();
    const mouseRatioX = (e.clientX - svgEl.left) / svgEl.width;
    const mouseRatioY = (e.clientY - svgEl.top) / svgEl.height;
    viewBox.value = {
        x: viewBox.value.x - (newW - viewBox.value.w) * mouseRatioX,
        y: viewBox.value.y - (newH - viewBox.value.h) * mouseRatioY,
        w: newW,
        h: newH,
    };
}

function onMouseDown(e: MouseEvent) {
    isDragging = true;
    dragStartX = e.clientX;
    dragStartY = e.clientY;
    dragStartVbX = viewBox.value.x;
    dragStartVbY = viewBox.value.y;
}

function onMouseMove(e: MouseEvent) {
    if (!isDragging) return;
    const svgEl = (e.currentTarget as SVGElement).getBoundingClientRect();
    const scaleX = viewBox.value.w / svgEl.width;
    const scaleY = viewBox.value.h / svgEl.height;
    viewBox.value = {
        ...viewBox.value,
        x: dragStartVbX - (e.clientX - dragStartX) * scaleX,
        y: dragStartVbY - (e.clientY - dragStartY) * scaleY,
    };
}

function onMouseUp() {
    isDragging = false;
}

function onMouseLeave() {
    isDragging = false;
}

const viewBoxStr = computed(
    () =>
        `${viewBox.value.x} ${viewBox.value.y} ${viewBox.value.w} ${viewBox.value.h}`,
);

const cellList = computed(() => Array.from(mapStore.cells.values()));

// --- Graphics layer (HR-405/406/407) ---

interface ForestRender {
  key: string;
  x0: number;
  y0: number;
  biome: ForestBiome;
  trees: TreePlacement[];
}

interface SettlementRender {
  key: string;
  x0: number;
  y0: number;
  size: string;
  houses: HousePlacement[];
}

const forestRenders = computed<ForestRender[]>(() => {
  const cells = mapStore.cells;
  const rows = mapStore.height || 1;
  const out: ForestRender[] = [];
  for (const cell of cells.values()) {
    if (cell.terrain !== "forest" || cell.explored !== true && cell.explored !== 1)
      continue;
    const { x, y } = cellToPixel(cell.q, cell.r);
    out.push({
      key: `${cell.q},${cell.r}`,
      x0: x,
      y0: y,
      biome: forestBiome(cell, rows, neighborTerrains(cell.q, cell.r, cells)),
      trees: forestTrees(
        cell.q,
        cell.r,
        forestNeighborCount(cell.q, cell.r, cells),
      ),
    });
  }
  return out;
});

interface WaterRender {
  key: string;
  x0: number;
  y0: number;
  kind: WaterKind;
  water: OrthoFlags; // which orthogonal neighbours are also water
  ripple: number; // deterministic ripple offset
}

const waterRenders = computed<WaterRender[]>(() => {
  const cells = mapStore.cells;
  const out: WaterRender[] = [];
  for (const cell of cells.values()) {
    if (cell.terrain !== "water") continue;
    if (cell.explored !== true && cell.explored !== 1) continue;
    const { x, y } = cellToPixel(cell.q, cell.r);
    out.push({
      key: `${cell.q},${cell.r}`,
      x0: x,
      y0: y,
      kind: waterKind(cell, cells),
      water: orthoWater(cell.q, cell.r, cells),
      ripple: 6 + hashUnit(cell.q, cell.r, 7) * 6,
    });
  }
  return out;
});

const settlementRenders = computed<SettlementRender[]>(() => {
  const out: SettlementRender[] = [];
  for (const cell of mapStore.cells.values()) {
    if (!cell.features.includes("settlement")) continue;
    if (cell.explored !== true && cell.explored !== 1) continue;
    const { x, y } = cellToPixel(cell.q, cell.r);
    const size = settlementSize(cell);
    out.push({
      key: `${cell.q},${cell.r}`,
      x0: x,
      y0: y,
      size,
      houses: settlementHouses(size),
    });
  }
  return out;
});

// --- Terrain + feature stamps (HR-740..745) ---
// All stamps draw only on fully-explored cells (explored === true | 1), matching
// the forest/water/settlement layers above.

interface MountainRender { key: string; x0: number; y0: number; peaks: PeakPlacement[] }
interface RuinRender { key: string; x0: number; y0: number; fragments: RuinFragment[] }
interface BarrenRender {
  key: string;
  x0: number;
  y0: number;
  kind: BarrenKind;
  dunes: Dune[];
  cracks: Crack[];
}
interface RoadRender { key: string; x0: number; y0: number; road: OrthoFlags; isolated: boolean }
interface MarkerRender { key: string; x0: number; y0: number }

const mountainRenders = computed<MountainRender[]>(() => {
  const out: MountainRender[] = [];
  for (const cell of mapStore.cells.values()) {
    if (cell.terrain !== "mountains") continue;
    if (cell.explored !== true && cell.explored !== 1) continue;
    const { x, y } = cellToPixel(cell.q, cell.r);
    out.push({ key: `${cell.q},${cell.r}`, x0: x, y0: y, peaks: mountainPeaks(cell.q, cell.r) });
  }
  return out;
});

const ruinRenders = computed<RuinRender[]>(() => {
  const out: RuinRender[] = [];
  for (const cell of mapStore.cells.values()) {
    if (cell.terrain !== "ruins" && !cell.features.includes("ruins")) continue;
    if (cell.explored !== true && cell.explored !== 1) continue;
    const { x, y } = cellToPixel(cell.q, cell.r);
    out.push({ key: `${cell.q},${cell.r}`, x0: x, y0: y, fragments: ruinFragments(cell.q, cell.r) });
  }
  return out;
});

const barrenRenders = computed<BarrenRender[]>(() => {
  const out: BarrenRender[] = [];
  for (const cell of mapStore.cells.values()) {
    if (cell.terrain !== "desert" && cell.terrain !== "wasteland") continue;
    if (cell.explored !== true && cell.explored !== 1) continue;
    const { x, y } = cellToPixel(cell.q, cell.r);
    const kind = barrenKind(cell.terrain);
    out.push({
      key: `${cell.q},${cell.r}`,
      x0: x,
      y0: y,
      kind,
      dunes: kind === "desert" ? desertDunes(cell.q, cell.r) : [],
      cracks: kind === "wasteland" ? wastelandCracks(cell.q, cell.r) : [],
    });
  }
  return out;
});

const roadRenders = computed<RoadRender[]>(() => {
  const cells = mapStore.cells;
  const out: RoadRender[] = [];
  for (const cell of cells.values()) {
    if (!isRoadAt(cells, cell.q, cell.r)) continue;
    if (cell.explored !== true && cell.explored !== 1) continue;
    const { x, y } = cellToPixel(cell.q, cell.r);
    const road = orthoRoad(cell.q, cell.r, cells);
    out.push({
      key: `${cell.q},${cell.r}`,
      x0: x,
      y0: y,
      road,
      isolated: !road.n && !road.e && !road.s && !road.w,
    });
  }
  return out;
});

const objectiveRenders = computed<MarkerRender[]>(() => {
  const out: MarkerRender[] = [];
  for (const cell of mapStore.cells.values()) {
    if (!hasAnyFeature(cell, OBJECTIVE_FEATURES)) continue;
    if (cell.explored !== true && cell.explored !== 1) continue;
    const { x, y } = cellToPixel(cell.q, cell.r);
    out.push({ key: `${cell.q},${cell.r}`, x0: x, y0: y });
  }
  return out;
});

const encounterRenders = computed<MarkerRender[]>(() => {
  const out: MarkerRender[] = [];
  for (const cell of mapStore.cells.values()) {
    if (!hasAnyFeature(cell, ENCOUNTER_FEATURES)) continue;
    if (cell.explored !== true && cell.explored !== 1) continue;
    const { x, y } = cellToPixel(cell.q, cell.r);
    out.push({ key: `${cell.q},${cell.r}`, x0: x, y0: y });
  }
  return out;
});

// HR-787: loot indicators from recoverable death-marker loot, one tiered gem
// per explored cell that holds loot (rarity tier from the pile's top value).
interface LootRender {
  key: string;
  q: number;
  r: number;
  x0: number;
  y0: number;
  tier: LootTier;
}
const lootRenders = computed<LootRender[]>(() => {
  const out: LootRender[] = [];
  for (const cell of mapStore.cells.values()) {
    if (cell.explored !== true && cell.explored !== 1) continue;
    const value = cellLootValue(cell);
    if (value === null) continue;
    const { x, y } = cellToPixel(cell.q, cell.r);
    out.push({
      key: `${cell.q},${cell.r}`,
      q: cell.q,
      r: cell.r,
      x0: x,
      y0: y,
      tier: lootTier(value),
    });
  }
  return out;
});

// Presentation helpers — build SVG `points` strings for the mountain/ruin
// polygons so the template stays readable.
function peakPoints(p: PeakPlacement): string {
  const apexY = p.baseY - p.h;
  return `${p.x},${apexY} ${p.x - p.w},${p.baseY} ${p.x + p.w},${p.baseY}`;
}
function peakShade(p: PeakPlacement): string {
  const apexY = p.baseY - p.h;
  return `${p.x},${apexY} ${p.x + p.w},${p.baseY} ${p.x},${p.baseY}`;
}
function peakSnow(p: PeakPlacement): string {
  const apexY = p.baseY - p.h;
  const capY = apexY + p.h * p.snow;
  const capW = p.w * p.snow;
  return `${p.x},${apexY} ${p.x - capW},${capY} ${p.x + capW},${capY}`;
}
function ruinPoints(f: RuinFragment): string {
  const base = 20;
  return `${f.x},${base} ${f.x + f.w},${base} ${f.x + f.w + f.lean},${f.topY + 1.6} ${f.x + f.lean},${f.topY}`;
}
</script>

<template>
    <div class="w-full h-full overflow-hidden bg-gray-950 relative select-none">
        <!-- Loading/empty state -->
        <div
            v-if="!mapStore.loaded"
            class="w-full h-full flex items-center justify-center text-gray-600 font-mono text-sm"
        >
            <span v-if="worldStore.activeWorld">Loading map&hellip;</span>
            <span v-else>No world loaded.</span>
        </div>

        <!-- SVG map -->
        <svg
            v-else
            ref="svgRef"
            class="w-full h-full cursor-grab"
            :class="{ 'cursor-grabbing': isDragging }"
            :viewBox="viewBoxStr"
            xmlns="http://www.w3.org/2000/svg"
            @wheel.passive="false"
            @wheel="onWheel"
            @mousedown="onMouseDown"
            @mousemove="onMouseMove"
            @mouseup="onMouseUp"
            @mouseleave="onMouseLeave"
            @contextmenu="onContextMenu"
            @click="closeMenu"
        >
            <!-- Grid cells -->
            <g v-for="cell in cellList" :key="`${cell.q},${cell.r}`">
                <template v-if="!cell.explored">
                    <!-- Fog / unexplored -->
                    <rect
                        :x="cellToPixel(cell.q, cell.r).x"
                        :y="cellToPixel(cell.q, cell.r).y"
                        :width="CELL_SIZE"
                        :height="CELL_SIZE"
                        :fill="FOG_COLOR"
                        stroke="#3a3a4a"
                        stroke-width="0.5"
                    />
                </template>
                <template v-else-if="cell.explored === 2">
                    <!-- Seen but not visited (adjacent impassable terrain) -->
                    <rect
                        :x="cellToPixel(cell.q, cell.r).x"
                        :y="cellToPixel(cell.q, cell.r).y"
                        :width="CELL_SIZE"
                        :height="CELL_SIZE"
                        :fill="terrainColor(cell.terrain)"
                        stroke="#2a2a3a"
                        stroke-width="0.5"
                        opacity="0.4"
                    />
                </template>
                <template v-else>
                    <!-- Explored terrain -->
                    <rect
                        :x="cellToPixel(cell.q, cell.r).x"
                        :y="cellToPixel(cell.q, cell.r).y"
                        :width="CELL_SIZE"
                        :height="CELL_SIZE"
                        :fill="terrainColor(cell.terrain)"
                        stroke="#222"
                        stroke-width="0.5"
                    />
                    <!-- Feature indicators (settlement uses an icon below) -->
                    <template v-for="feature in cell.features" :key="feature">
                        <text
                            v-if="FEATURE_LABELS[feature] && !STAMPED_FEATURES.has(feature)"
                            :x="cellToPixel(cell.q, cell.r).x + CELL_SIZE / 2"
                            :y="
                                cellToPixel(cell.q, cell.r).y +
                                CELL_SIZE / 2 +
                                3
                            "
                            text-anchor="middle"
                            dominant-baseline="middle"
                            :fill="FEATURE_LABELS[feature].color"
                            font-size="8"
                            font-family="monospace"
                            pointer-events="none"
                        >
                            {{ FEATURE_LABELS[feature].label }}
                        </text>
                    </template>
                </template>
            </g>

            <!-- Water bodies (HR-409/410): rivers (channels) + lakes (shore/ripples) -->
            <g
                v-for="w in waterRenders"
                :key="`water-${w.key}`"
                :transform="`translate(${w.x0}, ${w.y0})`"
                data-testid="water-feature"
                :data-water-kind="w.kind"
                pointer-events="none"
            >
                <template v-if="w.kind === 'river'">
                    <!-- Channel: connect the cell centre to each water-neighbour edge. -->
                    <g
                        :stroke="WATER_COLORS.channel"
                        stroke-width="3.5"
                        stroke-linecap="round"
                        fill="none"
                    >
                        <line v-if="w.water.n" x1="12" y1="12" x2="12" y2="0" />
                        <line v-if="w.water.e" x1="12" y1="12" x2="24" y2="12" />
                        <line v-if="w.water.s" x1="12" y1="12" x2="12" y2="24" />
                        <line v-if="w.water.w" x1="12" y1="12" x2="0" y2="12" />
                    </g>
                    <!-- Isolated source: a small spring/pool. -->
                    <circle
                        v-if="!w.water.n && !w.water.e && !w.water.s && !w.water.w"
                        cx="12"
                        cy="12"
                        r="4"
                        :fill="WATER_COLORS.channel"
                    />
                    <!-- A flow ripple highlight. -->
                    <line
                        x1="9"
                        :y1="w.ripple"
                        x2="15"
                        :y2="w.ripple"
                        :stroke="WATER_COLORS.ripple"
                        stroke-width="1"
                        stroke-linecap="round"
                    />
                </template>
                <template v-else>
                    <!-- Lake: shoreline on land-facing edges + a couple of ripples. -->
                    <g :stroke="WATER_COLORS.shore" stroke-width="1.5" fill="none">
                        <line v-if="!w.water.n" x1="1" y1="1.5" x2="23" y2="1.5" />
                        <line v-if="!w.water.s" x1="1" y1="22.5" x2="23" y2="22.5" />
                        <line v-if="!w.water.w" x1="1.5" y1="1" x2="1.5" y2="23" />
                        <line v-if="!w.water.e" x1="22.5" y1="1" x2="22.5" y2="23" />
                    </g>
                    <g
                        :stroke="WATER_COLORS.ripple"
                        stroke-width="1"
                        stroke-linecap="round"
                        fill="none"
                    >
                        <path :d="`M7,${w.ripple} q3,-2 6,0`" />
                        <path :d="`M11,${w.ripple + 5} q3,-2 6,0`" />
                    </g>
                </template>
            </g>

            <!-- Forest tree stamps (HR-406/407): drawn above terrain -->
            <g
                v-for="f in forestRenders"
                :key="`forest-${f.key}`"
                :transform="`translate(${f.x0}, ${f.y0})`"
                data-testid="forest-stamp"
                :data-biome="f.biome"
                pointer-events="none"
            >
                <g
                    v-for="(tree, ti) in f.trees"
                    :key="ti"
                    :transform="`translate(${tree.x}, ${tree.y}) scale(${tree.scale})`"
                >
                    <rect
                        x="-0.55"
                        y="-2.4"
                        width="1.1"
                        height="2.6"
                        :fill="BIOME_COLORS[f.biome].trunk"
                    />
                    <!-- temperate: round oak canopy -->
                    <template v-if="f.biome === 'temperate'">
                        <circle cx="0" cy="-4.4" r="2.7" :fill="BIOME_COLORS[f.biome].canopyDark" />
                        <circle cx="-0.9" cy="-5.1" r="1.7" :fill="BIOME_COLORS[f.biome].canopy" />
                    </template>
                    <!-- boreal: stacked fir triangles -->
                    <template v-else-if="f.biome === 'boreal'">
                        <polygon points="0,-7.4 -2.4,-2.6 2.4,-2.6" :fill="BIOME_COLORS[f.biome].canopyDark" />
                        <polygon points="0,-9.4 -1.7,-5.4 1.7,-5.4" :fill="BIOME_COLORS[f.biome].canopy" />
                    </template>
                    <!-- tropical: palm fronds -->
                    <template v-else>
                        <g
                            :stroke="BIOME_COLORS[f.biome].canopy"
                            stroke-width="0.85"
                            stroke-linecap="round"
                            fill="none"
                        >
                            <path d="M0,-4 Q-3,-6.2 -4.2,-3.4" />
                            <path d="M0,-4 Q3,-6.2 4.2,-3.4" />
                            <path d="M0,-4 Q-2.4,-7.8 -3,-6.2" />
                            <path d="M0,-4 Q2.4,-7.8 3,-6.2" />
                            <path d="M0,-4 Q0,-8 0,-8.4" />
                        </g>
                    </template>
                </g>
            </g>

            <!-- Roads (HR-743): auto-tiled path connecting road neighbours -->
            <g
                v-for="rd in roadRenders"
                :key="`road-${rd.key}`"
                :transform="`translate(${rd.x0}, ${rd.y0})`"
                data-testid="road-feature"
                pointer-events="none"
            >
                <template v-if="rd.isolated">
                    <circle cx="12" cy="12" r="3" :fill="ROAD_COLORS.path" :stroke="ROAD_COLORS.edge" stroke-width="1" />
                </template>
                <template v-else>
                    <g :stroke="ROAD_COLORS.path" stroke-width="4.5" stroke-linecap="round" fill="none">
                        <line v-if="rd.road.n" x1="12" y1="12" x2="12" y2="0" />
                        <line v-if="rd.road.e" x1="12" y1="12" x2="24" y2="12" />
                        <line v-if="rd.road.s" x1="12" y1="12" x2="12" y2="24" />
                        <line v-if="rd.road.w" x1="12" y1="12" x2="0" y2="12" />
                    </g>
                    <g
                        :stroke="ROAD_COLORS.dash"
                        stroke-width="0.8"
                        stroke-dasharray="2 2.5"
                        stroke-linecap="round"
                        fill="none"
                    >
                        <line v-if="rd.road.n" x1="12" y1="12" x2="12" y2="0" />
                        <line v-if="rd.road.e" x1="12" y1="12" x2="24" y2="12" />
                        <line v-if="rd.road.s" x1="12" y1="12" x2="12" y2="24" />
                        <line v-if="rd.road.w" x1="12" y1="12" x2="0" y2="12" />
                    </g>
                </template>
            </g>

            <!-- Barren terrain (HR-742): desert dunes / wasteland cracked earth -->
            <g
                v-for="b in barrenRenders"
                :key="`barren-${b.key}`"
                :transform="`translate(${b.x0}, ${b.y0})`"
                data-testid="barren-stamp"
                :data-barren-kind="b.kind"
                pointer-events="none"
            >
                <template v-if="b.kind === 'desert'">
                    <path
                        v-for="(d, di) in b.dunes"
                        :key="di"
                        :d="`M0,${d.y} Q12,${d.y - d.amp} 24,${d.y} L24,24 L0,24 Z`"
                        :fill="di % 2 ? DESERT_COLORS.duneDark : DESERT_COLORS.dune"
                        opacity="0.5"
                    />
                    <ellipse cx="16" cy="18" rx="2.4" ry="1.4" :fill="DESERT_COLORS.rock" opacity="0.8" />
                </template>
                <template v-else>
                    <g :stroke="WASTELAND_COLORS.crack" stroke-width="0.8" stroke-linecap="round" fill="none">
                        <line
                            v-for="(c, ci) in b.cracks"
                            :key="ci"
                            :x1="c.x1"
                            :y1="c.y1"
                            :x2="c.x2"
                            :y2="c.y2"
                        />
                    </g>
                    <g :stroke="WASTELAND_COLORS.shrub" stroke-width="0.7" stroke-linecap="round" fill="none">
                        <path d="M12,18 L12,13" />
                        <path d="M12,15 L10,12.5" />
                        <path d="M12,15 L14,12.5" />
                    </g>
                </template>
            </g>

            <!-- Mountain peaks (HR-741): snow-capped triangles -->
            <g
                v-for="m in mountainRenders"
                :key="`mtn-${m.key}`"
                :transform="`translate(${m.x0}, ${m.y0})`"
                data-testid="mountain-stamp"
                pointer-events="none"
            >
                <g v-for="(p, pi) in m.peaks" :key="pi">
                    <polygon :points="peakPoints(p)" :fill="MOUNTAIN_COLORS.rock" />
                    <polygon :points="peakShade(p)" :fill="MOUNTAIN_COLORS.rockDark" />
                    <polygon :points="peakSnow(p)" :fill="MOUNTAIN_COLORS.snow" />
                </g>
            </g>

            <!-- Ruins (HR-740): broken pillar fragments -->
            <g
                v-for="ru in ruinRenders"
                :key="`ruin-${ru.key}`"
                :transform="`translate(${ru.x0}, ${ru.y0})`"
                data-testid="ruin-stamp"
                pointer-events="none"
            >
                <rect x="3" y="19.5" width="18" height="2" :fill="RUIN_COLORS.stoneDark" opacity="0.7" />
                <g v-for="(f, fi) in ru.fragments" :key="fi">
                    <polygon :points="ruinPoints(f)" :fill="RUIN_COLORS.stone" />
                    <line
                        :x1="f.x + f.w + f.lean"
                        :y1="f.topY + 1.6"
                        :x2="f.x + f.w"
                        y2="20"
                        :stroke="RUIN_COLORS.stoneDark"
                        stroke-width="0.7"
                    />
                </g>
            </g>

            <!-- Settlement icons (HR-405): house cluster scaled by size -->
            <g
                v-for="s in settlementRenders"
                :key="`settle-${s.key}`"
                :transform="`translate(${s.x0}, ${s.y0})`"
                data-testid="settlement-icon"
                :data-size="s.size"
                pointer-events="none"
            >
                <g
                    v-for="(house, hi) in s.houses"
                    :key="hi"
                    :transform="`translate(${house.x}, ${house.y}) scale(${house.scale})`"
                >
                    <template v-if="house.keep">
                        <!-- keep / tower -->
                        <rect x="-1.8" y="-6.5" width="3.6" height="6.5" :fill="SETTLEMENT_COLORS.keep" />
                        <polygon points="-2.2,-6.5 0,-8.6 2.2,-6.5" :fill="SETTLEMENT_COLORS.roof" />
                    </template>
                    <template v-else>
                        <rect x="-2" y="-3" width="4" height="3" :fill="SETTLEMENT_COLORS.wall" />
                        <rect x="-2" y="-1.4" width="4" height="1.4" :fill="SETTLEMENT_COLORS.wallDark" />
                        <polygon points="-2.6,-3 0,-5.4 2.6,-3" :fill="SETTLEMENT_COLORS.roof" />
                    </template>
                </g>
            </g>

            <!-- Objective markers (HR-744): quest flag -->
            <g
                v-for="o in objectiveRenders"
                :key="`obj-${o.key}`"
                :transform="`translate(${o.x0}, ${o.y0})`"
                data-testid="objective-marker"
                pointer-events="none"
            >
                <circle cx="12" cy="12" r="7" :fill="OBJECTIVE_COLORS.glow" opacity="0.18" />
                <line x1="10" y1="5.5" x2="10" y2="18" :stroke="OBJECTIVE_COLORS.pole" stroke-width="1.2" stroke-linecap="round" />
                <polygon points="10,6 18,8.4 10,11" :fill="OBJECTIVE_COLORS.flag" />
            </g>

            <!-- Monster-encounter markers (HR-745): skull -->
            <g
                v-for="e in encounterRenders"
                :key="`enc-${e.key}`"
                :transform="`translate(${e.x0}, ${e.y0})`"
                data-testid="encounter-marker"
                pointer-events="none"
            >
                <g transform="translate(12, 11)">
                    <circle cx="0" cy="0" r="4.2" :fill="ENCOUNTER_COLORS.bone" />
                    <rect x="-2.4" y="2.8" width="4.8" height="3.2" rx="1" :fill="ENCOUNTER_COLORS.bone" />
                    <circle cx="-1.7" cy="-0.3" r="1.3" :fill="ENCOUNTER_COLORS.eye" />
                    <circle cx="1.7" cy="-0.3" r="1.3" :fill="ENCOUNTER_COLORS.eye" />
                    <polygon points="0,1.2 -0.9,2.8 0.9,2.8" :fill="ENCOUNTER_COLORS.boneShadow" />
                    <line x1="-1" y1="3" x2="-1" y2="6" :stroke="ENCOUNTER_COLORS.boneShadow" stroke-width="0.6" />
                    <line x1="1" y1="3" x2="1" y2="6" :stroke="ENCOUNTER_COLORS.boneShadow" stroke-width="0.6" />
                </g>
            </g>

            <!-- Loot indicators (HR-787): rarity-tiered gem on cells with loot -->
            <g
                v-for="l in lootRenders"
                :key="`loot-${l.key}`"
                :transform="`translate(${l.x0}, ${l.y0})`"
                :data-testid="`loot-indicator-${l.q}-${l.r}`"
                :data-loot-tier="l.tier"
                pointer-events="none"
            >
                <circle
                    cx="12"
                    cy="12"
                    :r="l.tier === 'legendary' ? 6 : l.tier === 'epic' ? 5 : 4"
                    :fill="LOOT_COLORS[l.tier]"
                    :opacity="l.tier === 'legendary' ? 0.3 : l.tier === 'epic' ? 0.22 : 0.14"
                />
                <polygon
                    :points="lootGemPoints(l.tier)"
                    :fill="LOOT_COLORS[l.tier]"
                    stroke="#1a1a1a"
                    stroke-width="0.4"
                />
                <g
                    v-if="l.tier === 'legendary'"
                    :stroke="LOOT_COLORS.legendary"
                    stroke-width="0.7"
                    stroke-linecap="round"
                >
                    <line x1="12" y1="3.5" x2="12" y2="6" />
                    <line x1="12" y1="18" x2="12" y2="20.5" />
                    <line x1="3.5" y1="12" x2="6" y2="12" />
                    <line x1="18" y1="12" x2="20.5" y2="12" />
                </g>
            </g>

            <!-- Player position marker -->
            <circle
                v-if="mapStore.playerQ !== null && mapStore.playerR !== null"
                data-testid="player-marker"
                :cx="
                    cellToPixel(mapStore.playerQ, mapStore.playerR).x +
                    CELL_SIZE / 2
                "
                :cy="
                    cellToPixel(mapStore.playerQ, mapStore.playerR).y +
                    CELL_SIZE / 2
                "
                r="5"
                :fill="PLAYER_COLOR"
                stroke="#000"
                stroke-width="1"
                pointer-events="none"
            />
        </svg>

        <!-- Map overlays -->
        <div v-if="mapStore.loaded" class="absolute bottom-2 left-2 flex gap-2">
            <button
                class="text-[10px] font-mono text-gray-300 bg-gray-900 border border-gray-700 hover:border-gray-500 hover:text-white px-2 py-1 rounded transition-colors shadow-lg"
                @click="showLegend = true"
            >
                [ Legend ]
            </button>
        </div>

        <!-- Mini help -->
        <div
            v-if="mapStore.loaded"
            class="absolute bottom-2 right-2 text-[10px] font-mono text-gray-500 bg-gray-950 bg-opacity-80 px-2 py-1 rounded pointer-events-none border border-gray-800"
        >
            Scroll: Zoom &middot; Drag: Pan
        </div>

        <!-- Resume-travel banner (HR-408) -->
        <div
            v-if="gameStore.pendingTravel"
            data-testid="travel-banner"
            class="absolute top-2 left-1/2 -translate-x-1/2 flex items-center gap-2 bg-gray-900/95 border border-amber-700 rounded px-3 py-1.5 shadow-lg text-xs font-mono"
        >
            <span class="text-amber-200">
                Travel to ({{ gameStore.pendingTravel.q }},
                {{ gameStore.pendingTravel.r }})?
            </span>
            <button
                data-testid="travel-resume"
                class="px-2 py-0.5 rounded border border-emerald-600 text-emerald-300 hover:bg-emerald-900/40"
                @click="resumeTravel"
            >
                Resume
            </button>
            <button
                data-testid="travel-cancel"
                class="px-2 py-0.5 rounded border border-gray-600 text-gray-300 hover:bg-gray-800"
                @click="cancelTravel"
            >
                Cancel
            </button>
        </div>

        <!-- Right-click cell menu (HR-408) -->
        <div
            v-if="menu.open"
            data-testid="cell-menu"
            class="fixed z-50 min-w-[9rem] bg-gray-900 border border-gray-700 rounded shadow-2xl text-xs font-mono py-1"
            :style="{ left: `${menu.x}px`, top: `${menu.y}px` }"
            @click.stop
            @contextmenu.prevent
        >
            <div class="px-3 py-1 text-[10px] text-gray-500 border-b border-gray-800">
                Cell ({{ menu.q }}, {{ menu.r }})
            </div>
            <button
                data-testid="cell-menu-move"
                class="block w-full text-left px-3 py-1.5 text-gray-200 hover:bg-gray-800"
                @click="travel('')"
            >
                Move here
            </button>
            <button
                data-testid="cell-menu-look"
                class="block w-full text-left px-3 py-1.5 text-gray-200 hover:bg-gray-800"
                @click="travel('look')"
            >
                Look here
            </button>
            <button
                v-if="menuHasSettlement"
                data-testid="cell-menu-enter"
                class="block w-full text-left px-3 py-1.5 text-gray-200 hover:bg-gray-800"
                @click="travel('enter')"
            >
                Enter town
            </button>
            <div class="relative">
                <button
                    data-testid="cell-menu-skills"
                    class="flex w-full items-center justify-between px-3 py-1.5 text-gray-200 hover:bg-gray-800"
                    @click="skillsOpen = !skillsOpen"
                >
                    <span>Use skill</span><span class="text-gray-500">▸</span>
                </button>
                <div
                    v-if="skillsOpen"
                    data-testid="cell-menu-skills-submenu"
                    class="mt-0.5 border-t border-gray-800 bg-gray-950"
                >
                    <button
                        v-for="s in TRAVEL_SKILLS"
                        :key="s.id"
                        :data-testid="`cell-menu-skill-${s.id}`"
                        class="block w-full text-left px-5 py-1.5 text-gray-300 hover:bg-gray-800"
                        @click="travelSkill(s.id)"
                    >
                        {{ s.label }}
                    </button>
                </div>
            </div>
            <button
                data-testid="cell-menu-cancel"
                class="block w-full text-left px-3 py-1.5 text-gray-500 hover:bg-gray-800 border-t border-gray-800"
                @click="closeMenu"
            >
                Cancel
            </button>
        </div>

        <!-- Click-away catcher for the menu -->
        <div
            v-if="menu.open"
            class="fixed inset-0 z-40"
            @click="closeMenu"
            @contextmenu.prevent="closeMenu"
        />

        <!-- Legend Overlay -->
        <MapLegend :show="showLegend" @close="showLegend = false" />
    </div>
</template>
