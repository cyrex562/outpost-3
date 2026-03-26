"""ECS-Lite World container.

Entities are integer IDs. Components are dataclass instances stored in a
type-keyed registry. Systems query by component type.

Design goals:
- No archetype tables or bitmasking — clean and simple for M1
- Typed queries via Type[C] — works at runtime without mypy magic
- Easy to extend toward full ECS later
"""

from __future__ import annotations

from typing import Any, Type, TypeVar

C = TypeVar("C")


class World:
    """Central registry of entities and their components."""

    def __init__(self) -> None:
        self._next_id: int = 0
        # comp_type → {entity_id → component}
        self._components: dict[type, dict[int, Any]] = {}

    # ── Entity lifecycle ───────────────────────────────────────────

    def new_entity(self) -> int:
        """Allocate a fresh entity ID."""
        eid = self._next_id
        self._next_id += 1
        return eid

    # ── Component storage ──────────────────────────────────────────

    def add(self, entity_id: int, component: Any) -> None:
        """Attach a component to an entity. Overwrites if already present."""
        comp_type = type(component)
        if comp_type not in self._components:
            self._components[comp_type] = {}
        self._components[comp_type][entity_id] = component

    def get(self, entity_id: int, comp_type: Type[C]) -> C | None:
        """Retrieve a single component for an entity, or None."""
        return self._components.get(comp_type, {}).get(entity_id)

    def remove(self, entity_id: int, comp_type: type) -> None:
        """Remove a component from an entity if present."""
        self._components.get(comp_type, {}).pop(entity_id, None)

    # ── Queries ────────────────────────────────────────────────────

    def query(self, comp_type: Type[C]) -> list[tuple[int, C]]:
        """Return all (entity_id, component) pairs of the given type."""
        return list(self._components.get(comp_type, {}).items())

    def query_one(self, comp_type: Type[C]) -> tuple[int, C] | None:
        """Return the first (entity_id, component) pair, or None."""
        pairs = self._components.get(comp_type, {})
        for eid, comp in pairs.items():
            return eid, comp
        return None

    # ── Convenience ────────────────────────────────────────────────

    def has(self, entity_id: int, comp_type: type) -> bool:
        return entity_id in self._components.get(comp_type, {})

    def clear_type(self, comp_type: type) -> None:
        """Remove all components of the given type (used for phase resets)."""
        self._components.pop(comp_type, None)

    def entity_count(self) -> int:
        return self._next_id
