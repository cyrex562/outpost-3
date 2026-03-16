"""Tests for the BehaviorSystem and deliberation records."""

from outpost3.simulation import GameTime, Severity
from outpost3.simulation.ecs import World
from outpost3.simulation.systems import Phase
from outpost3.simulation.components import (
    FactionPhase, Resources, Population, ShipSystems, Notable,
)
from outpost3.simulation.behavior import (
    BehaviorSystem, Deliberation, Option,
    _check_food_low, _check_hull_damage, _check_morale_low,
    _score_options, _find_advocate,
)


def _make_world_with_crisis(
    food: float = 500.0,
    hull: float = 0.95,
    morale: float = 0.75,
) -> tuple[World, int]:
    """Create a world with faction in TRANSIT and configurable resource levels."""
    world = World()
    faction_id = world.create_entity()
    world.add_component(faction_id, FactionPhase(phase=Phase.TRANSIT))
    world.add_component(faction_id, Resources(
        materials=500, fuel=2000, food=food, water=1000, spare_parts=300,
    ))
    world.add_component(faction_id, Population(
        total=1000, scientists=150, engineers=200, medical=80,
        military=70, civilians=500, morale=morale,
    ))
    world.add_component(faction_id, ShipSystems(
        hull_integrity=hull, engines=0.9, life_support=0.95, sensors=0.9,
    ))

    # Add a couple notables
    n1 = world.create_entity()
    world.add_component(n1, Notable(
        name="Dr. Chen", role="Chief Medical Officer", age=45,
        traits=["crew_welfare", "cautious"],
    ))
    n2 = world.create_entity()
    world.add_component(n2, Notable(
        name="Eng. Vasquez", role="Chief Engineer", age=38,
        traits=["resourceful", "bold"],
    ))

    return world, faction_id


class TestConditionEvaluators:
    def test_food_low_triggers(self):
        resources = Resources(food=100)
        population = Population(total=1000)
        result = _check_food_low(resources, population)
        assert result is not None
        assert result.trigger == "food_low"
        assert len(result.options) == 3

    def test_food_ok_no_trigger(self):
        resources = Resources(food=5000)
        population = Population(total=1000)
        result = _check_food_low(resources, population)
        assert result is None

    def test_hull_damage_triggers(self):
        ss = ShipSystems(hull_integrity=0.55)
        result = _check_hull_damage(ss)
        assert result is not None
        assert result.trigger == "hull_damage"

    def test_hull_ok_no_trigger(self):
        ss = ShipSystems(hull_integrity=0.95)
        result = _check_hull_damage(ss)
        assert result is None

    def test_morale_low_triggers(self):
        pop = Population(morale=0.30)
        result = _check_morale_low(pop)
        assert result is not None
        assert result.trigger == "morale_low"

    def test_morale_ok_no_trigger(self):
        pop = Population(morale=0.80)
        result = _check_morale_low(pop)
        assert result is None


class TestDeliberation:
    def test_to_dict(self):
        d = Deliberation(
            trigger="test",
            trigger_detail="test detail",
            options=[Option(action="do_thing", description="Do the thing", score=0.7)],
        )
        result = d.to_dict()
        assert result["trigger"] == "test"
        assert len(result["options"]) == 1
        assert result["options"][0]["score"] == 0.7

    def test_option_to_dict(self):
        o = Option(
            action="fix",
            description="Fix it",
            score=0.85,
            pros=["good"],
            cons=["costly"],
            advocate="Dr. Chen",
            advocate_trait="crew_welfare",
        )
        d = o.to_dict()
        assert d["action"] == "fix"
        assert d["advocate"] == "Dr. Chen"


class TestScoring:
    def test_score_options_assigns_scores(self):
        delib = Deliberation(
            trigger="food_low",
            trigger_detail="Food low",
            options=[
                Option(action="reduce_rations", description="Reduce rations",
                       pros=["Extends supply"], cons=["Morale hit"]),
                Option(action="expand_hydroponics", description="Expand hydroponics",
                       pros=["Long-term fix", "More food"], cons=["Costs parts"]),
            ],
        )
        resources = Resources(food=100, spare_parts=300)
        population = Population(total=1000)
        ship_systems = ShipSystems()
        notables = [
            Notable(name="Dr. Chen", role="CMO", age=45, traits=["crew_welfare"]),
        ]
        _score_options(delib, resources, population, ship_systems, notables)
        assert delib.chosen is not None
        assert delib.chosen.score > 0
        assert len(delib.rejected) == 1

    def test_find_advocate(self):
        notables = [
            Notable(name="Dr. Chen", role="CMO", age=45, traits=["crew_welfare"]),
            Notable(name="Eng. Vasquez", role="CE", age=38, traits=["resourceful"]),
        ]
        name, trait = _find_advocate(notables, ["improvise", "repurpose"])
        assert name == "Eng. Vasquez"
        assert trait == "resourceful"

    def test_find_advocate_dead_notable_excluded(self):
        notables = [
            Notable(name="Dead Person", role="CMO", age=45,
                    traits=["crew_welfare"], alive=False),
        ]
        name, trait = _find_advocate(notables, ["morale"])
        assert name is None


class TestBehaviorSystem:
    def test_no_trigger_when_all_ok(self):
        world, faction_id = _make_world_with_crisis(
            food=5000, hull=0.95, morale=0.75,
        )
        system = BehaviorSystem()
        system._last_check_day = -100  # force check
        events = system.tick(world, GameTime(100))
        assert len(events) == 0

    def test_triggers_on_food_crisis(self):
        world, faction_id = _make_world_with_crisis(food=50)
        system = BehaviorSystem()
        system._last_check_day = -100
        events = system.tick(world, GameTime(100))
        delib_events = [e for e in events if e.event_type == "behavior.deliberation"]
        assert len(delib_events) >= 1
        delib_data = delib_events[0].data["deliberation"]
        assert delib_data["trigger"] == "food_low"
        assert delib_data["chosen"] is not None

    def test_triggers_on_hull_damage(self):
        world, faction_id = _make_world_with_crisis(hull=0.45)
        system = BehaviorSystem()
        system._last_check_day = -100
        events = system.tick(world, GameTime(100))
        delib_events = [e for e in events if e.event_type == "behavior.deliberation"]
        triggers = [e.data["deliberation"]["trigger"] for e in delib_events]
        assert "hull_damage" in triggers

    def test_triggers_on_low_morale(self):
        world, faction_id = _make_world_with_crisis(morale=0.25)
        system = BehaviorSystem()
        system._last_check_day = -100
        events = system.tick(world, GameTime(100))
        delib_events = [e for e in events if e.event_type == "behavior.deliberation"]
        triggers = [e.data["deliberation"]["trigger"] for e in delib_events]
        assert "morale_low" in triggers

    def test_cooldown_prevents_rapid_retrigger(self):
        world, faction_id = _make_world_with_crisis(food=50)
        system = BehaviorSystem()
        system._last_check_day = -100
        events1 = system.tick(world, GameTime(100))
        assert len(events1) >= 1

        # Check again immediately (force check interval)
        system._last_check_day = 0
        events2 = system.tick(world, GameTime(130))
        food_events = [e for e in events2
                       if e.event_type == "behavior.deliberation"
                       and e.data["deliberation"]["trigger"] == "food_low"]
        assert len(food_events) == 0  # cooldown prevents retrigger

    def test_respects_check_interval(self):
        world, faction_id = _make_world_with_crisis(food=50)
        system = BehaviorSystem()
        system._last_check_day = 95
        events = system.tick(world, GameTime(100))
        assert len(events) == 0  # only 5 days since last check

    def test_only_runs_in_transit_or_survey(self):
        system = BehaviorSystem()
        assert system.should_run(Phase.TRANSIT) is True
        assert system.should_run(Phase.SURVEY) is True
        assert system.should_run(Phase.LOADOUT) is False
        assert system.should_run(Phase.SEARCH) is False
