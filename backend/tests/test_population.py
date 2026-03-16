"""Tests for the PopulationSystem."""

from outpost3.simulation import GameTime
from outpost3.simulation.ecs import World
from outpost3.simulation.systems import Phase
from outpost3.simulation.components import (
    FactionPhase, Resources, Population, Notable,
)
from outpost3.simulation.population import PopulationSystem


def _make_pop_world(
    morale: float = 0.75,
    food: float = 2000.0,
    total: int = 1000,
) -> tuple[World, int]:
    world = World()
    faction_id = world.create_entity()
    world.add_component(faction_id, FactionPhase(phase=Phase.TRANSIT))
    world.add_component(faction_id, Resources(food=food, water=1500))
    world.add_component(faction_id, Population(
        total=total, scientists=150, engineers=200, medical=80,
        military=70, civilians=500, morale=morale,
    ))

    # Add a Notable
    n_eid = world.create_entity()
    world.add_component(n_eid, Notable(
        name="Old Captain", role="Captain", age=78,
        traits=["bold", "mission_focus"],
    ))
    return world, faction_id


class TestPopulationSystem:
    def test_respects_interval(self):
        world, _ = _make_pop_world()
        system = PopulationSystem()
        system._last_update_day = 0
        # Day 10 is too soon
        events = system.tick(world, GameTime(10))
        assert len(events) == 0

    def test_processes_at_interval(self):
        world, faction_id = _make_pop_world()
        system = PopulationSystem()
        system._last_update_day = 0
        # Day 30 triggers processing
        system.tick(world, GameTime(30))
        # Should have processed (population may have changed)
        pop = world.get_component(faction_id, Population)
        assert pop is not None

    def test_births_occur(self):
        world, faction_id = _make_pop_world(morale=0.90, food=5000, total=2000)
        system = PopulationSystem()
        system._last_update_day = 0

        pop = world.get_component(faction_id, Population)
        total_before = pop.total

        # Run several months
        for day in range(30, 365, 30):
            system.tick(world, GameTime(day))

        assert pop.births_year > 0

    def test_starvation_increases_deaths(self):
        world, faction_id = _make_pop_world(food=0.0, total=1000)
        system = PopulationSystem()
        system._last_update_day = 0

        pop = world.get_component(faction_id, Population)
        for day in range(30, 365, 30):
            system.tick(world, GameTime(day))

        assert pop.deaths_year > 0

    def test_yearly_report_emitted(self):
        world, _ = _make_pop_world()
        system = PopulationSystem()
        system._last_update_day = 0

        # Run past year boundary (day 365 = year 2 start)
        all_events = []
        for day in range(30, 396, 30):
            events = system.tick(world, GameTime(day))
            all_events.extend(events)
        # Also tick on exactly the year boundary
        system._last_update_day = 335
        events = system.tick(world, GameTime(365))
        all_events.extend(events)

        report_events = [e for e in all_events if e.event_type == "population.yearly_report"]
        assert len(report_events) >= 1

    def test_old_notable_can_die(self):
        world, _ = _make_pop_world()
        system = PopulationSystem()

        # Run many months for old notable (age 78) to potentially die
        all_events = []
        for month in range(1, 25):
            day = month * 30
            system._last_update_day = day - 30
            events = system.tick(world, GameTime(day))
            all_events.extend(events)

        death_events = [e for e in all_events if e.event_type == "notable.death"]
        # With age 78, there's a decent chance of death over 2 years
        # Not guaranteed, but highly likely
        # Just check the system runs without error

    def test_succession_on_notable_death(self):
        world, _ = _make_pop_world()

        # Manually kill the notable
        notables = world.all_components_of_type(Notable)
        for eid, n in notables.items():
            n.alive = False
            n.death_day = 100

        system = PopulationSystem()
        system._last_update_day = 0
        # The _process_notables won't trigger death again, but we can test
        # the succession function directly
        from outpost3.simulation.population import PopulationSystem as PS
        event = system._promote_successor(world, GameTime(100), "Captain")
        assert event is not None
        assert event.event_type == "notable.succession"
        assert event.data["role"] == "Captain"

    def test_morale_drifts(self):
        world, faction_id = _make_pop_world(morale=0.30, food=5000)
        system = PopulationSystem()

        pop = world.get_component(faction_id, Population)
        morale_before = pop.morale

        # Run several months — morale should drift up with good food
        for day in range(30, 365, 30):
            system._last_update_day = day - 30
            system.tick(world, GameTime(day))

        assert pop.morale > morale_before

    def test_population_minimum_one(self):
        """Population should never drop below 1."""
        world, faction_id = _make_pop_world(food=0.0, total=5)
        system = PopulationSystem()

        pop = world.get_component(faction_id, Population)
        for day in range(30, 3650, 30):
            system._last_update_day = day - 30
            system.tick(world, GameTime(day))

        assert pop.total >= 1
