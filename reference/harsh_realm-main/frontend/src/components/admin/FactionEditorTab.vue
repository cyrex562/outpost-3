<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useAdminStore } from "../../stores/admin";
import type { Faction } from "../../types/api";
import ImportExportBar from "./ImportExportBar.vue";

const store = useAdminStore();
const selectedId = ref<string | null>(null);
const editData = ref<Faction | null>(null);
const statusMsg = ref("");

// New asset form
const newAssetType = ref("Warriors");
const newAssetCategory = ref("force");

async function loadData() {
  await store.loadFactions();
}

async function selectFaction(id: string) {
  selectedId.value = id;
  const faction = await store.getFaction(id);
  editData.value = JSON.parse(JSON.stringify(faction));
}

function parseList(value: string): string[] {
  return value
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
}

async function handleSave() {
  if (!editData.value) return;
  const { id, assets, relations, data, ...rest } = editData.value;
  await store.updateFaction(id, rest);
  statusMsg.value = "Faction saved.";
  await store.loadFactions();
  setTimeout(() => { statusMsg.value = ""; }, 3000);
}

async function handleCreate() {
  await store.createFaction({ name: "New Faction" });
  await store.loadFactions();
  statusMsg.value = "Faction created.";
  setTimeout(() => { statusMsg.value = ""; }, 3000);
}

async function handleDelete() {
  if (!editData.value) return;
  if (!window.confirm("Delete this faction and all its assets?")) return;
  await store.deleteFaction(editData.value.id);
  selectedId.value = null;
  editData.value = null;
  await store.loadFactions();
  statusMsg.value = "Faction deleted.";
  setTimeout(() => { statusMsg.value = ""; }, 3000);
}

async function handleAddAsset() {
  if (!editData.value) return;
  const res = await fetch(
    `/api/admin/factions/${editData.value.id}/assets${store.worldParam()}`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ asset_type: newAssetType.value, category: newAssetCategory.value }),
    }
  );
  if (res.ok) {
    await selectFaction(editData.value.id);
    statusMsg.value = "Asset added.";
    setTimeout(() => { statusMsg.value = ""; }, 3000);
  }
}

async function handleDeleteAsset(assetId: string) {
  if (!editData.value) return;
  await fetch(
    `/api/admin/factions/${editData.value.id}/assets/${assetId}${store.worldParam()}`,
    { method: "DELETE" }
  );
  await selectFaction(editData.value.id);
}

onMounted(loadData);
</script>

<template>
  <div>
    <ImportExportBar table="factions" @imported="loadData()" />
    <div class="flex gap-4 h-full">
    <!-- Left: faction list -->
    <div class="w-56 shrink-0 flex flex-col">
      <div class="flex items-center justify-between mb-2">
        <h2 class="text-sm font-bold text-gray-300 uppercase tracking-wide">Factions</h2>
        <span v-if="statusMsg" class="text-xs text-green-400">{{ statusMsg }}</span>
      </div>
      <button
        class="px-2 py-1 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30 mb-2"
        @click="handleCreate"
      >
        + New Faction
      </button>
      <div class="flex-1 overflow-y-auto">
        <div
          v-for="f in store.factions"
          :key="f.id"
          class="px-2 py-1 text-xs cursor-pointer rounded mb-0.5"
          :class="selectedId === f.id
            ? 'bg-blue-900/30 text-blue-300 border border-blue-700'
            : 'text-gray-400 hover:bg-gray-900 hover:text-gray-200 border border-transparent'"
          @click="selectFaction(f.id)"
        >
          <div class="truncate">{{ f.name }}</div>
          <div class="text-gray-600">F{{ f.force }}/C{{ f.cunning }}/W{{ f.wealth }} HP:{{ f.hp }}/{{ f.max_hp }}</div>
        </div>
        <p v-if="store.factions.length === 0" class="text-gray-600 text-xs mt-2">
          No factions found.
        </p>
      </div>
    </div>

    <!-- Right: edit form -->
    <div class="flex-1 overflow-y-auto" v-if="editData">
      <div class="grid grid-cols-2 gap-3 mb-3">
        <div>
          <label class="block text-xs text-gray-500 mb-0.5">Name</label>
          <input
            v-model="editData.name"
            class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-gray-500"
          />
        </div>
        <div class="grid grid-cols-2 gap-2">
          <div>
            <label class="block text-xs text-gray-500 mb-0.5">HP</label>
            <input v-model.number="editData.hp" type="number"
              class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-gray-500" />
          </div>
          <div>
            <label class="block text-xs text-gray-500 mb-0.5">Max HP</label>
            <input v-model.number="editData.max_hp" type="number"
              class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-gray-500" />
          </div>
        </div>
      </div>

      <div class="grid grid-cols-3 gap-3 mb-3">
        <div>
          <label class="block text-xs text-gray-500 mb-0.5">Force</label>
          <input v-model.number="editData.force" type="number"
            class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-gray-500" />
        </div>
        <div>
          <label class="block text-xs text-gray-500 mb-0.5">Cunning</label>
          <input v-model.number="editData.cunning" type="number"
            class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-gray-500" />
        </div>
        <div>
          <label class="block text-xs text-gray-500 mb-0.5">Wealth</label>
          <input v-model.number="editData.wealth" type="number"
            class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-gray-500" />
        </div>
      </div>

      <div class="grid grid-cols-3 gap-3 mb-3">
        <div>
          <label class="block text-xs text-gray-500 mb-0.5">XP</label>
          <input v-model.number="editData.xp" type="number"
            class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-gray-500" />
        </div>
        <div>
          <label class="block text-xs text-gray-500 mb-0.5">Home Q</label>
          <input v-model.number="editData.home_q" type="number"
            class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-gray-500" />
        </div>
        <div>
          <label class="block text-xs text-gray-500 mb-0.5">Home R</label>
          <input v-model.number="editData.home_r" type="number"
            class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-gray-500" />
        </div>
      </div>

      <div class="grid grid-cols-2 gap-3 mb-3">
        <div>
          <label class="block text-xs text-gray-500 mb-0.5">Goals (comma-separated)</label>
          <input
            :value="editData.goals.join(', ')"
            @input="editData.goals = parseList(($event.target as HTMLInputElement).value)"
            class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-gray-500"
          />
        </div>
        <div>
          <label class="block text-xs text-gray-500 mb-0.5">Tags (comma-separated)</label>
          <input
            :value="editData.tags.join(', ')"
            @input="editData.tags = parseList(($event.target as HTMLInputElement).value)"
            class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-gray-500"
          />
        </div>
      </div>

      <!-- Assets -->
      <div class="mb-3 border-t border-gray-800 pt-3">
        <div class="flex items-center justify-between mb-2">
          <label class="text-xs text-gray-500 font-bold uppercase">Assets</label>
        </div>
        <table class="w-full text-xs mb-2" v-if="editData.assets && editData.assets.length">
          <thead>
            <tr class="text-left text-gray-600 border-b border-gray-800">
              <th class="px-1 py-0.5">Type</th>
              <th class="px-1 py-0.5">Category</th>
              <th class="px-1 py-0.5">HP</th>
              <th class="px-1 py-0.5">Location</th>
              <th class="px-1 py-0.5"></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="a in editData.assets" :key="a.id" class="border-b border-gray-800/50">
              <td class="px-1 py-0.5 text-gray-300">{{ a.asset_type }}</td>
              <td class="px-1 py-0.5 text-gray-400">{{ a.category }}</td>
              <td class="px-1 py-0.5 text-gray-400">{{ a.hp }}/{{ a.max_hp }}</td>
              <td class="px-1 py-0.5 text-gray-400">{{ a.location_q }},{{ a.location_r }}</td>
              <td class="px-1 py-0.5">
                <button
                  class="text-red-500 hover:text-red-300"
                  @click="handleDeleteAsset(a.id)"
                >x</button>
              </td>
            </tr>
          </tbody>
        </table>
        <div class="flex gap-2 items-center">
          <input v-model="newAssetType" placeholder="Type"
            class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-1 py-0.5 text-xs w-24 focus:outline-none focus:border-gray-500" />
          <select v-model="newAssetCategory"
            class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-1 py-0.5 text-xs focus:outline-none focus:border-gray-500">
            <option value="force">Force</option>
            <option value="cunning">Cunning</option>
            <option value="wealth">Wealth</option>
          </select>
          <button
            class="px-2 py-0.5 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
            @click="handleAddAsset"
          >
            Add Asset
          </button>
        </div>
      </div>

      <!-- Relations -->
      <div class="mb-3 border-t border-gray-800 pt-3" v-if="editData.relations && editData.relations.length">
        <label class="block text-xs text-gray-500 font-bold uppercase mb-1">Relations</label>
        <div v-for="rel in editData.relations" :key="`${rel.faction_a}-${rel.faction_b}`" class="text-xs text-gray-400 mb-0.5">
          {{ rel.faction_a }} <span class="text-gray-600">&lt;-&gt;</span> {{ rel.faction_b }}: <span class="text-gray-300">{{ rel.disposition }}</span>
        </div>
      </div>

      <!-- Actions -->
      <div class="flex gap-2 mt-4 border-t border-gray-800 pt-3">
        <button
          class="px-3 py-1 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
          @click="handleSave"
        >
          Save
        </button>
        <button
          class="px-3 py-1 text-xs rounded border border-red-700 text-red-400 hover:bg-red-900/30"
          @click="handleDelete"
        >
          Delete Faction
        </button>
      </div>
    </div>

    <div v-else class="flex-1 flex items-center justify-center text-gray-600 text-sm">
      Select a faction from the list to edit.
    </div>
  </div>
  </div>
</template>
