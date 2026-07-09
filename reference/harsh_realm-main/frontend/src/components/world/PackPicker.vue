<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";

import { fetchPacks } from "../../api/packs";
import type { PackManifest } from "../../types/api";

const props = defineProps<{
  modelValue: string[];
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string[]];
}>();

const packs = ref<PackManifest[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);

const selected = computed({
  get: () => props.modelValue,
  set: (value: string[]) => emit("update:modelValue", value),
});

const orderedPacks = computed(() =>
  selected.value
    .map((id) => packs.value.find((pack) => pack.id === id))
    .filter((pack): pack is PackManifest => pack !== undefined)
);

onMounted(async () => {
  loading.value = true;
  error.value = null;
  try {
    packs.value = await fetchPacks();
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  } finally {
    loading.value = false;
  }
});

watch(
  packs,
  (loadedPacks) => {
    if (selected.value.length > 0 || loadedPacks.length === 0) return;
    const defaultPack = loadedPacks.find((pack) => pack.id === "xwn-core");
    selected.value = defaultPack ? [defaultPack.id] : [loadedPacks[0].id];
  },
  { immediate: true }
);

function isSelected(packId: string): boolean {
  return selected.value.includes(packId);
}

function togglePack(packId: string): void {
  if (isSelected(packId)) {
    if (
      packId === "xwn-core" &&
      !confirm("Create a world without XWN Core content?")
    ) {
      return;
    }
    selected.value = selected.value.filter((id) => id !== packId);
    return;
  }
  selected.value = [...selected.value, packId];
}

function movePack(packId: string, delta: number): void {
  const current = [...selected.value];
  const index = current.indexOf(packId);
  const target = index + delta;
  if (index < 0 || target < 0 || target >= current.length) return;
  const [item] = current.splice(index, 1);
  current.splice(target, 0, item);
  selected.value = current;
}
</script>

<template>
  <div class="space-y-2" data-testid="pack-picker">
    <div class="flex items-center justify-between">
      <label class="block text-xs text-gray-500 font-mono">Packs</label>
      <span v-if="loading" class="text-xs text-gray-600 font-mono">Loading</span>
    </div>

    <div v-if="error" class="text-xs text-red-300 font-mono">
      {{ error }}
    </div>

    <div class="space-y-1">
      <label
        v-for="pack in packs"
        :key="pack.id"
        class="flex items-start gap-2 rounded border border-gray-700 bg-gray-800 px-2 py-2"
        :data-testid="`pack-option-${pack.id}`"
      >
        <input
          type="checkbox"
          class="mt-0.5"
          :checked="isSelected(pack.id)"
          :data-testid="`pack-checkbox-${pack.id}`"
          @change="togglePack(pack.id)"
        />
        <span class="min-w-0 flex-1">
          <span class="block text-xs text-gray-200 font-mono">
            {{ pack.name }} <span class="text-gray-500">v{{ pack.version }}</span>
          </span>
          <span class="block text-[11px] text-gray-500 leading-snug">
            {{ pack.description || pack.id }}
          </span>
        </span>
      </label>
    </div>

    <div v-if="orderedPacks.length > 1" class="space-y-1">
      <div class="text-xs text-gray-500 font-mono">Load order</div>
      <div
        v-for="pack in orderedPacks"
        :key="pack.id"
        class="flex items-center gap-2 text-xs text-gray-300"
        :data-testid="`pack-order-${pack.id}`"
      >
        <span class="min-w-0 flex-1 truncate font-mono">{{ pack.id }}</span>
        <button
          class="px-1.5 py-0.5 rounded border border-gray-700 hover:border-gray-500"
          :data-testid="`pack-up-${pack.id}`"
          @click="movePack(pack.id, -1)"
        >
          ↑
        </button>
        <button
          class="px-1.5 py-0.5 rounded border border-gray-700 hover:border-gray-500"
          :data-testid="`pack-down-${pack.id}`"
          @click="movePack(pack.id, 1)"
        >
          ↓
        </button>
      </div>
    </div>
  </div>
</template>
