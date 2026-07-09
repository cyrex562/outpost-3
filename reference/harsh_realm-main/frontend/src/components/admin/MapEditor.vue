<script setup lang="ts">
import { ref, computed } from "vue";

export interface AdminCell {
  q: number;
  r: number;
  terrain: string;
  features: string[];
  explored: boolean;
  faction_id?: string | undefined;
}

const props = defineProps<{
  cells: AdminCell[];
  selectedQ: number | null;
  selectedR: number | null;
}>();

const emit = defineEmits<{
  cellClick: [q: number, r: number];
}>();

const CELL_SIZE = 24;
const SELECTED_STROKE = "#f0c040";
const FOG_COLOR = "#1e1e2a";

const TERRAIN_COLORS: Record<string, string> = {
  plains: "#4a5d3a",
  forest: "#2d4a2d",
  hills: "#6b6b4f",
  mountains: "#5a5a5a",
  water: "#2a3d5a",
  swamp: "#3d4a3a",
  desert: "#7a6a4a",
  jungle: "#1e4a1e",
  tundra: "#6a7a8a",
  wasteland: "#5a4a3a",
  coast: "#3a5a6a",
  ruins: "#4a4a5a",
  corridor: "#5a5040",
  room_floor: "#6a6050",
  wall: "#2a2a2a",
  door: "#8a7040",
  stairs_up: "#50706a",
  stairs_down: "#705060",
  trap_floor: "#6a3030",
  road: "#6a6a5a",
  building: "#4a4040",
  shop: "#5a6a40",
  tavern: "#6a5a30",
  temple: "#4a4a6a",
  open_ground: "#4a5a3a",
  plaza: "#6a6a6a",
};

const FEATURE_LABELS: Record<string, { label: string; color: string }> = {
  settlement: { label: "S", color: "white" },
  ruins: { label: "R", color: "#888" },
  lair: { label: "L", color: "#f44" },
  landmark: { label: "\u2605", color: "#ff0" },
};

function terrainColor(terrain: string): string {
  return TERRAIN_COLORS[terrain] ?? "#3a3a3a";
}

// ViewBox state for pan/zoom
const viewBox = ref({ x: -20, y: -20, w: 600, h: 500 });

// Drag state
let isDragging = false;
let dragStartX = 0;
let dragStartY = 0;
let dragStartVbX = 0;
let dragStartVbY = 0;

const viewBoxStr = computed(
  () =>
    `${viewBox.value.x} ${viewBox.value.y} ${viewBox.value.w} ${viewBox.value.h}`
);

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

function isSelected(q: number, r: number): boolean {
  return props.selectedQ === q && props.selectedR === r;
}

function handleCellClick(q: number, r: number, e: MouseEvent) {
  e.stopPropagation();
  emit("cellClick", q, r);
}
</script>

<template>
  <div class="w-full h-full overflow-hidden bg-gray-950 relative select-none">
    <div
      v-if="cells.length === 0"
      class="w-full h-full flex items-center justify-center text-gray-600 font-mono text-sm"
    >
      No cells loaded.
    </div>

    <svg
      v-else
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
    >
      <g v-for="cell in cells" :key="`${cell.q},${cell.r}`">
        <!-- Cell rectangle -->
        <rect
          :x="cell.q * CELL_SIZE"
          :y="cell.r * CELL_SIZE"
          :width="CELL_SIZE"
          :height="CELL_SIZE"
          :fill="cell.explored ? terrainColor(cell.terrain) : FOG_COLOR"
          :stroke="isSelected(cell.q, cell.r) ? SELECTED_STROKE : '#222'"
          :stroke-width="isSelected(cell.q, cell.r) ? 2 : 0.5"
          class="cursor-pointer"
          @click="handleCellClick(cell.q, cell.r, $event)"
        />
        <!-- Feature indicators -->
        <template v-for="feature in cell.features" :key="feature">
          <text
            v-if="FEATURE_LABELS[feature]"
            :x="cell.q * CELL_SIZE + CELL_SIZE / 2"
            :y="cell.r * CELL_SIZE + CELL_SIZE / 2 + 3"
            text-anchor="middle"
            dominant-baseline="middle"
            :fill="FEATURE_LABELS[feature].color"
            font-size="8"
            font-family="monospace"
            pointer-events="none"
          >{{ FEATURE_LABELS[feature].label }}</text>
        </template>
        <!-- Coord label (small, for admin context) -->
        <text
          :x="cell.q * CELL_SIZE + CELL_SIZE / 2"
          :y="cell.r * CELL_SIZE + 5"
          text-anchor="middle"
          fill="#555"
          font-size="4"
          font-family="monospace"
          pointer-events="none"
        >{{ cell.q }},{{ cell.r }}</text>
      </g>
    </svg>

    <div
      v-if="cells.length > 0"
      class="absolute bottom-2 right-2 text-xs font-mono text-gray-500 bg-gray-950 bg-opacity-80 px-2 py-1 rounded pointer-events-none"
    >
      Scroll to zoom · Drag to pan · Click to select
    </div>
  </div>
</template>
