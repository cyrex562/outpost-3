/**
 * Reducer registry for the renderer-agnostic world model (HR-795, 104a).
 *
 * Side-effect import: importing this module populates `reducerRegistry` in
 * `model.ts`.  The live app path (`_websocketHandlers.ts`) is NOT changed —
 * these reducers are additive and wired to nothing in 104a.
 *
 * Each reducer:
 *   - narrows `data: unknown` to the concrete payload type via a type
 *     assertion that is safe because `apply()` only dispatches to a reducer
 *     when `event.event_type` matches the registered key;
 *   - mutates the model in-place (no Vue reactivity — subscribers handle that);
 *   - returns the Change[] tokens that `apply()` broadcasts to subscribers.
 *
 * COVERAGE (104a — grid/position events; 104b extensions marked):
 *   exploration.moved ✓   exploration.revealed ✓   travel.step ✓
 *   town.map ✓            town.move ✓
 *   combat.positions ✓
 *   combat.start ✓        (104b: also populates model.combat.enemies)
 *   combat.enemy_defeated ✓ (104b: also marks model.combat.enemies[].defeated)
 *   gm.scene_change ✓     (104b: clears model.combat; emits refetch-character)
 *
 * All remaining events are implemented in reducers-b.ts (side-effect imported
 * at the bottom of this file).
 */
import type {
  CellPreview,
  ExplorationMoved,
  ExplorationRevealedNotice,
  TravelStepNotice,
  TownMapNotice,
  TownMoveNotice,
  CombatPositionsNotice,
  CombatStartNotice,
  CombatEnemyDefeatedNotice,
  SceneChangeNotice,
  LootSourceRevealedNotice,
  QuestAcceptedNotice,
  QuestCompletedNotice,
  QuestFailedNotice,
} from "../types/events.gen";
import {
  reducerRegistry,
  type WorldModel,
  type Change,
  type LootSourceView,
  type CombatEnemyModel,
} from "./model";
import { setCell, emptyGrid, type Cell } from "./grid";

// ---------------------------------------------------------------------------
// Internal conversion helpers
// ---------------------------------------------------------------------------

/**
 * Converts a `CellPreview` (from events.gen — `explored: unknown`) to the
 * model's `Cell` type.
 */
function cellFromPreview(preview: CellPreview): Cell {
  const explored: number | boolean =
    typeof preview.explored === "number" ||
    typeof preview.explored === "boolean"
      ? preview.explored
      : 1;
  return {
    q: preview.q,
    r: preview.r,
    terrain: preview.terrain,
    features: preview.features,
    explored,
  };
}

/**
 * Converts a `Record<string, unknown>` cell (as carried in `TownMapNotice`)
 * to the model's `Cell` type, extracting well-known fields with safe defaults.
 */
function cellFromRecord(rec: Record<string, unknown>): Cell {
  const explored: number | boolean =
    typeof rec.explored === "number" || typeof rec.explored === "boolean"
      ? rec.explored
      : 1;
  return {
    q: typeof rec.q === "number" ? rec.q : 0,
    r: typeof rec.r === "number" ? rec.r : 0,
    terrain: typeof rec.terrain === "string" ? rec.terrain : "unknown",
    features: Array.isArray(rec.features) ? (rec.features as string[]) : [],
    explored,
  };
}

// ---------------------------------------------------------------------------
// Reducers
// ---------------------------------------------------------------------------

// --- exploration.moved ------------------------------------------------------
reducerRegistry["exploration.moved"] = (
  model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "exploration.moved".
  const d = data as ExplorationMoved;
  const changedCells: Cell[] = [cellFromPreview(d.target_cell)];
  for (const adj of d.adjacent_cells) {
    changedCells.push(cellFromPreview(adj));
  }
  for (const c of changedCells) {
    setCell(model.world, c);
  }
  model.player = { q: d.to_q, r: d.to_r };
  // Incremental world update: carry just the changed cells so the projection
  // does an O(1)-per-cell Map.set instead of rebuilding the whole cells Map.
  const changes: Change[] = [
    { kind: "world", cells: changedCells },
    { kind: "player" },
  ];
  // 104c: also update character location so StatusSidebar stays current
  // (mirrors gameStore.updateLocation() called by the old handler).
  if (model.character !== null) {
    model.character.locationQ = d.to_q;
    model.character.locationR = d.to_r;
    model.character.terrain = d.target_cell.terrain;
    model.character.features = d.target_cell.features;
    // Set weather unconditionally so a hex with no weather CLEARS the previous
    // hex's weather (mirrors the old updateLocation(...weather) call, which set
    // character.weather = null when d.weather was null).
    if (d.weather !== null) {
      const w = d.weather;
      if (
        typeof w.condition === "string" &&
        typeof w.temperature === "string" &&
        typeof w.description === "string"
      ) {
        model.character.weather = {
          condition: w.condition,
          temperature: w.temperature,
          description: w.description,
        };
      } else {
        model.character.weather = null;
      }
    } else {
      model.character.weather = null;
    }
    changes.push({ kind: "character" });
  }
  // HR-786: leaving a cell drops any revealed loot sources from the panel.
  if (model.loot.length > 0) {
    model.loot = [];
    changes.push({ kind: "loot" });
  }
  return changes;
};

// --- loot.source_revealed (HR-786) ------------------------------------------
reducerRegistry["loot.source_revealed"] = (
  model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "loot.source_revealed".
  const d = data as LootSourceRevealedNotice;
  const idx = model.loot.findIndex(
    (s) => s.id === d.id && s.q === d.q && s.r === d.r,
  );
  const exhausted = d.items.length === 0 && d.gold <= 0;
  const view: LootSourceView = {
    q: d.q,
    r: d.r,
    id: d.id,
    kind: d.kind,
    name: d.name,
    items: d.items,
    gold: d.gold,
  };
  if (exhausted) {
    if (idx !== -1) model.loot.splice(idx, 1);
  } else if (idx !== -1) {
    model.loot[idx] = view;
  } else {
    model.loot.push(view);
  }
  return [{ kind: "loot" }];
};

// --- exploration.revealed ---------------------------------------------------
reducerRegistry["exploration.revealed"] = (
  model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "exploration.revealed".
  const d = data as ExplorationRevealedNotice;
  const cell = cellFromPreview(d.cell);
  setCell(model.world, cell);
  return [{ kind: "world", cells: [cell] }];
};

// --- travel.step ------------------------------------------------------------
reducerRegistry["travel.step"] = (
  model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "travel.step".
  const d = data as TravelStepNotice;
  const cell = cellFromPreview(d.target_cell);
  setCell(model.world, cell);
  model.player = { q: d.to_q, r: d.to_r };
  const changes: Change[] = [
    { kind: "world", cells: [cell] },
    { kind: "player" },
  ];
  // 104c: also update character location (no weather on travel.step).
  if (model.character !== null) {
    model.character.locationQ = d.to_q;
    model.character.locationR = d.to_r;
    model.character.terrain = d.target_cell.terrain;
    model.character.features = d.target_cell.features;
    changes.push({ kind: "character" });
  }
  return changes;
};

// --- town.map ---------------------------------------------------------------
reducerRegistry["town.map"] = (
  model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "town.map".
  const d = data as TownMapNotice;
  model.town.cells.clear();
  model.town.entities = [];
  for (const raw of d.cells) {
    setCell(model.town, cellFromRecord(raw));
  }
  model.player = { q: d.player_q, r: d.player_r };
  // 104c: also store buildings and settlement name for projection
  model.townBuildings = d.buildings as Record<string, unknown>[];
  model.townSettlementName = d.settlement_name;
  return [{ kind: "town" }, { kind: "player" }];
};

// --- town.move --------------------------------------------------------------
reducerRegistry["town.move"] = (
  model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "town.move".
  const d = data as TownMoveNotice;
  model.player = { q: d.player_q, r: d.player_r };
  return [{ kind: "player" }];
};

// --- combat.positions -------------------------------------------------------
reducerRegistry["combat.positions"] = (
  model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "combat.positions".
  const d = data as CombatPositionsNotice;
  model.encounter.width = d.width;
  model.encounter.height = d.height;
  model.encounter.cells.clear();
  for (const pc of d.cells) {
    setCell(model.encounter, {
      q: pc.q,
      r: pc.r,
      terrain: pc.terrain,
      features: pc.features,
      explored: 1,
    });
  }
  model.encounter.entities = d.entities.map((pe) => ({
    id: pe.entity_id,
    kind: pe.kind,
    q: pe.q,
    r: pe.r,
    name: pe.name,
    hp: pe.hp,
    maxHp: pe.max_hp,
    alive: pe.alive,
  }));
  // HR-787: loot piles dropped by defeated enemies.
  model.combat.loot = (d.loot ?? []).map((l) => ({
    q: l.q,
    r: l.r,
    value: l.value,
    itemCount: l.item_count,
  }));
  return [{ kind: "encounter" }];
};

// --- combat.start -----------------------------------------------------------
reducerRegistry["combat.start"] = (
  model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "combat.start".
  // Positions are not yet known; combat.positions will set them.
  const d = data as CombatStartNotice;
  model.encounter.entities = d.enemy_states.map((s) => ({
    id: s.entity_id,
    kind: "monster" as const,
    q: 0,
    r: 0,
    name: s.name,
    hp: s.hp,
    maxHp: s.max_hp,
    alive: true,
  }));
  // 104b: also initialise the typed combat roster (mirrors gameStore.initCombatEnemies).
  const enemyModels: CombatEnemyModel[] = d.enemy_states.map((s) => ({
    entityId: s.entity_id,
    name: s.name,
    hp: s.hp,
    maxHp: s.max_hp,
    defeated: false,
  }));
  model.combat.enemies = enemyModels;
  model.combat.actions = [];
  model.combat.phase = "";
  return [{ kind: "encounter" }, { kind: "combat" }];
};

// --- combat.enemy_defeated --------------------------------------------------
reducerRegistry["combat.enemy_defeated"] = (
  model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "combat.enemy_defeated".
  const d = data as CombatEnemyDefeatedNotice;
  const changes: Change[] = [];

  const encIdx = model.encounter.entities.findIndex((e) => e.id === d.entity_id);
  if (encIdx !== -1) {
    // Immutably replace the entry so subscribers see a new reference.
    model.encounter.entities[encIdx] = {
      ...model.encounter.entities[encIdx],
      alive: false,
    };
    changes.push({ kind: "encounter" });
  }

  // 104b: also mark defeated in the typed combat roster.
  const combatIdx = model.combat.enemies.findIndex((e) => e.entityId === d.entity_id);
  if (combatIdx !== -1) {
    model.combat.enemies[combatIdx] = {
      ...model.combat.enemies[combatIdx],
      hp: 0,
      defeated: true,
    };
    changes.push({ kind: "combat" });
  }

  return changes;
};

// --- gm.scene_change --------------------------------------------------------
reducerRegistry["gm.scene_change"] = (
  model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "gm.scene_change".
  const d = data as SceneChangeNotice;
  model.scene = d.to;
  const changes: Change[] = [
    { kind: "scene" },
    // 104b: mirrors gameStore.loadCharacter() called by the handler.
    { kind: "refetch-character" },
  ];

  // Clear town map only when truly leaving town — NOT when entering a shop or
  // talking to an NPC (those return to town afterwards).  Mirrors the logic in
  // _websocketHandlers.ts.
  if (d.to !== "town" && d.to !== "shopping" && d.to !== "social") {
    model.town = emptyGrid("town");
    changes.push({ kind: "town" });
  }

  // Clear encounter and typed combat roster when leaving combat.
  // Mirrors gameStore.clearCombatEnemies() + encounterStore.clear().
  if (d.to !== "combat") {
    model.encounter = emptyGrid("encounter");
    model.combat = { enemies: [], actions: [], phase: "", loot: [] };
    changes.push({ kind: "encounter" }, { kind: "combat" });
  }

  // HR-786: a scene change also clears any revealed loot sources.
  if (model.loot.length > 0) {
    model.loot = [];
    changes.push({ kind: "loot" });
  }

  return changes;
};

// ---------------------------------------------------------------------------
// 104b extensions — side-effect import populates the registry with all
// remaining event reducers (character, combat, oracle, shopping, etc.).
// ---------------------------------------------------------------------------
import "./reducers-b";

// --- quest.* (HR-19) --------------------------------------------------------
// Quest lifecycle events re-fetch the active-quest count from the server
// (authoritative — no client-side counter drift) and surface a chat message.
function questRefetch(): Change[] {
  return [{ kind: "refetch-quests" }];
}

reducerRegistry["quest.accepted"] = (_m: WorldModel, data: unknown): Change[] => {
  const d = data as QuestAcceptedNotice;
  return [
    { kind: "message", text: `Quest accepted: ${d.quest_name}`, style: "system" },
    ...questRefetch(),
  ];
};

reducerRegistry["quest.completed"] = (_m: WorldModel, data: unknown): Change[] => {
  const d = data as QuestCompletedNotice;
  return [
    { kind: "message", text: `Quest completed: ${d.quest_name}`, style: "system" },
    ...questRefetch(),
  ];
};

reducerRegistry["quest.failed"] = (_m: WorldModel, data: unknown): Change[] => {
  const d = data as QuestFailedNotice;
  return [
    { kind: "message", text: `Quest failed: ${d.quest_name}`, style: "system" },
    ...questRefetch(),
  ];
};

reducerRegistry["quest.progress_updated"] = (): Change[] => questRefetch();
