"""SurveySystem — surveys planets in the target system and decides accept/reject.

Runs during the SURVEY phase. Surveys planets one by one, reveals detailed data,
computes refined habitability, then decides whether to accept or reject the system.
Rejection triggers a new SEARCH cycle; acceptance transitions to FOUNDING.
"""

from __future__ import annotations

import random

from . import GameEvent, GameTime, Severity
from .ecs import World
from .systems import System, Phase
from .components import (
    FactionPhase, Resources, Population, ShipSystems, Notable,
    StarSystem, Planet, TransitState, SearchProgress,
    SurveyProgress, SurveyDetail,
)

# ── Survey detail generation ─────────────────────────────────────

ATMOSPHERE_COMPOSITIONS = {
    "rocky": [
        "nitrogen-oxygen mix", "thin carbon dioxide", "dense nitrogen",
        "trace atmosphere only", "methane-rich", "argon-nitrogen",
    ],
    "gas": ["hydrogen-helium", "ammonia clouds", "methane atmosphere"],
    "ice": ["thin nitrogen", "carbon dioxide frost", "trace gases only"],
}

WATER_QUALITIES = {
    "high": ["fresh subsurface aquifers", "clean surface lakes", "abundant fresh water"],
    "medium": ["saline oceans", "mineral-rich groundwater", "seasonal meltwater"],
    "low": ["trace moisture only", "toxic brine pools", "frozen subsurface ice"],
    "none": ["no detectable water", "completely arid"],
}

MINERAL_TYPES = [
    "rich iron and copper deposits", "abundant rare earth elements",
    "extensive silicate formations", "titanium and aluminum veins",
    "sparse mineral resources", "moderate mixed ores",
    "deep-crust heavy metals", "surface-accessible lithium deposits",
]

HAZARDS = [
    "none detected", "minor seismic activity", "periodic radiation storms",
    "volcanic outgassing", "unstable tectonic plates", "toxic surface chemistry",
    "extreme temperature swings", "high UV exposure",
]

SEASONAL_PATTERNS = [
    "mild, Earth-like seasons", "extreme temperature variation",
    "perpetual twilight zone", "tidally locked — permanent day/night divide",
    "minimal seasonal change", "long harsh winters, brief summers",
    "stable tropical climate", "dust storm seasons",
]


def _generate_survey_detail(planet: Planet) -> SurveyDetail:
    """Generate detailed survey data for a planet."""
    ptype = planet.planet_type

    atmosphere = random.choice(ATMOSPHERE_COMPOSITIONS.get(ptype, ["unknown"]))

    if planet.water > 0.5:
        water_quality = random.choice(WATER_QUALITIES["high"])
    elif planet.water > 0.2:
        water_quality = random.choice(WATER_QUALITIES["medium"])
    elif planet.water > 0.05:
        water_quality = random.choice(WATER_QUALITIES["low"])
    else:
        water_quality = random.choice(WATER_QUALITIES["none"])

    minerals = random.choice(MINERAL_TYPES)

    # Hazards: more likely on less habitable planets
    if planet.habitability > 0.5:
        hazard = random.choices(
            HAZARDS, weights=[5, 2, 1, 1, 1, 0.5, 0.5, 0.5], k=1
        )[0]
    else:
        hazard = random.choices(
            HAZARDS, weights=[1, 2, 2, 2, 2, 2, 2, 2], k=1
        )[0]

    seasonal = random.choice(SEASONAL_PATTERNS)

    # Refined habitability: original ± random drift (survey reveals truth)
    drift = random.uniform(-0.15, 0.15)
    refined = max(0.0, min(1.0, planet.habitability + drift))
    # Hazards penalize
    if hazard not in ("none detected", "minor seismic activity"):
        refined = max(0.0, refined - random.uniform(0.05, 0.15))

    return SurveyDetail(
        atmosphere_composition=atmosphere,
        water_quality=water_quality,
        mineral_deposits=minerals,
        hazards=hazard,
        seasonal_pattern=seasonal,
        refined_habitability=round(refined, 2),
    )


# ── Acceptance threshold ─────────────────────────────────────────

def _compute_acceptance_threshold(
    rejections: int,
    max_rejections: int,
    resources: Resources | None,
    population: Population | None,
    notables: list[Notable],
) -> float:
    """Compute the habitability threshold for accepting a system.

    Starts at ~0.45, decreases with each rejection and resource depletion.
    Trait-based: cautious notables raise it, bold/optimistic lower it.
    """
    base = 0.45

    # Desperation: lower threshold with each rejection
    base -= rejections * 0.08

    # Resource depletion lowers standards
    if resources:
        if resources.food < 300:
            base -= 0.05
        if resources.fuel < 300:
            base -= 0.05

    # Notable traits influence
    for notable in notables:
        if not notable.alive:
            continue
        for trait in notable.traits:
            if trait in ("cautious", "conservative"):
                base += 0.02
            elif trait in ("bold", "optimistic"):
                base -= 0.02
            elif trait == "pragmatic":
                base -= 0.01

    # Force accept on last chance
    if rejections >= max_rejections:
        base = 0.0

    return max(0.0, min(0.8, base))


class SurveySystem(System):
    """Surveys planets in the target system, decides accept/reject."""

    order = 15  # runs after search, before transit
    active_phases = {Phase.SURVEY}

    def tick(self, world: World, game_time: GameTime) -> list[GameEvent]:
        faction_entities = world.entities_with(FactionPhase)
        if not faction_entities:
            return []
        faction_id = faction_entities[0]

        fp = world.get_component(faction_id, FactionPhase)
        if not fp or fp.phase != Phase.SURVEY:
            return []

        # Get or create SurveyProgress
        progress = world.get_component(faction_id, SurveyProgress)
        if progress is None:
            return self._start_survey(world, game_time, faction_id)

        if progress.decided:
            return []

        events: list[GameEvent] = []

        # State machine: in_transit → surveying → next planet or evaluating
        if progress.phase == "in_transit":
            if game_time.day_offset >= progress.in_transit_until:
                progress.phase = "surveying"
                progress.current_planet_survey_start = game_time.day_offset
                # Survey duration: 30–120 days per planet
                progress.current_planet_survey_days = random.randint(30, 120)
                events.append(GameEvent(
                    event_type="survey.planet_survey_begin",
                    severity=Severity.INFO,
                    game_time=game_time,
                    data={
                        "planet_index": progress.current_planet_index,
                        "total_planets": progress.total_planets,
                        "survey_days": progress.current_planet_survey_days,
                    },
                ))

        elif progress.phase == "surveying":
            survey_elapsed = game_time.day_offset - progress.current_planet_survey_start
            if survey_elapsed >= progress.current_planet_survey_days:
                events.extend(
                    self._complete_planet_survey(world, game_time, faction_id, progress)
                )

        elif progress.phase == "evaluating":
            events.extend(
                self._evaluate_system(world, game_time, faction_id, progress)
            )

        return events

    def _start_survey(
        self, world: World, game_time: GameTime, faction_id: int,
    ) -> list[GameEvent]:
        """Initialize survey of the current star system."""
        # Find the selected (arrived-at) star system
        transit = world.get_component(faction_id, TransitState)
        if not transit:
            return []

        system_id = transit.destination_id
        star = world.get_component(system_id, StarSystem)
        if not star:
            return []

        # Count planets in this system
        all_planets = world.all_components_of_type(Planet)
        system_planets = [
            (eid, p) for eid, p in all_planets.items()
            if p.parent_system_id == system_id
        ]

        if not system_planets:
            return []

        # Carry over rejection count from previous survey if any
        old_progress = world.get_component(faction_id, SurveyProgress)
        rejections = old_progress.rejections if old_progress else 0

        progress = SurveyProgress(
            system_id=system_id,
            current_planet_index=0,
            total_planets=len(system_planets),
            survey_start_day=game_time.day_offset,
            in_transit_until=game_time.day_offset + random.randint(3, 14),
            phase="in_transit",
            rejections=rejections,
        )
        world.add_component(faction_id, progress)

        return [GameEvent(
            event_type="survey.begin",
            severity=Severity.NOTABLE,
            game_time=game_time,
            auto_pause=True,
            data={
                "system_name": star.name,
                "num_planets": len(system_planets),
                "rejections_so_far": rejections,
            },
        )]

    def _complete_planet_survey(
        self, world: World, game_time: GameTime,
        faction_id: int, progress: SurveyProgress,
    ) -> list[GameEvent]:
        """Complete survey of current planet, reveal detailed data."""
        events: list[GameEvent] = []

        # Find the planet at current index
        all_planets = world.all_components_of_type(Planet)
        system_planets = sorted(
            [(eid, p) for eid, p in all_planets.items()
             if p.parent_system_id == progress.system_id],
            key=lambda x: x[0],  # stable order by entity ID
        )

        if progress.current_planet_index >= len(system_planets):
            progress.phase = "evaluating"
            return events

        p_eid, planet = system_planets[progress.current_planet_index]

        # Generate and store survey detail
        detail = _generate_survey_detail(planet)
        detail.planet_id = p_eid
        world.add_component(p_eid, detail)

        # Track best planet
        if detail.refined_habitability > progress.best_planet_hab:
            progress.best_planet_hab = detail.refined_habitability
            progress.best_planet_name = planet.name

        progress.planets_surveyed += 1

        events.append(GameEvent(
            event_type="survey.planet_report",
            severity=Severity.NOTABLE,
            game_time=game_time,
            data={
                "planet_name": planet.name,
                "planet_type": planet.planet_type,
                "size": planet.size,
                "initial_hab": planet.habitability,
                "refined_hab": detail.refined_habitability,
                "atmosphere": detail.atmosphere_composition,
                "water": detail.water_quality,
                "minerals": detail.mineral_deposits,
                "hazards": detail.hazards,
                "seasons": detail.seasonal_pattern,
                "planet_number": progress.planets_surveyed,
                "total_planets": progress.total_planets,
            },
        ))

        # Move to next planet or start evaluation
        progress.current_planet_index += 1
        if progress.current_planet_index >= progress.total_planets:
            progress.phase = "evaluating"
        else:
            # In-system transit to next planet: 3–21 days
            progress.phase = "in_transit"
            transit_days = random.randint(3, 21)
            progress.in_transit_until = game_time.day_offset + transit_days

        return events

    def _evaluate_system(
        self, world: World, game_time: GameTime,
        faction_id: int, progress: SurveyProgress,
    ) -> list[GameEvent]:
        """Evaluate the system and decide: accept (→ FOUNDING) or reject (→ SEARCH)."""
        events: list[GameEvent] = []

        resources = world.get_component(faction_id, Resources)
        population = world.get_component(faction_id, Population)

        # Gather living notables
        notables: list[Notable] = []
        for eid, n in world.all_components_of_type(Notable).items():
            if n.alive:
                notables.append(n)

        threshold = _compute_acceptance_threshold(
            progress.rejections, progress.max_rejections,
            resources, population, notables,
        )

        star = world.get_component(progress.system_id, StarSystem)
        system_name = star.name if star else "Unknown"

        progress.decided = True

        if progress.best_planet_hab >= threshold:
            # ACCEPT → FOUNDING
            progress.accepted = True
            if star:
                star.surveyed = True

            fp = world.get_component(faction_id, FactionPhase)
            if fp:
                fp.phase = Phase.FOUNDING

            events.append(GameEvent(
                event_type="survey.system_accepted",
                severity=Severity.MILESTONE,
                game_time=game_time,
                auto_pause=True,
                data={
                    "system_name": system_name,
                    "best_planet": progress.best_planet_name,
                    "best_hab": progress.best_planet_hab,
                    "threshold": round(threshold, 2),
                    "rejections": progress.rejections,
                },
            ))

            events.append(GameEvent(
                event_type="phase.transition",
                severity=Severity.MILESTONE,
                game_time=game_time,
                data={"from": Phase.SURVEY.value, "to": Phase.FOUNDING.value},
            ))

        else:
            # REJECT → back to SEARCH
            progress.accepted = False
            progress.rejections += 1
            if star:
                star.rejected = True
                star.surveyed = True
                star.selected = False

            events.append(GameEvent(
                event_type="survey.system_rejected",
                severity=Severity.NOTABLE,
                game_time=game_time,
                auto_pause=True,
                data={
                    "system_name": system_name,
                    "best_planet": progress.best_planet_name,
                    "best_hab": progress.best_planet_hab,
                    "threshold": round(threshold, 2),
                    "rejections": progress.rejections,
                    "max_rejections": progress.max_rejections,
                },
            ))

            # Reset for new search cycle
            self._initiate_new_search(world, game_time, faction_id, progress)

            events.append(GameEvent(
                event_type="phase.transition",
                severity=Severity.MILESTONE,
                game_time=game_time,
                data={"from": Phase.SURVEY.value, "to": Phase.SEARCH.value},
            ))

        return events

    def _initiate_new_search(
        self, world: World, game_time: GameTime,
        faction_id: int, progress: SurveyProgress,
    ) -> None:
        """Reset to SEARCH phase for a new cycle, preserving rejection count."""
        fp = world.get_component(faction_id, FactionPhase)
        if fp:
            fp.phase = Phase.SEARCH

        # Remove old TransitState
        world.remove_component(faction_id, TransitState)

        # Reset SearchProgress for a new round (fewer targets, faster)
        old_search = world.get_component(faction_id, SearchProgress)
        new_search = SearchProgress(
            systems_discovered=0,
            target_discoveries=random.randint(2, 3),  # fewer targets on retry
            discovery_chance_per_day=0.012,  # faster discovery on retry
        )
        world.add_component(faction_id, new_search)

        # Keep the SurveyProgress with updated rejection count
        # It will be replaced when next survey starts
