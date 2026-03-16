"""Tests for the SearchSystem."""

import random

from outpost3.simulation import GameTime
from outpost3.simulation.ecs import World
from outpost3.simulation.systems import Phase
from outpost3.simulation.components import (
    FactionPhase, Resources, Population, ShipSystems,
    SearchProgress, StarSystem, Planet, TransitState,
)
from outpost3.simulation.generation.search import (
    SearchSystem, _generate_planet, _compute_system_habitability,
)


def _make_search_world() -> tuple[World, int]:
    """Create a world in SEARCH phase with basic faction setup."""
    world = World()
    faction_id = world.create_entity()
    world.add_component(faction_id, FactionPhase(phase=Phase.SEARCH))
    world.add_component(faction_id, Resources(fuel=3000, food=2000, water=1500))
    world.add_component(faction_id, Population(total=1000))
    world.add_component(faction_id, ShipSystems())
    return world, faction_id


class TestPlanetGeneration:
    def test_generate_planet_returns_valid(self):
        planet, name = _generate_planet("TestStar", 0, 0)
        assert planet.planet_type in ("rocky", "gas", "ice")
        assert planet.size > 0
        assert 0.0 <= planet.habitability <= 1.0
        assert name == "TestStar-a"

    def test_rocky_planets_have_habitability(self):
        # Force rocky planet generation
        random.seed(42)
        found_habitable = False
        for _ in range(50):
            planet, _ = _generate_planet("Test", 0, 0)
            if planet.planet_type == "rocky" and planet.habitability > 0.3:
                found_habitable = True
                break
        assert found_habitable

    def test_gas_planets_low_habitability(self):
        for _ in range(50):
            planet, _ = _generate_planet("Test", 0, 0)
            if planet.planet_type == "gas":
                assert planet.habitability == 0.0
                break


class TestSystemHabitability:
    def test_empty_system(self):
        assert _compute_system_habitability([]) == 0.0

    def test_uses_best_planet(self):
        planets = [
            Planet(habitability=0.3),
            Planet(habitability=0.8),
            Planet(habitability=0.1),
        ]
        assert _compute_system_habitability(planets) == 0.8


class TestSearchSystem:
    def test_creates_search_progress(self):
        world, faction_id = _make_search_world()
        system = SearchSystem()
        system.tick(world, GameTime(0))
        progress = world.get_component(faction_id, SearchProgress)
        assert progress is not None
        assert 3 <= progress.target_discoveries <= 5

    def test_discovers_systems_over_time(self):
        world, faction_id = _make_search_world()
        system = SearchSystem()

        # Run many ticks to accumulate discoveries
        discovered = 0
        for day in range(1000):
            events = system.tick(world, GameTime(day))
            for e in events:
                if e.event_type == "search.system_discovered":
                    discovered += 1

        assert discovered > 0

    def test_selects_destination_after_enough_discoveries(self):
        world, faction_id = _make_search_world()
        # Pre-populate with enough star systems
        progress = SearchProgress(systems_discovered=4, target_discoveries=4)
        world.add_component(faction_id, progress)

        for i in range(4):
            sys_eid = world.create_entity()
            world.add_component(sys_eid, StarSystem(
                name=f"System-{i}",
                distance_ly=10 + i,
                habitability_score=0.3 + i * 0.15,
            ))

        system = SearchSystem()
        events = system.tick(world, GameTime(100))

        # Should have selected destination and transitioned
        types = [e.event_type for e in events]
        assert "search.destination_selected" in types
        assert "phase.transition" in types

        # Phase should be TRANSIT
        fp = world.get_component(faction_id, FactionPhase)
        assert fp.phase == Phase.TRANSIT

        # TransitState should exist
        transit = world.get_component(faction_id, TransitState)
        assert transit is not None
        assert transit.distance_ly > 0
        assert transit.eta_days > 0

    def test_selects_best_habitability(self):
        world, faction_id = _make_search_world()
        progress = SearchProgress(systems_discovered=3, target_discoveries=3)
        world.add_component(faction_id, progress)

        # Create systems with varying habitability
        for i, hab in enumerate([0.3, 0.9, 0.5]):
            sys_eid = world.create_entity()
            world.add_component(sys_eid, StarSystem(
                name=f"System-{i}", distance_ly=10, habitability_score=hab,
            ))

        system = SearchSystem()
        events = system.tick(world, GameTime(100))

        dest_events = [e for e in events if e.event_type == "search.destination_selected"]
        assert len(dest_events) == 1
        assert dest_events[0].data["habitability_score"] == 0.9

    def test_destination_selected_auto_pauses(self):
        world, faction_id = _make_search_world()
        progress = SearchProgress(systems_discovered=3, target_discoveries=3)
        world.add_component(faction_id, progress)

        sys_eid = world.create_entity()
        world.add_component(sys_eid, StarSystem(
            name="Test", distance_ly=10, habitability_score=0.7,
        ))

        system = SearchSystem()
        events = system.tick(world, GameTime(100))
        dest = [e for e in events if e.event_type == "search.destination_selected"]
        assert dest[0].auto_pause is True
