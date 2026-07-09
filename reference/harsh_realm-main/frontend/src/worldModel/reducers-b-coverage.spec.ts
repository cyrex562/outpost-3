/**
 * 104b coverage test — asserts that every non-suppressed CLIENT_FACING_EVENT_TYPE
 * has a registered reducer, and that suppressed events have none.
 *
 * The `ALL_CLIENT_EVENT_TYPES` list mirrors the `ClientEvent` union in
 * `events.gen.ts`.  Keep it in sync with that union and with the backend's
 * `CLIENT_FACING_EVENT_TYPES` constant.
 */
import { describe, it, expect } from "vitest";
import { reducerRegistry } from "./model";
import { SUPPRESSED_GAME_EVENTS } from "./suppressed";
import "./reducers"; // side-effect: populates reducerRegistry

/**
 * All event types in the `ClientEvent` discriminated union (events.gen.ts).
 * Must stay in sync with the backend `CLIENT_FACING_EVENT_TYPES`.
 */
const ALL_CLIENT_EVENT_TYPES: ReadonlyArray<string> = [
  "action.skill_check",
  "character.created",
  "character.death",
  "character.death_final",
  "character.expert_reroll",
  "character.hp_changed",
  "character.level_up",
  "character.respawn",
  "character.xp_gained",
  "combat.actions",
  "combat.attack",
  "combat.enemy_defeated",
  "combat.fled",           // SUPPRESSED
  "combat.player_hit",
  "combat.positions",
  "combat.save",
  "combat.start",
  "dungeon.enter_room",
  "exploration.actions",
  "exploration.enter_hex", // SUPPRESSED
  "exploration.moved",
  "exploration.revealed",
  "gm.scene_change",
  "gm.suggestions",
  "inventory.ammo_consumed",
  "inventory.item_given",
  "inventory.item_lost",
  "loot.source_revealed",
  "oracle.chaos_changed",
  "oracle.fate_check",     // SUPPRESSED
  "oracle.random_event",   // SUPPRESSED
  "oracle.scene_check",    // SUPPRESSED
  "quest.accepted",
  "quest.completed",
  "quest.failed",
  "quest.progress_updated",
  "shopping.purchase",
  "shopping.sale",
  "social.disposition_change",
  "social.healer",         // SUPPRESSED
  "status.applied",
  "status.expired",
  "status.removed",
  "town.map",
  "town.move",
  "travel.blocked",
  "travel.cancelled",
  "travel.completed",
  "travel.interrupted",
  "travel.resumed",
  "travel.started",
  "travel.step",
];

/**
 * Events that produce no client-visible output — `apply()` intentionally has
 * no reducer for them.  Sourced from the single canonical list in
 * `suppressed.ts` (which the Rust coverage gate also reads), so there is no
 * duplicated hardcoded set to drift.
 *
 * (`gm.narrate` is in `SUPPRESSED_GAME_EVENTS` but is not a `game_event`
 * frame and is absent from `ClientEvent`/`ALL_CLIENT_EVENT_TYPES`; it simply
 * never matches the test-1 loop and has no reducer for test 2.)
 */
const SUPPRESSED_EVENT_TYPES: ReadonlySet<string> = new Set(
  SUPPRESSED_GAME_EVENTS,
);

describe("reducerRegistry coverage", () => {
  it("has a reducer for every non-suppressed CLIENT_FACING_EVENT_TYPE", () => {
    const missing: string[] = [];
    for (const evType of ALL_CLIENT_EVENT_TYPES) {
      if (SUPPRESSED_EVENT_TYPES.has(evType)) continue;
      if (!(evType in reducerRegistry)) {
        missing.push(evType);
      }
    }
    expect(missing).toEqual([]);
  });

  it("has NO reducer for each suppressed event type", () => {
    // Validates that suppressed events stay suppressed (no accidental registration).
    for (const evType of SUPPRESSED_EVENT_TYPES) {
      expect(evType in reducerRegistry).toBe(false);
    }
  });
});
