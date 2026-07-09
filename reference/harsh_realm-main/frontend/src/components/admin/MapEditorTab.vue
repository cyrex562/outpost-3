<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useAdminStore } from "../../stores/admin";
import type { CellData, JsonObject } from "../../types/api";
import MapEditor from "./MapEditor.vue";

interface CellEditForm {
  q: number;
  r: number;
  terrain: string;
  explored: boolean;
  faction_id: string;
  features: string;
}

const store = useAdminStore();
const saving = ref(false);
const statusMsg = ref("");
const viewMode = ref<"map" | "list">("map");

// Selected cell for the map view edit form
const selectedQ = ref<number | null>(null);
const selectedR = ref<number | null>(null);
const editCell = ref<CellEditForm | null>(null);

// Bulk update fields
const bulkCellList = ref("");
const bulkTerrain = ref("");
const bulkFaction = ref("");
const bulkExplored = ref<string>("");

const terrainOptions = [
  "plains", "forest", "hills", "mountains", "swamp", "desert",
  "water", "jungle", "tundra", "wasteland", "coast", "ruins",
];

async function loadData() {
  await store.loadCells();
}

async function handleSelectCell(q: number, r: number) {
  selectedQ.value = q;
  selectedR.value = r;
  // Find cell in local store data
  const cell = store.cells.find((h) => h.q === q && h.r === r);
  if (cell) {
    editCell.value = {
      q: cell.q,
      r: cell.r,
      terrain: cell.terrain,
      explored: cell.explored,
      faction_id: cell.faction_id || "",
      features: cell.features.join(", "),
    };
  }
}

async function saveSelectedCell() {
  if (!editCell.value) return;
  saving.value = true;
  try {
    const features = editCell.value.features
      .split(",")
      .map((s: string) => s.trim())
      .filter(Boolean);
    await store.updateCell(editCell.value.q, editCell.value.r, {
      terrain: editCell.value.terrain,
      explored: editCell.value.explored,
      faction_id: editCell.value.faction_id || "",
      features,
    });
    statusMsg.value = `Cell (${editCell.value.q},${editCell.value.r}) saved.`;
    await store.loadCells();
    setTimeout(() => { statusMsg.value = ""; }, 3000);
  } finally {
    saving.value = false;
  }
}

async function saveCell(cell: CellData) {
  try {
    await store.updateCell(cell.q, cell.r, {
      terrain: cell.terrain,
      explored: cell.explored,
      faction_id: cell.faction_id || "",
      features: cell.features,
    });
    statusMsg.value = `Cell (${cell.q},${cell.r}) saved.`;
    setTimeout(() => { statusMsg.value = ""; }, 3000);
  } catch {
    statusMsg.value = "Save failed.";
  }
}

async function handleBulkUpdate() {
  const cells = bulkCellList.value
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => {
      const [q, r] = line.split(",").map(Number);
      return { q, r };
    })
    .filter((h) => !isNaN(h.q) && !isNaN(h.r));

  if (cells.length === 0) {
    statusMsg.value = "No valid cell coordinates provided.";
    return;
  }

  const data: JsonObject = {};
  if (bulkTerrain.value) data.terrain = bulkTerrain.value;
  if (bulkFaction.value) data.faction_id = bulkFaction.value;
  if (bulkExplored.value !== "") data.explored = bulkExplored.value === "true";

  await store.bulkUpdateCells(cells, data);
  statusMsg.value = `Bulk updated ${cells.length} cells.`;
  await store.loadCells();
  setTimeout(() => { statusMsg.value = ""; }, 3000);
}

function featuresDisplay(features: string[]): string {
  return features.join(", ");
}

function parseFeatures(value: string): string[] {
  return value
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
}

const cellsForEditor = computed(() =>
  store.cells.map((h) => ({
    q: h.q,
    r: h.r,
    terrain: h.terrain,
    features: h.features,
    explored: h.explored,
    faction_id: h.faction_id ?? undefined,
  }))
);

onMounted(loadData);
</script>

<template>
  <div>
    <div class="flex items-center justify-between mb-3">
      <h2 class="text-sm font-bold text-gray-300 uppercase tracking-wide">Map Editor</h2>
      <div class="flex items-center gap-2">
        <span v-if="statusMsg" class="text-xs text-green-400">{{ statusMsg }}</span>
        <button
          class="px-2 py-1 text-xs rounded border transition-colors"
          :class="viewMode === 'map'
            ? 'border-blue-600 text-blue-400'
            : 'border-gray-600 text-gray-400 hover:text-gray-200'"
          @click="viewMode = 'map'"
        >
          Map View
        </button>
        <button
          class="px-2 py-1 text-xs rounded border transition-colors"
          :class="viewMode === 'list'
            ? 'border-blue-600 text-blue-400'
            : 'border-gray-600 text-gray-400 hover:text-gray-200'"
          @click="viewMode = 'list'"
        >
          List View
        </button>
        <button
          class="px-2 py-1 text-xs rounded border border-gray-600 text-gray-400 hover:text-gray-200"
          @click="loadData"
        >
          Reload
        </button>
      </div>
    </div>

    <!-- MAP VIEW: split layout -->
    <div v-if="viewMode === 'map'" class="flex gap-4" style="height: 560px;">
      <!-- Left: SVG cell map -->
      <div class="flex-1 border border-gray-800 rounded overflow-hidden">
        <MapEditor
          :cells="cellsForEditor"
          :selected-q="selectedQ"
          :selected-r="selectedR"
          @cell-click="handleSelectCell"
        />
      </div>

      <!-- Right: edit sidebar -->
      <div class="w-72 shrink-0 overflow-y-auto">
        <div v-if="editCell" class="flex flex-col gap-3">
          <div class="text-sm font-mono text-gray-300 border-b border-gray-800 pb-2">
            Cell ({{ editCell.q }}, {{ editCell.r }})
          </div>

          <div>
            <label class="block text-xs text-gray-500 mb-0.5">Terrain</label>
            <select
              v-model="editCell.terrain"
              class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-gray-500"
            >
              <option v-for="t in terrainOptions" :key="t" :value="t">{{ t }}</option>
            </select>
          </div>

          <div>
            <label class="block text-xs text-gray-500 mb-0.5">Features (comma-separated)</label>
            <input
              v-model="editCell.features"
              class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-gray-500"
              placeholder="settlement, ruins, lair"
            />
          </div>

          <div>
            <label class="block text-xs text-gray-500 mb-0.5">Faction ID</label>
            <input
              v-model="editCell.faction_id"
              class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-gray-500"
              placeholder="none"
            />
          </div>

          <div class="flex items-center gap-2">
            <input v-model="editCell.explored" type="checkbox" class="accent-blue-500" id="cell-explored" />
            <label for="cell-explored" class="text-xs text-gray-400">Explored</label>
          </div>

          <button
            class="px-3 py-1 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
            :disabled="saving"
            @click="saveSelectedCell"
          >
            {{ saving ? "Saving..." : "Save" }}
          </button>
        </div>

        <div v-else class="flex items-center justify-center h-full text-gray-600 text-xs">
          Click a cell on the map to edit.
        </div>
      </div>
    </div>

    <!-- LIST VIEW: original table -->
    <div v-if="viewMode === 'list'">
      <div class="overflow-x-auto">
        <table class="w-full text-sm">
          <thead>
            <tr class="text-left text-gray-500 border-b border-gray-800">
              <th class="px-2 py-1">Q</th>
              <th class="px-2 py-1">R</th>
              <th class="px-2 py-1">Terrain</th>
              <th class="px-2 py-1">Explored</th>
              <th class="px-2 py-1">Faction</th>
              <th class="px-2 py-1">Features</th>
              <th class="px-2 py-1">Actions</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="cell in store.cells"
              :key="`${cell.q},${cell.r}`"
              class="border-b border-gray-800/50 hover:bg-gray-900/50"
            >
              <td class="px-2 py-1 font-mono text-gray-400">{{ cell.q }}</td>
              <td class="px-2 py-1 font-mono text-gray-400">{{ cell.r }}</td>
              <td class="px-2 py-1">
                <select
                  v-model="cell.terrain"
                  class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-1 py-0.5 text-xs focus:outline-none focus:border-gray-500"
                >
                  <option v-for="t in terrainOptions" :key="t" :value="t">{{ t }}</option>
                </select>
              </td>
              <td class="px-2 py-1 text-center">
                <input v-model="cell.explored" type="checkbox" class="accent-blue-500" />
              </td>
              <td class="px-2 py-1">
                <input
                  v-model="cell.faction_id"
                  class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-1 py-0.5 w-24 text-xs focus:outline-none focus:border-gray-500"
                  placeholder="none"
                />
              </td>
              <td class="px-2 py-1">
                <input
                  :value="featuresDisplay(cell.features)"
                  @input="cell.features = parseFeatures(($event.target as HTMLInputElement).value)"
                  class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-1 py-0.5 w-40 text-xs focus:outline-none focus:border-gray-500"
                  placeholder="comma-separated"
                />
              </td>
              <td class="px-2 py-1">
                <button
                  class="px-2 py-0.5 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
                  @click="saveCell(cell)"
                >
                  Save
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <p v-if="store.cells.length === 0 && !store.loading" class="text-gray-600 text-sm mt-4">
        No cells found. Is a world loaded?
      </p>

      <!-- Bulk Update -->
      <div class="mt-6 border-t border-gray-800 pt-4">
        <h3 class="text-sm font-bold text-gray-400 mb-2">Bulk Update</h3>
        <div class="grid grid-cols-2 gap-4">
          <div>
            <label class="block text-xs text-gray-500 mb-1">Cell coordinates (one per line: q,r)</label>
            <textarea
              v-model="bulkCellList"
              rows="4"
              class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs font-mono focus:outline-none focus:border-gray-500"
              placeholder="0,0&#10;1,0&#10;2,1"
            ></textarea>
          </div>
          <div class="flex flex-col gap-2">
            <div>
              <label class="block text-xs text-gray-500 mb-1">Set terrain</label>
              <select
                v-model="bulkTerrain"
                class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-1 py-0.5 text-xs focus:outline-none focus:border-gray-500"
              >
                <option value="">-- no change --</option>
                <option v-for="t in terrainOptions" :key="t" :value="t">{{ t }}</option>
              </select>
            </div>
            <div>
              <label class="block text-xs text-gray-500 mb-1">Set faction</label>
              <input
                v-model="bulkFaction"
                class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-1 py-0.5 w-full text-xs focus:outline-none focus:border-gray-500"
                placeholder="faction id (blank = no change)"
              />
            </div>
            <div>
              <label class="block text-xs text-gray-500 mb-1">Set explored</label>
              <select
                v-model="bulkExplored"
                class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-1 py-0.5 text-xs focus:outline-none focus:border-gray-500"
              >
                <option value="">-- no change --</option>
                <option value="true">Yes</option>
                <option value="false">No</option>
              </select>
            </div>
            <button
              class="px-3 py-1 text-xs rounded border border-blue-600 text-blue-400 hover:bg-blue-900/30 self-start mt-1"
              @click="handleBulkUpdate"
            >
              Apply Bulk Update
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
