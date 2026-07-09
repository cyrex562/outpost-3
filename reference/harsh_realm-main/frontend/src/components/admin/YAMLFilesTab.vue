<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useAdminStore } from "../../stores/admin";
import jsyaml from "js-yaml";

const store = useAdminStore();
const statusMsg = ref("");
const uploadPath = ref<string | null>(null);
const fileInput = ref<HTMLInputElement | null>(null);

// Inline editor state
const editingPath = ref<string | null>(null);
const editContent = ref("");
const editValid = ref<boolean | null>(null);
const editError = ref("");
const editSaving = ref(false);

// New file state
const showNewFile = ref(false);
const newFilePath = ref("");

// Drag and drop
const dragOver = ref(false);

// Group files by directory
const fileTree = computed(() => {
  const tree: Record<string, { path: string; size: number }[]> = {};
  for (const f of store.yamlFiles) {
    const parts = f.path.split(/[/\\]/);
    const dir = parts.length > 1 ? parts.slice(0, -1).join("/") : ".";
    if (!tree[dir]) tree[dir] = [];
    tree[dir].push(f);
  }
  return tree;
});

async function loadData() {
  await store.loadYamlFiles();
}

async function handleDownload(path: string) {
  await store.downloadYaml(path);
}

function startUpload(path: string) {
  uploadPath.value = path;
  if (fileInput.value) {
    fileInput.value.click();
  }
}

async function handleFileSelected(event: Event) {
  const input = event.target as HTMLInputElement;
  if (!input.files?.length || !uploadPath.value) return;
  const file = input.files[0];
  await store.uploadYaml(uploadPath.value, file);
  input.value = "";
  uploadPath.value = null;
  await store.loadYamlFiles();
  statusMsg.value = "File uploaded.";
  setTimeout(() => { statusMsg.value = ""; }, 3000);
}

// Inline editor
async function openEditor(path: string) {
  if (editingPath.value === path) {
    editingPath.value = null;
    return;
  }
  editingPath.value = path;
  editContent.value = "";
  editValid.value = null;
  editError.value = "";
  const content = await store.getYamlContent(path);
  editContent.value = content;
  validateYaml(content);
}

function validateYaml(content: string) {
  try {
    jsyaml.load(content);
    editValid.value = true;
    editError.value = "";
  } catch (e) {
    editValid.value = false;
    editError.value = e instanceof Error ? e.message : "Invalid YAML";
  }
}

function onEditInput(event: Event) {
  const val = (event.target as HTMLTextAreaElement).value;
  editContent.value = val;
  validateYaml(val);
}

async function saveEdit() {
  if (!editingPath.value || !editValid.value) return;
  editSaving.value = true;
  try {
    await store.saveYamlContent(editingPath.value, editContent.value);
    statusMsg.value = `Saved ${editingPath.value}.`;
    await store.loadYamlFiles();
    setTimeout(() => { statusMsg.value = ""; }, 3000);
  } catch {
    statusMsg.value = "Save failed.";
  } finally {
    editSaving.value = false;
  }
}

// New file creation
async function createNewFile() {
  if (!newFilePath.value.trim()) return;
  let path = newFilePath.value.trim();
  if (!path.endsWith(".yaml") && !path.endsWith(".yml")) {
    path += ".yaml";
  }
  const template = "# New YAML file\n";
  await store.createYamlFile(path, template);
  showNewFile.value = false;
  newFilePath.value = "";
  await store.loadYamlFiles();
  statusMsg.value = `Created ${path}.`;
  setTimeout(() => { statusMsg.value = ""; }, 3000);
}

// Drag and drop
function onDragOver(e: DragEvent) {
  e.preventDefault();
  dragOver.value = true;
}

function onDragLeave() {
  dragOver.value = false;
}

async function onDrop(e: DragEvent) {
  e.preventDefault();
  dragOver.value = false;
  const files = e.dataTransfer?.files;
  if (!files?.length) return;

  for (const file of Array.from(files)) {
    const name = file.name;
    if (!name.endsWith(".yaml") && !name.endsWith(".yml")) {
      statusMsg.value = `Skipped ${name} — not a YAML file.`;
      continue;
    }
    // Read and validate
    const text = await file.text();
    try {
      jsyaml.load(text);
    } catch {
      statusMsg.value = `Invalid YAML in ${name}. Skipped.`;
      continue;
    }
    await store.saveYamlContent(name, text);
    statusMsg.value = `Uploaded ${name}.`;
  }
  await store.loadYamlFiles();
  setTimeout(() => { statusMsg.value = ""; }, 3000);
}

function fileName(path: string): string {
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1];
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  return `${(bytes / 1024).toFixed(1)} KB`;
}

onMounted(loadData);
</script>

<template>
  <div>
    <div class="flex items-center justify-between mb-3">
      <h2 class="text-sm font-bold text-gray-300 uppercase tracking-wide">YAML Data Files</h2>
      <div class="flex items-center gap-2">
        <span v-if="statusMsg" class="text-xs text-green-400">{{ statusMsg }}</span>
        <button
          class="px-2 py-1 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
          @click="showNewFile = !showNewFile"
        >
          + New File
        </button>
        <button
          class="px-2 py-1 text-xs rounded border border-gray-600 text-gray-400 hover:text-gray-200"
          @click="loadData"
        >
          Reload
        </button>
      </div>
    </div>

    <!-- New file form -->
    <div v-if="showNewFile" class="flex items-center gap-2 mb-3 p-2 bg-gray-900 rounded border border-gray-800">
      <label class="text-xs text-gray-500">Path (relative to data/):</label>
      <input
        v-model="newFilePath"
        class="flex-1 bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs font-mono focus:outline-none focus:border-gray-500"
        placeholder="tables/my_table.yaml"
        @keyup.enter="createNewFile"
      />
      <button
        class="px-2 py-1 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
        @click="createNewFile"
      >Create</button>
      <button
        class="px-2 py-1 text-xs rounded border border-gray-600 text-gray-400 hover:text-gray-200"
        @click="showNewFile = false"
      >Cancel</button>
    </div>

    <!-- Drag and drop zone -->
    <div
      class="mb-4 border-2 border-dashed rounded p-4 text-center text-xs transition-colors"
      :class="dragOver
        ? 'border-blue-500 bg-blue-950/20 text-blue-400'
        : 'border-gray-700 text-gray-600'"
      @dragover="onDragOver"
      @dragleave="onDragLeave"
      @drop="onDrop"
    >
      Drop .yaml / .yml files here to upload
    </div>

    <input
      ref="fileInput"
      type="file"
      accept=".yaml,.yml"
      class="hidden"
      @change="handleFileSelected"
    />

    <div v-for="(files, dir) in fileTree" :key="dir" class="mb-4">
      <h3 class="text-xs font-bold text-gray-500 uppercase mb-1 border-b border-gray-800 pb-1">
        {{ dir }}
      </h3>
      <div v-for="f in files" :key="f.path">
        <div
          class="flex items-center justify-between px-2 py-1 text-xs hover:bg-gray-900/50 rounded cursor-pointer"
          @click="openEditor(f.path)"
        >
          <div class="flex items-center gap-2">
            <span class="text-gray-400 font-mono">{{ fileName(f.path) }}</span>
            <span class="text-gray-600">{{ formatSize(f.size) }}</span>
            <span
              v-if="editingPath === f.path"
              class="text-blue-400"
            >editing</span>
          </div>
          <div class="flex gap-1" @click.stop>
            <button
              class="px-2 py-0.5 text-xs rounded border border-gray-600 text-gray-400 hover:text-gray-200 hover:bg-gray-800"
              @click="handleDownload(f.path)"
            >
              Download
            </button>
            <button
              class="px-2 py-0.5 text-xs rounded border border-blue-700 text-blue-400 hover:bg-blue-900/30"
              @click="startUpload(f.path)"
            >
              Replace
            </button>
          </div>
        </div>

        <!-- Inline editor -->
        <div v-if="editingPath === f.path" class="mx-2 mb-2 p-2 bg-gray-900 rounded border border-gray-800">
          <div class="flex items-center justify-between mb-1">
            <div class="flex items-center gap-2">
              <span
                class="w-2 h-2 rounded-full"
                :class="editValid === true ? 'bg-green-500' : editValid === false ? 'bg-red-500' : 'bg-gray-600'"
              ></span>
              <span class="text-xs" :class="editValid === false ? 'text-red-400' : 'text-gray-500'">
                {{ editValid === false ? editError : editValid === true ? 'Valid YAML' : 'Checking...' }}
              </span>
            </div>
            <div class="flex gap-1">
              <button
                class="px-2 py-0.5 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
                :disabled="!editValid || editSaving"
                @click="saveEdit"
              >{{ editSaving ? "Saving..." : "Save" }}</button>
              <button
                class="px-2 py-0.5 text-xs rounded border border-gray-600 text-gray-400 hover:text-gray-200"
                @click="editingPath = null"
              >Close</button>
            </div>
          </div>
          <textarea
            :value="editContent"
            @input="onEditInput"
            rows="16"
            class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-1 text-xs font-mono focus:outline-none focus:border-gray-500"
            spellcheck="false"
          ></textarea>
        </div>
      </div>
    </div>

    <p v-if="store.yamlFiles.length === 0 && !store.loading" class="text-gray-600 text-sm mt-4">
      No YAML files found in data directory.
    </p>
  </div>
</template>
