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


@dataclass
class StarSystem:
    """A discovered candidate star system."""
    name: str = ""
    distance_ly: float = 0.0  # light-years from departure
    habitability_score: float = 0.0  # 0.0–1.0 aggregate
    num_planets: int = 0
    selected: bool = False  # chosen as destination
    surveyed: bool = False
    rejected: bool = False


@dataclass
class Planet:
    """A planet within a star system."""
    name: str = ""
    parent_system_id: int = -1
    planet_type: str = "rocky"  # rocky, gas, ice
    size: float = 1.0  # Earth radii
    atmosphere: float = 0.0  # 0.0–1.0 (0=none, 1=earth-like)
    water: float = 0.0  # 0.0–1.0 fraction
    minerals: float = 0.0  # 0.0–1.0 richness
    habitability: float = 0.0  # 0.0–1.0 computed


@dataclass
class SearchProgress:
    """Tracks the search phase progress."""
    systems_discovered: int = 0
    target_discoveries: int = 4  # discover this many before choosing
    days_since_last_discovery: int = 0
    discovery_chance_per_day: float = 0.008  # ~3 per year


@dataclass
class TransitState:
    """Tracks the ship's transit between star systems."""
    destination_id: int = -1  # entity ID of target StarSystem
    distance_ly: float = 0.0  # total distance
    distance_traveled_ly: float = 0.0  # how far we've come
    cruise_velocity_c: float = 0.5  # fraction of c
    transit_phase: str = "accelerating"  # accelerating, cruising, decelerating
    accel_days: int = 60  # days to accelerate
    decel_days: int = 60  # days to decelerate
    accel_done_day: int = 0
    decel_start_day: int = 0
    departure_day: int = 0
    eta_days: int = 0  # estimated total transit days


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
