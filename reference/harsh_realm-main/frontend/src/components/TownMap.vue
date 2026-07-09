<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { useTownStore } from "../stores/town";

const townStore = useTownStore();

const CELL_SIZE = 16;

const TERRAIN_COLORS: Record<string, string> = {
  road: "#6a6a5a",
  plaza: "#7a7a6a",
  open_ground: "#3a4a3a",
  building: "#4a3535",
  shop: "#4a5a30",
  tavern: "#5a4a20",
  temple: "#3a3a5a",
};

const TERRAIN_LABELS: Record<string, { label: string; color: string }> = {
  shop: { label: "$", color: "#8f8" },
  tavern: { label: "T", color: "#fa0" },
  temple: { label: "+", color: "#aaf" },
  building: { label: "#", color: "#888" },
};

const PLAYER_COLOR = "#f0c040";

// ViewBox state for pan/zoom
const viewBox = ref({ x: 0, y: 0, w: 400, h: 400 });

let isDragging = false;
let dragStartX = 0;
let dragStartY = 0;
let dragStartVbX = 0;
let dragStartVbY = 0;

function cellToPixel(q: number, r: number) {
  return { x: q * CELL_SIZE, y: r * CELL_SIZE };
}

function centerOnPlayer() {
  const { x, y } = cellToPixel(townStore.playerQ, townStore.playerR);
  viewBox.value = {
    x: x - viewBox.value.w / 2 + CELL_SIZE / 2,
    y: y - viewBox.value.h / 2 + CELL_SIZE / 2,
    w: viewBox.value.w,
    h: viewBox.value.h,
  };
}

watch(
  () => [townStore.playerQ, townStore.playerR],
  () => centerOnPlayer(),
);

watch(
  () => townStore.active,
  (active) => { if (active) centerOnPlayer(); },
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

function onMouseUp() { isDragging = false; }
function onMouseLeave() { isDragging = false; }

const viewBoxStr = computed(
  () => `${viewBox.value.x} ${viewBox.value.y} ${viewBox.value.w} ${viewBox.value.h}`
);
</script>

<template>
  <div class="w-full h-full overflow-hidden bg-gray-950 relative select-none">
    <div
      v-if="!townStore.active"
      class="w-full h-full flex items-center justify-center text-gray-600 font-mono text-sm"
    >
      Not in a town.
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
      <!-- Grid cells -->
      <g v-for="cell in townStore.cells" :key="`${cell.q},${cell.r}`">
        <rect
          :x="cellToPixel(cell.q, cell.r).x"
          :y="cellToPixel(cell.q, cell.r).y"
          :width="CELL_SIZE"
          :height="CELL_SIZE"
          :fill="TERRAIN_COLORS[cell.terrain] ?? '#3a3a3a'"
          stroke="#222"
          stroke-width="0.3"
        />
        <!-- Building/shop/tavern/temple labels -->
        <text
          v-if="TERRAIN_LABELS[cell.terrain]"
          :x="cellToPixel(cell.q, cell.r).x + CELL_SIZE / 2"
          :y="cellToPixel(cell.q, cell.r).y + CELL_SIZE / 2 + 2"
          text-anchor="middle"
          dominant-baseline="middle"
          :fill="TERRAIN_LABELS[cell.terrain].color"
          font-size="7"
          font-family="monospace"
          pointer-events="none"
        >{{ TERRAIN_LABELS[cell.terrain].label }}</text>
      </g>

      <!-- Player marker -->
      <circle
        :cx="cellToPixel(townStore.playerQ, townStore.playerR).x + CELL_SIZE / 2"
        :cy="cellToPixel(townStore.playerQ, townStore.playerR).y + CELL_SIZE / 2"
        r="4"
        :fill="PLAYER_COLOR"
        stroke="#000"
        stroke-width="0.8"
        pointer-events="none"
      />
    </svg>

    <!-- Legend -->
    <div
      v-if="townStore.active"
      class="absolute bottom-1 left-1 text-[9px] font-mono text-gray-500 bg-gray-950/80 px-1.5 py-1 rounded pointer-events-none leading-tight"
    >
      <span class="text-green-400">$</span> Shop
      <span class="text-amber-400 ml-1">T</span> Tavern
      <span class="text-blue-300 ml-1">+</span> Temple
      <span class="text-gray-400 ml-1">#</span> Building
    </div>

    <!-- Town name -->
    <div
      v-if="townStore.active"
      class="absolute top-1 left-1 text-[10px] font-mono text-gray-400 bg-gray-950/80 px-1.5 py-0.5 rounded pointer-events-none"
    >
      {{ townStore.settlementName }}
    </div>
  </div>
</template>
