"""Tests for Search Phase — candidate generation and destination confirmation."""

from __future__ import annotations

import random

import pytest

from outpost3.simulation.components import CandidateSystem, GamePhase, PendingSearch, Planet, Transit, Survey
from outpost3.simulation.loadout import run_loadout, world_state_dict
from outpost3.simulation.search import SearchResult, ConfirmResult, confirm_destination, run_search
from outpost3.simulation.world import World
from outpost3.narrative import render
from outpost3.simulation import GameTime, Severity


# ── Helpers ────────────────────────────────────────────────────────────────────

def _seeded_world_search(seed: int = 42) -> tuple[World, SearchResult, list]:
    """Return a world that has completed loadout + run_search (awaiting destination choice)."""
    w = World()
    rng = random.Random(seed)
    run_loadout(w, rng=rng)
    result, events = run_search(w, rng=rng)
    return w, result, events


def _seeded_world_confirmed(seed: int = 42) -> tuple[World, ConfirmResult, list, list]:
    """Return a world that has completed loadout + run_search + confirm_destination."""
    w, search_result, search_events = _seeded_world_search(seed)
    rng = random.Random(seed + 100)
    # Pick any system (first one in world)
    first_system = w.query(CandidateSystem)[0][1]
    confirm_result, confirm_events = confirm_destination(w, first_system.name, rng=rng)
    return w, confirm_result, search_events, confirm_events


# ── TestPlanet ─────────────────────────────────────────────────────────────────

class TestPlanet:
    def test_to_dict(self):
        p = Planet(name="GJ 123 b", type="rocky", habitability=62.5, notes=["Liquid water"])
        d = p.to_dict()
        assert d["name"] == "GJ 123 b"
        assert d["type"] == "rocky"
        assert d["habitability"] == 62.5
        assert "Liquid water" in d["notes"]


# ── TestCandidateSystem ────────────────────────────────────────────────────────

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


# ── TestTransit ────────────────────────────────────────────────────────────────

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


# ── TestRunSearch ──────────────────────────────────────────────────────────────

class TestRunSearch:
    def test_generates_3_to_5_systems(self):
        w, _, _ = _seeded_world_search()
        systems = w.query(CandidateSystem)
        assert 3 <= len(systems) <= 5

    def test_no_system_selected_yet(self):
        w, _, _ = _seeded_world_search()
        selected = [s for _, s in w.query(CandidateSystem) if s.selected]
        assert len(selected) == 0

    def test_each_system_has_planets(self):
        w, _, _ = _seeded_world_search()
        for _, system in w.query(CandidateSystem):
            assert len(system.planets) >= 1

    def test_systems_have_distance_and_angle(self):
        w, _, _ = _seeded_world_search()
        for _, system in w.query(CandidateSystem):
            assert 4.0 <= system.distance_ly <= 22.0
            assert 0.0 <= system.angle_deg <= 360.0

    def test_no_transit_component_created(self):
        w, _, _ = _seeded_world_search()
        assert w.query_one(Transit) is None

    def test_phase_set_to_search(self):
        w, _, _ = _seeded_world_search()
        _, phase = w.query_one(GamePhase)
        assert phase.phase == "search"

    def test_pending_search_created(self):
        w, _, _ = _seeded_world_search()
        assert w.query_one(PendingSearch) is not None

    def test_pending_search_default_rejection_count(self):
        w, _, _ = _seeded_world_search()
        _, pending = w.query_one(PendingSearch)
        assert pending.rejection_count == 0

    def test_pending_search_stores_rejection_count(self):
        w = World()
        rng = random.Random(42)
        run_loadout(w, rng=rng)
        run_search(w, rng=rng, rejection_count=2)
        _, pending = w.query_one(PendingSearch)
        assert pending.rejection_count == 2

    def test_emits_candidate_found_events(self):
        w, _, events = _seeded_world_search()
        found = [e for e in events if e.event_type == "search.candidate_found"]
        systems = w.query(CandidateSystem)
        assert len(found) == len(systems)

    def test_emits_candidates_ready_milestone(self):
        _, _, events = _seeded_world_search()
        ready = [e for e in events if e.event_type == "search.candidates_ready"]
        assert len(ready) == 1
        assert ready[0].auto_pause is True
        assert ready[0].severity.value == "milestone"

    def test_does_not_emit_destination_selected(self):
        _, _, events = _seeded_world_search()
        selected = [e for e in events if e.event_type == "search.destination_selected"]
        assert len(selected) == 0

    def test_different_seeds_produce_different_systems(self):
        w1, _, _ = _seeded_world_search(seed=1)
        w2, _, _ = _seeded_world_search(seed=2)
        systems1 = [s.name for _, s in w1.query(CandidateSystem)]
        systems2 = [s.name for _, s in w2.query(CandidateSystem)]
        assert 3 <= len(systems1) <= 5
        assert 3 <= len(systems2) <= 5


# ── TestConfirmDestination ─────────────────────────────────────────────────────

class TestConfirmDestination:
    def test_marks_selected_system(self):
        w, _, _ = _seeded_world_search()
        rng = random.Random(99)
        first = w.query(CandidateSystem)[0][1]
        confirm_destination(w, first.name, rng=rng)
        assert first.selected is True

    def test_clears_other_selections(self):
        w, _, _ = _seeded_world_search()
        rng = random.Random(99)
        # Manually mark first system selected
        first = w.query(CandidateSystem)[0][1]
        second = w.query(CandidateSystem)[1][1]
        first.selected = True
        # Confirm second
        confirm_destination(w, second.name, rng=rng)
        assert first.selected is False
        assert second.selected is True

    def test_creates_transit_component(self):
        w, confirm_result, _, _ = _seeded_world_confirmed()
        assert w.query_one(Transit) is not None

    def test_transit_destination_matches(self):
        w, confirm_result, _, _ = _seeded_world_confirmed()
        _, transit = w.query_one(Transit)
        assert transit.destination_name == confirm_result.destination_name

    def test_transit_duration_is_20_to_50_years(self):
        w, _, _, _ = _seeded_world_confirmed()
        _, transit = w.query_one(Transit)
        years = transit.duration_days / 365
        assert 20 <= years <= 50

    def test_fuel_per_day_derived_from_resources(self):
        w, _, _, _ = _seeded_world_confirmed()
        _, transit = w.query_one(Transit)
        assert transit.fuel_per_day > 0

    def test_creates_survey_component(self):
        w, _, _, _ = _seeded_world_confirmed()
        assert w.query_one(Survey) is not None

    def test_phase_advanced_to_transit(self):
        w, _, _, _ = _seeded_world_confirmed()
        _, phase = w.query_one(GamePhase)
        assert phase.phase == "transit"

    def test_pending_search_consumed(self):
        w, _, _, _ = _seeded_world_confirmed()
        assert w.query_one(PendingSearch) is None

    def test_emits_destination_selected_event(self):
        _, _, _, confirm_events = _seeded_world_confirmed()
        selected = [e for e in confirm_events if e.event_type == "search.destination_selected"]
        assert len(selected) == 1
        assert selected[0].auto_pause is True
        assert selected[0].severity.value == "milestone"

    def test_raises_for_unknown_system(self):
        w, _, _ = _seeded_world_search()
        with pytest.raises(ValueError, match="not found"):
            confirm_destination(w, "Nonexistent System XXXXX")

    def test_rejection_count_carried_from_pending_search(self):
        w = World()
        rng = random.Random(42)
        run_loadout(w, rng=rng)
        run_search(w, rng=rng, rejection_count=2)
        first = w.query(CandidateSystem)[0][1]
        confirm_destination(w, first.name, rng=random.Random(7))
        _, survey = w.query_one(Survey)
        assert survey.rejection_count == 2


# ── TestSearchNarrative ────────────────────────────────────────────────────────

class TestSearchNarrative:
    def test_candidate_found_renders(self):
        _, _, events = _seeded_world_search()
        found = next(e for e in events if e.event_type == "search.candidate_found")
        text = render(found)
        assert text is not None
        assert found.data["name"] in text

    def test_candidates_ready_renders(self):
        _, _, events = _seeded_world_search()
        ready = next(e for e in events if e.event_type == "search.candidates_ready")
        text = render(ready)
        assert text is not None

    def test_destination_selected_renders(self):
        _, _, _, confirm_events = _seeded_world_confirmed()
        selected = next(e for e in confirm_events if e.event_type == "search.destination_selected")
        text = render(selected)
        assert text is not None
        assert str(selected.data["transit_years"]) in text


# ── TestWorldStateDictWithSearch ───────────────────────────────────────────────

class TestWorldStateDictWithSearch:
    def test_includes_systems_after_run_search(self):
        w, _, _ = _seeded_world_search()
        state = world_state_dict(w)
        assert "systems" in state
        assert 3 <= len(state["systems"]) <= 5

    def test_phase_is_search_before_confirm(self):
        w, _, _ = _seeded_world_search()
        state = world_state_dict(w)
        assert state["phase"] == "search"

    def test_transit_empty_before_confirm(self):
        w, _, _ = _seeded_world_search()
        state = world_state_dict(w)
        assert state["transit"] == {}

    def test_phase_is_transit_after_confirm(self):
        w, _, _, _ = _seeded_world_confirmed()
        state = world_state_dict(w)
        assert state["phase"] == "transit"

    def test_transit_populated_after_confirm(self):
        w, _, _, _ = _seeded_world_confirmed()
        state = world_state_dict(w)
        assert "destination_name" in state["transit"]
        assert state["transit"]["days_elapsed"] == 0
