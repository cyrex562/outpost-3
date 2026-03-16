"""Tests for the LoadoutSystem."""

from outpost3.simulation import GameTime
from outpost3.simulation.ecs import World
from outpost3.simulation.systems import Phase
from outpost3.simulation.components import (
    FactionPhase, Resources, Population, ShipSystems, Notable, Loadout,
)
from outpost3.simulation.generation.loadout import LoadoutSystem


def _make_world() -> tuple[World, int]:
    """Create a world with a faction entity in LOADOUT phase."""
    world = World()
    faction_id = world.create_entity()
    world.add_component(faction_id, FactionPhase(phase=Phase.LOADOUT))
    return world, faction_id


class TestLoadoutSystem:
    def test_generates_on_first_tick(self):
        world, faction_id = _make_world()
        system = LoadoutSystem()
        events = system.tick(world, GameTime(0))
        assert len(events) > 0

    def test_only_generates_once(self):
        world, faction_id = _make_world()
        system = LoadoutSystem()
        events1 = system.tick(world, GameTime(0))
        events2 = system.tick(world, GameTime(1))
        assert len(events1) > 0
        assert len(events2) == 0

    def test_creates_resources(self):
        world, faction_id = _make_world()
        system = LoadoutSystem()
        system.tick(world, GameTime(0))
        res = world.get_component(faction_id, Resources)
        assert res is not None
        assert res.food > 0
        assert res.fuel > 0
        assert res.water > 0
        assert res.materials > 0
        assert res.spare_parts > 0

    def test_creates_population(self):
        world, faction_id = _make_world()
        system = LoadoutSystem()
        system.tick(world, GameTime(0))
        pop = world.get_component(faction_id, Population)
        assert pop is not None
        assert 500 <= pop.total <= 2000
        assert pop.scientists > 0
        assert pop.engineers > 0
        assert pop.civilians > 0

    def test_creates_ship_systems(self):
        world, faction_id = _make_world()
        system = LoadoutSystem()
        system.tick(world, GameTime(0))
        ss = world.get_component(faction_id, ShipSystems)
        assert ss is not None
        assert 0.85 <= ss.hull_integrity <= 1.0
        assert 0.85 <= ss.engines <= 1.0

    def test_creates_notables(self):
        world, faction_id = _make_world()
        system = LoadoutSystem()
        system.tick(world, GameTime(0))
        notables = world.all_components_of_type(Notable)
        assert 5 <= len(notables) <= 8
        for eid, notable in notables.items():
            assert notable.name
            assert notable.role
            assert 25 <= notable.age <= 55
            assert 2 <= len(notable.traits) <= 3
            assert notable.alive

    def test_notables_have_unique_names(self):
        world, faction_id = _make_world()
        system = LoadoutSystem()
        system.tick(world, GameTime(0))
        notables = world.all_components_of_type(Notable)
        names = [n.name for n in notables.values()]
        assert len(names) == len(set(names))

    def test_creates_loadout_record(self):
        world, faction_id = _make_world()
        system = LoadoutSystem()
        system.tick(world, GameTime(0))
        lo = world.get_component(faction_id, Loadout)
        assert lo is not None
        assert 500 <= lo.crew_size <= 2000
        assert lo.ship_name
        assert len(lo.notable_names) >= 5

    def test_transitions_to_search_phase(self):
        world, faction_id = _make_world()
        system = LoadoutSystem()
        system.tick(world, GameTime(0))
        fp = world.get_component(faction_id, FactionPhase)
        assert fp is not None
        assert fp.phase == Phase.SEARCH

    def test_emits_expected_event_types(self):
        world, faction_id = _make_world()
        system = LoadoutSystem()
        events = system.tick(world, GameTime(0))
        types = [e.event_type for e in events]
        assert "loadout.ship_commissioned" in types
        assert "loadout.summary" in types
        assert "loadout.crew_manifest" in types
        assert "loadout.notable_introduction" in types
        assert "phase.transition" in types

    def test_ship_commissioned_auto_pauses(self):
        world, faction_id = _make_world()
        system = LoadoutSystem()
        events = system.tick(world, GameTime(0))
        commissioned = [e for e in events if e.event_type == "loadout.ship_commissioned"]
        assert len(commissioned) == 1
        assert commissioned[0].auto_pause is True

    def test_notable_traits_no_contradictions(self):
        """Notables should not have both traits from the same axis."""
        from outpost3.simulation.components import TRAIT_AXES
        world, faction_id = _make_world()
        system = LoadoutSystem()
        system.tick(world, GameTime(0))
        notables = world.all_components_of_type(Notable)
        for eid, notable in notables.items():
            for a, b in TRAIT_AXES:
                assert not (a in notable.traits and b in notable.traits), \
                    f"{notable.name} has contradicting traits {a} and {b}"
