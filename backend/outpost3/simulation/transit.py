"""Transit Phase system — simulates the interstellar voyage day-by-day.

Handles:
- Daily/annual resource consumption (food, water, medicine, fuel)
- Annual population updates (births, deaths)
- Slow hull degradation from micrometeorite impacts
- Random transit events (~8 % chance per day)
- Arrival detection → phase transition to "survey"

Both tick() (low speed, per-day) and batch() (high speed, range) are
implemented so the simulation remains consistent across all speed levels.
"""

from __future__ import annotations

import random

from ..simulation import GameEvent, GameTime, Severity
from .components import GamePhase, Population, Resources, ShipHull, Transit
from .systems import System
from .world import World

# ── Consumption rates ─────────────────────────────────────────────

FOOD_PER_PERSON_DAY = 1        # person-days of food consumed per person per day
WATER_PER_PERSON_DAY = 1       # same
MEDICINE_PER_PERSON_YEAR = 2   # medicine units consumed per person per year

# ── Population vital rates (annual) ──────────────────────────────

BIRTH_RATE = 0.015   # 1.5 % per year
DEATH_RATE = 0.008   # 0.8 % per year

# ── Hull degradation ──────────────────────────────────────────────

HULL_CRACK_CHANCE_PER_DAY = 0.04    # 4 % chance of micrometeorite impact
HULL_CRACK_SEVERITY = 0.05          # % integrity lost per impact

# ── Random transit events ─────────────────────────────────────────

# (event_type, weight, severity, food_cost, water_cost, medicine_cost, parts_cost, hull_dmg)
_RANDOM_EVENTS: list[tuple[str, int, Severity, int, int, int, int, float]] = [
    ("transit.equipment_failure",     3, Severity.NOTABLE,  0,  0, 0, 500, 0.0),
    ("transit.medical_emergency",     2, Severity.NOTABLE,  0,  0, 50, 0,  0.0),
    ("transit.hull_microcrack",       2, Severity.INFO,      0,  0, 0, 200, 0.2),
    ("transit.resource_leak",         1, Severity.NOTABLE, 500, 500, 0,  0, 0.0),
    ("transit.crew_conflict",         2, Severity.INFO,     0,   0, 0,   0, 0.0),
    ("transit.scientific_observation",3, Severity.INFO,     0,   0, 0,   0, 0.0),
    ("transit.morale_high",           3, Severity.DEBUG,    0,   0, 0,   0, 0.0),
    ("transit.system_anomaly",        1, Severity.NOTABLE,  0,   0, 0, 100, 0.0),
]
_EVENT_TYPES  = [e[0] for e in _RANDOM_EVENTS]
_EVENT_WEIGHTS = [e[1] for e in _RANDOM_EVENTS]


def _pick_random_event(rng: random.Random = random) -> tuple:   # type: ignore[assignment]
    return random.choices(_RANDOM_EVENTS, weights=_EVENT_WEIGHTS, k=1)[0]


# ── Helper: apply resource costs ──────────────────────────────────

def _apply_costs(
    res: Resources,
    food: int = 0,
    water: int = 0,
    medicine: int = 0,
    parts: int = 0,
) -> None:
    res.food       = max(0, res.food       - food)
    res.water      = max(0, res.water      - water)
    res.medicine   = max(0, res.medicine   - medicine)
    res.spare_parts = max(0, res.spare_parts - parts)


# ── Helper: fetch shared state ────────────────────────────────────

def _get_state(world: World):
    """Return (phase, transit, hull, ship_eid, res, pop) or None if incomplete."""
    phase_pair = world.query_one(GamePhase)
    if not phase_pair or phase_pair[1].phase != "transit":
        return None

    transit_pair = world.query_one(Transit)
    if not transit_pair or transit_pair[1].arrived:
        return None

    ship_pair = world.query_one(ShipHull)
    pop_pair  = world.query_one(Population)
    if not ship_pair or not pop_pair:
        return None

    ship_eid, hull = ship_pair
    _, pop = pop_pair
    res = world.get(ship_eid, Resources)
    phase = phase_pair[1]
    _, transit = transit_pair

    return phase, transit, hull, ship_eid, res, pop


# ── Year-boundary logic ────────────────────────────────────────────

def _process_year(
    game_time: GameTime,
    transit: Transit,
    hull: ShipHull,
    res: Resources | None,
    pop: Population,
) -> list[GameEvent]:
    """Annual population update, medicine consumption, and year summary event."""
    events: list[GameEvent] = []

    # Vital statistics
    births = int(pop.count * BIRTH_RATE)
    deaths = int(pop.count * DEATH_RATE)
    pop.count = max(1, pop.count + births - deaths)
    pop.births_this_year = births
    pop.deaths_this_year = deaths

    # Annual medicine consumption
    if res:
        annual_medicine = pop.count * MEDICINE_PER_PERSON_YEAR
        res.medicine = max(0, res.medicine - annual_medicine)

    transit_year = transit.days_elapsed // 365
    years_remaining = transit.days_remaining / 365
    food_years = (res.food / (pop.count * 365)) if (res and pop.count > 0) else 0.0

    events.append(GameEvent(
        event_type="transit.year_summary",
        severity=Severity.NOTABLE,
        game_time=game_time,
        data={
            "transit_year": transit_year,
            "population": pop.count,
            "births": births,
            "deaths": deaths,
            "years_remaining": round(years_remaining, 1),
            "food_years": round(food_years, 1),
            "hull_integrity": round(hull.integrity, 1),
        },
    ))

    # Low-resource warnings
    if res:
        if res.food < pop.count * 365:   # less than 1 year of food
            events.append(GameEvent(
                event_type="transit.food_critical",
                severity=Severity.CRITICAL,
                game_time=game_time,
                auto_pause=True,
                data={"food_years": round(food_years, 2), "population": pop.count},
            ))
        if res.medicine < pop.count * 10:
            events.append(GameEvent(
                event_type="transit.medicine_low",
                severity=Severity.NOTABLE,
                game_time=game_time,
                data={"medicine": res.medicine, "population": pop.count},
            ))

    return events


def _check_arrival(
    game_time: GameTime,
    transit: Transit,
    hull: ShipHull,
    pop: Population,
    phase: GamePhase,
) -> GameEvent | None:
    """Return a MILESTONE arrival event and advance phase if journey complete."""
    if transit.days_elapsed < transit.duration_days:
        return None

    transit.arrived = True
    phase.phase = "survey"

    return GameEvent(
        event_type="transit.arrival",
        severity=Severity.MILESTONE,
        game_time=game_time,
        auto_pause=True,
        data={
            "destination": transit.destination_name,
            "population": pop.count,
            "hull_integrity": round(hull.integrity, 1),
            "transit_years": transit.duration_days // 365,
        },
    )


def _random_transit_event(
    game_time: GameTime,
    hull: ShipHull,
    res: Resources | None,
) -> GameEvent | None:
    """Randomly generate a transit event and apply its effects."""
    evt_type, _, severity, food_cost, water_cost, med_cost, parts_cost, hull_dmg = (
        _pick_random_event()
    )
    if res:
        _apply_costs(res, food=food_cost, water=water_cost, medicine=med_cost, parts=parts_cost)
    hull.integrity = max(0.0, hull.integrity - hull_dmg)

    return GameEvent(
        event_type=evt_type,
        severity=severity,
        game_time=game_time,
        data={
            "hull_integrity": round(hull.integrity, 1),
            "food_cost": food_cost,
            "water_cost": water_cost,
            "medicine_cost": med_cost,
            "parts_cost": parts_cost,
        },
    )


# ── TransitSystem ─────────────────────────────────────────────────

class TransitSystem(System):
    """Drives the interstellar transit simulation each tick or batch."""

    RANDOM_EVENT_CHANCE = 0.08   # per day at low speed

    # ── tick (low speed: per-day) ─────────────────────────────────

    async def tick(self, game_time: GameTime) -> list[GameEvent]:
        state = _get_state(self.world)
        if state is None:
            return []

        phase, transit, hull, ship_eid, res, pop = state
        events: list[GameEvent] = []

        # Advance transit clock
        transit.days_elapsed += 1

        # Daily resource consumption
        if res:
            _apply_costs(
                res,
                food=pop.count * FOOD_PER_PERSON_DAY,
                water=pop.count * WATER_PER_PERSON_DAY,
            )
            fuel_consumed = int(transit.fuel_per_day)
            res.fuel = max(0, res.fuel - fuel_consumed)

        # Hull micrometeorite degradation
        if random.random() < HULL_CRACK_CHANCE_PER_DAY:
            hull.integrity = max(0.0, hull.integrity - HULL_CRACK_SEVERITY)
            if hull.integrity < 70.0 and random.random() < 0.3:
                events.append(GameEvent(
                    event_type="transit.hull_warning",
                    severity=Severity.CRITICAL,
                    game_time=game_time,
                    auto_pause=True,
                    data={"hull_integrity": round(hull.integrity, 1)},
                ))

        # Annual events (year boundary)
        if game_time.is_year_start and transit.days_elapsed > 0:
            events.extend(_process_year(game_time, transit, hull, res, pop))

        # Random transit event
        if random.random() < self.RANDOM_EVENT_CHANCE:
            evt = _random_transit_event(game_time, hull, res)
            if evt:
                events.append(evt)

        # Arrival check
        arrival = _check_arrival(game_time, transit, hull, pop, phase)
        if arrival:
            events.append(arrival)

        return events

    # ── batch (high speed: day range) ────────────────────────────

    async def batch(
        self,
        start_time: GameTime,
        end_time: GameTime,
        num_days: int,
    ) -> list[GameEvent]:
        state = _get_state(self.world)
        if state is None:
            return []

        phase, transit, hull, ship_eid, res, pop = state
        events: list[GameEvent] = []

        days_to_process = min(num_days, transit.duration_days - transit.days_elapsed)
        if days_to_process <= 0:
            return []

        # ── Bulk resource consumption (snapshot at batch start) ───
        if res:
            _apply_costs(
                res,
                food=pop.count * FOOD_PER_PERSON_DAY * days_to_process,
                water=pop.count * WATER_PER_PERSON_DAY * days_to_process,
            )
            res.fuel = max(0, res.fuel - int(transit.fuel_per_day * days_to_process))

        # ── Hull degradation (expected value over range) ───────────
        total_hull_loss = HULL_CRACK_CHANCE_PER_DAY * HULL_CRACK_SEVERITY * days_to_process
        hull.integrity = max(0.0, hull.integrity - total_hull_loss)

        # ── Year-boundary events for each year crossed ────────────
        # Use absolute game day offsets to find year starts in [start, end].
        for d in range(start_time.day_offset, end_time.day_offset + 1):
            gt = GameTime(d)
            if gt.is_year_start and d > 0:
                # Snapshot of transit.days_elapsed at this year boundary
                days_into_batch = d - start_time.day_offset
                snapshot_elapsed = transit.days_elapsed + days_into_batch
                # Temporarily set for the event data
                saved = transit.days_elapsed
                transit.days_elapsed = snapshot_elapsed
                events.extend(_process_year(gt, transit, hull, res, pop))
                transit.days_elapsed = saved

        # ── Advance transit clock ──────────────────────────────────
        transit.days_elapsed += days_to_process

        # ── Batch summary (only if no year events were emitted) ────
        if not any(e.event_type == "transit.year_summary" for e in events):
            transit_year = transit.days_elapsed // 365
            years_remaining = transit.days_remaining / 365
            food_years = (
                res.food / (pop.count * 365) if (res and pop.count > 0) else 0.0
            )
            events.append(GameEvent(
                event_type="transit.batch_summary",
                severity=Severity.INFO,
                game_time=end_time,
                data={
                    "days_processed": days_to_process,
                    "transit_year": transit_year,
                    "population": pop.count,
                    "years_remaining": round(years_remaining, 1),
                    "food_years": round(food_years, 1),
                    "hull_integrity": round(hull.integrity, 1),
                },
            ))

        # ── Random event (scaled) ─────────────────────────────────
        if random.random() < self.RANDOM_EVENT_CHANCE * (days_to_process / 10):
            evt = _random_transit_event(end_time, hull, res)
            if evt:
                events.append(evt)

        # ── Arrival check ─────────────────────────────────────────
        arrival = _check_arrival(end_time, transit, hull, pop, phase)
        if arrival:
            events.append(arrival)

        return events
