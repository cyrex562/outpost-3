"""ECS-Lite — World container with entity/component storage and typed queries.

Entities are integer IDs. Components are dataclass instances stored in
dict[type, dict[int, Component]]. Systems query the World each tick.
"""

from __future__ import annotations

from dataclasses import asdict, fields, is_dataclass
from typing import Any, TypeVar, get_type_hints

T = TypeVar("T")


class World:
    """Central container for all entities and their components."""

    def __init__(self) -> None:
        self._next_id: int = 0
        self._components: dict[type, dict[int, Any]] = {}
        self._alive: set[int] = set()

    # ── entity lifecycle ─────────────────────────────────────────

    def create_entity(self) -> int:
        """Create a new entity and return its ID."""
        eid = self._next_id
        self._next_id += 1
        self._alive.add(eid)
        return eid

    def remove_entity(self, entity_id: int) -> None:
        """Remove an entity and all its components."""
        self._alive.discard(entity_id)
        for store in self._components.values():
            store.pop(entity_id, None)

    def entity_exists(self, entity_id: int) -> bool:
        return entity_id in self._alive

    # ── component access ─────────────────────────────────────────

    def add_component(self, entity_id: int, component: Any) -> None:
        """Attach a component instance to an entity."""
        if entity_id not in self._alive:
            raise KeyError(f"Entity {entity_id} does not exist")
        comp_type = type(component)
        if comp_type not in self._components:
            self._components[comp_type] = {}
        self._components[comp_type][entity_id] = component

    def get_component(self, entity_id: int, comp_type: type[T]) -> T | None:
        """Get a single component from an entity, or None."""
        store = self._components.get(comp_type)
        if store is None:
            return None
        return store.get(entity_id)

    def get_components(self, entity_id: int, *comp_types: type) -> tuple | None:
        """Get multiple components from an entity. Returns None if any missing."""
        result = []
        for ct in comp_types:
            store = self._components.get(ct)
            if store is None or entity_id not in store:
                return None
            result.append(store[entity_id])
        return tuple(result)

    def has_component(self, entity_id: int, comp_type: type) -> bool:
        store = self._components.get(comp_type)
        return store is not None and entity_id in store

    def remove_component(self, entity_id: int, comp_type: type) -> None:
        """Remove a component from an entity."""
        store = self._components.get(comp_type)
        if store is not None:
            store.pop(entity_id, None)

    # ── queries ──────────────────────────────────────────────────

    def entities_with(self, *comp_types: type) -> list[int]:
        """Return all entity IDs that have ALL of the given component types."""
        if not comp_types:
            return list(self._alive)

        # Start with the smallest store for efficiency
        stores = []
        for ct in comp_types:
            store = self._components.get(ct)
            if store is None:
                return []
            stores.append(store)

        # Intersect from smallest
        stores.sort(key=len)
        candidates = set(stores[0].keys())
        for store in stores[1:]:
            candidates &= store.keys()

        return [eid for eid in candidates if eid in self._alive]

    def all_components_of_type(self, comp_type: type[T]) -> dict[int, T]:
        """Return dict of entity_id → component for all entities with this type."""
        store = self._components.get(comp_type)
        if store is None:
            return {}
        return {eid: comp for eid, comp in store.items() if eid in self._alive}

    # ── serialization ────────────────────────────────────────────

    def to_dict(self) -> dict[str, Any]:
        """Serialize the entire world to a dict for save/load."""
        components: dict[str, dict[str, Any]] = {}
        for comp_type, store in self._components.items():
            type_key = f"{comp_type.__module__}.{comp_type.__qualname__}"
            entries: dict[str, Any] = {}
            for eid, comp in store.items():
                if eid in self._alive:
                    if is_dataclass(comp) and not isinstance(comp, type):
                        entries[str(eid)] = asdict(comp)
                    elif hasattr(comp, "to_dict"):
                        entries[str(eid)] = comp.to_dict()
                    else:
                        entries[str(eid)] = str(comp)
            if entries:
                components[type_key] = entries

        return {
            "next_id": self._next_id,
            "alive": sorted(self._alive),
            "components": components,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any], type_registry: dict[str, type] | None = None) -> World:
        """Deserialize a world from a dict.

        Args:
            data: Previously serialized world dict.
            type_registry: Mapping of type_key → dataclass type for reconstruction.
                           If None, components are stored as raw dicts.
        """
        world = cls()
        world._next_id = data["next_id"]
        world._alive = set(data["alive"])

        registry = type_registry or {}
        for type_key, entries in data.get("components", {}).items():
            comp_type = registry.get(type_key)
            for eid_str, comp_data in entries.items():
                eid = int(eid_str)
                if comp_type is not None and is_dataclass(comp_type):
                    component = comp_type(**comp_data)
                else:
                    component = comp_data  # store as raw dict
                if comp_type is not None:
                    if comp_type not in world._components:
                        world._components[comp_type] = {}
                    world._components[comp_type][eid] = component

        return world

    def __repr__(self) -> str:
        n_entities = len(self._alive)
        n_comp_types = len(self._components)
        return f"World(entities={n_entities}, component_types={n_comp_types})"
