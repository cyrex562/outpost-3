"""Tests for the TransitSystem."""

from outpost3.simulation import GameTime
from outpost3.simulation.ecs import World
from outpost3.simulation.systems import Phase
from outpost3.simulation.components import (
    FactionPhase, Resources, Population, ShipSystems,
    TransitState, StarSystem,
)
from outpost3.simulation.transit import TransitSystem


def _make_transit_world(
    distance_ly: float = 10.0,
    eta_days: int = 7300,  # ~20 years
    departure_day: int = 0,
) -> tuple[World, int, int]:
    """Create a world in TRANSIT phase."""
    world = World()
    faction_id = world.create_entity()
    world.add_component(faction_id, FactionPhase(phase=Phase.TRANSIT))
    world.add_component(faction_id, Resources(
        materials=500, fuel=3000, food=2000, water=1500, spare_parts=400,
    ))
    world.add_component(faction_id, Population(
        total=1000, scientists=150, engineers=200, medical=80,
        military=70, civilians=500, morale=0.75,
    ))
    world.add_component(faction_id, ShipSystems())

    dest_eid = world.create_entity()
    world.add_component(dest_eid, StarSystem(
        name="Kepler-427", distance_ly=distance_ly, habitability_score=0.7,
    ))

    transit = TransitState(
        destination_id=dest_eid,
        distance_ly=distance_ly,
        cruise_velocity_c=0.5,
        accel_days=60,
        decel_days=60,
        departure_day=departure_day,
        accel_done_day=departure_day + 60,
        decel_start_day=departure_day + eta_days - 60,
        eta_days=eta_days,
    )
    world.add_component(faction_id, transit)

    return world, faction_id, dest_eid


class TestTransitSystem:
    def test_consumes_fuel(self):
        world, faction_id, _ = _make_transit_world(departure_day=0)
        system = TransitSystem()
        res_before = world.get_component(faction_id, Resources)
        fuel_before = res_before.fuel

        system.tick(world, GameTime(10))  # day 10, accelerating
        assert res_before.fuel < fuel_before

    def test_consumes_food_and_water(self):
        world, faction_id, _ = _make_transit_world(departure_day=0)
        system = TransitSystem()
        res = world.get_component(faction_id, Resources)
        food_before = res.food
        water_before = res.water

        system.tick(world, GameTime(10))
        assert res.food < food_before
        assert res.water < water_before

    def test_fuel_higher_during_accel(self):
        world, faction_id, _ = _make_transit_world(departure_day=0, eta_days=7300)
        system = TransitSystem()
        res = world.get_component(faction_id, Resources)

        # Acceleration phase (day 10)
        fuel_start = res.fuel
        system.tick(world, GameTime(10))
        accel_cost = fuel_start - res.fuel

        # Reset and test cruise phase (day 500)
        world2, fid2, _ = _make_transit_world(departure_day=0, eta_days=7300)
        system2 = TransitSystem()
        res2 = world2.get_component(fid2, Resources)
        fuel_start2 = res2.fuel
        system2.tick(world2, GameTime(500))
        cruise_cost = fuel_start2 - res2.fuel

        assert accel_cost > cruise_cost

    def test_tracks_distance(self):
        world, faction_id, _ = _make_transit_world(departure_day=0, eta_days=1000)
        system = TransitSystem()
        transit = world.get_component(faction_id, TransitState)

        system.tick(world, GameTime(500))
        assert transit.distance_traveled_ly > 0
        assert transit.distance_traveled_ly <= transit.distance_ly

    def test_transit_phase_changes(self):
        world, faction_id, _ = _make_transit_world(departure_day=0, eta_days=1000)
        system = TransitSystem()
        transit = world.get_component(faction_id, TransitState)

        system.tick(world, GameTime(10))
        assert transit.transit_phase == "accelerating"

        system.tick(world, GameTime(500))
        assert transit.transit_phase == "cruising"

        system.tick(world, GameTime(960))
        assert transit.transit_phase == "decelerating"

    def test_arrival_transitions_to_survey(self):
        world, faction_id, _ = _make_transit_world(departure_day=0, eta_days=100)
        system = TransitSystem()

        events = system.tick(world, GameTime(100))

        fp = world.get_component(faction_id, FactionPhase)
        assert fp.phase == Phase.SURVEY

        types = [e.event_type for e in events]
        assert "transit.arrival" in types
        assert "phase.transition" in types

    def test_arrival_auto_pauses(self):
        world, faction_id, _ = _make_transit_world(departure_day=0, eta_days=100)
        system = TransitSystem()

        events = system.tick(world, GameTime(100))
        arrival = [e for e in events if e.event_type == "transit.arrival"]
        assert len(arrival) == 1
        assert arrival[0].auto_pause is True

    def test_yearly_summary(self):
        world, faction_id, _ = _make_transit_world(departure_day=0, eta_days=7300)
        system = TransitSystem()

        # Year 2 start (day 365)
        events = system.tick(world, GameTime(365))
        summaries = [e for e in events if e.event_type == "transit.yearly_summary"]
        assert len(summaries) == 1
        data = summaries[0].data
        assert "pct_complete" in data
        assert "fuel_remaining" in data
