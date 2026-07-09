# Python Typing Inventory

This document inventories current `object` and `Any` usage in active runtime
code under `src/harsh_realm` and classifies each usage as:

- acceptable boundary
- temporary migration shim
- violation

The goal is to focus cleanup on the highest-risk runtime paths first.

## Classification Rules

### Acceptable boundary

Allowed only where the code is genuinely dynamic and the type is local to a
framework or protocol boundary.

Typical examples:

- `__contains__(self, key: object) -> bool`
- DB parameter tuples using `tuple[Any, ...]`
- Pydantic validators that accept unknown input before narrowing

### Temporary migration shim

A type escape hatch that exists to preserve compatibility during an ongoing
migration, but should be removed once the surrounding code is typed.

Typical examples:

- `list[dict[str, Any]] | list[DungeonRoom]`
- `SceneNpcRecord | dict[str, object]`
- repository update helpers using `**kwargs: Any`

### Violation

A convenience use of `object` or `Any` where a `Protocol`, Pydantic model,
`JsonObject`/`JsonValue`, typed row alias, or concrete union should be used.

Typical examples:

- `app_state: Any`
- `event_bus: Any`
- `narrator: Any`
- `dict[str, object]` for structured payloads
- `-> object` return types

## Inventory

## 1. Acceptable Boundaries

These are acceptable and low priority.

| Location | Usage | Classification | Notes |
| --- | --- | --- | --- |
| `src/harsh_realm/payloads.py:27` | `__contains__(self, key: object)` | acceptable boundary | Standard membership protocol shape. |
| `src/harsh_realm/models/creature.py:21` | `__contains__(self, key: object)` | acceptable boundary | Standard membership protocol shape. |
| `src/harsh_realm/models/faction.py:188` | `__contains__(self, key: object)` | acceptable boundary | Standard membership protocol shape. |
| `src/harsh_realm/models/generation.py:117` | `__contains__(self, key: object)` | acceptable boundary | Standard membership protocol shape. |
| `src/harsh_realm/db.py:335,350,364` | `tuple[Any, ...]` SQL params | acceptable boundary | DB wrapper boundary; SQL params are intentionally heterogeneous. |
| `src/harsh_realm/models/entity_state.py:119` | validator `data: Any -> Any` | acceptable boundary | Pydantic normalization boundary. |
| `src/harsh_realm/models/npc.py:33` | validator `data: Any -> Any` | acceptable boundary | Pydantic normalization boundary. |

## 2. Temporary Migration Shims

These are understandable in the current architecture, but should be removed as
nearby modules are tightened.

| Location | Usage | Classification | Notes |
| --- | --- | --- | --- |
| `src/harsh_realm/gm/scenes/dungeon.py:44-45` | `list[dict[str, Any]] | list[DungeonRoom]` and `list[dict[str, Any]] | list[DungeonConnection]` | temporary migration shim | Legacy/raw room payload support should disappear once all callers are typed. |
| `src/harsh_realm/gm/scenes/social.py:51` | `SceneNpcRecord | dict[str, object]` | temporary migration shim | Legacy dict-style NPC scene payload still supported. |
| `src/harsh_realm/generators/npc_gen.py:57` | `NPCGenerationContext | dict[str, object] | None` | temporary migration shim | Should converge on one typed context model. |
| `src/harsh_realm/faction/repository.py:117,170,211,280` | `**kwargs: Any` / `**fields: Any` | temporary migration shim | Repository patch APIs need explicit patch models or field unions. |
| `src/harsh_realm/admin/service.py:275` | `**fields: Any` | temporary migration shim | Same issue as faction repository. |
| `src/harsh_realm/admin/content_mixin.py:117,175` | `JsonObjectDocument | dict[str, object]` | temporary migration shim | Transitional dual support; should collapse onto typed wrapper. |
| `src/harsh_realm/engine/healing.py:202` | `InventoryItemRecord | dict[str, object]` | temporary migration shim | Legacy item payload acceptance should be narrowed away. |
| `src/harsh_realm/engine/character_recalc.py:73,140` | `list[dict[str, object]]` equipment | temporary migration shim | Should become `list[InventoryItemRecord]` or equivalent typed payload model. |
| `src/harsh_realm/gm/entity_state_repository.py:138` | `data: dict[str, object]` | temporary migration shim | NPC typed persistence payload should become a dedicated model. |
| `src/harsh_realm/models/faction.py:31` | `payload: JsonObject | None` | temporary migration shim | Transitional boundary for older faction payloads. |

## 3. Violations

These are the priority cleanup targets because they weaken static analysis in
active runtime code.

### High Priority

Latest scan found no remaining high-priority violations. The previous entries
were closed by adding protocol-typed runtime app state, concrete controller
collaborator types, typed exploration persistence helpers, `FleeOpponent`
returns for combat support, and replacing the character raw-row return path
with a typed `EntityRecord`.

### Medium Priority

Latest scan found no remaining medium-priority violations from the previous
inventory. The event bus accessors now use `EventBus`, pending combat scenes
use concrete `CombatScene | None` annotations, respawn commands use
`ParsedCommand`, equipment kit caches use `list[EquipmentKit] | None`,
table export returns are concrete, item lookup returns `InventoryItemRecord`,
and dungeon update payloads use scalar update values.

### Lower Priority / Mechanical Cleanup

Latest scan found no remaining lower-priority violations from the previous
inventory. SQL parameter lists are narrowed to scalar unions, editor update
payloads use concrete scalar/JSON aliases, websocket transport returns
`JsonObject`, cell features are `list[str]`, shopping dispatch is callable-typed,
social NPC payloads use `SceneNpcRecord | JsonObject`, and admin routes no
longer import `Any`.

## Summary

### Acceptable boundaries

- DB parameter tuples
- Pydantic validator normalization hooks
- `__contains__(..., key: object)` protocol-compatible helpers

### Temporary migration shims

- raw dict support at legacy scene/repository boundaries
- patch/update helpers using open-ended `**kwargs`
- typed model plus dict dual-support paths that should collapse later

### Violations

The high-, medium-, and lower-priority violations from the original inventory
are closed. Remaining cleanup work lives in the temporary migration shims and
any newly discovered escape hatches from future scans.

## Recommended Cleanup Order

1. Collapse remaining temporary migration shims onto their typed models.
2. Replace broad repository patch helpers with explicit patch/update models.
3. Re-run this inventory after the next large subsystem or API refactor.
