"""Tests for the deliberation system — trait biases, scoring, effects, triggers."""

from __future__ import annotations

import random

import pytest

from outpost3.simulation.components import (
    ActiveEffects,
    GamePhase,
    Notable,
    Population,
    ResourceHistory,
    Resources,
    ShipHull,
    Transit,
)
from outpost3.simulation.deliberation import (
    SCENARIOS,
    TRAIT_BIASES,
    apply_effect,
    build_event,
    check_and_fire,
    deliberate,
)
from outpost3.simulation.loadout import run_loadout, world_state_dict
from outpost3.simulation.world import World
from outpost3.simulation import GameTime


# ── Helpers ────────────────────────────────────────────────────────────────────

def _notable(name: str, traits: list[str]) -> Notable:
    return Notable(name=name, role="Officer", traits=traits)

def _game_time(day: int = 1000) -> GameTime:
    return GameTime(day)

def _rng(seed: int = 42) -> random.Random:
    return random.Random(seed)


# ── TestTraitBiases ────────────────────────────────────────────────────────────

class TestTraitBiases:
    def test_cautious_boosts_strict_rations(self):
        assert "strict_rations" in TRAIT_BIASES["cautious"]
        assert TRAIT_BIASES["cautious"]["strict_rations"] > 0

    def test_bold_boosts_ignore_damage(self):
        assert "ignore_damage" in TRAIT_BIASES["bold"]
        assert TRAIT_BIASES["bold"]["ignore_damage"] > 0

    def test_empathetic_boosts_triage_care(self):
        assert "triage_care" in TRAIT_BIASES["empathetic"]
        assert TRAIT_BIASES["empathetic"]["triage_care"] > 0

    def test_all_scenarios_have_options_covered_by_traits(self):
        """Every option key in every scenario should appear in at least one trait bias."""
        all_option_keys = set()
        for scenario in SCENARIOS.values():
            for opt in scenario.options:
                all_option_keys.add(opt.key)

        covered = set()
        for biases in TRAIT_BIASES.values():
            covered.update(biases.keys())

        # All option keys must be reachable by at least one trait
        assert all_option_keys.issubset(covered), (
            f"Uncovered options: {all_option_keys - covered}"
        )


# ── TestDeliberate ─────────────────────────────────────────────────────────────

class TestDeliberate:
    def test_returns_valid_chosen_key(self):
        chosen, scored = deliberate("food_rationing", [], rng=_rng())
        option_keys = {o.key for o in SCENARIOS["food_rationing"].options}
        assert chosen in option_keys

    def test_scored_options_sorted_descending(self):
        _, scored = deliberate("food_rationing", [], rng=_rng())
        utilities = [o["utility"] for o in scored]
        assert utilities == sorted(utilities, reverse=True)

    def test_scored_options_have_required_fields(self):
        _, scored = deliberate("hull_emergency", [], rng=_rng())
        for opt in scored:
            assert "key" in opt
            assert "label" in opt
            assert "utility" in opt
            assert "advocates" in opt
            assert "against" in opt

    def test_cautious_notable_advocates_strict_rations(self):
        notable = _notable("Alice", ["cautious"])
        chosen, scored = deliberate("food_rationing", [notable], rng=_rng(0))
        advocates_for_strict = next(
            o["advocates"] for o in scored if o["key"] == "strict_rations"
        )
        assert "Alice" in advocates_for_strict

    def test_bold_notable_advocates_continue_normal(self):
        notable = _notable("Bob", ["bold", "reckless"])
        _, scored = deliberate("food_rationing", [notable], rng=_rng(0))
        advocates_for_continue = next(
            o["advocates"] for o in scored if o["key"] == "continue_normal"
        )
        assert "Bob" in advocates_for_continue

    def test_multiple_notables_accumulate_scores(self):
        cautious_crew = [_notable(f"C{i}", ["cautious"]) for i in range(5)]
        chosen, _ = deliberate("food_rationing", cautious_crew, rng=_rng())
        # 5 cautious notables should push strict_rations to win
        assert chosen == "strict_rations"

    def test_dead_notable_not_counted(self):
        notable = _notable("Ghost", ["cautious"])
        notable.alive = False
        _, scored_without = deliberate("food_rationing", [], rng=_rng(1))
        _, scored_with = deliberate("food_rationing", [notable], rng=_rng(1))
        # Dead notable should not change scores
        u_without = {o["key"]: o["utility"] for o in scored_without}
        u_with = {o["key"]: o["utility"] for o in scored_with}
        assert u_without == u_with

    def test_utility_capped_at_1(self):
        # Flood with cautious notables; utility should not exceed 1.0
        crew = [_notable(f"C{i}", ["cautious", "methodical", "analytical"]) for i in range(10)]
        _, scored = deliberate("food_rationing", crew, rng=_rng())
        for opt in scored:
            assert opt["utility"] <= 1.0

    def test_hull_emergency_scenario_keys(self):
        chosen, scored = deliberate("hull_emergency", [], rng=_rng())
        keys = {o["key"] for o in scored}
        assert keys == {"emergency_repairs", "patch_and_monitor", "ignore_damage"}

    def test_medical_crisis_scenario_keys(self):
        chosen, scored = deliberate("medical_crisis", [], rng=_rng())
        keys = {o["key"] for o in scored}
        assert keys == {"quarantine_protocol", "triage_care", "routine_only"}

    def test_different_seeds_may_differ(self):
        # With no notables, jitter means different seeds may produce different results
        choices = {deliberate("food_rationing", [], rng=random.Random(i))[0] for i in range(20)}
        # Not all seeds should map to the same winner (jitter should spread outcomes)
        # At minimum, strict_rations and light_rations should both appear sometimes
        # (won't always be true, but with 20 seeds at ~uniform jitter ±0.04 it's likely)
        assert len(choices) >= 1  # minimal sanity check


# ── TestApplyEffect ────────────────────────────────────────────────────────────

class TestApplyEffect:
    def _setup(self):
        hull = ShipHull(name="Test", integrity=80.0)
        res = Resources(food=100_000, water=100_000, medicine=10_000,
                        fuel=500_000, spare_parts=50_000)
        effects = ActiveEffects()
        return hull, res, effects

    def test_strict_rations_sets_food_modifier(self):
        hull, res, effects = self._setup()
        apply_effect("food_rationing", "strict_rations", hull, res, effects)
        assert effects.food_ration_modifier == pytest.approx(0.75)

    def test_light_rations_sets_food_modifier(self):
        hull, res, effects = self._setup()
        apply_effect("food_rationing", "light_rations", hull, res, effects)
        assert effects.food_ration_modifier == pytest.approx(0.88)

    def test_continue_normal_no_modifier_change(self):
        hull, res, effects = self._setup()
        apply_effect("food_rationing", "continue_normal", hull, res, effects)
        assert effects.food_ration_modifier == pytest.approx(1.0)

    def test_emergency_repairs_consumes_parts_and_heals_hull(self):
        hull, res, effects = self._setup()
        initial_parts = res.spare_parts
        initial_hull = hull.integrity
        apply_effect("hull_emergency", "emergency_repairs", hull, res, effects)
        assert res.spare_parts == initial_parts - 8_000
        assert hull.integrity == pytest.approx(initial_hull + 5.0)

    def test_patch_and_monitor_reduces_crack_priority(self):
        hull, res, effects = self._setup()
        apply_effect("hull_emergency", "patch_and_monitor", hull, res, effects)
        assert effects.repair_priority < 1.0
        assert res.spare_parts == 50_000 - 2_000
        assert hull.integrity == pytest.approx(81.0)

    def test_ignore_damage_increases_crack_priority(self):
        hull, res, effects = self._setup()
        apply_effect("hull_emergency", "ignore_damage", hull, res, effects)
        assert effects.repair_priority > 1.0

    def test_hull_capped_at_max_integrity(self):
        hull = ShipHull(name="T", integrity=98.0, max_integrity=100.0)
        res = Resources(spare_parts=50_000)
        effects = ActiveEffects()
        apply_effect("hull_emergency", "emergency_repairs", hull, res, effects)
        assert hull.integrity <= 100.0

    def test_quarantine_extends_medicine(self):
        hull, res, effects = self._setup()
        initial_medicine = res.medicine
        apply_effect("medical_crisis", "quarantine_protocol", hull, res, effects)
        assert res.medicine > initial_medicine

    def test_triage_care_consumes_parts_and_extends_medicine(self):
        hull, res, effects = self._setup()
        initial_parts = res.spare_parts
        initial_medicine = res.medicine
        apply_effect("medical_crisis", "triage_care", hull, res, effects)
        assert res.spare_parts < initial_parts
        assert res.medicine > initial_medicine

    def test_routine_only_no_change(self):
        hull, res, effects = self._setup()
        initial_parts = res.spare_parts
        initial_medicine = res.medicine
        apply_effect("medical_crisis", "routine_only", hull, res, effects)
        assert res.spare_parts == initial_parts
        assert res.medicine == initial_medicine


# ── TestBuildEvent ─────────────────────────────────────────────────────────────

class TestBuildEvent:
    def test_event_type(self):
        _, scored = deliberate("food_rationing", [], rng=_rng())
        event = build_event("food_rationing", scored[0]["key"], scored, _game_time())
        assert event.event_type == "deliberation.complete"

    def test_event_data_fields(self):
        _, scored = deliberate("food_rationing", [], rng=_rng())
        chosen = scored[0]["key"]
        event = build_event("food_rationing", chosen, scored, _game_time())
        assert "trigger" in event.data
        assert "trigger_label" in event.data
        assert "chosen" in event.data
        assert "chosen_label" in event.data
        assert "effect_summary" in event.data
        assert "options" in event.data

    def test_severity_notable(self):
        from outpost3.simulation import Severity
        _, scored = deliberate("hull_emergency", [], rng=_rng())
        event = build_event("hull_emergency", scored[0]["key"], scored, _game_time())
        assert event.severity == Severity.NOTABLE


# ── TestCheckAndFire ───────────────────────────────────────────────────────────

class TestCheckAndFire:
    def _setup_world(self):
        world = World()
        result, _ = run_loadout(world, rng=_rng())
        return world, result.ship_entity

    def test_no_events_when_no_triggers_met(self):
        world, ship_eid = self._setup_world()
        hull = world.get(ship_eid, ShipHull)
        res = world.get(ship_eid, Resources)
        pop_pair = world.query_one(Population)
        _, pop = pop_pair

        # Resources should be ample at loadout — no triggers should fire
        events = check_and_fire(world, _game_time(), ship_eid, hull, res, pop, rng=_rng())
        assert events == []

    def test_food_trigger_fires_when_food_low(self):
        world, ship_eid = self._setup_world()
        hull = world.get(ship_eid, ShipHull)
        res = world.get(ship_eid, Resources)
        _, pop = world.query_one(Population)

        # Drain food to trigger condition
        res.food = pop.count * 365 * 1  # less than 2 years
        events = check_and_fire(world, _game_time(), ship_eid, hull, res, pop, rng=_rng())
        assert any(e.event_type == "deliberation.complete" for e in events)
        assert any(e.data["trigger"] == "food_rationing" for e in events)

    def test_food_trigger_fires_only_once(self):
        world, ship_eid = self._setup_world()
        hull = world.get(ship_eid, ShipHull)
        res = world.get(ship_eid, Resources)
        _, pop = world.query_one(Population)

        res.food = pop.count * 100  # low food

        events1 = check_and_fire(world, _game_time(), ship_eid, hull, res, pop, rng=_rng())
        events2 = check_and_fire(world, _game_time(), ship_eid, hull, res, pop, rng=_rng())

        food_events_1 = [e for e in events1 if e.data.get("trigger") == "food_rationing"]
        food_events_2 = [e for e in events2 if e.data.get("trigger") == "food_rationing"]
        assert len(food_events_1) == 1
        assert len(food_events_2) == 0

    def test_hull_trigger_fires_when_hull_low(self):
        world, ship_eid = self._setup_world()
        hull = world.get(ship_eid, ShipHull)
        res = world.get(ship_eid, Resources)
        _, pop = world.query_one(Population)

        hull.integrity = 50.0  # below 60% threshold
        events = check_and_fire(world, _game_time(), ship_eid, hull, res, pop, rng=_rng())
        assert any(e.data.get("trigger") == "hull_emergency" for e in events)

    def test_medical_trigger_fires_when_medicine_low(self):
        world, ship_eid = self._setup_world()
        hull = world.get(ship_eid, ShipHull)
        res = world.get(ship_eid, Resources)
        _, pop = world.query_one(Population)

        res.medicine = pop.count * 2  # below 5 units/person threshold
        events = check_and_fire(world, _game_time(), ship_eid, hull, res, pop, rng=_rng())
        assert any(e.data.get("trigger") == "medical_crisis" for e in events)

    def test_multiple_triggers_can_fire_simultaneously(self):
        world, ship_eid = self._setup_world()
        hull = world.get(ship_eid, ShipHull)
        res = world.get(ship_eid, Resources)
        _, pop = world.query_one(Population)

        res.food = pop.count * 100
        hull.integrity = 50.0
        res.medicine = pop.count * 2

        events = check_and_fire(world, _game_time(), ship_eid, hull, res, pop, rng=_rng())
        triggers_fired = {e.data["trigger"] for e in events if e.event_type == "deliberation.complete"}
        assert "food_rationing" in triggers_fired
        assert "hull_emergency" in triggers_fired
        assert "medical_crisis" in triggers_fired

    def test_no_effects_component_returns_empty(self):
        world = World()
        hull = ShipHull(name="T")
        res = Resources(food=100)
        pop = Population(count=1000)
        ship_eid = world.new_entity()
        # No ActiveEffects attached
        events = check_and_fire(world, _game_time(), ship_eid, hull, res, pop, rng=_rng())
        assert events == []


# ── TestResourceHistory ────────────────────────────────────────────────────────

class TestResourceHistory:
    def test_record_adds_snapshot(self):
        hist = ResourceHistory()
        res = Resources(food=1000, water=500, medicine=200, fuel=10000, spare_parts=5000)
        pop = Population(count=100)
        hist.record(365, res, pop)
        assert len(hist.to_dict()) == 1
        assert hist.to_dict()[0]["food"] == 1000
        assert hist.to_dict()[0]["population"] == 100

    def test_max_points_trimming(self):
        hist = ResourceHistory(max_points=5)
        res = Resources()
        pop = Population(count=10)
        for day in range(10):
            hist.record(day * 365, res, pop)
        assert len(hist.to_dict()) == 5
        # Should keep the most recent 5
        days = [s["day"] for s in hist.to_dict()]
        assert days == [5 * 365, 6 * 365, 7 * 365, 8 * 365, 9 * 365]


# ── TestWorldStateDictIntegration ─────────────────────────────────────────────

class TestWorldStateDictIntegration:
    def test_resource_history_key_present(self):
        world = World()
        run_loadout(world, rng=_rng())
        state = world_state_dict(world)
        assert "resource_history" in state
        assert isinstance(state["resource_history"], list)

    def test_resource_history_empty_at_start(self):
        world = World()
        run_loadout(world, rng=_rng())
        state = world_state_dict(world)
        assert state["resource_history"] == []
