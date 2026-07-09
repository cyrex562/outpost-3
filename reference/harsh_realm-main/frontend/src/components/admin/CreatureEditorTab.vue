<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { useAdminStore } from "../../stores/admin";
import type { EditorRecord } from "../../types/api";
import SchemaFormEditor from "./schema-editor/SchemaFormEditor.vue";
import ImportExportBar from "./ImportExportBar.vue";
import OverrideIndicator from "./OverrideIndicator.vue";

const store = useAdminStore();
const statusMsg = ref("");
const selectedFile = ref("creatures/beasts.yaml");
const selectedCreatureId = ref<string | null>(null);
const editJson = ref("");
const editValid = ref(true);

const files = [
  "creatures/beasts.yaml", "creatures/humanoids.yaml", "creatures/undead.yaml",
  "creatures/constructs.yaml", "creatures/mythical.yaml", "creatures/elemental.yaml",
];

const isWorldMode = computed(() => !!store.activeWorldPath);

async function loadData() {
  if (isWorldMode.value) await store.loadWorldCreatures();
}

watch(() => store.activeWorldPath, () => { selectedCreatureId.value = null; loadData(); });
onMounted(loadData);

function onStatus(msg: string) {
  statusMsg.value = msg;
  if (msg) setTimeout(() => { statusMsg.value = ""; }, 3000);
}

// World mode
const creatureTree = computed(() => {
  const tree: Record<string, EditorRecord[]> = {};
  for (const c of store.worldCreatures) {
    const tags = Array.isArray(c.tags) ? c.tags : [];
    const cat = (typeof tags[0] === "string" ? tags[0] : "") || "other";
    if (!tree[cat]) tree[cat] = [];
    tree[cat].push(c);
  }
  return tree;
});

const selectedCreatureOverridden = computed(() => {
  try {
    const parsed = JSON.parse(editJson.value) as { _overridden?: unknown };
    return parsed._overridden === true;
  } catch {
    return false;
  }
});

function selectCreature(id: string) {
  selectedCreatureId.value = id;
  const creature = store.worldCreatures.find((c) => c.id === id);
  if (creature) {
    editJson.value = JSON.stringify(creature, null, 2);
    editValid.value = true;
  }
  statusMsg.value = "";
}

function onEditInput(event: Event) {
  const val = (event.target as HTMLTextAreaElement).value;
  editJson.value = val;
  try { JSON.parse(val); editValid.value = true; } catch { editValid.value = false; }
}

async function handleSave() {
  if (!selectedCreatureId.value || !editValid.value) return;
  const data = JSON.parse(editJson.value);
  data.id = selectedCreatureId.value;
  await store.saveWorldCreature(data);
  statusMsg.value = "Saved.";
  await store.loadWorldCreatures();
  setTimeout(() => { statusMsg.value = ""; }, 3000);
}

async function handleReset(id: string) {
  if (!confirm(`Reset "${id}" to YAML default?`)) return;
  await store.resetWorldCreature(id);
  await store.loadWorldCreatures();
  if (selectedCreatureId.value === id) selectCreature(id);
  statusMsg.value = `Reset ${id}.`;
  setTimeout(() => { statusMsg.value = ""; }, 3000);
}

async function handleDelete(id: string) {
  if (!confirm(`Delete "${id}"?`)) return;
  await store.deleteWorldCreature(id);
  if (selectedCreatureId.value === id) selectedCreatureId.value = null;
  await store.loadWorldCreatures();
  statusMsg.value = `Deleted ${id}.`;
  setTimeout(() => { statusMsg.value = ""; }, 3000);
}

async function handleResetAll() {
  if (!confirm("Reset ALL creatures to YAML defaults?")) return;
  await store.resetAllWorldCreatures();
  selectedCreatureId.value = null;
  await store.loadWorldCreatures();
  statusMsg.value = "All creatures reset.";
  setTimeout(() => { statusMsg.value = ""; }, 3000);
}
</script>

<template>
  <div>
    <!-- WORLD MODE -->
    <template v-if="isWorldMode">
      <div class="flex items-center gap-3 mb-3 flex-wrap">
        <h2 class="text-sm font-bold text-gray-300 uppercase tracking-wide">Creatures (World)</h2>
        <span class="text-xs text-gray-500">{{ store.worldCreatures.length }} creatures</span>
        <span v-if="statusMsg" class="text-xs text-green-400">{{ statusMsg }}</span>
        <div class="ml-auto flex items-center gap-2">
          <button class="px-2 py-1 text-xs rounded border border-yellow-700 text-yellow-400 hover:bg-yellow-900/30"
            @click="handleResetAll">Reset All</button>
          <ImportExportBar table="creature_templates" />
          <button class="px-2 py-1 text-xs rounded border border-gray-600 text-gray-400 hover:text-gray-200"
            @click="loadData">Reload</button>
        </div>
      </div>
      <div class="flex gap-4">
        <div class="w-64 shrink-0 max-h-[70vh] overflow-y-auto">
          <div v-for="(creatures, cat) in creatureTree" :key="cat" class="mb-2">
            <div class="text-xs font-bold text-gray-500 uppercase tracking-wide px-2 py-0.5">{{ cat }}</div>
            <div v-for="c in creatures" :key="c.id"
              class="group flex items-center justify-between px-2 py-0.5 text-xs cursor-pointer rounded"
              :class="selectedCreatureId === c.id ? 'bg-blue-900/50 text-blue-300' : 'text-gray-400 hover:bg-gray-800'"
              @click="selectCreature(c.id)">
              <span class="truncate">{{ c.name || c.id }}</span>
              <div class="opacity-0 group-hover:opacity-100 flex gap-1 shrink-0">
                <OverrideIndicator :overridden="c._overridden === true" @revert="handleReset(c.id)" />
                <button class="text-yellow-500 hover:text-yellow-400 px-1" @click.stop="handleReset(c.id)" title="Reset">r</button>
                <button class="text-red-500 hover:text-red-400 px-1" @click.stop="handleDelete(c.id)" title="Delete">x</button>
              </div>
            </div>
          </div>
        </div>
        <div v-if="selectedCreatureId" class="flex-1 max-h-[70vh] overflow-y-auto">
          <div class="flex items-center gap-2 mb-2">
            <span class="text-xs text-gray-500">{{ selectedCreatureId }}</span>
            <OverrideIndicator
              :overridden="selectedCreatureOverridden"
              @revert="selectedCreatureId && handleReset(selectedCreatureId)"
            />
            <span class="w-2 h-2 rounded-full" :class="editValid ? 'bg-green-500' : 'bg-red-500'"></span>
            <button class="ml-auto px-3 py-1 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
              :disabled="!editValid" @click="handleSave">Save</button>
          </div>
          <textarea :value="editJson" @input="onEditInput" rows="28"
            class="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded px-3 py-2 text-xs font-mono focus:outline-none focus:border-gray-500"
            spellcheck="false"></textarea>
        </div>
        <div v-else class="flex-1 flex items-center justify-center text-gray-600 text-sm">Select a creature to edit.</div>
      </div>
    </template>

    <!-- YAML MODE -->
    <template v-else>
      <div class="flex items-center gap-3 mb-3">
        <h2 class="text-sm font-bold text-gray-300 uppercase tracking-wide">Creatures (Global YAML)</h2>
        <select v-model="selectedFile" class="bg-gray-800 border border-gray-700 text-gray-200 text-xs rounded px-2 py-1">
          <option v-for="f in files" :key="f" :value="f">{{ f }}</option>
        </select>
        <span v-if="statusMsg" class="text-xs text-green-400">{{ statusMsg }}</span>
      </div>
      <SchemaFormEditor :key="selectedFile" :filePath="selectedFile" @status="onStatus" />
    </template>
  </div>
</template>
