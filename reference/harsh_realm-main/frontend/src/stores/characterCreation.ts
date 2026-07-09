import { defineStore } from "pinia";
import { computed, ref } from "vue";
import type {
  AttributeMethod,
  CharacterCreationOptions,
  CharacterDraftSubmission,
  ClassOption,
  EquipmentKitOption,
} from "../types/api";
import type { SkillDraftContext } from "../utils/skillRequirements";

export const ATTRS = ["str", "dex", "con", "int", "wis", "cha"] as const;
export type Attr = (typeof ATTRS)[number];

/** XWN attribute modifier — mirrors the backend `attr_modifier`. */
export function attrModifier(score: number): number {
  if (score <= 3) return -2;
  if (score <= 7) return -1;
  if (score <= 13) return 0;
  if (score <= 17) return 1;
  return 2;
}

/** Total point-buy cost for a set of scores, mirroring the backend table. */
export function pointBuyCost(
  scores: number[],
  costs: Record<number, number>,
): number {
  return scores.reduce((sum, s) => sum + (costs[s] ?? 0), 0);
}

/** Cost to train an untrained skill (-1) up to `level`, mirroring the backend. */
export function skillCost(level: number): number {
  return level - -1;
}

function emptyAttrs(): Record<Attr, number | null> {
  return { str: null, dex: null, con: null, int: null, wis: null, cha: null };
}

export const useCharacterCreationStore = defineStore("characterCreation", () => {
  const options = ref<CharacterCreationOptions | null>(null);
  const loading = ref(false);
  const submitting = ref(false);
  const error = ref<string | null>(null);

  const name = ref("");
  const characterClass = ref("");
  // Attributes are rolled up-front by default; players may switch methods.
  const method = ref<AttributeMethod>("roll");
  const attrValues = ref<Record<Attr, number | null>>(emptyAttrs());
  const rolledScores = ref<number[]>([]);
  const skills = ref<Record<string, number>>({});
  const kitId = ref("");
  const currentMilestone = ref(1);

  // --- derived option lookups ---
  const selectedClass = computed<ClassOption | null>(
    () => options.value?.classes.find((c) => c.id === characterClass.value) ?? null,
  );
  const selectedKit = computed<EquipmentKitOption | null>(
    () => options.value?.kits.find((k) => k.id === kitId.value) ?? null,
  );
  const availableKits = computed<EquipmentKitOption[]>(
    () => options.value?.kits.filter((k) => k.class === characterClass.value) ?? [],
  );

  /** The multiset of scores to distribute (roll/standard_array methods). */
  const sourcePool = computed<number[]>(() => {
    if (method.value === "roll") return rolledScores.value;
    if (method.value === "standard_array")
      return options.value?.standard_array ?? [];
    return [];
  });

  /** Count of how many of `value` remain unassigned (excluding `exclude`). */
  function remainingCount(value: number, exclude?: Attr): number {
    const total = sourcePool.value.filter((v) => v === value).length;
    const used = ATTRS.filter(
      (a) => a !== exclude && attrValues.value[a] === value,
    ).length;
    return total - used;
  }

  /** Distinct values selectable for `attr` (those still available + current). */
  function availableValues(attr: Attr): number[] {
    const distinct = [...new Set(sourcePool.value)].sort((a, b) => b - a);
    return distinct.filter(
      (v) => remainingCount(v, attr) > 0 || attrValues.value[attr] === v,
    );
  }

  // --- point buy ---
  const pointBuySpent = computed(() => {
    if (method.value !== "point_buy" || !options.value) return 0;
    const scores = ATTRS.map((a) => attrValues.value[a]).filter(
      (v): v is number => v != null,
    );
    return pointBuyCost(scores, options.value.point_buy.costs);
  });
  const pointBuyRemaining = computed(
    () => (options.value?.point_buy.budget ?? 0) - pointBuySpent.value,
  );

  // --- skills ---
  const skillBudget = computed(() => selectedClass.value?.skill_points_start ?? 0);
  const skillSpent = computed(() =>
    Object.values(skills.value).reduce((sum, lvl) => sum + skillCost(lvl), 0),
  );
  const skillRemaining = computed(() => skillBudget.value - skillSpent.value);

  // --- attribute completeness ---
  const attributesComplete = computed(() =>
    ATTRS.every((a) => attrValues.value[a] != null),
  );

  /** Numeric attribute map (assigned values only) for requirement checks. */
  const attributesNumeric = computed<Record<string, number>>(() => {
    const out: Record<string, number> = {};
    for (const a of ATTRS) {
      const v = attrValues.value[a];
      if (v != null) out[a] = v;
    }
    return out;
  });

  /** Context the SkillTree evaluates requirements against. */
  const skillContext = computed<SkillDraftContext>(() => ({
    characterClass: characterClass.value,
    attributes: attributesNumeric.value,
    currentMilestone: currentMilestone.value,
    traits: [],
  }));

  // --- derived preview ---
  const attrMods = computed<Record<Attr, number>>(() => {
    const out = {} as Record<Attr, number>;
    for (const a of ATTRS) {
      const v = attrValues.value[a];
      out[a] = v == null ? 0 : attrModifier(v);
    }
    return out;
  });

  const previewAc = computed(() => {
    let base = 10 + attrMods.value.dex;
    const kit = selectedKit.value;
    if (kit) {
      let armor: number | null = null;
      let shield = 0;
      for (const item of kit.items) {
        if (item.type === "armor") {
          if (item.ac != null) armor = Math.max(armor ?? 0, item.ac);
          else if (item.ac_bonus != null)
            armor = Math.max(armor ?? 0, item.ac_bonus + 10);
        } else if (item.type === "shield") {
          shield += item.ac_bonus ?? 0;
        }
      }
      base = armor != null ? armor + shield : base + shield;
    }
    return base;
  });

  const previewHp = computed<{ min: number; max: number } | null>(() => {
    const hd = selectedClass.value?.hit_die;
    if (hd == null) return null;
    const conMod = attrMods.value.con;
    return { min: Math.max(1, 1 + conMod), max: Math.max(1, hd + conMod) };
  });

  const previewSaves = computed(() => {
    const v = (a: Attr): number => attrValues.value[a] ?? 0;
    return {
      physical: 15 - Math.max(v("con"), v("str")),
      evasion: 15 - Math.max(v("dex"), v("int")),
      mental: 15 - Math.max(v("wis"), v("cha")),
    };
  });

  // --- validation ---
  const validationError = computed<string | null>(() => {
    if (!name.value.trim()) return "Enter a character name.";
    if (!characterClass.value) return "Choose a class.";
    if (!attributesComplete.value) return "Assign all six attributes.";
    if (method.value === "point_buy" && pointBuyRemaining.value < 0)
      return "Point-buy is over budget.";
    if (skillRemaining.value < 0) return "Too many skill points spent.";
    if (!kitId.value) return "Choose a starting kit.";
    return null;
  });
  const canSubmit = computed(() => validationError.value == null && !submitting.value);

  // --- actions ---
  async function loadOptions(): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      const r = await fetch("/api/character/options");
      if (!r.ok) {
        error.value = `Failed to load options (${r.status}).`;
        return;
      }
      options.value = (await r.json()) as CharacterCreationOptions;
      currentMilestone.value = options.value.current_milestone;
      if (!options.value.methods.includes(method.value)) {
        setMethod(options.value.methods[0] ?? "standard_array");
      } else {
        setMethod(method.value);
      }
    } catch {
      error.value = "Could not reach the server.";
    } finally {
      loading.value = false;
    }
  }

  function setClass(id: string): void {
    characterClass.value = id;
    // Kit and skills are class-specific — reset them when the class changes.
    skills.value = {};
    if (selectedKit.value && selectedKit.value.class !== id) kitId.value = "";
  }

  function setMethod(m: AttributeMethod): void {
    method.value = m;
    if (m === "point_buy") {
      const min = options.value?.point_buy.min_score ?? 8;
      const next = emptyAttrs();
      for (const a of ATTRS) next[a] = min;
      attrValues.value = next;
    } else {
      attrValues.value = emptyAttrs();
      if (m === "roll" && rolledScores.value.length !== 6) {
        void rollAttributes();
      }
    }
  }

  async function rollAttributes(): Promise<void> {
    try {
      const r = await fetch("/api/character/roll", { method: "POST" });
      if (!r.ok) return;
      const data = (await r.json()) as { scores: number[] };
      rolledScores.value = data.scores;
      if (method.value === "roll") attrValues.value = emptyAttrs();
    } catch {
      /* ignore — keep prior roll */
    }
  }

  function assignAttr(attr: Attr, value: number | null): void {
    attrValues.value[attr] = value;
  }

  function bumpPointBuy(attr: Attr, delta: number): void {
    if (!options.value || method.value !== "point_buy") return;
    const { min_score, max_score, costs } = options.value.point_buy;
    const current = attrValues.value[attr] ?? min_score;
    const next = current + delta;
    if (next < min_score || next > max_score) return;
    // Reject increases that would exceed the budget.
    const projected = ATTRS.map((a) =>
      a === attr ? next : (attrValues.value[a] ?? min_score),
    );
    if (pointBuyCost(projected, costs) > options.value.point_buy.budget) return;
    attrValues.value[attr] = next;
  }

  async function autoPickSkills(): Promise<boolean> {
    if (!characterClass.value) return false;
    try {
      const r = await fetch("/api/character/auto-pick-skills", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          character_class: characterClass.value,
          attributes: attributesNumeric.value,
        }),
      });
      if (!r.ok) return false;
      const data = (await r.json()) as { skills: Record<string, number> };
      skills.value = { ...data.skills };
      return true;
    } catch {
      return false;
    }
  }

  function setSkill(skillId: string, level: number): void {
    if (level < 0) {
      const next = { ...skills.value };
      delete next[skillId];
      skills.value = next;
    } else {
      skills.value = { ...skills.value, [skillId]: level };
    }
  }

  function setKit(id: string): void {
    kitId.value = id;
  }

  async function submit(): Promise<boolean> {
    if (!canSubmit.value) return false;
    submitting.value = true;
    error.value = null;
    const attributes: Record<string, number> = {};
    for (const a of ATTRS) {
      const v = attrValues.value[a];
      if (v == null) {
        submitting.value = false;
        return false;
      }
      attributes[a] = v;
    }
    const payload: CharacterDraftSubmission = {
      name: name.value.trim(),
      character_class: characterClass.value,
      attribute_method: method.value,
      attributes,
      skills: skills.value,
      kit_id: kitId.value,
      ...(method.value === "roll" ? { rolled_scores: rolledScores.value } : {}),
    };
    try {
      const r = await fetch("/api/character/create", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
      });
      if (!r.ok) {
        const body = (await r.json().catch(() => ({}))) as { detail?: string };
        error.value = body.detail ?? `Creation failed (${r.status}).`;
        return false;
      }
      return true;
    } catch {
      error.value = "Could not reach the server.";
      return false;
    } finally {
      submitting.value = false;
    }
  }

  function reset(): void {
    options.value = null;
    name.value = "";
    characterClass.value = "";
    method.value = "roll";
    attrValues.value = emptyAttrs();
    rolledScores.value = [];
    skills.value = {};
    kitId.value = "";
    currentMilestone.value = 1;
    error.value = null;
    submitting.value = false;
  }

  return {
    options,
    loading,
    submitting,
    error,
    name,
    characterClass,
    method,
    attrValues,
    rolledScores,
    skills,
    kitId,
    currentMilestone,
    selectedClass,
    selectedKit,
    availableKits,
    attributesNumeric,
    skillContext,
    sourcePool,
    availableValues,
    pointBuySpent,
    pointBuyRemaining,
    skillBudget,
    skillSpent,
    skillRemaining,
    attributesComplete,
    attrMods,
    previewAc,
    previewHp,
    previewSaves,
    validationError,
    canSubmit,
    loadOptions,
    setClass,
    setMethod,
    rollAttributes,
    assignAttr,
    bumpPointBuy,
    autoPickSkills,
    setSkill,
    setKit,
    submit,
    reset,
  };
});
