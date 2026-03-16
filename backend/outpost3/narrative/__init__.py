"""Narrative template renderer — turns structured events into readable text.

Supports simple format strings, variant lists, dict-keyed variants,
and custom render functions for complex event types like deliberations.
"""

from __future__ import annotations

import random as _random
from typing import Callable

from ..simulation import GameEvent


# Template registry: event_type → format string, list of variants, dict, or callable
_TEMPLATES: dict[str, str | list[str] | dict | Callable] = {
    # ── Time markers ─────────────────────────────────────────────
    "time.year_start": "— Year {year} begins —",
    "time.month_start": "Month {month} of Year {year}.",

    # ── M0 daily flavor — keyed by variant ───────────────────────
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

    # ── Auto-pause ───────────────────────────────────────────────
    "auto_pause.year_end": "⏸ Year {year} complete — simulation paused.",

    # ── Engine errors ────────────────────────────────────────────
    "engine.error": "⚠ Simulation error: {error}",

    # ── Loadout events ───────────────────────────────────────────
    "loadout.ship_commissioned": [
        "⏸ The {ship_name} has been commissioned. Loadout complete.",
        "⏸ Colony ship {ship_name} stands ready. Reviewing manifest...",
    ],

    "loadout.summary": [
        "The {ship_name} carries {crew_size} colonists with supplies for the journey ahead. "
        "Ship systems are in good condition. {num_notables} senior officers will lead the expedition.",
        "{ship_name} departs with {crew_size} souls aboard. Stores are provisioned, "
        "systems are green. {num_notables} Notable individuals form the command structure.",
    ],

    "loadout.crew_manifest": [
        "Crew manifest: {total} personnel — {scientists} scientists, {engineers} engineers, "
        "{medical} medical staff, {military} security, {civilians} civilians.",
        "Final headcount: {total} colonists. Sciences division: {scientists}. "
        "Engineering: {engineers}. Medical: {medical}. Security: {military}. Civilians: {civilians}.",
    ],

    "loadout.notable_introduction": None,  # uses custom renderer

    # ── Phase transitions ────────────────────────────────────────
    "phase.transition": None,  # uses custom renderer

    # ── Behavior deliberations ───────────────────────────────────
    "behavior.deliberation": None,  # uses custom renderer
}


# ── Custom render functions ──────────────────────────────────────

def _render_notable_introduction(event: GameEvent) -> str:
    data = event.data
    name = data.get("name", "Unknown")
    role = data.get("role", "Unknown")
    age = data.get("age", "?")
    traits = data.get("traits", [])

    trait_desc = ", ".join(traits) if traits else "no notable traits"

    templates = [
        f"{name}, {role} (age {age}) — {trait_desc}.",
        f"Assigned: {name} as {role}. Age {age}. Known for being {trait_desc}.",
        f"{role} {name}, {age} years old. Described as {trait_desc}.",
    ]
    return _random.choice(templates)


def _render_phase_transition(event: GameEvent) -> str:
    from_phase = event.data.get("from", "unknown")
    to_phase = event.data.get("to", "unknown")

    transitions = {
        ("loadout", "search"): [
            "— Ship loadout complete. Beginning search for candidate star systems. —",
            "— Manifest sealed. Scanning for suitable destinations... —",
        ],
        ("search", "transit"): [
            "— Destination selected. Engaging engines for interstellar transit. —",
            "— Course plotted. The long journey begins. —",
        ],
        ("transit", "survey"): [
            "— Arrival. Beginning survey of the target system. —",
            "— Deceleration complete. Commencing planetary surveys. —",
        ],
        ("survey", "founding"): [
            "— Landing site selected. Colony founding underway. —",
            "— The journey is over. A new world awaits. —",
        ],
        ("survey", "search"): [
            "— System rejected. Resuming search for a new destination. —",
            "— Not suitable. The search continues. —",
        ],
    }

    key = (from_phase, to_phase)
    if key in transitions:
        return _random.choice(transitions[key])
    return f"— Phase transition: {from_phase} → {to_phase} —"


def _render_deliberation(event: GameEvent) -> str:
    delib = event.data.get("deliberation", {})
    trigger = delib.get("trigger_detail", delib.get("trigger", "Unknown condition"))
    options = delib.get("options", [])
    chosen = delib.get("chosen")

    lines = [f"DELIBERATION: {trigger}"]
    lines.append("Options evaluated:")

    for i, opt in enumerate(options, 1):
        action = opt.get("description", opt.get("action", "?"))
        score = opt.get("score", 0)
        is_chosen = chosen and opt.get("action") == chosen.get("action")
        marker = "→" if is_chosen else " "

        lines.append(f"  {marker} {i}. {action} [score: {score:.2f}]")

        for pro in opt.get("pros", []):
            lines.append(f"      + {pro}")
        for con in opt.get("cons", []):
            lines.append(f"      − {con}")

        advocate = opt.get("advocate")
        advocate_trait = opt.get("advocate_trait")
        if advocate:
            lines.append(f"      Advocated by: {advocate} ({advocate_trait})")

    if chosen:
        lines.append(f"Decision: {chosen.get('description', chosen.get('action', '?'))} (highest utility)")

    return "\n".join(lines)


# ── Custom renderer registry ────────────────────────────────────

_CUSTOM_RENDERERS: dict[str, Callable[[GameEvent], str]] = {
    "loadout.notable_introduction": _render_notable_introduction,
    "phase.transition": _render_phase_transition,
    "behavior.deliberation": _render_deliberation,
}


def register_template(event_type: str, template) -> None:
    """Register a narrative template for an event type."""
    _TEMPLATES[event_type] = template


def register_renderer(event_type: str, renderer: Callable[[GameEvent], str]) -> None:
    """Register a custom render function for an event type."""
    _CUSTOM_RENDERERS[event_type] = renderer


def render(event: GameEvent) -> str | None:
    """Render an event into narrative text using templates or custom renderers.

    Supports:
    - Simple string templates: "Year {year} begins"
    - List of variants: randomly chosen
    - Dict of variant → template(s): keyed by event.data["variant"]
    - Custom render functions for complex event types
    """
    # Check custom renderers first
    custom = _CUSTOM_RENDERERS.get(event.event_type)
    if custom is not None:
        try:
            return custom(event)
        except Exception:
            return None

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
