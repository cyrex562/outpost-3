<script setup lang="ts">
import { ref, computed } from "vue";
import {
  ATTRS,
  type Attr,
  useCharacterCreationStore,
} from "../stores/characterCreation";
import type { AttributeMethod } from "../types/api";

const store = useCharacterCreationStore();

const METHOD_LABELS: Record<AttributeMethod, string> = {
  roll: "Roll (4d6)",
  standard_array: "Standard array",
  point_buy: "Point buy",
};

/**
 * Chips still in the pool — sourcePool minus already-assigned values.
 * Uses indexOf so duplicate values (e.g. two 14s) are consumed one at a time.
 */
const poolChips = computed<number[]>(() => {
  const pool = [...store.sourcePool];
  for (const a of ATTRS) {
    const v = store.attrValues[a];
    if (v != null) {
      const idx = pool.indexOf(v);
      if (idx >= 0) pool.splice(idx, 1);
    }
  }
  return pool;
});

// --- drag state ---
const dragging = ref<{ value: number; source: "pool" | Attr } | null>(null);

// --- click-to-assign state: index in poolChips of selected chip ---
const selectedChipIdx = ref<number | null>(null);

function sign(n: number): string {
  return n >= 0 ? `+${n}` : `${n}`;
}

function modClass(mod: number): string {
  if (mod >= 2) return "text-amber-300 font-bold text-base";
  if (mod >= 1) return "text-amber-200 font-semibold text-base";
  if (mod <= -2) return "text-red-400 font-semibold text-base";
  if (mod <= -1) return "text-orange-400 font-semibold text-base";
  return "text-gray-500 text-sm";
}

// --- drag handlers ---
function startDrag(e: DragEvent, value: number, source: "pool" | Attr): void {
  dragging.value = { value, source };
  e.dataTransfer?.setData("text/plain", String(value));
  if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
  selectedChipIdx.value = null;
}

/** Called from slot v-if-guarded area; safe because caller checks attrValues[attr] != null */
function startSlotDrag(e: DragEvent, attr: Attr): void {
  const v = store.attrValues[attr];
  if (v != null) startDrag(e, v, attr);
}

function onDragOver(e: DragEvent): void {
  e.preventDefault();
  if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
}

function dropOnSlot(attr: Attr): void {
  const d = dragging.value;
  if (!d) return;
  const prev = store.attrValues[attr];

  if (d.source === "pool") {
    // Assign chip to slot; prev (if any) returns to pool automatically
    // because poolChips is recomputed from attrValues.
    store.assignAttr(attr, d.value);
  } else if (d.source !== attr) {
    // Slot-to-slot swap: put prev value into source slot.
    store.assignAttr(d.source, prev);
    store.assignAttr(attr, d.value);
  }
  dragging.value = null;
  selectedChipIdx.value = null;
}

function dropOnPool(): void {
  const d = dragging.value;
  if (!d) return;
  if (d.source !== "pool") {
    store.assignAttr(d.source, null);
  }
  dragging.value = null;
  selectedChipIdx.value = null;
}

// --- click-to-assign handlers ---
function onChipClick(idx: number): void {
  selectedChipIdx.value = selectedChipIdx.value === idx ? null : idx;
}

function onSlotClick(attr: Attr): void {
  if (selectedChipIdx.value !== null) {
    const chip = poolChips.value[selectedChipIdx.value];
    if (chip !== undefined) {
      store.assignAttr(attr, chip);
    }
    selectedChipIdx.value = null;
  }
}

function clearSlot(attr: Attr): void {
  store.assignAttr(attr, null);
  selectedChipIdx.value = null;
}

function autoAssign(): void {
  const chips = [...poolChips.value];
  for (const a of ATTRS) {
    if (store.attrValues[a] == null) {
      const chip = chips.shift();
      // shift() is safe: the loop condition ensures chips has an element here
      if (chip === undefined) break;
      store.assignAttr(a, chip);
    }
  }
  selectedChipIdx.value = null;
}
</script>

<template>
  <!-- Method selector -->
  <div class="flex items-center justify-between mb-3">
    <h2 class="text-xs uppercase text-gray-500 tracking-wide">Attributes</h2>
    <div class="flex gap-1" v-if="store.options">
      <button
        v-for="m in store.options.methods"
        :key="m"
        type="button"
        class="px-2 py-1 text-[11px] rounded border transition-colors"
        :class="
          store.method === m
            ? 'border-amber-600 text-amber-200 bg-amber-900/20'
            : 'border-gray-700 text-gray-400 hover:border-gray-500'
        "
        :data-testid="`cc-method-${m}`"
        @click="store.setMethod(m)"
      >{{ METHOD_LABELS[m] }}</button>
    </div>
  </div>

  <!-- ── POINT BUY ─────────────────────────────────── -->
  <template v-if="store.method === 'point_buy' && store.options">
    <div class="text-[11px] text-gray-400 mb-3">
      Points remaining:
      <span
        class="font-mono"
        :class="store.pointBuyRemaining < 0 ? 'text-red-400' : 'text-amber-200'"
        data-testid="cc-pb-remaining"
      >{{ store.pointBuyRemaining }}</span>
      / {{ store.options.point_buy.budget }}
    </div>

    <div class="grid grid-cols-2 sm:grid-cols-3 gap-2">
      <div
        v-for="attr in ATTRS"
        :key="attr"
        class="flex items-center gap-2 bg-gray-900 border border-gray-800 rounded p-2"
      >
        <span class="text-xs uppercase text-gray-500 w-9">{{ attr }}</span>
        <button
          type="button"
          class="w-6 h-6 rounded border border-gray-600 text-gray-300 hover:border-amber-600 leading-none"
          :data-testid="`cc-pb-dec-${attr}`"
          @click="store.bumpPointBuy(attr, -1)"
        >−</button>
        <span
          class="font-mono text-sm w-6 text-center text-amber-100"
          :data-testid="`cc-attr-val-${attr}`"
        >{{ store.attrValues[attr] }}</span>
        <button
          type="button"
          class="w-6 h-6 rounded border border-gray-600 text-gray-300 hover:border-amber-600 leading-none"
          :data-testid="`cc-pb-inc-${attr}`"
          @click="store.bumpPointBuy(attr, 1)"
        >+</button>
        <span class="ml-auto" :class="modClass(store.attrMods[attr])">
          {{ sign(store.attrMods[attr]) }}
        </span>
      </div>
    </div>
  </template>

  <!-- ── POOL / DRAG MODE (roll + standard_array) ──── -->
  <template v-else-if="store.method !== 'point_buy'">
    <!-- Pool drop zone -->
    <div
      class="mb-3 p-3 rounded bg-gray-950 border border-dashed border-gray-700 min-h-[3.5rem] flex items-center gap-2 flex-wrap"
      @dragover="onDragOver"
      @drop="dropOnPool"
    >
      <span class="text-xs text-gray-500 mr-1">Pool:</span>
      <button
        v-for="(score, idx) in poolChips"
        :key="`${idx}-${score}`"
        type="button"
        draggable="true"
        class="select-none cursor-grab active:cursor-grabbing font-mono text-sm rounded px-3 py-1 border transition-colors"
        :class="
          selectedChipIdx === idx
            ? 'bg-amber-700/60 border-amber-400 text-white ring-2 ring-amber-400'
            : 'bg-amber-900/40 border-amber-700 text-amber-100 hover:bg-amber-800/50'
        "
        :data-testid="`cc-pool-chip-${idx}`"
        @click="onChipClick(idx)"
        @dragstart="startDrag($event, score, 'pool')"
      >{{ score }}</button>
      <span v-if="poolChips.length === 0" class="text-xs text-gray-600 italic">
        All scores assigned
      </span>
    </div>

    <!-- Controls row -->
    <div class="flex items-center gap-2 mb-3">
      <button
        v-if="store.method === 'roll'"
        type="button"
        class="px-2 py-0.5 text-[11px] rounded border border-gray-600 text-gray-400 hover:text-amber-200 hover:border-amber-700 transition-colors"
        data-testid="cc-roll-again"
        @click="store.rollAttributes()"
      >Roll again</button>
      <button
        type="button"
        class="px-2 py-0.5 text-[11px] rounded border border-gray-600 text-gray-400 hover:text-amber-200 hover:border-amber-700 transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
        :disabled="poolChips.length === 0"
        data-testid="cc-auto-assign"
        @click="autoAssign"
      >Auto-assign</button>
      <span class="text-[11px] text-gray-600">· click a chip, then a slot</span>
    </div>

    <!-- Attribute slots -->
    <div class="grid grid-cols-2 sm:grid-cols-3 gap-2">
      <div
        v-for="attr in ATTRS"
        :key="attr"
        class="relative bg-gray-900 rounded p-2 cursor-pointer transition-colors select-none"
        :class="[
          selectedChipIdx !== null && store.attrValues[attr] == null
            ? 'border border-amber-600/60 bg-amber-900/10 hover:border-amber-500'
            : store.attrValues[attr] != null
              ? 'border border-gray-700 hover:border-gray-500'
              : 'border border-dashed border-gray-700 hover:border-gray-600'
        ]"
        :data-testid="`cc-attr-slot-${attr}`"
        @dragover="onDragOver"
        @drop="dropOnSlot(attr)"
        @click="onSlotClick(attr)"
      >
        <span class="text-[10px] uppercase text-gray-500 tracking-wider block mb-0.5">{{ attr }}</span>

        <div class="flex items-baseline gap-1.5">
          <!-- Assigned chip (draggable back to pool or to another slot) -->
          <div
            v-if="store.attrValues[attr] != null"
            draggable="true"
            class="font-mono text-xl font-bold text-amber-100 cursor-grab active:cursor-grabbing"
            @dragstart="startSlotDrag($event, attr)"
          >{{ store.attrValues[attr] }}</div>
          <!-- Empty placeholder -->
          <div v-else class="text-xs text-gray-600 italic leading-tight">
            {{ selectedChipIdx !== null ? "← assign here" : "drag or click" }}
          </div>

          <!-- Modifier — prominent, right-aligned -->
          <span
            v-if="store.attrValues[attr] != null"
            class="ml-auto"
            :class="modClass(store.attrMods[attr])"
          >{{ sign(store.attrMods[attr]) }}</span>
        </div>

        <!-- Clear button (returns to pool) -->
        <button
          v-if="store.attrValues[attr] != null"
          type="button"
          class="absolute top-1 right-1 text-[10px] text-gray-600 hover:text-red-400 leading-none"
          title="Return to pool"
          @click.stop="clearSlot(attr)"
        >×</button>
      </div>
    </div>
  </template>
</template>
