<script setup lang="ts">
import type { CombatActionButton } from "../types/api";

/**
 * Presentational action-bar component shared by exploration (tile actions) and
 * potentially other scenes. Renders a horizontal row of styled buttons; clicking
 * any button emits `action` with the button's raw command string.
 */
const props = defineProps<{ actions: CombatActionButton[] }>();
const emit = defineEmits<{ (e: "action", command: string): void }>();

/** Tailwind classes for each button style variant. */
function btnClass(style: string): string {
  if (style === "primary") {
    return "bg-amber-600 hover:bg-amber-500 text-white border border-amber-400";
  }
  if (style === "danger") {
    return "bg-red-700 hover:bg-red-600 text-white border border-red-500";
  }
  // default
  return "bg-gray-700 hover:bg-gray-600 text-gray-100 border border-gray-500";
}
</script>

<template>
  <div
    class="flex flex-wrap gap-1 px-2 py-1.5 bg-gray-900 border-t border-gray-700 shrink-0"
    data-testid="action-bar"
  >
    <button
      v-for="a in props.actions"
      :key="a.command"
      :data-testid="`action-btn-${a.command.replace(/\s+/g, '-')}`"
      class="px-2.5 py-1 text-xs font-mono rounded transition-colors"
      :class="btnClass(a.style)"
      @click="emit('action', a.command)"
    >
      {{ a.label }}
    </button>
  </div>
</template>
