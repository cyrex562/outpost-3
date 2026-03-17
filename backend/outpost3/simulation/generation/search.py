"""SearchSystem — discovers candidate star systems and selects a destination.

Runs during the SEARCH phase. Periodically discovers new star systems with
procedurally generated planets. After enough discoveries, selects the best
candidate and transitions to TRANSIT.
"""

from __future__ import annotations

import random
import math

from .. import GameEvent, GameTime, Severity
from ..ecs import World
from ..systems import System, Phase
from ..components import (
    FactionPhase, SearchProgress, StarSystem, Planet, TransitState,
)
from .names import generate_star_system_name


PLANET_TYPES = ["rocky", "gas", "ice"]
PLANET_TYPE_WEIGHTS = [0.5, 0.25, 0.25]


def _generate_planet(system_name: str, system_eid: int, index: int) -> tuple[Planet, str]:
    """Generate a single planet with procedural properties."""
    ptype = random.choices(PLANET_TYPES, weights=PLANET_TYPE_WEIGHTS, k=1)[0]

    if ptype == "rocky":
        size = round(random.uniform(0.4, 2.5), 2)
        atmosphere = round(random.uniform(0.0, 0.9), 2)
        water = round(random.uniform(0.0, 0.8), 2)
        minerals = round(random.uniform(0.1, 0.9), 2)
    elif ptype == "gas":
        size = round(random.uniform(4.0, 15.0), 2)
        atmosphere = 1.0
        water = 0.0
        minerals = round(random.uniform(0.0, 0.3), 2)
    else:  # ice
        size = round(random.uniform(0.3, 3.0), 2)
        atmosphere = round(random.uniform(0.0, 0.4), 2)
        water = round(random.uniform(0.3, 0.95), 2)
        minerals = round(random.uniform(0.1, 0.5), 2)

    # Habitability: rocky planets in the right size range with atmosphere+water score highest
    hab = 0.0
    if ptype == "rocky":
        size_score = max(0, 1.0 - abs(size - 1.0) * 0.5)
        hab = (atmosphere * 0.35 + water * 0.35 + minerals * 0.1 + size_score * 0.2)
        hab = round(min(1.0, hab), 2)
    elif ptype == "ice":
        hab = round(water * 0.2 + minerals * 0.1, 2)

    name = f"{system_name}-{chr(97 + index)}"  # e.g., "Kepler-427-a"
    planet = Planet(
        name=name,
        parent_system_id=system_eid,
        planet_type=ptype,
        size=size,
        atmosphere=atmosphere,
        water=water,
        minerals=minerals,
        habitability=hab,
    )
    return planet, name


def _compute_system_habitability(planets: list[Planet]) -> float:
    """Aggregate habitability from the best planet in the system."""
    if not planets:
        return 0.0
    return max(p.habitability for p in planets)


class SearchSystem(System):
    """Discovers star systems and selects a destination."""

    order = 10
    active_phases = {Phase.SEARCH}

    def tick(self, world: World, game_time: GameTime) -> list[GameEvent]:
        faction_entities = world.entities_with(FactionPhase)
        if not faction_entities:
            return []
        faction_id = faction_entities[0]

        # Ensure SearchProgress exists
        progress = world.get_component(faction_id, SearchProgress)
        if progress is None:
            progress = SearchProgress(
                target_discoveries=random.randint(3, 5),
            )
            world.add_component(faction_id, progress)

        progress.days_since_last_discovery += 1

        events: list[GameEvent] = []

        # Check for discovery
        if progress.systems_discovered < progress.target_discoveries:
            # Increasing chance over time
            chance = progress.discovery_chance_per_day * (
                1.0 + progress.days_since_last_discovery * 0.002
            )
            if random.random() < chance:
                events.extend(self._discover_system(world, game_time, faction_id, progress))

        # Check if we should select a destination
        elif progress.systems_discovered >= progress.target_discoveries:
            events.extend(self._select_destination(world, game_time, faction_id))

        return events

    def _discover_system(
        self, world: World, game_time: GameTime, faction_id: int, progress: SearchProgress
    ) -> list[GameEvent]:
        events: list[GameEvent] = []
        progress.days_since_last_discovery = 0
        progress.systems_discovered += 1

        # Generate star system
        name = generate_star_system_name()
        distance = round(random.uniform(5.0, 25.0), 1)
        num_planets = random.randint(2, 8)

        sys_eid = world.create_entity()
        planets: list[Planet] = []
        planet_data: list[dict] = []

        for i in range(num_planets):
            planet, pname = _generate_planet(name, sys_eid, i)
            p_eid = world.create_entity()
            world.add_component(p_eid, planet)
            planets.append(planet)
            planet_data.append({
                "name": pname,
                "type": planet.planet_type,
                "size": planet.size,
                "habitability": planet.habitability,
            })

        hab_score = _compute_system_habitability(planets)

        star_system = StarSystem(
            name=name,
            distance_ly=distance,
            habitability_score=hab_score,
            num_planets=num_planets,
        )
        world.add_component(sys_eid, star_system)

        events.append(GameEvent(
            event_type="search.system_discovered",
            severity=Severity.NOTABLE,
            game_time=game_time,
            data={
                "system_name": name,
                "distance_ly": distance,
                "num_planets": num_planets,
                "habitability_score": hab_score,
                "discovery_number": progress.systems_discovered,
                "target_discoveries": progress.target_discoveries,
                "planets": planet_data,
            },
        ))

        # If this is the last needed, announce evaluation
        if progress.systems_discovered >= progress.target_discoveries:
            events.append(GameEvent(
                event_type="search.evaluation_complete",
                severity=Severity.NOTABLE,
                game_time=game_time,
                data={
                    "systems_evaluated": progress.systems_discovered,
                },
            ))

        return events

    def _select_destination(
        self, world: World, game_time: GameTime, faction_id: int
    ) -> list[GameEvent]:
        events: list[GameEvent] = []

        # Find all non-rejected, non-surveyed star systems and pick the best
        all_systems = world.all_components_of_type(StarSystem)
        candidates = {
            eid: sys for eid, sys in all_systems.items()
            if not sys.rejected and not sys.surveyed
        }
        if not candidates:
            return []

        best_eid = max(candidates, key=lambda eid: candidates[eid].habitability_score)
        best_system = candidates[best_eid]
        best_system.selected = True

        # Set up transit
        total_distance = best_system.distance_ly
        cruise_v = round(random.uniform(0.3, 0.6), 2)  # fraction of c
        # Transit time: distance / velocity (in years), plus accel/decel
        cruise_years = total_distance / cruise_v
        accel_days = random.randint(30, 90)
        decel_days = random.randint(30, 90)
        total_days = int(cruise_years * 365) + accel_days + decel_days

        transit = TransitState(
            destination_id=best_eid,
            distance_ly=total_distance,
            cruise_velocity_c=cruise_v,
            accel_days=accel_days,
            decel_days=decel_days,
            departure_day=game_time.day_offset,
            accel_done_day=game_time.day_offset + accel_days,
            decel_start_day=game_time.day_offset + total_days - decel_days,
            eta_days=total_days,
        )
        world.add_component(faction_id, transit)

        # Transition to TRANSIT phase
        faction_phase = world.get_component(faction_id, FactionPhase)
        if faction_phase:
            faction_phase.phase = Phase.TRANSIT

        transit_years = round(total_days / 365, 1)

        events.append(GameEvent(
            event_type="search.destination_selected",
            severity=Severity.MILESTONE,
            game_time=game_time,
            auto_pause=True,
            data={
                "system_name": best_system.name,
                "distance_ly": best_system.distance_ly,
                "habitability_score": best_system.habitability_score,
                "cruise_velocity_c": cruise_v,
                "transit_years": transit_years,
                "eta_days": total_days,
            },
        ))

        events.append(GameEvent(
            event_type="phase.transition",
            severity=Severity.MILESTONE,
            game_time=game_time,
            data={"from": Phase.SEARCH.value, "to": Phase.TRANSIT.value},
        ))

        return events
