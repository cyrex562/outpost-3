# ECS Item And Loot Evaluation

## Status

Accepted

## Question

Should Harsh Realm move loot and item instances into ECS now, or defer that
work until the inventory and equipment rules become more stateful?

## Current Runtime State

Harsh Realm has typed item definitions and typed inventory payload records, but
runtime item behavior is still relatively shallow.

Current patterns:

- character inventory is a flat `character.equipment` list of
  `InventoryItemRecord` payloads
- shopping appends or removes one payload from that list
- exploration `take` appends one payload from a death marker into that list
- item use searches the list by name and pops a consumable on success
- dungeon room loot is still modeled as a list payload attached to the room
- ECS currently carries items inside `InventoryComponent` and `LootComponent`
  rather than as first-class item entities

Current limitations:

- item instances do not have stable runtime identity
- equipped state is still mostly inferred from payload shape and scene logic
- ammo is tracked in `character.class_abilities["ammo"]`, not as inventory item
  instances
- stacking and quantity are not first-class runtime mechanics yet
- dropped-item mechanics exist, but only as payload movement between lists and
  markers
- containers, slot occupancy, and equipment-specific state are not modeled as
  independent runtime entities

## Evaluation

Moving items into ECS now would create significant adapter and persistence
surface area without solving a pressing runtime complexity problem.

Why item-instance ECS is not justified yet:

- most item operations are list operations, not behavior-rich interactions
- current scenes do not need item-level queries across many orthogonal
  components
- ammo, stack quantity, and equipment-slot state are not yet represented as
  item-instance state, so ECS would mostly wrap today's flat payloads
- the current `InventoryComponent` plus `EquipmentComponent` already cover the
  actor-side runtime needs that exist today

Where ECS already helps enough:

- actor inventory can stay as a typed carried-item list on the actor
- dropped or searchable loot can stay as a typed payload list on a spatial loot
  entity
- scene systems can continue to reason about inventory and loot at the actor or
  container level without per-item entity churn

## Decision

Harsh Realm should defer first-class loot/item ECS entities for now.

The current design should remain:

- actors own carried inventory through `InventoryComponent`
- actor-ready state is represented by `EquipmentComponent`
- dropped/discoverable loot remains payload data inside `LootComponent`
- repositories continue to persist inventory as typed payload lists rather than
  item-instance rows

This means loot and item entities are still an ECS candidate, but not an active
migration target yet.

## Revisit Triggers

Revisit item-instance ECS when at least two of the following become true:

1. inventory stacking is implemented with real quantity semantics
2. ammo is moved from `class_abilities["ammo"]` into inventory or equipped item
   state
3. equipment uses explicit slots, readied vs stowed state, durability, charges,
   or condition
4. dropped items become persistent world/container instances rather than list
   payload transfers
5. inventory interactions require system-level queries across many item traits,
   such as stackable, equipped, consumable, quest-tagged, container-bound, or
   trade-restricted

## Future Direction

The strongest future justification for item ECS is when items become world
instances rather than only inventory payloads.

Examples:

- chest contents
- corpse loot
- shelf or room inventory
- dropped gear on the map
- persistent containers in settlements or dungeons

At that point, an item is no longer just "data in a list". It becomes a runtime
object with:

- stable instance identity
- location or containment
- movement between world, container, and actor inventory
- stack split and merge behavior
- optional state such as charges, ammo count, durability, condition, lock state,
  or quest binding

When that threshold is reached, item ECS should begin with world and container
item instances first, not with a blanket migration of every carried item.

Recommended future rollout:

1. dropped-world item entities and chest/container item entities
2. stackable loot and ammo instance entities
3. equipped and readied item instances
4. broader carried-inventory item ECS only if the rules depth continues to grow

## Recommended Sequence Before Item ECS

1. implement item stacking and quantity-aware inventory records
2. move ammo into typed inventory or equipped-item state
3. model explicit equipment slots and readied/stowed state
4. deepen dropped-item and container mechanics
5. then evaluate a pilot with item-instance ECS for:
   - dropped world loot
   - container contents
   - equipped/readied item instances

## Consequences

Positive:

- keeps ECS focused on the actor and spatial slices that already reduce runtime
  complexity
- avoids creating item entities before the runtime needs stable item identity
- preserves a simpler migration path while inventory rules are still changing

Costs:

- item logic remains partly list-oriented for now
- ammo, stacking, and equipment mechanics will need a later refactor before item
  ECS is reconsidered

## Related Work

- [docs/ecs_adr.md](/home/cyrex/Projects/harsh_realm/docs/ecs_adr.md)
- [docs/ecs_entity_inventory.md](/home/cyrex/Projects/harsh_realm/docs/ecs_entity_inventory.md)
- [docs/ecs_component_catalog.md](/home/cyrex/Projects/harsh_realm/docs/ecs_component_catalog.md)
- [docs/model_ecs_plan.md](/home/cyrex/Projects/harsh_realm/docs/model_ecs_plan.md)
