/**
 * 104b reducers — oracle, combat, and exploration-action events.
 *
 * Side-effect imported by `reducers-b.ts`; do not import directly.
 *
 * Covers:
 *   oracle.chaos_changed   combat.actions    combat.attack
 *   combat.save            exploration.actions
 */
import type {
  OracleChaosChangedNotice,
  CombatActionsNotice,
  CombatAttackNotice,
  CombatSaveNotice,
  ExplorationActionsNotice,
} from "../types/events.gen";
import { reducerRegistry, type WorldModel, type Change } from "./model";

// ---------------------------------------------------------------------------
// oracle.chaos_changed
// ---------------------------------------------------------------------------

reducerRegistry["oracle.chaos_changed"] = (
  model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "oracle.chaos_changed".
  const d = data as OracleChaosChangedNotice;
  const oldChaos = d.old_value;
  const chaos = d.chaos_factor;
  model.chaos = chaos;
  return [
    { kind: "chaos" },
    {
      kind: "message",
      text: `[Oracle] Chaos: ${oldChaos} → ${chaos}`,
      style: "chaos",
    },
  ];
};

// ---------------------------------------------------------------------------
// combat.actions — action bar + phase update
// ---------------------------------------------------------------------------

reducerRegistry["combat.actions"] = (
  model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "combat.actions".
  const d = data as CombatActionsNotice;
  model.combat.phase = d.phase;
  model.combat.actions = d.actions;
  return [{ kind: "combat" }];
};

// ---------------------------------------------------------------------------
// combat.attack — update enemy HP in roster + encounter grid, emit log line
// ---------------------------------------------------------------------------

reducerRegistry["combat.attack"] = (
  model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "combat.attack".
  const d = data as CombatAttackNotice;
  const changes: Change[] = [];

  // Update enemy HP in the typed combat roster and encounter grid.
  if (d.target_hp_remaining !== null && d.target_max_hp !== null) {
    const rosterIdx = model.combat.enemies.findIndex((e) => e.entityId === d.target_id);
    if (rosterIdx !== -1) {
      model.combat.enemies[rosterIdx] = {
        ...model.combat.enemies[rosterIdx],
        hp: d.target_hp_remaining,
        maxHp: d.target_max_hp,
      };
      changes.push({ kind: "combat" });
    }

    const encIdx = model.encounter.entities.findIndex((e) => e.id === d.target_id);
    if (encIdx !== -1) {
      model.encounter.entities[encIdx] = {
        ...model.encounter.entities[encIdx],
        hp: d.target_hp_remaining,
        maxHp: d.target_max_hp,
      };
      changes.push({ kind: "encounter" });
    }
  }

  // Build combat log line (identical to the _websocketHandlers.ts branch).
  const rollStr = d.critical ? `${d.roll} (CRITICAL)` : String(d.roll);
  const hitStr = d.hit ? "Hit!" : "Miss";
  let line = `${d.attacker} attacks ${d.target}  [${d.weapon}]\n`;
  line += `   Roll: ${rollStr} + ${d.modifier} = ${d.total}  vs  AC ${d.target_ac}  →  ${hitStr}`;
  if (d.hit && d.damage != null) line += `  Damage: ${d.damage}`;
  if (!d.hit && d.shock > 0) line += `  |  Shock: ${d.shock}`;

  changes.push({ kind: "message", text: line, style: "combat_attack" });
  return changes;
};

// ---------------------------------------------------------------------------
// combat.save
// ---------------------------------------------------------------------------

reducerRegistry["combat.save"] = (
  _model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "combat.save".
  const d = data as CombatSaveNotice;
  const result = d.passed ? "Passed" : "Failed";
  return [
    {
      kind: "message",
      text:
        `${d.save_type} Save  [${d.character}]\n` +
        `   Roll: ${d.roll} + ${d.modifier} = ${d.total}  vs  ${d.target}  →  ${result}`,
      style: "combat_save",
    },
  ];
};

// ---------------------------------------------------------------------------
// exploration.actions — tile-context action bar
// ---------------------------------------------------------------------------

reducerRegistry["exploration.actions"] = (
  model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "exploration.actions".
  const d = data as ExplorationActionsNotice;
  model.explorationActions = d.actions;
  return [{ kind: "explorationActions" }];
};
