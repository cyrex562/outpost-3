# ECS Component Catalog

This document defines the first ECS component catalog for Harsh Realm.

It is intentionally small enough to implement, but broad enough to cover the
first actor pilot and the next spatial/feature expansion.

This catalog follows:

- [docs/ecs_adr.md](/home/cyrex/Projects/harsh_realm/docs/ecs_adr.md)
- [docs/ecs_entity_inventory.md](/home/cyrex/Projects/harsh_realm/docs/ecs_entity_inventory.md)
- [docs/ecs_runtime_architecture.md](/home/cyrex/Projects/harsh_realm/docs/ecs_runtime_architecture.md)

## Design Rules

- Components represent orthogonal concerns, not full aggregate objects.
- Components are typed Pydantic models.
- Components should be small and focused.
- Components may carry persistent IDs for reconciliation, but not repository or
  transport objects.
- Business logic belongs in systems, not inside component methods.
- If a field is only useful for one scene-specific edge case, do not make it a
  component until that pattern repeats.

## Component Groups

## 1. Actor Components

These components define the minimum vocabulary for player characters, NPCs, and
combat enemies.

### `IdentityComponent`

Purpose:

- stable display and origin metadata for any runtime entity

Suggested fields:

- `entity_kind: str`
- `persistent_id: str | None`
- `name: str`
- `display_name: str | None`
- `tags: list[str]`

Used by:

- actors
- feature instances
- encounter groups
- hazards

### `ActorRoleComponent`

Purpose:

- distinguish player, NPC, enemy, ally, summon, or other actor categories

Suggested fields:

- `role: str`
- `is_player: bool = False`
- `is_hostile: bool = False`

Used by:

- actors only

### `PositionComponent`

Purpose:

- current runtime location within a scene/grid

Suggested fields:

- `scene_kind: str`
- `q: int`
- `r: int`
- `world_q: int | None = None`
- `world_r: int | None = None`

Used by:

- actors
- spatial cells
- features
- hazards
- loot entities

### `MovementComponent`

Purpose:

- movement capability and restrictions

Suggested fields:

- `speed: int = 1`
- `blocks_movement: bool = False`
- `can_diagonal: bool = True`
- `can_leave_scene: bool = False`
- `traversal_tags: list[str]`

Used by:

- actors
- blocking spatial entities
- features such as exits or doors later

### `HealthComponent`

Purpose:

- HP state and defeat/death thresholds

Suggested fields:

- `hp: int`
- `max_hp: int`
- `alive: bool = True`
- `downed: bool = False`
- `last_stand_available: bool = False`

Used by:

- actors
- destructible hazards or objects later

### `CombatStatsComponent`

Purpose:

- combat-facing numeric stats

Suggested fields:

- `ac: int`
- `attack_bonus: int`
- `damage_expr: str`
- `num_attacks: int = 1`
- `range_band: str = "melee"`
- `weapon_id: str | None = None`

Used by:

- actors engaged in combat

### `InventoryComponent`

Purpose:

- runtime-carried item list and currency-like holdings

Suggested fields:

- `items: list[dict[str, object]]`
- `gold: int | None = None`
- `capacity: int | None = None`

Used by:

- player character
- NPCs if lootable later
- containers later

Note:

- for the first pass, item entries may remain typed inventory payload records
  nested inside the component rather than turning each item into its own ECS
  entity

### `EquipmentComponent`

Purpose:

- active equipped/ready item state that affects combat or defense

Suggested fields:

- `weapon_id: str | None = None`
- `armor_id: str | None = None`
- `readied_item_ids: list[str]`

Used by:

- actors

### `FactionAffiliationComponent`

Purpose:

- actor or feature allegiance and ownership

Suggested fields:

- `faction_id: str | None`
- `disposition_to_player: int | None = None`

Used by:

- actors
- features
- faction-presence entities later

## 2. Spatial Components

These components define world hexes, town cells, dungeon cells, and similar
runtime map entities.

### `SpatialCellComponent`

Purpose:

- classify a spatial runtime node

Suggested fields:

- `cell_kind: str`
- `terrain: str`
- `passable: bool = True`
- `visible: bool = True`

Used by:

- world hexes
- town cells
- dungeon cells

### `ScenePresenceComponent`

Purpose:

- declare which scene/runtime slice the entity currently belongs to

Suggested fields:

- `scene_kind: str`
- `active: bool = True`
- `layer: str = "default"`

Used by:

- actors
- spatial cells
- features
- hazards
- encounter groups

### `OccupancyComponent`

Purpose:

- track whether a spatial entity can hold actors or blocks them

Suggested fields:

- `capacity: int | None = None`
- `occupied_by: list[str]`
- `exclusive: bool = False`

Used by:

- spatial cells
- containers or interactable stations later

### `ConnectivityComponent`

Purpose:

- represent graph relationships between room nodes, doors, exits, or linked
  tiles

Suggested fields:

- `neighbors: list[str]`
- `connection_tags: list[str]`

Used by:

- dungeon rooms
- exits
- feature links

## 3. Interaction Components

These components define how cells, features, NPCs, and objects respond to the
player.

### `InteractableComponent`

Purpose:

- generic interaction capability

Suggested fields:

- `interaction_kind: str`
- `prompt: str | None = None`
- `enabled: bool = True`

Used by:

- NPCs
- shops
- healers
- dungeon exits
- landmarks

### `SearchableComponent`

Purpose:

- discovery/search behavior and cooldown state

Suggested fields:

- `search_kind: str`
- `last_searched_tick: int | None = None`
- `cooldown_ticks: int = 0`
- `hidden_until_found: bool = False`

Used by:

- world hex runtime overlays
- feature instances
- dungeon cells
- ruins/lairs/landmarks

### `EnterableComponent`

Purpose:

- transition into a subscene or special location

Suggested fields:

- `destination_kind: str`
- `destination_id: str | None = None`
- `requires_command: bool = True`

Used by:

- settlements
- dungeons
- buildings
- exits

### `LootComponent`

Purpose:

- dropped or discoverable loot payloads

Suggested fields:

- `items: list[dict[str, object]]`
- `claimable: bool = True`
- `loot_kind: str = "drop"`

Used by:

- death markers
- feature instances
- containers
- loot entities later

## 4. Social Components

These components support NPC interaction and disposition-driven behavior.

### `DispositionComponent`

Purpose:

- player-facing social stance

Suggested fields:

- `score: int`
- `band: str | None = None`

Used by:

- NPC actors
- faction-presence actors later

### `NpcRoleComponent`

Purpose:

- NPC occupation and world-facing role

Suggested fields:

- `occupation: str`
- `building_id: str | None = None`
- `establishment_type: str | None = None`
- `establishment_name: str | None = None`

Used by:

- NPC actors

### `DialogueComponent`

Purpose:

- greeting and short interaction text/state

Suggested fields:

- `greeting: str = ""`
- `appearance: str = ""`
- `motivation: str = ""`
- `personality_traits: list[str]`

Used by:

- NPC actors

## 5. AI Components

These components support enemy and NPC decision behavior without encoding the
behavior directly into actor classes.

### `AiIntentComponent`

Purpose:

- current selected runtime intent

Suggested fields:

- `intent: str`
- `target_entity_id: str | None = None`
- `target_position_q: int | None = None`
- `target_position_r: int | None = None`

Used by:

- enemies
- NPCs with richer movement later
- faction patrols later

### `BehaviorProfileComponent`

Purpose:

- stable AI strategy profile

Suggested fields:

- `behavior: str`
- `aggression: int | None = None`
- `preferred_range: str | None = None`

Used by:

- combat enemies
- patrols later

## 6. Environmental Components

These components support hazards, triggers, and timed world effects.

### `HazardComponent`

Purpose:

- describes a harmful or restrictive environmental effect

Suggested fields:

- `hazard_kind: str`
- `damage_expr: str | None = None`
- `save_type: str | None = None`
- `trigger_on_enter: bool = False`

Used by:

- traps
- damage zones
- environmental effects later

### `TriggerComponent`

Purpose:

- generic event-on-condition behavior

Suggested fields:

- `trigger_kind: str`
- `once: bool = False`
- `armed: bool = True`

Used by:

- hazards
- discovery features
- exits
- special map interactions

### `DurationComponent`

Purpose:

- time-limited existence or effect window

Suggested fields:

- `remaining_ticks: int | None = None`
- `expires_at_tick: int | None = None`

Used by:

- temporary hazards
- summoned entities later
- encounter groups later

## 7. Deferred Components

These should not be in the first implementation unless a concrete pilot needs
them:

- `StatusEffectsComponent`
- `EncounterComponent`
- `WeatherComponent`
- `ContainerComponent`
- `QuestMarkerComponent`
- `VisibilityComponent`

They are likely to become useful later, but they should not be built before the
actor pilot proves the ECS runtime.

## Recommended First-Pass Implementation Set

The first implementation should include only the components needed for the actor
pilot and immediate spatial follow-up:

- `IdentityComponent`
- `ActorRoleComponent`
- `PositionComponent`
- `MovementComponent`
- `HealthComponent`
- `CombatStatsComponent`
- `InventoryComponent`
- `EquipmentComponent`
- `FactionAffiliationComponent`
- `ScenePresenceComponent`
- `InteractableComponent`
- `DispositionComponent`
- `NpcRoleComponent`
- `DialogueComponent`
- `AiIntentComponent`
- `BehaviorProfileComponent`
- `SpatialCellComponent`
- `SearchableComponent`
- `EnterableComponent`
- `LootComponent`

## Mapping By Behavior Area

### Actor behavior

- `IdentityComponent`
- `ActorRoleComponent`
- `PositionComponent`
- `MovementComponent`
- `HealthComponent`
- `CombatStatsComponent`
- `InventoryComponent`
- `EquipmentComponent`
- `FactionAffiliationComponent`

### Spatial behavior

- `ScenePresenceComponent`
- `SpatialCellComponent`
- `OccupancyComponent`
- `ConnectivityComponent`

### Interaction behavior

- `InteractableComponent`
- `SearchableComponent`
- `EnterableComponent`
- `LootComponent`

### Combat behavior

- `HealthComponent`
- `CombatStatsComponent`
- `EquipmentComponent`
- `BehaviorProfileComponent`
- `AiIntentComponent`

### Inventory behavior

- `InventoryComponent`
- `EquipmentComponent`
- `LootComponent`

### Social behavior

- `DispositionComponent`
- `NpcRoleComponent`
- `DialogueComponent`
- `FactionAffiliationComponent`

### AI behavior

- `AiIntentComponent`
- `BehaviorProfileComponent`

### Environmental behavior

- `HazardComponent`
- `TriggerComponent`
- `DurationComponent`

## Implementation Notes

- The first package should define the component classes and nothing more
  opinionated than shared base config.
- If a component ends up always appearing together with another component across
  all pilot entities, merge them only after that pattern is proven.
- If a component grows broad and scene-specific, split it rather than adding
  optional fields for unrelated behavior.
