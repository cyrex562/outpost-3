"""Crew deliberation system — Named Notables advocate for options via trait biases.

When a deliberation trigger fires, each Notable's personality traits bias the
utility scores of available options.  The highest-scoring option wins and its
effect is applied to the world.  The full record (options, scores, advocates,
chosen action) is emitted as a deliberation.complete event.
"""

from __future__ import annotations

import random
from dataclasses import dataclass
from typing import Any

from ..simulation import GameEvent, GameTime, Severity
from .components import ActiveEffects, Notable, Population, Resources, ShipHull
from .world import World

# ── Trait → option-key bias weights ───────────────────────────────────────────
# Maps personality trait → {option_key → utility_bonus}.
# Options not in the dict are unaffected by that trait.

TRAIT_BIASES: dict[str, dict[str, float]] = {
    "cautious":     {"strict_rations": 0.22, "emergency_repairs": 0.18, "quarantine_protocol": 0.18},
    "pragmatic":    {"light_rations": 0.18,  "patch_and_monitor": 0.22, "triage_care": 0.15},
    "bold":         {"continue_normal": 0.22, "ignore_damage": 0.20,    "routine_only": 0.18},
    "analytical":   {"strict_rations": 0.12, "patch_and_monitor": 0.18, "quarantine_protocol": 0.14},
    "empathetic":   {"light_rations": 0.18,  "triage_care": 0.25},
    "decisive":     {"continue_normal": 0.12, "patch_and_monitor": 0.12, "routine_only": 0.12},
    "idealistic":   {"light_rations": 0.12,  "triage_care": 0.20},
    "skeptical":    {"strict_rations": 0.18, "emergency_repairs": 0.12},
    "methodical":   {"strict_rations": 0.12, "emergency_repairs": 0.12, "quarantine_protocol": 0.16},
    "innovative":   {"light_rations": 0.12,  "patch_and_monitor": 0.16},
    "resilient":    {"continue_normal": 0.16, "ignore_damage": 0.12},
    "stoic":        {"continue_normal": 0.14, "routine_only": 0.14},
    "compassionate":{"light_rations": 0.14,  "triage_care": 0.20},
    "ambitious":    {"emergency_repairs": 0.10, "strict_rations": 0.10},
    "reckless":     {"ignore_damage": 0.20,  "continue_normal": 0.16},
    "tenacious":    {"strict_rations": 0.10, "emergency_repairs": 0.14},
    "diplomatic":   {"light_rations": 0.14,  "triage_care": 0.14},
    "traditional":  {"strict_rations": 0.10, "routine_only": 0.10},
    "creative":     {"patch_and_monitor": 0.12, "triage_care": 0.12},
    "charismatic":  {"continue_normal": 0.10, "light_rations": 0.10},
    "introspective":{"quarantine_protocol": 0.12, "patch_and_monitor": 0.10},
}


# ── Scenario definitions ───────────────────────────────────────────────────────

@dataclass(frozen=True)
class _Option:
    key: str
    label: str
    base_utility: float


@dataclass(frozen=True)
class _Scenario:
    trigger_label: str
    options: tuple[_Option, ...]
    # Human-readable summary of each option's outcome (keyed by option.key)
    effect_summaries: dict[str, str]


SCENARIOS: dict[str, _Scenario] = {
    "food_rationing": _Scenario(
        trigger_label="Food supply running low",
        options=(
            _Option("strict_rations",  "Strict rationing protocol",      0.68),
            _Option("light_rations",   "Moderate consumption reduction",  0.58),
            _Option("continue_normal", "Maintain current allocation",     0.28),
        ),
        effect_summaries={
            "strict_rations":  "Food consumption reduced by 25% — reserves extended.",
            "light_rations":   "Food consumption reduced by 12% — morale preserved.",
            "continue_normal": "Allocation unchanged — rationing deferred.",
        },
    ),
    "hull_emergency": _Scenario(
        trigger_label="Hull integrity critical",
        options=(
            _Option("emergency_repairs", "Emergency repair operations",    0.76),
            _Option("patch_and_monitor", "Targeted patching + monitoring", 0.54),
            _Option("ignore_damage",     "Continue without major repairs", 0.22),
        ),
        effect_summaries={
            "emergency_repairs": "8,000 parts consumed — hull integrity restored by 5%.",
            "patch_and_monitor": "2,000 parts consumed — hull braced, crack rate reduced.",
            "ignore_damage":     "No repairs — hull degradation rate increases.",
        },
    ),
    "medical_crisis": _Scenario(
        trigger_label="Medical supplies critically low",
        options=(
            _Option("quarantine_protocol", "Quarantine and isolation",    0.62),
            _Option("triage_care",         "Triage-based care rationing", 0.66),
            _Option("routine_only",        "Routine care only",           0.34),
        ),
        effect_summaries={
            "quarantine_protocol": "Isolation reduces exposure — medicine reserves extended.",
            "triage_care":         "Facility upgrades (300 parts) improve medicine efficiency.",
            "routine_only":        "Standard protocols maintained — no additional intervention.",
        },
    ),
}

# ── Trigger conditions ─────────────────────────────────────────────────────────
# Each returns True when the scenario should fire.

def _should_trigger_food_rationing(res: Resources, pop: Population) -> bool:
    return pop.count > 0 and res.food < pop.count * 365 * 2

def _should_trigger_hull_emergency(hull: ShipHull) -> bool:
    return hull.integrity < 60.0

def _should_trigger_medical_crisis(res: Resources, pop: Population) -> bool:
    return pop.count > 0 and res.medicine < pop.count * 5


TRIGGER_CHECKS = {
    "food_rationing": _should_trigger_food_rationing,
    "hull_emergency": _should_trigger_hull_emergency,
    "medical_crisis": _should_trigger_medical_crisis,
}


# ── Core deliberation function ─────────────────────────────────────────────────

def deliberate(
    trigger: str,
    notables: list[Notable],
    *,
    rng: random.Random | None = None,
) -> tuple[str, list[dict]]:
    """Score options with Notable trait biases; return (chosen_key, scored_options).

    scored_options is a list of dicts, highest-utility first:
        {key, label, utility, advocates: [str], against: [str]}
    """
    _rng = rng or random.Random()
    scenario = SCENARIOS[trigger]
    options = scenario.options

    # Base utility + small jitter to prevent ties
    scores: dict[str, float] = {
        opt.key: opt.base_utility + _rng.uniform(-0.04, 0.04)
        for opt in options
    }
    advocates: dict[str, list[str]] = {opt.key: [] for opt in options}
    against: dict[str, list[str]] = {opt.key: [] for opt in options}

    for notable in notables:
        if not notable.alive:
            continue

        # Sum all trait-bias contributions for each option
        contributions: dict[str, float] = {opt.key: 0.0 for opt in options}
        for trait in notable.traits:
            for opt_key, delta in TRAIT_BIASES.get(trait, {}).items():
                if opt_key in contributions:
                    contributions[opt_key] += delta

        # Apply to global scores
        for opt_key, delta in contributions.items():
            scores[opt_key] = min(1.0, scores[opt_key] + delta)

        # Classify notable: advocate for most-boosted, against least-boosted
        best_key = max(contributions, key=lambda k: contributions[k])
        worst_key = min(contributions, key=lambda k: contributions[k])
        if contributions[best_key] > 0:
            advocates[best_key].append(notable.name)
            if worst_key != best_key:
                against[worst_key].append(notable.name)

    chosen_key = max(scores, key=lambda k: scores[k])

    scored = sorted(
        [
            {
                "key": opt.key,
                "label": opt.label,
                "utility": round(min(1.0, scores[opt.key]), 3),
                "advocates": advocates[opt.key],
                "against": against[opt.key],
            }
            for opt in options
        ],
        key=lambda d: d["utility"],
        reverse=True,
    )
    return chosen_key, scored


# ── Effect application ─────────────────────────────────────────────────────────

def apply_effect(
    trigger: str,
    chosen_key: str,
    hull: ShipHull,
    res: Resources,
    effects: ActiveEffects,
) -> None:
    """Mutate world state to reflect the chosen deliberation outcome."""
    if trigger == "food_rationing":
        if chosen_key == "strict_rations":
            effects.food_ration_modifier = 0.75
        elif chosen_key == "light_rations":
            effects.food_ration_modifier = 0.88

    elif trigger == "hull_emergency":
        if chosen_key == "emergency_repairs":
            res.spare_parts = max(0, res.spare_parts - 8_000)
            hull.integrity = min(hull.max_integrity, hull.integrity + 5.0)
        elif chosen_key == "patch_and_monitor":
            res.spare_parts = max(0, res.spare_parts - 2_000)
            hull.integrity = min(hull.max_integrity, hull.integrity + 1.0)
            effects.repair_priority = 0.65
        elif chosen_key == "ignore_damage":
            effects.repair_priority = 1.35

    elif trigger == "medical_crisis":
        if chosen_key == "quarantine_protocol":
            # Quarantine extends medicine reserves
            res.medicine += res.medicine // 3  # recoup ~33% through rationing
        elif chosen_key == "triage_care":
            res.spare_parts = max(0, res.spare_parts - 300)
            res.medicine += res.medicine // 5  # smaller gain from facility upgrade


# ── Event builder ──────────────────────────────────────────────────────────────

def build_event(
    trigger: str,
    chosen_key: str,
    scored_options: list[dict],
    game_time: GameTime,
) -> GameEvent:
    """Construct a deliberation.complete GameEvent with the full record."""
    scenario = SCENARIOS[trigger]
    chosen_label = next(o["label"] for o in scored_options if o["key"] == chosen_key)
    effect_summary = scenario.effect_summaries[chosen_key]

    return GameEvent(
        event_type="deliberation.complete",
        severity=Severity.NOTABLE,
        game_time=game_time,
        data={
            "trigger": trigger,
            "trigger_label": scenario.trigger_label,
            "options": scored_options,
            "chosen": chosen_key,
            "chosen_label": chosen_label,
            "effect_summary": effect_summary,
        },
    )


# ── World-level orchestrator ───────────────────────────────────────────────────

def check_and_fire(
    world: World,
    game_time: GameTime,
    ship_eid: int,
    hull: ShipHull,
    res: Resources,
    pop: Population,
    *,
    rng: random.Random | None = None,
) -> list[GameEvent]:
    """Check all deliberation triggers; fire any that haven't been resolved yet.

    Should be called once per year boundary (or batch) during transit.
    """
    effects = world.get(ship_eid, ActiveEffects)
    if effects is None:
        return []

    notables = [n for _, n in world.query(Notable) if n.alive]
    events: list[GameEvent] = []

    trigger_args: dict[str, Any] = {
        "food_rationing": (res, pop),
        "hull_emergency":  (hull,),
        "medical_crisis":  (res, pop),
    }

    for trigger, args in trigger_args.items():
        if trigger in effects.deliberations_fired:
            continue
        check_fn = TRIGGER_CHECKS[trigger]
        if not check_fn(*args):
            continue

        # Fire deliberation
        chosen_key, scored = deliberate(trigger, notables, rng=rng)
        apply_effect(trigger, chosen_key, hull, res, effects)
        effects.deliberations_fired.add(trigger)
        events.append(build_event(trigger, chosen_key, scored, game_time))

    return events
