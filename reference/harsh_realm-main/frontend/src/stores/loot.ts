import { defineStore } from "pinia";
import { ref } from "vue";
import type { LootSourceView } from "../worldModel/model";

/**
 * Revealed searchable loot sources on the current cell (HR-786), shown in the
 * loot panel. Populated by the worldModel projection from `model.loot`.
 */
export const useLootStore = defineStore("loot", () => {
  const sources = ref<LootSourceView[]>([]);

  function setSources(next: LootSourceView[]): void {
    sources.value = next;
  }

  function clear(): void {
    sources.value = [];
  }

  return { sources, setSources, clear };
});
