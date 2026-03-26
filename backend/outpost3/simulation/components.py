"""ECS-Lite components — dataclass-based state containers.

Components are pure data. Systems read and write them via the World.
"""

from __future__ import annotations

from dataclasses import dataclass, field


# ── Ship ──────────────────────────────────────────────────────────

@dataclass
class ShipHull:
    """Physical ship state."""
    name: str
    integrity: float = 100.0   # 0–100 %
    max_integrity: float = 100.0


# ── Resources ─────────────────────────────────────────────────────

@dataclass
class Resources:
    """Consumable supplies aboard the ship."""
    food: int = 0         # person-days of food
    water: int = 0        # person-days of water
    medicine: int = 0     # treatment units
    fuel: int = 0         # propulsion units
    spare_parts: int = 0  # repair units

    def to_dict(self) -> dict[str, int]:
        return {
            "food": self.food,
            "water": self.water,
            "medicine": self.medicine,
            "fuel": self.fuel,
            "spare_parts": self.spare_parts,
        }


# ── Population ────────────────────────────────────────────────────

@dataclass
class Population:
    """Colonist headcount and yearly vital statistics."""
    count: int = 0
    births_this_year: int = 0
    deaths_this_year: int = 0

    def to_dict(self) -> dict[str, int]:
        return {
            "count": self.count,
            "births_this_year": self.births_this_year,
            "deaths_this_year": self.deaths_this_year,
        }


# ── Notable ───────────────────────────────────────────────────────

@dataclass
class Notable:
    """A named individual with a role and personality traits."""
    name: str
    role: str
    traits: list[str] = field(default_factory=list)
    alive: bool = True

    def to_dict(self) -> dict:
        return {
            "name": self.name,
            "role": self.role,
            "traits": list(self.traits),
            "alive": self.alive,
        }


# ── Phase ─────────────────────────────────────────────────────────

PHASES = ("loadout", "search", "transit", "survey", "founding")


@dataclass
class GamePhase:
    """Current narrative phase of the mission."""
    phase: str = "loadout"   # one of PHASES
