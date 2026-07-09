<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useAdminStore } from "../../stores/admin";

const store = useAdminStore();
const statusMsg = ref("");
const cloneName = ref("");
const cloneSource = ref("");
const importingAll = ref(false);
const fileInputAll = ref<HTMLInputElement | null>(null);

async function loadData() {
  await store.loadWorldFiles();
}

async function handleClone(name: string) {
  cloneSource.value = name;
  cloneName.value = "";
}

async function doClone() {
  if (!cloneName.value.trim()) return;
  await store.cloneWorld(cloneSource.value, cloneName.value.trim());
  cloneSource.value = "";
  cloneName.value = "";
  await store.loadWorldFiles();
  statusMsg.value = "World cloned.";
  setTimeout(() => { statusMsg.value = ""; }, 3000);
}

async function handleExport(name: string) {
  await store.exportWorld(name);
}

async function handleDelete(name: string) {
  if (!window.confirm(`Delete world "${name}"? This cannot be undone.`)) return;
  await store.deleteWorld(name);
  await store.loadWorldFiles();
  statusMsg.value = "World deleted.";
  setTimeout(() => { statusMsg.value = ""; }, 3000);
}

async function handleExportAll() {
  try {
    const wp = store.worldParam();
    const res = await fetch(`/api/admin/export-all${wp || ""}`);
    if (!res.ok) { statusMsg.value = "Export failed."; return; }
    const data = await res.json();
    const blob = new Blob([JSON.stringify(data, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "world_export_all.json";
    a.click();
    URL.revokeObjectURL(url);
    const total = Object.values(data).reduce((sum: number, arr: unknown) => sum + (Array.isArray(arr) ? arr.length : 0), 0);
    statusMsg.value = `Exported ${total} total rows across ${Object.keys(data).length} tables.`;
  } catch {
    statusMsg.value = "Export failed.";
  }
  setTimeout(() => { statusMsg.value = ""; }, 5000);
}

function startImportAll() {
  fileInputAll.value?.click();
}

async function handleImportAllFile(event: Event) {
  const input = event.target as HTMLInputElement;
  if (!input.files?.length) return;
  const file = input.files[0];
  input.value = "";

  if (!window.confirm("This will replace ALL data in the selected tables. Continue?")) return;

  importingAll.value = true;
  try {
    const text = await file.text();
    const data = JSON.parse(text);
    const wp = store.worldParam();
    const res = await fetch(`/api/admin/import-all${wp || ""}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ tables: data }),
    });
    const result = await res.json();
    const totalImported = Object.values(result).reduce((sum: number, r: unknown) => {
      const imported =
        r && typeof r === "object" && "imported" in r
          ? (r as Record<string, unknown>).imported
          : 0;
      return sum + (typeof imported === "number" ? imported : 0);
    }, 0);
    statusMsg.value = `Imported ${totalImported} rows across ${Object.keys(result).length} tables.`;
  } catch (e) {
    statusMsg.value = `Import failed: ${e}`;
  } finally {
    importingAll.value = false;
  }
  setTimeout(() => { statusMsg.value = ""; }, 5000);
}

function formatSize(bytes: number | undefined): string {
  if (!bytes) return "?";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

onMounted(loadData);
</script>

<template>
  <div>
    <div class="flex items-center justify-between mb-3">
      <h2 class="text-sm font-bold text-gray-300 uppercase tracking-wide">World Files</h2>
      <div class="flex items-center gap-2">
        <span v-if="statusMsg" class="text-xs text-green-400">{{ statusMsg }}</span>
        <input ref="fileInputAll" type="file" accept=".json" class="hidden" @change="handleImportAllFile" />
        <button
          class="px-2 py-1 text-xs rounded border border-blue-700 text-blue-400 hover:bg-blue-900/30"
          @click="handleExportAll"
        >
          Export All Tables
        </button>
        <button
          class="px-2 py-1 text-xs rounded border border-blue-700 text-blue-400 hover:bg-blue-900/30"
          :disabled="importingAll"
          @click="startImportAll"
        >
          {{ importingAll ? "Importing..." : "Import All Tables" }}
        </button>
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
          <th class="px-2 py-1"></th>
          <th class="px-2 py-1">Name</th>
          <th class="px-2 py-1">File</th>
          <th class="px-2 py-1">Size</th>
          <th class="px-2 py-1">Actions</th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="w in store.worlds"
          :key="w.file || w.name"
          class="border-b border-gray-800/50 hover:bg-gray-900/50"
        >
          <td class="px-2 py-1 text-center">
            <span
              v-if="store.activeWorldPath === w.file"
              class="text-green-400 text-xs"
              title="Currently loaded"
            >&#9679;</span>
          </td>
          <td class="px-2 py-1 font-mono text-gray-300">{{ w.name }}</td>
          <td class="px-2 py-1 text-gray-500 text-xs">{{ w.file }}</td>
          <td class="px-2 py-1 text-gray-500 text-xs">{{ formatSize(w.size) }}</td>
          <td class="px-2 py-1 whitespace-nowrap">
            <button
              class="px-2 py-0.5 text-xs rounded border border-blue-700 text-blue-400 hover:bg-blue-900/30 mr-1"
              @click="handleClone(w.file || w.name)"
            >
              Clone
            </button>
            <button
              class="px-2 py-0.5 text-xs rounded border border-gray-600 text-gray-400 hover:bg-gray-800 mr-1"
              @click="handleExport(w.file || w.name)"
            >
              Export
            </button>
            <button
              class="px-2 py-0.5 text-xs rounded border border-red-700 text-red-400 hover:bg-red-900/30"
              @click="handleDelete(w.file || w.name)"
            >
              Delete
            </button>
          </td>
        </tr>
      </tbody>
    </table>

    <p v-if="store.worlds.length === 0 && !store.loading" class="text-gray-600 text-sm mt-4">
      No world files found.
    </p>

    <!-- Clone dialog -->
    <div v-if="cloneSource" class="mt-4 p-3 bg-gray-900 border border-gray-700 rounded inline-flex items-center gap-2">
      <span class="text-xs text-gray-400">Clone "{{ cloneSource }}" as:</span>
      <input
        v-model="cloneName"
        class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-gray-500"
        placeholder="new_world_name"
        @keyup.enter="doClone"
      />
      <button
        class="px-2 py-0.5 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
        @click="doClone"
      >
        Clone
      </button>
      <button
        class="px-2 py-0.5 text-xs rounded border border-gray-600 text-gray-400"
        @click="cloneSource = ''"
      >
        Cancel
      </button>
    </div>
  </div>
</template>
