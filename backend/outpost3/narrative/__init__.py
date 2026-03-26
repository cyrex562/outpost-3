"""Narrative template renderer — turns structured events into readable text.

For Milestone 0 this is minimal. Milestone 1 will add ~30 templates for the
colony ship arc.
"""

from __future__ import annotations

import random as _random

from ..simulation import GameEvent


# Template registry: event_type → format string or list of variants
_TEMPLATES: dict[str, str | list[str]] = {
    # Time markers
    "time.year_start": "— Year {year} begins —",
    "time.month_start": "Month {month} of Year {year}.",

    # Daily flavor — keyed by variant in event.data
    "daily.weather": {
        "clear_skies": [
            "Clear skies over the colony site today.",
            "Another cloudless day on the surface.",
        ],
        "dust_storm": [
            "A dust storm rolls across the plains, reducing visibility.",
            "High winds kick up surface dust — outdoor work suspended.",
        ],
        "solar_flare_warning": [
            "Solar observatory reports elevated flare activity — crews advised to stay near shelter.",
        ],
    },
    "daily.survey": {
        "mineral_deposit_found": [
            "Survey team reports a promising mineral deposit in sector {day}.",
            "Geological scan reveals subsurface ore concentration.",
        ],
        "geological_survey_complete": [
            "Geological survey of the surrounding region is complete.",
        ],
    },
    "daily.crew": {
        "crew_morale_high": [
            "Crew spirits are high today.",
            "Good morale among the work teams.",
        ],
        "minor_injury_reported": [
            "Minor injury reported during routine maintenance — treated on site.",
        ],
        "equipment_malfunction": [
            "Equipment malfunction in the workshop — repairs underway.",
            "A faulty relay caused a brief power interruption in hab module 2.",
        ],
    },
    "daily.construction": {
        "foundation_work_continues": [
            "Foundation work continues on schedule.",
        ],
        "structural_milestone": [
            "Structural framework for the new module is taking shape.",
            "Construction crew completes another section of the outer hull.",
        ],
    },
    "daily.power": {
        "solar_panel_output_nominal": [
            "Solar array output is nominal.",
        ],
        "power_grid_fluctuation": [
            "Minor fluctuation detected in the power grid — engineering is monitoring.",
        ],
    },
    "daily.life_support": {
        "oxygen_levels_stable": [
            "Oxygen levels stable across all habitation modules.",
        ],
        "water_recycler_maintenance": [
            "Water recycler undergoing scheduled maintenance cycle.",
        ],
    },

    # Auto-pause
    "auto_pause.year_end": "⏸ Year {year} complete — simulation paused.",

    # Engine errors
    "engine.error": "⚠ Simulation error: {error}",

    # ── Loadout Phase ─────────────────────────────────────────────
    "ship.loadout_complete": [
        "⏸ {ship_name} is fully provisioned with {population:,} colonists and {notable_count} mission officers. Departure imminent.",
        "⏸ Loadout complete. {ship_name} — {population:,} souls aboard, {notable_count} named officers. The stars await.",
        "⏸ {ship_name} clears final inspection. {population:,} colonists embarked, crew of {notable_count} officers at their stations.",
    ],
    "notable.introduced": [
        "{name} ({role}) — {trait_list}.",
    ],
}


def register_template(event_type: str, template) -> None:
    """Register a narrative template for an event type."""
    _TEMPLATES[event_type] = template


def render(event: GameEvent) -> str | None:
    """Render an event into narrative text using templates.

    Supports:
    - Simple string templates: "Year {year} begins"
    - List of variants: randomly chosen
    - Dict of variant → template(s): keyed by event.data["variant"]
    """
    template = _TEMPLATES.get(event.event_type)
    if template is None:
        return None

    # Build substitution context
    ctx: dict[str, object] = {
        "game_time": str(event.game_time),
        "year": event.game_time.year,
        "day": event.game_time.day_of_year,
        "month": event.game_time.month,
        "day_of_month": event.game_time.day_of_month,
        "day_offset": event.game_time.day_offset,
        **event.data,
    }

    # Resolve dict-of-variants
    if isinstance(template, dict):
        variant = event.data.get("variant", "")
        template = template.get(variant)
        if template is None:
            return None

    # Resolve list to a random choice
    if isinstance(template, list):
        template = _random.choice(template)

    try:
        return template.format_map(ctx)
    except (KeyError, ValueError):
        return template
