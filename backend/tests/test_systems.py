"""Tests for System base class and Phase."""

from outpost3.simulation import GameEvent, GameTime, Severity
from outpost3.simulation.ecs import World
from outpost3.simulation.systems import System, Phase


class DummySystem(System):
    order = 10
    active_phases = {Phase.TRANSIT}

    def tick(self, world: World, game_time: GameTime) -> list[GameEvent]:
        return [GameEvent(
            event_type="dummy.tick",
            severity=Severity.DEBUG,
            game_time=game_time,
        )]


class AlwaysSystem(System):
    order = 5
    active_phases = None

    def tick(self, world: World, game_time: GameTime) -> list[GameEvent]:
        return []


class TestPhase:
    def test_phase_values(self):
        assert Phase.LOADOUT.value == "loadout"
        assert Phase.SEARCH.value == "search"
        assert Phase.TRANSIT.value == "transit"
        assert Phase.SURVEY.value == "survey"
        assert Phase.FOUNDING.value == "founding"


class TestSystem:
    def test_should_run_matching_phase(self):
        sys = DummySystem()
        assert sys.should_run(Phase.TRANSIT) is True

    def test_should_not_run_wrong_phase(self):
        sys = DummySystem()
        assert sys.should_run(Phase.LOADOUT) is False

    def test_should_run_always(self):
        sys = AlwaysSystem()
        assert sys.should_run(Phase.TRANSIT) is True
        assert sys.should_run(Phase.LOADOUT) is True
        assert sys.should_run(None) is True

    def test_should_run_none_phase(self):
        sys = DummySystem()
        assert sys.should_run(None) is True

    def test_tick_returns_events(self):
        sys = DummySystem()
        world = World()
        gt = GameTime(10)
        events = sys.tick(world, gt)
        assert len(events) == 1
        assert events[0].event_type == "dummy.tick"
