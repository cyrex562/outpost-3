"""Tests for Milestone 1 narrative rendering (loadout, deliberation, phase transitions)."""

from outpost3.simulation import GameEvent, GameTime, Severity
from outpost3.narrative import render


class TestLoadoutNarrative:
    def test_ship_commissioned(self):
        event = GameEvent(
            event_type="loadout.ship_commissioned",
            severity=Severity.MILESTONE,
            game_time=GameTime(0),
            data={"ship_name": "CSV Horizon"},
        )
        text = render(event)
        assert text is not None
        assert "CSV Horizon" in text

    def test_loadout_summary(self):
        event = GameEvent(
            event_type="loadout.summary",
            severity=Severity.NOTABLE,
            game_time=GameTime(0),
            data={
                "ship_name": "CSV Horizon",
                "crew_size": 1200,
                "resources": {},
                "ship_systems": {},
                "morale": 0.8,
                "num_notables": 6,
            },
        )
        text = render(event)
        assert text is not None
        assert "1200" in text
        assert "CSV Horizon" in text

    def test_crew_manifest(self):
        event = GameEvent(
            event_type="loadout.crew_manifest",
            severity=Severity.INFO,
            game_time=GameTime(0),
            data={
                "total": 1200, "scientists": 180, "engineers": 240,
                "medical": 96, "military": 84, "civilians": 600,
            },
        )
        text = render(event)
        assert text is not None
        assert "1200" in text

    def test_notable_introduction(self):
        event = GameEvent(
            event_type="loadout.notable_introduction",
            severity=Severity.INFO,
            game_time=GameTime(0),
            data={
                "name": "Dr. Chen Wei",
                "role": "Chief Medical Officer",
                "age": 45,
                "traits": ["crew_welfare", "cautious"],
            },
        )
        text = render(event)
        assert text is not None
        assert "Dr. Chen Wei" in text
        assert "Chief Medical Officer" in text


class TestPhaseTransitionNarrative:
    def test_loadout_to_search(self):
        event = GameEvent(
            event_type="phase.transition",
            severity=Severity.MILESTONE,
            game_time=GameTime(0),
            data={"from": "loadout", "to": "search"},
        )
        text = render(event)
        assert text is not None
        assert len(text) > 10

    def test_unknown_transition(self):
        event = GameEvent(
            event_type="phase.transition",
            severity=Severity.MILESTONE,
            game_time=GameTime(0),
            data={"from": "x", "to": "y"},
        )
        text = render(event)
        assert text is not None
        assert "x" in text and "y" in text


class TestDeliberationNarrative:
    def test_renders_deliberation(self):
        event = GameEvent(
            event_type="behavior.deliberation",
            severity=Severity.NOTABLE,
            game_time=GameTime(100),
            data={
                "deliberation": {
                    "trigger": "food_low",
                    "trigger_detail": "Food stores at 28%",
                    "options": [
                        {
                            "action": "reduce_rations",
                            "description": "Reduce daily rations to 80%",
                            "score": 0.72,
                            "pros": ["Extends food 40 days"],
                            "cons": ["Morale impact"],
                            "advocate": "Dr. Chen",
                            "advocate_trait": "crew_welfare",
                        },
                        {
                            "action": "expand_hydroponics",
                            "description": "Expand hydroponics",
                            "score": 0.45,
                            "pros": ["Long-term fix"],
                            "cons": ["Costs parts"],
                            "advocate": None,
                            "advocate_trait": None,
                        },
                    ],
                    "chosen": {
                        "action": "reduce_rations",
                        "description": "Reduce daily rations to 80%",
                        "score": 0.72,
                    },
                    "rejected": [],
                },
            },
        )
        text = render(event)
        assert text is not None
        assert "DELIBERATION" in text
        assert "Food stores at 28%" in text
        assert "reduce_rations" in text or "Reduce daily rations" in text
        assert "Dr. Chen" in text
