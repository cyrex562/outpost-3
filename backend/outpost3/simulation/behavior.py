"""BehaviorSystem — crew decision-making with deliberation records.

Hybrid behavior tree + weighted utility scoring. When conditions trigger,
the system generates deliberation records showing the full decision process:
trigger, options evaluated, Notable advocates, chosen action.
"""

from __future__ import annotations

import random
from dataclasses import dataclass, field
from typing import Any

from . import GameEvent, GameTime, Severity
from .ecs import World
from .systems import System, Phase
from .components import (
    FactionPhase, Resources, Population, ShipSystems, Notable,
)


# ── Deliberation data structures ─────────────────────────────────

@dataclass
class Option:
    """A single option in a deliberation."""
    action: str
    description: str
    score: float = 0.0
    pros: list[str] = field(default_factory=list)
    cons: list[str] = field(default_factory=list)
    advocate: str | None = None  # Notable name who advocates
    advocate_trait: str | None = None  # Which trait drives advocacy

    def to_dict(self) -> dict[str, Any]:
        return {
            "action": self.action,
            "description": self.description,
            "score": round(self.score, 2),
            "pros": self.pros,
            "cons": self.cons,
            "advocate": self.advocate,
            "advocate_trait": self.advocate_trait,
        }


@dataclass
class Deliberation:
    """A full deliberation record: trigger, options, decision."""
    trigger: str
    trigger_detail: str
    options: list[Option] = field(default_factory=list)
    chosen: Option | None = None
    rejected: list[Option] = field(default_factory=list)

    def to_dict(self) -> dict[str, Any]:
        return {
            "trigger": self.trigger,
            "trigger_detail": self.trigger_detail,
            "options": [o.to_dict() for o in self.options],
            "chosen": self.chosen.to_dict() if self.chosen else None,
            "rejected": [o.to_dict() for o in self.rejected],
        }


# ── Trait → preference mapping ───────────────────────────────────

TRAIT_PREFERENCES: dict[str, dict[str, float]] = {
    # trait → action_keyword → score modifier
    "cautious": {"reduce": 0.15, "conserve": 0.15, "ration": 0.10, "slow": 0.10},
    "bold": {"divert": 0.15, "push": 0.10, "accelerate": 0.10, "risk": 0.10},
    "crew_welfare": {"ration": -0.10, "morale": 0.15, "medical": 0.10, "rest": 0.10},
    "mission_focus": {"ration": 0.10, "efficiency": 0.15, "push": 0.10, "speed": 0.10},
    "resourceful": {"improvise": 0.15, "repurpose": 0.15, "divert": 0.10},
    "conservative": {"standard": 0.15, "protocol": 0.10, "conserve": 0.10},
    "optimistic": {"push": 0.10, "risk": 0.05, "opportunity": 0.10},
    "pragmatic": {"reduce": 0.10, "conserve": 0.10, "standard": 0.05},
}


def _find_advocate(
    notables: list[Notable],
    action_keywords: list[str],
) -> tuple[str | None, str | None]:
    """Find the Notable most likely to advocate for an action based on traits."""
    best_name = None
    best_trait = None
    best_score = 0.0

    for notable in notables:
        if not notable.alive:
            continue
        for trait in notable.traits:
            prefs = TRAIT_PREFERENCES.get(trait, {})
            for keyword in action_keywords:
                score = prefs.get(keyword, 0.0)
                if score > best_score:
                    best_score = score
                    best_name = notable.name
                    best_trait = trait

    return best_name, best_trait


# ── Condition evaluators ─────────────────────────────────────────

def _check_food_low(resources: Resources, population: Population) -> Deliberation | None:
    """Trigger when food stores are below 30% of original capacity."""
    # Estimate: food per person per year ~ 1.5 units
    days_of_food = resources.food / max(population.total * 0.004, 1)  # ~0.004/person/day
    if days_of_food > 180:  # more than ~6 months supply
        return None

    pct = min(100, max(0, int(days_of_food / 365 * 100)))
    return Deliberation(
        trigger="food_low",
        trigger_detail=f"Food stores at estimated {pct}% annual supply ({int(days_of_food)} days remaining)",
        options=[
            Option(
                action="reduce_rations",
                description="Reduce daily rations to 80%",
                pros=["Extends food supply by ~25%"],
                cons=["Morale impact: -10 to -15"],
            ),
            Option(
                action="expand_hydroponics",
                description="Expand hydroponics production using spare parts",
                pros=["Increases food regeneration", "Long-term solution"],
                cons=["Costs spare parts", "Takes 30+ days to take effect"],
            ),
            Option(
                action="maintain_rations",
                description="Maintain current rations, hope for the best",
                pros=["No immediate morale impact"],
                cons=["Risk of running out completely"],
            ),
        ],
    )


def _check_hull_damage(ship_systems: ShipSystems) -> Deliberation | None:
    """Trigger when hull integrity drops below 70%."""
    if ship_systems.hull_integrity > 0.70:
        return None

    pct = int(ship_systems.hull_integrity * 100)
    return Deliberation(
        trigger="hull_damage",
        trigger_detail=f"Hull integrity at {pct}% — structural risk increasing",
        options=[
            Option(
                action="emergency_repair",
                description="Divert engineering crew to emergency hull repairs",
                pros=["Restores hull integrity", "Prevents further degradation"],
                cons=["Costs spare parts", "Other systems unattended"],
            ),
            Option(
                action="seal_damaged_sections",
                description="Seal off damaged sections, reduce habitable area",
                pros=["Minimal resource cost", "Quick to implement"],
                cons=["Reduces living space", "Morale impact"],
            ),
            Option(
                action="slow_repair",
                description="Schedule gradual repairs during routine maintenance",
                pros=["No disruption to other work"],
                cons=["Slow recovery", "Risk of further damage"],
            ),
        ],
    )


def _check_morale_low(population: Population) -> Deliberation | None:
    """Trigger when morale drops below 0.40."""
    if population.morale > 0.40:
        return None

    pct = int(population.morale * 100)
    return Deliberation(
        trigger="morale_low",
        trigger_detail=f"Crew morale at {pct}% — risk of unrest",
        options=[
            Option(
                action="community_event",
                description="Organize a community celebration or rest day",
                pros=["Significant morale boost", "Builds community"],
                cons=["One day of reduced productivity"],
            ),
            Option(
                action="improve_conditions",
                description="Allocate resources to improve living conditions",
                pros=["Sustained morale improvement"],
                cons=["Costs materials and spare parts"],
            ),
            Option(
                action="address_grievances",
                description="Hold town hall to address crew concerns directly",
                pros=["Targeted solution", "No resource cost"],
                cons=["May surface deeper issues", "Unpredictable outcome"],
            ),
        ],
    )


def _check_water_low(resources: Resources, population: Population) -> Deliberation | None:
    """Trigger when water stores are critically low."""
    days_of_water = resources.water / max(population.total * 0.003, 1)
    if days_of_water > 180:
        return None

    pct = min(100, max(0, int(days_of_water / 365 * 100)))
    return Deliberation(
        trigger="water_low",
        trigger_detail=f"Water reserves at {pct}% annual supply ({int(days_of_water)} days remaining)",
        options=[
            Option(
                action="water_rationing",
                description="Implement strict water rationing",
                pros=["Extends supply by ~30%"],
                cons=["Morale impact", "Hygiene concerns"],
            ),
            Option(
                action="reclamation_upgrade",
                description="Upgrade water reclamation systems using spare parts",
                pros=["Reduces net water loss", "Permanent improvement"],
                cons=["Costs spare parts", "Engineering time required"],
            ),
        ],
    )


def _check_fuel_low(resources: Resources) -> Deliberation | None:
    """Trigger when fuel is dangerously low."""
    if resources.fuel > 500:
        return None

    return Deliberation(
        trigger="fuel_low",
        trigger_detail=f"Fuel reserves at {int(resources.fuel)} units — deceleration margin threatened",
        options=[
            Option(
                action="reduce_speed",
                description="Reduce cruise velocity to conserve fuel",
                pros=["Extends fuel for deceleration"],
                cons=["Adds transit time", "Crew uncertainty"],
            ),
            Option(
                action="power_conservation",
                description="Switch non-essential systems to low-power mode",
                pros=["Reduces fuel consumption 10-15%"],
                cons=["Reduced sensor range", "Minor comfort loss"],
            ),
            Option(
                action="maintain_course",
                description="Maintain current velocity and hope reserves suffice",
                pros=["No disruption"],
                cons=["Risk of insufficient deceleration fuel"],
            ),
        ],
    )


def _check_engine_critical(ship_systems: ShipSystems) -> Deliberation | None:
    """Trigger when engines are severely damaged."""
    if ship_systems.engines > 0.50:
        return None

    pct = int(ship_systems.engines * 100)
    return Deliberation(
        trigger="engine_critical",
        trigger_detail=f"Engine systems at {pct}% — propulsion compromised",
        options=[
            Option(
                action="engine_overhaul",
                description="Major engine overhaul using spare parts reserves",
                pros=["Significant engine restoration", "Prevents total failure"],
                cons=["Heavy spare parts cost", "Weeks of reduced thrust"],
            ),
            Option(
                action="redistribute_power",
                description="Redistribute power from other systems to engines",
                pros=["Stabilizes engine output"],
                cons=["Weakens sensors or life support temporarily"],
            ),
        ],
    )


def _check_life_support_critical(ship_systems: ShipSystems) -> Deliberation | None:
    """Trigger when life support is dangerously degraded."""
    if ship_systems.life_support > 0.55:
        return None

    pct = int(ship_systems.life_support * 100)
    return Deliberation(
        trigger="life_support_critical",
        trigger_detail=f"Life support at {pct}% — atmosphere quality declining",
        options=[
            Option(
                action="life_support_repair",
                description="Emergency life support repairs",
                pros=["Restores atmosphere quality", "Critical safety measure"],
                cons=["Costs spare parts", "All hands repair effort"],
            ),
            Option(
                action="seal_sections",
                description="Reduce inhabited volume to ease life support load",
                pros=["Reduces system strain immediately"],
                cons=["Crowding", "Morale impact"],
            ),
        ],
    )


# ── Scoring ──────────────────────────────────────────────────────

def _score_options(
    deliberation: Deliberation,
    resources: Resources,
    population: Population,
    ship_systems: ShipSystems,
    notables: list[Notable],
) -> None:
    """Score each option and assign advocates from Notable individuals."""
    for option in deliberation.options:
        # Base score from pros/cons count (simple heuristic)
        score = 0.5 + len(option.pros) * 0.1 - len(option.cons) * 0.08

        # Context-based scoring adjustments
        if deliberation.trigger == "food_low":
            if option.action == "reduce_rations":
                score += 0.2 if resources.food < 500 else 0.1
                keywords = ["reduce", "ration", "conserve"]
            elif option.action == "expand_hydroponics":
                score += 0.15 if resources.spare_parts > 200 else -0.1
                keywords = ["improvise", "repurpose"]
            else:
                score -= 0.1
                keywords = ["risk", "opportunity"]

        elif deliberation.trigger == "hull_damage":
            if option.action == "emergency_repair":
                score += 0.2 if ship_systems.hull_integrity < 0.50 else 0.1
                keywords = ["protocol", "standard"]
            elif option.action == "seal_damaged_sections":
                score += 0.1
                keywords = ["conserve", "efficiency"]
            else:
                score += 0.05
                keywords = ["slow", "standard"]

        elif deliberation.trigger == "morale_low":
            if option.action == "community_event":
                score += 0.15
                keywords = ["morale", "rest", "crew_welfare"]
            elif option.action == "improve_conditions":
                score += 0.1 if resources.materials > 300 else -0.05
                keywords = ["improvise", "repurpose"]
            else:
                score += 0.05
                keywords = ["standard", "protocol"]

        elif deliberation.trigger == "water_low":
            if option.action == "water_rationing":
                score += 0.15
                keywords = ["reduce", "ration", "conserve"]
            elif option.action == "reclamation_upgrade":
                score += 0.15 if resources.spare_parts > 150 else -0.1
                keywords = ["improvise", "repurpose"]
            else:
                keywords = ["standard"]

        elif deliberation.trigger == "fuel_low":
            if option.action == "reduce_speed":
                score += 0.15
                keywords = ["conserve", "slow", "reduce"]
            elif option.action == "power_conservation":
                score += 0.10
                keywords = ["efficiency", "conserve"]
            else:
                score -= 0.05
                keywords = ["risk", "push"]

        elif deliberation.trigger == "engine_critical":
            if option.action == "engine_overhaul":
                score += 0.2 if resources.spare_parts > 200 else 0.05
                keywords = ["protocol", "standard"]
            else:
                score += 0.1
                keywords = ["improvise", "repurpose"]

        elif deliberation.trigger == "life_support_critical":
            if option.action == "life_support_repair":
                score += 0.2
                keywords = ["protocol", "standard", "medical"]
            else:
                score += 0.05
                keywords = ["conserve", "efficiency"]

        else:
            keywords = []

        # Add some randomness
        score += random.uniform(-0.1, 0.1)
        option.score = max(0.0, min(1.0, score))

        # Find advocate
        advocate, trait = _find_advocate(notables, keywords)
        option.advocate = advocate
        option.advocate_trait = trait

    # Sort by score descending, pick the best
    deliberation.options.sort(key=lambda o: o.score, reverse=True)
    if deliberation.options:
        deliberation.chosen = deliberation.options[0]
        deliberation.rejected = deliberation.options[1:]


# ── Apply effects ────────────────────────────────────────────────

def _apply_decision(
    deliberation: Deliberation,
    resources: Resources,
    population: Population,
    ship_systems: ShipSystems,
) -> None:
    """Apply the effects of a chosen action to the world state."""
    if not deliberation.chosen:
        return

    action = deliberation.chosen.action

    if action == "reduce_rations":
        # Food consumption reduced but morale takes a hit
        population.morale = max(0.0, population.morale - random.uniform(0.08, 0.15))

    elif action == "expand_hydroponics":
        cost = min(resources.spare_parts, random.uniform(30, 80))
        resources.spare_parts -= cost
        resources.food += cost * random.uniform(0.5, 1.5)

    elif action == "emergency_repair":
        cost = min(resources.spare_parts, random.uniform(40, 100))
        resources.spare_parts -= cost
        repair = random.uniform(0.05, 0.15)
        ship_systems.hull_integrity = min(1.0, ship_systems.hull_integrity + repair)

    elif action == "seal_damaged_sections":
        population.morale = max(0.0, population.morale - random.uniform(0.05, 0.10))

    elif action == "community_event":
        population.morale = min(1.0, population.morale + random.uniform(0.10, 0.20))

    elif action == "improve_conditions":
        cost = min(resources.materials, random.uniform(20, 60))
        resources.materials -= cost
        population.morale = min(1.0, population.morale + random.uniform(0.05, 0.15))

    elif action == "address_grievances":
        # Unpredictable outcome
        delta = random.uniform(-0.05, 0.15)
        population.morale = max(0.0, min(1.0, population.morale + delta))

    elif action == "water_rationing":
        population.morale = max(0.0, population.morale - random.uniform(0.05, 0.10))

    elif action == "reclamation_upgrade":
        cost = min(resources.spare_parts, random.uniform(30, 70))
        resources.spare_parts -= cost
        resources.water += cost * random.uniform(0.3, 0.8)

    elif action == "reduce_speed":
        population.morale = max(0.0, population.morale - random.uniform(0.02, 0.06))

    elif action == "power_conservation":
        ship_systems.sensors = max(0.3, ship_systems.sensors - random.uniform(0.05, 0.10))

    elif action == "engine_overhaul":
        cost = min(resources.spare_parts, random.uniform(60, 150))
        resources.spare_parts -= cost
        repair = random.uniform(0.10, 0.25)
        ship_systems.engines = min(1.0, ship_systems.engines + repair)

    elif action == "redistribute_power":
        # Boost engines at cost to sensors or life support
        target = random.choice(["sensors", "life_support"])
        current = getattr(ship_systems, target)
        setattr(ship_systems, target, max(0.3, current - random.uniform(0.05, 0.10)))
        ship_systems.engines = min(1.0, ship_systems.engines + random.uniform(0.05, 0.12))

    elif action == "life_support_repair":
        cost = min(resources.spare_parts, random.uniform(40, 100))
        resources.spare_parts -= cost
        repair = random.uniform(0.08, 0.20)
        ship_systems.life_support = min(1.0, ship_systems.life_support + repair)

    elif action == "seal_sections":
        population.morale = max(0.0, population.morale - random.uniform(0.05, 0.12))
        ship_systems.life_support = min(1.0, ship_systems.life_support + random.uniform(0.03, 0.08))


# ── BehaviorSystem ───────────────────────────────────────────────

class BehaviorSystem(System):
    """Evaluates conditions and produces deliberation records."""

    order = 60  # runs after maintenance systems
    active_phases = {Phase.TRANSIT, Phase.SURVEY}

    def __init__(self) -> None:
        self._last_check_day: int = 0
        self._check_interval: int = 30  # check every 30 days
        self._recent_triggers: dict[str, int] = {}  # trigger → last day fired
        self._cooldown: int = 90  # min days between same trigger

    def tick(self, world: World, game_time: GameTime) -> list[GameEvent]:
        # Only check periodically
        if game_time.day_offset - self._last_check_day < self._check_interval:
            return []
        self._last_check_day = game_time.day_offset

        # Get faction entity with resources/population/ship
        faction_entities = world.entities_with(FactionPhase, Resources, Population, ShipSystems)
        if not faction_entities:
            return []
        faction_id = faction_entities[0]

        resources = world.get_component(faction_id, Resources)
        population = world.get_component(faction_id, Population)
        ship_systems = world.get_component(faction_id, ShipSystems)
        if not resources or not population or not ship_systems:
            return []

        # Gather all living notables
        notables: list[Notable] = []
        for eid, notable in world.all_components_of_type(Notable).items():
            if notable.alive:
                notables.append(notable)

        # Run condition evaluators
        events: list[GameEvent] = []
        conditions = [
            _check_food_low(resources, population),
            _check_hull_damage(ship_systems),
            _check_morale_low(population),
            _check_water_low(resources, population),
            _check_fuel_low(resources),
            _check_engine_critical(ship_systems),
            _check_life_support_critical(ship_systems),
        ]

        for deliberation in conditions:
            if deliberation is None:
                continue

            # Check cooldown
            last_fired = self._recent_triggers.get(deliberation.trigger, 0)
            if game_time.day_offset - last_fired < self._cooldown:
                continue

            # Score options and assign advocates
            _score_options(deliberation, resources, population, ship_systems, notables)

            # Apply the chosen action's effects
            _apply_decision(deliberation, resources, population, ship_systems)

            self._recent_triggers[deliberation.trigger] = game_time.day_offset

            events.append(GameEvent(
                event_type="behavior.deliberation",
                severity=Severity.NOTABLE,
                game_time=game_time,
                data={"deliberation": deliberation.to_dict()},
            ))

        return events
