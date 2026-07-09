/**
 * 104b spec — oracle, combat event reducers.
 *
 * Covers: oracle.chaos_changed, combat.actions, combat.attack, combat.save,
 * exploration.actions, gm.suggestions, dungeon.enter_room.
 *
 * Extension tests for combat.start, combat.enemy_defeated, and gm.scene_change
 * live in reducers-b-extensions.spec.ts.
 */
import { describe, it, expect, beforeEach } from "vitest";
import { createWorldModel, apply, type WorldModel } from "./model";
import "./reducers"; // side-effect: populates reducerRegistry
import { seededCharacter, applyAndCollect } from "./reducers-b.spec";

// ---------------------------------------------------------------------------
// oracle.chaos_changed
// ---------------------------------------------------------------------------

describe("apply — oracle.chaos_changed", () => {
  it("updates model.chaos", () => {
    const model = createWorldModel();
    apply(model, {
      event_type: "oracle.chaos_changed",
      data: { old_value: 5, new_value: 6, chaos_factor: 6 },
    });
    expect(model.chaos).toBe(6);
  });

  it("emits chaos change and message", () => {
    const model = createWorldModel();
    const changes = applyAndCollect(model, {
      event_type: "oracle.chaos_changed",
      data: { old_value: 5, new_value: 6, chaos_factor: 6 },
    });
    expect(changes).toContainEqual({ kind: "chaos" });
    expect(changes).toContainEqual({
      kind: "message",
      text: "[Oracle] Chaos: 5 → 6",
      style: "chaos",
    });
  });
});

// ---------------------------------------------------------------------------
// combat.actions
// ---------------------------------------------------------------------------

describe("apply — combat.actions", () => {
  it("sets combat.actions and combat.phase", () => {
    const model = createWorldModel();
    apply(model, {
      event_type: "combat.actions",
      data: {
        phase: "active",
        actions: [{ command: "attack", label: "Attack", style: "primary" }],
      },
    });
    expect(model.combat.phase).toBe("active");
    expect(model.combat.actions).toHaveLength(1);
    expect(model.combat.actions[0]?.command).toBe("attack");
  });

  it("emits { kind: 'combat' }", () => {
    const model = createWorldModel();
    const changes = applyAndCollect(model, {
      event_type: "combat.actions",
      data: { phase: "prompt", actions: [] },
    });
    expect(changes).toEqual([{ kind: "combat" }]);
  });
});

// ---------------------------------------------------------------------------
// combat.attack
// ---------------------------------------------------------------------------

describe("apply — combat.attack", () => {
  let model: WorldModel;
  beforeEach(() => {
    model = createWorldModel();
    model.combat.enemies = [
      { entityId: "goblin-1", name: "Goblin", hp: 8, maxHp: 8, defeated: false },
    ];
    model.encounter.entities = [
      { id: "goblin-1", kind: "monster", q: 2, r: 2, name: "Goblin", hp: 8, maxHp: 8, alive: true },
    ];
  });

  it("updates roster HP", () => {
    apply(model, {
      event_type: "combat.attack",
      data: {
        attacker: "Hero", target: "Goblin", attacker_id: "char-1", target_id: "goblin-1",
        weapon: "sword", roll: 14, modifier: 2, total: 16, target_ac: 12,
        hit: true, damage: 5, shock: 0, critical: false,
        target_hp_remaining: 3, target_max_hp: 8,
      },
    });
    expect(model.combat.enemies[0]?.hp).toBe(3);
  });

  it("updates encounter entity HP", () => {
    apply(model, {
      event_type: "combat.attack",
      data: {
        attacker: "Hero", target: "Goblin", attacker_id: "char-1", target_id: "goblin-1",
        weapon: "sword", roll: 14, modifier: 2, total: 16, target_ac: 12,
        hit: true, damage: 5, shock: 0, critical: false,
        target_hp_remaining: 3, target_max_hp: 8,
      },
    });
    expect(model.encounter.entities[0]?.hp).toBe(3);
  });

  it("emits a combat_attack message with hit and damage", () => {
    const changes = applyAndCollect(model, {
      event_type: "combat.attack",
      data: {
        attacker: "Hero", target: "Goblin", attacker_id: "char-1", target_id: "goblin-1",
        weapon: "sword", roll: 14, modifier: 2, total: 16, target_ac: 12,
        hit: true, damage: 5, shock: 0, critical: false,
        target_hp_remaining: 3, target_max_hp: 8,
      },
    });
    const msg = changes.find((c) => c.kind === "message");
    expect(msg).toBeDefined();
    if (msg?.kind === "message") {
      expect(msg.style).toBe("combat_attack");
      expect(msg.text).toContain("Hero attacks Goblin");
      expect(msg.text).toContain("Hit!");
      expect(msg.text).toContain("Damage: 5");
    }
  });

  it("shows CRITICAL label on critical hit", () => {
    const changes = applyAndCollect(model, {
      event_type: "combat.attack",
      data: {
        attacker: "Hero", target: "Goblin", attacker_id: "char-1", target_id: "goblin-1",
        weapon: "sword", roll: 20, modifier: 2, total: 22, target_ac: 12,
        hit: true, damage: 10, shock: 0, critical: true,
        target_hp_remaining: 0, target_max_hp: 8,
      },
    });
    const msg = changes.find((c) => c.kind === "message");
    if (msg?.kind === "message") expect(msg.text).toContain("(CRITICAL)");
  });

  it("shows Shock on a miss with shock damage", () => {
    const changes = applyAndCollect(model, {
      event_type: "combat.attack",
      data: {
        attacker: "Hero", target: "Goblin", attacker_id: "char-1", target_id: "goblin-1",
        weapon: "sword", roll: 5, modifier: 2, total: 7, target_ac: 12,
        hit: false, damage: null, shock: 2, critical: false,
        target_hp_remaining: null, target_max_hp: null,
      },
    });
    const msg = changes.find((c) => c.kind === "message");
    if (msg?.kind === "message") expect(msg.text).toContain("Shock: 2");
  });
});

// ---------------------------------------------------------------------------
// combat.save
// ---------------------------------------------------------------------------

describe("apply — combat.save", () => {
  it("emits a combat_save message", () => {
    const model = createWorldModel();
    const changes = applyAndCollect(model, {
      event_type: "combat.save",
      data: {
        character: "Hero", save_type: "evasion",
        roll: 12, modifier: 1, total: 13, target: 15, passed: false,
      },
    });
    expect(changes).toContainEqual({
      kind: "message",
      text: "evasion Save  [Hero]\n   Roll: 12 + 1 = 13  vs  15  →  Failed",
      style: "combat_save",
    });
  });

  it("shows Passed when passed is true", () => {
    const model = createWorldModel();
    const changes = applyAndCollect(model, {
      event_type: "combat.save",
      data: {
        character: "Hero", save_type: "physical",
        roll: 16, modifier: 0, total: 16, target: 15, passed: true,
      },
    });
    const msg = changes.find((c) => c.kind === "message");
    if (msg?.kind === "message") expect(msg.text).toContain("Passed");
  });
});

// ---------------------------------------------------------------------------
// Extension tests for combat.start, combat.enemy_defeated, gm.scene_change
// are in reducers-b-extensions.spec.ts
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// exploration.actions / gm.suggestions / dungeon.enter_room
// ---------------------------------------------------------------------------

describe("apply — exploration.actions", () => {
  it("sets explorationActions", () => {
    const model = createWorldModel();
    apply(model, {
      event_type: "exploration.actions",
      data: { actions: [{ command: "explore", label: "Explore", style: "default" }] },
    });
    expect(model.explorationActions).toHaveLength(1);
    expect(model.explorationActions[0]?.command).toBe("explore");
  });

  it("emits { kind: 'explorationActions' }", () => {
    const model = createWorldModel();
    const changes = applyAndCollect(model, {
      event_type: "exploration.actions",
      data: { actions: [] },
    });
    expect(changes).toEqual([{ kind: "explorationActions" }]);
  });
});

describe("apply — gm.suggestions", () => {
  it("sets suggestions when commands are non-empty", () => {
    const model = createWorldModel();
    apply(model, {
      event_type: "gm.suggestions",
      data: { commands: ["north", "south", "look"] },
    });
    expect(model.suggestions).toEqual(["north", "south", "look"]);
  });

  it("is a no-op when commands is empty", () => {
    const model = createWorldModel();
    model.suggestions = ["north"];
    const changes = applyAndCollect(model, {
      event_type: "gm.suggestions",
      data: { commands: [] },
    });
    expect(changes).toHaveLength(0);
    expect(model.suggestions).toEqual(["north"]); // unchanged
  });
});

describe("apply — dungeon.enter_room", () => {
  it("sets character location to room type and name", () => {
    const model = createWorldModel();
    model.character = seededCharacter();
    apply(model, {
      event_type: "dungeon.enter_room",
      data: {
        dungeon_id: "d-1", room_id: "r-1",
        room_name: "Bone Chamber", room_type: "chamber",
        direction: "north", first_visit: true,
      },
    });
    expect(model.character!.locationQ).toBe(-1); // eslint-disable-line @typescript-eslint/no-non-null-assertion
    expect(model.character!.terrain).toBe("chamber"); // eslint-disable-line @typescript-eslint/no-non-null-assertion
    expect(model.character!.features).toEqual(["Bone Chamber"]); // eslint-disable-line @typescript-eslint/no-non-null-assertion
  });

  it("is a no-op when character is null", () => {
    const model = createWorldModel();
    const changes = applyAndCollect(model, {
      event_type: "dungeon.enter_room",
      data: {
        dungeon_id: "d-1", room_id: "r-1",
        room_name: "Entry", room_type: "corridor",
        direction: "north", first_visit: true,
      },
    });
    expect(changes).toHaveLength(0);
  });
});
