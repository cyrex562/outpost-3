# Modular Rules Phase 3: Trigger / Effect Engine — Codebase Audit

**Date:** 2026-05-06
**Author:** Gemini CLI
**Status:** Complete (Task 3.1)

## 1. Existing Event Taxonomy

The following `GameEvent` types are currently emitted by the engine and are candidates for trigger sources:

### 1.1 Spatial & Movement
- `exploration.move_requested`: Emitted when the player attempts to move to a new hex.
- `exploration.enter_cell`: Emitted when the player successfully enters a new hex.
- `dungeon.enter_room`: Emitted when the player enters a dungeon room.
- `town.move`: Emitted during movement within a settlement.

### 1.2 Character & Combat
- `character.hp_changed`: Emitted whenever HP is modified (damage or healing).
- `character.death_final`: Emitted when a character's HP hits 0 and they collapse.
- `combat.attack`: Emitted during an attack roll.
- `combat.player_hit`: Emitted when the player takes damage in combat.
- `combat.enemy_defeated`: Emitted when an NPC combatant is killed.
- `combat.victory_requested`: Emitted when all enemies are defeated.

### 1.3 Interaction & Meta
- `exploration.search_requested`: Emitted when the player uses the `search` command.
- `exploration.take_requested`: Emitted when the player picks up a death marker.
- `gm.scene_change`: Emitted when switching between scene handlers (e.g., Exploration → Combat).
- `status.applied` / `status.expired`: Emitted by the Status Effect system.

## 2. Hardcoded Trigger Candidates

The following logic blocks are currently hardcoded and should be replaced by the Trigger/Effect engine:

### 2.1 Environmental Reactions
- **Dungeon Encounters:** `DungeonScene._handle_move` (lines 219–225) manually checks `target_room.encounter` and narrates flavor text. This should be a trigger on `dungeon.enter_room`.
- **Blocked Movement:** `ExplorationMovementMixin._handle_move` checks `terrain_type.passable` and narrates a blocked message. While passability is a core mechanic, "Danger" or "Hazard" zones could use triggers on `exploration.move_requested` to prevent entry or apply damage.
- **Trap Detection:** Currently, "Traps" do not exist as a formal mechanic. They should be implemented as triggers on `exploration.enter_cell` or `dungeon.enter_room`.

### 2.2 Character Abilities
- **Veteran's Luck:** `CombatEnemyMixin._run_enemy_turns` (lines 62–80) manually branches to check for Warrior class and "veteran_luck_used". This should be a trigger on `combat.player_hit`.
- **Expert Reroll:** `SocialScene._handle_command` (and others) handles the Expert reroll logic. This is a candidate for a `on_failure` trigger on `action.skill_check`.

### 2.3 Consumption & Resources
- **Ammo Consumption:** `CombatActionsMixin._handle_attack` manually calls `_consume_ammo_event`. This could be a trigger on `combat.attack` where the item has a "Consumes: ammo" trait.

## 3. Content-Pack Readiness

### 3.1 Trait Schema
The `Trait` model in `src/harsh_realm/traits/schema.py` already includes a `triggers` field:
```python
triggers: list[JsonObject] = Field(
    default_factory=list,
    description="Trigger records stored for forward compatibility. Phase 3 adds the runner."
)
```

### 3.2 Discovery Data
`packs/xwn-core/content/tables/discoveries/wasteland.yaml` contains "environmental" discoveries. These currently just add a feature string to the cell. They should instead materialize a Trigger Entity or attach a Trigger to the cell.

## 4. Integration Seams

The `TriggerEngine` should be integrated at the following points:

1.  **`DomainEventDispatcher`:** The engine should subscribe to ALL events (or a broad wildcard) to evaluate potential triggers.
2.  **`ProcedureRunner`:** Triggers will likely execute `Procedures` as their "Effects".
3.  **`GMController`:** The controller already handles the event cascade; the `TriggerEngine` should be a "System" or "Middleware" that injects new events into the cascade based on triggers.

## 5. Formal Taxonomy Proposal

For Phase 3, we will formalize the following trigger structure:

- **Source:** The `event_type` that trips the trigger.
- **Condition:** A declarative expression (or procedure call) that must evaluate to `True`.
- **Effect:** A `Procedure` to execute, or a list of `GameEvents` to emit.

### Recommended First Implementation Targets:
1.  **Traps:** `on: dungeon.enter_room` -> `condition: has_trap` -> `effect: deal_damage_procedure`.
2.  **Ambient Hazards:** `on: exploration.enter_cell` -> `condition: terrain == "wasteland"` -> `effect: apply_status_procedure("irradiated")`.
