import { ref } from "vue";
import type { ItemData } from "../types/api";
import type { AdminSliceCtx } from "./_adminSliceCtx";

export function createAdminGmSlice(_ctx: AdminSliceCtx) {

  const eventLog = ref<Record<string, unknown>[]>([]);

  async function gmTeleport(entityId: string, q: number, r: number) {
    const res = await fetch("/api/gm/teleport", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ entity_id: entityId, q, r }),
    });
    return res.json();
  }

  async function gmSpawnEntity(data: {
    entity_type: string;
    name: string;
    q: number;
    r: number;
    data?: Record<string, unknown>;
  }) {
    const res = await fetch("/api/gm/spawn-entity", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(data),
    });
    return res.json();
  }

  async function gmGiveItem(entityId: string, item: ItemData) {
    const res = await fetch("/api/gm/give-item", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ entity_id: entityId, item }),
    });
    return res.json();
  }

  async function gmSetHp(entityId: string, hp: number) {
    const res = await fetch("/api/gm/set-hp", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ entity_id: entityId, hp }),
    });
    return res.json();
  }

  async function gmSetGold(entityId: string, gold: number) {
    const res = await fetch("/api/gm/set-gold", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ entity_id: entityId, gold }),
    });
    return res.ok;
  }

  async function gmSetXp(entityId: string, xp: number) {
    const res = await fetch("/api/gm/set-xp", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ entity_id: entityId, xp }),
    });
    return res.ok;
  }

  async function gmSetAttr(entityId: string, attribute: string, value: number) {
    const res = await fetch("/api/gm/set-attr", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ entity_id: entityId, attribute, value }),
    });
    return res.ok;
  }
  function addEvent(event: Record<string, unknown>) {
    eventLog.value.push({
      ...event,
      timestamp: new Date().toISOString(),
    });
    if (eventLog.value.length > 200) {
      eventLog.value.shift();
    }
  }

  return {
    eventLog,
    gmTeleport,
    gmSpawnEntity,
    gmGiveItem,
    gmSetHp,
    gmSetGold,
    gmSetXp,
    gmSetAttr,
    addEvent,
  };
}
