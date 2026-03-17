"""Tests for the FoundingSystem."""

from outpost3.simulation import GameTime
from outpost3.simulation.ecs import World
from outpost3.simulation.systems import Phase
from outpost3.simulation.components import (
    FactionPhase, Resources, Population, ShipSystems, Notable,
    StarSystem, SurveyProgress, Loadout,
)
from outpost3.simulation.founding import (
    FoundingSystem, _generate_colony_name, _assess_founding_conditions,
)


def _make_founding_world() -> tuple[World, int]:
    """Create a world in FOUNDING phase with typical components."""
    world = World()
    faction_id = world.create_entity()
    world.add_component(faction_id, FactionPhase(phase=Phase.FOUNDING))
    world.add_component(faction_id, Resources(
        fuel=500, food=800, water=600, spare_parts=150, materials=300,
    ))
    world.add_component(faction_id, Population(
        total=750, morale=0.65,
    ))
    world.add_component(faction_id, ShipSystems(
        hull_integrity=0.6, engines=0.5, life_support=0.7, sensors=0.8,
    ))
    world.add_component(faction_id, Loadout(crew_size=900, ship_name="Artemis"))

    # Star system and survey progress
    sys_eid = world.create_entity()
    world.add_component(sys_eid, StarSystem(
        name="Kepler-427", distance_ly=12.0,
        habitability_score=0.6, num_planets=3, selected=True,
    ))
    world.add_component(faction_id, SurveyProgress(
        system_id=sys_eid,
        best_planet_name="Kepler-427-b",
        best_planet_hab=0.65,
        decided=True,
        accepted=True,
    ))

    # Some notables
    for i, (name, role) in enumerate([
        ("Elena Vasquez", "Captain"),
        ("James Chen", "Chief Engineer"),
        ("Aisha Okoye", "Chief Scientist"),
    ]):
        n_eid = world.create_entity()
        world.add_component(n_eid, Notable(
            name=name, role=role, age=45 + i * 5,
            traits=["bold", "pragmatic"], alive=True,
        ))

    return world, faction_id


class TestFoundingSequence:
    def test_starts_with_landing_site_selected(self):
        world, faction_id = _make_founding_world()
        system = FoundingSystem()

        events = system.tick(world, GameTime(500))
        assert len(events) == 1
        assert events[0].event_type == "founding.landing_site_selected"
        assert events[0].auto_pause is True
        assert events[0].data["planet_name"] == "Kepler-427-b"

    def test_descent_complete_after_7_days(self):
        world, faction_id = _make_founding_world()
        system = FoundingSystem()

        system.tick(world, GameTime(500))  # init → descent

        # Too early
        events = system.tick(world, GameTime(505))
        assert events == []

        # Day 507 = founding_day + 7
        events = system.tick(world, GameTime(507))
        assert len(events) == 1
        assert events[0].event_type == "founding.descent_complete"

    def test_first_structures_after_37_days(self):
        world, faction_id = _make_founding_world()
        system = FoundingSystem()

        system.tick(world, GameTime(500))
        system.tick(world, GameTime(507))

        # Too early
        events = system.tick(world, GameTime(530))
        assert events == []

        # Day 537 = founding_day + 37
        events = system.tick(world, GameTime(537))
        assert len(events) == 1
        assert events[0].event_type == "founding.first_structures"

    def test_colony_established_after_44_days(self):
        world, faction_id = _make_founding_world()
        system = FoundingSystem()

        system.tick(world, GameTime(500))
        system.tick(world, GameTime(507))
        system.tick(world, GameTime(537))

        # Day 544 = founding_day + 44
        events = system.tick(world, GameTime(544))
        assert len(events) == 1
        assert events[0].event_type == "founding.colony_established"
        assert events[0].auto_pause is True
        assert "colony_name" in events[0].data
        assert events[0].data["population"] == 750

    def test_no_events_after_complete(self):
        world, faction_id = _make_founding_world()
        system = FoundingSystem()

        system.tick(world, GameTime(500))
        system.tick(world, GameTime(507))
        system.tick(world, GameTime(537))
        system.tick(world, GameTime(544))

        events = system.tick(world, GameTime(600))
        assert events == []

    def test_full_sequence_event_types(self):
        world, faction_id = _make_founding_world()
        system = FoundingSystem()

        all_events = []
        for day in range(500, 600):
            events = system.tick(world, GameTime(day))
            all_events.extend(events)

        types = [e.event_type for e in all_events]
        assert types == [
            "founding.landing_site_selected",
            "founding.descent_complete",
            "founding.first_structures",
            "founding.colony_established",
        ]

    def test_active_phases(self):
        system = FoundingSystem()
        assert system.should_run(Phase.FOUNDING)
        assert not system.should_run(Phase.TRANSIT)
        assert not system.should_run(Phase.SURVEY)


class TestColonyNameGeneration:
    def test_generates_nonempty_name(self):
        notables = [Notable(name="Elena Vasquez", role="Captain", alive=True)]
        for _ in range(20):
            name = _generate_colony_name(notables, "Kepler-427-b")
            assert len(name) > 0

    def test_generates_name_without_notables(self):
        for _ in range(20):
            name = _generate_colony_name([], "Kepler-427-b")
            assert len(name) > 0

    def test_planet_derived_name(self):
        # With empty notables and high roll, should get planet-derived
        # Just verify it doesn't crash and produces something
        name = _generate_colony_name([], "Alpha-Centauri-b")
        assert isinstance(name, str)


class TestFoundingConditionAssessment:
    def test_strong_condition(self):
        resources = Resources(fuel=2000, food=1500, spare_parts=200, materials=500)
        population = Population(total=900, morale=0.8)
        ship_systems = ShipSystems(hull_integrity=0.9, engines=0.9, life_support=0.9, sensors=0.9)
        loadout = Loadout(crew_size=900)

        result = _assess_founding_conditions(resources, population, ship_systems, loadout)
        assert result["condition"] == "strong"

    def test_desperate_condition(self):
        resources = Resources(fuel=50, food=100, spare_parts=10, materials=50)
        population = Population(total=400, morale=0.2)
        ship_systems = ShipSystems(hull_integrity=0.2, engines=0.2, life_support=0.3, sensors=0.3)
        loadout = Loadout(crew_size=900)

        result = _assess_founding_conditions(resources, population, ship_systems, loadout)
        assert result["condition"] == "desperate"
        assert len(result["details"]) > 0

    def test_moderate_condition(self):
        resources = Resources(fuel=500, food=400, spare_parts=100, materials=200)
        population = Population(total=800, morale=0.6)
        ship_systems = ShipSystems()
        loadout = Loadout(crew_size=900)

        result = _assess_founding_conditions(resources, population, ship_systems, loadout)
        assert result["condition"] in ("moderate", "strong")

    def test_handles_none_components(self):
        result = _assess_founding_conditions(None, None, None, None)
        assert result["condition"] == "strong"
        assert "Crew and supplies in good condition" in result["details"]

    def test_low_food_flagged(self):
        resources = Resources(food=150)
        result = _assess_founding_conditions(resources, None, None, None)
        assert any("Food" in d for d in result["details"])

    def test_population_loss_flagged(self):
        population = Population(total=400, morale=0.6)
        loadout = Loadout(crew_size=900)
        result = _assess_founding_conditions(None, population, None, loadout)
        assert any("Population" in d or "population" in d for d in result["details"])
