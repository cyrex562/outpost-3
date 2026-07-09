import { defineStore } from "pinia";
import { ref } from "vue";
import { worldModel } from "../worldModel/instance";
import { loadWorldMap } from "../worldModel/hydrate";

export interface CellData {
  q: number;
  r: number;
  terrain: string;
  features: string[];
  explored: boolean | number;  // false/0 = fog, true/1 = explored, 2 = seen (adjacent impassable)
  data?: Record<string, unknown>;  // per-cell JSON payload (settlement, forest_biome, …)
}

export const useMapStore = defineStore("map", () => {
  const cells = ref<Map<string, CellData>>(new Map());
  const playerQ = ref<number | null>(null);
  const playerR = ref<number | null>(null);
  const width = ref(0);
  const height = ref(0);
  const loaded = ref(false);

  function cellKey(q: number, r: number): string {
    return `${q},${r}`;
  }

  async function loadMap(): Promise<void> {
    // Delegate to the model hydration path; the projection updates this store.
    const ok = await loadWorldMap(worldModel);
    if (!ok) {
      loaded.value = false;
    }
  }

  function setPlayerPosition(q: number, r: number): void {
    playerQ.value = q;
    playerR.value = r;
  }

  function updateCell(cell: CellData): void {
    cells.value.set(cellKey(cell.q, cell.r), cell);
  }

  function reset(): void {
    cells.value = new Map();
    playerQ.value = null;
    playerR.value = null;
    width.value = 0;
    height.value = 0;
    loaded.value = false;
  }

  return {
    cells,
    playerQ,
    playerR,
    width,
    height,
    loaded,
    cellKey,
    loadMap,
    setPlayerPosition,
    updateCell,
    reset,
  };
});
