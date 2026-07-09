/**
 * 104b spec — character events and status effects.
 *
 * Covers: character.hp_changed, combat.player_hit, character.xp_gained,
 * character.level_up, character.respawn, character.created, character.death,
 * character.death_final, character.expert_reroll,
 * status.applied, status.removed, status.expired.
 */
import { describe, it, expect, beforeEach } from "vitest";
import {
  createWorldModel,
  apply,
  subscribe,
  type WorldModel,
  type Change,
  type CharacterModel,
} from "./model";
import "./reducers"; // side-effect: populates reducerRegistry

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

export function seededCharacter(): CharacterModel {
  return {
    id: "char-1",
    name: "Hero",
    characterClass: "Warrior",
    level: 1,
    hp: 10,
    maxHp: 10,
    ac: 12,
    xp: 0,
    xpNext: 100,
    gold: 50,
    str: 14,
    locationQ: 0,
    locationR: 0,
    terrain: "plains",
    features: [],
    weather: null,
    conditions: [],
    statusEffects: [],
    equipment: [],
  };
}

export function applyAndCollect(
  model: WorldModel,
  event: Parameters<typeof apply>[1],
): Change[] {
  const changes: Change[] = [];
  const unsub = subscribe(model, (c) => changes.push(c));
  apply(model, event);
  unsub();
  return changes;
}

// ---------------------------------------------------------------------------
// character.hp_changed
// ---------------------------------------------------------------------------

describe("apply — character.hp_changed", () => {
  it("updates character hp and maxHp", () => {
    const model = createWorldModel();
    model.character = seededCharacter();
    apply(model, { event_type: "character.hp_changed", data: { hp: 6, max_hp: 12 } });
    expect(model.character.hp).toBe(6);
    expect(model.character.maxHp).toBe(12);
  });

  it("emits { kind: 'character' }", () => {
    const model = createWorldModel();
    model.character = seededCharacter();
    const changes = applyAndCollect(model, {
      event_type: "character.hp_changed",
      data: { hp: 5, max_hp: 10 },
    });
    expect(changes).toEqual([{ kind: "character" }]);
  });

  it("is a no-op when character is null", () => {
    const model = createWorldModel();
    const changes = applyAndCollect(model, {
      event_type: "character.hp_changed",
      data: { hp: 5, max_hp: 10 },
    });
    expect(changes).toHaveLength(0);
    expect(model.character).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// combat.player_hit
// ---------------------------------------------------------------------------

describe("apply — combat.player_hit", () => {
  let model: WorldModel;
  beforeEach(() => {
    model = createWorldModel();
    model.character = seededCharacter();
  });

  it("updates character hp and maxHp", () => {
    apply(model, {
      event_type: "combat.player_hit",
      data: {
        attacker: "Goblin",
        damage: 4,
        player_hp: 6,
        player_max_hp: 10,
        player_id: "char-1",
        player_name: "Hero",
        player_alive: true,
      },
    });
    // Non-null asserted: model.character set in beforeEach.
    expect(model.character!.hp).toBe(6); // eslint-disable-line @typescript-eslint/no-non-null-assertion
    expect(model.character!.maxHp).toBe(10); // eslint-disable-line @typescript-eslint/no-non-null-assertion
  });

  it("mirrors PC HP into encounter entity when player_id is present", () => {
    model.encounter.entities = [
      { id: "char-1", kind: "pc", q: 4, r: 4, name: "Hero", hp: 10, maxHp: 10, alive: true },
    ];
    apply(model, {
      event_type: "combat.player_hit",
      data: {
        attacker: "Goblin",
        damage: 3,
        player_hp: 7,
        player_max_hp: 10,
        player_id: "char-1",
        player_name: "Hero",
        player_alive: true,
      },
    });
    expect(model.encounter.entities[0]?.hp).toBe(7);
  });

  it("emits no message — pure state update", () => {
    const changes = applyAndCollect(model, {
      event_type: "combat.player_hit",
      data: {
        attacker: "Goblin",
        damage: 2,
        player_hp: 8,
        player_max_hp: 10,
        player_id: "char-1",
        player_name: "Hero",
        player_alive: true,
      },
    });
    expect(changes.every((c) => c.kind !== "message")).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// character.xp_gained
// ---------------------------------------------------------------------------

describe("apply — character.xp_gained", () => {
  it("updates xp and xpNext", () => {
    const model = createWorldModel();
    model.character = seededCharacter();
    apply(model, {
      event_type: "character.xp_gained",
      data: { xp_gained: 30, new_total: 30, xp_next: 100 },
    });
    expect(model.character!.xp).toBe(30); // eslint-disable-line @typescript-eslint/no-non-null-assertion
    expect(model.character!.xpNext).toBe(100); // eslint-disable-line @typescript-eslint/no-non-null-assertion
  });

  it("is a no-op when character is null", () => {
    const model = createWorldModel();
    const changes = applyAndCollect(model, {
      event_type: "character.xp_gained",
      data: { xp_gained: 30, new_total: 30, xp_next: 100 },
    });
    expect(changes).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// character.level_up / respawn / created
// ---------------------------------------------------------------------------

describe("apply — character.level_up", () => {
  it("emits refetch-character", () => {
    const model = createWorldModel();
    const changes = applyAndCollect(model, {
      event_type: "character.level_up",
      data: { new_level: 2, new_max_hp: 14, hp_gained: 4 },
    });
    expect(changes).toContainEqual({ kind: "refetch-character" });
  });
});

describe("apply — character.respawn", () => {
  it("emits refetch-character (not refetch-map)", () => {
    const model = createWorldModel();
    const changes = applyAndCollect(model, {
      event_type: "character.respawn",
      data: {
        name: "Hero",
        settlement_q: 0,
        settlement_r: 0,
        new_hp: 8,
        xp_lost: 10,
        item_lost: null,
      },
    });
    expect(changes).toContainEqual({ kind: "refetch-character" });
    expect(changes).not.toContainEqual({ kind: "refetch-map" });
  });
});

describe("apply — character.created", () => {
  it("emits refetch-character AND refetch-map", () => {
    const model = createWorldModel();
    const changes = applyAndCollect(model, {
      event_type: "character.created",
      data: { character_id: "char-1" },
    });
    expect(changes).toContainEqual({ kind: "refetch-character" });
    expect(changes).toContainEqual({ kind: "refetch-map" });
  });

  it("emits clear-attribute-roll to dismiss the chat-creation modal (D-3)", () => {
    const model = createWorldModel();
    const changes = applyAndCollect(model, {
      event_type: "character.created",
      data: { character_id: "char-1" },
    });
    expect(changes).toContainEqual({ kind: "clear-attribute-roll" });
  });
});

// ---------------------------------------------------------------------------
// character.death / death_final / expert_reroll
// ---------------------------------------------------------------------------

describe("apply — character.death", () => {
  it("emits a death message", () => {
    const model = createWorldModel();
    const changes = applyAndCollect(model, {
      event_type: "character.death",
      data: { position_q: 3, position_r: 5 },
    });
    expect(changes).toContainEqual({
      kind: "message",
      text: "You have fallen in battle!",
      style: "death",
    });
  });
});

describe("apply — character.death_final", () => {
  it("emits death message with character name", () => {
    const model = createWorldModel();
    const changes = applyAndCollect(model, {
      event_type: "character.death_final",
      data: { name: "Aldric" },
    });
    expect(changes).toContainEqual({
      kind: "message",
      text: "Aldric has died. Their story ends here.",
      style: "death",
    });
  });
});

describe("apply — character.expert_reroll", () => {
  it("emits a reroll message with original and reroll totals", () => {
    const model = createWorldModel();
    const changes = applyAndCollect(model, {
      event_type: "character.expert_reroll",
      data: { verb: "persuade", original_total: 8, reroll_total: 11, used: "reroll" },
    });
    expect(changes).toContainEqual({
      kind: "message",
      text: "[Expert Reroll] Original: 8 → Reroll: 11  (new result used)",
      style: "reroll",
    });
  });
});

// ---------------------------------------------------------------------------
// status.applied / removed / expired
// ---------------------------------------------------------------------------

const stubEffect = {
  id: 1,
  entity_id: "char-1",
  effect_id: "conditions.stunned",
  applied_at_tick: 100,
  expires_at_tick: 110,
  source: "goblin",
  data: {},
};

describe("apply — status.applied", () => {
  it("adds a new status effect", () => {
    const model = createWorldModel();
    model.character = seededCharacter();
    apply(model, { event_type: "status.applied", data: { active_effect: stubEffect } });
    expect(model.character!.statusEffects).toHaveLength(1); // eslint-disable-line @typescript-eslint/no-non-null-assertion
    expect(model.character!.statusEffects[0]?.effect_id).toBe("conditions.stunned"); // eslint-disable-line @typescript-eslint/no-non-null-assertion
  });

  it("replaces an existing effect with the same id", () => {
    const model = createWorldModel();
    model.character = seededCharacter();
    model.character.statusEffects = [{ ...stubEffect, expires_at_tick: 105 }];
    apply(model, {
      event_type: "status.applied",
      data: { active_effect: { ...stubEffect, expires_at_tick: 120 } },
    });
    expect(model.character!.statusEffects).toHaveLength(1); // eslint-disable-line @typescript-eslint/no-non-null-assertion
    expect(model.character!.statusEffects[0]?.expires_at_tick).toBe(120); // eslint-disable-line @typescript-eslint/no-non-null-assertion
  });

  it("updates conditions array", () => {
    const model = createWorldModel();
    model.character = seededCharacter();
    apply(model, { event_type: "status.applied", data: { active_effect: stubEffect } });
    expect(model.character!.conditions).toContain("stunned"); // eslint-disable-line @typescript-eslint/no-non-null-assertion
  });

  it("is a no-op when entity_id does not match character.id", () => {
    const model = createWorldModel();
    model.character = seededCharacter(); // id: "char-1"
    apply(model, {
      event_type: "status.applied",
      data: { active_effect: { ...stubEffect, entity_id: "other-char" } },
    });
    expect(model.character!.statusEffects).toHaveLength(0); // eslint-disable-line @typescript-eslint/no-non-null-assertion
  });
});

describe("apply — status.removed", () => {
  it("removes the effect matching effect_id", () => {
    const model = createWorldModel();
    model.character = seededCharacter();
    model.character.statusEffects = [{ ...stubEffect }];
    apply(model, {
      event_type: "status.removed",
      data: { entity_id: "char-1", effect_id: "conditions.stunned", removed_count: 1 },
    });
    expect(model.character!.statusEffects).toHaveLength(0); // eslint-disable-line @typescript-eslint/no-non-null-assertion
  });

  it("is a no-op when entity_id does not match", () => {
    const model = createWorldModel();
    model.character = seededCharacter();
    model.character.statusEffects = [{ ...stubEffect }];
    apply(model, {
      event_type: "status.removed",
      data: { entity_id: "other-char", effect_id: "conditions.stunned", removed_count: 1 },
    });
    expect(model.character!.statusEffects).toHaveLength(1); // eslint-disable-line @typescript-eslint/no-non-null-assertion
  });
});

describe("apply — status.expired", () => {
  it("removes the effect matching the numeric id", () => {
    const model = createWorldModel();
    model.character = seededCharacter();
    model.character.statusEffects = [{ ...stubEffect }]; // id: 1
    apply(model, { event_type: "status.expired", data: { active_effect: stubEffect } });
    expect(model.character!.statusEffects).toHaveLength(0); // eslint-disable-line @typescript-eslint/no-non-null-assertion
  });
});
