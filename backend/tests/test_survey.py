"""Tests for Survey Phase and Founding Phase."""

from __future__ import annotations

import asyncio
import random

import pytest

from outpost3.simulation.components import (
    Colony, GamePhase, Population, Resources, ShipHull, Survey, Transit,
    CandidateSystem,
)
from outpost3.simulation.loadout import run_loadout, world_state_dict
from outpost3.simulation.search import confirm_destination, run_search
from outpost3.simulation.survey import (
    SurveySystem, SURVEY_DURATION_DAYS, VERDICT_DAY,
    _colony_quality, _decide_verdict, confirm_survey_decision,
)
from outpost3.simulation.world import World
from outpost3.simulation import GameTime, Severity
from outpost3.narrative import render


# ── Helpers ────────────────────────────────────────────────────────

def _make_world(seed: int = 42) -> tuple[World, SurveySystem]:
    w = World()
    rng = random.Random(seed)
    run_loadout(w, rng=rng)
    run_search(w, rng=rng)
    # Confirm destination (required by new two-step flow)
    first_system = w.query(CandidateSystem)[0][1]
    confirm_destination(w, first_system.name, rng=rng)
    system = SurveySystem(w)
    return w, system


def _fast_forward_to_survey(w: World, system: SurveySystem):
    """Advance transit to completion so phase == 'survey'."""
    _, transit = w.query_one(Transit)
    transit.days_elapsed = transit.duration_days
    transit.arrived = True
    _, phase = w.query_one(GamePhase)
    phase.phase = "survey"


def run_tick(system: SurveySystem, day_offset: int):
    return asyncio.get_event_loop().run_until_complete(
        system.tick(GameTime(day_offset))
    )


def run_batch(system: SurveySystem, start: int, end: int):
    s, e = GameTime(start), GameTime(end)
    return asyncio.get_event_loop().run_until_complete(
        system.batch(s, e, end - start + 1)
    )


def _setup_survey(seed: int = 42) -> tuple[World, SurveySystem]:
    """Return world with phase == 'survey' and survey.days_elapsed == 0."""
    w, sys = _make_world(seed)
    _fast_forward_to_survey(w, sys)
    return w, sys


# ── Colony quality ─────────────────────────────────────────────────

class TestColonyQuality:
    def test_thriving(self):
        assert _colony_quality(80, 95, 15) == "thriving"

    def test_strong(self):
        assert _colony_quality(60, 75, 8) == "strong"

    def test_standard(self):
        assert _colony_quality(40, 60, 4) == "standard"

    def test_struggling(self):
        assert _colony_quality(10, 40, 1) == "struggling"


# ── Verdict logic ──────────────────────────────────────────────────

class TestDecideVerdict:
    def test_forced_accept_at_max_rejections(self):
        assert _decide_verdict(3, 10.0, 50.0, 1.0) == "accept"

    def test_accept_high_habitability(self):
        assert _decide_verdict(0, 65.0, 90.0, 20.0) == "accept"

    def test_reject_low_habitability_good_resources(self):
        assert _decide_verdict(0, 15.0, 90.0, 20.0) == "reject"

    def test_accept_low_habitability_damaged_hull(self):
        # Can't afford to search again — hull too damaged
        assert _decide_verdict(1, 20.0, 50.0, 1.0) == "accept"

    def test_accept_low_habitability_low_food(self):
        assert _decide_verdict(1, 20.0, 80.0, 1.5) == "accept"

    def test_reject_borderline_with_resources(self):
        assert _decide_verdict(0, 30.0, 90.0, 15.0) == "reject"


# ── Phase gating ───────────────────────────────────────────────────

class TestSurveyPhaseGating:
    def test_returns_empty_when_not_survey_phase(self):
        w, system = _make_world()
        # Phase is "transit" before arrival
        events = run_tick(system, 1)
        assert events == []

    def test_returns_empty_when_founding_phase(self):
        w, system = _setup_survey()
        _, phase = w.query_one(GamePhase)
        phase.phase = "founding"
        events = run_tick(system, 1)
        assert events == []


# ── Survey started event ───────────────────────────────────────────

class TestSurveyStarted:
    def test_started_event_on_first_tick(self):
        w, system = _setup_survey()
        events = run_tick(system, 1)
        started = [e for e in events if e.event_type == "survey.started"]
        assert len(started) == 1

    def test_started_not_repeated(self):
        w, system = _setup_survey()
        run_tick(system, 1)   # first tick: emits started + advances days_elapsed
        events = run_tick(system, 2)
        started = [e for e in events if e.event_type == "survey.started"]
        assert len(started) == 0

    def test_started_has_system_name(self):
        w, system = _setup_survey()
        events = run_tick(system, 1)
        started = next(e for e in events if e.event_type == "survey.started")
        _, survey = w.query_one(Survey)
        assert started.data["system_name"] == survey.target_system_name


# ── Planet scans ───────────────────────────────────────────────────

class TestPlanetScans:
    def test_planet_scan_events_emitted(self):
        w, system = _setup_survey()
        _, survey = w.query_one(Survey)
        target = next(s for _, s in w.query(CandidateSystem) if s.selected)
        # Run enough ticks to cover all scan days
        all_events = []
        for day in range(1, SURVEY_DURATION_DAYS + 1):
            all_events.extend(run_tick(system, day))
            if survey.verdict is not None:
                break

        scans = [e for e in all_events if e.event_type == "survey.planet_scanned"]
        # Should get one scan per planet
        assert len(scans) == len(target.planets)

    def test_planet_scan_has_habitability(self):
        w, system = _setup_survey()
        target = next(s for _, s in w.query(CandidateSystem) if s.selected)
        all_events = []
        for day in range(1, VERDICT_DAY + 10):
            all_events.extend(run_tick(system, day))
        scans = [e for e in all_events if e.event_type == "survey.planet_scanned"]
        if scans:
            assert "habitability" in scans[0].data
            assert "planet_name" in scans[0].data


# ── Accept path ────────────────────────────────────────────────────

class TestAcceptPath:
    def _force_accept(self, w: World):
        """Ensure verdict will be 'accept' by setting high habitability."""
        for _, sys in w.query(CandidateSystem):
            if sys.selected:
                for p in sys.planets:
                    p.habitability = 75.0
                sys.best_habitability = 75.0

    def _run_to_verdict(self, w: World, system: SurveySystem):
        """Tick until survey.verdict_pending fires; return (day, all_events)."""
        all_events = []
        for day in range(1, SURVEY_DURATION_DAYS + 10):
            evts = run_tick(system, day)
            all_events.extend(evts)
            if any(e.event_type == "survey.verdict_pending" for e in evts):
                return day, all_events
        return SURVEY_DURATION_DAYS, all_events

    def test_verdict_pending_emitted_on_accept(self):
        w, system = _setup_survey(seed=10)
        self._force_accept(w)
        _, all_events = self._run_to_verdict(w, system)
        pending = [e for e in all_events if e.event_type == "survey.verdict_pending"]
        assert len(pending) == 1
        assert pending[0].data["recommendation"] == "accept"

    def test_colony_founded_event_on_accept(self):
        w, system = _setup_survey(seed=10)
        self._force_accept(w)
        day, _ = self._run_to_verdict(w, system)
        events = confirm_survey_decision(w, "accept", GameTime(day))
        founded = [e for e in events if e.event_type == "colony.founded"]
        assert len(founded) == 1
        assert founded[0].auto_pause is True
        assert founded[0].severity == Severity.MILESTONE

    def test_colony_component_created(self):
        w, system = _setup_survey(seed=10)
        self._force_accept(w)
        day, _ = self._run_to_verdict(w, system)
        confirm_survey_decision(w, "accept", GameTime(day))
        pair = w.query_one(Colony)
        assert pair is not None
        _, colony = pair
        assert colony.quality in ("struggling", "standard", "strong", "thriving")
        assert colony.starting_population > 0

    def test_phase_advances_to_founding(self):
        w, system = _setup_survey(seed=10)
        self._force_accept(w)
        day, _ = self._run_to_verdict(w, system)
        confirm_survey_decision(w, "accept", GameTime(day))
        _, phase = w.query_one(GamePhase)
        assert phase.phase == "founding"

    def test_colony_founded_narrative_renders(self):
        w, system = _setup_survey(seed=10)
        self._force_accept(w)
        day, _ = self._run_to_verdict(w, system)
        events = confirm_survey_decision(w, "accept", GameTime(day))
        event = next(e for e in events if e.event_type == "colony.founded")
        text = render(event)
        assert text is not None
        assert len(text) > 20


# ── Reject path ────────────────────────────────────────────────────

class TestRejectPath:
    def _force_reject(self, w: World):
        """Set very low habitability so crew recommends rejection."""
        for _, sys in w.query(CandidateSystem):
            if sys.selected:
                for p in sys.planets:
                    p.habitability = 5.0
                sys.best_habitability = 5.0
        # Ensure resources are plentiful so crew will reject (not forced accept)
        ship_eid, _ = w.query_one(ShipHull)
        res = w.get(ship_eid, Resources)
        if res:
            _, pop = w.query_one(Population)
            res.food = pop.count * 365 * 30  # 30 years of food
        _, hull = w.query_one(ShipHull)
        hull.integrity = 95.0

    def _run_to_verdict(self, w: World, system: SurveySystem):
        for day in range(1, SURVEY_DURATION_DAYS + 10):
            evts = run_tick(system, day)
            if any(e.event_type == "survey.verdict_pending" for e in evts):
                return day
        return SURVEY_DURATION_DAYS

    def test_survey_rejected_event(self):
        w, system = _setup_survey(seed=5)
        self._force_reject(w)
        day = self._run_to_verdict(w, system)
        events = confirm_survey_decision(w, "reject", GameTime(day))
        rejected = [e for e in events if e.event_type == "survey.rejected"]
        assert len(rejected) == 1
        # auto_pause is False — player already decided to reject
        assert rejected[0].auto_pause is False

    def test_new_search_triggered_after_rejection(self):
        w, system = _setup_survey(seed=5)
        self._force_reject(w)
        day = self._run_to_verdict(w, system)
        confirm_survey_decision(w, "reject", GameTime(day))
        # After rejection, run_search() is called → phase = "search" waiting for player
        _, phase = w.query_one(GamePhase)
        assert phase.phase == "search"

    def test_new_candidates_generated_after_rejection(self):
        w, system = _setup_survey(seed=5)
        self._force_reject(w)
        day = self._run_to_verdict(w, system)
        confirm_survey_decision(w, "reject", GameTime(day))
        # New candidate systems exist and await player choice
        systems = w.query(CandidateSystem)
        assert 3 <= len(systems) <= 5

    def test_rejection_count_increments(self):
        from outpost3.simulation.components import PendingSearch
        w, system = _setup_survey(seed=5)
        self._force_reject(w)
        day = self._run_to_verdict(w, system)
        confirm_survey_decision(w, "reject", GameTime(day))
        # rejection_count is now in PendingSearch (survey was cleared)
        pending_pair = w.query_one(PendingSearch)
        assert pending_pair is not None
        _, pending = pending_pair
        assert pending.rejection_count == 1

    def test_forced_accept_recommendation_at_max_rejections(self):
        w, system = _setup_survey(seed=5)
        self._force_reject(w)
        # Manually set rejection count to 3 (max)
        _, survey = w.query_one(Survey)
        survey.rejection_count = 3

        all_events = []
        for day in range(1, SURVEY_DURATION_DAYS + 10):
            evts = run_tick(system, day)
            all_events.extend(evts)
            if any(e.event_type == "survey.verdict_pending" for e in evts):
                break

        # Recommendation should be "accept" despite low habitability
        pending_events = [e for e in all_events if e.event_type == "survey.verdict_pending"]
        assert len(pending_events) == 1
        assert pending_events[0].data["recommendation"] == "accept"


# ── Batch handler ──────────────────────────────────────────────────

class TestSurveyBatch:
    def _force_accept(self, w: World):
        for _, sys in w.query(CandidateSystem):
            if sys.selected:
                for p in sys.planets:
                    p.habitability = 75.0
                sys.best_habitability = 75.0

    def test_batch_emits_started(self):
        w, system = _setup_survey()
        events = run_batch(system, 0, 30)
        started = [e for e in events if e.event_type == "survey.started"]
        assert len(started) == 1

    def test_batch_scans_planets(self):
        w, system = _setup_survey()
        events = run_batch(system, 0, SURVEY_DURATION_DAYS)
        scans = [e for e in events if e.event_type == "survey.batch_scan"]
        assert len(scans) >= 1

    def test_batch_emits_verdict_when_crossing_verdict_day(self):
        w, system = _setup_survey(seed=10)
        self._force_accept(w)
        events = run_batch(system, 0, SURVEY_DURATION_DAYS)
        pending = [e for e in events if e.event_type == "survey.verdict_pending"]
        assert len(pending) == 1
        assert pending[0].data["recommendation"] == "accept"

    def test_batch_returns_empty_when_not_survey_phase(self):
        w, system = _make_world()
        events = run_batch(system, 0, 365)
        assert events == []


# ── World state dict ───────────────────────────────────────────────

class TestWorldStateDictSurveyFounding:
    def _make_confirmed_world(self, seed: int = 42) -> World:
        w = World()
        rng = random.Random(seed)
        run_loadout(w, rng=rng)
        run_search(w, rng=rng)
        first_system = w.query(CandidateSystem)[0][1]
        confirm_destination(w, first_system.name, rng=rng)
        return w

    def test_includes_survey_key(self):
        w = self._make_confirmed_world()
        state = world_state_dict(w)
        assert "survey" in state
        assert "colony" in state

    def test_survey_has_target_name(self):
        w = self._make_confirmed_world()
        state = world_state_dict(w)
        assert "target_system_name" in state["survey"]

    def test_colony_empty_before_founding(self):
        w = self._make_confirmed_world()
        state = world_state_dict(w)
        assert state["colony"] == {}
