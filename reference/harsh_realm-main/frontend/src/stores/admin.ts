import { defineStore } from "pinia";
import { ref } from "vue";
import { createAdminContentSlice } from "./_adminContentSlice";
import { createAdminFilesSlice } from "./_adminFilesSlice";
import { createAdminGmSlice } from "./_adminGmSlice";
import { createAdminOracleSlice } from "./_adminOracleSlice";
import { createAdminWorldStateSlice } from "./_adminWorldStateSlice";

export interface SkillMapping {
  verb: string;
  skill: string;
  attribute: string;
  base_difficulty: number;
  opposed: boolean;
  description: string;
  notes: string;
}

export interface DifficultyTarget {
  name: string;
  target: number;
  description: string;
}

export interface DispositionOutcome {
  outcome_key: string;
  delta: number;
  description: string;
}

export interface EncounterWeight {
  faction_disposition: string;
  encounter_tag: string;
  weight_modifier: number;
}

export interface FactionAssetStat {
  asset_type: string;
  category: string;
  min_attribute: number;
  cost: number;
  upkeep: number;
  max_hp: number;
  attack_stat: string;
  counter_stat: string;
  attack_roll: string;
  special: string;
  description: string;
}

export interface XpProgression {
  level: number;
  xp_needed: number;
}

export const useAdminStore = defineStore("admin", () => {
  const activeWorldPath = ref<string>("");
  const availableWorlds = ref<{ name: string; file: string }[]>([]);
  const isDirty = ref(false);

  const skillMappings = ref<SkillMapping[]>([]);
  const difficultyTargets = ref<DifficultyTarget[]>([]);
  const dispositionOutcomes = ref<DispositionOutcome[]>([]);
  const encounterWeights = ref<EncounterWeight[]>([]);
  const factionAssets = ref<FactionAssetStat[]>([]);
  const xpProgression = ref<XpProgression[]>([]);

  const loading = ref(false);
  const error = ref<string | null>(null);

  function worldParam() {
    return activeWorldPath.value
      ? `?world=${encodeURIComponent(activeWorldPath.value)}`
      : "";
  }

  async function loadWorlds() {
    const res = await fetch("/api/worlds");
    if (res.ok) availableWorlds.value = await res.json();
  }

  async function safeJsonArray(url: string): Promise<unknown[]> {
    const r = await fetch(url);
    if (!r.ok) return [];
    const data = await r.json();
    return Array.isArray(data) ? data : [];
  }

  async function loadAllData() {
    loading.value = true;
    error.value = null;
    try {
      const wp = worldParam();
      const [sm, dt, dispo, ew, fa] = await Promise.all([
        safeJsonArray(`/api/admin/skill-mappings${wp}`),
        safeJsonArray(`/api/admin/difficulty-targets${wp}`),
        safeJsonArray(`/api/admin/disposition-outcomes${wp}`),
        safeJsonArray(`/api/admin/encounter-weights${wp}`),
        safeJsonArray(`/api/admin/faction-assets${wp}`),
      ]);
      skillMappings.value = sm as SkillMapping[];
      difficultyTargets.value = dt as DifficultyTarget[];
      dispositionOutcomes.value = dispo as DispositionOutcome[];
      encounterWeights.value = ew as EncounterWeight[];
      factionAssets.value = fa as FactionAssetStat[];
      const xp = await safeJsonArray("/api/admin/xp-progression" + wp);

      xpProgression.value = xp as XpProgression[];
      isDirty.value = false;
      if (sm.length === 0 && !activeWorldPath.value) {
        error.value =
          "No world loaded. Create or load a world first, or select one in the world selector.";
      }
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function saveSkillMapping(mapping: SkillMapping) {
    const wp = worldParam();
    await fetch(`/api/admin/skill-mappings/${mapping.verb}${wp}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(mapping),
    });
    await loadAllData();
  }

  async function resetSkillMapping(verb: string) {
    const wp = worldParam();
    await fetch(`/api/admin/skill-mappings/${verb}/reset${wp}`, {
      method: "POST",
    });
    await loadAllData();
  }

  async function deleteSkillMapping(verb: string) {
    const wp = worldParam();
    await fetch(`/api/admin/skill-mappings/${encodeURIComponent(verb)}${wp}`, {
      method: "DELETE",
    });
    await loadAllData();
  }

  async function saveDifficultyTarget(dt: DifficultyTarget) {
    const wp = worldParam();
    await fetch(`/api/admin/difficulty-targets/${dt.name}${wp}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ target: dt.target, description: dt.description }),
    });
    await loadAllData();
  }

  async function resetDifficultyTarget(name: string) {
    const wp = worldParam();
    await fetch(`/api/admin/difficulty-targets/${name}/reset${wp}`, {
      method: "POST",
    });
    await loadAllData();
  }

  async function saveDispositionOutcome(outcome: DispositionOutcome) {
    const wp = worldParam();
    await fetch(`/api/admin/disposition-outcomes/${outcome.outcome_key}${wp}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        delta: outcome.delta,
        description: outcome.description,
      }),
    });
    await loadAllData();
  }

  async function saveEncounterWeight(ew: EncounterWeight) {
    const wp = worldParam();
    await fetch(
      `/api/admin/encounter-weights/${ew.faction_disposition}/${ew.encounter_tag}${wp}`,
      {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ weight_modifier: ew.weight_modifier }),
      },
    );
    await loadAllData();
  }

  async function exportConfig() {
    const wp = worldParam();
    const res = await fetch(`/api/admin/export-config${wp}`, {
      method: "POST",
    });
    return res.json();
  }

  async function loadXpProgression() {
    loading.value = true;
    try {
      const wp = worldParam();
      const res = await fetch("/api/admin/xp-progression" + wp);
      if (res.ok) xpProgression.value = await res.json();
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function saveXpProgression(level: number, xp_needed: number) {
    const wp = worldParam();
    const res = await fetch("/api/admin/xp-progression/" + level + wp, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ xp_needed }),
    });
    if (res.ok) await loadXpProgression();
    return res.ok;
  }

  async function resetXpProgression(level: number) {
    const wp = worldParam();
    const res = await fetch(
      "/api/admin/xp-progression/" + level + "/reset" + wp,
      {
        method: "POST",
      },
    );
    if (res.ok) await loadXpProgression();
    return res.ok;
  }

  async function resetAllXpProgression() {
    const wp = worldParam();
    const res = await fetch("/api/admin/xp-progression/reset-all" + wp, {
      method: "POST",
    });
    if (res.ok) await loadXpProgression();
    return res.ok;
  }

  const ctx = { worldParam, activeWorldPath, loading, error };
  const worldState = createAdminWorldStateSlice(ctx);
  const files = createAdminFilesSlice(ctx);
  const content = createAdminContentSlice(ctx);
  const oracle = createAdminOracleSlice(ctx);
  const gm = createAdminGmSlice(ctx);

  return {
    activeWorldPath,
    availableWorlds,
    isDirty,
    loading,
    error,
    skillMappings,
    difficultyTargets,
    dispositionOutcomes,
    encounterWeights,
    factionAssets,
    xpProgression,
    worldParam,
    loadWorlds,
    loadAllData,
    saveSkillMapping,
    resetSkillMapping,
    deleteSkillMapping,
    saveDifficultyTarget,
    resetDifficultyTarget,
    saveDispositionOutcome,
    saveEncounterWeight,
    exportConfig,
    loadXpProgression,
    saveXpProgression,
    resetXpProgression,
    resetAllXpProgression,
    ...worldState,
    ...files,
    ...content,
    ...oracle,
    ...gm,
  };
});
