"""Survey Phase and Founding Phase systems.

Survey Phase:
- 1-year observation period after transit arrival
- Scans each planet in the target system sequentially
- At 90% survey completion: autonomous accept/reject verdict
  Accept: colony founded → "founding" phase (MILESTONE auto-pause)
  Reject: re-run search, loop back to "transit" (max 3 rejections)

Founding Phase:
- Terminal state for M1 (observe-only). Colony quality is derived from
  hull integrity, remaining food, and target planet habitability.

Colony quality tiers:
  Struggling  — poor resources / damaged hull / low habitability
  Standard    — moderate conditions
  Strong      — good conditions
  Thriving    — excellent conditions on all fronts
"""

from __future__ import annotations

import random

from ..simulation import GameEvent, GameTime, Severity
from ..narrative import render
from .components import (
    CandidateSystem, Colony, GamePhase, Population,
    Resources, ShipHull, Survey, Transit,
)
from .search import run_search
from .systems import System
from .world import World

# ── Survey timing ─────────────────────────────────────────────────

SURVEY_DURATION_DAYS = 365       # one full year
VERDICT_DAY = int(SURVEY_DURATION_DAYS * 0.90)   # 328 — decision day

# ── Rejection policy ──────────────────────────────────────────────

MAX_REJECTIONS = 3   # forced accept after this many

# ── Colony quality scoring ────────────────────────────────────────

def _colony_quality(habitability: float, hull_integrity: float, food_years: float) -> str:
    score = (habitability / 10) + (hull_integrity / 10) + min(food_years, 10)
    if score >= 25:
        return "thriving"
    if score >= 18:
        return "strong"
    if score >= 12:
        return "standard"
    return "struggling"


# ── Verdict logic ─────────────────────────────────────────────────

def _decide_verdict(
    rejection_count: int,
    best_habitability: float,
    hull_integrity: float,
    food_years: float,
) -> str:
    """Autonomous accept/reject decision.

    Crew always accepts if forced (3 prior rejections), if habitability
    is reasonable, or if they can't afford another long transit.
    """
    if rejection_count >= MAX_REJECTIONS:
        return "accept"
    if best_habitability >= 40.0:
        return "accept"
    # Can't afford another long transit — accept regardless of habitability
    if hull_integrity < 65.0 or food_years < 3.0:
        return "accept"
    return "reject"


# ── Helpers ───────────────────────────────────────────────────────

def _get_state(world: World):
    """Return (phase, survey, target, hull, res, pop) or None."""
    phase_pair = world.query_one(GamePhase)
    if not phase_pair or phase_pair[1].phase != "survey":
        return None

    survey_pair = world.query_one(Survey)
    if not survey_pair:
        return None

    _, phase = phase_pair
    _, survey = survey_pair

    # Find the currently selected target system
    target = None
    for _, sys in world.query(CandidateSystem):
        if sys.selected:
            target = sys
            break

    ship_pair = world.query_one(ShipHull)
    pop_pair = world.query_one(Population)
    if not ship_pair or not pop_pair:
        return None

    ship_eid, hull = ship_pair
    _, pop = pop_pair
    res = world.get(ship_eid, Resources)

    return phase, survey, target, hull, res, pop


def _scan_days(num_planets: int) -> list[int]:
    """Distribute planet scans evenly across the survey period."""
    if num_planets == 0:
        return []
    return [
        int(SURVEY_DURATION_DAYS * (i + 1) / (num_planets + 1))
        for i in range(num_planets)
    ]


# ── Accept / Reject handlers ──────────────────────────────────────

def _found_colony(
    world: World,
    game_time: GameTime,
    survey: Survey,
    target: CandidateSystem | None,
    hull: ShipHull,
    res: Resources | None,
    pop: Population,
    phase: GamePhase,
) -> list[GameEvent]:
    """Create Colony component, transition to "founding", emit MILESTONE."""
    best_planet = (
        max(target.planets, key=lambda p: p.habitability)
        if (target and target.planets) else None
    )
    habitability = target.best_habitability if target else 0.0
    food_years = (
        res.food / (pop.count * 365) if (res and pop.count > 0) else 0.0
    )
    quality = _colony_quality(habitability, hull.integrity, food_years)

    colony_eid = world.new_entity()
    colony = Colony(
        system_name=target.name if target else "Unknown",
        planet_name=best_planet.name if best_planet else "Unknown World",
        habitability=habitability,
        founded_year=game_time.year,
        starting_population=pop.count,
        hull_integrity_at_landing=hull.integrity,
        quality=quality,
    )
    world.add(colony_eid, colony)
    phase.phase = "founding"
    survey.verdict = "accept"

    return [GameEvent(
        event_type="colony.founded",
        severity=Severity.MILESTONE,
        game_time=game_time,
        auto_pause=True,
        data={
            "system_name": colony.system_name,
            "planet_name": colony.planet_name,
            "habitability": habitability,
            "population": pop.count,
            "hull_integrity": round(hull.integrity, 1),
            "quality": quality,
            "founded_year": game_time.year,
            "rejection_count": survey.rejection_count,
        },
    )]


def _reject_system(
    world: World,
    game_time: GameTime,
    survey: Survey,
    target: CandidateSystem | None,
    phase: GamePhase,
) -> list[GameEvent]:
    """Reject the current system, re-run search, loop back to transit."""
    new_rejection_count = survey.rejection_count + 1
    survey.verdict = "reject"

    events: list[GameEvent] = [GameEvent(
        event_type="survey.rejected",
        severity=Severity.NOTABLE,
        game_time=game_time,
        auto_pause=False,
        data={
            "system_name": target.name if target else "unknown",
            "habitability": target.best_habitability if target else 0.0,
            "rejection_count": new_rejection_count,
            "rejections_remaining": max(0, MAX_REJECTIONS - new_rejection_count),
        },
    )]

    # Clear old star systems, transit, and survey state
    # New ones are created by run_search below
    world.clear_type(CandidateSystem)
    world.clear_type(Transit)
    world.clear_type(Survey)

    _, new_search_events = run_search(world, rejection_count=new_rejection_count)
    for evt in new_search_events:
        evt.text = render(evt)
    events.extend(new_search_events)

    # run_search already set phase → "transit"
    return events


# ── SurveySystem ──────────────────────────────────────────────────

class SurveySystem(System):
    """Drives planetary survey and colony founding."""

    # ── tick (low speed) ─────────────────────────────────────────

    async def tick(self, game_time: GameTime) -> list[GameEvent]:
        state = _get_state(self.world)
        if state is None:
            return []

        phase, survey, target, hull, res, pop = state
        events: list[GameEvent] = []

        # First tick of survey — announce arrival
        if survey.days_elapsed == 0:
            events.append(GameEvent(
                event_type="survey.started",
                severity=Severity.NOTABLE,
                game_time=game_time,
                data={
                    "system_name": survey.target_system_name,
                    "planet_count": len(target.planets) if target else 0,
                },
            ))

        survey.days_elapsed += 1

        # Planet scan events at computed intervals
        if target:
            days = _scan_days(len(target.planets))
            if survey.days_elapsed in days:
                idx = days.index(survey.days_elapsed)
                planet = target.planets[idx]
                events.append(GameEvent(
                    event_type="survey.planet_scanned",
                    severity=Severity.INFO,
                    game_time=game_time,
                    data={
                        "planet_name": planet.name,
                        "planet_type": planet.type,
                        "habitability": planet.habitability,
                        "notes": planet.notes,
                        "notes_text": "; ".join(planet.notes) if planet.notes else "No notable features.",
                        "scan_number": idx + 1,
                        "total_planets": len(target.planets),
                    },
                ))

        # Verdict at VERDICT_DAY — pause for player decision
        if survey.days_elapsed >= VERDICT_DAY and survey.verdict is None:
            best_hab = target.best_habitability if target else 0.0
            food_years = (
                res.food / (pop.count * 365) if (res and pop.count > 0) else 0.0
            )
            recommendation = _decide_verdict(
                survey.rejection_count, best_hab, hull.integrity, food_years
            )
            survey.verdict = recommendation
            events.append(GameEvent(
                event_type="survey.verdict_pending",
                severity=Severity.MILESTONE,
                game_time=game_time,
                auto_pause=True,
                data={
                    "system_name": survey.target_system_name,
                    "recommendation": recommendation,
                    "habitability": round(best_hab, 1),
                    "hull_integrity": round(hull.integrity, 1),
                    "food_years": round(food_years, 1),
                    "rejection_count": survey.rejection_count,
                    "rejections_remaining": max(0, MAX_REJECTIONS - survey.rejection_count),
                },
            ))

        return events

    # ── batch (high speed) ────────────────────────────────────────

    async def batch(
        self,
        start_time: GameTime,
        end_time: GameTime,
        num_days: int,
    ) -> list[GameEvent]:
        state = _get_state(self.world)
        if state is None:
            return []

        phase, survey, target, hull, res, pop = state
        events: list[GameEvent] = []

        days_remaining = SURVEY_DURATION_DAYS - survey.days_elapsed
        days_to_process = min(num_days, days_remaining)
        if days_to_process <= 0:
            return []

        batch_start = survey.days_elapsed
        batch_end = survey.days_elapsed + days_to_process

        # Started event on first batch
        if batch_start == 0:
            events.append(GameEvent(
                event_type="survey.started",
                severity=Severity.NOTABLE,
                game_time=start_time,
                data={
                    "system_name": survey.target_system_name,
                    "planet_count": len(target.planets) if target else 0,
                },
            ))

        # Any planet scans within the batch → one summary event
        if target:
            scanned_planets = [
                target.planets[i]
                for i, d in enumerate(_scan_days(len(target.planets)))
                if batch_start < d <= batch_end
            ]
            if scanned_planets:
                events.append(GameEvent(
                    event_type="survey.batch_scan",
                    severity=Severity.INFO,
                    game_time=end_time,
                    data={
                        "system_name": target.name,
                        "planets_scanned": len(scanned_planets),
                        "best_habitability": target.best_habitability,
                    },
                ))

        survey.days_elapsed += days_to_process

        # Verdict if VERDICT_DAY crossed — pause for player decision
        if batch_end >= VERDICT_DAY and survey.verdict is None:
            best_hab = target.best_habitability if target else 0.0
            food_years = (
                res.food / (pop.count * 365) if (res and pop.count > 0) else 0.0
            )
            recommendation = _decide_verdict(
                survey.rejection_count, best_hab, hull.integrity, food_years
            )
            survey.verdict = recommendation
            events.append(GameEvent(
                event_type="survey.verdict_pending",
                severity=Severity.MILESTONE,
                game_time=end_time,
                auto_pause=True,
                data={
                    "system_name": survey.target_system_name,
                    "recommendation": recommendation,
                    "habitability": round(best_hab, 1),
                    "hull_integrity": round(hull.integrity, 1),
                    "food_years": round(food_years, 1),
                    "rejection_count": survey.rejection_count,
                    "rejections_remaining": max(0, MAX_REJECTIONS - survey.rejection_count),
                },
            ))

        return events


# ── Player command ─────────────────────────────────────────────────────────────

def confirm_survey_decision(
    world: World,
    decision: str,
    game_time: GameTime,
) -> list[GameEvent]:
    """Execute the player's accept/reject decision after survey.verdict_pending.

    Args:
        world: the ECS World (must be in survey phase with verdict set)
        decision: "accept" to found the colony, "reject" to search again
        game_time: current engine time for event timestamps

    Returns:
        List of events to broadcast (colony.founded or survey.rejected + search events).

    Raises:
        ValueError: if world is not in a valid pending-verdict state.
    """
    state = _get_state(world)
    if state is None:
        raise ValueError("Not in survey phase — cannot confirm survey decision")

    phase, survey, target, hull, res, pop = state
    if survey.verdict is None:
        raise ValueError("No pending verdict — survey has not reached decision day yet")

    if decision == "accept":
        return _found_colony(world, game_time, survey, target, hull, res, pop, phase)
    elif decision == "reject":
        return _reject_system(world, game_time, survey, target, phase)
    else:
        raise ValueError(f"Invalid decision {decision!r}; must be 'accept' or 'reject'")
