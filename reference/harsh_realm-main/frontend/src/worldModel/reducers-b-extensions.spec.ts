/**
 * 104b spec — 104a reducer extensions:
 *   combat.start   (104b: also populates model.combat.enemies)
 *   combat.enemy_defeated  (104b: also marks model.combat.enemies[].defeated)
 *   gm.scene_change  (104b: clears model.combat; emits refetch-character)
 */
import { describe, it, expect } from "vitest";
import { createWorldModel, apply, type WorldModel } from "./model";
import "./reducers"; // side-effect: populates reducerRegistry
import { applyAndCollect } from "./reducers-b.spec";

function seedCombat(model: WorldModel) {
  apply(model, {
    event_type: "combat.start",
    data: {
      awareness: "normal",
      enemies: ["Goblin"],
      enemy_states: [{ entity_id: "g-1", name: "Goblin", hp: 5, max_hp: 5 }],
    },
  });
}

// ---------------------------------------------------------------------------
// combat.start extension (104b: also sets model.combat.enemies)
// ---------------------------------------------------------------------------

describe("apply — combat.start (104b extension)", () => {
  it("populates model.combat.enemies from enemy_states", () => {
    const model = createWorldModel();
    apply(model, {
      event_type: "combat.start",
      data: {
        awareness: "normal",
        enemies: ["Goblin"],
        enemy_states: [{ entity_id: "g-1", name: "Goblin", hp: 5, max_hp: 5 }],
      },
    });
    expect(model.combat.enemies).toHaveLength(1);
    expect(model.combat.enemies[0]?.entityId).toBe("g-1");
    expect(model.combat.enemies[0]?.defeated).toBe(false);
  });

  it("resets actions and phase", () => {
    const model = createWorldModel();
    model.combat.phase = "active";
    model.combat.actions = [{ command: "attack", label: "Attack", style: "primary" }];
    apply(model, {
      event_type: "combat.start",
      data: { awareness: "normal", enemies: [], enemy_states: [] },
    });
    expect(model.combat.phase).toBe("");
    expect(model.combat.actions).toHaveLength(0);
  });

  it("still populates encounter.entities (104a behaviour preserved)", () => {
    const model = createWorldModel();
    apply(model, {
      event_type: "combat.start",
      data: {
        awareness: "normal",
        enemies: ["Orc"],
        enemy_states: [{ entity_id: "o-1", name: "Orc", hp: 12, max_hp: 12 }],
      },
    });
    expect(model.encounter.entities).toHaveLength(1);
    expect(model.encounter.entities[0]?.id).toBe("o-1");
  });

  it("emits encounter and combat changes", () => {
    const model = createWorldModel();
    const changes = applyAndCollect(model, {
      event_type: "combat.start",
      data: {
        awareness: "normal",
        enemies: ["Goblin"],
        enemy_states: [{ entity_id: "g-1", name: "Goblin", hp: 5, max_hp: 5 }],
      },
    });
    expect(changes).toContainEqual({ kind: "encounter" });
    expect(changes).toContainEqual({ kind: "combat" });
  });
});

// ---------------------------------------------------------------------------
// combat.enemy_defeated extension (104b: also marks combat.enemies[].defeated)
// ---------------------------------------------------------------------------

describe("apply — combat.enemy_defeated (104b extension)", () => {
  it("marks combat.enemies entry as defeated with hp=0", () => {
    const model = createWorldModel();
    seedCombat(model);
    apply(model, { event_type: "combat.enemy_defeated", data: { entity_id: "g-1", name: "Goblin" } });
    const goblin = model.combat.enemies.find((e) => e.entityId === "g-1");
    expect(goblin?.defeated).toBe(true);
    expect(goblin?.hp).toBe(0);
  });

  it("also sets encounter entity alive=false (104a behaviour preserved)", () => {
    const model = createWorldModel();
    seedCombat(model);
    apply(model, { event_type: "combat.enemy_defeated", data: { entity_id: "g-1", name: "Goblin" } });
    const encEntity = model.encounter.entities.find((e) => e.id === "g-1");
    expect(encEntity?.alive).toBe(false);
  });

  it("emits combat change in addition to encounter", () => {
    const model = createWorldModel();
    seedCombat(model);
    const changes = applyAndCollect(model, {
      event_type: "combat.enemy_defeated",
      data: { entity_id: "g-1", name: "Goblin" },
    });
    expect(changes).toContainEqual({ kind: "encounter" });
    expect(changes).toContainEqual({ kind: "combat" });
  });
});

// ---------------------------------------------------------------------------
// gm.scene_change extension (104b: clears model.combat; emits refetch-character)
// ---------------------------------------------------------------------------

describe("apply — gm.scene_change (104b extension)", () => {
  it("clears model.combat when leaving combat", () => {
    const model = createWorldModel();
    seedCombat(model);
    expect(model.combat.enemies).toHaveLength(1);
    apply(model, { event_type: "gm.scene_change", data: { to: "exploration" } });
    expect(model.combat.enemies).toHaveLength(0);
    expect(model.combat.actions).toHaveLength(0);
    expect(model.combat.phase).toBe("");
  });

  it("preserves model.combat when staying in combat scene", () => {
    const model = createWorldModel();
    seedCombat(model);
    apply(model, { event_type: "gm.scene_change", data: { to: "combat" } });
    expect(model.combat.enemies).toHaveLength(1);
  });

  it("emits refetch-character effect", () => {
    const model = createWorldModel();
    const changes = applyAndCollect(model, {
      event_type: "gm.scene_change",
      data: { to: "exploration" },
    });
    expect(changes).toContainEqual({ kind: "refetch-character" });
  });
});
