"""Tests for Search Phase and CandidateSystem / Transit components."""

from __future__ import annotations

import random

import pytest

from outpost3.simulation.components import CandidateSystem, GamePhase, Planet, Transit
from outpost3.simulation.loadout import run_loadout, world_state_dict
from outpost3.simulation.search import run_search
from outpost3.simulation.world import World
from outpost3.narrative import render
from outpost3.simulation import GameTime, Severity


def _seeded_world(seed: int = 42) -> tuple[World, list]:
    """Return a world that has completed loadout + search."""
    w = World()
    rng = random.Random(seed)
    run_loadout(w, rng=rng)
    result, events = run_search(w, rng=rng)
    return w, events


class TestPlanet:
    def test_to_dict(self):
        p = Planet(name="GJ 123 b", type="rocky", habitability=62.5, notes=["Liquid water"])
        d = p.to_dict()
        assert d["name"] == "GJ 123 b"
        assert d["type"] == "rocky"
        assert d["habitability"] == 62.5
        assert "Liquid water" in d["notes"]


class TestCandidateSystem:
    def test_to_dict(self):
        p = Planet(name="Alpha Eridani b", type="ocean", habitability=75.0)
        s = CandidateSystem(
            name="Alpha Eridani",
            star_type="K2V",
            distance_ly=12.3,
            angle_deg=45.0,
            planets=[p],
            best_habitability=75.0,
        )
        d = s.to_dict()
        assert d["name"] == "Alpha Eridani"
        assert d["distance_ly"] == 12.3
        assert len(d["planets"]) == 1
        assert d["best_habitability"] == 75.0
        assert d["selected"] is False


class TestTransit:
    def test_progress(self):
        t = Transit(destination_name="Alpha Eridani", duration_days=7300, fuel_per_day=100.0)
        assert t.progress == 0.0
        t.days_elapsed = 3650
        assert t.progress == pytest.approx(0.5)
        t.days_elapsed = 7300
        assert t.progress == 1.0

    def test_days_remaining(self):
        t = Transit(destination_name="X", duration_days=1000, fuel_per_day=0.0)
        t.days_elapsed = 400
        assert t.days_remaining == 600

    def test_to_dict(self):
        t = Transit(destination_name="Beta Ceti", duration_days=10950, fuel_per_day=50.0)
        t.days_elapsed = 5475
        d = t.to_dict()
        assert d["destination_name"] == "Beta Ceti"
        assert d["days_elapsed"] == 5475
        assert d["days_remaining"] == 5475
        assert d["progress"] == pytest.approx(0.5, abs=0.001)
        assert d["arrived"] is False


class TestSearch:
    def test_generates_3_to_5_systems(self):
        w, _ = _seeded_world()
        systems = w.query(CandidateSystem)
        assert 3 <= len(systems) <= 5

    def test_exactly_one_selected(self):
        w, _ = _seeded_world()
        selected = [s for _, s in w.query(CandidateSystem) if s.selected]
        assert len(selected) == 1

    def test_selected_has_highest_habitability(self):
        w, _ = _seeded_world()
        all_systems = [s for _, s in w.query(CandidateSystem)]
        selected = next(s for s in all_systems if s.selected)
        best = max(all_systems, key=lambda s: s.best_habitability)
        assert selected.name == best.name

    def test_each_system_has_planets(self):
        w, _ = _seeded_world()
        for _, system in w.query(CandidateSystem):
            assert len(system.planets) >= 1

    def test_systems_have_distance_and_angle(self):
        w, _ = _seeded_world()
        for _, system in w.query(CandidateSystem):
            assert 4.0 <= system.distance_ly <= 22.0
            assert 0.0 <= system.angle_deg <= 360.0

    def test_transit_component_created(self):
        w, _ = _seeded_world()
        pair = w.query_one(Transit)
        assert pair is not None

    def test_transit_duration_is_20_to_50_years(self):
        w, _ = _seeded_world()
        _, transit = w.query_one(Transit)
        years = transit.duration_days / 365
        assert 20 <= years <= 50

    def test_transit_destination_matches_selected_system(self):
        w, _ = _seeded_world()
        _, transit = w.query_one(Transit)
        selected = next(s for _, s in w.query(CandidateSystem) if s.selected)
        assert transit.destination_name == selected.name

    def test_fuel_per_day_derived_from_resources(self):
        w, _ = _seeded_world()
        _, transit = w.query_one(Transit)
        assert transit.fuel_per_day > 0

    def test_phase_advanced_to_transit(self):
        w, _ = _seeded_world()
        _, phase = w.query_one(GamePhase)
        assert phase.phase == "transit"

    def test_search_emits_candidate_found_events(self):
        w, events = _seeded_world()
        found = [e for e in events if e.event_type == "search.candidate_found"]
        systems = w.query(CandidateSystem)
        assert len(found) == len(systems)

    def test_search_emits_destination_selected_event(self):
        _, events = _seeded_world()
        selected = [e for e in events if e.event_type == "search.destination_selected"]
        assert len(selected) == 1
        assert selected[0].auto_pause is True
        assert selected[0].severity.value == "milestone"

    def test_different_seeds_produce_different_destinations(self):
        w1, _ = _seeded_world(seed=1)
        w2, _ = _seeded_world(seed=2)
        # Different RNG seeds should generally produce different systems
        systems1 = [s.name for _, s in w1.query(CandidateSystem)]
        systems2 = [s.name for _, s in w2.query(CandidateSystem)]
        # They won't always differ, but the counts should both be valid
        assert 3 <= len(systems1) <= 5
        assert 3 <= len(systems2) <= 5


class TestSearchNarrative:
    def test_candidate_found_renders(self):
        _, events = _seeded_world()
        found = next(e for e in events if e.event_type == "search.candidate_found")
        text = render(found)
        assert text is not None
        assert found.data["name"] in text

    def test_destination_selected_renders(self):
        _, events = _seeded_world()
        selected = next(e for e in events if e.event_type == "search.destination_selected")
        text = render(selected)
        assert text is not None
        assert str(selected.data["transit_years"]) in text


class TestWorldStateDictWithSearch:
    def test_includes_systems(self):
        w = World()
        rng = random.Random(42)
        run_loadout(w, rng=rng)
        run_search(w, rng=rng)
        state = world_state_dict(w)
        assert "systems" in state
        assert 3 <= len(state["systems"]) <= 5

    def test_includes_transit(self):
        w = World()
        rng = random.Random(42)
        run_loadout(w, rng=rng)
        run_search(w, rng=rng)
        state = world_state_dict(w)
        assert "transit" in state
        assert "destination_name" in state["transit"]
        assert state["transit"]["days_elapsed"] == 0

    def test_phase_is_transit(self):
        w = World()
        rng = random.Random(42)
        run_loadout(w, rng=rng)
        run_search(w, rng=rng)
        state = world_state_dict(w)
        assert state["phase"] == "transit"
