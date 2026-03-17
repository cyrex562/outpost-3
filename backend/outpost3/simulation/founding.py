"""FoundingSystem — landing, first structures, colony naming, and milestone.

Runs during the FOUNDING phase. Selects the best surveyed planet as a landing
site, sequences through descent → first structures → colony naming, then
emits the founding milestone that completes the game arc.
"""

from __future__ import annotations

import random

from . import GameEvent, GameTime, Severity
from .ecs import World
from .systems import System, Phase
from .components import (
    FactionPhase, Resources, Population, ShipSystems, Notable,
    StarSystem, Planet, SurveyProgress, SurveyDetail, Loadout,
)
from .generation.names import FIRST_NAMES, LAST_NAMES


# ── Colony name generation ───────────────────────────────────────

COLONY_PREFIXES = [
    "New", "Nova", "Port", "Fort", "Camp", "Haven",
]

COLONY_NATURE_WORDS = [
    "Horizon", "Dawn", "Hope", "Promise", "Haven", "Anchor",
    "Refuge", "Prospect", "Meridian", "Threshold", "Genesis",
    "Solace", "Venture", "Summit", "Cradle", "Hearth",
]


def _generate_colony_name(notables: list[Notable], planet_name: str) -> str:
    """Generate a colony name, possibly from a Notable's surname."""
    roll = random.random()

    if roll < 0.3 and notables:
        # Named after a Notable (living or recently deceased)
        notable = random.choice(notables)
        surname = notable.name.split()[-1] if " " in notable.name else notable.name
        return f"{random.choice(COLONY_PREFIXES)} {surname}"
    elif roll < 0.6:
        # Nature/concept name
        return f"{random.choice(COLONY_PREFIXES)} {random.choice(COLONY_NATURE_WORDS)}"
    else:
        # Planet-derived
        base = planet_name.split("-")[-1].capitalize() if "-" in planet_name else planet_name
        return f"{base} Colony"


# ── Founding condition assessment ────────────────────────────────

def _assess_founding_conditions(
    resources: Resources | None,
    population: Population | None,
    ship_systems: ShipSystems | None,
    loadout: Loadout | None,
) -> dict:
    """Assess how well the colony is starting based on journey outcomes."""
    assessment = {
        "condition": "strong",  # strong, moderate, weakened, desperate
        "details": [],
    }

    issues = 0

    if resources:
        if resources.food < 200:
            assessment["details"].append("Food stores critically low")
            issues += 2
        elif resources.food < 500:
            assessment["details"].append("Food stores diminished")
            issues += 1

        if resources.fuel < 100:
            assessment["details"].append("Almost no fuel remaining")
            issues += 1

        if resources.spare_parts < 50:
            assessment["details"].append("Spare parts nearly exhausted")
            issues += 1

        if resources.materials < 100:
            assessment["details"].append("Building materials scarce")
            issues += 1

    if population:
        if loadout and loadout.crew_size > 0:
            pop_ratio = population.total / loadout.crew_size
            if pop_ratio < 0.5:
                assessment["details"].append(f"Population halved during journey ({population.total} of {loadout.crew_size})")
                issues += 2
            elif pop_ratio < 0.8:
                assessment["details"].append(f"Significant population loss ({population.total} of {loadout.crew_size})")
                issues += 1

        if population.morale < 0.3:
            assessment["details"].append("Crew morale dangerously low")
            issues += 2
        elif population.morale < 0.5:
            assessment["details"].append("Crew morale shaky")
            issues += 1

    if ship_systems:
        avg_health = (
            ship_systems.hull_integrity + ship_systems.engines +
            ship_systems.life_support + ship_systems.sensors
        ) / 4
        if avg_health < 0.4:
            assessment["details"].append("Ship systems severely degraded — limited salvageable equipment")
            issues += 2
        elif avg_health < 0.6:
            assessment["details"].append("Ship systems worn — some equipment salvageable")
            issues += 1

    if issues >= 4:
        assessment["condition"] = "desperate"
    elif issues >= 2:
        assessment["condition"] = "weakened"
    elif issues >= 1:
        assessment["condition"] = "moderate"

    if not assessment["details"]:
        assessment["details"].append("Crew and supplies in good condition")

    return assessment


class FoundingSystem(System):
    """Handles the colony founding sequence."""

    order = 5  # runs early
    active_phases = {Phase.FOUNDING}

    def __init__(self) -> None:
        self._founding_day: int = 0
        self._stage: str = "init"  # init, descent, structures, naming, complete

    def tick(self, world: World, game_time: GameTime) -> list[GameEvent]:
        faction_entities = world.entities_with(FactionPhase)
        if not faction_entities:
            return []
        faction_id = faction_entities[0]

        if self._stage == "complete":
            return []

        events: list[GameEvent] = []

        if self._stage == "init":
            events.extend(self._start_founding(world, game_time, faction_id))
        elif self._stage == "descent" and game_time.day_offset >= self._founding_day + 7:
            events.extend(self._descent_complete(world, game_time, faction_id))
        elif self._stage == "structures" and game_time.day_offset >= self._founding_day + 37:
            events.extend(self._first_structures(world, game_time, faction_id))
        elif self._stage == "naming" and game_time.day_offset >= self._founding_day + 44:
            events.extend(self._name_colony(world, game_time, faction_id))

        return events

    def _start_founding(
        self, world: World, game_time: GameTime, faction_id: int,
    ) -> list[GameEvent]:
        """Begin founding sequence — select landing site."""
        self._founding_day = game_time.day_offset
        self._stage = "descent"

        # Find best surveyed planet
        survey = world.get_component(faction_id, SurveyProgress)
        planet_name = survey.best_planet_name if survey else "the target world"
        best_hab = survey.best_planet_hab if survey else 0.5

        # Get system name
        system_name = "Unknown"
        if survey:
            star = world.get_component(survey.system_id, StarSystem)
            if star:
                system_name = star.name

        return [GameEvent(
            event_type="founding.landing_site_selected",
            severity=Severity.MILESTONE,
            game_time=game_time,
            auto_pause=True,
            data={
                "planet_name": planet_name,
                "system_name": system_name,
                "habitability": best_hab,
            },
        )]

    def _descent_complete(
        self, world: World, game_time: GameTime, faction_id: int,
    ) -> list[GameEvent]:
        """Descent sequence complete — boots on the ground."""
        self._stage = "structures"

        population = world.get_component(faction_id, Population)
        pop_count = population.total if population else 0

        return [GameEvent(
            event_type="founding.descent_complete",
            severity=Severity.NOTABLE,
            game_time=game_time,
            data={
                "population": pop_count,
            },
        )]

    def _first_structures(
        self, world: World, game_time: GameTime, faction_id: int,
    ) -> list[GameEvent]:
        """First structures established on the surface."""
        self._stage = "naming"

        resources = world.get_component(faction_id, Resources)
        ship_systems = world.get_component(faction_id, ShipSystems)

        return [GameEvent(
            event_type="founding.first_structures",
            severity=Severity.NOTABLE,
            game_time=game_time,
            data={
                "materials_remaining": round(resources.materials, 1) if resources else 0,
                "salvaged_equipment": round(
                    (ship_systems.hull_integrity + ship_systems.engines) * 50, 1
                ) if ship_systems else 0,
            },
        )]

    def _name_colony(
        self, world: World, game_time: GameTime, faction_id: int,
    ) -> list[GameEvent]:
        """Name the colony and emit the founding milestone."""
        self._stage = "complete"

        events: list[GameEvent] = []

        resources = world.get_component(faction_id, Resources)
        population = world.get_component(faction_id, Population)
        ship_systems = world.get_component(faction_id, ShipSystems)
        loadout = world.get_component(faction_id, Loadout)
        survey = world.get_component(faction_id, SurveyProgress)

        # Gather all notables (living and dead) for naming
        all_notables = [n for eid, n in world.all_components_of_type(Notable).items()]
        living_notables = [n for n in all_notables if n.alive]

        planet_name = survey.best_planet_name if survey else "the world"

        # Generate colony name
        colony_name = _generate_colony_name(all_notables, planet_name)

        # Assess founding conditions
        assessment = _assess_founding_conditions(
            resources, population, ship_systems, loadout,
        )

        # Compute journey stats
        journey_years = round(game_time.day_offset / 365, 1)
        rejections = survey.rejections if survey else 0

        events.append(GameEvent(
            event_type="founding.colony_established",
            severity=Severity.MILESTONE,
            game_time=game_time,
            auto_pause=True,
            data={
                "colony_name": colony_name,
                "planet_name": planet_name,
                "population": population.total if population else 0,
                "morale": round(population.morale, 2) if population else 0,
                "condition": assessment["condition"],
                "condition_details": assessment["details"],
                "journey_years": journey_years,
                "rejections": rejections,
                "notables_alive": len(living_notables),
                "notables_total": len(all_notables),
                "resources": resources.to_summary() if resources else {},
                "ship_systems": ship_systems.to_summary() if ship_systems else {},
            },
        ))

        return events
