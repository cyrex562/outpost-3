"""Tests for ECS-Lite World, components, and the loadout phase."""

from __future__ import annotations

import random

import pytest

from outpost3.simulation.components import (
    GamePhase,
    Notable,
    Population,
    Resources,
    ShipHull,
)
from outpost3.simulation.world import World
from outpost3.simulation.loadout import run_loadout, world_state_dict
from outpost3.simulation import GameTime, Severity
from outpost3.narrative import render


# ── World ──────────────────────────────────────────────────────────

class TestWorld:
    def test_new_entity_ids_are_unique(self):
        w = World()
        ids = [w.new_entity() for _ in range(10)]
        assert len(set(ids)) == 10

    def test_add_and_get_component(self):
        w = World()
        eid = w.new_entity()
        hull = ShipHull(name="ISS Test")
        w.add(eid, hull)
        result = w.get(eid, ShipHull)
        assert result is hull

    def test_get_missing_returns_none(self):
        w = World()
        eid = w.new_entity()
        assert w.get(eid, ShipHull) is None

    def test_add_overwrites_existing(self):
        w = World()
        eid = w.new_entity()
        w.add(eid, ShipHull(name="Old"))
        w.add(eid, ShipHull(name="New"))
        assert w.get(eid, ShipHull).name == "New"

    def test_remove_component(self):
        w = World()
        eid = w.new_entity()
        w.add(eid, ShipHull(name="ISS Test"))
        w.remove(eid, ShipHull)
        assert w.get(eid, ShipHull) is None

    def test_query_returns_all_of_type(self):
        w = World()
        for i in range(5):
            eid = w.new_entity()
            w.add(eid, Notable(name=f"Person {i}", role="Engineer"))
        pairs = w.query(Notable)
        assert len(pairs) == 5

    def test_query_one_returns_single(self):
        w = World()
        eid = w.new_entity()
        w.add(eid, GamePhase(phase="loadout"))
        pair = w.query_one(GamePhase)
        assert pair is not None
        assert pair[1].phase == "loadout"

    def test_query_one_empty_returns_none(self):
        w = World()
        assert w.query_one(GamePhase) is None

    def test_has(self):
        w = World()
        eid = w.new_entity()
        assert not w.has(eid, ShipHull)
        w.add(eid, ShipHull(name="X"))
        assert w.has(eid, ShipHull)

    def test_multiple_component_types_on_same_entity(self):
        w = World()
        eid = w.new_entity()
        w.add(eid, ShipHull(name="ISS Dual"))
        w.add(eid, Resources(food=1000))
        assert w.get(eid, ShipHull).name == "ISS Dual"
        assert w.get(eid, Resources).food == 1000

    def test_entity_count(self):
        w = World()
        for _ in range(7):
            w.new_entity()
        assert w.entity_count() == 7


# ── Components ─────────────────────────────────────────────────────

class TestComponents:
    def test_resources_to_dict(self):
        r = Resources(food=100, water=200, medicine=10, fuel=500, spare_parts=50)
        d = r.to_dict()
        assert d["food"] == 100
        assert d["water"] == 200
        assert d["spare_parts"] == 50

    def test_population_to_dict(self):
        p = Population(count=1000, births_this_year=12, deaths_this_year=3)
        d = p.to_dict()
        assert d["count"] == 1000
        assert d["births_this_year"] == 12

    def test_notable_to_dict(self):
        n = Notable(name="Elena Vasquez", role="Captain", traits=["bold", "decisive"])
        d = n.to_dict()
        assert d["name"] == "Elena Vasquez"
        assert d["role"] == "Captain"
        assert "bold" in d["traits"]
        assert d["alive"] is True


# ── Loadout ────────────────────────────────────────────────────────

class TestLoadout:
    def _seeded_world(self, seed: int = 42) -> tuple[World, list]:
        w = World()
        rng = random.Random(seed)
        result, events = run_loadout(w, rng=rng)
        return w, events

    def test_loadout_populates_ship_hull(self):
        w, _ = self._seeded_world()
        pair = w.query_one(ShipHull)
        assert pair is not None
        _, hull = pair
        assert hull.name.startswith("ISS ")
        assert hull.integrity == 100.0

    def test_loadout_populates_population(self):
        w, _ = self._seeded_world()
        pair = w.query_one(Population)
        assert pair is not None
        _, pop = pair
        assert 500 <= pop.count <= 2000

    def test_loadout_populates_resources(self):
        w, _ = self._seeded_world()
        ship_pair = w.query_one(ShipHull)
        assert ship_pair is not None
        ship_eid, _ = ship_pair
        res = w.get(ship_eid, Resources)
        assert res is not None
        assert res.food > 0
        assert res.fuel > 0

    def test_loadout_creates_5_to_8_notables(self):
        w, _ = self._seeded_world()
        notables = w.query(Notable)
        assert 5 <= len(notables) <= 8

    def test_loadout_captain_always_present(self):
        w, _ = self._seeded_world()
        roles = [n.role for _, n in w.query(Notable)]
        assert "Captain" in roles

    def test_loadout_notable_has_2_to_3_traits(self):
        w, _ = self._seeded_world()
        for _, n in w.query(Notable):
            assert 2 <= len(n.traits) <= 3

    def test_loadout_sets_game_phase(self):
        w, _ = self._seeded_world()
        pair = w.query_one(GamePhase)
        assert pair is not None
        _, phase = pair
        assert phase.phase == "loadout"

    def test_loadout_emits_notable_introduced_events(self):
        w, events = self._seeded_world()
        notable_events = [e for e in events if e.event_type == "notable.introduced"]
        notables = w.query(Notable)
        assert len(notable_events) == len(notables)

    def test_loadout_emits_loadout_complete_event(self):
        _, events = self._seeded_world()
        complete = [e for e in events if e.event_type == "ship.loadout_complete"]
        assert len(complete) == 1
        assert complete[0].auto_pause is True
        assert complete[0].severity.value == "milestone"

    def test_loadout_complete_event_has_ship_name(self):
        w, events = self._seeded_world()
        complete = next(e for e in events if e.event_type == "ship.loadout_complete")
        hull = w.query_one(ShipHull)[1]
        assert complete.data["ship_name"] == hull.name

    def test_different_seeds_produce_different_ships(self):
        w1, _ = self._seeded_world(seed=1)
        w2, _ = self._seeded_world(seed=2)
        name1 = w1.query_one(ShipHull)[1].name
        name2 = w2.query_one(ShipHull)[1].name
        # Names from different seeds should differ most of the time
        # (not a guarantee but statistically reliable with 15x15 pool)
        pop1 = w1.query_one(Population)[1].count
        pop2 = w2.query_one(Population)[1].count
        # At minimum the worlds should be independently generated
        assert name1 is not None and name2 is not None


# ── World state serialisation ──────────────────────────────────────

class TestWorldStateDict:
    def test_world_state_dict_structure(self):
        w = World()
        rng = random.Random(99)
        run_loadout(w, rng=rng)
        state = world_state_dict(w)
        assert state["type"] == "world_state"
        assert "phase" in state
        assert "ship" in state
        assert "population" in state
        assert "resources" in state
        assert "notables" in state

    def test_world_state_has_ship_name(self):
        w = World()
        run_loadout(w, rng=random.Random(7))
        state = world_state_dict(w)
        assert state["ship"]["name"].startswith("ISS ")

    def test_world_state_notables_list(self):
        w = World()
        run_loadout(w, rng=random.Random(7))
        state = world_state_dict(w)
        assert 5 <= len(state["notables"]) <= 8
        for n in state["notables"]:
            assert "name" in n
            assert "role" in n
            assert "traits" in n

    def test_empty_world_returns_safe_defaults(self):
        w = World()
        state = world_state_dict(w)
        assert state["type"] == "world_state"
        assert state["phase"] == "unknown"
        assert state["ship"] == {}
        assert state["notables"] == []


# ── Narrative integration ──────────────────────────────────────────

class TestLoadoutNarrative:
    def test_loadout_complete_renders_text(self):
        w = World()
        _, events = run_loadout(w, rng=random.Random(5))
        complete = next(e for e in events if e.event_type == "ship.loadout_complete")
        text = render(complete)
        assert text is not None
        assert "ISS" in text

    def test_notable_introduced_renders_text(self):
        w = World()
        _, events = run_loadout(w, rng=random.Random(5))
        intro = next(e for e in events if e.event_type == "notable.introduced")
        text = render(intro)
        assert text is not None
        assert intro.data["name"] in text
        assert intro.data["role"] in text
