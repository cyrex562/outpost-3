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


# ── Star systems ──────────────────────────────────────────────────

@dataclass
class Planet:
    """A single world in a candidate star system."""
    name: str
    type: str           # rocky | ocean | frozen | barren | volcanic | gas_giant
    habitability: float  # 0–100
    notes: list[str] = field(default_factory=list)

    def to_dict(self) -> dict:
        return {
            "name": self.name,
            "type": self.type,
            "habitability": round(self.habitability, 1),
            "notes": list(self.notes),
        }


@dataclass
class CandidateSystem:
    """A prospective colony destination discovered during the Search Phase."""
    name: str
    star_type: str       # e.g. "G2V", "K5V", "M3V"
    distance_ly: float   # light-years from home
    angle_deg: float     # 2-D map bearing (0–360°)
    planets: list[Planet]
    best_habitability: float = 0.0
    selected: bool = False   # true for the chosen destination

    def to_dict(self) -> dict:
        return {
            "name": self.name,
            "star_type": self.star_type,
            "distance_ly": round(self.distance_ly, 1),
            "angle_deg": round(self.angle_deg, 1),
            "planets": [p.to_dict() for p in self.planets],
            "best_habitability": round(self.best_habitability, 1),
            "selected": self.selected,
        }


# ── Transit ────────────────────────────────────────────────────────

@dataclass
class Transit:
    """State of the interstellar transit phase."""
    destination_name: str
    duration_days: int    # total journey length
    fuel_per_day: float   # propulsion units burned per day
    days_elapsed: int = 0
    arrived: bool = False

    @property
    def progress(self) -> float:
        if self.duration_days == 0:
            return 1.0
        return min(1.0, self.days_elapsed / self.duration_days)

    @property
    def days_remaining(self) -> int:
        return max(0, self.duration_days - self.days_elapsed)

    def to_dict(self) -> dict:
        return {
            "destination_name": self.destination_name,
            "duration_days": self.duration_days,
            "days_elapsed": self.days_elapsed,
            "days_remaining": self.days_remaining,
            "progress": round(self.progress, 4),
            "arrived": self.arrived,
        }


# ── Phase ─────────────────────────────────────────────────────────

PHASES = ("loadout", "search", "transit", "survey", "founding")


@dataclass
class GamePhase:
    """Current narrative phase of the mission."""
    phase: str = "loadout"   # one of PHASES


# ── Survey ────────────────────────────────────────────────────────

@dataclass
class Survey:
    """State of the planetary survey phase at a candidate system."""
    target_system_name: str
    survey_duration_days: int = 365
    days_elapsed: int = 0
    rejection_count: int = 0         # persists across re-searches
    verdict: str | None = None       # "accept" | "reject" | None

    @property
    def progress(self) -> float:
        if self.survey_duration_days == 0:
            return 1.0
        return min(1.0, self.days_elapsed / self.survey_duration_days)

    def to_dict(self) -> dict:
        return {
            "target_system_name": self.target_system_name,
            "survey_duration_days": self.survey_duration_days,
            "days_elapsed": self.days_elapsed,
            "rejection_count": self.rejection_count,
            "verdict": self.verdict,
            "progress": round(self.progress, 4),
        }


# ── Colony ────────────────────────────────────────────────────────

@dataclass
class Colony:
    """Founding conditions of the established colony."""
    system_name: str
    planet_name: str
    habitability: float
    founded_year: int
    starting_population: int
    hull_integrity_at_landing: float
    quality: str   # "struggling" | "standard" | "strong" | "thriving"

    def to_dict(self) -> dict:
        return {
            "system_name": self.system_name,
            "planet_name": self.planet_name,
            "habitability": round(self.habitability, 1),
            "founded_year": self.founded_year,
            "starting_population": self.starting_population,
            "hull_integrity_at_landing": round(self.hull_integrity_at_landing, 1),
            "quality": self.quality,
        }
