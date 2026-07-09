<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount } from "vue";
import { useEncounterStore } from "../stores/encounter";
import { useGameStore, type EquipmentItem } from "../stores/game";
import { useWebSocket } from "../composables/useWebSocket";
import type { PositionsEntity, CombatActionButton } from "../types/api";
import {
  LOOT_COLORS,
  lootGemPoints,
  lootTier,
  type LootTier,
} from "../utils/mapGraphics";

const encounterStore = useEncounterStore();
const gameStore = useGameStore();
const { send } = useWebSocket();

const CELL_SIZE = 24;

// Terrain fill colours for encounter battle cells
const TERRAIN_COLORS: Record<string, string> = {
  // World terrains
  forest: "#2a4a2a",
  grassland: "#3a5a3a",
  plains: "#4a5a3a",
  hills: "#4a4a3a",
  mountains: "#5a5a5a",
  desert: "#8a7a4a",
  wasteland: "#5a4a3a",
  ruins: "#4a3a3a",
  swamp: "#2a4a3a",
  coast: "#3a5a6a",
  // Town terrains (encounter can start in towns)
  road: "#6a6a5a",
  plaza: "#7a7a6a",
  open_ground: "#3a4a3a",
  building: "#4a3535",
};

// Feature stamp labels/colours for key terrain cells
const FEATURE_MARKERS: Record<string, { label: string; color: string }> = {
  rubble: { label: "~", color: "#9a8a7a" },
  wall: { label: "#", color: "#8a8a9a" },
  water: { label: "≈", color: "#3a6a8a" },
  fire: { label: "^", color: "#e06020" },
  obstacle: { label: "X", color: "#aa8888" },
  cover: { label: "C", color: "#888877" },
  difficult: { label: ".", color: "#777766" },
};

const PC_FILL = "#f0c040";
const PC_STROKE = "#c08a00";

// Token fill colour based on kind and HP ratio
function tokenFill(entity: PositionsEntity): string {
  if (!entity.alive) return "#444";
  if (entity.kind === "pc") return PC_FILL;
  if (entity.max_hp <= 0) return "#cc4444";
  const ratio = entity.hp / entity.max_hp;
  if (ratio >= 1.0) return "#44bb44";
  if (ratio >= 0.75) return "#bbcc22";
  if (ratio >= 0.5) return "#dd8822";
  if (ratio >= 0.25) return "#cc3333";
  return "#882222";
}

function tokenStroke(entity: PositionsEntity): string {
  if (entity.kind === "pc") return PC_STROKE;
  return "#111";
}

// HP bar fill percentage (0–100)
function hpBarPct(entity: PositionsEntity): number {
  if (entity.max_hp <= 0 || !entity.alive) return 0;
  return Math.max(0, Math.min(100, (entity.hp / entity.max_hp) * 100));
}

function cellToPixel(q: number, r: number): { x: number; y: number } {
  return { x: q * CELL_SIZE, y: r * CELL_SIZE };
}

// HR-787: rarity-tiered loot piles dropped by defeated enemies on the grid.
interface EncounterLootRender {
  key: string;
  q: number;
  r: number;
  x: number;
  y: number;
  tier: LootTier;
}
const lootRenders = computed<EncounterLootRender[]>(() =>
  encounterStore.loot.map((l) => {
    const { x, y } = cellToPixel(l.q, l.r);
    return { key: `${l.q},${l.r}`, q: l.q, r: l.r, x, y, tier: lootTier(l.value) };
  }),
);

// ViewBox state for pan/zoom
const viewBox = ref({ x: -12, y: -12, w: 240, h: 240 });

let isDragging = false;
let dragStartX = 0;
let dragStartY = 0;
let dragStartVbX = 0;
let dragStartVbY = 0;

function centerOnPc(): void {
  const pc = encounterStore.entities.find((e) => e.kind === "pc");
  const target = pc ?? { q: 4, r: 4 }; // default: centre of a 9×9 grid
  const { x, y } = cellToPixel(target.q, target.r);
  viewBox.value = {
    x: x - viewBox.value.w / 2 + CELL_SIZE / 2,
    y: y - viewBox.value.h / 2 + CELL_SIZE / 2,
    w: viewBox.value.w,
    h: viewBox.value.h,
  };
}

// Re-centre whenever the battle grid is first populated
watch(
  () => encounterStore.entities.length,
  (len) => {
    if (len > 0) centerOnPc();
  },
);

function onWheel(e: WheelEvent): void {
  e.preventDefault();
  const zoomFactor = e.deltaY > 0 ? 1.15 : 0.87;
  const newW = viewBox.value.w * zoomFactor;
  const newH = viewBox.value.h * zoomFactor;
  const svgEl = (e.currentTarget as SVGElement).getBoundingClientRect();
  const mouseRatioX = (e.clientX - svgEl.left) / svgEl.width;
  const mouseRatioY = (e.clientY - svgEl.top) / svgEl.height;
  viewBox.value = {
    x: viewBox.value.x - (newW - viewBox.value.w) * mouseRatioX,
    y: viewBox.value.y - (newH - viewBox.value.h) * mouseRatioY,
    w: newW,
    h: newH,
  };
}

function onMouseDown(e: MouseEvent): void {
  isDragging = true;
  dragStartX = e.clientX;
  dragStartY = e.clientY;
  dragStartVbX = viewBox.value.x;
  dragStartVbY = viewBox.value.y;
}

function onMouseMove(e: MouseEvent): void {
  if (!isDragging) return;
  const svgEl = (e.currentTarget as SVGElement).getBoundingClientRect();
  const scaleX = viewBox.value.w / svgEl.width;
  const scaleY = viewBox.value.h / svgEl.height;
  viewBox.value = {
    ...viewBox.value,
    x: dragStartVbX - (e.clientX - dragStartX) * scaleX,
    y: dragStartVbY - (e.clientY - dragStartY) * scaleY,
  };
}

function onMouseUp(): void {
  isDragging = false;
}
function onMouseLeave(): void {
  isDragging = false;
}

const viewBoxStr = computed(
  () =>
    `${viewBox.value.x} ${viewBox.value.y} ${viewBox.value.w} ${viewBox.value.h}`,
);

const active = computed(() => encounterStore.width > 0);

// ---------------------------------------------------------------------------
// Target selection
// ---------------------------------------------------------------------------

/** Click handler for entity tokens — selects enemies as the attack target. */
function onEntityClick(entity: PositionsEntity): void {
  if (entity.kind !== "pc" && entity.alive) {
    encounterStore.selectTarget(entity.entity_id);
  }
}

/**
 * Whether a given entity token is the currently selected target. Only an alive
 * enemy can show the selection ring — a dead token never lingers highlighted.
 */
function isSelected(entity: PositionsEntity): boolean {
  return (
    encounterStore.selectedTargetId === entity.entity_id && entity.alive
  );
}

// ---------------------------------------------------------------------------
// Action bar
// ---------------------------------------------------------------------------

/**
 * Resolve the display name for the attack target. Only resolves to an entity
 * that both exists AND is alive; if the selected target is missing or dead,
 * falls back to the first alive enemy entity.
 */
function resolveTargetName(): string {
  const selected = encounterStore.selectedTargetId;
  if (selected) {
    const entity = encounterStore.entities.find(
      (e) => e.entity_id === selected && e.alive,
    );
    if (entity) return entity.name;
  }
  // Fallback: first alive enemy
  const firstAlive = encounterStore.entities.find(
    (e) => e.kind !== "pc" && e.alive,
  );
  return firstAlive?.name ?? "";
}

// ---------------------------------------------------------------------------
// Use Item picker
// ---------------------------------------------------------------------------

/** Whether the item-picker popover is open. */
const usePickerOpen = ref(false);

/** Consumable items from the character's inventory usable in combat. */
const consumables = computed<EquipmentItem[]>(() => {
  const char = gameStore.character;
  if (!char) return [];
  return char.equipment.filter((i) => i.type === "consumable");
});

/** The Use Item button is disabled when there is nothing to use. */
const hasConsumables = computed(() => consumables.value.length > 0);

function useItem(item: EquipmentItem): void {
  send(`use ${item.name}`);
  usePickerOpen.value = false;
}

function onAction(action: CombatActionButton): void {
  if (action.command === "attack") {
    const targetName = resolveTargetName();
    if (targetName) {
      send(`attack ${targetName}`);
    } else {
      send("attack");
    }
  } else if (action.command === "use") {
    // Never send a bare `use` — open the consumable picker instead.
    if (hasConsumables.value) {
      usePickerOpen.value = !usePickerOpen.value;
    }
  } else {
    send(action.command);
  }
}

/** Close the picker on any click outside the encounter window. */
function onDocumentClick(e: MouseEvent): void {
  if (!usePickerOpen.value) return;
  const target = e.target as Node | null;
  if (rootEl.value && target && !rootEl.value.contains(target)) {
    usePickerOpen.value = false;
  }
}

const rootEl = ref<HTMLElement | null>(null);

onMounted(() => {
  document.addEventListener("click", onDocumentClick);
});
onBeforeUnmount(() => {
  document.removeEventListener("click", onDocumentClick);
});

/** Tailwind classes for each button style variant. */
function actionBtnClass(style: string): string {
  if (style === "primary") {
    return "bg-amber-600 hover:bg-amber-500 text-white border border-amber-400";
  }
  if (style === "danger") {
    return "bg-red-700 hover:bg-red-600 text-white border border-red-500";
  }
  // default
  return "bg-gray-700 hover:bg-gray-600 text-gray-100 border border-gray-500";
}

const isLastStand = computed(() => gameStore.combatPhase === "last_stand");
const isPreCombat = computed(() => gameStore.combatPhase === "pre_combat");
</script>

<template>
  <div
    ref="rootEl"
    class="w-full h-full overflow-hidden bg-gray-950 relative select-none flex flex-col"
    data-testid="encounter-window"
  >
    <!-- Pre-combat encounter menu: encounter staged, player choosing a response -->
    <div
      v-if="isPreCombat"
      data-testid="precombat-encounter"
      class="flex flex-col items-center justify-center flex-1 min-h-0 gap-2 px-4 py-4 text-center"
    >
      <div class="text-2xl font-bold text-amber-400 tracking-wide">⚔ Encounter!</div>
      <div class="text-sm text-gray-400 font-mono">Choose your response.</div>
    </div>

    <!-- Placeholder when no encounter is active (not in combat, not pre-combat) -->
    <div
      v-else-if="!active"
      class="w-full h-full flex items-center justify-center text-gray-600 font-mono text-sm"
    >
      No encounter active.
    </div>

    <!-- Grid fills the space above the action bar (active combat) -->
    <div v-else class="relative flex-1 min-h-0">
      <svg
        class="w-full h-full cursor-grab"
        :class="{ 'cursor-grabbing': isDragging }"
        :viewBox="viewBoxStr"
        xmlns="http://www.w3.org/2000/svg"
        @wheel.passive="false"
        @wheel="onWheel"
        @mousedown="onMouseDown"
        @mousemove="onMouseMove"
        @mouseup="onMouseUp"
        @mouseleave="onMouseLeave"
      >
        <!-- Terrain cells -->
        <g v-for="cell in encounterStore.cells" :key="`c${cell.q},${cell.r}`">
          <rect
            :x="cellToPixel(cell.q, cell.r).x"
            :y="cellToPixel(cell.q, cell.r).y"
            :width="CELL_SIZE"
            :height="CELL_SIZE"
            :fill="TERRAIN_COLORS[cell.terrain] ?? '#3a3a3a'"
            stroke="#1a1a1a"
            stroke-width="0.4"
            :data-testid="`encounter-cell-${cell.q}-${cell.r}`"
          />
          <!-- Feature stamps (key terrain corners) -->
          <text
            v-for="feature in cell.features"
            :key="`f${cell.q},${cell.r},${feature}`"
            :x="cellToPixel(cell.q, cell.r).x + CELL_SIZE / 2"
            :y="cellToPixel(cell.q, cell.r).y + CELL_SIZE / 2 + 1"
            text-anchor="middle"
            dominant-baseline="middle"
            :fill="FEATURE_MARKERS[feature]?.color ?? '#888'"
            font-size="8"
            font-family="monospace"
            pointer-events="none"
          >{{ FEATURE_MARKERS[feature]?.label ?? "?" }}</text>
        </g>

        <!-- Loot piles (HR-787): rarity-tiered gem per dropped pile -->
        <g
          v-for="l in lootRenders"
          :key="`loot-${l.key}`"
          :transform="`translate(${l.x}, ${l.y})`"
          :data-testid="`encounter-loot-${l.q}-${l.r}`"
          :data-loot-tier="l.tier"
          pointer-events="none"
        >
          <circle
            cx="12"
            cy="12"
            :r="l.tier === 'legendary' ? 6 : l.tier === 'epic' ? 5 : 4"
            :fill="LOOT_COLORS[l.tier]"
            :opacity="l.tier === 'legendary' ? 0.3 : 0.18"
          />
          <polygon
            :points="lootGemPoints(l.tier)"
            :fill="LOOT_COLORS[l.tier]"
            stroke="#1a1a1a"
            stroke-width="0.4"
          />
        </g>

        <!-- Entity tokens -->
        <g
          v-for="entity in encounterStore.entities"
          :key="entity.entity_id"
          :data-testid="`encounter-entity-${entity.entity_id}`"
          :opacity="entity.alive ? 1 : 0.3"
          :style="entity.kind !== 'pc' && entity.alive ? 'cursor: pointer' : ''"
          @click="onEntityClick(entity)"
        >
          <!-- Selection ring for the targeted enemy -->
          <circle
            v-if="isSelected(entity)"
            :cx="cellToPixel(entity.q, entity.r).x + CELL_SIZE / 2"
            :cy="cellToPixel(entity.q, entity.r).y + CELL_SIZE / 2 - 1"
            :r="entity.kind === 'pc' ? 10 : 9"
            fill="none"
            stroke="#f0c040"
            stroke-width="2"
            pointer-events="none"
            :data-testid="`encounter-target-${entity.entity_id}`"
          />
          <!-- HP bar background (PC + monsters with an HP pool) -->
          <rect
            v-if="entity.max_hp > 0"
            :x="cellToPixel(entity.q, entity.r).x + 1"
            :y="cellToPixel(entity.q, entity.r).y + CELL_SIZE - 5"
            :width="CELL_SIZE - 2"
            height="3"
            fill="#222"
            rx="1"
            pointer-events="none"
          />
          <!-- HP bar fill (PC + monsters) -->
          <rect
            v-if="entity.alive && hpBarPct(entity) > 0"
            :x="cellToPixel(entity.q, entity.r).x + 1"
            :y="cellToPixel(entity.q, entity.r).y + CELL_SIZE - 5"
            :width="(CELL_SIZE - 2) * hpBarPct(entity) / 100"
            height="3"
            :fill="tokenFill(entity)"
            rx="1"
            pointer-events="none"
          />
          <!-- Token circle -->
          <circle
            :cx="cellToPixel(entity.q, entity.r).x + CELL_SIZE / 2"
            :cy="cellToPixel(entity.q, entity.r).y + CELL_SIZE / 2 - 1"
            :r="entity.kind === 'pc' ? 8 : 7"
            :fill="tokenFill(entity)"
            :stroke="tokenStroke(entity)"
            stroke-width="1.2"
            pointer-events="none"
          />
          <!-- First letter of entity name -->
          <text
            :x="cellToPixel(entity.q, entity.r).x + CELL_SIZE / 2"
            :y="cellToPixel(entity.q, entity.r).y + CELL_SIZE / 2"
            text-anchor="middle"
            dominant-baseline="middle"
            fill="#fff"
            font-size="8"
            font-weight="bold"
            font-family="monospace"
            pointer-events="none"
          >{{ entity.name.charAt(0).toUpperCase() }}</text>
        </g>
      </svg>

      <!-- Legend overlay -->
      <div
        class="absolute bottom-1 left-1 text-[9px] font-mono text-gray-500 bg-gray-950/80 px-1.5 py-1 rounded pointer-events-none leading-tight"
      >
        <span class="text-yellow-300">●</span> You
        <span class="text-green-400 ml-1">●</span> Enemy
        <span class="text-gray-500 ml-1 italic">click enemy to target · drag/scroll to pan/zoom</span>
      </div>
    </div>

    <!-- Action bar — rendered when there are buttons to show -->
    <div
      v-if="gameStore.combatActions.length > 0"
      class="flex-shrink-0 relative flex flex-wrap gap-1 px-2 py-1.5 bg-gray-900 border-t border-gray-700"
      :class="{ 'border-red-700 bg-red-950/40': isLastStand }"
      data-testid="encounter-action-bar"
    >
      <button
        v-for="action in gameStore.combatActions"
        :key="action.command"
        :data-testid="`encounter-action-${action.command.replace(/\s+/g, '-')}`"
        class="px-2.5 py-1 text-xs font-mono rounded transition-colors"
        :class="[
          actionBtnClass(action.style),
          action.command === 'use' && !hasConsumables
            ? 'opacity-40 cursor-not-allowed'
            : '',
        ]"
        :disabled="action.command === 'use' && !hasConsumables"
        @click.stop="onAction(action)"
      >
        {{ action.label }}
      </button>

      <!-- Use Item picker popover -->
      <div
        v-if="usePickerOpen"
        class="absolute bottom-full left-2 mb-1 z-20 min-w-[10rem] bg-gray-800 border border-gray-600 rounded shadow-lg p-1 flex flex-col gap-0.5"
        data-testid="encounter-use-picker"
        @click.stop
      >
        <button
          v-for="item in consumables"
          :key="item.name"
          :data-testid="`encounter-use-item-${item.name}`"
          class="px-2 py-1 text-xs font-mono text-left rounded text-gray-100 hover:bg-gray-700 transition-colors"
          @click.stop="useItem(item)"
        >
          {{ item.name
          }}<span
            v-if="item.quantity && item.quantity > 1"
            class="text-gray-400"
          >
            ×{{ item.quantity }}</span
          >
        </button>
      </div>
    </div>
  </div>
</template>
