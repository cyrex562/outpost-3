<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { useAdminStore } from "../../stores/admin";
import type { RandomTable } from "../../types/api";
import SchemaFormEditor from "./schema-editor/SchemaFormEditor.vue";
import ImportExportBar from "./ImportExportBar.vue";

interface TableInfo {
  path: string;
  size: number;
  entry_count: number;
  has_todo: boolean;
}

const store = useAdminStore();
const statusMsg = ref("");
const zipInput = ref<HTMLInputElement | null>(null);
const filterTodo = ref(false);

// --- YAML mode state ---
const allFiles = ref<TableInfo[]>([]);
const selectedFile = ref<string | null>(null);

// --- World mode state ---
const selectedTableId = ref<string | null>(null);
const editJson = ref("");
const editValid = ref(true);
const editMeta = ref({ category: "", name: "", tags: "" });

const isWorldMode = computed(() => !!store.activeWorldPath);

// --- Data loading ---

async function loadData() {
  if (isWorldMode.value) {
    await store.loadRandomTables();
  } else {
    try {
      allFiles.value = await store.loadTableStatus();
    } catch {
      allFiles.value = [];
    }
  }
}

watch(() => store.activeWorldPath, () => {
  selectedFile.value = null;
  selectedTableId.value = null;
  loadData();
});

onMounted(loadData);

// --- YAML mode ---

const filteredFiles = computed(() => {
  if (!filterTodo.value) return allFiles.value;
  return allFiles.value.filter((f) => f.has_todo);
});

const fileTree = computed(() => {
  const tree: Record<string, TableInfo[]> = {};
  for (const f of filteredFiles.value) {
    const parts = f.path.split("/");
    const category = parts.length > 2 ? parts[1] : "root";
    if (!tree[category]) tree[category] = [];
    tree[category].push(f);
  }
  return tree;
});

const todoCount = computed(() => allFiles.value.filter((f) => f.has_todo).length);
const totalEntries = computed(() =>
  allFiles.value.reduce((sum, f) => sum + (f.entry_count > 0 ? f.entry_count : 0), 0)
);

function selectFile(path: string) {
  selectedFile.value = path;
  statusMsg.value = "";
}

function onStatus(msg: string) {
  statusMsg.value = msg;
  if (msg) setTimeout(() => { statusMsg.value = ""; }, 3000);
}

async function handleDeleteFile(path: string) {
  if (!confirm(`Delete ${path}?`)) return;
  const ok = await store.deleteYamlFile(path);
  if (ok) {
    if (selectedFile.value === path) selectedFile.value = null;
    statusMsg.value = `Deleted ${path}.`;
    await loadData();
  } else {
    statusMsg.value = "Delete failed.";
  }
  setTimeout(() => { statusMsg.value = ""; }, 3000);
}

async function handleDownloadZip() {
  await store.downloadTablesZip();
}

function startZipUpload() {
  zipInput.value?.click();
}

async function handleZipSelected(event: Event) {
  const input = event.target as HTMLInputElement;
  if (!input.files?.length) return;
  const result = await store.uploadTablesZip(input.files[0]);
  input.value = "";
  if (result.errors.length) {
    statusMsg.value = `Imported ${result.written.length} files, ${result.errors.length} errors.`;
  } else {
    statusMsg.value = `Imported ${result.written.length} files.`;
  }
  await loadData();
  setTimeout(() => { statusMsg.value = ""; }, 5000);
}

function fileName(path: string): string {
  return path.split("/").pop() || path;
}

// --- World mode ---

const worldTableTree = computed(() => {
  const tree: Record<string, RandomTable[]> = {};
  for (const t of store.randomTables) {
    const cat = t.category || "uncategorized";
    if (!tree[cat]) tree[cat] = [];
    tree[cat].push(t);
  }
  return tree;
});

function selectTable(id: string) {
  selectedTableId.value = id;
  const table = store.randomTables.find((t: RandomTable) => t.id === id);
  if (table) {
    editMeta.value = {
      category: table.category,
      name: table.name,
      tags: (table.tags || []).join(", "),
    };
    editJson.value = JSON.stringify(table.entries, null, 2);
    editValid.value = true;
  }
  statusMsg.value = "";
}

function onEditJsonInput(event: Event) {
  const val = (event.target as HTMLTextAreaElement).value;
  editJson.value = val;
  try {
    JSON.parse(val);
    editValid.value = true;
  } catch {
    editValid.value = false;
  }
}

async function handleSaveTable() {
  if (!selectedTableId.value || !editValid.value) return;
  const entries = JSON.parse(editJson.value);
  const tags = editMeta.value.tags.split(",").map((s: string) => s.trim()).filter(Boolean);
  await store.saveRandomTable({
    id: selectedTableId.value,
    category: editMeta.value.category,
    name: editMeta.value.name,
    entries,
    tags,
  });
  statusMsg.value = "Saved.";
  await store.loadRandomTables();
  setTimeout(() => { statusMsg.value = ""; }, 3000);
}

async function handleResetTable(id: string) {
  if (!confirm(`Reset "${id}" to YAML default?`)) return;
  await store.resetRandomTable(id);
  statusMsg.value = `Reset ${id}.`;
  await store.loadRandomTables();
  if (selectedTableId.value === id) selectTable(id);
  setTimeout(() => { statusMsg.value = ""; }, 3000);
}

async function handleDeleteTable(id: string) {
  if (!confirm(`Delete "${id}"?`)) return;
  await store.deleteRandomTable(id);
  if (selectedTableId.value === id) selectedTableId.value = null;
  statusMsg.value = `Deleted ${id}.`;
  await store.loadRandomTables();
  setTimeout(() => { statusMsg.value = ""; }, 3000);
}

async function handleResetAll() {
  if (!confirm("Reset ALL random tables to YAML defaults? Custom tables will be removed.")) return;
  await store.resetAllRandomTables();
  selectedTableId.value = null;
  statusMsg.value = "All tables reset to defaults.";
  await store.loadRandomTables();
  setTimeout(() => { statusMsg.value = ""; }, 3000);
}
</script>

<template>
  <div>
    <!-- ============ WORLD MODE (SQLite per-world) ============ -->
    <template v-if="isWorldMode">
      <div class="flex items-center gap-3 mb-3 flex-wrap">
        <h2 class="text-sm font-bold text-gray-300 uppercase tracking-wide">Random Tables (World)</h2>
        <span class="text-xs text-gray-500">{{ store.randomTables.length }} tables</span>
        <span v-if="statusMsg" class="text-xs text-green-400">{{ statusMsg }}</span>
        <div class="ml-auto flex items-center gap-2">
          <button
            class="px-2 py-1 text-xs rounded border border-yellow-700 text-yellow-400 hover:bg-yellow-900/30"
            @click="handleResetAll"
          >Reset All to Defaults</button>
          <ImportExportBar table="random_tables" />
          <button
            class="px-2 py-1 text-xs rounded border border-gray-600 text-gray-400 hover:text-gray-200"
            @click="loadData"
          >Reload</button>
        </div>
      </div>

      <div class="flex gap-4">
        <!-- Left: table list -->
        <div class="w-72 shrink-0 max-h-[70vh] overflow-y-auto">
          <div v-for="(tables, category) in worldTableTree" :key="category" class="mb-2">
            <div class="text-xs font-bold text-gray-500 uppercase tracking-wide px-2 py-0.5">{{ category }}</div>
            <div
              v-for="t in tables"
              :key="t.id"
              class="group flex items-center justify-between px-2 py-0.5 text-xs cursor-pointer rounded"
              :class="selectedTableId === t.id ? 'bg-blue-900/50 text-blue-300' : 'text-gray-400 hover:bg-gray-800'"
              @click="selectTable(t.id)"
            >
              <div class="flex items-center gap-1.5 min-w-0">
                <span class="truncate">{{ t.name || t.id }}</span>
                <span class="text-gray-600 shrink-0">({{ (t.entries || []).length }})</span>
              </div>
              <div class="opacity-0 group-hover:opacity-100 flex gap-1 shrink-0">
                <button
                  class="text-yellow-500 hover:text-yellow-400 px-1"
                  @click.stop="handleResetTable(t.id)"
                  title="Reset to YAML default"
                >r</button>
                <button
                  class="text-red-500 hover:text-red-400 px-1"
                  @click.stop="handleDeleteTable(t.id)"
                  title="Delete"
                >x</button>
              </div>
            </div>
          </div>
          <p v-if="store.randomTables.length === 0" class="text-gray-600 text-xs px-2">No tables in this world.</p>
        </div>

        <!-- Right: editor -->
        <div v-if="selectedTableId" class="flex-1 max-h-[70vh] overflow-y-auto">
          <div class="flex flex-wrap items-center gap-3 mb-3">
            <div class="flex items-center gap-1">
              <label class="text-xs text-gray-500">Name:</label>
              <input v-model="editMeta.name"
                class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-0.5 text-xs font-mono focus:outline-none focus:border-gray-500 w-48" />
            </div>
            <div class="flex items-center gap-1">
              <label class="text-xs text-gray-500">Category:</label>
              <input v-model="editMeta.category"
                class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-0.5 text-xs font-mono focus:outline-none focus:border-gray-500 w-32" />
            </div>
            <div class="flex items-center gap-1">
              <label class="text-xs text-gray-500">Tags:</label>
              <input v-model="editMeta.tags"
                class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-0.5 text-xs font-mono focus:outline-none focus:border-gray-500 w-48"
                placeholder="comma-separated" />
            </div>
          </div>

          <div class="flex items-center gap-2 mb-2">
            <span class="text-xs text-gray-500">Entries (JSON):</span>
            <span
              class="w-2 h-2 rounded-full"
              :class="editValid ? 'bg-green-500' : 'bg-red-500'"
            ></span>
            <button
              class="ml-auto px-3 py-1 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
              :disabled="!editValid"
              @click="handleSaveTable"
            >Save</button>
          </div>

          <textarea
            :value="editJson"
            @input="onEditJsonInput"
            rows="24"
            class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-3 py-2 text-xs font-mono focus:outline-none focus:border-gray-500"
            spellcheck="false"
          ></textarea>
        </div>

        <div v-else class="flex-1 flex items-center justify-center text-gray-600 text-sm">
          Select a table to edit.
        </div>
      </div>
    </template>

    <!-- ============ YAML MODE (global, no world) ============ -->
    <template v-else>
      <div class="flex items-center gap-3 mb-3 flex-wrap">
        <h2 class="text-sm font-bold text-gray-300 uppercase tracking-wide">Random Tables (Global YAML)</h2>
        <span class="text-xs text-gray-500">
          {{ allFiles.length }} files, {{ totalEntries }} entries
        </span>
        <span v-if="todoCount > 0" class="text-xs text-yellow-500">
          {{ todoCount }} need work
        </span>
        <span v-if="statusMsg" class="text-xs text-green-400">{{ statusMsg }}</span>
        <div class="ml-auto flex items-center gap-2">
          <label class="flex items-center gap-1 text-xs text-gray-400 cursor-pointer select-none">
            <input type="checkbox" v-model="filterTodo" class="accent-yellow-500" />
            TODO only
          </label>
          <button
            class="px-2 py-1 text-xs rounded border border-blue-700 text-blue-400 hover:bg-blue-900/30"
            @click="handleDownloadZip"
            title="Download all tables as zip"
          >Download All</button>
          <button
            class="px-2 py-1 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
            @click="startZipUpload"
            title="Upload zip of table files"
          >Upload Zip</button>
          <input ref="zipInput" type="file" accept=".zip" class="hidden" @change="handleZipSelected" />
          <button
            class="px-2 py-1 text-xs rounded border border-gray-600 text-gray-400 hover:text-gray-200"
            @click="loadData"
          >Reload</button>
        </div>
      </div>

      <div class="flex gap-4">
        <div class="w-72 shrink-0 max-h-[70vh] overflow-y-auto">
          <div v-for="(files, category) in fileTree" :key="category" class="mb-2">
            <div class="text-xs font-bold text-gray-500 uppercase tracking-wide px-2 py-0.5">{{ category }}</div>
            <div
              v-for="f in files"
              :key="f.path"
              class="group flex items-center justify-between px-2 py-0.5 text-xs cursor-pointer rounded"
              :class="selectedFile === f.path ? 'bg-blue-900/50 text-blue-300' : 'text-gray-400 hover:bg-gray-800'"
              @click="selectFile(f.path)"
            >
              <div class="flex items-center gap-1.5 min-w-0">
                <span
                  v-if="f.has_todo"
                  class="w-1.5 h-1.5 rounded-full bg-yellow-500 shrink-0"
                  title="Needs work (TODO/PLACEHOLDER)"
                ></span>
                <span class="truncate">{{ fileName(f.path) }}</span>
                <span class="text-gray-600 shrink-0" v-if="f.entry_count >= 0">({{ f.entry_count }})</span>
              </div>
              <button
                class="opacity-0 group-hover:opacity-100 text-red-500 hover:text-red-400 px-1 shrink-0"
                @click.stop="handleDeleteFile(f.path)"
                title="Delete file"
              >x</button>
            </div>
          </div>
          <p v-if="filteredFiles.length === 0" class="text-gray-600 text-xs px-2">
            {{ filterTodo ? 'No TODO tables found.' : 'No table files found.' }}
          </p>
        </div>

        <div v-if="selectedFile" class="flex-1 max-h-[70vh] overflow-y-auto">
          <SchemaFormEditor :filePath="selectedFile" @status="onStatus" />
        </div>
        <div v-else class="flex-1 flex items-center justify-center text-gray-600 text-sm">
          Select a table file to edit.
        </div>
      </div>
    </template>
  </div>
</template>
