"""Tests for the TimeEngine with ECS integration and 15-level speed system."""

import asyncio

from outpost3.simulation import GameEvent, GameTime, Severity
from outpost3.simulation.ecs import World
from outpost3.simulation.engine import (
    TimeEngine, SPEED_CONFIG, SPEED_LEVELS, VALID_SPEEDS,
    BATCH_THRESHOLD_LEVEL,
)
from outpost3.simulation.systems import System, Phase


class CountingSystem(System):
    """Test system that counts ticks."""
    order = 1
    active_phases = None

    def __init__(self):
        self.tick_count = 0

    def tick(self, world: World, game_time: GameTime) -> list[GameEvent]:
        self.tick_count += 1
        return []


class EventSystem(System):
    """Test system that emits events on year boundaries."""
    order = 2
    active_phases = None

    def tick(self, world: World, game_time: GameTime) -> list[GameEvent]:
        if game_time.is_year_start and game_time.year > 1:
            return [GameEvent(
                event_type="test.year",
                severity=Severity.NOTABLE,
                game_time=game_time,
                data={"year": game_time.year},
            )]
        return []


class TransitOnlySystem(System):
    order = 10
    active_phases = {Phase.TRANSIT}

    def __init__(self):
        self.tick_count = 0

    def tick(self, world: World, game_time: GameTime) -> list[GameEvent]:
        self.tick_count += 1
        return []


def _run(coro):
    """Helper to run async tests."""
    return asyncio.get_event_loop().run_until_complete(coro)


class TestSpeedLevels:
    def test_15_speed_levels(self):
        assert len(SPEED_LEVELS) == 15

    def test_valid_speeds_are_1_to_15(self):
        assert VALID_SPEEDS == list(range(1, 16))

    def test_speed_config_keys(self):
        for level in range(1, 16):
            assert level in SPEED_CONFIG

    def test_speed_levels_increasing(self):
        days = [s["days"] for s in SPEED_LEVELS]
        assert days == sorted(days)

    def test_batch_threshold(self):
        assert BATCH_THRESHOLD_LEVEL == 8
        # Levels below threshold should have <= 100 days
        for level in range(1, BATCH_THRESHOLD_LEVEL):
            assert SPEED_CONFIG[level] <= 100
        # Levels at/above threshold should have >= 250 days
        for level in range(BATCH_THRESHOLD_LEVEL, 16):
            assert SPEED_CONFIG[level] >= 250


class TestEngineSystemRegistration:
    def test_register_system(self):
        engine = TimeEngine()
        sys1 = CountingSystem()
        sys2 = EventSystem()
        engine.register_system(sys2)
        engine.register_system(sys1)
        # Should be sorted by order
        assert engine._systems[0] is sys1  # order=1
        assert engine._systems[1] is sys2  # order=2

    def test_engine_has_world(self):
        engine = TimeEngine()
        assert isinstance(engine.world, World)


class TestEngineSpeedControl:
    def test_set_valid_speed(self):
        engine = TimeEngine()
        engine.set_speed(5)
        assert engine.speed == 5

    def test_set_invalid_speed_ignored(self):
        engine = TimeEngine()
        engine.set_speed(99)
        assert engine.speed == 1  # unchanged from default

    def test_pause_resume(self):
        engine = TimeEngine()
        engine.paused = False
        engine.pause()
        assert engine.paused
        engine.resume()
        assert not engine.paused


class TestEngineTickOneDay:
    def test_tick_runs_systems(self):
        engine = TimeEngine()
        counter = CountingSystem()
        engine.register_system(counter)

        events = _run(engine._tick_one_day(GameTime(10)))
        assert counter.tick_count == 1
        assert isinstance(events, list)

    def test_tick_collects_events(self):
        engine = TimeEngine()
        event_sys = EventSystem()
        engine.register_system(event_sys)

        # Day 365 is year 2 start
        events = _run(engine._tick_one_day(GameTime(365)))
        assert len(events) == 1
        assert events[0].event_type == "test.year"

    def test_tick_handles_system_error(self):
        class BrokenSystem(System):
            order = 1
            active_phases = None
            def tick(self, world, game_time):
                raise ValueError("broke")

        engine = TimeEngine()
        engine.register_system(BrokenSystem())
        events = _run(engine._tick_one_day(GameTime(1)))
        assert len(events) == 1
        assert events[0].event_type == "engine.error"
        assert "broke" in events[0].data["error"]

    def test_legacy_tick_handler_still_works(self):
        engine = TimeEngine()
        called = []

        async def legacy_handler(game_time):
            called.append(game_time)
            return []

        engine.on_tick(legacy_handler)
        _run(engine._tick_one_day(GameTime(5)))
        assert len(called) == 1


class TestEngineStateDict:
    def test_state_dict_format(self):
        engine = TimeEngine()
        engine.day = 100
        engine.speed = 7
        d = engine.state_dict()
        assert d["type"] == "state"
        assert d["speed"] == 7
        assert "year" in d
        assert "month" in d
        assert "day_offset" in d
