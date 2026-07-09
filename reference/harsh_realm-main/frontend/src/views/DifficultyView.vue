<script setup lang="ts">
import { onMounted } from "vue";
import { RouterLink } from "vue-router";
import { storeToRefs } from "pinia";
import { useDifficultyStore } from "../stores/difficulty";

const store = useDifficultyStore();
const { dimensions, loading, error } = storeToRefs(store);

onMounted(() => {
  void store.load();
});

function onSlide(key: string, event: Event): void {
  const target = event.target as HTMLInputElement;
  void store.setGrade(key, Number(target.value));
}
</script>

<template>
  <div
    data-testid="difficulty-view"
    class="min-h-screen bg-gray-950 text-gray-200 font-mono p-6"
  >
    <div class="max-w-3xl mx-auto">
      <div class="flex items-center justify-between mb-6">
        <h1 class="text-xl font-bold text-green-400">Difficulty</h1>
        <RouterLink
          to="/"
          data-testid="difficulty-back"
          class="px-2 py-1 text-xs rounded border border-gray-600 text-gray-400 hover:border-gray-400 hover:text-gray-200 transition-colors select-none"
        >
          ← Back to Game
        </RouterLink>
      </div>

      <p class="text-xs text-gray-500 mb-6">
        Adjust each setting from easier (left) to harder (right). Changes apply
        to this world and take effect on your next encounter. Player HP updates
        immediately.
      </p>

      <div v-if="loading" data-testid="difficulty-loading" class="text-gray-500">
        Loading…
      </div>
      <div v-if="error" data-testid="difficulty-error" class="text-red-400 mb-4">
        {{ error }}
      </div>

      <ul class="space-y-6">
        <li
          v-for="dim in dimensions"
          :key="dim.key"
          :data-testid="`difficulty-row-${dim.key}`"
          class="border border-gray-800 rounded p-4 bg-gray-900/40"
        >
          <div class="flex items-baseline justify-between mb-1">
            <span class="text-sm font-semibold text-gray-100">{{ dim.label }}</span>
            <span
              :data-testid="`difficulty-effect-${dim.key}`"
              class="text-sm text-green-400 tabular-nums"
              >{{ dim.current_effect }}</span
            >
          </div>
          <p class="text-xs text-gray-500 mb-3">{{ dim.description }}</p>

          <input
            type="range"
            min="0"
            :max="dim.grade_count - 1"
            step="1"
            :value="dim.grade"
            :data-testid="`difficulty-slider-${dim.key}`"
            class="w-full accent-green-500"
            @change="onSlide(dim.key, $event)"
          />

          <div class="flex justify-between text-[10px] text-gray-500 mt-1">
            <span>{{ dim.easy_label }}</span>
            <span
              :class="
                dim.grade === dim.normal_grade ? 'text-gray-300' : 'text-gray-600'
              "
              >Normal</span
            >
            <span>{{ dim.hard_label }}</span>
          </div>
        </li>
      </ul>
    </div>
  </div>
</template>
