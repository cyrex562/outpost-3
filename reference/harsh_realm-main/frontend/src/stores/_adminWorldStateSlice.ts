import { ref } from "vue";
import type {
  CellData,
  Character,
  DungeonSummary,
  Faction,
  ItemData,
} from "../types/api";
import type { AdminSliceCtx } from "./_adminSliceCtx";

export function createAdminWorldStateSlice(ctx: AdminSliceCtx) {
  const { worldParam, activeWorldPath, loading, error } = ctx;

  const cells = ref<CellData[]>([]);
  const characters = ref<Character[]>([]);
  const factions = ref<Faction[]>([]);
  const dungeons = ref<DungeonSummary[]>([]);
  const selectedCellCoord = ref<{ q: number; r: number } | null>(null);

  async function loadCells() {
    loading.value = true;
    try {
      const wp = worldParam();
      const sep = wp ? "&" : "?";
      const res = await fetch(`/api/admin/cells${wp}${sep}limit=2000`);
      if (res.ok) cells.value = await res.json();
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function updateCell(q: number, r: number, data: Record<string, unknown>) {
    const wp = worldParam();
    await fetch(`/api/admin/cells/${q}/${r}${wp}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(data),
    });
  }

  async function bulkUpdateCells(
    cellList: { q: number; r: number }[],
    data: Record<string, unknown>,
  ) {
    const wp = worldParam();
    await fetch(`/api/admin/cells/bulk-update${wp}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ cells: cellList, ...data }),
    });
  }

  // Characters
  async function loadCharacters(filter?: {
    entity_type?: string;
    alive?: boolean;
  }) {
    loading.value = true;
    try {
      const params = new URLSearchParams();
      if (activeWorldPath.value) params.set("world", activeWorldPath.value);
      if (filter?.entity_type) params.set("entity_type", filter.entity_type);
      if (filter?.alive !== undefined)
        params.set("alive", String(filter.alive));
      const res = await fetch(`/api/admin/characters?${params}`);
      if (res.ok) characters.value = await res.json();
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function getCharacter(id: string) {
    const wp = worldParam();
    const res = await fetch(`/api/admin/characters/${id}${wp}`);
    return res.json();
  }

  async function updateCharacter(id: string, data: Record<string, unknown>) {
    const wp = worldParam();
    await fetch(`/api/admin/characters/${id}${wp}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(data),
    });
  }

  async function createCharacter(data: Record<string, unknown>) {
    const wp = worldParam();
    const res = await fetch(`/api/admin/characters${wp}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(data),
    });
    return res.json();
  }

  async function deleteCharacter(id: string, hard: boolean = false) {
    const wp = worldParam();
    const sep = wp ? "&" : "?";
    await fetch(`/api/admin/characters/${id}${wp}${sep}hard=${hard}`, {
      method: "DELETE",
    });
  }

  async function previewRecalc(data: {
    character_class: string;
    level: number;
    attributes: Record<string, number>;
    equipment?: ItemData[];
  }) {
    const res = await fetch("/api/admin/characters/preview-recalc", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(data),
    });
    return res.json();
  }

  // Factions (world state)
  async function loadFactions() {
    loading.value = true;
    try {
      const wp = worldParam();
      const res = await fetch(`/api/admin/factions${wp}`);
      if (res.ok) factions.value = await res.json();
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function getFaction(id: string) {
    const wp = worldParam();
    const res = await fetch(`/api/admin/factions/${id}${wp}`);
    return res.json();
  }

  async function updateFaction(id: string, data: Record<string, unknown>) {
    const wp = worldParam();
    await fetch(`/api/admin/factions/${id}${wp}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(data),
    });
  }

  async function createFaction(data: Record<string, unknown>) {
    const wp = worldParam();
    const res = await fetch(`/api/admin/factions${wp}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(data),
    });
    return res.json();
  }

  async function deleteFaction(id: string) {
    const wp = worldParam();
    await fetch(`/api/admin/factions/${id}${wp}`, { method: "DELETE" });
  }

  // Dungeons
  async function loadDungeons() {
    loading.value = true;
    try {
      const wp = worldParam();
      const res = await fetch(`/api/admin/dungeons${wp}`);
      if (res.ok) dungeons.value = await res.json();
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function getDungeon(id: string) {
    const wp = worldParam();
    const res = await fetch(`/api/admin/dungeons/${id}${wp}`);
    return res.json();
  }

  async function updateDungeon(id: string, data: Record<string, unknown>) {
    const wp = worldParam();
    await fetch(`/api/admin/dungeons/${id}${wp}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(data),
    });
  }

  async function createDungeon(data: Record<string, unknown>) {
    const wp = worldParam();
    const res = await fetch(`/api/admin/dungeons${wp}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(data),
    });
    return res.json();
  }

  async function deleteDungeon(id: string) {
    const wp = worldParam();
    await fetch(`/api/admin/dungeons/${id}${wp}`, { method: "DELETE" });
  }


  return {
    cells,
    characters,
    factions,
    dungeons,
    selectedCellCoord,
    loadCells,
    updateCell,
    bulkUpdateCells,
    loadCharacters,
    getCharacter,
    updateCharacter,
    createCharacter,
    deleteCharacter,
    previewRecalc,
    loadFactions,
    getFaction,
    updateFaction,
    createFaction,
    deleteFaction,
    loadDungeons,
    getDungeon,
    updateDungeon,
    createDungeon,
    deleteDungeon,
  };
}
