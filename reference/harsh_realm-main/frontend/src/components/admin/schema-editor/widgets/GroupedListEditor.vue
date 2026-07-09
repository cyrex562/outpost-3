<script setup lang="ts">
import { ref } from "vue";
import type { FieldDef, EntryRecord } from "../types";
import FieldInput from "../shared/FieldInput.vue";
import EntryToolbar from "../shared/EntryToolbar.vue";

const props = defineProps<{
  groups: { name: string; entries: EntryRecord[] }[];
  fields: FieldDef[];
  groupFields?: FieldDef[] | undefined;
  allowAdd?: boolean | undefined;
  allowDelete?: boolean | undefined;
}>();

const emit = defineEmits<{
  "update:groups": [groups: { name: string; entries: EntryRecord[] }[]];
}>();

const collapsed = ref<Set<number>>(new Set());

function toggle(idx: number) {
  const next = new Set(collapsed.value);
  if (next.has(idx)) next.delete(idx);
  else next.add(idx);
  collapsed.value = next;
}

function updateGroupField(gIdx: number, key: string, val: unknown) {
  const next = props.groups.map((g) => ({ ...g, entries: [...g.entries] }));
  (next[gIdx] as Record<string, unknown>)[key] = val;
  emit("update:groups", next);
}

function updateEntry(gIdx: number, eIdx: number, key: string, val: unknown) {
  const next = props.groups.map((g) => ({ ...g, entries: g.entries.map((e) => ({ ...e })) }));
  next[gIdx].entries[eIdx] = { ...next[gIdx].entries[eIdx], [key]: val };
  emit("update:groups", next);
}

function addEntry(gIdx: number) {
  const entry: EntryRecord = {};
  for (const f of props.fields) {
    entry[f.key] = f.default ?? (f.type === "number" ? 0 : f.type === "tags" ? [] : "");
  }
  const next = props.groups.map((g) => ({ ...g, entries: [...g.entries] }));
  next[gIdx].entries.push(entry);
  emit("update:groups", next);
}

function deleteEntry(gIdx: number, eIdx: number) {
  const next = props.groups.map((g) => ({ ...g, entries: [...g.entries] }));
  next[gIdx].entries.splice(eIdx, 1);
  emit("update:groups", next);
}

function addGroup() {
  const next = [...props.groups, { name: "New Group", entries: [] }];
  emit("update:groups", next);
}

function deleteGroup(gIdx: number) {
  const next = [...props.groups];
  next.splice(gIdx, 1);
  emit("update:groups", next);
}
</script>

<template>
  <div class="space-y-2">
    <div
      v-for="(group, gIdx) in groups"
      :key="gIdx"
      class="border border-gray-800 rounded"
    >
      <!-- Group header -->
      <div
        class="flex items-center gap-2 px-3 py-1.5 bg-gray-800/50 cursor-pointer"
        @click="toggle(gIdx)"
      >
        <span class="text-xs text-gray-500">{{ collapsed.has(gIdx) ? '&#9654;' : '&#9660;' }}</span>
        <span class="text-xs text-gray-300 font-medium flex-1">{{ group.name }}</span>
        <span class="text-xs text-gray-600">{{ group.entries.length }} entries</span>
        <button
          v-if="allowDelete !== false"
          class="text-xs text-red-400 hover:text-red-300 px-1"
          @click.stop="deleteGroup(gIdx)"
        >&times;</button>
      </div>

      <!-- Group content -->
      <div v-if="!collapsed.has(gIdx)" class="p-2">
        <!-- Group-level fields -->
        <div v-if="groupFields?.length" class="flex flex-wrap gap-2 mb-2">
          <div v-for="f in groupFields" :key="f.key" class="flex items-center gap-1">
            <label class="text-xs text-gray-500">{{ f.label ?? f.key }}:</label>
            <FieldInput
              :field="f"
              :modelValue="(group as unknown as EntryRecord)[f.key]"
              @update:modelValue="updateGroupField(gIdx, f.key, $event)"
            />
          </div>
        </div>

        <!-- Entries table -->
        <table v-if="group.entries.length > 0" class="w-full text-xs">
          <thead>
            <tr class="text-left text-gray-500 border-b border-gray-800">
              <th v-for="f in fields" :key="f.key" class="px-2 py-0.5">{{ f.label ?? f.key }}</th>
              <th class="px-2 py-0.5 w-20">Actions</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="(entry, eIdx) in group.entries"
              :key="eIdx"
              class="border-b border-gray-800/30"
            >
              <td v-for="f in fields" :key="f.key" class="px-2 py-0.5">
                <FieldInput
                  :field="f"
                  :modelValue="entry[f.key]"
                  @update:modelValue="updateEntry(gIdx, eIdx, f.key, $event)"
                />
              </td>
              <td class="px-2 py-0.5">
                <EntryToolbar
                  :canMoveUp="false"
                  :canMoveDown="false"
                  :canClone="false"
                  :canDelete="allowDelete !== false"
                  @add="addEntry(gIdx)"
                  @delete="deleteEntry(gIdx, eIdx)"
                />
              </td>
            </tr>
          </tbody>
        </table>
        <div v-else class="text-gray-600 text-xs py-2 text-center">
          No entries.
        </div>
        <button
          v-if="allowAdd !== false"
          class="mt-1 px-2 py-0.5 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
          @click="addEntry(gIdx)"
        >+ Add Entry</button>
      </div>
    </div>

    <button
      v-if="allowAdd !== false"
      class="px-2 py-0.5 text-xs rounded border border-blue-700 text-blue-400 hover:bg-blue-900/30"
      @click="addGroup"
    >+ Add Group</button>
  </div>
</template>
