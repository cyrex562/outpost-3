"""Search Phase — procedural discovery of candidate star systems.

Runs once after Loadout Phase. Generates 3–5 candidate star systems,
scores them by habitability, selects the best destination, creates a
Transit component, and advances the game phase to "transit".

All randomness injectable for deterministic tests.
"""

from __future__ import annotations

import random as _random_module
from dataclasses import dataclass

from ..simulation import GameEvent, GameTime, Severity
from .components import CandidateSystem, GamePhase, Planet, Resources, ShipHull, Survey, Transit
from .world import World

# ── Star name pools ────────────────────────────────────────────────

_GREEK = [
    "Alpha", "Beta", "Gamma", "Delta", "Epsilon",
    "Zeta", "Eta", "Theta", "Iota", "Kappa",
]
_CONSTELLATIONS = [
    "Eridani", "Ceti", "Ophiuchi", "Aurigae", "Leporis",
    "Pavonis", "Gruis", "Tucanae", "Hydri", "Reticuli",
    "Doradus", "Velorum", "Carinae", "Puppis", "Columbae",
]
_CATALOG_PREFIXES = ["GJ", "HD", "Ross", "Wolf", "Lalande", "TW"]

# ── Star types and their properties ───────────────────────────────

# (designation, friendly name, habitability modifier)
_STAR_TYPES: list[tuple[str, str, float]] = [
    ("G2V", "Sun-like (G)", +10.0),
    ("G8V", "Sun-like (G)", +8.0),
    ("K2V", "Orange dwarf (K)", +5.0),
    ("K5V", "Orange dwarf (K)", +3.0),
    ("M1V", "Red dwarf (M)", -5.0),
    ("M3V", "Red dwarf (M)", -8.0),
    ("F7V", "White-yellow (F)", -3.0),
]

# ── Planet data ────────────────────────────────────────────────────

# type → (base_habitability_min, base_habitability_max, weight)
_PLANET_TYPES: dict[str, tuple[float, float, int]] = {
    "rocky":     (5.0,  78.0, 5),
    "ocean":     (30.0, 85.0, 2),
    "frozen":    (0.0,  22.0, 3),
    "barren":    (0.0,  12.0, 4),
    "volcanic":  (0.0,   8.0, 2),
    "gas_giant": (0.0,   0.0, 3),
}

_PLANET_NOTES: dict[str, list[str]] = {
    "rocky": [
        "Nitrogen-oxygen atmosphere detected",
        "Liquid water confirmed on surface",
        "Temperate climate in equatorial regions",
        "Thin atmosphere, surface pressure marginal",
        "Dense CO₂ atmosphere — extreme greenhouse effect",
        "Arid surface, subsurface ice possible",
        "Habitable zone positioning confirmed",
        "Seasonal temperature variation within tolerable range",
    ],
    "ocean": [
        "Global ocean world, no surface landmass",
        "Biochemistry signatures in upper atmosphere",
        "Deep ocean — platform colony feasible",
        "Tidal patterns suggest internal heating",
    ],
    "frozen": [
        "Ice-covered surface, subsurface ocean suspected",
        "Tidal heating from nearby gas giant",
        "Minimal atmosphere, extreme cold",
        "Resource extraction potential moderate",
    ],
    "barren": [
        "Rocky, airless surface",
        "Trace atmosphere, heavy radiation exposure",
        "Iron-rich regolith — resource potential",
        "Cratered terrain, geologically inactive",
    ],
    "volcanic": [
        "Active volcanic system — extreme surface hazard",
        "Sulfur dioxide atmosphere, uninhabitable surface",
        "Geothermal energy potential",
    ],
    "gas_giant": [
        "Jovian-class gas giant",
        "Multiple moons — secondary survey recommended",
        "Strong magnetic field — radiation hazard for moons",
    ],
}


# ── Generation helpers ─────────────────────────────────────────────

def _star_name(rng: _random_module.Random, used: set[str]) -> str:
    for _ in range(50):
        if rng.random() < 0.5:
            name = f"{rng.choice(_GREEK)} {rng.choice(_CONSTELLATIONS)}"
        else:
            prefix = rng.choice(_CATALOG_PREFIXES)
            number = rng.randint(100, 9999)
            name = f"{prefix} {number}"
        if name not in used:
            used.add(name)
            return name
    return f"System-{rng.randint(10000, 99999)}"


def _planet_name(system_name: str, index: int) -> str:
    letters = "bcdefghij"
    return f"{system_name} {letters[index]}"


def _generate_planet(
    rng: _random_module.Random,
    system_name: str,
    index: int,
    star_hab_mod: float,
) -> Planet:
    # Weighted random planet type
    types = list(_PLANET_TYPES.keys())
    weights = [_PLANET_TYPES[t][2] for t in types]
    planet_type = rng.choices(types, weights=weights, k=1)[0]

    lo, hi, _ = _PLANET_TYPES[planet_type]
    base_hab = rng.uniform(lo, hi)
    hab = max(0.0, min(100.0, base_hab + star_hab_mod * 0.5))

    # Pick 1–2 notes for flavor
    note_pool = _PLANET_NOTES.get(planet_type, [])
    notes = rng.sample(note_pool, min(rng.randint(1, 2), len(note_pool)))

    return Planet(
        name=_planet_name(system_name, index),
        type=planet_type,
        habitability=round(hab, 1),
        notes=notes,
    )


def _generate_system(
    rng: _random_module.Random,
    used_names: set[str],
    angle_deg: float,
) -> CandidateSystem:
    name = _star_name(rng, used_names)
    star_type_data = rng.choice(_STAR_TYPES)
    star_type, _, hab_mod = star_type_data
    distance_ly = round(rng.uniform(4.0, 22.0), 1)

    num_planets = rng.randint(1, 4)
    planets = [
        _generate_planet(rng, name, i, hab_mod)
        for i in range(num_planets)
    ]
    best_hab = max((p.habitability for p in planets), default=0.0)

    return CandidateSystem(
        name=name,
        star_type=star_type,
        distance_ly=distance_ly,
        angle_deg=round(angle_deg, 1),
        planets=planets,
        best_habitability=round(best_hab, 1),
    )


# ── Public interface ───────────────────────────────────────────────

@dataclass
class SearchResult:
    system_entities: list[int]
    transit_entity: int
    survey_entity: int
    destination_name: str
    transit_years: int


def run_search(
    world: World,
    *,
    rng: _random_module.Random | None = None,
    rejection_count: int = 0,
) -> tuple[SearchResult, list[GameEvent]]:
    """Generate candidate systems, select the best, create Transit + Survey.

    Advances the GamePhase to "transit".

    Args:
        world: populated ECS World (must already contain ShipHull + Resources)
        rng: optional Random instance for deterministic tests
        rejection_count: how many times a system has already been rejected
    """
    if rng is None:
        rng = _random_module.Random()

    game_time = GameTime(0)
    events: list[GameEvent] = []

    # ── Generate 3–5 candidate systems ────────────────────────────
    num_systems = rng.randint(3, 5)
    used_names: set[str] = set()

    # Spread systems around the sky (angles spaced out with some jitter)
    base_angles = [360 * i / num_systems for i in range(num_systems)]
    angles = [a + rng.uniform(-20, 20) for a in base_angles]

    systems: list[CandidateSystem] = [
        _generate_system(rng, used_names, angle)
        for angle in angles
    ]

    # ── Register systems in World ──────────────────────────────────
    system_eids: list[int] = []
    for system in systems:
        eid = world.new_entity()
        world.add(eid, system)
        system_eids.append(eid)

        # Best planet for narrative
        best_planet = max(system.planets, key=lambda p: p.habitability)
        events.append(GameEvent(
            event_type="search.candidate_found",
            severity=Severity.NOTABLE,
            game_time=game_time,
            data={
                "name": system.name,
                "star_type": system.star_type,
                "distance_ly": system.distance_ly,
                "planet_count": len(system.planets),
                "best_planet": best_planet.name,
                "best_habitability": system.best_habitability,
            },
        ))

    # ── Select best destination ────────────────────────────────────
    best_system = max(systems, key=lambda s: s.best_habitability)
    best_system.selected = True

    # ── Determine transit duration (20–50 years) ───────────────────
    # Transit years scales loosely with distance but is ultimately driven
    # by the ship's propulsion — we use distance as a floor.
    min_years = max(20, int(best_system.distance_ly * 1.5))
    max_years = min(50, min_years + rng.randint(5, 20))
    transit_years = rng.randint(min_years, max_years)
    transit_days = transit_years * 365

    # ── Fuel burn rate from existing reserves ─────────────────────
    fuel_per_day = 0.0
    ship_pair = world.query_one(ShipHull)
    if ship_pair:
        ship_eid, _ = ship_pair
        res = world.get(ship_eid, Resources)
        if res and transit_days > 0:
            fuel_per_day = res.fuel / transit_days

    # ── Create Transit component ───────────────────────────────────
    transit_eid = world.new_entity()
    world.add(transit_eid, Transit(
        destination_name=best_system.name,
        duration_days=transit_days,
        fuel_per_day=fuel_per_day,
    ))

    # ── Create Survey component for the upcoming survey ───────────
    survey_eid = world.new_entity()
    world.add(survey_eid, Survey(
        target_system_name=best_system.name,
        rejection_count=rejection_count,
    ))

    # ── Advance game phase ─────────────────────────────────────────
    phase_pair = world.query_one(GamePhase)
    if phase_pair:
        phase_pair[1].phase = "transit"

    # ── Destination selected event (auto-pause = player reads the news) ──
    events.append(GameEvent(
        event_type="search.destination_selected",
        severity=Severity.MILESTONE,
        game_time=game_time,
        auto_pause=True,
        data={
            "name": best_system.name,
            "star_type": best_system.star_type,
            "distance_ly": best_system.distance_ly,
            "best_habitability": best_system.best_habitability,
            "transit_years": transit_years,
            "candidates_rejected": num_systems - 1,
            "rejection_count": rejection_count,
        },
    ))

    result = SearchResult(
        system_entities=system_eids,
        transit_entity=transit_eid,
        survey_entity=survey_eid,
        destination_name=best_system.name,
        transit_years=transit_years,
    )
    return result, events
