# Typed Events Design Note

## Summary

`GameEvent` remains the canonical transport envelope for the event bus, event log,
and websocket broadcast path. Typed event classes are a validation layer on top of
that envelope for stable event families whose payload shape is known and worth
protecting with a contract.

## Model

- `GameEvent` owns transport metadata: `id`, `tick`, `event_type`, `source`,
  `timestamp`, and the JSON-safe `data` payload.
- Typed event classes wrap a specific `event_type` plus a typed `PayloadModel`
  instance exposed as `payload`.
- Conversion is explicit:
  - `TypedGameEvent.to_game_event()` serializes a typed payload back into the
    `GameEvent.data` envelope without changing downstream bus behavior.
  - `TypedGameEvent.from_game_event()` validates both `event_type` and payload
    shape before reconstructing the typed wrapper.

## Goals

- Keep the existing bus, logging, and websocket surfaces centered on `GameEvent`.
- Validate stable payload contracts close to producers and consumers.
- Reuse existing `PayloadModel` subclasses from `payloads.py`.
- Allow gradual adoption: typed wrappers can coexist with raw `GameEvent`
  producers and consumers during migration.

## Non-Goals

- Replacing `GameEvent` with typed events at the bus boundary.
- Requiring every event type in the system to gain a wrapper immediately.
- Changing published event names or payload JSON for existing clients.

## Initial Scope

The first narrow family covers stable runtime events already backed by payload
models:

- `gm.narrate`
- `character.death`
- `combat.start`
- `combat.attack`
- `combat.player_hit`
- `exploration.encounter`

These are good starters because they already use dedicated payload models and are
read in multiple places.

## Registry

A small typed-event registry maps known `event_type` strings to wrapper classes.
This avoids ad hoc `if`/`match` parsing in consumers that want a typed event when
available while still returning `None` for unknown event families.
