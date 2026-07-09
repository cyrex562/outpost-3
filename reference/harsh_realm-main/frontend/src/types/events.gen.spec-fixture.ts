/**
 * Compile-time fixture for HR-794: proves that `events.gen.ts` type-checks
 * under strict mode.  This file is never executed — its sole purpose is to
 * satisfy `vue-tsc --noEmit`.  If a Rust payload field is renamed or its type
 * changes, the matching assignment below will produce a TypeScript error, which
 * is exactly the contract we want.
 *
 * Run: npx vue-tsc --noEmit   (from the frontend/ directory)
 */

import type {
  ClientEvent,
  CharacterHpChangedNotice,
  CombatAttackNotice,
  ActiveStatusEffectPayload,
  StatusAppliedNotice,
  ExplorationMoved,
  CharacterSnapshot,
  CellPreview,
  TravelEndNotice,
  GmSuggestionsNotice,
  CharacterLevelUpNotice,
  NarrationNotice,
} from "./events.gen";

// ---------------------------------------------------------------------------
// Simple payload — CharacterHpChangedNotice
// ---------------------------------------------------------------------------

const hpPayload: CharacterHpChangedNotice = {
  hp: 42,
  max_hp: 100,
};

// ---------------------------------------------------------------------------
// Nested optional fields — CombatAttackNotice
// ---------------------------------------------------------------------------

const attackPayload: CombatAttackNotice = {
  attacker: "PC",
  target: "Goblin",
  attacker_id: "pc-1",
  target_id: "mob-1",
  weapon: "short sword",
  roll: 15,
  modifier: 2,
  total: 17,
  target_ac: 13,
  hit: true,
  damage: 6,
  shock: 0,
  critical: false,
  target_hp_remaining: 4,
  target_max_hp: 10,
};

// ---------------------------------------------------------------------------
// number (i64) fields and Record<string, unknown> — ActiveStatusEffectPayload
// i64 is pinned to `number` via #[ts(type = "number")] because serde_json
// serializes i64 as a plain JSON number, not a bigint.
// ---------------------------------------------------------------------------

const effectPayload: ActiveStatusEffectPayload = {
  id: 1,
  entity_id: "e1",
  effect_id: "dread",
  applied_at_tick: 10,
  expires_at_tick: 20,
  source: "ruins",
  data: { severity: 2 },
};

// ---------------------------------------------------------------------------
// Nested custom type — StatusAppliedNotice → ActiveStatusEffectPayload
// ---------------------------------------------------------------------------

const statusApplied: StatusAppliedNotice = {
  active_effect: effectPayload,
};

// ---------------------------------------------------------------------------
// Complex nested — ExplorationMoved (uses CharacterSnapshot + CellPreview)
// ---------------------------------------------------------------------------

const snap: CharacterSnapshot = {
  id: "pc-1",
  name: "Kira",
  character_class: "Warrior",
  level: 3,
  xp: 1200,
  xp_next: 2000,
  attributes: { str: 12, dex: 14, con: 11, int: 10, wis: 9, cha: 13 },
  attr_mods: { str: 0, dex: 1, con: 0, int: 0, wis: -1, cha: 0 },
  skills: { stab: 2, exert: 1 },
  hp: 18,
  max_hp: 22,
  ac: 14,
  attack_bonus: 2,
  physical_save: 14,
  evasion_save: 13,
  mental_save: 15,
  equipment: [],
  class_abilities: {},
  position_q: 2,
  position_r: 3,
};

const cell: CellPreview = {
  q: 2,
  r: 3,
  terrain: "forest",
  features: ["ruins"],
  explored: 1 as unknown, // unknown per #[ts(type = "unknown")] override
};

const moved: ExplorationMoved = {
  character_id: "pc-1",
  character_data: snap,
  from_q: 1,
  from_r: 1,
  to_q: 2,
  to_r: 3,
  direction: "ne",
  first_visit: true,
  target_cell: cell,
  adjacent_cells: [cell],
  weather: null,
};

// ---------------------------------------------------------------------------
// Shared-struct variants — TravelEndNotice used for blocked/cancelled/completed
// ---------------------------------------------------------------------------

const travelEnd: TravelEndNotice = { target_q: 5, target_r: 3 };

// ---------------------------------------------------------------------------
// New HR-794 structs
// ---------------------------------------------------------------------------

const lvlUp: CharacterLevelUpNotice = {
  new_level: 4,
  new_max_hp: 28,
  hp_gained: 6,
};

const suggestions: GmSuggestionsNotice = {
  commands: ["north", "look", "inventory"],
};

// NarrationNotice is the data shape of the `{type:"narration"}` frame; it is
// still exported for the frontend but is intentionally NOT a ClientEvent
// (game_event) variant — see the ClientEvent docstring in client_event.rs.
const narration: NarrationNotice = {
  text: "The dungeon is silent.",
};

// ---------------------------------------------------------------------------
// Top-level discriminated union
// ---------------------------------------------------------------------------

const ev1: ClientEvent = { event_type: "character.hp_changed", data: hpPayload };
const ev2: ClientEvent = { event_type: "combat.attack", data: attackPayload };
const ev3: ClientEvent = { event_type: "status.applied", data: statusApplied };
const ev4: ClientEvent = { event_type: "exploration.moved", data: moved };
const ev5: ClientEvent = { event_type: "travel.blocked", data: travelEnd };
const ev6: ClientEvent = { event_type: "travel.cancelled", data: travelEnd };
const ev7: ClientEvent = { event_type: "travel.completed", data: travelEnd };
const ev8: ClientEvent = { event_type: "character.level_up", data: lvlUp };
const ev9: ClientEvent = { event_type: "gm.suggestions", data: suggestions };

// Suppress "declared but never used" errors — this file is a pure type-check fixture.
void [
  hpPayload,
  attackPayload,
  effectPayload,
  statusApplied,
  snap,
  cell,
  moved,
  travelEnd,
  lvlUp,
  suggestions,
  narration,
  ev1, ev2, ev3, ev4, ev5, ev6, ev7, ev8, ev9,
];
