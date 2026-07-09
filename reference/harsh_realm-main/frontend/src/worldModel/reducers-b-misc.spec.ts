/**
 * 104b spec — shopping, travel, inventory, action.skill_check,
 * social.disposition_change, and suppressed-event tests.
 */
import { describe, it, expect, vi } from "vitest";
import { createWorldModel, apply } from "./model";
import "./reducers"; // side-effect: populates reducerRegistry
import { seededCharacter, applyAndCollect } from "./reducers-b.spec";

// ---------------------------------------------------------------------------
// shopping.purchase
// ---------------------------------------------------------------------------

describe("apply — shopping.purchase", () => {
  it("updates character.gold", () => {
    const model = createWorldModel();
    model.character = seededCharacter(); // gold: 50
    apply(model, {
      event_type: "shopping.purchase",
      data: { item: "Torch", price: 1, gold_remaining: 49 },
    });
    expect(model.character!.gold).toBe(49); // eslint-disable-line @typescript-eslint/no-non-null-assertion
  });

  it("emits character, purchase message, and refetch-character", () => {
    const model = createWorldModel();
    model.character = seededCharacter();
    const changes = applyAndCollect(model, {
      event_type: "shopping.purchase",
      data: { item: "Torch", price: 1, gold_remaining: 49 },
    });
    expect(changes).toContainEqual({ kind: "character" });
    expect(changes).toContainEqual({ kind: "refetch-character" });
    expect(changes).toContainEqual({
      kind: "message",
      text: "[Purchased] Torch  —  1 gp  (Balance: 49 gp)",
      style: "purchase",
    });
  });

  it("still emits message and refetch when character is null", () => {
    const model = createWorldModel();
    const changes = applyAndCollect(model, {
      event_type: "shopping.purchase",
      data: { item: "Sword", price: 40, gold_remaining: 10 },
    });
    expect(changes).toContainEqual({ kind: "refetch-character" });
    expect(changes.some((c) => c.kind === "message")).toBe(true);
    expect(changes.every((c) => c.kind !== "character")).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// shopping.sale
// ---------------------------------------------------------------------------

describe("apply — shopping.sale", () => {
  it("updates character.gold to gold_total", () => {
    const model = createWorldModel();
    model.character = seededCharacter(); // gold: 50
    apply(model, {
      event_type: "shopping.sale",
      data: { item: "Old Shield", price: 5, gold_total: 55 },
    });
    expect(model.character!.gold).toBe(55); // eslint-disable-line @typescript-eslint/no-non-null-assertion
  });

  it("emits sale message with correct text", () => {
    const model = createWorldModel();
    model.character = seededCharacter();
    const changes = applyAndCollect(model, {
      event_type: "shopping.sale",
      data: { item: "Old Shield", price: 5, gold_total: 55 },
    });
    expect(changes).toContainEqual({
      kind: "message",
      text: "[Sold] Old Shield  —  5 gp  (Balance: 55 gp)",
      style: "sale",
    });
    expect(changes).toContainEqual({ kind: "refetch-character" });
  });
});

// ---------------------------------------------------------------------------
// travel.interrupted
// ---------------------------------------------------------------------------

describe("apply — travel.interrupted", () => {
  it("sets pendingTravel to target coordinates", () => {
    const model = createWorldModel();
    apply(model, {
      event_type: "travel.interrupted",
      data: { target_q: 7, target_r: 3, reason: "encounter" },
    });
    expect(model.pendingTravel).toEqual({ q: 7, r: 3 });
  });

  it("emits { kind: 'pendingTravel' }", () => {
    const model = createWorldModel();
    const changes = applyAndCollect(model, {
      event_type: "travel.interrupted",
      data: { target_q: 7, target_r: 3, reason: "encounter" },
    });
    expect(changes).toEqual([{ kind: "pendingTravel" }]);
  });
});

// ---------------------------------------------------------------------------
// travel clear events
// ---------------------------------------------------------------------------

describe("apply — travel.resumed clears pendingTravel", () => {
  it("sets pendingTravel to null", () => {
    const model = createWorldModel();
    model.pendingTravel = { q: 5, r: 5 };
    apply(model, {
      event_type: "travel.resumed",
      data: { target_q: 5, target_r: 5, steps_remaining: 3 },
    });
    expect(model.pendingTravel).toBeNull();
  });
});

describe("apply — travel.started clears pendingTravel", () => {
  it("sets pendingTravel to null", () => {
    const model = createWorldModel();
    model.pendingTravel = { q: 5, r: 5 };
    apply(model, {
      event_type: "travel.started",
      data: { target_q: 5, target_r: 5, steps_remaining: 3 },
    });
    expect(model.pendingTravel).toBeNull();
  });
});

describe("apply — travel.completed clears pendingTravel", () => {
  it("sets pendingTravel to null", () => {
    const model = createWorldModel();
    model.pendingTravel = { q: 5, r: 5 };
    apply(model, { event_type: "travel.completed", data: { target_q: 5, target_r: 5 } });
    expect(model.pendingTravel).toBeNull();
  });
});

describe("apply — travel.cancelled clears pendingTravel", () => {
  it("sets pendingTravel to null", () => {
    const model = createWorldModel();
    model.pendingTravel = { q: 5, r: 5 };
    apply(model, { event_type: "travel.cancelled", data: { target_q: 5, target_r: 5 } });
    expect(model.pendingTravel).toBeNull();
  });
});

describe("apply — travel.blocked clears pendingTravel", () => {
  it("sets pendingTravel to null", () => {
    const model = createWorldModel();
    model.pendingTravel = { q: 5, r: 5 };
    apply(model, { event_type: "travel.blocked", data: { target_q: 5, target_r: 5 } });
    expect(model.pendingTravel).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// inventory.item_given / item_lost / ammo_consumed
// ---------------------------------------------------------------------------

describe("apply — inventory.item_given", () => {
  it("emits inventory message for a regular item", () => {
    const model = createWorldModel();
    const changes = applyAndCollect(model, {
      event_type: "inventory.item_given",
      data: { character_id: "char-1", item: { type: "weapon", name: "Short Sword" } },
    });
    expect(changes).toContainEqual({
      kind: "message",
      text: "You obtained Short Sword.",
      style: "inventory",
    });
    expect(changes).toContainEqual({ kind: "refetch-character" });
  });

  it("shows gold amount for currency items", () => {
    const model = createWorldModel();
    const changes = applyAndCollect(model, {
      event_type: "inventory.item_given",
      data: { character_id: "char-1", item: { type: "currency", value: 25 } },
    });
    expect(changes).toContainEqual({
      kind: "message",
      text: "You received 25 gp.",
      style: "inventory",
    });
  });
});

describe("apply — inventory.item_lost", () => {
  it("emits inventory message and refetch-character", () => {
    const model = createWorldModel();
    const changes = applyAndCollect(model, {
      event_type: "inventory.item_lost",
      data: { character_id: "char-1", item_name: "Rope" },
    });
    expect(changes).toContainEqual({
      kind: "message",
      text: "You lost Rope.",
      style: "inventory",
    });
    expect(changes).toContainEqual({ kind: "refetch-character" });
  });
});

describe("apply — inventory.ammo_consumed", () => {
  it("emits inventory message with ammo type and remaining count", () => {
    const model = createWorldModel();
    const changes = applyAndCollect(model, {
      event_type: "inventory.ammo_consumed",
      data: { character_id: "char-1", ammo_type: "arrow", remaining: 7 },
    });
    expect(changes).toContainEqual({
      kind: "message",
      text: "Ammo spent: arrow (7 remaining).",
      style: "inventory",
    });
    expect(changes).toContainEqual({ kind: "refetch-character" });
  });
});

// ---------------------------------------------------------------------------
// action.skill_check
// ---------------------------------------------------------------------------

describe("apply — action.skill_check", () => {
  it("emits a skill_check message with correct format", () => {
    const model = createWorldModel();
    const changes = applyAndCollect(model, {
      event_type: "action.skill_check",
      data: {
        verb: "persuade",
        skill: "Persuade",
        attribute: "cha",
        roll: 9,
        total: 11,
        difficulty: 10,
        margin: 1,
        success: true,
        outcome_key: "success",
        disposition_delta: 1,
        npc_entity_id: "npc-1",
      },
    });
    expect(changes).toContainEqual({
      kind: "message",
      text: "[Skill Check] persuade (Persuade)\nRoll: 9 + 2 mod = 11  vs  Difficulty 10  →  Success (+1)",
      style: "skill_check",
    });
  });

  it("shows Fail and negative margin", () => {
    const model = createWorldModel();
    const changes = applyAndCollect(model, {
      event_type: "action.skill_check",
      data: {
        verb: "deceive",
        skill: "Deceive",
        attribute: "cha",
        roll: 4,
        total: 5,
        difficulty: 10,
        margin: -5,
        success: false,
        outcome_key: "fail",
        disposition_delta: -1,
        npc_entity_id: null,
      },
    });
    const msg = changes.find((c) => c.kind === "message");
    if (msg?.kind === "message") {
      expect(msg.text).toContain("Fail");
      expect(msg.text).toContain("-5");
    }
  });
});

// ---------------------------------------------------------------------------
// social.disposition_change
// ---------------------------------------------------------------------------

describe("apply — social.disposition_change", () => {
  it("emits a disposition message with delta", () => {
    const model = createWorldModel();
    const changes = applyAndCollect(model, {
      event_type: "social.disposition_change",
      data: {
        entity_id: "npc-1",
        npc: "Innkeeper",
        old_score: 0,
        new_score: 1,
        old_mood: "indifferent",
        new_mood: "friendly",
        reason: "persuasion",
      },
    });
    expect(changes).toContainEqual({
      kind: "message",
      text: "Innkeeper: indifferent → friendly  (+1)",
      style: "disposition",
    });
  });

  it("shows negative delta correctly", () => {
    const model = createWorldModel();
    const changes = applyAndCollect(model, {
      event_type: "social.disposition_change",
      data: {
        entity_id: "npc-1",
        npc: "Guard",
        old_score: 1,
        new_score: -2,
        old_mood: "friendly",
        new_mood: "hostile",
        reason: "deception",
      },
    });
    const msg = changes.find((c) => c.kind === "message");
    if (msg?.kind === "message") expect(msg.text).toContain("-3");
  });
});

// ---------------------------------------------------------------------------
// Suppressed event — no reducer registered, dev warning emitted
// ---------------------------------------------------------------------------

describe("suppressed event — oracle.fate_check", () => {
  it("does not throw and stays quiet (suppressed, no dev warning) — D-6", () => {
    const model = createWorldModel();
    const warnSpy = vi
      .spyOn(console, "warn")
      .mockImplementation((): void => { /* suppress */ });

    expect(() =>
      apply(model, {
        event_type: "oracle.fate_check",
        data: {
          question: "Will the guard patrol?",
          likelihood: "likely",
          chaos_factor: 5,
          roll: 42,
        },
      }),
    ).not.toThrow();
    // oracle.fate_check is in SUPPRESSED_GAME_EVENTS — it must NOT warn.
    expect(warnSpy).not.toHaveBeenCalled();
    warnSpy.mockRestore();
  });
});
