"""PopulationSystem — births, deaths, aging, Notable succession.

Runs during TRANSIT and SURVEY phases. Manages demographic changes,
ages Notable individuals, handles death and succession events.
"""

from __future__ import annotations

import random

from . import GameEvent, GameTime, Severity
from .ecs import World
from .systems import System, Phase
from .components import (
    FactionPhase, Resources, Population, Notable, ROLES,
)
from .generation.names import generate_name


class PopulationSystem(System):
    """Manages population changes: births, deaths, aging, Notable lifecycle."""

    order = 30
    active_phases = {Phase.TRANSIT, Phase.SURVEY}

    def __init__(self) -> None:
        self._last_update_day: int = 0
        self._update_interval: int = 30  # process monthly

    def tick(self, world: World, game_time: GameTime) -> list[GameEvent]:
        if game_time.day_offset - self._last_update_day < self._update_interval:
            return []
        self._last_update_day = game_time.day_offset

        faction_entities = world.entities_with(FactionPhase, Population)
        if not faction_entities:
            return []
        faction_id = faction_entities[0]

        population = world.get_component(faction_id, Population)
        resources = world.get_component(faction_id, Resources)
        if not population:
            return []

        events: list[GameEvent] = []
        days = self._update_interval

        # ── Births ───────────────────────────────────────────────
        # Base: ~15 per 1000 per year, modified by morale and food
        base_birth_rate = 0.015 / 365 * days
        morale_factor = 0.5 + population.morale * 0.5  # 0.5–1.0
        food_factor = 1.0
        if resources and resources.food < population.total * 0.5:
            food_factor = 0.5  # reduced births when food is scarce

        expected_births = population.total * base_birth_rate * morale_factor * food_factor
        births = max(0, int(random.gauss(expected_births, max(1, expected_births * 0.3))))

        # ── Deaths ───────────────────────────────────────────────
        # Base: ~8 per 1000 per year, increased by poor conditions
        base_death_rate = 0.008 / 365 * days
        medical_factor = 1.0
        if population.medical < population.total * 0.03:
            medical_factor = 1.5  # insufficient medical staff
        food_death_factor = 1.0
        if resources and resources.food <= 0:
            food_death_factor = 3.0  # starvation
        elif resources and resources.food < population.total * 0.2:
            food_death_factor = 1.5

        expected_deaths = population.total * base_death_rate * medical_factor * food_death_factor
        deaths = max(0, int(random.gauss(expected_deaths, max(1, expected_deaths * 0.3))))
        deaths = min(deaths, population.total - 1)  # at least 1 survives

        # Apply
        population.total = max(1, population.total + births - deaths)
        population.births_year += births
        population.deaths_year += deaths

        # Rebalance roles proportionally
        total = population.total
        if total > 0:
            # Keep rough proportions
            sci_pct = max(0.08, min(0.25, population.scientists / max(total, 1)))
            eng_pct = max(0.12, min(0.30, population.engineers / max(total, 1)))
            med_pct = max(0.04, min(0.12, population.medical / max(total, 1)))
            mil_pct = max(0.04, min(0.12, population.military / max(total, 1)))

            population.scientists = int(total * sci_pct)
            population.engineers = int(total * eng_pct)
            population.medical = int(total * med_pct)
            population.military = int(total * mil_pct)
            population.civilians = total - (
                population.scientists + population.engineers +
                population.medical + population.military
            )

        # ── Reset yearly counters on year start ──────────────────
        if game_time.is_year_start:
            if population.births_year > 0 or population.deaths_year > 0:
                events.append(GameEvent(
                    event_type="population.yearly_report",
                    severity=Severity.INFO,
                    game_time=game_time,
                    data={
                        "total": population.total,
                        "births": population.births_year,
                        "deaths": population.deaths_year,
                        "net_change": population.births_year - population.deaths_year,
                        "morale": round(population.morale, 2),
                    },
                ))
            population.births_year = 0
            population.deaths_year = 0

        # ── Morale drift ─────────────────────────────────────────
        # Slowly drifts toward a baseline based on conditions
        target_morale = 0.60
        if resources:
            if resources.food > population.total * 1.0:
                target_morale += 0.1
            if resources.food < population.total * 0.3:
                target_morale -= 0.15
        drift = (target_morale - population.morale) * 0.02
        population.morale = max(0.05, min(0.98, population.morale + drift))

        # ── Notable aging and death ──────────────────────────────
        events.extend(self._process_notables(world, game_time, days))

        return events

    def _process_notables(
        self, world: World, game_time: GameTime, days: int
    ) -> list[GameEvent]:
        """Age notables, handle deaths from old age or accident."""
        events: list[GameEvent] = []
        all_notables = world.all_components_of_type(Notable)

        for eid, notable in all_notables.items():
            if not notable.alive:
                continue

            # Age (roughly: days/365 years per update)
            years_passed = days / 365
            notable.age += years_passed  # fractional aging is fine

            # Death chance: increases with age
            # Base: very low, ramps up past 60
            age = int(notable.age)
            if age > 80:
                death_chance = 0.02 * (days / 30)
            elif age > 70:
                death_chance = 0.008 * (days / 30)
            elif age > 60:
                death_chance = 0.003 * (days / 30)
            else:
                death_chance = 0.0005 * (days / 30)  # accident/illness

            if random.random() < death_chance:
                notable.alive = False
                notable.death_day = game_time.day_offset

                events.append(GameEvent(
                    event_type="notable.death",
                    severity=Severity.NOTABLE,
                    game_time=game_time,
                    data={
                        "name": notable.name,
                        "role": notable.role,
                        "age": age,
                    },
                ))

                # Succession: promote a new Notable
                successor_event = self._promote_successor(
                    world, game_time, notable.role
                )
                if successor_event:
                    events.append(successor_event)

        return events

    def _promote_successor(
        self, world: World, game_time: GameTime, role: str
    ) -> GameEvent | None:
        """Promote a new Notable to fill a vacant role."""
        # Check if role is already filled by another living Notable
        all_notables = world.all_components_of_type(Notable)
        for eid, n in all_notables.items():
            if n.alive and n.role == role:
                return None  # role still filled

        # Generate a new Notable
        from .components import ALL_TRAITS, TRAIT_AXES
        name = generate_name()
        age = random.randint(28, 45)

        num_traits = random.randint(2, 3)
        chosen_traits: list[str] = []
        available = list(ALL_TRAITS)
        random.shuffle(available)
        for trait in available:
            if len(chosen_traits) >= num_traits:
                break
            conflict = False
            for a, b in TRAIT_AXES:
                if (trait == a and b in chosen_traits) or (trait == b and a in chosen_traits):
                    conflict = True
                    break
            if not conflict:
                chosen_traits.append(trait)

        new_eid = world.create_entity()
        new_notable = Notable(
            name=name,
            role=role,
            age=age,
            traits=chosen_traits,
            alive=True,
        )
        world.add_component(new_eid, new_notable)

        return GameEvent(
            event_type="notable.succession",
            severity=Severity.NOTABLE,
            game_time=game_time,
            data={
                "name": name,
                "role": role,
                "age": age,
                "traits": chosen_traits,
            },
        )
