import { defineStore } from "pinia";
import { ref } from "vue";
import type { ActiveStatusEffect, CombatActionButton } from "../types/api";
import { useMapStore } from "./map";
import { worldModel } from "../worldModel/instance";
import {
  loadCharacter as modelLoadCharacter,
} from "../worldModel/hydrate";

export interface ChatMessage {
  id: number;
  sender: "you" | "gm" | "system";
  type:
    | "player_input"
    | "narration"
    | "game_event"
    | "system"
    | "skill_check"
    | "disposition"
    | "reroll"
    | "purchase"
    | "sale"
    | "faction_event"
    | "reputation"
    | "chaos"
    | "combat_attack"
    | "combat_save"
    | "inventory"
    | "death";
  text: string;
  timestamp: string;
}

export interface EquipmentItem {
  id?: string;
  item_id?: string;
  name: string;
  type: string;
  enc?: number;
  damage?: string;
  weapon_damage?: string;
  ac?: number;
  ac_bonus?: number;
  quantity?: number;
  shock_damage?: string;
  shock_ac_threshold?: number;
}

export interface CharacterState {
  id: string;
  name: string;
  characterClass: string;
  level: number;
  hp: number;
  maxHp: number;
  ac: number;
  xp: number;
  xpNext: number;
  gold: number;
  str: number;
  locationQ: number | null;
  locationR: number | null;
  terrain: string | null;
  features: string[];
  weather: {
    condition: string;
    temperature: string;
    description: string;
  } | null;
  conditions: string[];
  statusEffects: ActiveStatusEffect[];
  equipment: EquipmentItem[];
}

export interface CombatEnemy {
  entityId: string;
  name: string;
  hp: number;
  maxHp: number;
  defeated: boolean;
}

let _msgId = 0;

export const useGameStore = defineStore("game", () => {
  const messages = ref<ChatMessage[]>([]);
  const character = ref<CharacterState | null>(null);
  const pendingInput = ref<string | null>(null);
  const currentSuggestions = ref<string[]>([]);
  const currentScene = ref<string>("exploration");
  const chaosFactor = ref<number>(5);
  /** Active (accepted / in-progress) quest count (HR-19). */
  const questActive = ref<number>(0);
  const combatEnemies = ref<CombatEnemy[]>([]);
  /** Structured action buttons for the combat action bar (from `combat.actions`). */
  const combatActions = ref<CombatActionButton[]>([]);
  /** Current combat phase string (from `combat.actions`). */
  const combatPhase = ref<string>("");
  /** Tile-context action buttons for the exploration action bar (from `exploration.actions`). */
  const explorationActions = ref<CombatActionButton[]>([]);
  // Pending attribute assignment — set when the GM announces rolled scores
  // so the drag-and-drop modal can pop up in place of typing numbers one by one.
  const attributeRoll = ref<number[] | null>(null);
  // Whether the first /api/character fetch has completed (so we can tell
  // "no character yet, show creation" apart from "still loading").
  const characterChecked = ref<boolean>(false);
  // Character-creation UI mode: the forms flow is the default; players may opt
  // into the turn-by-turn chat wizard instead.
  const creationMode = ref<"form" | "chat">("form");
  // An interrupted journey awaiting a resume/cancel decision (HR-408).
  const pendingTravel = ref<{ q: number; r: number } | null>(null);

  function addMessage(
    sender: ChatMessage["sender"],
    text: string,
    type: ChatMessage["type"] = "system",
  ) {
    messages.value.push({
      id: ++_msgId,
      sender,
      type,
      text,
      timestamp: new Date().toISOString(),
    });
  }

  async function loadCharacter() {
    // Delegate to the model hydration path so the worldModel stays current
    // and the projection updates this store reactively.
    try {
      await modelLoadCharacter(worldModel);
      // Mirror mapStore player position from model after hydration
      const ch = worldModel.character;
      if (ch !== null) {
        const locationQ = ch.locationQ;
        const locationR = ch.locationR;
        if (locationQ != null && locationR != null) {
          useMapStore().setPlayerPosition(locationQ, locationR);
        }
      }
    } catch {
      /* silently ignore */
    } finally {
      characterChecked.value = true;
    }
  }

  async function loadStatusEffects(entityId: string) {
    try {
      const r = await fetch(
        `/api/character/${encodeURIComponent(entityId)}/status_effects`,
      );
      if (!r.ok) return;
      const data = (await r.json()) as unknown;
      if (
        !Array.isArray(data) ||
        !character.value ||
        character.value.id !== entityId
      ) {
        return;
      }
      character.value.statusEffects = data.filter(isActiveStatusEffect);
      character.value.conditions =
        character.value.statusEffects.map(effectLabel);
    } catch {
      /* silently ignore */
    }
  }

  /** Refresh the active-quest count from the server (HR-19). */
  async function loadQuests(entityId: string) {
    try {
      const r = await fetch(
        `/api/character/${encodeURIComponent(entityId)}/quests`,
      );
      if (!r.ok) return;
      const data = (await r.json()) as { active?: unknown };
      questActive.value = Array.isArray(data.active) ? data.active.length : 0;
    } catch {
      /* silently ignore */
    }
  }

  function updateLocation(
    q: number,
    r: number,
    terrain?: string,
    features?: string[],
    weather?: {
      condition: string;
      temperature: string;
      description: string;
    },
  ) {
    if (character.value) {
      character.value.locationQ = q;
      character.value.locationR = r;
      if (terrain !== undefined) character.value.terrain = terrain;
      if (features !== undefined) character.value.features = features;
      if (weather !== undefined) character.value.weather = weather;
    }
  }

  function updateHp(hp: number, maxHp?: number) {
    if (character.value) {
      character.value.hp = hp;
      if (maxHp !== undefined) character.value.maxHp = maxHp;
    }
  }

  function updateXp(xp: number, xpNext?: number) {
    if (character.value) {
      character.value.xp = xp;
      if (xpNext !== undefined) character.value.xpNext = xpNext;
    }
  }

  function updateGold(gold: number) {
    if (character.value) {
      character.value.gold = gold;
    }
  }

  function addStatusEffect(effect: ActiveStatusEffect) {
    if (!character.value || character.value.id !== effect.entity_id) return;
    const existingIndex = character.value.statusEffects.findIndex(
      (existing) => existing.id === effect.id,
    );
    if (existingIndex >= 0) {
      character.value.statusEffects.splice(existingIndex, 1, effect);
    } else {
      character.value.statusEffects.push(effect);
    }
    character.value.conditions = character.value.statusEffects.map(effectLabel);
  }

  function removeStatusEffectById(effectId: number) {
    if (!character.value) return;
    character.value.statusEffects = character.value.statusEffects.filter(
      (effect) => effect.id !== effectId,
    );
    character.value.conditions = character.value.statusEffects.map(effectLabel);
  }

  function removeStatusEffect(entityId: string, effectId: string) {
    if (!character.value || character.value.id !== entityId) return;
    character.value.statusEffects = character.value.statusEffects.filter(
      (effect) => effect.effect_id !== effectId,
    );
    character.value.conditions = character.value.statusEffects.map(effectLabel);
  }

  function setScene(scene: string) {
    currentScene.value = scene;
  }

  function setChaos(chaos: number) {
    chaosFactor.value = chaos;
  }

  function addSuggestions(commands: string[]) {
    currentSuggestions.value = commands;
  }

  function setSuggestInput(text: string) {
    pendingInput.value = text;
  }

  function reset() {
    messages.value = [];
    character.value = null;
    pendingInput.value = null;
    currentSuggestions.value = [];
    currentScene.value = "exploration";
    chaosFactor.value = 5;
    characterChecked.value = false;
    creationMode.value = "form";
    pendingTravel.value = null;
    combatEnemies.value = [];
    combatActions.value = [];
    combatPhase.value = "";
    explorationActions.value = [];
  }

  function setAttributeRoll(scores: number[] | null) {
    attributeRoll.value = scores;
  }

  function setCreationMode(mode: "form" | "chat") {
    creationMode.value = mode;
  }

  function setPendingTravel(target: { q: number; r: number } | null) {
    pendingTravel.value = target;
  }

  function initCombatEnemies(enemies: Array<{ entity_id: string; name: string; hp: number; max_hp: number }>) {
    combatEnemies.value = enemies.map((e) => ({
      entityId: e.entity_id,
      name: e.name,
      hp: e.hp,
      maxHp: e.max_hp,
      defeated: false,
    }));
  }

  function updateCombatEnemyHp(entityId: string, hp: number, maxHp: number) {
    const enemy = combatEnemies.value.find((e) => e.entityId === entityId);
    if (enemy) {
      enemy.hp = hp;
      enemy.maxHp = maxHp;
    }
  }

  function markEnemyDefeated(entityId: string) {
    const enemy = combatEnemies.value.find((e) => e.entityId === entityId);
    if (enemy) {
      enemy.hp = 0;
      enemy.defeated = true;
    }
  }

  function clearCombatEnemies() {
    combatEnemies.value = [];
    combatActions.value = [];
    combatPhase.value = "";
  }

  function setCombatActions(payload: { phase: string; actions: CombatActionButton[] }): void {
    combatPhase.value = payload.phase;
    combatActions.value = payload.actions;
  }

  function setExplorationActions(actions: CombatActionButton[]): void {
    explorationActions.value = actions;
  }

  return {
    messages,
    character,
    pendingInput,
    currentSuggestions,
    currentScene,
    chaosFactor,
    attributeRoll,
    characterChecked,
    creationMode,
    pendingTravel,
    combatEnemies,
    combatActions,
    combatPhase,
    explorationActions,
    addMessage,
    loadCharacter,
    loadStatusEffects,
    loadQuests,
    questActive,
    updateLocation,
    updateHp,
    updateXp,
    updateGold,
    addStatusEffect,
    removeStatusEffectById,
    removeStatusEffect,
    setScene,
    setChaos,
    setAttributeRoll,
    setCreationMode,
    setPendingTravel,
    initCombatEnemies,
    updateCombatEnemyHp,
    markEnemyDefeated,
    clearCombatEnemies,
    setCombatActions,
    setExplorationActions,
    addSuggestions,
    setSuggestInput,
    reset,
  };
});

function isActiveStatusEffect(value: unknown): value is ActiveStatusEffect {
  if (typeof value !== "object" || value === null) return false;
  const record = value as Record<string, unknown>;
  return (
    typeof record.id === "number" &&
    typeof record.entity_id === "string" &&
    typeof record.effect_id === "string" &&
    typeof record.applied_at_tick === "number" &&
    (typeof record.expires_at_tick === "number" ||
      record.expires_at_tick === null) &&
    (typeof record.source === "string" || record.source === null) &&
    typeof record.data === "object" &&
    record.data !== null
  );
}

function effectLabel(effect: ActiveStatusEffect): string {
  const parts = effect.effect_id.split(".");
  return effect.name ?? parts[parts.length - 1] ?? effect.effect_id;
}
