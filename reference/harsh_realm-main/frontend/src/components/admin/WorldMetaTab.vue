<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useAdminStore } from "../../stores/admin";

const store = useAdminStore();
const statusMsg = ref("");
const editValues = ref<Record<string, string>>({});

const knownKeyLabels: Record<string, string> = {
  world_name: "World Name",
  world_seed: "World Seed",
  created_at: "Created At",
  width: "Map Width",
  height: "Map Height",
  chaos_factor: "Chaos Factor",
  current_tick: "Current Tick",
  panel_layout: "Panel Layout (JSON)",
};

const metaEntries = computed(() => {
  return Object.entries(editValues.value).map(([key, value]) => ({
    key,
    label: knownKeyLabels[key] || key,
    value,
  }));
});

async function loadData() {
  await store.loadWorldMeta();
  editValues.value = { ...store.worldMeta };
}

async function handleSave(key: string) {
  await store.setWorldMeta(key, editValues.value[key]);
  statusMsg.value = `Saved "${key}".`;
  setTimeout(() => { statusMsg.value = ""; }, 3000);
}

onMounted(loadData);
</script>

<template>
  <div>
    <div class="flex items-center justify-between mb-3">
      <h2 class="text-sm font-bold text-gray-300 uppercase tracking-wide">World Meta</h2>
      <div class="flex items-center gap-2">
        <span v-if="statusMsg" class="text-xs text-green-400">{{ statusMsg }}</span>
        <button
          class="px-2 py-1 text-xs rounded border border-gray-600 text-gray-400 hover:text-gray-200"
          @click="loadData"
        >
          Reload
        </button>
      </div>
    </div>

    <table class="w-full text-sm">
      <thead>
        <tr class="text-left text-gray-500 border-b border-gray-800">
          <th class="px-2 py-1 w-48">Key</th>
          <th class="px-2 py-1">Value</th>
          <th class="px-2 py-1 w-20">Actions</th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="entry in metaEntries"
          :key="entry.key"
          class="border-b border-gray-800/50 hover:bg-gray-900/50"
        >
          <td class="px-2 py-1">
            <div class="text-gray-300 text-xs">{{ entry.label }}</div>
            <div v-if="entry.label !== entry.key" class="text-gray-600 text-xs font-mono">{{ entry.key }}</div>
          </td>
          <td class="px-2 py-1">
            <input
              v-model="editValues[entry.key]"
              class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs font-mono focus:outline-none focus:border-gray-500"
            />
          </td>
          <td class="px-2 py-1">
            <button
              class="px-2 py-0.5 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
              @click="handleSave(entry.key)"
            >
              Save
            </button>
          </td>
        </tr>
      </tbody>
    </table>

    <p v-if="metaEntries.length === 0 && !store.loading" class="text-gray-600 text-sm mt-4">
      No world meta entries found. Is a world loaded?
    </p>
  </div>
</template>
