"""Tests for the simulation engine and event system."""

from outpost3.simulation import GameTime, GameEvent, Severity, DAYS_PER_YEAR
from outpost3.narrative import render


class TestGameTime:
    def test_year_one_day_one(self):
        gt = GameTime(0)
        assert gt.year == 1
        assert gt.day_of_year == 1
        assert gt.month == 1
        assert gt.day_of_month == 1

    def test_year_boundary(self):
        gt = GameTime(DAYS_PER_YEAR)
        assert gt.year == 2
        assert gt.day_of_year == 1
        assert gt.is_year_start

    def test_month_boundaries(self):
        # Day 31 (offset 30) should be month 2, day 1
        gt = GameTime(30)
        assert gt.month == 2
        assert gt.day_of_month == 1
        assert gt.is_month_start

    def test_month_capped_at_12(self):
        # Last day of the year should still be month 12
        gt = GameTime(DAYS_PER_YEAR - 1)
        assert gt.month == 12

    def test_is_year_start(self):
        assert GameTime(0).is_year_start
        assert GameTime(DAYS_PER_YEAR).is_year_start
        assert not GameTime(1).is_year_start

    def test_is_month_start(self):
        assert GameTime(0).is_month_start  # day_of_month == 1
        assert GameTime(30).is_month_start  # month 2, day 1
        assert not GameTime(1).is_month_start

    def test_to_dict(self):
        gt = GameTime(42)
        d = gt.to_dict()
        assert d["day_offset"] == 42
        assert d["year"] == 1
        assert d["day"] == 43
        assert "month" in d
        assert "day_of_month" in d

    def test_str(self):
        gt = GameTime(0)
        s = str(gt)
        assert "Year 1" in s


class TestGameEvent:
    def test_to_dict(self):
        gt = GameTime(10)
        event = GameEvent(
            event_type="test.event",
            severity=Severity.INFO,
            game_time=gt,
            data={"key": "value"},
        )
        d = event.to_dict()
        assert d["event_type"] == "test.event"
        assert d["severity"] == "info"
        assert d["game_time"]["day_offset"] == 10
        assert d["data"]["key"] == "value"
        assert d["auto_pause"] is False

    def test_auto_pause_flag(self):
        event = GameEvent(
            event_type="test",
            severity=Severity.MILESTONE,
            game_time=GameTime(0),
            auto_pause=True,
        )
        assert event.auto_pause is True
        assert event.to_dict()["auto_pause"] is True


class TestNarrative:
    def test_year_start_template(self):
        event = GameEvent(
            event_type="time.year_start",
            severity=Severity.NOTABLE,
            game_time=GameTime(DAYS_PER_YEAR),
            data={"year": 2},
        )
        text = render(event)
        assert text is not None
        assert "Year 2" in text

    def test_month_start_template(self):
        event = GameEvent(
            event_type="time.month_start",
            severity=Severity.INFO,
            game_time=GameTime(30),
            data={"month": 2, "year": 1},
        )
        text = render(event)
        assert text is not None
        assert "Month 2" in text

    def test_unknown_event_returns_none(self):
        event = GameEvent(
            event_type="unknown.type",
            severity=Severity.INFO,
            game_time=GameTime(0),
        )
        assert render(event) is None

    def test_variant_template(self):
        event = GameEvent(
            event_type="daily.weather",
            severity=Severity.DEBUG,
            game_time=GameTime(5),
            data={"variant": "clear_skies", "year": 1, "month": 1, "day": 6},
        )
        text = render(event)
        assert text is not None
        assert len(text) > 0

    def test_auto_pause_template(self):
        event = GameEvent(
            event_type="auto_pause.year_end",
            severity=Severity.MILESTONE,
            game_time=GameTime(DAYS_PER_YEAR),
            auto_pause=True,
            data={"year": 1},
        )
        text = render(event)
        assert text is not None
        assert "Year 1" in text
