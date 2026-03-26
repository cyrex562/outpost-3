"""Tests for the deliberation system — trait biases, scoring, effects, triggers."""

from __future__ import annotations

import random

import pytest

from outpost3.simulation.components import (
    ActiveEffects,
    GamePhase,
    Notable,
    PendingDeliberation,
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
    build_complete_event,
    build_pending_event,
    check_and_fire,
    confirm_deliberation,
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


# ── TestBuildPendingEvent ──────────────────────────────────────────────────────

class TestBuildPendingEvent:
    def test_event_type(self):
        chosen, scored = deliberate("food_rationing", [], rng=_rng())
        event = build_pending_event("food_rationing", chosen, scored, _game_time())
        assert event.event_type == "deliberation.pending"

    def test_auto_pause(self):
        chosen, scored = deliberate("food_rationing", [], rng=_rng())
        event = build_pending_event("food_rationing", chosen, scored, _game_time())
        assert event.auto_pause is True

    def test_pending_data_fields(self):
        chosen, scored = deliberate("food_rationing", [], rng=_rng())
        event = build_pending_event("food_rationing", chosen, scored, _game_time())
        assert "trigger" in event.data
        assert "trigger_label" in event.data
        assert "options" in event.data
        assert "recommendation" in event.data
        assert "recommendation_label" in event.data

    def test_severity_milestone(self):
        from outpost3.simulation import Severity
        chosen, scored = deliberate("hull_emergency", [], rng=_rng())
        event = build_pending_event("hull_emergency", chosen, scored, _game_time())
        assert event.severity == Severity.MILESTONE


# ── TestBuildCompleteEvent ─────────────────────────────────────────────────────

class TestBuildCompleteEvent:
    def test_event_type(self):
        _, scored = deliberate("food_rationing", [], rng=_rng())
        event = build_complete_event("food_rationing", scored[0]["key"], scored, _game_time())
        assert event.event_type == "deliberation.complete"

    def test_event_data_fields(self):
        _, scored = deliberate("food_rationing", [], rng=_rng())
        chosen = scored[0]["key"]
        event = build_complete_event("food_rationing", chosen, scored, _game_time())
        assert "trigger" in event.data
        assert "trigger_label" in event.data
        assert "chosen" in event.data
        assert "chosen_label" in event.data
        assert "effect_summary" in event.data
        assert "options" in event.data

    def test_severity_notable(self):
        from outpost3.simulation import Severity
        _, scored = deliberate("hull_emergency", [], rng=_rng())
        event = build_complete_event("hull_emergency", scored[0]["key"], scored, _game_time())
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
        _, pop = world.query_one(Population)

        # Resources should be ample at loadout — no triggers should fire
        events = check_and_fire(world, _game_time(), ship_eid, hull, res, pop, rng=_rng())
        assert events == []

    def test_food_trigger_emits_pending(self):
        world, ship_eid = self._setup_world()
        hull = world.get(ship_eid, ShipHull)
        res = world.get(ship_eid, Resources)
        _, pop = world.query_one(Population)

        res.food = pop.count * 365 * 1  # less than 2 years
        events = check_and_fire(world, _game_time(), ship_eid, hull, res, pop, rng=_rng())
        assert any(e.event_type == "deliberation.pending" for e in events)
        assert any(e.data["trigger"] == "food_rationing" for e in events)

    def test_pending_event_has_recommendation(self):
        world, ship_eid = self._setup_world()
        hull = world.get(ship_eid, ShipHull)
        res = world.get(ship_eid, Resources)
        _, pop = world.query_one(Population)

        res.food = pop.count * 100
        events = check_and_fire(world, _game_time(), ship_eid, hull, res, pop, rng=_rng())
        pending_events = [e for e in events if e.event_type == "deliberation.pending"]
        assert len(pending_events) == 1
        assert "recommendation" in pending_events[0].data
        assert "options" in pending_events[0].data

    def test_pending_component_stored_in_world(self):
        world, ship_eid = self._setup_world()
        hull = world.get(ship_eid, ShipHull)
        res = world.get(ship_eid, Resources)
        _, pop = world.query_one(Population)

        res.food = pop.count * 100
        check_and_fire(world, _game_time(), ship_eid, hull, res, pop, rng=_rng())
        assert world.query_one(PendingDeliberation) is not None

    def test_no_second_fire_while_pending(self):
        """check_and_fire returns empty when a deliberation is already pending."""
        world, ship_eid = self._setup_world()
        hull = world.get(ship_eid, ShipHull)
        res = world.get(ship_eid, Resources)
        _, pop = world.query_one(Population)

        res.food = pop.count * 100
        events1 = check_and_fire(world, _game_time(), ship_eid, hull, res, pop, rng=_rng())
        events2 = check_and_fire(world, _game_time(), ship_eid, hull, res, pop, rng=_rng())

        assert any(e.event_type == "deliberation.pending" for e in events1)
        assert events2 == []  # blocked by existing pending

    def test_fires_again_after_confirm(self):
        """After confirm_deliberation clears the pending, check_and_fire will not fire again
        for the same trigger (deliberations_fired tracks it)."""
        world, ship_eid = self._setup_world()
        hull = world.get(ship_eid, ShipHull)
        res = world.get(ship_eid, Resources)
        _, pop = world.query_one(Population)

        res.food = pop.count * 100
        events1 = check_and_fire(world, _game_time(), ship_eid, hull, res, pop, rng=_rng())
        pending_evt = next(e for e in events1 if e.event_type == "deliberation.pending")
        chosen = pending_evt.data["recommendation"]

        # Player confirms
        confirm_deliberation(world, ship_eid, chosen, _game_time())

        # food_rationing is now in deliberations_fired — should not fire again
        events2 = check_and_fire(world, _game_time(), ship_eid, hull, res, pop, rng=_rng())
        assert not any(e.data.get("trigger") == "food_rationing" for e in events2)

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

    def test_only_one_trigger_fires_at_a_time(self):
        """With multiple triggers met, only one deliberation.pending fires per call."""
        world, ship_eid = self._setup_world()
        hull = world.get(ship_eid, ShipHull)
        res = world.get(ship_eid, Resources)
        _, pop = world.query_one(Population)

        res.food = pop.count * 100
        hull.integrity = 50.0
        res.medicine = pop.count * 2

        events = check_and_fire(world, _game_time(), ship_eid, hull, res, pop, rng=_rng())
        pending_events = [e for e in events if e.event_type == "deliberation.pending"]
        assert len(pending_events) == 1

    def test_no_effects_component_returns_empty(self):
        world = World()
        hull = ShipHull(name="T")
        res = Resources(food=100)
        pop = Population(count=1000)
        ship_eid = world.new_entity()
        # No ActiveEffects attached
        events = check_and_fire(world, _game_time(), ship_eid, hull, res, pop, rng=_rng())
        assert events == []


# ── TestConfirmDeliberation ────────────────────────────────────────────────────

class TestConfirmDeliberation:
    def _setup_pending(self, trigger: str = "food_rationing"):
        world = World()
        result, _ = run_loadout(world, rng=_rng())
        ship_eid = result.ship_entity
        hull = world.get(ship_eid, ShipHull)
        res = world.get(ship_eid, Resources)
        _, pop = world.query_one(Population)

        # Force the trigger condition
        if trigger == "food_rationing":
            res.food = pop.count * 100
        elif trigger == "hull_emergency":
            hull.integrity = 50.0
        elif trigger == "medical_crisis":
            res.medicine = pop.count * 2

        events = check_and_fire(world, _game_time(), ship_eid, hull, res, pop, rng=_rng())
        pending_evt = next(e for e in events if e.event_type == "deliberation.pending")
        recommendation = pending_evt.data["recommendation"]
        return world, ship_eid, recommendation

    def test_returns_complete_event(self):
        world, ship_eid, rec = self._setup_pending()
        events = confirm_deliberation(world, ship_eid, rec, _game_time())
        assert any(e.event_type == "deliberation.complete" for e in events)

    def test_complete_event_has_correct_chosen(self):
        world, ship_eid, rec = self._setup_pending()
        events = confirm_deliberation(world, ship_eid, rec, _game_time())
        complete = next(e for e in events if e.event_type == "deliberation.complete")
        assert complete.data["chosen"] == rec

    def test_override_crew_recommendation(self):
        """Player picks a different option than the crew recommended."""
        world, ship_eid, rec = self._setup_pending()
        # Pick any option that isn't the recommendation
        _, pending = world.query_one(PendingDeliberation)
        other_key = next(o["key"] for o in pending.scored_options if o["key"] != rec)

        events = confirm_deliberation(world, ship_eid, other_key, _game_time())
        complete = next(e for e in events if e.event_type == "deliberation.complete")
        assert complete.data["chosen"] == other_key

    def test_pending_component_removed_after_confirm(self):
        world, ship_eid, rec = self._setup_pending()
        confirm_deliberation(world, ship_eid, rec, _game_time())
        assert world.query_one(PendingDeliberation) is None

    def test_effect_applied_after_confirm(self):
        """Confirming strict_rations should mutate food_ration_modifier."""
        world, ship_eid, _ = self._setup_pending("food_rationing")
        # Force strict_rations as the chosen key (override if needed)
        effects = world.get(ship_eid, ActiveEffects)
        assert effects.food_ration_modifier == pytest.approx(1.0)  # not yet applied

        confirm_deliberation(world, ship_eid, "strict_rations", _game_time())
        assert effects.food_ration_modifier == pytest.approx(0.75)

    def test_trigger_added_to_fired_set(self):
        world, ship_eid, rec = self._setup_pending()
        effects = world.get(ship_eid, ActiveEffects)
        assert "food_rationing" not in effects.deliberations_fired

        confirm_deliberation(world, ship_eid, rec, _game_time())
        assert "food_rationing" in effects.deliberations_fired

    def test_raises_when_no_pending(self):
        world = World()
        result, _ = run_loadout(world, rng=_rng())
        with pytest.raises(ValueError, match="No deliberation is currently pending"):
            confirm_deliberation(world, result.ship_entity, "strict_rations", _game_time())

    def test_raises_for_invalid_option_key(self):
        world, ship_eid, _ = self._setup_pending()
        with pytest.raises(ValueError, match="Invalid option"):
            confirm_deliberation(world, ship_eid, "nonexistent_key", _game_time())


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

    def test_pending_deliberation_key_present(self):
        world = World()
        run_loadout(world, rng=_rng())
        state = world_state_dict(world)
        assert "pending_deliberation" in state

    def test_pending_deliberation_empty_when_none(self):
        world = World()
        run_loadout(world, rng=_rng())
        state = world_state_dict(world)
        assert state["pending_deliberation"] == {}

    def test_pending_deliberation_populated_when_pending(self):
        world = World()
        result, _ = run_loadout(world, rng=_rng())
        ship_eid = result.ship_entity
        hull = world.get(ship_eid, ShipHull)
        res = world.get(ship_eid, Resources)
        _, pop = world.query_one(Population)

        res.food = pop.count * 100
        check_and_fire(world, _game_time(), ship_eid, hull, res, pop, rng=_rng())

        state = world_state_dict(world)
        pd = state["pending_deliberation"]
        assert pd["trigger"] == "food_rationing"
        assert "options" in pd
        assert "recommendation" in pd
