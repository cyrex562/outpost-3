<script setup lang="ts">
import { ref, watch, nextTick } from "vue";
import { useGameStore } from "../stores/game";

const gameStore = useGameStore();

const logEl = ref<HTMLElement | null>(null);
const isAtBottom = ref(true);

watch(
  () => gameStore.messages.length,
  async () => {
    await nextTick();
    // Only auto-scroll if already at bottom
    if (isAtBottom.value && logEl.value) {
      logEl.value.scrollTop = logEl.value.scrollHeight;
    }
  },
);

function onScroll() {
  if (!logEl.value) return;
  const { scrollTop, scrollHeight, clientHeight } = logEl.value;
  isAtBottom.value = scrollHeight - scrollTop - clientHeight < 40;
}

function scrollToBottom() {
  if (logEl.value) {
    logEl.value.scrollTop = logEl.value.scrollHeight;
    isAtBottom.value = true;
  }
}
</script>

<template>
  <div class="relative flex-1 min-h-0">
    <div
      ref="logEl"
      class="h-full overflow-y-auto px-4 py-3 space-y-1 font-mono text-sm"
      @scroll="onScroll"
    >
      <div
        v-for="msg in gameStore.messages"
        :key="msg.id"
        class="leading-relaxed"
      >
        <!-- Player input: green, prefixed with > -->
        <span v-if="msg.sender === 'you'" class="text-green-400">
          &gt; {{ msg.text }}
        </span>

        <!-- GM narration: amber/warm white, no prefix -->
        <span v-else-if="msg.type === 'narration'" class="text-amber-100">
          {{ msg.text }}
        </span>

        <!-- Skill check: blue accent -->
        <span
          v-else-if="msg.type === 'skill_check'"
          class="block text-xs border-l-2 border-sky-600 pl-2 text-sky-300 whitespace-pre-line"
        >
          {{ msg.text }}
        </span>

        <!-- Disposition change: green/red accent -->
        <span
          v-else-if="msg.type === 'disposition'"
          class="block text-xs border-l-2 pl-2 whitespace-pre-line"
          :class="msg.text.includes('(+') ? 'border-emerald-600 text-emerald-300' : 'border-red-600 text-red-300'"
        >
          {{ msg.text }}
        </span>

        <!-- Expert reroll: purple accent -->
        <span
          v-else-if="msg.type === 'reroll'"
          class="block text-xs border-l-2 border-purple-600 pl-2 text-purple-300 whitespace-pre-line"
        >
          {{ msg.text }}
        </span>

        <!-- Purchase: amber accent -->
        <span
          v-else-if="msg.type === 'purchase'"
          class="block text-xs border-l-2 border-amber-600 pl-2 text-amber-300"
        >
          {{ msg.text }}
        </span>

        <!-- Sale: amber accent -->
        <span
          v-else-if="msg.type === 'sale'"
          class="block text-xs border-l-2 border-amber-600 pl-2 text-amber-200"
        >
          {{ msg.text }}
        </span>

        <!-- Faction event: muted grey italic -->
        <span
          v-else-if="msg.type === 'faction_event'"
          class="block text-xs text-gray-500 italic"
        >
          {{ msg.text }}
        </span>

        <!-- Reputation change: orange accent -->
        <span
          v-else-if="msg.type === 'reputation'"
          class="block text-xs border-l-2 border-orange-600 pl-2 text-orange-300"
        >
          {{ msg.text }}
        </span>

        <!-- Chaos change: subtle -->
        <span
          v-else-if="msg.type === 'chaos'"
          class="block text-xs text-gray-500 italic"
        >
          {{ msg.text }}
        </span>

        <!-- Combat attack: red accent -->
        <span
          v-else-if="msg.type === 'combat_attack'"
          class="block text-xs border-l-2 border-red-600 pl-2 text-red-200 whitespace-pre-line"
        >
          {{ msg.text }}
        </span>

        <!-- Combat save: orange accent -->
        <span
          v-else-if="msg.type === 'combat_save'"
          class="block text-xs border-l-2 border-orange-600 pl-2 text-orange-200 whitespace-pre-line"
        >
          {{ msg.text }}
        </span>

        <!-- Inventory change: subtle green gain accent -->
        <span
          v-else-if="msg.type === 'inventory'"
          class="block text-xs border-l-2 border-emerald-700 pl-2 text-emerald-300"
        >
          {{ msg.text }}
        </span>

        <!-- Character death: prominent bold red with background (HR-780) -->
        <span
          v-else-if="msg.type === 'death'"
          class="block border-l-4 border-red-500 pl-3 py-1 bg-red-950/40 text-red-400 font-bold text-sm tracking-wide"
        >
          {{ msg.text }}
        </span>

        <!-- System / game events: dim grey, italic -->
        <span v-else class="text-gray-500 italic text-xs">
          {{ msg.text }}
        </span>
      </div>

      <div v-if="gameStore.messages.length === 0" class="text-gray-600 italic">
        Awaiting connection&hellip;
      </div>
    </div>

    <!-- Scroll-to-bottom button, shown when user has scrolled up -->
    <button
      v-if="!isAtBottom"
      @click="scrollToBottom"
      class="absolute bottom-2 right-3 bg-gray-800 border border-gray-600 rounded-full
             px-2 py-1 text-xs text-gray-400 hover:text-gray-200 hover:border-gray-400
             transition-colors shadow"
    >
      &darr; latest
    </button>
  </div>
</template>
