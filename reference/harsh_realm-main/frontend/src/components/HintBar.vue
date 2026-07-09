<script setup lang="ts">
import { useGameStore } from "../stores/game";

const gameStore = useGameStore();
</script>

<template>
  <div
    v-if="gameStore.currentSuggestions.length"
    class="border-t border-gray-800 px-4 py-1.5 flex items-center gap-x-1.5 gap-y-1 flex-wrap
           bg-gray-950/60 min-h-[2rem] shrink-0"
  >
    <span class="text-gray-600 text-[10px] select-none font-mono shrink-0">hints:</span>
    <button
      v-for="cmd in gameStore.currentSuggestions"
      :key="cmd"
      @click="gameStore.setSuggestInput(cmd)"
      class="text-gray-500 hover:text-cyan-400 text-[11px] font-mono transition-colors
             cursor-pointer border border-gray-800 hover:border-cyan-800 rounded
             px-1.5 py-0.5 leading-none select-none"
    >{{ cmd }}</button>
  </div>
</template>
