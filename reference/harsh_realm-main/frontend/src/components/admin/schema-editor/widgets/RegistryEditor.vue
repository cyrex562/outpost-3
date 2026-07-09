<script setup lang="ts">
import { ref, watch, computed } from "vue";
import type { FieldDef, EntryRecord } from "../types";
import FieldInput from "../shared/FieldInput.vue";

const props = defineProps<{
  entries: EntryRecord[];
  fields: FieldDef[];
  entryKey?: string | undefined;
  allowAdd?: boolean | undefined;
  allowDelete?: boolean | undefined;
}>();

const emit = defineEmits<{
  "update:entries": [entries: EntryRecord[]];
}>();

const selectedIdx = ref<number | null>(null);
const editItem = ref<EntryRecord | null>(null);

const keyField = computed(() => props.entryKey ?? "id");

watch(() => props.entries, () => {
  if (selectedIdx.value !== null && selectedIdx.value >= props.entries.length) {
    selectedIdx.value = null;
    editItem.value = null;
  }
}, { deep: true });

function selectEntry(idx: number) {
  selectedIdx.value = idx;
  editItem.value = { ...props.entries[idx] };
}

function updateField(key: string, val: unknown) {
  if (!editItem.value) return;
  editItem.value = { ...editItem.value, [key]: val };
}

function applyChanges() {
  if (editItem.value === null || selectedIdx.value === null) return;
  const next = props.entries.map((e) => ({ ...e }));
  next[selectedIdx.value] = { ...editItem.value };
  emit("update:entries", next);
}

function addEntry() {
  const entry: EntryRecord = {};
  for (const f of props.fields) {
    entry[f.key] = f.default ?? (f.type === "number" ? 0 : f.type === "tags" ? [] : f.type === "boolean" ? false : "");
  }
  const next = [...props.entries, entry];
  emit("update:entries", next);
  selectedIdx.value = next.length - 1;
  editItem.value = { ...entry };
}

function deleteEntry() {
  if (selectedIdx.value === null) return;
  const next = [...props.entries];
  next.splice(selectedIdx.value, 1);
  emit("update:entries", next);
  selectedIdx.value = null;
  editItem.value = null;
}
</script>

<template>
  <div class="flex gap-4">
    <!-- Left: entry list -->
    <div class="w-56 shrink-0 max-h-[60vh] overflow-y-auto">
      <div
        v-for="(entry, idx) in entries"
        :key="String(entry[keyField] ?? idx)"
        class="px-2 py-1 text-xs cursor-pointer rounded mb-0.5"
        :class="selectedIdx === idx ? 'bg-blue-900/50 text-blue-300' : 'text-gray-400 hover:bg-gray-800'"
        @click="selectEntry(idx)"
      >
        <span class="font-mono">{{ entry[keyField] }}</span>
        <span v-if="entry['name']" class="text-gray-500 ml-1">{{ entry["name"] }}</span>
      </div>
      <p v-if="entries.length === 0" class="text-gray-600 text-xs px-2">No entries.</p>
      <div v-if="allowAdd !== false" class="mt-2 px-2">
        <button
          class="px-2 py-0.5 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
          @click="addEntry"
        >+ Add</button>
      </div>
    </div>

    <!-- Right: detail form -->
    <div v-if="editItem" class="flex-1 max-h-[60vh] overflow-y-auto">
      <div class="space-y-2">
        <div v-for="f in fields" :key="f.key" class="flex items-start gap-2">
          <label class="text-xs text-gray-500 w-32 text-right shrink-0 pt-0.5">{{ f.label ?? f.key }}</label>
          <FieldInput
            :field="f"
            :modelValue="editItem[f.key]"
            @update:modelValue="updateField(f.key, $event)"
          />
        </div>
      </div>
      <div class="flex gap-2 mt-3">
        <button
          class="px-3 py-1 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
          @click="applyChanges"
        >Apply</button>
        <button
          v-if="allowDelete !== false"
          class="px-3 py-1 text-xs rounded border border-red-700 text-red-400 hover:bg-red-900/30"
          @click="deleteEntry"
        >Delete</button>
      </div>
    </div>

    <div v-else class="flex-1 flex items-center justify-center text-gray-600 text-xs">
      Select an entry to edit.
    </div>
  </div>
</template>
