<script setup lang="ts">
import { ref, watch } from "vue";
import { useGameStore } from "../stores/game";

const props = defineProps<{
  disabled?: boolean;
}>();

const emit = defineEmits<{
  send: [text: string];
}>();

const gameStore = useGameStore();
const input = ref("");
const history = ref<string[]>([]); // last 50 commands, oldest first
const historyIndex = ref(-1); // -1 = not browsing history
const tabIndex = ref(-1); // tracks Tab cycling position
const tabPrefix = ref(""); // the text when Tab was first pressed

// Fill the input when a suggestion chip is clicked
watch(() => gameStore.pendingInput, (cmd) => {
  if (cmd !== null) {
    input.value = cmd;
    gameStore.pendingInput = null;
    // Focus input so the user can edit or just hit Enter
    const el = document.querySelector<HTMLInputElement>("input[type=text]");
    el?.focus();
  }
});

function submit() {
  if (props.disabled) return;
  const text = input.value.trim();
  // Push non-empty input to history (avoid consecutive duplicates)
  if (text && history.value[history.value.length - 1] !== text) {
    history.value.push(text);
    if (history.value.length > 50) history.value.shift();
  }
  historyIndex.value = -1;
  emit("send", text);
  input.value = "";
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "ArrowUp") {
    e.preventDefault();
    if (history.value.length === 0) return;
    const newIndex =
      historyIndex.value === -1
        ? history.value.length - 1
        : Math.max(0, historyIndex.value - 1);
    historyIndex.value = newIndex;
    input.value = history.value[newIndex];
  } else if (e.key === "ArrowDown") {
    e.preventDefault();
    if (historyIndex.value === -1) return;
    const newIndex = historyIndex.value + 1;
    if (newIndex >= history.value.length) {
      historyIndex.value = -1;
      input.value = "";
    } else {
      historyIndex.value = newIndex;
      input.value = history.value[newIndex];
    }
  } else if (e.key === "Tab") {
    e.preventDefault();
    autocomplete(e.shiftKey);
    return;
  } else if (e.key === "Enter") {
    submit();
  }
  // Any non-Tab key resets the Tab cycling state
  if (e.key !== "Tab") {
    tabIndex.value = -1;
  }
}

function autocomplete(reverse: boolean) {
  const suggestions = gameStore.currentSuggestions;
  if (suggestions.length === 0) return;

  // On first Tab press, capture the current prefix
  if (tabIndex.value === -1) {
    tabPrefix.value = input.value.toLowerCase();
  }

  // Filter suggestions that start with the prefix (or all if prefix is empty)
  const matches = tabPrefix.value
    ? suggestions.filter(s => s.toLowerCase().startsWith(tabPrefix.value))
    : suggestions;

  if (matches.length === 0) return;

  // Cycle through matches
  if (reverse) {
    tabIndex.value = tabIndex.value <= 0 ? matches.length - 1 : tabIndex.value - 1;
  } else {
    tabIndex.value = (tabIndex.value + 1) % matches.length;
  }

  input.value = matches[tabIndex.value];
}
</script>

<template>
  <div
    class="border-t border-gray-700 px-4 py-3 flex items-center gap-2"
    :class="disabled ? 'opacity-40' : ''"
  >
    <span class="text-green-500 font-mono font-bold select-none">&rsaquo;</span>
    <input
      v-model="input"
      type="text"
      :placeholder="disabled ? 'Load a world to begin\u2026' : 'Enter command\u2026'"
      :disabled="disabled"
      class="flex-1 bg-transparent text-gray-100 font-mono text-sm placeholder-gray-600
             outline-none focus:outline-none disabled:cursor-not-allowed"
      @keydown="onKeydown"
      :autofocus="!disabled"
    />
  </div>
</template>
