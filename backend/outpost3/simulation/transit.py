"""TransitSystem — moves the ship, consumes resources, handles arrival.

Runs during the TRANSIT phase. Tracks acceleration/cruise/deceleration,
consumes fuel and food/water, and detects arrival at the destination.
"""

from __future__ import annotations

import random

from . import GameEvent, GameTime, Severity
from .ecs import World
from .systems import System, Phase
from .components import (
    FactionPhase, Resources, Population, ShipSystems, TransitState, StarSystem,
)


class TransitSystem(System):
    """Moves the ship toward its destination and consumes resources."""

    order = 20
    active_phases = {Phase.TRANSIT}

    def tick(self, world: World, game_time: GameTime) -> list[GameEvent]:
        faction_entities = world.entities_with(FactionPhase, TransitState, Resources)
        if not faction_entities:
            return []
        faction_id = faction_entities[0]

        transit = world.get_component(faction_id, TransitState)
        resources = world.get_component(faction_id, Resources)
        population = world.get_component(faction_id, Population)
        if not transit or not resources:
            return []

        events: list[GameEvent] = []
        day = game_time.day_offset

        # ── Update transit phase ─────────────────────────────────
        days_in_transit = day - transit.departure_day
        old_phase = transit.transit_phase

        if days_in_transit <= transit.accel_days:
            transit.transit_phase = "accelerating"
        elif day >= transit.decel_start_day:
            transit.transit_phase = "decelerating"
        else:
            transit.transit_phase = "cruising"

        # Emit phase change event
        if transit.transit_phase != old_phase and days_in_transit > 1:
            events.append(GameEvent(
                event_type="transit.phase_change",
                severity=Severity.INFO,
                game_time=game_time,
                data={
                    "transit_phase": transit.transit_phase,
                    "days_in_transit": days_in_transit,
                },
            ))

        # ── Distance tracking ────────────────────────────────────
        # Simplified: constant effective velocity, distance = v * t
        if transit.eta_days > 0:
            fraction = days_in_transit / transit.eta_days
            transit.distance_traveled_ly = min(
                transit.distance_ly,
                transit.distance_ly * fraction,
            )

        # ── Resource consumption ─────────────────────────────────
        pop_total = population.total if population else 1000
        pop_factor = pop_total / 1000.0

        # Fuel: high during accel/decel, minimal during cruise
        if transit.transit_phase in ("accelerating", "decelerating"):
            fuel_cost = 3.0 * pop_factor
        else:
            fuel_cost = 0.3 * pop_factor
        resources.fuel = max(0, resources.fuel - fuel_cost)

        # Food + water: continuous, based on population
        food_cost = pop_total * 0.004  # ~1.46/person/year
        water_cost = pop_total * 0.003
        resources.food = max(0, resources.food - food_cost)
        resources.water = max(0, resources.water - water_cost)

        # ── Periodic transit summary (yearly) ────────────────────
        if game_time.is_year_start and days_in_transit > 30:
            years_in = round(days_in_transit / 365, 1)
            remaining_days = max(0, transit.eta_days - days_in_transit)
            remaining_years = round(remaining_days / 365, 1)
            pct_complete = round(transit.distance_traveled_ly / max(transit.distance_ly, 0.1) * 100, 1)

            events.append(GameEvent(
                event_type="transit.yearly_summary",
                severity=Severity.INFO,
                game_time=game_time,
                data={
                    "years_in_transit": years_in,
                    "remaining_years": remaining_years,
                    "pct_complete": pct_complete,
                    "distance_traveled_ly": round(transit.distance_traveled_ly, 2),
                    "distance_total_ly": transit.distance_ly,
                    "fuel_remaining": round(resources.fuel, 1),
                    "food_remaining": round(resources.food, 1),
                    "population": pop_total,
                    "transit_phase": transit.transit_phase,
                },
            ))

        # ── Arrival detection ────────────────────────────────────
        if days_in_transit >= transit.eta_days:
            transit.distance_traveled_ly = transit.distance_ly

            # Get destination name
            dest_system = world.get_component(transit.destination_id, StarSystem)
            dest_name = dest_system.name if dest_system else "Unknown System"

            events.append(GameEvent(
                event_type="transit.arrival",
                severity=Severity.MILESTONE,
                game_time=game_time,
                auto_pause=True,
                data={
                    "system_name": dest_name,
                    "transit_years": round(days_in_transit / 365, 1),
                    "population": pop_total,
                    "fuel_remaining": round(resources.fuel, 1),
                },
            ))

            # Transition to SURVEY
            faction_phase = world.get_component(faction_id, FactionPhase)
            if faction_phase:
                faction_phase.phase = Phase.SURVEY

            events.append(GameEvent(
                event_type="phase.transition",
                severity=Severity.MILESTONE,
                game_time=game_time,
                data={"from": Phase.TRANSIT.value, "to": Phase.SURVEY.value},
            ))

        return events
