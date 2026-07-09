<script setup lang="ts">
import { computed, onUnmounted } from "vue";
import { useLayoutStore } from "../stores/layout";

const props = withDefaults(
  defineProps<{
    panelId: string;
    title: string;
    minWidth?: number;
    minHeight?: number;
    closable?: boolean;
  }>(),
  {
    minWidth: 200,
    minHeight: 150,
    closable: false,
  }
);

const layoutStore = useLayoutStore();

if (!layoutStore.panels[props.panelId]) {
  layoutStore.updatePanel(props.panelId, {});
}

const panel = computed(() => layoutStore.panels[props.panelId]);

// Outer div: position + size only, overflow:visible so resize grips can bleed outside
const panelStyle = computed(() => ({
  position: "fixed" as const,
  left: `${panel.value.x}px`,
  top: `${panel.value.y}px`,
  width: `${panel.value.width}px`,
  height: panel.value.minimized ? "36px" : `${panel.value.height}px`,
  zIndex: panel.value.zIndex,
  pointerEvents: "auto" as const,
  overflow: "visible" as const,
}));

// Height of the fixed app header — panels must not be dragged above it
const HEADER_HEIGHT = 44;

// ----------------------------------------------------------------
// Move logic (title bar + side strips)
// ----------------------------------------------------------------
let dragOffsetX = 0;
let dragOffsetY = 0;

function startMove(event: MouseEvent) {
  if (event.button !== 0) return;
  if ((event.target as HTMLElement).closest("button")) return;
  event.preventDefault();
  layoutStore.bringToFront(props.panelId);
  dragOffsetX = event.clientX - panel.value.x;
  dragOffsetY = event.clientY - panel.value.y;
  document.addEventListener("mousemove", onDragMove);
  document.addEventListener("mouseup", onDragEnd);
  document.body.style.userSelect = "none";
}

function onDragMove(event: MouseEvent) {
  const minVisible = 40;
  layoutStore.updatePanel(props.panelId, {
    x: Math.max(-(panel.value.width - minVisible), Math.min(window.innerWidth - minVisible, event.clientX - dragOffsetX)),
    y: Math.max(HEADER_HEIGHT, Math.min(window.innerHeight - minVisible, event.clientY - dragOffsetY)),
  });
}

function onDragEnd() {
  document.removeEventListener("mousemove", onDragMove);
  document.removeEventListener("mouseup", onDragEnd);
  document.body.style.userSelect = "";
  layoutStore.saveLayout();
}

// ----------------------------------------------------------------
// Resize logic — 8 directions
// ----------------------------------------------------------------
type ResizeDir = "n" | "ne" | "e" | "se" | "s" | "sw" | "w" | "nw";

let resizeDir: ResizeDir | null = null;
let rsX = 0, rsY = 0, rsLeft = 0, rsTop = 0, rsW = 0, rsH = 0;

function startResize(dir: ResizeDir, event: MouseEvent) {
  if (event.button !== 0) return;
  event.preventDefault();
  event.stopPropagation();
  resizeDir = dir;
  rsX = event.clientX;
  rsY = event.clientY;
  rsLeft = panel.value.x;
  rsTop = panel.value.y;
  rsW = panel.value.width;
  rsH = panel.value.height;
  layoutStore.bringToFront(props.panelId);
  document.addEventListener("mousemove", onResizeMove);
  document.addEventListener("mouseup", onResizeEnd);
  document.body.style.userSelect = "none";
}

function onResizeMove(event: MouseEvent) {
  const dx = event.clientX - rsX;
  const dy = event.clientY - rsY;
  const dir = resizeDir!;

  let x = rsLeft, y = rsTop, w = rsW, h = rsH;

  if (dir.includes("e")) w = Math.max(props.minWidth, rsW + dx);
  if (dir.includes("s")) h = Math.max(props.minHeight, rsH + dy);
  if (dir.includes("w")) {
    w = Math.max(props.minWidth, rsW - dx);
    x = rsLeft + rsW - w;
  }
  if (dir.includes("n")) {
    h = Math.max(props.minHeight, rsH - dy);
    y = rsTop + rsH - h;
    if (y < HEADER_HEIGHT) { y = HEADER_HEIGHT; h = rsTop + rsH - HEADER_HEIGHT; }
  }

  layoutStore.updatePanel(props.panelId, { x, y, width: w, height: h });
}

function onResizeEnd() {
  document.removeEventListener("mousemove", onResizeMove);
  document.removeEventListener("mouseup", onResizeEnd);
  document.body.style.userSelect = "";
  resizeDir = null;
  layoutStore.saveLayout();
}

// ----------------------------------------------------------------
// Panel controls
// ----------------------------------------------------------------
function onPanelClick() {
  layoutStore.bringToFront(props.panelId);
}

function toggleMinimize() {
  layoutStore.updatePanel(props.panelId, { minimized: !panel.value.minimized });
  layoutStore.saveLayout();
}

function closePanel() {
  layoutStore.updatePanel(props.panelId, { visible: false });
  layoutStore.saveLayout();
}

// ----------------------------------------------------------------
// Cleanup on unmount
// ----------------------------------------------------------------
onUnmounted(() => {
  document.removeEventListener("mousemove", onDragMove);
  document.removeEventListener("mouseup", onDragEnd);
  document.removeEventListener("mousemove", onResizeMove);
  document.removeEventListener("mouseup", onResizeEnd);
  document.body.style.userSelect = "";
});
</script>

<template>
  <div
    v-if="panel && panel.visible"
    :style="panelStyle"
    @mousedown="onPanelClick"
  >
    <!--
      Resize grip layer — sits at inset:-4px so grips bleed 4px outside the
      window border, giving a generous hit-area without being visually distracting.
      pointer-events:none on wrapper; each grip re-enables pointer-events.
    -->
    <div class="absolute pointer-events-none" style="inset: -4px; z-index: 30;">
      <!-- corners (16×16 px) -->
      <div class="absolute top-0    left-0    w-4 h-4 cursor-nw-resize pointer-events-auto" @mousedown.stop="startResize('nw', $event)" />
      <div class="absolute top-0    right-0   w-4 h-4 cursor-ne-resize pointer-events-auto" @mousedown.stop="startResize('ne', $event)" />
      <div class="absolute bottom-0 right-0   w-4 h-4 cursor-se-resize pointer-events-auto" @mousedown.stop="startResize('se', $event)" />
      <div class="absolute bottom-0 left-0    w-4 h-4 cursor-sw-resize pointer-events-auto" @mousedown.stop="startResize('sw', $event)" />
      <!-- edges (between corners) -->
      <div class="absolute top-0    left-4 right-4  h-2 cursor-n-resize pointer-events-auto" @mousedown.stop="startResize('n', $event)" />
      <div class="absolute bottom-0 left-4 right-4  h-2 cursor-s-resize pointer-events-auto" @mousedown.stop="startResize('s', $event)" />
      <div class="absolute left-0   top-4  bottom-4 w-2 cursor-w-resize pointer-events-auto" @mousedown.stop="startResize('w', $event)" />
      <div class="absolute right-0  top-4  bottom-4 w-2 cursor-e-resize pointer-events-auto" @mousedown.stop="startResize('e', $event)" />
    </div>

    <!-- Inner window: visual styling, clipped content -->
    <div class="absolute inset-0 border border-gray-700 rounded shadow-2xl flex flex-col overflow-hidden">

      <!-- Title bar — primary move handle -->
      <div
        class="flex items-center justify-between px-3 bg-gray-800 border-b border-gray-700 shrink-0 cursor-move select-none"
        style="height: 36px; min-height: 36px;"
        @mousedown="startMove"
      >
        <span class="text-gray-200 font-mono text-xs uppercase tracking-wider truncate">
          {{ title }}
        </span>
        <div class="flex items-center gap-1 shrink-0">
          <button
            class="w-5 h-5 flex items-center justify-center text-gray-500 hover:text-gray-300 transition-colors rounded"
            :title="panel.minimized ? 'Restore' : 'Minimize'"
            @click.stop="toggleMinimize"
          >
            <span class="text-xs leading-none">{{ panel.minimized ? '▲' : '▼' }}</span>
          </button>
          <button
            v-if="closable"
            class="w-5 h-5 flex items-center justify-center text-gray-500 hover:text-gray-300 transition-colors rounded"
            title="Close"
            @click.stop="closePanel"
          >
            <span class="text-xs leading-none">✕</span>
          </button>
        </div>
      </div>

      <!-- Content area -->
      <div
        v-if="!panel.minimized"
        class="flex-1 min-h-0 bg-gray-900 overflow-hidden relative"
      >
        <!--
          Side move strips — 4px wide/tall strips on the left, right, and bottom
          interior edges. Allow moving the window by grabbing any side, not just
          the title bar. z-index:10 sits above slot content.
        -->
        <div class="absolute left-0   top-0 bottom-0 w-1 cursor-move z-10" @mousedown.stop="startMove" />
        <div class="absolute right-0  top-0 bottom-0 w-1 cursor-move z-10" @mousedown.stop="startMove" />
        <div class="absolute bottom-0 left-1 right-1  h-1 cursor-move z-10" @mousedown.stop="startMove" />

        <slot />
      </div>
    </div>
  </div>
</template>
