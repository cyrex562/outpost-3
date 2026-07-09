<script setup lang="ts">
import { ref, computed, watch } from "vue";
import type { FieldDef, EntryRecord } from "../types";
import FieldInput from "../shared/FieldInput.vue";
import EntryToolbar from "../shared/EntryToolbar.vue";

const props = defineProps<{
  tiers: Record<string, { name?: string; items: EntryRecord[] }>;
  tierKeys: string[];
  fields: FieldDef[];
  allowAdd?: boolean | undefined;
  allowDelete?: boolean | undefined;
}>();

const emit = defineEmits<{
  "update:tiers": [tiers: Record<string, { name?: string; items: EntryRecord[] }>];
}>();

const activeTier = ref(props.tierKeys[0] ?? "");

watch(() => props.tierKeys, (keys) => {
  if (!keys.includes(activeTier.value) && keys.length > 0) {
    activeTier.value = keys[0];
  }
});

const currentItems = computed(() => props.tiers[activeTier.value]?.items ?? []);
const currentName = computed(() => props.tiers[activeTier.value]?.name ?? "");

function updateItem(idx: number, key: string, val: unknown) {
  const next = { ...props.tiers };
  const tier = { ...next[activeTier.value] };
  const items = tier.items.map((e) => ({ ...e }));
  items[idx] = { ...items[idx], [key]: val };
  tier.items = items;
  next[activeTier.value] = tier;
  emit("update:tiers", next);
}

function updateTierName(val: string) {
  const next = { ...props.tiers };
  next[activeTier.value] = { ...next[activeTier.value], name: val };
  emit("update:tiers", next);
}

function addItem() {
  const entry: EntryRecord = {};
  for (const f of props.fields) {
    entry[f.key] = f.default ?? (f.type === "number" ? 0 : f.type === "tags" ? [] : "");
  }
  const next = { ...props.tiers };
  const tier = { ...next[activeTier.value] };
  tier.items = [...tier.items, entry];
  next[activeTier.value] = tier;
  emit("update:tiers", next);
}

function deleteItem(idx: number) {
  const next = { ...props.tiers };
  const tier = { ...next[activeTier.value] };
  tier.items = [...tier.items];
  tier.items.splice(idx, 1);
  next[activeTier.value] = tier;
  emit("update:tiers", next);
}

function moveItem(idx: number, dir: -1 | 1) {
  const target = idx + dir;
  const items = [...currentItems.value];
  if (target < 0 || target >= items.length) return;
  [items[idx], items[target]] = [items[target], items[idx]];
  const next = { ...props.tiers };
  next[activeTier.value] = { ...next[activeTier.value], items };
  emit("update:tiers", next);
}
</script>

<template>
  <div>
    <!-- Tier tabs -->
    <div class="flex items-center gap-1 mb-3 border-b border-gray-800 pb-2">
      <button
        v-for="tier in tierKeys"
        :key="tier"
        class="px-3 py-1 text-xs rounded-t border border-b-0"
        :class="activeTier === tier
          ? 'border-gray-600 text-gray-200 bg-gray-800'
          : 'border-transparent text-gray-500 hover:text-gray-300'"
        @click="activeTier = tier"
      >{{ tier }}</button>
    </div>

    <!-- Tier name -->
    <div class="flex items-center gap-2 mb-3">
      <label class="text-xs text-gray-500">Name:</label>
      <input
        :value="currentName"
        class="bg-gray-800 border border-gray-700 text-gray-200 rounded px-2 py-0.5 text-xs w-48 focus:outline-none focus:border-gray-500"
        @input="updateTierName(($event.target as HTMLInputElement).value)"
      />
    </div>

    <!-- Items table -->
    <table class="w-full text-xs">
      <thead>
        <tr class="text-left text-gray-500 border-b border-gray-800">
          <th v-for="f in fields" :key="f.key" class="px-2 py-1">{{ f.label ?? f.key }}</th>
          <th class="px-2 py-1 w-28">Actions</th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="(item, idx) in currentItems"
          :key="idx"
          class="border-b border-gray-800/50 hover:bg-gray-900/50"
        >
          <td v-for="f in fields" :key="f.key" class="px-2 py-0.5">
            <FieldInput
              :field="f"
              :modelValue="item[f.key]"
              @update:modelValue="updateItem(idx, f.key, $event)"
            />
          </td>
          <td class="px-2 py-0.5">
            <EntryToolbar
              :canMoveUp="idx > 0"
              :canMoveDown="idx < currentItems.length - 1"
              :canDelete="allowDelete !== false"
              :canClone="false"
              @add="addItem"
              @delete="deleteItem(idx)"
              @moveUp="moveItem(idx, -1)"
              @moveDown="moveItem(idx, 1)"
            />
          </td>
        </tr>
      </tbody>
    </table>
    <div v-if="currentItems.length === 0" class="text-gray-600 text-xs py-4 text-center">
      No items in this tier.
      <button class="ml-2 text-green-400 underline" @click="addItem">Add one</button>
    </div>
    <div v-else-if="allowAdd !== false" class="mt-2">
      <button
        class="px-2 py-0.5 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30"
        @click="addItem"
      >+ Add Item</button>
    </div>
  </div>
</template>
