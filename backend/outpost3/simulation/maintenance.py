"""ShipMaintenanceSystem — degradation, breakdowns, repairs, and random transit events.

Runs during TRANSIT and SURVEY phases. Handles gradual ship system degradation,
random breakdowns, repair using spare parts, and narrative transit events.
"""

from __future__ import annotations

import random

from . import GameEvent, GameTime, Severity
from .ecs import World
from .systems import System, Phase
from .components import (
    FactionPhase, Resources, Population, ShipSystems, TransitState,
)


# ── Transit random event definitions ─────────────────────────────

TRANSIT_EVENTS = [
    {
        "type": "transit.event.asteroid_field",
        "name": "Asteroid Field",
        "weight": 2.0,
        "effects": {"hull_damage": (0.02, 0.08), "fuel_cost": (10, 40)},
    },
    {
        "type": "transit.event.equipment_failure",
        "name": "Equipment Failure",
        "weight": 2.0,
        "effects": {"system_damage": (0.05, 0.15)},
    },
    {
        "type": "transit.event.cosmic_phenomenon",
        "name": "Cosmic Phenomenon",
        "weight": 3.0,
        "effects": {},  # flavor only
    },
    {
        "type": "transit.event.resource_find",
        "name": "Resource Discovery",
        "weight": 1.0,
        "effects": {"resource_gain": (20, 80)},
    },
    {
        "type": "transit.event.crew_conflict",
        "name": "Crew Conflict",
        "weight": 1.5,
        "effects": {"morale_cost": (0.03, 0.10)},
    },
    {
        "type": "transit.event.celebration",
        "name": "Crew Celebration",
        "weight": 1.5,
        "effects": {"morale_gain": (0.03, 0.08)},
    },
    {
        "type": "transit.event.cultural_shift",
        "name": "Cultural Shift",
        "weight": 1.0,
        "effects": {},  # narrative only
    },
]

# Flavor text for cosmic phenomena
COSMIC_PHENOMENA = [
    "A distant nebula paints the void in shades of crimson and gold.",
    "Sensors detect a rogue planet drifting through interstellar space — a world without a star.",
    "The ship passes through a region of unusually dense cosmic dust.",
    "A pulsar beacon sweeps across the ship's path — beautiful and eerie.",
    "Gravitational ripples from a distant binary system create faint hull vibrations.",
]

CULTURAL_SHIFTS = [
    "A new tradition emerges among the crew — marking each transit anniversary with shared stories.",
    "The ship's children have developed their own dialect, blending their parents' languages.",
    "A philosophical movement gains traction: the journey matters more than the destination.",
    "Crew members begin naming corridors and compartments after places they'll never see again.",
    "The line between 'crew' and 'family' blurs further with each passing year.",
]


class ShipMaintenanceSystem(System):
    """Handles ship degradation, repairs, breakdowns, and transit events."""

    order = 40
    active_phases = {Phase.TRANSIT, Phase.SURVEY}

    def __init__(self) -> None:
        self._last_maintenance_day: int = 0
        self._maintenance_interval: int = 30  # monthly
        self._last_event_day: int = 0
        self._event_interval_min: int = 60   # minimum days between random events
        self._event_interval_max: int = 240  # maximum days between random events
        self._next_event_day: int = 0

    def tick(self, world: World, game_time: GameTime) -> list[GameEvent]:
        events: list[GameEvent] = []

        faction_entities = world.entities_with(FactionPhase, Resources, ShipSystems)
        if not faction_entities:
            return []
        faction_id = faction_entities[0]

        resources = world.get_component(faction_id, Resources)
        ship_systems = world.get_component(faction_id, ShipSystems)
        population = world.get_component(faction_id, Population)
        if not resources or not ship_systems:
            return []

        # ── Gradual degradation (monthly) ────────────────────────
        if game_time.day_offset - self._last_maintenance_day >= self._maintenance_interval:
            self._last_maintenance_day = game_time.day_offset
            events.extend(self._degrade_and_repair(
                resources, ship_systems, population, game_time
            ))

        # ── Random transit events ────────────────────────────────
        faction_phase = world.get_component(faction_id, FactionPhase)
        if faction_phase and faction_phase.phase == Phase.TRANSIT:
            if self._next_event_day == 0:
                self._next_event_day = game_time.day_offset + random.randint(
                    self._event_interval_min, self._event_interval_max
                )

            if game_time.day_offset >= self._next_event_day:
                events.extend(self._trigger_random_event(
                    world, game_time, resources, ship_systems, population
                ))
                self._next_event_day = game_time.day_offset + random.randint(
                    self._event_interval_min, self._event_interval_max
                )

        return events

    def _degrade_and_repair(
        self,
        resources: Resources,
        ship_systems: ShipSystems,
        population: Population | None,
        game_time: GameTime,
    ) -> list[GameEvent]:
        """Apply gradual degradation and auto-repair using spare parts."""
        events: list[GameEvent] = []

        systems = [
            ("hull_integrity", ship_systems),
            ("engines", ship_systems),
            ("life_support", ship_systems),
            ("sensors", ship_systems),
        ]

        for sys_name, ss in systems:
            current = getattr(ss, sys_name)

            # Degradation: ~0.1–0.3% per month, slightly random
            degrade = random.uniform(0.001, 0.003)
            new_val = max(0.0, current - degrade)
            setattr(ss, sys_name, round(new_val, 4))

            # Random breakdown chance: ~2% per system per month
            if random.random() < 0.02:
                breakdown_damage = random.uniform(0.03, 0.10)
                new_val = max(0.0, getattr(ss, sys_name) - breakdown_damage)
                setattr(ss, sys_name, round(new_val, 4))
                events.append(GameEvent(
                    event_type="maintenance.breakdown",
                    severity=Severity.NOTABLE,
                    game_time=game_time,
                    data={
                        "system": sys_name,
                        "damage": round(breakdown_damage * 100, 1),
                        "health": round(new_val * 100, 1),
                    },
                ))

        # Auto-repair: use spare parts to fix the worst system
        worst_name = min(
            ["hull_integrity", "engines", "life_support", "sensors"],
            key=lambda s: getattr(ship_systems, s),
        )
        worst_val = getattr(ship_systems, worst_name)

        if worst_val < 0.90 and resources.spare_parts > 5:
            repair = min(random.uniform(0.01, 0.03), (1.0 - worst_val))
            cost = repair * random.uniform(100, 200)
            cost = min(cost, resources.spare_parts)
            if cost > 0:
                resources.spare_parts -= cost
                new_val = min(1.0, worst_val + repair)
                setattr(ship_systems, worst_name, round(new_val, 4))

        return events

    def _trigger_random_event(
        self,
        world: World,
        game_time: GameTime,
        resources: Resources,
        ship_systems: ShipSystems,
        population: Population | None,
    ) -> list[GameEvent]:
        """Trigger a random transit event."""
        events: list[GameEvent] = []

        # Weighted random selection
        weights = [e["weight"] for e in TRANSIT_EVENTS]
        chosen = random.choices(TRANSIT_EVENTS, weights=weights, k=1)[0]

        effects = chosen["effects"]
        event_data: dict = {"event_name": chosen["name"]}

        # Apply effects
        if "hull_damage" in effects:
            lo, hi = effects["hull_damage"]
            damage = random.uniform(lo, hi)
            ship_systems.hull_integrity = max(0.0, ship_systems.hull_integrity - damage)
            event_data["hull_damage"] = round(damage * 100, 1)

        if "fuel_cost" in effects:
            lo, hi = effects["fuel_cost"]
            cost = random.uniform(lo, hi)
            resources.fuel = max(0, resources.fuel - cost)
            event_data["fuel_cost"] = round(cost, 1)

        if "system_damage" in effects:
            lo, hi = effects["system_damage"]
            damage = random.uniform(lo, hi)
            system_name = random.choice(["engines", "life_support", "sensors"])
            current = getattr(ship_systems, system_name)
            setattr(ship_systems, system_name, max(0.0, round(current - damage, 4)))
            event_data["system"] = system_name
            event_data["damage"] = round(damage * 100, 1)

        if "resource_gain" in effects:
            lo, hi = effects["resource_gain"]
            gain = random.uniform(lo, hi)
            resource_type = random.choice(["materials", "spare_parts", "food"])
            current = getattr(resources, resource_type)
            setattr(resources, resource_type, round(current + gain, 1))
            event_data["resource_type"] = resource_type
            event_data["gain"] = round(gain, 1)

        if "morale_cost" in effects and population:
            lo, hi = effects["morale_cost"]
            cost = random.uniform(lo, hi)
            population.morale = max(0.05, population.morale - cost)
            event_data["morale_change"] = round(-cost * 100, 1)

        if "morale_gain" in effects and population:
            lo, hi = effects["morale_gain"]
            gain = random.uniform(lo, hi)
            population.morale = min(0.98, population.morale + gain)
            event_data["morale_change"] = round(gain * 100, 1)

        # Add flavor text for narrative-only events
        if chosen["type"] == "transit.event.cosmic_phenomenon":
            event_data["flavor"] = random.choice(COSMIC_PHENOMENA)
        elif chosen["type"] == "transit.event.cultural_shift":
            event_data["flavor"] = random.choice(CULTURAL_SHIFTS)

        severity = Severity.NOTABLE
        if chosen["type"] in ("transit.event.cosmic_phenomenon", "transit.event.cultural_shift",
                              "transit.event.celebration"):
            severity = Severity.INFO

        events.append(GameEvent(
            event_type=chosen["type"],
            severity=severity,
            game_time=game_time,
            data=event_data,
        ))

        return events
