<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{
  matrix: Record<string, Record<string, unknown>>;
  rowKeys: string[];
  colKeys: string[];
  cellType?: "number" | "text";
}>();

const emit = defineEmits<{
  "update:matrix": [matrix: Record<string, Record<string, unknown>>];
}>();

const inputType = computed(() => props.cellType ?? "number");

function updateCell(row: string, col: string, val: string) {
  const next: Record<string, Record<string, unknown>> = {};
  for (const rk of props.rowKeys) {
    next[rk] = { ...(props.matrix[rk] ?? {}) };
  }
  next[row][col] = inputType.value === "number" ? Number(val) : val;
  emit("update:matrix", next);
}
</script>

<template>
  <div class="overflow-x-auto">
    <table class="text-xs">
      <thead>
        <tr class="text-left text-gray-500 border-b border-gray-800">
          <th class="px-2 py-1 sticky left-0 bg-gray-900"></th>
          <th v-for="col in colKeys" :key="col" class="px-2 py-1 text-center min-w-[60px]">{{ col }}</th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="row in rowKeys"
          :key="row"
          class="border-b border-gray-800/50"
        >
          <td class="px-2 py-0.5 text-gray-400 sticky left-0 bg-gray-900 font-mono">{{ row }}</td>
          <td v-for="col in colKeys" :key="col" class="px-1 py-0.5">
            <input
              :type="inputType"
              :value="matrix[row]?.[col] ?? ''"
              class="w-14 bg-gray-800 border border-gray-700 text-gray-200 rounded px-1 py-0 text-xs text-center focus:outline-none focus:border-gray-500"
              @input="updateCell(row, col, ($event.target as HTMLInputElement).value)"
            />
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
