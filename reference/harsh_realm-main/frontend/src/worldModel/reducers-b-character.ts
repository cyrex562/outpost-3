/**
 * 104b reducers — character events and status effects.
 *
 * Side-effect imported by `reducers-b.ts`; do not import directly.
 *
 * Covers:
 *   character.hp_changed   combat.player_hit      character.xp_gained
 *   character.level_up     character.respawn       character.created
 *   character.death        character.death_final   character.expert_reroll
 *   status.applied         status.removed          status.expired
 */
import type {
  CharacterHpChangedNotice,
  CharacterXpGainedNotice,
  CharacterLevelUpNotice,
  CharacterRespawnNotice,
  CharacterCreatedNotice,
  CharacterDeathNotice,
  CharacterDeathFinalNotice,
  CharacterExpertRerollNotice,
  CombatPlayerHitNotice,
  StatusAppliedNotice,
  StatusRemovedNotice,
  StatusExpiredNotice,
} from "../types/events.gen";
import {
  reducerRegistry,
  type WorldModel,
  type Change,
  type StatusEffectModel,
} from "./model";

// ---------------------------------------------------------------------------
// Internal helper
// ---------------------------------------------------------------------------

/** Derives a display label from a status-effect payload (mirrors effectLabel in game.ts). */
function statusEffectLabel(effect: StatusEffectModel): string {
  // Prefer the display `name` if the payload carries one (matches the old
  // effectLabel: `effect.name ?? lastIdSegment ?? effect_id`). `name` is not on
  // the generated payload type, so read it defensively.
  const name = (effect as Record<string, unknown>).name;
  if (typeof name === "string") return name;
  const parts = effect.effect_id.split(".");
  return parts[parts.length - 1] ?? effect.effect_id;
}

// ---------------------------------------------------------------------------
// character.hp_changed
// ---------------------------------------------------------------------------

reducerRegistry["character.hp_changed"] = (
  model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "character.hp_changed".
  const d = data as CharacterHpChangedNotice;
  if (model.character === null) return [];
  model.character.hp = d.hp;
  model.character.maxHp = d.max_hp;
  return [{ kind: "character" }];
};

// ---------------------------------------------------------------------------
// combat.player_hit — updates character HP; mirrors PC HP into encounter grid
// ---------------------------------------------------------------------------

reducerRegistry["combat.player_hit"] = (
  model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "combat.player_hit".
  // Mirrors the combined `character.hp_changed || combat.player_hit` branch in
  // _websocketHandlers.ts — no chat message is generated; this is pure state.
  const d = data as CombatPlayerHitNotice;
  const changes: Change[] = [];

  if (model.character !== null) {
    model.character.hp = d.player_hp;
    model.character.maxHp = d.player_max_hp;
    changes.push({ kind: "character" });
  }

  // Mirror the PC's HP into the encounter grid (keyed by the real combatant id).
  if (d.player_id !== null) {
    const encIdx = model.encounter.entities.findIndex((e) => e.id === d.player_id);
    if (encIdx !== -1) {
      model.encounter.entities[encIdx] = {
        ...model.encounter.entities[encIdx],
        hp: d.player_hp,
        maxHp: d.player_max_hp,
      };
      changes.push({ kind: "encounter" });
    }
  }

  return changes;
};

// ---------------------------------------------------------------------------
// character.xp_gained
// ---------------------------------------------------------------------------

reducerRegistry["character.xp_gained"] = (
  model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "character.xp_gained".
  const d = data as CharacterXpGainedNotice;
  if (model.character === null) return [];
  model.character.xp = d.new_total;
  model.character.xpNext = d.xp_next;
  return [{ kind: "character" }];
};

// ---------------------------------------------------------------------------
// character.level_up — full state refetch; HP max changes
// ---------------------------------------------------------------------------

reducerRegistry["character.level_up"] = (
  _model: WorldModel,
  _data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "character.level_up".
  // Handler calls gameStore.loadCharacter(); no message emitted.
  void (_data as CharacterLevelUpNotice);
  return [{ kind: "refetch-character" }];
};

// ---------------------------------------------------------------------------
// character.respawn — full state refetch after respawn
// ---------------------------------------------------------------------------

reducerRegistry["character.respawn"] = (
  _model: WorldModel,
  _data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "character.respawn".
  // Handler calls gameStore.loadCharacter(); no message emitted.
  void (_data as CharacterRespawnNotice);
  return [{ kind: "refetch-character" }];
};

// ---------------------------------------------------------------------------
// character.created — refetch character and map
// ---------------------------------------------------------------------------

reducerRegistry["character.created"] = (
  _model: WorldModel,
  _data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "character.created".
  // Handler calls gameStore.loadCharacter() + mapStore.loadMap() and dismisses
  // the chat-creation attribute-roll modal (setAttributeRoll(null)).
  void (_data as CharacterCreatedNotice);
  return [
    { kind: "refetch-character" },
    { kind: "refetch-map" },
    { kind: "clear-attribute-roll" },
  ];
};

// ---------------------------------------------------------------------------
// character.death — "You have fallen in battle!"
// ---------------------------------------------------------------------------

reducerRegistry["character.death"] = (
  _model: WorldModel,
  _data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "character.death".
  void (_data as CharacterDeathNotice);
  return [{ kind: "message", text: "You have fallen in battle!", style: "death" }];
};

// ---------------------------------------------------------------------------
// character.death_final — permadeath notice
// ---------------------------------------------------------------------------

reducerRegistry["character.death_final"] = (
  _model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "character.death_final".
  const d = data as CharacterDeathFinalNotice;
  return [
    {
      kind: "message",
      text: `${d.name} has died. Their story ends here.`,
      style: "death",
    },
  ];
};

// ---------------------------------------------------------------------------
// character.expert_reroll
// ---------------------------------------------------------------------------

reducerRegistry["character.expert_reroll"] = (
  _model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "character.expert_reroll".
  const d = data as CharacterExpertRerollNotice;
  return [
    {
      kind: "message",
      text: `[Expert Reroll] Original: ${d.original_total} → Reroll: ${d.reroll_total}  (new result used)`,
      style: "reroll",
    },
  ];
};

// ---------------------------------------------------------------------------
// status.applied
// ---------------------------------------------------------------------------

reducerRegistry["status.applied"] = (
  model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "status.applied".
  const d = data as StatusAppliedNotice;
  const effect = d.active_effect;
  if (model.character === null || model.character.id !== effect.entity_id) {
    return [];
  }
  const existingIdx = model.character.statusEffects.findIndex(
    (e) => e.id === effect.id,
  );
  if (existingIdx >= 0) {
    model.character.statusEffects.splice(existingIdx, 1, effect);
  } else {
    model.character.statusEffects.push(effect);
  }
  model.character.conditions = model.character.statusEffects.map(statusEffectLabel);
  return [{ kind: "character" }];
};

// ---------------------------------------------------------------------------
// status.removed
// ---------------------------------------------------------------------------

reducerRegistry["status.removed"] = (
  model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "status.removed".
  const d = data as StatusRemovedNotice;
  if (model.character === null || model.character.id !== d.entity_id) {
    return [];
  }
  model.character.statusEffects = model.character.statusEffects.filter(
    (e) => e.effect_id !== d.effect_id,
  );
  model.character.conditions = model.character.statusEffects.map(statusEffectLabel);
  return [{ kind: "character" }];
};

// ---------------------------------------------------------------------------
// status.expired
// ---------------------------------------------------------------------------

reducerRegistry["status.expired"] = (
  model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "status.expired".
  // Mirrors removeStatusEffectById — removes by numeric row id, no entity check.
  const d = data as StatusExpiredNotice;
  if (model.character === null) return [];
  const targetId = d.active_effect.id;
  model.character.statusEffects = model.character.statusEffects.filter(
    (e) => e.id !== targetId,
  );
  model.character.conditions = model.character.statusEffects.map(statusEffectLabel);
  return [{ kind: "character" }];
};
