<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useGameStore } from "../stores/game";

const props = defineProps<{
  send: (text: string) => void;
}>();

const gameStore = useGameStore();

const ATTRS = ["str", "dex", "con", "int", "wis", "cha"] as const;
type Attr = typeof ATTRS[number];

// Each slot holds one score (or null if unfilled).
const slots = ref<Record<Attr, number | null>>({
  str: null, dex: null, con: null, int: null, wis: null, cha: null,
});

// Scores currently in the pool (not yet assigned to a slot).
const pool = ref<number[]>([]);

// Drag-tracking — holds the value + whether it came from pool or a slot.
const dragging = ref<{ value: number; source: "pool" | Attr } | null>(null);

// Open when the GM has given us 6 rolled scores. Clears after confirm.
const open = computed(() => gameStore.attributeRoll != null);

watch(
  () => gameStore.attributeRoll,
  (scores) => {
    if (scores && scores.length === 6) {
      pool.value = [...scores];
      slots.value = { str: null, dex: null, con: null, int: null, wis: null, cha: null };
    }
  },
  { immediate: true },
);

function modifier(score: number | null): string {
  if (score == null) return "";
  if (score <= 3) return "-2";
  if (score <= 7) return "-1";
  if (score <= 13) return "+0";
  if (score <= 17) return "+1";
  return "+2";
}

function startDrag(e: DragEvent, value: number, source: "pool" | Attr) {
  dragging.value = { value, source };
  e.dataTransfer?.setData("text/plain", String(value));
  if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
}

function onDragOver(e: DragEvent) {
  e.preventDefault();
  if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
}

function dropOnSlot(attr: Attr) {
  const d = dragging.value;
  if (!d) return;
  const prev = slots.value[attr];
  if (d.source === "pool") {
    // Remove from pool; swap with any existing slot occupant.
    const idx = pool.value.indexOf(d.value);
    if (idx >= 0) pool.value.splice(idx, 1);
    slots.value[attr] = d.value;
    if (prev != null) pool.value.push(prev);
  } else if (d.source !== attr) {
    // Slot → slot swap
    slots.value[d.source] = prev;
    slots.value[attr] = d.value;
  }
  dragging.value = null;
}

function dropOnPool() {
  const d = dragging.value;
  if (!d) return;
  if (d.source !== "pool") {
    slots.value[d.source] = null;
    pool.value.push(d.value);
  }
  dragging.value = null;
}

function clearSlot(attr: Attr) {
  const v = slots.value[attr];
  if (v != null) {
    slots.value[attr] = null;
    pool.value.push(v);
  }
}

function autoFill() {
  // Assign remaining pool scores to unfilled slots in display order.
  const remaining = [...pool.value];
  for (const a of ATTRS) {
    if (slots.value[a] == null && remaining.length > 0) {
      slots.value[a] = remaining.shift()!;
    }
  }
  pool.value = remaining;
}

const allAssigned = computed(() => ATTRS.every((a) => slots.value[a] != null));

async function confirm() {
  if (!allAssigned.value) return;
  // Send one score per attribute in the canonical order, then "done" to pass
  // the review step. The backend accepts input one command at a time, so
  // quick-fire sends are fine — each only blocks on the previous event loop.
  for (const a of ATTRS) {
    const v = slots.value[a];
    if (v == null) return;
    props.send(String(v));
  }
  props.send("done");
  gameStore.setAttributeRoll(null);
}

function cancel() {
  // Fall back to text-driven assignment: clear the modal, user types scores.
  gameStore.setAttributeRoll(null);
}
</script>

<template>
  <div
    v-if="open"
    class="fixed inset-0 bg-black/70 flex items-center justify-center z-50"
  >
    <div class="bg-gray-900 border border-gray-700 rounded-lg shadow-xl p-6 w-[36rem] max-w-[95vw]">
      <h2 class="text-sm font-bold text-amber-200 uppercase tracking-wide mb-1">
        Assign Attributes
      </h2>
      <p class="text-xs text-gray-500 mb-4">
        Drag a score onto an attribute slot. Drag between slots to swap, or
        drag back to the pool to unassign.
      </p>

      <!-- Pool -->
      <div
        class="mb-4 p-3 rounded bg-gray-950 border border-dashed border-gray-700 min-h-[3.5rem] flex items-center gap-2 flex-wrap"
        @dragover="onDragOver"
        @drop="dropOnPool"
      >
        <span class="text-xs text-gray-500 mr-2">Pool:</span>
        <div
          v-for="(score, idx) in pool"
          :key="idx + '-' + score"
          draggable="true"
          class="select-none cursor-grab active:cursor-grabbing bg-amber-900/40 border border-amber-700 text-amber-100 font-mono text-sm rounded px-3 py-1"
          @dragstart="startDrag($event, score, 'pool')"
        >{{ score }}</div>
        <span v-if="pool.length === 0" class="text-xs text-gray-600 italic">
          (empty — all scores assigned)
        </span>
      </div>

      <!-- Slots -->
      <div class="grid grid-cols-2 gap-2 mb-4">
        <div
          v-for="attr in ATTRS"
          :key="attr"
          class="flex items-center gap-2 bg-gray-950 border border-gray-800 rounded p-2"
          @dragover="onDragOver"
          @drop="dropOnSlot(attr)"
        >
          <span class="text-xs text-gray-500 w-10 uppercase">{{ attr }}</span>
          <div
            v-if="slots[attr] != null"
            draggable="true"
            class="select-none cursor-grab active:cursor-grabbing bg-amber-900/40 border border-amber-700 text-amber-100 font-mono text-sm rounded px-3 py-1"
            @dragstart="startDrag($event, slots[attr]!, attr)"
          >{{ slots[attr] }}</div>
          <div
            v-else
            class="text-xs text-gray-600 italic border border-dashed border-gray-700 rounded px-3 py-1"
          >drop here</div>
          <span v-if="slots[attr] != null" class="text-xs text-gray-500 ml-auto">
            mod {{ modifier(slots[attr]) }}
          </span>
          <button
            v-if="slots[attr] != null"
            class="text-xs text-gray-500 hover:text-red-400 ml-1"
            @click="clearSlot(attr)"
            title="Return to pool"
          >×</button>
        </div>
      </div>

      <div class="flex items-center justify-end gap-2">
        <button
          class="px-3 py-1 text-xs rounded border border-gray-600 text-gray-400 hover:text-gray-200"
          @click="autoFill"
          :disabled="pool.length === 0"
        >Auto-fill</button>
        <button
          class="px-3 py-1 text-xs rounded border border-gray-600 text-gray-500 hover:text-gray-300"
          @click="cancel"
        >Type instead</button>
        <button
          class="px-3 py-1 text-xs rounded border border-green-700 text-green-400 hover:bg-green-900/30 disabled:opacity-40 disabled:cursor-not-allowed"
          :disabled="!allAssigned"
          @click="confirm"
        >Confirm</button>
      </div>
    </div>
  </div>
</template>
