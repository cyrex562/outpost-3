# ECS Entity Inventory

This document inventories the candidate ECS entity families for Harsh Realm and
ranks them by implementation payoff.

The ranking is based on the ECS ADR criteria:

- frequent runtime mutation
- overlapping behavior across entity types
- compositional mechanics
- expected future feature growth

The goal is not to move every runtime object into ECS. The goal is to identify
the slices where ECS is most likely to reduce behavioral complexity.

## Ranking Summary

| Rank | Entity family | Payoff | Recommendation |
| --- | --- | --- | --- |
| 1 | Actors | Very high | First ECS pilot |
| 2 | Feature instances | High | Second-wave ECS slice after actor pilot |
| 3 | Dungeon cells / room nodes | High | Migrate with or just after feature instances |
| 4 | Town cells | Medium-high | Migrate after dungeon/world feature patterns settle |
| 5 | World hexes | Medium-high | Use selectively for runtime overlays, not all map data |
| 6 | Encounter groups | Medium | Add when encounter orchestration grows beyond current scene flow |
| 7 | Hazards | Medium | Add when traps/environmental effects become richer |
| 8 | Loot and item entities | Medium-low now, high later | Defer until stacking/ammo/equipment depth justifies item-instance ECS |
| 9 | Faction-presence entities | Medium-low now | Defer until factions produce persistent runtime patrols or expeditions |

## Ranked Inventory

## 1. Actors

Examples:

- player character
- NPCs
- combat enemies
- later summoned or temporary allies

Current pressure points:

- combat state duplicates or reprojects actor information
- exploration, social, shopping, town, and combat all use overlapping actor data
- actor behavior spans position, health, inventory, faction, disposition,
  interaction, and AI concerns

Why payoff is highest:

- actors already cross the most systems
- new mechanics such as status effects, cooldowns, equipment rules, AI intent,
  and temporary buffs fit naturally as components
- the actor slice can be piloted without changing persistence ownership

Recommendation:

- first ECS pilot

## 2. Feature Instances

Examples:

- lairs
- ruins
- landmarks
- exits
- searchable points
- settlement entry markers
- dungeon entry points

Current pressure points:

- feature behavior is increasingly trigger-driven
- features cut across exploration, town, dungeon, and discovery logic
- “what happens when I enter/search/interact here?” is becoming behavior-rich

Why payoff is high:

- feature instances are naturally compositional
- they avoid turning cells into monolithic bags of special-case flags
- they align with the original “cells as ECS” instinct, but in a more precise
  way

Recommendation:

- second-wave ECS slice after actors

## 3. Dungeon Cells / Room Nodes

Examples:

- dungeon traversal cells
- room nodes
- doors
- stairs
- chokepoints

Current pressure points:

- dungeon runtime will accumulate blocking, visibility, encounter, loot,
  interactable, and exit behavior
- square-grid dungeon movement is behavior-heavy and likely to grow

Why payoff is high:

- dungeon runtime is local, bounded, and simulation-oriented
- cells/rooms share many orthogonal flags and triggers
- dungeon runtime is a safer ECS expansion than the entire world map

Recommendation:

- move with or just after feature-instance ECS

## 4. Town Cells

Examples:

- roads
- shops
- taverns
- temples
- houses
- plaza cells

Current pressure points:

- town cells carry movement, building-entry, NPC-placement, shop, healer, and
  interaction behavior
- likely future UX work will deepen cell-specific town interactions

Why payoff is medium-high:

- town scenes already resemble a local simulation grid
- multiple cell behaviors overlap but are still somewhat simpler than dungeon
  and combat actor logic

Recommendation:

- migrate after actor and dungeon/feature patterns are proven

## 5. World Hexes

Examples:

- exploration hexes
- runtime overlays for searched/discovered/hostile/settlement-linked state

Current pressure points:

- world hexes have exploration, search, encounter, and settlement context
- feature presence and regional effects will likely add more orthogonal behavior

Why payoff is medium-high:

- useful for runtime overlays and interactions
- less useful if treated as a full replacement for the map persistence model

Risk:

- the world map is broad and easy to over-model

Recommendation:

- use ECS for runtime-active hex behavior only, not as a replacement for all map
  storage

## 6. Encounter Groups

Examples:

- roaming enemy groups
- ambush groups
- transient encounter controllers

Current pressure points:

- encounters are currently scene-triggered rather than deeply modeled
- richer encounter setup, awareness, reinforcements, and flee behavior could
  benefit from group-level runtime state

Why payoff is medium:

- useful once encounters gain more structure than “spawn and resolve”
- not the first bottleneck today

Recommendation:

- defer until combat/exploration actor ECS is stable

## 7. Hazards

Examples:

- traps
- environmental damage zones
- weather effects
- timed danger areas

Current pressure points:

- most hazard mechanics are still shallow or not yet implemented
- these mechanics will likely become timer- and trigger-driven

Why payoff is medium:

- ECS fits hazards well once they exist in meaningful numbers
- current engine does not yet need a hazard-specific runtime architecture

Recommendation:

- defer until hazard and environmental systems deepen

## 8. Loot And Item Entities

Examples:

- dropped item piles
- death-marker loot bundles
- containers
- future stack instances
- future ammo-bearing ranged weapons

Current pressure points:

- item handling is still largely inventory-list based
- upcoming mechanics like stacking, ammo, equipment state, and dropped-item
  behavior could push items toward richer instance logic

Why payoff is medium-low now:

- current complexity is not yet item-entity orchestration
- item-instance ECS too early would add churn without much simplification

Why payoff may rise later:

- stacking
- ammo depletion
- equipment slots
- container nesting
- ground loot interaction

Recommendation:

- revisit after inventory mechanics expand

## 9. Faction-Presence Entities

Examples:

- patrols
- expeditions
- scouts
- mobile threat sources

Current pressure points:

- faction strategy logic is already reasonably served by repositories and turn
  processing
- live faction presence on the map is still limited

Why payoff is medium-low now:

- the faction strategy layer itself is not the current ECS target
- ECS becomes useful only when factions create runtime map presences that move,
  trigger encounters, or occupy spaces

Recommendation:

- defer until faction patrol and world-presence mechanics exist

## Recommended Pilot Order

1. Actors
2. Feature instances
3. Dungeon cells / room nodes
4. Town cells
5. World hex runtime overlays
6. Encounter groups
7. Hazards
8. Loot and item entities
9. Faction-presence entities

## Boundary Notes

This ranking does not change the ECS ADR boundaries:

- persistence stays repository-driven
- ECS remains runtime-only
- Pydantic remains the structured model layer
- admin/editor/API models remain outside ECS
