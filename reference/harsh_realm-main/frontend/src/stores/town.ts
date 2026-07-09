import { defineStore } from "pinia";
import { ref } from "vue";

export interface TownCell {
  q: number;
  r: number;
  terrain: string;
  features?: string[];
}

export interface TownBuilding {
  name: string;
  type: string;
}

export const useTownStore = defineStore("town", () => {
  const cells = ref<TownCell[]>([]);
  const buildings = ref<TownBuilding[]>([]);
  const playerQ = ref(0);
  const playerR = ref(0);
  const settlementName = ref("");
  const active = ref(false);

  function setTownMap(data: {
    cells: TownCell[];
    buildings: TownBuilding[];
    player_q: number;
    player_r: number;
    settlement_name: string;
  }) {
    cells.value = data.cells;
    buildings.value = data.buildings;
    playerQ.value = data.player_q;
    playerR.value = data.player_r;
    settlementName.value = data.settlement_name;
    active.value = true;
  }

  function movePlayer(q: number, r: number) {
    playerQ.value = q;
    playerR.value = r;
  }

  function clear() {
    cells.value = [];
    buildings.value = [];
    active.value = false;
  }

  return {
    cells,
    buildings,
    playerQ,
    playerR,
    settlementName,
    active,
    setTownMap,
    movePlayer,
    clear,
  };
});
