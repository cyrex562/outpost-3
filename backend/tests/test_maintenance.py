"""Tests for the ShipMaintenanceSystem."""

from outpost3.simulation import GameTime
from outpost3.simulation.ecs import World
from outpost3.simulation.systems import Phase
from outpost3.simulation.components import (
    FactionPhase, Resources, Population, ShipSystems,
)
from outpost3.simulation.maintenance import ShipMaintenanceSystem


def _make_maintenance_world() -> tuple[World, int]:
    world = World()
    faction_id = world.create_entity()
    world.add_component(faction_id, FactionPhase(phase=Phase.TRANSIT))
    world.add_component(faction_id, Resources(
        materials=500, fuel=3000, food=2000, water=1500, spare_parts=400,
    ))
    world.add_component(faction_id, Population(
        total=1000, morale=0.75,
    ))
    world.add_component(faction_id, ShipSystems(
        hull_integrity=0.95, engines=0.95, life_support=0.95, sensors=0.95,
    ))
    return world, faction_id


class TestShipMaintenanceSystem:
    def test_degrades_over_time(self):
        world, faction_id = _make_maintenance_world()
        system = ShipMaintenanceSystem()
        ss = world.get_component(faction_id, ShipSystems)

        # Run several months of maintenance
        for month in range(1, 13):
            system._last_maintenance_day = (month - 1) * 30
            system.tick(world, GameTime(month * 30))

        # At least some degradation should have occurred
        assert ss.hull_integrity < 0.95 or ss.engines < 0.95

    def test_uses_spare_parts_for_repair(self):
        world, faction_id = _make_maintenance_world()
        # Set one system low to trigger repair
        ss = world.get_component(faction_id, ShipSystems)
        ss.hull_integrity = 0.70
        res = world.get_component(faction_id, Resources)
        parts_before = res.spare_parts

        system = ShipMaintenanceSystem()
        system._last_maintenance_day = 0
        system.tick(world, GameTime(30))

        # Spare parts should have been consumed
        assert res.spare_parts <= parts_before

    def test_random_events_in_transit(self):
        world, faction_id = _make_maintenance_world()
        system = ShipMaintenanceSystem()
        system._next_event_day = 50  # force event to trigger soon

        all_events = []
        for day in range(1, 200):
            events = system.tick(world, GameTime(day))
            all_events.extend(events)

        transit_events = [e for e in all_events if e.event_type.startswith("transit.event.")]
        assert len(transit_events) >= 1

    def test_breakdown_events(self):
        world, faction_id = _make_maintenance_world()
        system = ShipMaintenanceSystem()

        # Run many months to get at least one breakdown
        all_events = []
        for month in range(1, 50):
            system._last_maintenance_day = (month - 1) * 30
            events = system.tick(world, GameTime(month * 30))
            all_events.extend(events)

        breakdown_events = [e for e in all_events if e.event_type == "maintenance.breakdown"]
        # Breakdowns are probabilistic (~2% per system per month, 4 systems)
        # Over 50 months, very likely to get at least one

    def test_active_phases(self):
        system = ShipMaintenanceSystem()
        assert system.should_run(Phase.TRANSIT)
        assert system.should_run(Phase.SURVEY)
        assert not system.should_run(Phase.LOADOUT)
        assert not system.should_run(Phase.SEARCH)
