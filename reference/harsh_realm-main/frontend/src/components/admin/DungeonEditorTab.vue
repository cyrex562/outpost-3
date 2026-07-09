<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useAdminStore } from "../../stores/admin";
import DungeonGraphEditor from "./DungeonGraphEditor.vue";
import ImportExportBar from "./ImportExportBar.vue";
import type { DungeonRoom, DungeonConnection } from "./DungeonGraphEditor.vue";

interface DungeonSource {
  id: string;
  name: string;
  location_q: number | null;
  location_r: number | null;
  rooms: Partial<DungeonRoom>[];
  connections: DungeonConnection[];
}

interface DungeonEditData extends DungeonSource {
  roomsJson: string;
  connectionsJson: string;
}

const store = useAdminStore();
const selectedId = ref<string | null>(null);
const editData = ref<DungeonEditData | null>(null);
const statusMsg = ref("");
const viewMode = ref<"graph" | "json">("graph");

// Graph editor state
const rooms = ref<DungeonRoom[]>([]);
const connections = ref<DungeonConnection[]>([]);
const selectedRoomId = ref<string | null>(null);
const selectedRoom = ref<DungeonRoom | null>(null);

// Room type options
const roomTypes = ["entrance", "room", "corridor", "trap", "stairs", "boss"];

async function loadData() {
  await store.loadDungeons();
}

async function selectDungeon(id: string) {
  selectedId.value = id;
  const dungeon: DungeonSource = await store.getDungeon(id);
  editData.value = {
    ...dungeon,
    roomsJson: JSON.stringify(dungeon.rooms || [], null, 2),
    connectionsJson: JSON.stringify(dungeon.connections || [], null, 2),
  };
  rooms.value = (dungeon.rooms || []).map((r: Partial<DungeonRoom>, i: number) => ({
    ...r,
    id: r.id || `room_${i}`,
    name: r.name || `Room ${i}`,
    type: r.type || "room",
    editor_pos: r.editor_pos ?? { x: 80 + (i % 5) * 130, y: 60 + Math.floor(i / 5) * 80 },
  }));
  connections.value = dungeon.connections || [];
  selectedRoomId.value = null;
  selectedRoom.value = null;
}

function handleRoomsUpdate(updated: DungeonRoom[]) {
  rooms.value = updated;
  if (editData.value) {
    editData.value.roomsJson = JSON.stringify(updated, null, 2);
  }
}

function handleConnectionsUpdate(updated: DungeonConnection[]) {
  connections.value = updated;
  if (editData.value) {
    editData.value.connectionsJson = JSON.stringify(updated, null, 2);
  }
}

function handleSelectRoom(roomId: string | null) {
  selectedRoomId.value = roomId;
  if (roomId) {
    const r = rooms.value.find((rm) => rm.id === roomId);
    selectedRoom.value = r ? { ...r } : null;
  } else {
    selectedRoom.value = null;
  }
}

function saveRoomDetail() {
  if (!selectedRoom.value) return;
  const idx = rooms.value.findIndex((r) => r.id === selectedRoom.value!.id);
  if (idx >= 0) {
    rooms.value[idx] = {
      ...rooms.value[idx],
      name: selectedRoom.value.name,
      type: selectedRoom.value.type,
      description: selectedRoom.value.description,
    };
    if (editData.value) {
      editData.value.roomsJson = JSON.stringify(rooms.value, null, 2);
    }
  }
}

async function handleSave() {
  if (!editData.value) return;
  let saveRooms, saveConnections;

  if (viewMode.value === "graph") {
    saveRooms = rooms.value;
    saveConnections = connections.value;
  } else {
    try {
      saveRooms = JSON.parse(editData.value.roomsJson);
    } catch {
      statusMsg.value = "Invalid rooms JSON.";
      return;
    }
    try {
      saveConnections = JSON.parse(editData.value.connectionsJson);
    } catch {
      statusMsg.value = "Invalid connections JSON.";
      return;
    }
  }

  await store.updateDungeon(editData.value.id, {
    name: editData.value.name,
    location_q: editData.value.location_q,
    location_r: editData.value.location_r,
    rooms: saveRooms,
    connections: saveConnections,
  });
  statusMsg.value = "Dungeon saved.";
  await store.loadDungeons();
  setTimeout(() => { statusMsg.value = ""; }, 3000);
}

async function handleCreate() {
  await store.createDungeon({ name: "New Dungeon" });
  await store.loadDungeons();
  statusMsg.value = "Dungeon created.";
  setTimeout(() => { statusMsg.value = ""; }, 3000);
}

async function handleDelete() {
  if (!editData.value) return;
  if (!window.confirm("Delete this dungeon?")) return;
  await store.deleteDungeon(editData.value.id);
  selectedId.value = null;
  editData.value = null;
  await store.loadDungeons();
  statusMsg.value = "Dungeon deleted.";
  setTimeout(() => { statusMsg.value = ""; }, 3000);
}

onMounted(loadData);
</script>

<template>
  <div>
    <ImportExportBar table="dungeons" @imported="loadData()" />
    <div class="flex gap-4 h-full">
    <!-- Left: dungeon list -->
    <div class="w-56 shrink-0 flex flex-col">
      <div class="flex items-center justify-between mb-2">
        <h2 class="text-sm font-bold text-gray-300 uppercase tracking-wide">Dungeons</h2>
        <span v-if="statusMsg" class="text-xs text-green-400">{{ statusMsg }}</span>
      </div>
      <button
        class="px-2 py-1 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30 mb-2"
        @click="handleCreate"
      >
        + New Dungeon
      </button>
      <div class="flex-1 overflow-y-auto">
        <div
          v-for="d in store.dungeons"
          :key="d.id"
          class="px-2 py-1 text-xs cursor-pointer rounded mb-0.5"
          :class="selectedId === d.id
            ? 'bg-blue-900/30 text-blue-300 border border-blue-700'
            : 'text-gray-400 hover:bg-gray-900 hover:text-gray-200 border border-transparent'"
          @click="selectDungeon(d.id)"
        >
          <div class="truncate">{{ d.name }}</div>
          <div class="text-gray-600">Location ({{ d.location_q }},{{ d.location_r }}) | {{ d.room_count }} rooms</div>
        </div>
        <p v-if="store.dungeons.length === 0" class="text-gray-600 text-xs mt-2">
          No dungeons found.
        </p>
      </div>
    </div>

    <!-- Right: edit area -->
    <div class="flex-1 flex flex-col overflow-hidden" v-if="editData">
      <!-- Dungeon metadata -->
      <div class="grid grid-cols-4 gap-3 mb-3 shrink-0">
        <div>
          <label class="block text-xs text-gray-500 mb-0.5">Name</label>
          <input
            v-model="editData.name"
            class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-gray-500"
          />
        </div>
        <div>
          <label class="block text-xs text-gray-500 mb-0.5">Location Q</label>
          <input v-model.number="editData.location_q" type="number"
            class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-gray-500" />
        </div>
        <div>
          <label class="block text-xs text-gray-500 mb-0.5">Location R</label>
          <input v-model.number="editData.location_r" type="number"
            class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-gray-500" />
        </div>
        <div class="flex items-end gap-1">
          <button
            class="px-2 py-1 text-xs rounded border transition-colors"
            :class="viewMode === 'graph' ? 'border-blue-600 text-blue-400' : 'border-gray-600 text-gray-400'"
            @click="viewMode = 'graph'"
          >Graph</button>
          <button
            class="px-2 py-1 text-xs rounded border transition-colors"
            :class="viewMode === 'json' ? 'border-blue-600 text-blue-400' : 'border-gray-600 text-gray-400'"
            @click="viewMode = 'json'"
          >JSON</button>
        </div>
      </div>

      <!-- Graph view -->
      <div v-if="viewMode === 'graph'" class="flex-1 flex gap-3 min-h-0">
        <div class="flex-1 border border-gray-800 rounded overflow-hidden">
          <DungeonGraphEditor
            :rooms="rooms"
            :connections="connections"
            @update:rooms="handleRoomsUpdate"
            @update:connections="handleConnectionsUpdate"
            @select-room="handleSelectRoom"
          />
        </div>
        <!-- Room detail sidebar -->
        <div class="w-60 shrink-0 overflow-y-auto" v-if="selectedRoom">
          <div class="text-xs text-gray-400 mb-2 border-b border-gray-800 pb-1">
            Room: {{ selectedRoom.id }}
          </div>
          <div class="flex flex-col gap-2">
            <div>
              <label class="block text-xs text-gray-500 mb-0.5">Name</label>
              <input
                v-model="selectedRoom.name"
                class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-gray-500"
              />
            </div>
            <div>
              <label class="block text-xs text-gray-500 mb-0.5">Type</label>
              <select
                v-model="selectedRoom.type"
                class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-gray-500"
              >
                <option v-for="t in roomTypes" :key="t" :value="t">{{ t }}</option>
              </select>
            </div>
            <div>
              <label class="block text-xs text-gray-500 mb-0.5">Description</label>
              <textarea
                v-model="selectedRoom.description"
                rows="3"
                class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs font-mono focus:outline-none focus:border-gray-500"
              ></textarea>
            </div>
            <button
              class="px-2 py-1 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30 self-start"
              @click="saveRoomDetail"
            >Apply</button>
          </div>
        </div>
      </div>

      <!-- JSON view -->
      <div v-if="viewMode === 'json'" class="flex-1 overflow-y-auto">
        <div class="mb-3">
          <label class="block text-xs text-gray-500 mb-1">Rooms (JSON)</label>
          <textarea
            v-model="editData.roomsJson"
            rows="12"
            class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs font-mono focus:outline-none focus:border-gray-500"
          ></textarea>
        </div>
        <div class="mb-3">
          <label class="block text-xs text-gray-500 mb-1">Connections (JSON)</label>
          <textarea
            v-model="editData.connectionsJson"
            rows="6"
            class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs font-mono focus:outline-none focus:border-gray-500"
          ></textarea>
        </div>
      </div>

      <!-- Actions bar -->
      <div class="flex gap-2 mt-2 border-t border-gray-800 pt-3 shrink-0">
        <button
          class="px-3 py-1 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
          @click="handleSave"
        >Save</button>
        <button
          class="px-3 py-1 text-xs rounded border border-red-700 text-red-400 hover:bg-red-900/30"
          @click="handleDelete"
        >Delete</button>
      </div>
    </div>

    <div v-else class="flex-1 flex items-center justify-center text-gray-600 text-sm">
      Select a dungeon from the list to edit.
    </div>
  </div>
  </div>
</template>
