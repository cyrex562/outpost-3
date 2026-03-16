"""Component dataclasses for the colony ship simulation.

Components are pure data attached to entities via the World container.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

from .systems import Phase


@dataclass
class FactionPhase:
    """Tracks which phase the faction is currently in."""
    phase: Phase = Phase.LOADOUT


@dataclass
class Resources:
    """Ship resource stores."""
    materials: float = 0.0
    fuel: float = 0.0
    food: float = 0.0
    water: float = 0.0
    spare_parts: float = 0.0

    def to_summary(self) -> dict[str, float]:
        return {
            "materials": round(self.materials, 1),
            "fuel": round(self.fuel, 1),
            "food": round(self.food, 1),
            "water": round(self.water, 1),
            "spare_parts": round(self.spare_parts, 1),
        }


@dataclass
class Population:
    """Crew population tracking."""
    total: int = 0
    scientists: int = 0
    engineers: int = 0
    medical: int = 0
    military: int = 0
    civilians: int = 0
    births_year: int = 0
    deaths_year: int = 0
    morale: float = 0.75  # 0.0–1.0


@dataclass
class ShipSystems:
    """Ship system health values (0.0–1.0 each)."""
    hull_integrity: float = 1.0
    engines: float = 1.0
    life_support: float = 1.0
    sensors: float = 1.0

    def to_summary(self) -> dict[str, float]:
        return {
            "hull_integrity": round(self.hull_integrity, 3),
            "engines": round(self.engines, 3),
            "life_support": round(self.life_support, 3),
            "sensors": round(self.sensors, 3),
        }


@dataclass
class Notable:
    """A named individual on the ship with role and personality traits."""
    name: str = ""
    role: str = ""
    age: int = 30
    traits: list[str] = field(default_factory=list)
    alive: bool = True
    death_day: int | None = None


@dataclass
class Loadout:
    """Immutable record of the ship's starting configuration."""
    crew_size: int = 0
    resources: dict[str, float] = field(default_factory=dict)
    ship_systems: dict[str, float] = field(default_factory=dict)
    notable_names: list[str] = field(default_factory=list)
    ship_name: str = ""


# ── Trait definitions ────────────────────────────────────────────

TRAIT_AXES: list[tuple[str, str]] = [
    ("cautious", "bold"),
    ("crew_welfare", "mission_focus"),
    ("resourceful", "conservative"),
    ("optimistic", "pragmatic"),
]

ALL_TRAITS: list[str] = [t for pair in TRAIT_AXES for t in pair]

ROLES: list[str] = [
    "Captain",
    "Chief Engineer",
    "Chief Medical Officer",
    "Head of Agriculture",
    "Chief Scientist",
    "Security Chief",
    "Navigator",
    "Quartermaster",
]
