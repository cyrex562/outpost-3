<script setup lang="ts">
import type { FieldDef, EntryRecord } from "../types";
import FieldInput from "../shared/FieldInput.vue";
import EntryToolbar from "../shared/EntryToolbar.vue";

const props = defineProps<{
  entries: EntryRecord[];
  fields: FieldDef[];
  allowAdd?: boolean | undefined;
  allowDelete?: boolean | undefined;
}>();

const emit = defineEmits<{
  "update:entries": [entries: EntryRecord[]];
}>();

const rangeFields: FieldDef[] = [
  { key: "low", type: "number", label: "Low", width: "sm" },
  { key: "high", type: "number", label: "High", width: "sm" },
];

const allFields = [...rangeFields, ...props.fields.filter((f) => f.key !== "low" && f.key !== "high")];

function update(idx: number, key: string, val: unknown) {
  const next = props.entries.map((e) => ({ ...e }));
  next[idx] = { ...next[idx], [key]: val };
  emit("update:entries", next);
}

function addEntry() {
  const entry: EntryRecord = {};
  for (const f of allFields) {
    entry[f.key] = f.default ?? (f.type === "number" ? 0 : "");
  }
  emit("update:entries", [...props.entries, entry]);
}

function deleteEntry(idx: number) {
  const next = [...props.entries];
  next.splice(idx, 1);
  emit("update:entries", next);
}

function moveEntry(idx: number, dir: -1 | 1) {
  const next = [...props.entries];
  const target = idx + dir;
  if (target < 0 || target >= next.length) return;
  [next[idx], next[target]] = [next[target], next[idx]];
  emit("update:entries", next);
}
</script>

<template>
  <div class="overflow-x-auto">
    <table class="w-full text-xs">
      <thead>
        <tr class="text-left text-gray-500 border-b border-gray-800">
          <th v-for="f in allFields" :key="f.key" class="px-2 py-1">{{ f.label ?? f.key }}</th>
          <th class="px-2 py-1 w-28">Actions</th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="(entry, idx) in entries"
          :key="idx"
          class="border-b border-gray-800/50 hover:bg-gray-900/50"
        >
          <td v-for="f in allFields" :key="f.key" class="px-2 py-0.5">
            <FieldInput
              :field="f"
              :modelValue="entry[f.key]"
              @update:modelValue="update(idx, f.key, $event)"
            />
          </td>
          <td class="px-2 py-0.5">
            <EntryToolbar
              :canMoveUp="idx > 0"
              :canMoveDown="idx < entries.length - 1"
              :canDelete="allowDelete !== false"
              @add="addEntry"
              @delete="deleteEntry(idx)"
              @moveUp="moveEntry(idx, -1)"
              @moveDown="moveEntry(idx, 1)"
            />
          </td>
        </tr>
      </tbody>
    </table>
    <div v-if="entries.length === 0" class="text-gray-600 text-xs py-4 text-center">
      No ranges defined.
      <button class="ml-2 text-green-400 underline" @click="addEntry">Add one</button>
    </div>
    <div v-else-if="allowAdd !== false" class="mt-2">
      <button
        class="px-2 py-0.5 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
        @click="addEntry"
      >+ Add Range</button>
    </div>
  </div>
</template>
