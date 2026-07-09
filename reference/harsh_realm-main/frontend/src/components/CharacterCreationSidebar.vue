<script setup lang="ts">
import { ATTRS, type Attr, useCharacterCreationStore } from "../stores/characterCreation";

const store = useCharacterCreationStore();

function sign(n: number): string {
  return n >= 0 ? `+${n}` : `${n}`;
}

function modColour(attr: Attr): string {
  const m = store.attrMods[attr];
  if (m >= 2) return "text-amber-300";
  if (m >= 1) return "text-amber-200";
  if (m <= -1) return "text-red-400";
  return "text-gray-500";
}
</script>

<template>
  <aside
    class="flex flex-col gap-4 p-4 h-full overflow-y-auto"
    data-testid="cc-summary"
  >
    <h2 class="text-xs uppercase text-gray-500 tracking-wide">Summary</h2>

    <!-- Selected class info -->
    <div class="rounded bg-gray-900 border border-gray-800 p-3">
      <h3 class="text-xs uppercase text-gray-600 mb-2">Class</h3>
      <template v-if="store.selectedClass">
        <p class="text-sm font-semibold text-amber-100">{{ store.selectedClass.name }}</p>
        <p class="text-xs text-gray-400 mt-1 leading-snug">{{ store.selectedClass.description }}</p>
        <p class="text-xs text-gray-600 mt-2">
          HD d{{ store.selectedClass.hit_die }} · {{ store.selectedClass.skill_points_start }} skill pts
        </p>
      </template>
      <p v-else class="text-xs text-gray-600 italic">No class selected</p>
    </div>

    <!-- Derived stats -->
    <div class="rounded bg-gray-900 border border-gray-800 p-3">
      <h3 class="text-xs uppercase text-gray-600 mb-2">Derived Stats</h3>
      <div class="space-y-1.5">
        <div class="flex justify-between text-xs">
          <span class="text-gray-500">HP</span>
          <span class="font-mono text-amber-100" data-testid="cc-preview-hp">
            {{ store.previewHp ? `${store.previewHp.min}–${store.previewHp.max}` : "—" }}
          </span>
        </div>
        <div class="flex justify-between text-xs">
          <span class="text-gray-500">AC</span>
          <span class="font-mono text-amber-100" data-testid="cc-preview-ac">{{ store.previewAc }}</span>
        </div>
        <div class="pt-1.5 border-t border-gray-800">
          <p class="text-[10px] uppercase text-gray-600 mb-1.5">Saves</p>
          <div class="flex justify-between text-xs">
            <span class="text-gray-500">Physical</span>
            <span class="font-mono text-amber-100">{{ store.previewSaves.physical }}</span>
          </div>
          <div class="flex justify-between text-xs">
            <span class="text-gray-500">Evasion</span>
            <span class="font-mono text-amber-100">{{ store.previewSaves.evasion }}</span>
          </div>
          <div class="flex justify-between text-xs">
            <span class="text-gray-500">Mental</span>
            <span class="font-mono text-amber-100">{{ store.previewSaves.mental }}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Attribute modifier grid (shown once all are assigned) -->
    <div
      v-if="store.attributesComplete"
      class="rounded bg-gray-900 border border-gray-800 p-3"
    >
      <h3 class="text-xs uppercase text-gray-600 mb-2">Modifiers</h3>
      <div class="grid grid-cols-3 gap-1">
        <div
          v-for="attr in ATTRS"
          :key="attr"
          class="text-center py-1"
        >
          <p class="text-[10px] uppercase text-gray-600">{{ attr }}</p>
          <p class="font-mono text-sm font-semibold" :class="modColour(attr)">
            {{ sign(store.attrMods[attr]) }}
          </p>
        </div>
      </div>
    </div>

    <!-- Validation hint -->
    <p v-if="store.validationError" class="text-xs text-gray-500 italic">
      {{ store.validationError }}
    </p>
  </aside>
</template>
