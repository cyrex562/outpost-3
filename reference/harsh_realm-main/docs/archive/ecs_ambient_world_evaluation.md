# ECS Ambient World Evaluation

## Status

Accepted

## Question

Should faction patrols, expeditions, hazards, and other ambient world entities
become ECS-managed runtime entities now that the actor and spatial pilots have
stabilized?

## Current Runtime State

The current engine does not yet model most ambient-world concepts as persistent,
behavior-rich runtime entities.

### Faction patrols and expeditions

Current implementation:

- faction turns operate as weekly repository-driven state updates and event
  summaries
- faction reputation modifies encounter-table weights such as
  `patrol_hostile`, `patrol_friendly`, and `bounty_hunter`
- encounter outcomes are still mostly scene-triggered rolls, not persistent
  roaming groups on the map

What is missing:

- no patrol entity with identity, route, destination, strength, or goal
- no expedition lifecycle moving through world space over time
- no persistent ambient faction presence projected into active hexes

### Hazards and ambient world effects

Current implementation:

- hazards mostly exist as content or scene concepts such as trap rooms,
  environmental encounter entries, and planning notes
- the runtime does not yet have a dedicated hazard system with timers,
  occupancy checks, trigger radii, or ongoing world effects

What is missing:

- no persistent hazard instances with lifecycle state
- no weather-front or regional-effect entities
- no active trap/disaster systems that need cross-entity queries

### Encounter groups

Current implementation:

- encounters are still resolved from table rolls and immediate scene setup
- hostile encounters become combat state, not persistent roaming groups

What is missing:

- no encounter controller entity carrying awareness, reinforcement, pursuit, or
  retreat state across scene boundaries

## Evaluation

The actor and spatial ECS pilots succeeded because those domains already had
real shared runtime state and cross-scene behavioral overlap.

Ambient-world entities are not at that threshold yet.

### Faction-presence entities

Faction patrols and expeditions should not move into ECS yet.

Why:

- the current faction layer is strategic and repository-driven, not a live map
  simulation
- current faction hostility is expressed through encounter modifiers rather than
  persistent mobile entities
- creating ECS patrol entities now would mostly invent a simulation layer that
  the gameplay does not yet use

### Hazards

Hazards are a stronger future ECS candidate than faction patrols, but they
should still be deferred for now.

Why:

- hazards naturally fit component-driven runtime behavior
- they would benefit from spatial queries, triggers, timers, and occupancy
  checks
- the current engine does not yet have enough implemented hazard mechanics to
  justify the adapter and system work

### Encounter groups

Encounter-group ECS should also wait until encounter orchestration becomes
deeper than "roll encounter, spawn scene, resolve scene."

## Decision

Harsh Realm should defer ambient-world ECS as an implementation target for now.

More specifically:

- do not add faction patrol or expedition ECS yet
- do not add hazard ECS yet
- do not add ambient encounter-group ECS yet

The future priority order should be:

1. hazards and encounter groups, once they acquire richer runtime mechanics
2. faction patrols and expeditions, once they become persistent world actors

## Revisit Triggers

Revisit hazard and encounter ECS when at least two of the following become
true:

1. traps or environmental hazards have persistent runtime instances
2. hazards need timers, trigger volumes, occupancy checks, or delayed effects
3. encounters gain group-level state such as pursuit, reinforcement, morale, or
   retreat across multiple ticks
4. weather or regional effects begin applying mechanical modifiers over time and
   space

Revisit faction-presence ECS when at least two of the following become true:

1. patrols or expeditions exist as persistent map presences rather than table
   modifiers
2. faction units move between hexes or sites over time
3. patrols can be tracked, avoided, intercepted, or redirected
4. faction ambient entities need shared runtime behavior with actors, hazards,
   or spatial triggers

## Recommended Future Rollout

When those thresholds are met, the rollout should be staged.

1. hazard ECS
   - traps
   - danger zones
   - weather or environmental effects
2. encounter-group ECS
   - roaming hostile groups
   - ambush controllers
   - reinforcement or pursuit groups
3. faction-presence ECS
   - patrols
   - expeditions
   - mobile threat sources

This ordering keeps the next ECS work focused on systems that are clearly
runtime-driven before introducing a broader world simulation layer.

## Consequences

Positive:

- keeps ECS targeted at domains with real runtime complexity
- avoids inventing persistent patrol or hazard entities before gameplay needs
  them
- preserves the current simpler boundary between faction strategy, encounters,
  and world traversal

Costs:

- future hazard and faction-presence work will need another structural pass
- some ambient-world logic will remain scene- and table-driven until those
  mechanics mature

## Related Work

- [docs/ecs_adr.md](/home/cyrex/Projects/harsh_realm/docs/ecs_adr.md)
- [docs/ecs_entity_inventory.md](/home/cyrex/Projects/harsh_realm/docs/ecs_entity_inventory.md)
- [docs/ecs_component_catalog.md](/home/cyrex/Projects/harsh_realm/docs/ecs_component_catalog.md)
- [docs/model_ecs_plan.md](/home/cyrex/Projects/harsh_realm/docs/model_ecs_plan.md)
