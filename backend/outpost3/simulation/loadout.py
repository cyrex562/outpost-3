"""Loadout Phase — procedural generation of starting ship, crew, and notables.

Runs once at game start. Populates the World and returns a list of GameEvents
(ship.loadout_complete + notable.introduced × N) for narrative broadcast.

Design: all randomness is seeded via the stdlib random module; pass a custom
Random instance in tests for determinism.
"""

from __future__ import annotations

import random as _random_module
from dataclasses import dataclass

from ..simulation import GameEvent, GameTime, Severity
from .components import (
    CandidateSystem, Colony, GamePhase, Notable, Population,
    Resources, ShipHull, Survey, Transit,
)
from .world import World

# ── Name pools ────────────────────────────────────────────────────

_SHIP_ADJECTIVES = [
    "Persistent", "Undaunted", "Resolute", "Steadfast", "Enduring",
    "Relentless", "Boundless", "Unflinching", "Unyielding", "Eternal",
    "Hopeful", "Radiant", "Sovereign", "Valiant", "Ascendant",
]

_SHIP_NOUNS = [
    "Horizon", "Frontier", "Endeavour", "Exodus", "Covenant",
    "Pioneer", "Pilgrim", "Wanderer", "Vanguard", "Harbinger",
    "Beacon", "Threshold", "Meridian", "Solstice", "Perihelion",
]

_FIRST_NAMES = [
    "Elena", "Marcus", "Yuki", "Amara", "Chen",
    "Priya", "James", "Fatima", "Alejandro", "Ingrid",
    "Obi", "Miriam", "Konstantin", "Sana", "Diego",
    "Leila", "Viktor", "Nadia", "Samuel", "Lin",
    "Tariq", "Esme", "Darius", "Chioma", "Helene",
    "Rafael", "Zara", "Ivan", "Mei", "Tobias",
]

_LAST_NAMES = [
    "Vasquez", "Chen", "Nakamura", "Okonkwo", "Morrison",
    "Sharma", "al-Rashid", "Bergstrom", "Osei", "Cohen",
    "Petrov", "Reyes", "Schmidt", "Johansson", "Kim",
    "Patel", "Santos", "Mueller", "Adeyemi", "Tanaka",
    "Ferreira", "Lindqvist", "Mbeki", "Hayashi", "Dubois",
]

# Roles in priority order — Captain is always included first.
_ROLES_POOL = [
    "Captain",
    "Chief Engineer",
    "Chief Medical Officer",
    "Chief Science Officer",
    "Security Chief",
    "Quartermaster",
    "Lead Agronomist",
    "Chief Navigator",
    "Mission Commander",
]

_TRAITS_POOL = [
    "bold", "cautious", "pragmatic", "idealistic", "methodical",
    "creative", "empathetic", "stoic", "ambitious", "compassionate",
    "analytical", "decisive", "reckless", "innovative", "traditional",
    "resilient", "charismatic", "introspective", "tenacious", "diplomatic",
]


# ── Generation helpers ────────────────────────────────────────────

def _ship_name(rng: _random_module.Random) -> str:
    adj = rng.choice(_SHIP_ADJECTIVES)
    noun = rng.choice(_SHIP_NOUNS)
    return f"ISS {adj} {noun}"


def _notable_name(rng: _random_module.Random, used: set[str]) -> str:
    for _ in range(50):
        name = f"{rng.choice(_FIRST_NAMES)} {rng.choice(_LAST_NAMES)}"
        if name not in used:
            used.add(name)
            return name
    return f"Colonist #{rng.randint(1000, 9999)}"


def _generate_notables(
    rng: _random_module.Random,
    count: int,
) -> list[tuple[str, str, list[str]]]:
    """Return list of (name, role, traits) tuples."""
    used_names: set[str] = set()
    # Captain always first; shuffle the rest, pick count-1 more
    roles = [_ROLES_POOL[0]] + rng.sample(_ROLES_POOL[1:], count - 1)
    result = []
    for role in roles:
        name = _notable_name(rng, used_names)
        num_traits = rng.randint(2, 3)
        traits = rng.sample(_TRAITS_POOL, num_traits)
        result.append((name, role, traits))
    return result


def _starting_resources(rng: _random_module.Random, pop: int) -> Resources:
    """Scale resources for a generation ship voyage (15–35 years of reserves).

    Food and water are stored reserves beyond closed-loop life support.
    Fuel is interstellar propulsion; spare parts cover decades of maintenance.
    ±20 % variance creates meaningful resource uncertainty for the player.
    """
    def vary(base: int) -> int:
        return int(base * rng.uniform(0.80, 1.20))

    years_reserve = rng.randint(15, 35)   # unknown voyage length creates tension
    return Resources(
        food=vary(pop * 365 * years_reserve),
        water=vary(pop * 365 * (years_reserve + 5)),  # extra buffer (recycled)
        medicine=vary(pop * 100),
        fuel=vary(1_000_000),
        spare_parts=vary(100_000),
    )


# ── Public interface ──────────────────────────────────────────────

@dataclass
class LoadoutResult:
    """Snapshot of what was generated, for easy serialisation."""
    ship_entity: int
    pop_entity: int
    phase_entity: int
    notable_entities: list[int]


def run_loadout(
    world: World,
    *,
    rng: _random_module.Random | None = None,
) -> tuple[LoadoutResult, list[GameEvent]]:
    """Populate the World with starting state and return generated events.

    Args:
        world: the ECS World to populate (must be empty)
        rng: optional Random instance for deterministic tests

    Returns:
        (LoadoutResult, list[GameEvent]) — result metadata and events to emit
    """
    if rng is None:
        rng = _random_module.Random()

    game_time = GameTime(0)
    events: list[GameEvent] = []

    # ── Ship hull ──────────────────────────────────────────────────
    ship_eid = world.new_entity()
    hull = ShipHull(name=_ship_name(rng))
    world.add(ship_eid, hull)

    # ── Population ────────────────────────────────────────────────
    pop_eid = world.new_entity()
    pop_count = rng.randint(500, 2000)
    pop = Population(count=pop_count)
    world.add(pop_eid, pop)

    # ── Resources ─────────────────────────────────────────────────
    resources = _starting_resources(rng, pop_count)
    world.add(ship_eid, resources)   # resources live on the ship entity

    # ── Game phase ────────────────────────────────────────────────
    phase_eid = world.new_entity()
    world.add(phase_eid, GamePhase(phase="loadout"))

    # ── Notables ──────────────────────────────────────────────────
    notable_count = rng.randint(5, 8)
    notable_raw = _generate_notables(rng, notable_count)
    notable_eids: list[int] = []
    for name, role, traits in notable_raw:
        eid = world.new_entity()
        world.add(eid, Notable(name=name, role=role, traits=traits))
        notable_eids.append(eid)

        events.append(GameEvent(
            event_type="notable.introduced",
            severity=Severity.INFO,
            game_time=game_time,
            data={
                "name": name,
                "role": role,
                "traits": traits,
                "trait_list": ", ".join(traits),
            },
        ))

    # ── Loadout complete event (auto-pause so player can read roster) ──
    events.append(GameEvent(
        event_type="ship.loadout_complete",
        severity=Severity.MILESTONE,
        game_time=game_time,
        auto_pause=True,
        data={
            "ship_name": hull.name,
            "population": pop_count,
            "notable_count": notable_count,
        },
    ))

    result = LoadoutResult(
        ship_entity=ship_eid,
        pop_entity=pop_eid,
        phase_entity=phase_eid,
        notable_entities=notable_eids,
    )
    return result, events


def world_state_dict(world: World) -> dict:
    """Serialise the current world state for REST / WebSocket clients."""
    # Ship hull
    ship_data: dict = {}
    resources_data: dict = {}
    ship_pair = world.query_one(ShipHull)
    if ship_pair:
        ship_eid, hull = ship_pair
        ship_data = {"name": hull.name, "integrity": hull.integrity}
        res = world.get(ship_eid, Resources)
        if res:
            resources_data = res.to_dict()

    # Population
    pop_data: dict = {}
    pop_pair = world.query_one(Population)
    if pop_pair:
        _, pop = pop_pair
        pop_data = pop.to_dict()

    # Phase
    phase = "unknown"
    phase_pair = world.query_one(GamePhase)
    if phase_pair:
        _, gp = phase_pair
        phase = gp.phase

    # Notables
    notables = [n.to_dict() for _, n in world.query(Notable)]

    # Candidate star systems
    systems = [s.to_dict() for _, s in world.query(CandidateSystem)]

    # Transit progress
    transit_data: dict = {}
    transit_pair = world.query_one(Transit)
    if transit_pair:
        _, tr = transit_pair
        transit_data = tr.to_dict()

    # Survey progress
    survey_data: dict = {}
    survey_pair = world.query_one(Survey)
    if survey_pair:
        _, sv = survey_pair
        survey_data = sv.to_dict()

    # Colony (if founded)
    colony_data: dict = {}
    colony_pair = world.query_one(Colony)
    if colony_pair:
        _, col = colony_pair
        colony_data = col.to_dict()

    return {
        "type": "world_state",
        "phase": phase,
        "ship": ship_data,
        "population": pop_data,
        "resources": resources_data,
        "notables": notables,
        "systems": systems,
        "transit": transit_data,
        "survey": survey_data,
        "colony": colony_data,
    }
