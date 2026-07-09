<script setup lang="ts">
import { ref, computed, watch } from "vue";

export interface DungeonRoom {
  id: string;
  name: string;
  type: string;
  description?: string | undefined;
  encounter?: unknown;
  editor_pos?: { x: number; y: number } | undefined;
  [key: string]: unknown;
}

export interface DungeonConnection {
  from: string;
  to: string;
  direction?: string;
  locked?: boolean;
}

const props = defineProps<{
  rooms: DungeonRoom[];
  connections: DungeonConnection[];
}>();

const emit = defineEmits<{
  "update:rooms": [rooms: DungeonRoom[]];
  "update:connections": [connections: DungeonConnection[]];
  "select-room": [roomId: string | null];
}>();

const ROOM_W = 100;
const ROOM_H = 40;

const ROOM_COLORS: Record<string, string> = {
  entrance: "#2d6a2e",
  room: "#2a4a6a",
  corridor: "#555",
  trap: "#8a2a2a",
  stairs: "#8a7a2a",
  boss: "#6a2a6a",
};

// Internal mutable copy of rooms for drag operations
const localRooms = ref<DungeonRoom[]>([]);
watch(
  () => props.rooms,
  (newRooms) => {
    localRooms.value = newRooms.map((r) => ({
      ...r,
      editor_pos: r.editor_pos ?? { x: Math.random() * 400 + 50, y: Math.random() * 300 + 50 },
    }));
  },
  { immediate: true, deep: true }
);

const selectedRoomId = ref<string | null>(null);

// Connection mode
const connectMode = ref(false);
const connectSource = ref<string | null>(null);

// ViewBox state for pan/zoom
const viewBox = ref({ x: -20, y: -20, w: 700, h: 500 });

// Drag state
let isDragging = false;
let dragTarget: string | null = null;
let dragOffsetX = 0;
let dragOffsetY = 0;
let isPanning = false;
let panStartX = 0;
let panStartY = 0;
let panStartVbX = 0;
let panStartVbY = 0;

const viewBoxStr = computed(
  () => `${viewBox.value.x} ${viewBox.value.y} ${viewBox.value.w} ${viewBox.value.h}`
);

function roomColor(type: string): string {
  return ROOM_COLORS[type] ?? ROOM_COLORS.room;
}

function roomCenter(room: DungeonRoom): { x: number; y: number } {
  const pos = room.editor_pos ?? { x: 100, y: 100 };
  return { x: pos.x + ROOM_W / 2, y: pos.y + ROOM_H / 2 };
}

function getRoomById(id: string): DungeonRoom | undefined {
  return localRooms.value.find((r) => r.id === id);
}

// SVG coordinate transform for mouse events
function svgPoint(e: MouseEvent): { x: number; y: number } {
  const svg = (e.currentTarget as SVGElement).closest("svg");
  if (!svg) return { x: e.clientX, y: e.clientY };
  const rect = svg.getBoundingClientRect();
  const scaleX = viewBox.value.w / rect.width;
  const scaleY = viewBox.value.h / rect.height;
  return {
    x: viewBox.value.x + (e.clientX - rect.left) * scaleX,
    y: viewBox.value.y + (e.clientY - rect.top) * scaleY,
  };
}

function onRoomMouseDown(roomId: string, e: MouseEvent) {
  e.stopPropagation();

  if (connectMode.value) {
    if (!connectSource.value) {
      connectSource.value = roomId;
    } else if (connectSource.value !== roomId) {
      // Add connection
      const newConns = [...props.connections, { from: connectSource.value, to: roomId }];
      emit("update:connections", newConns);
      connectSource.value = null;
      connectMode.value = false;
    }
    return;
  }

  isDragging = true;
  dragTarget = roomId;
  const room = getRoomById(roomId);
  if (room?.editor_pos) {
    const pt = svgPoint(e);
    dragOffsetX = pt.x - room.editor_pos.x;
    dragOffsetY = pt.y - room.editor_pos.y;
  }
  selectedRoomId.value = roomId;
  emit("select-room", roomId);
}

function onSvgMouseDown(e: MouseEvent) {
  if (connectMode.value) {
    connectMode.value = false;
    connectSource.value = null;
    return;
  }
  isPanning = true;
  panStartX = e.clientX;
  panStartY = e.clientY;
  panStartVbX = viewBox.value.x;
  panStartVbY = viewBox.value.y;
}

function onSvgMouseMove(e: MouseEvent) {
  if (isDragging && dragTarget) {
    const pt = svgPoint(e);
    const room = getRoomById(dragTarget);
    if (room) {
      room.editor_pos = { x: pt.x - dragOffsetX, y: pt.y - dragOffsetY };
    }
    return;
  }
  if (isPanning) {
    const svg = (e.currentTarget as SVGElement);
    const rect = svg.getBoundingClientRect();
    const scaleX = viewBox.value.w / rect.width;
    const scaleY = viewBox.value.h / rect.height;
    viewBox.value = {
      ...viewBox.value,
      x: panStartVbX - (e.clientX - panStartX) * scaleX,
      y: panStartVbY - (e.clientY - panStartY) * scaleY,
    };
  }
}

function onSvgMouseUp() {
  if (isDragging && dragTarget) {
    // Emit updated rooms with new positions
    emit("update:rooms", localRooms.value.map((r) => ({ ...r })));
  }
  isDragging = false;
  dragTarget = null;
  isPanning = false;
}

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

function addRoom() {
  const newId = `room_${Date.now()}`;
  const newRoom: DungeonRoom = {
    id: newId,
    name: `Room ${localRooms.value.length + 1}`,
    type: "room",
    editor_pos: { x: 200, y: 200 },
  };
  const updated = [...localRooms.value, newRoom];
  localRooms.value = updated;
  emit("update:rooms", updated);
  selectedRoomId.value = newId;
  emit("select-room", newId);
}

function startConnect() {
  connectMode.value = true;
  connectSource.value = null;
}

function deleteSelected() {
  if (!selectedRoomId.value) return;
  const id = selectedRoomId.value;
  const updatedRooms = localRooms.value.filter((r) => r.id !== id);
  const updatedConns = props.connections.filter(
    (c) => c.from !== id && c.to !== id
  );
  localRooms.value = updatedRooms;
  selectedRoomId.value = null;
  emit("update:rooms", updatedRooms);
  emit("update:connections", updatedConns);
  emit("select-room", null);
}
</script>

<template>
  <div class="w-full h-full overflow-hidden bg-gray-950 relative select-none">
    <!-- Toolbar -->
    <div class="absolute top-2 left-2 z-10 flex gap-1">
      <button
        class="px-2 py-1 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30 bg-gray-950/90"
        @click="addRoom"
      >
        + Room
      </button>
      <button
        class="px-2 py-1 text-xs rounded border transition-colors bg-gray-950/90"
        :class="connectMode
          ? 'border-yellow-500 text-yellow-300'
          : 'border-blue-700 text-blue-400 hover:bg-blue-900/30'"
        @click="startConnect"
      >
        {{ connectMode ? "Click source, then target..." : "Connect" }}
      </button>
      <button
        v-if="selectedRoomId"
        class="px-2 py-1 text-xs rounded border border-red-700 text-red-400 hover:bg-red-900/30 bg-gray-950/90"
        @click="deleteSelected"
      >
        Delete
      </button>
    </div>

    <svg
      class="w-full h-full cursor-grab"
      :viewBox="viewBoxStr"
      xmlns="http://www.w3.org/2000/svg"
      @wheel.passive="false"
      @wheel="onWheel"
      @mousedown="onSvgMouseDown"
      @mousemove="onSvgMouseMove"
      @mouseup="onSvgMouseUp"
      @mouseleave="onSvgMouseUp"
    >
      <!-- Connections -->
      <line
        v-for="(conn, idx) in connections"
        :key="`conn-${idx}`"
        :x1="getRoomById(conn.from) ? roomCenter(getRoomById(conn.from)!).x : 0"
        :y1="getRoomById(conn.from) ? roomCenter(getRoomById(conn.from)!).y : 0"
        :x2="getRoomById(conn.to) ? roomCenter(getRoomById(conn.to)!).x : 0"
        :y2="getRoomById(conn.to) ? roomCenter(getRoomById(conn.to)!).y : 0"
        :stroke="conn.locked ? '#a33' : '#666'"
        stroke-width="2"
        :stroke-dasharray="conn.locked ? '5,3' : 'none'"
      />

      <!-- Rooms -->
      <g
        v-for="room in localRooms"
        :key="room.id"
        class="cursor-pointer"
        @mousedown="onRoomMouseDown(room.id, $event)"
      >
        <rect
          :x="(room.editor_pos?.x ?? 0)"
          :y="(room.editor_pos?.y ?? 0)"
          :width="ROOM_W"
          :height="ROOM_H"
          :fill="roomColor(room.type)"
          :stroke="selectedRoomId === room.id ? '#f0c040' : '#888'"
          :stroke-width="selectedRoomId === room.id ? 2.5 : 1"
          rx="4"
          ry="4"
        />
        <text
          :x="(room.editor_pos?.x ?? 0) + ROOM_W / 2"
          :y="(room.editor_pos?.y ?? 0) + ROOM_H / 2 + 1"
          text-anchor="middle"
          dominant-baseline="middle"
          fill="white"
          font-size="10"
          font-family="monospace"
          pointer-events="none"
        >{{ room.name }}</text>
        <text
          :x="(room.editor_pos?.x ?? 0) + ROOM_W / 2"
          :y="(room.editor_pos?.y ?? 0) - 4"
          text-anchor="middle"
          fill="#777"
          font-size="7"
          font-family="monospace"
          pointer-events="none"
        >{{ room.type }}</text>
      </g>
    </svg>

    <div
      class="absolute bottom-2 right-2 text-xs font-mono text-gray-500 bg-gray-950 bg-opacity-80 px-2 py-1 rounded pointer-events-none"
    >
      Drag rooms · Scroll to zoom · Drag canvas to pan
    </div>
  </div>
</template>
