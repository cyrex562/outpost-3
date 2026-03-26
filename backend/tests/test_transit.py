"""Tests for TransitSystem tick and batch handlers."""

from __future__ import annotations

import asyncio
import random

import pytest

from outpost3.simulation.components import (
    CandidateSystem, GamePhase, Population, Resources, ShipHull, Transit,
)
from outpost3.simulation.loadout import run_loadout
from outpost3.simulation.search import confirm_destination, run_search
from outpost3.simulation.transit import TransitSystem, BIRTH_RATE, DEATH_RATE
from outpost3.simulation.world import World
from outpost3.simulation import GameTime, Severity
from outpost3.narrative import render


# ── helpers ────────────────────────────────────────────────────────

def _make_world(seed: int = 42) -> tuple[World, TransitSystem]:
    w = World()
    rng = random.Random(seed)
    run_loadout(w, rng=rng)
    run_search(w, rng=rng)
    # Confirm destination so Transit component exists and phase == "transit"
    first_system = w.query(CandidateSystem)[0][1]
    confirm_destination(w, first_system.name, rng=rng)
    system = TransitSystem(w)
    return w, system


def run_tick(system: TransitSystem, day_offset: int):
    return asyncio.get_event_loop().run_until_complete(
        system.tick(GameTime(day_offset))
    )


def run_batch(system: TransitSystem, start: int, end: int):
    s = GameTime(start)
    e = GameTime(end)
    return asyncio.get_event_loop().run_until_complete(
        system.batch(s, e, end - start + 1)
    )


# ── Phase gating ───────────────────────────────────────────────────

class TestTransitPhaseGating:
    def test_tick_returns_empty_if_not_transit_phase(self):
        w = World()
        rng = random.Random(1)
        run_loadout(w, rng=rng)
        # phase is "loadout" before search
        system = TransitSystem(w)
        events = run_tick(system, 1)
        assert events == []

    def test_batch_returns_empty_if_not_transit_phase(self):
        w = World()
        rng = random.Random(1)
        run_loadout(w, rng=rng)
        system = TransitSystem(w)
        events = run_batch(system, 1, 365)
        assert events == []

    def test_tick_returns_empty_after_arrival(self):
        w, system = _make_world()
        _, transit = w.query_one(Transit)
        transit.arrived = True
        events = run_tick(system, 1)
        assert events == []


# ── Tick — resource consumption ────────────────────────────────────

class TestTickResources:
    def test_tick_consumes_food(self):
        w, system = _make_world()
        ship_eid, _ = w.query_one(ShipHull)
        res = w.get(ship_eid, Resources)
        food_before = res.food

        run_tick(system, 1)

        assert res.food < food_before

    def test_tick_consumes_water(self):
        w, system = _make_world()
        ship_eid, _ = w.query_one(ShipHull)
        res = w.get(ship_eid, Resources)
        water_before = res.water

        run_tick(system, 1)

        assert res.water < water_before

    def test_tick_consumes_fuel(self):
        w, system = _make_world()
        ship_eid, _ = w.query_one(ShipHull)
        res = w.get(ship_eid, Resources)
        fuel_before = res.fuel

        run_tick(system, 1)

        assert res.fuel < fuel_before

    def test_resources_never_go_negative(self):
        w, system = _make_world()
        ship_eid, _ = w.query_one(ShipHull)
        res = w.get(ship_eid, Resources)
        res.food = 0
        res.water = 0

        run_tick(system, 1)

        assert res.food == 0
        assert res.water == 0

    def test_tick_advances_transit_days(self):
        w, system = _make_world()
        _, transit = w.query_one(Transit)
        assert transit.days_elapsed == 0

        run_tick(system, 1)

        assert transit.days_elapsed == 1


# ── Tick — year boundary ───────────────────────────────────────────

class TestTickYearBoundary:
    def test_year_summary_emitted_on_year_start(self):
        w, system = _make_world()
        _, transit = w.query_one(Transit)
        transit.days_elapsed = 364   # one day before year end

        events = run_tick(system, 365)   # day 365 = year 2 start

        year_events = [e for e in events if e.event_type == "transit.year_summary"]
        assert len(year_events) == 1

    def test_year_summary_not_emitted_mid_year(self):
        w, system = _make_world()
        _, transit = w.query_one(Transit)
        transit.days_elapsed = 50

        events = run_tick(system, 51)

        year_events = [e for e in events if e.event_type == "transit.year_summary"]
        assert len(year_events) == 0

    def test_population_increases_on_year_start(self):
        w, system = _make_world()
        _, pop = w.query_one(Population)
        _, transit = w.query_one(Transit)
        transit.days_elapsed = 364
        initial_pop = pop.count

        run_tick(system, 365)

        # Net +0.7% per year
        assert pop.count > initial_pop

    def test_medicine_consumed_on_year_start(self):
        w, system = _make_world()
        ship_eid, _ = w.query_one(ShipHull)
        res = w.get(ship_eid, Resources)
        _, transit = w.query_one(Transit)
        transit.days_elapsed = 364
        medicine_before = res.medicine

        run_tick(system, 365)

        assert res.medicine < medicine_before


# ── Tick — arrival ────────────────────────────────────────────────

class TestTickArrival:
    def test_arrival_event_emitted_when_done(self):
        w, system = _make_world()
        _, transit = w.query_one(Transit)
        transit.days_elapsed = transit.duration_days - 1

        events = run_tick(system, transit.duration_days)

        arrival_events = [e for e in events if e.event_type == "transit.arrival"]
        assert len(arrival_events) == 1
        assert arrival_events[0].auto_pause is True
        assert arrival_events[0].severity == Severity.MILESTONE

    def test_arrival_advances_phase_to_survey(self):
        w, system = _make_world()
        _, transit = w.query_one(Transit)
        transit.days_elapsed = transit.duration_days - 1
        _, phase = w.query_one(GamePhase)

        run_tick(system, transit.duration_days)

        assert phase.phase == "survey"

    def test_arrived_flag_set(self):
        w, system = _make_world()
        _, transit = w.query_one(Transit)
        transit.days_elapsed = transit.duration_days - 1

        run_tick(system, transit.duration_days)

        assert transit.arrived is True


# ── Batch ─────────────────────────────────────────────────────────

class TestBatch:
    def test_batch_consumes_more_resources_than_tick(self):
        w1, sys1 = _make_world(1)
        w2, sys2 = _make_world(1)

        ship1_eid, _ = w1.query_one(ShipHull)
        res1 = w1.get(ship1_eid, Resources)
        food_before_single = res1.food
        run_tick(sys1, 1)
        food_after_single = res1.food

        ship2_eid, _ = w2.query_one(ShipHull)
        res2 = w2.get(ship2_eid, Resources)
        food_before_batch = res2.food
        run_batch(sys2, 1, 30)
        food_after_batch = res2.food

        single_consumed = food_before_single - food_after_single
        batch_consumed = food_before_batch - food_after_batch
        assert batch_consumed > single_consumed

    def test_batch_advances_transit_days(self):
        w, system = _make_world()
        _, transit = w.query_one(Transit)

        run_batch(system, 1, 30)

        assert transit.days_elapsed == 30

    def test_batch_emits_year_events_when_year_crossed(self):
        w, system = _make_world()
        # Batch that crosses a year boundary (day 0 → day 366)
        events = run_batch(system, 0, 366)

        year_events = [e for e in events if e.event_type == "transit.year_summary"]
        assert len(year_events) >= 1

    def test_batch_arrival_sets_phase(self):
        w, system = _make_world()
        _, transit = w.query_one(Transit)
        _, phase = w.query_one(GamePhase)

        # Batch past the end of transit
        run_batch(system, 0, transit.duration_days)

        assert phase.phase == "survey"
        assert transit.arrived is True

    def test_batch_summary_event_emitted_when_no_year_boundary(self):
        w, system = _make_world()
        # 5-day batch within a single year — no year boundary
        events = run_batch(system, 1, 5)

        summary = [e for e in events if e.event_type == "transit.batch_summary"]
        year = [e for e in events if e.event_type == "transit.year_summary"]
        # If no year boundary crossed, we get a batch_summary
        if not year:
            assert len(summary) >= 1


# ── Narrative ─────────────────────────────────────────────────────

class TestTransitNarrative:
    def test_year_summary_renders(self):
        from outpost3.simulation import GameEvent, Severity
        event = GameEvent(
            event_type="transit.year_summary",
            severity=Severity.NOTABLE,
            game_time=GameTime(365),
            data={
                "transit_year": 1,
                "population": 1000,
                "births": 15,
                "deaths": 8,
                "years_remaining": 24.0,
                "food_years": 18.5,
                "hull_integrity": 99.2,
            },
        )
        text = render(event)
        assert text is not None
        assert len(text) > 10

    def test_arrival_renders(self):
        from outpost3.simulation import GameEvent, Severity
        event = GameEvent(
            event_type="transit.arrival",
            severity=Severity.MILESTONE,
            game_time=GameTime(9125),
            auto_pause=True,
            data={
                "destination": "Alpha Eridani",
                "population": 1150,
                "hull_integrity": 82.3,
                "transit_years": 25,
            },
        )
        text = render(event)
        assert text is not None
        assert "Alpha Eridani" in text

    def test_equipment_failure_renders(self):
        from outpost3.simulation import GameEvent, Severity
        event = GameEvent(
            event_type="transit.equipment_failure",
            severity=Severity.NOTABLE,
            game_time=GameTime(100),
            data={
                "hull_integrity": 99.5,
                "food_cost": 0,
                "water_cost": 0,
                "medicine_cost": 0,
                "parts_cost": 500,
            },
        )
        text = render(event)
        assert text is not None
        assert "500" in text
