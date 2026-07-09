/**
 * Content-authoring store: drives compile / migrate / store and the references.
 */
import { defineStore } from "pinia";
import { ref } from "vue";

import type {
  ContentCompileReport,
  DemoCombatResponse,
  JsonObject,
  LegacyCategory,
  MigrateResponse,
  StoreResponse,
} from "../types/api";

export const useContentStore = defineStore("content", () => {
  const report = ref<ContentCompileReport | null>(null);
  const reference = ref<string>("");
  const compiling = ref<boolean>(false);
  const error = ref<string | null>(null);

  const legacyCategories = ref<LegacyCategory[]>([]);
  const needs = ref<string[]>([]);
  const unmet = ref<string[]>([]);
  const notes = ref<string[]>([]);
  const storedCount = ref<number | null>(null);
  const storedCounts = ref<Record<string, number>>({});
  const storedRecords = ref<JsonObject[]>([]);
  const demo = ref<DemoCombatResponse | null>(null);
  const demoRunning = ref<boolean>(false);

  async function compile(yaml: string): Promise<void> {
    compiling.value = true;
    error.value = null;
    storedCount.value = null;
    try {
      const resp = await fetch("/api/content/compile", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ yaml }),
      });
      if (!resp.ok) {
        error.value = await errorDetail(resp);
        report.value = null;
        return;
      }
      report.value = (await resp.json()) as ContentCompileReport;
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
      report.value = null;
    } finally {
      compiling.value = false;
    }
  }

  async function store(yaml: string): Promise<void> {
    error.value = null;
    storedCount.value = null;
    const resp = await fetch("/api/content/store", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ yaml }),
    });
    if (!resp.ok) {
      error.value = await errorDetail(resp);
      return;
    }
    const body = (await resp.json()) as StoreResponse;
    report.value = body.report;
    storedCount.value = body.stored;
    await loadStoredRecords();
  }

  async function loadLegacyCategories(): Promise<void> {
    const resp = await fetch("/api/content/legacy/categories");
    if (resp.ok) {
      const body = (await resp.json()) as { categories: LegacyCategory[] };
      legacyCategories.value = body.categories;
    }
  }

  async function migrate(category: string): Promise<string> {
    error.value = null;
    const resp = await fetch("/api/content/migrate", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ category }),
    });
    if (!resp.ok) {
      error.value = await errorDetail(resp);
      return "";
    }
    const body = (await resp.json()) as MigrateResponse;
    needs.value = body.needs;
    unmet.value = body.unmet;
    notes.value = body.notes;
    return body.yaml;
  }

  async function seedActions(): Promise<void> {
    error.value = null;
    storedCount.value = null;
    const resp = await fetch("/api/content/seed/actions", { method: "POST" });
    if (!resp.ok) {
      error.value = await errorDetail(resp);
      return;
    }
    const body = (await resp.json()) as StoreResponse;
    storedCount.value = body.stored;
    await loadStoredRecords();
  }

  async function loadStoredRecords(): Promise<void> {
    const resp = await fetch("/api/content/records");
    if (resp.ok) {
      const body = (await resp.json()) as {
        records: JsonObject[];
        counts: Record<string, number>;
      };
      storedRecords.value = body.records;
      storedCounts.value = body.counts;
    }
  }

  async function loadRecordYaml(componentType: string, id: string): Promise<string> {
    error.value = null;
    const resp = await fetch(
      `/api/content/records/${componentType}/${id}/yaml`,
    );
    if (!resp.ok) {
      error.value = await errorDetail(resp);
      return "";
    }
    return ((await resp.json()) as { yaml: string }).yaml;
  }

  async function deleteRecord(componentType: string, id: string): Promise<void> {
    error.value = null;
    const resp = await fetch(`/api/content/records/${componentType}/${id}`, {
      method: "DELETE",
    });
    if (!resp.ok) {
      error.value = await errorDetail(resp);
      return;
    }
    await loadStoredRecords();
  }

  /** Run an isolated demo combat for the authored content (off the session). */
  async function runDemoCombat(
    yaml: string,
    creatureId: string,
    weaponId: string | null,
  ): Promise<void> {
    demoRunning.value = true;
    error.value = null;
    demo.value = null;
    try {
      const resp = await fetch("/api/demo/combat", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          yaml,
          creature_id: creatureId,
          pc: weaponId ? { weapon_id: weaponId } : {},
        }),
      });
      if (!resp.ok) {
        error.value = await errorDetail(resp);
        return;
      }
      demo.value = (await resp.json()) as DemoCombatResponse;
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    } finally {
      demoRunning.value = false;
    }
  }

  async function loadReference(): Promise<void> {
    const resp = await fetch("/api/content/reference");
    if (resp.ok) {
      const body = (await resp.json()) as { markdown: string };
      reference.value = body.markdown;
    }
  }

  async function loadTemplate(): Promise<string> {
    const resp = await fetch("/api/content/template");
    if (!resp.ok) {
      return "";
    }
    const body = (await resp.json()) as { yaml: string };
    return body.yaml;
  }

  async function errorDetail(resp: Response): Promise<string> {
    const body = (await resp.json().catch(() => ({}))) as { detail?: string };
    return body.detail ?? `Request failed (${resp.status})`;
  }

  return {
    report,
    reference,
    compiling,
    error,
    legacyCategories,
    needs,
    unmet,
    notes,
    storedCount,
    storedCounts,
    storedRecords,
    demo,
    demoRunning,
    compile,
    store,
    seedActions,
    loadLegacyCategories,
    migrate,
    loadStoredRecords,
    loadRecordYaml,
    deleteRecord,
    runDemoCombat,
    loadReference,
    loadTemplate,
  };
});
