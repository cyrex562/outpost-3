import { ref } from "vue";
import type { OracleNpc, OracleThread } from "../types/api";
import type { AdminSliceCtx } from "./_adminSliceCtx";

export function createAdminOracleSlice(ctx: AdminSliceCtx) {
  const { worldParam } = ctx;

  const threads = ref<OracleThread[]>([]);
  const oracleNpcs = ref<OracleNpc[]>([]);
  const oracleState = ref<{ chaos_factor: number }>({ chaos_factor: 5 });

  async function loadThreads() {
    const wp = worldParam();
    const res = await fetch(`/api/admin/threads${wp}`);
    if (res.ok) threads.value = await res.json();
  }

  async function createThread(data: { title: string; type?: string }) {
    const wp = worldParam();
    const res = await fetch(`/api/admin/threads${wp}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(data),
    });
    await loadThreads();
    return res.json();
  }

  async function updateThread(id: string, data: Record<string, unknown>) {
    const wp = worldParam();
    await fetch(`/api/admin/threads/${id}${wp}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(data),
    });
    await loadThreads();
  }

  async function deleteThread(id: string) {
    const wp = worldParam();
    await fetch(`/api/admin/threads/${id}${wp}`, { method: "DELETE" });
    await loadThreads();
  }

  async function loadOracleNpcs() {
    const wp = worldParam();
    const res = await fetch(`/api/admin/oracle-npcs${wp}`);
    if (res.ok) oracleNpcs.value = await res.json();
  }

  async function createOracleNpc(data: {
    name: string;
    notes?: string;
    entity_id?: string;
  }) {
    const wp = worldParam();
    const res = await fetch(`/api/admin/oracle-npcs${wp}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(data),
    });
    await loadOracleNpcs();
    return res.json();
  }

  async function updateOracleNpc(id: string, data: Record<string, unknown>) {
    const wp = worldParam();
    await fetch(`/api/admin/oracle-npcs/${id}${wp}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(data),
    });
    await loadOracleNpcs();
  }

  async function deleteOracleNpc(id: string) {
    const wp = worldParam();
    await fetch(`/api/admin/oracle-npcs/${id}${wp}`, { method: "DELETE" });
    await loadOracleNpcs();
  }

  async function loadOracleState() {
    const wp = worldParam();
    const res = await fetch(`/api/admin/oracle-state${wp}`);
    if (res.ok) oracleState.value = await res.json();
  }

  async function updateOracleState(data: { chaos_factor?: number }) {
    const wp = worldParam();
    await fetch(`/api/admin/oracle-state${wp}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(data),
    });
    await loadOracleState();
  }

  return {
    threads,
    oracleNpcs,
    oracleState,
    loadThreads,
    createThread,
    updateThread,
    deleteThread,
    loadOracleNpcs,
    createOracleNpc,
    updateOracleNpc,
    deleteOracleNpc,
    loadOracleState,
    updateOracleState,
  };
}
