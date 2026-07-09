import { ref } from "vue";
import type { EditorRecord, JsonObject, RandomTable } from "../types/api";
import type { AdminSliceCtx } from "./_adminSliceCtx";

export function createAdminContentSlice(ctx: AdminSliceCtx) {
  const { worldParam } = ctx;

  const randomTables = ref<RandomTable[]>([]);
  const worldItems = ref<EditorRecord[]>([]);
  const worldCreatures = ref<EditorRecord[]>([]);

  async function loadRandomTables() {
    const wp = worldParam();
    const res = await fetch(`/api/admin/random-tables${wp}`);
    if (res.ok) randomTables.value = await res.json();
  }

  async function saveRandomTable(table: {
    id: string;
    category: string;
    name: string;
    entries: JsonObject[];
    tags: string[];
  }) {
    const wp = worldParam();
    await fetch(`/api/admin/random-tables/${table.id}${wp}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(table),
    });
  }

  async function deleteRandomTable(id: string) {
    const wp = worldParam();
    await fetch(`/api/admin/random-tables/${id}${wp}`, { method: "DELETE" });
  }

  async function resetRandomTable(id: string) {
    const wp = worldParam();
    await fetch(`/api/admin/random-tables/${id}/reset${wp}`, {
      method: "POST",
    });
  }

  async function resetAllRandomTables() {
    const wp = worldParam();
    await fetch(`/api/admin/random-tables/reset-all${wp}`, { method: "POST" });
  }

  async function loadWorldItems() {
    const wp = worldParam();
    const res = await fetch(`/api/admin/items-data${wp}`);
    if (res.ok) worldItems.value = await res.json();
  }

  async function saveWorldItem(item: EditorRecord) {
    const wp = worldParam();
    await fetch(`/api/admin/items-data/${item.id}${wp}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(item),
    });
  }

  async function deleteWorldItem(id: string) {
    const wp = worldParam();
    await fetch(`/api/admin/items-data/${id}${wp}`, { method: "DELETE" });
  }

  async function resetWorldItem(id: string) {
    const wp = worldParam();
    await fetch(`/api/admin/items-data/${id}/reset${wp}`, { method: "POST" });
  }

  async function resetAllWorldItems() {
    const wp = worldParam();
    await fetch(`/api/admin/items-data/reset-all${wp}`, { method: "POST" });
  }
  async function loadWorldCreatures() {
    const wp = worldParam();
    const res = await fetch(`/api/admin/creature-templates${wp}`);
    if (res.ok) worldCreatures.value = await res.json();
  }

  async function saveWorldCreature(creature: EditorRecord) {
    const wp = worldParam();
    await fetch(`/api/admin/creature-templates/${creature.id}${wp}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(creature),
    });
  }

  async function deleteWorldCreature(id: string) {
    const wp = worldParam();
    await fetch(`/api/admin/creature-templates/${id}${wp}`, {
      method: "DELETE",
    });
  }

  async function resetWorldCreature(id: string) {
    const wp = worldParam();
    await fetch(`/api/admin/creature-templates/${id}/reset${wp}`, {
      method: "POST",
    });
  }

  async function resetAllWorldCreatures() {
    const wp = worldParam();
    await fetch(`/api/admin/creature-templates/reset-all${wp}`, {
      method: "POST",
    });
  }

  return {
    randomTables,
    loadRandomTables,
    saveRandomTable,
    deleteRandomTable,
    resetRandomTable,
    resetAllRandomTables,
    worldItems,
    loadWorldItems,
    saveWorldItem,
    deleteWorldItem,
    resetWorldItem,
    resetAllWorldItems,
    worldCreatures,
    loadWorldCreatures,
    saveWorldCreature,
    deleteWorldCreature,
    resetWorldCreature,
    resetAllWorldCreatures,
  };
}
