"""LoadoutSystem — generates the ship's starting configuration.

Runs once on day 1 during the LOADOUT phase, then transitions to SEARCH.
"""

from __future__ import annotations

import random

from .. import GameEvent, GameTime, Severity
from ..ecs import World
from ..systems import System, Phase
from ..components import (
    FactionPhase, Resources, Population, ShipSystems,
    Notable, Loadout, ROLES, ALL_TRAITS,
)
from .names import generate_name, generate_ship_name


class LoadoutSystem(System):
    """Generates starting crew, resources, ship systems, and notable individuals."""

    order = 1  # runs first
    active_phases = {Phase.LOADOUT}

    def __init__(self) -> None:
        self._generated = False

    def tick(self, world: World, game_time: GameTime) -> list[GameEvent]:
        if self._generated:
            return []
        self._generated = True

        events: list[GameEvent] = []

        # Find (or create) the faction entity
        faction_entities = world.entities_with(FactionPhase)
        if not faction_entities:
            return []
        faction_id = faction_entities[0]

        ship_name = generate_ship_name()

        # ── Generate crew size ───────────────────────────────────
        crew_size = random.randint(500, 2000)

        # ── Generate resources ───────────────────────────────────
        # Scale resources relative to crew size for viable transit
        scale = crew_size / 1000.0  # normalize around 1000 crew
        resources = Resources(
            materials=round(random.uniform(800, 1500) * scale, 1),
            fuel=round(random.uniform(2000, 4000) * scale, 1),
            food=round(random.uniform(1500, 3000) * scale, 1),
            water=round(random.uniform(1200, 2500) * scale, 1),
            spare_parts=round(random.uniform(300, 800) * scale, 1),
        )

        # ── Generate ship systems ────────────────────────────────
        ship_systems = ShipSystems(
            hull_integrity=round(random.uniform(0.85, 1.0), 3),
            engines=round(random.uniform(0.85, 1.0), 3),
            life_support=round(random.uniform(0.85, 1.0), 3),
            sensors=round(random.uniform(0.85, 1.0), 3),
        )

        # ── Generate population breakdown ────────────────────────
        scientists = int(crew_size * random.uniform(0.10, 0.20))
        engineers = int(crew_size * random.uniform(0.15, 0.25))
        medical = int(crew_size * random.uniform(0.05, 0.10))
        military = int(crew_size * random.uniform(0.05, 0.10))
        civilians = crew_size - scientists - engineers - medical - military

        population = Population(
            total=crew_size,
            scientists=scientists,
            engineers=engineers,
            medical=medical,
            military=military,
            civilians=civilians,
            morale=round(random.uniform(0.70, 0.90), 2),
        )

        # ── Generate Notable individuals ─────────────────────────
        num_notables = random.randint(5, 8)
        available_roles = list(ROLES)
        random.shuffle(available_roles)
        notable_ids: list[int] = []
        notable_names: list[str] = []

        used_names: set[str] = set()
        for i in range(num_notables):
            # Generate unique name
            name = generate_name()
            attempts = 0
            while name in used_names and attempts < 20:
                name = generate_name()
                attempts += 1
            used_names.add(name)

            role = available_roles[i] if i < len(available_roles) else "Specialist"
            age = random.randint(25, 55)

            # Pick 2-3 traits (no contradictions from same axis)
            num_traits = random.randint(2, 3)
            chosen_traits: list[str] = []
            available_traits = list(ALL_TRAITS)
            random.shuffle(available_traits)
            for trait in available_traits:
                if len(chosen_traits) >= num_traits:
                    break
                # Check this trait doesn't conflict with already chosen
                conflict = False
                from ..components import TRAIT_AXES
                for a, b in TRAIT_AXES:
                    if trait == a and b in chosen_traits:
                        conflict = True
                        break
                    if trait == b and a in chosen_traits:
                        conflict = True
                        break
                if not conflict:
                    chosen_traits.append(trait)

            notable_eid = world.create_entity()
            notable = Notable(
                name=name,
                role=role,
                age=age,
                traits=chosen_traits,
                alive=True,
            )
            world.add_component(notable_eid, notable)
            notable_ids.append(notable_eid)
            notable_names.append(name)

            # Emit introduction event for each notable
            events.append(GameEvent(
                event_type="loadout.notable_introduction",
                severity=Severity.INFO,
                game_time=game_time,
                data={
                    "name": name,
                    "role": role,
                    "age": age,
                    "traits": chosen_traits,
                },
            ))

        # ── Attach components to faction entity ──────────────────
        world.add_component(faction_id, resources)
        world.add_component(faction_id, population)
        world.add_component(faction_id, ship_systems)

        # Record immutable loadout
        loadout = Loadout(
            crew_size=crew_size,
            resources=resources.to_summary(),
            ship_systems=ship_systems.to_summary(),
            notable_names=notable_names,
            ship_name=ship_name,
        )
        world.add_component(faction_id, loadout)

        # ── Emit loadout summary event ───────────────────────────
        events.insert(0, GameEvent(
            event_type="loadout.ship_commissioned",
            severity=Severity.MILESTONE,
            game_time=game_time,
            data={"ship_name": ship_name},
            auto_pause=True,
        ))

        events.insert(1, GameEvent(
            event_type="loadout.summary",
            severity=Severity.NOTABLE,
            game_time=game_time,
            data={
                "ship_name": ship_name,
                "crew_size": crew_size,
                "resources": resources.to_summary(),
                "ship_systems": ship_systems.to_summary(),
                "morale": population.morale,
                "num_notables": num_notables,
            },
        ))

        events.insert(2, GameEvent(
            event_type="loadout.crew_manifest",
            severity=Severity.INFO,
            game_time=game_time,
            data={
                "total": crew_size,
                "scientists": scientists,
                "engineers": engineers,
                "medical": medical,
                "military": military,
                "civilians": civilians,
            },
        ))

        # ── Transition to SEARCH ─────────────────────────────────
        faction_phase = world.get_component(faction_id, FactionPhase)
        if faction_phase:
            faction_phase.phase = Phase.SEARCH

        events.append(GameEvent(
            event_type="phase.transition",
            severity=Severity.MILESTONE,
            game_time=game_time,
            data={"from": Phase.LOADOUT.value, "to": Phase.SEARCH.value},
        ))

        return events
