import { defineStore } from "pinia";
import { ref } from "vue";
import type {
  CombatPositionsData,
  PositionsCell,
  PositionsEntity,
  PositionsLoot,
} from "../types/api";

export const useEncounterStore = defineStore("encounter", () => {
  const width = ref(0);
  const height = ref(0);
  const cells = ref<PositionsCell[]>([]);
  const entities = ref<PositionsEntity[]>([]);
  /** Loot piles dropped by defeated enemies (HR-787). */
  const loot = ref<PositionsLoot[]>([]);
  /** entity_id of the currently selected target, or null if none. */
  const selectedTargetId = ref<string | null>(null);

  function setGrid(data: CombatPositionsData): void {
    width.value = data.width;
    height.value = data.height;
    cells.value = data.cells;
    entities.value = data.entities;
    loot.value = data.loot ?? [];
  }

  function updateHp(id: string, hp: number, maxHp: number): void {
    const entity = entities.value.find((e) => e.entity_id === id);
    if (entity) {
      entity.hp = hp;
      entity.max_hp = maxHp;
    }
  }

  function markDefeated(id: string): void {
    const idx = entities.value.findIndex((e) => e.entity_id === id);
    if (idx !== -1) {
      entities.value[idx] = { ...entities.value[idx], alive: false };
    }
    // Clear the selection if the defeated enemy was the current target, so a
    // subsequent Attack doesn't silently retarget a dead enemy.
    if (selectedTargetId.value === id) {
      selectedTargetId.value = null;
    }
  }

  function selectTarget(id: string | null): void {
    selectedTargetId.value = id;
  }

  function clear(): void {
    width.value = 0;
    height.value = 0;
    cells.value = [];
    entities.value = [];
    loot.value = [];
    selectedTargetId.value = null;
  }

  return {
    width,
    height,
    cells,
    entities,
    loot,
    selectedTargetId,
    setGrid,
    updateHp,
    markDefeated,
    selectTarget,
    clear,
  };
});
