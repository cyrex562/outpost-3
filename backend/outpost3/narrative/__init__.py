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
        "⏸ {ship_name} — loaded, fueled, and crewed. The departure window opens.",
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

    # ── Search phase events ──────────────────────────────────────
    "search.system_discovered": None,  # custom renderer
    "search.evaluation_complete": [
        "Sensor sweep complete. {systems_evaluated} candidate systems identified. Evaluating habitability...",
        "All {systems_evaluated} candidate systems mapped. Analyzing data for destination selection.",
    ],
    "search.destination_selected": None,  # custom renderer

    # ── Transit phase events ─────────────────────────────────────
    "transit.phase_change": {
        "accelerating": [
            "Main engines firing — the ship accelerates toward cruise velocity.",
            "The burn begins. Acceleration pushes the crew back in their couches.",
            "Engine ignition confirmed. The stars begin to shift.",
        ],
        "cruising": [
            "Acceleration complete. Cruising at relativistic velocity.",
            "The ship has reached cruise speed. The long journey continues.",
            "Engines throttle down to standby. Cruise velocity achieved — years of silence ahead.",
        ],
        "decelerating": [
            "Deceleration burn initiated — the destination system grows closer.",
            "Engines reversed. Beginning approach to target system.",
            "The long coast is over. Retro-burn commences as a faint star grows brighter.",
        ],
    },
    "transit.yearly_summary": None,  # custom renderer
    "transit.arrival": None,  # custom renderer

    # ── Transit random events ────────────────────────────────────
    "transit.event.asteroid_field": [
        "The ship passes through an unexpected asteroid field. Hull takes minor damage ({hull_damage}%) "
        "and evasive maneuvers cost {fuel_cost} fuel.",
        "Proximity alerts sound as debris impacts the hull ({hull_damage}% damage). "
        "Navigation burns through {fuel_cost} extra fuel avoiding the worst of it.",
        "Debris field contact — {hull_damage}% hull damage sustained. {fuel_cost} fuel spent on evasive burns.",
    ],
    "transit.event.equipment_failure": [
        "Critical failure in {system} systems — {damage}% degradation. Engineering crews scramble to contain the damage.",
        "A cascade failure drops {system} performance by {damage}%. Repair teams dispatched.",
        "Warning klaxon in {system} compartment — {damage}% capacity lost before the breach is sealed.",
    ],
    "transit.event.cosmic_phenomenon": ["{flavor}"],
    "transit.event.resource_find": [
        "Long-range sensors detect a resource-rich asteroid. A brief diversion yields +{gain} {resource_type}.",
        "Opportunistic mining of a passing body adds {gain} units of {resource_type} to stores.",
    ],
    "transit.event.crew_conflict": [
        "Tensions between crew factions boil over into a heated dispute. Morale drops {morale_change}%.",
        "A disagreement over resource allocation leads to unrest. Crew morale affected ({morale_change}%).",
        "Factions form around a divisive issue. Arguments turn bitter before cooler heads prevail. Morale: {morale_change}%.",
    ],
    "transit.event.celebration": [
        "The crew holds a celebration marking another year of the journey. Spirits lift (+{morale_change}% morale).",
        "A festival day brings the community together. Morale improves by {morale_change}%.",
        "Music and laughter echo through the hab decks. For a night, the void feels less empty. Morale +{morale_change}%.",
    ],
    "transit.event.cultural_shift": ["{flavor}"],

    # ── Ship maintenance events ──────────────────────────────────
    "maintenance.breakdown": [
        "Breakdown in {system} — {damage}% damage. Current health: {health}%.",
        "Unexpected failure in {system} systems ({damage}% degradation). Health now at {health}%.",
        "{system} reliability drops another notch — {damage}% damage, {health}% remaining.",
    ],

    # ── Population events ────────────────────────────────────────
    "population.yearly_report": None,  # custom renderer
    "notable.death": None,  # custom renderer
    "notable.succession": None,  # custom renderer

    # ── Survey phase events ──────────────────────────────────────
    "survey.begin": None,  # custom renderer
    "survey.planet_survey_begin": [
        "Commencing survey of planet {planet_index}/{total_planets}. Estimated duration: {survey_days} days.",
        "Survey probes deployed to planet {planet_index} of {total_planets}. Expected completion in {survey_days} days.",
        "Orbital insertion complete. Beginning detailed scan of planet {planet_index}/{total_planets} ({survey_days} days).",
    ],
    "survey.planet_report": None,  # custom renderer
    "survey.system_accepted": None,  # custom renderer
    "survey.system_rejected": None,  # custom renderer

    # ── Founding phase events ────────────────────────────────────
    "founding.landing_site_selected": None,  # custom renderer
    "founding.descent_complete": [
        "Touchdown confirmed. {population} colonists set foot on alien soil for the first time.",
        "The landers are down. {population} souls step out into the light of a new sun.",
        "Descent complete. {population} people stand on the surface, looking up at unfamiliar stars.",
        "Landing successful. {population} colonists breathe open air for the first time in years.",
    ],
    "founding.first_structures": [
        "First hab modules assembled from {materials_remaining} tons of materials and {salvaged_equipment} tons of salvaged ship equipment.",
        "The first structures rise from the surface — shelter, at last. Materials: {materials_remaining}t. Salvaged gear: {salvaged_equipment}t.",
        "Prefab shelters deployed. {materials_remaining}t of building materials remain. {salvaged_equipment}t stripped from the ship.",
    ],
    "founding.colony_established": None,  # custom renderer
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


def _render_system_discovered(event: GameEvent) -> str:
    d = event.data
    name = d.get("system_name", "Unknown")
    dist = d.get("distance_ly", 0)
    n_planets = d.get("num_planets", 0)
    hab = d.get("habitability_score", 0)
    num = d.get("discovery_number", "?")
    target = d.get("target_discoveries", "?")
    planets = d.get("planets", [])

    lines = [
        f"Star system discovered: {name} ({dist} ly, {n_planets} planets, "
        f"habitability: {hab:.2f}) [{num}/{target}]"
    ]
    for p in planets:
        h = p.get("habitability", 0)
        marker = " ★" if h > 0.5 else ""
        lines.append(
            f"  · {p.get('name', '?')} — {p.get('type', '?')}, "
            f"size {p.get('size', '?')}R⊕, hab: {h:.2f}{marker}"
        )
    return "\n".join(lines)


def _render_destination_selected(event: GameEvent) -> str:
    d = event.data
    templates = [
        f"⏸ Destination selected: {d.get('system_name', '?')} "
        f"({d.get('distance_ly', '?')} ly, habitability {d.get('habitability_score', 0):.2f}). "
        f"Transit at {d.get('cruise_velocity_c', '?')}c — estimated {d.get('transit_years', '?')} years.",
        f"⏸ Course set for {d.get('system_name', '?')}. "
        f"Distance: {d.get('distance_ly', '?')} light-years. "
        f"ETA: {d.get('transit_years', '?')} years at {d.get('cruise_velocity_c', '?')}c.",
    ]
    return _random.choice(templates)


def _render_transit_yearly_summary(event: GameEvent) -> str:
    d = event.data
    return (
        f"Transit year {d.get('years_in_transit', '?')}: "
        f"{d.get('pct_complete', '?')}% complete "
        f"({d.get('distance_traveled_ly', '?')}/{d.get('distance_total_ly', '?')} ly). "
        f"Population: {d.get('population', '?')}. "
        f"Fuel: {d.get('fuel_remaining', '?')}. Food: {d.get('food_remaining', '?')}. "
        f"Phase: {d.get('transit_phase', '?')}."
    )


def _render_transit_arrival(event: GameEvent) -> str:
    d = event.data
    templates = [
        f"⏸ The ship arrives at {d.get('system_name', '?')} after "
        f"{d.get('transit_years', '?')} years. Population: {d.get('population', '?')}. "
        f"Fuel remaining: {d.get('fuel_remaining', '?')}.",
        f"⏸ Journey's end — {d.get('system_name', '?')} fills the viewports. "
        f"{d.get('transit_years', '?')} years of travel. "
        f"{d.get('population', '?')} souls have reached a new star.",
    ]
    return _random.choice(templates)


def _render_population_yearly(event: GameEvent) -> str:
    d = event.data
    net = d.get("net_change", 0)
    direction = "grew" if net > 0 else "shrank" if net < 0 else "held steady"
    return (
        f"Population {direction}: {d.get('total', '?')} "
        f"(+{d.get('births', 0)} births, -{d.get('deaths', 0)} deaths). "
        f"Morale: {int(d.get('morale', 0) * 100)}%."
    )


def _render_notable_death(event: GameEvent) -> str:
    d = event.data
    templates = [
        f"{d.get('name', '?')}, {d.get('role', '?')}, has died at age {d.get('age', '?')}.",
        f"The ship mourns the loss of {d.get('name', '?')} ({d.get('role', '?')}), "
        f"who passed at {d.get('age', '?')} years old.",
        f"{d.get('role', '?')} {d.get('name', '?')} is gone. Age {d.get('age', '?')}. "
        f"The crew gathers to pay their respects.",
    ]
    return _random.choice(templates)


def _render_notable_succession(event: GameEvent) -> str:
    d = event.data
    traits = ", ".join(d.get("traits", []))
    templates = [
        f"{d.get('name', '?')} promoted to {d.get('role', '?')} (age {d.get('age', '?')}). "
        f"Known for being {traits}.",
        f"New {d.get('role', '?')}: {d.get('name', '?')}, age {d.get('age', '?')}. "
        f"Traits: {traits}.",
    ]
    return _random.choice(templates)


def _render_transit_phase_change(event: GameEvent) -> str:
    phase = event.data.get("transit_phase", "unknown")
    days = event.data.get("days_in_transit", 0)
    years = round(days / 365, 1)

    templates = {
        "cruising": [
            f"Acceleration complete after {days} days. The ship settles into cruise velocity.",
            f"Main engines throttle down — cruising speed achieved. Day {days} of transit.",
        ],
        "decelerating": [
            f"Deceleration burn initiated at year {years} of transit. The destination nears.",
            f"After {years} years of cruising, the engines fire in reverse. Approach phase begins.",
        ],
    }
    options = templates.get(phase, [f"Transit phase: {phase} (day {days})."])
    return _random.choice(options)


def _render_landing_site_selected(event: GameEvent) -> str:
    d = event.data
    hab = d.get("habitability", 0)
    quality = "promising" if hab > 0.6 else "adequate" if hab > 0.35 else "challenging"
    templates = [
        f"⏸ Landing site selected: {d.get('planet_name', '?')} in the {d.get('system_name', '?')} system. "
        f"Habitability {hab:.2f} — a {quality} new home. Preparing descent sequence.",
        f"⏸ {d.get('planet_name', '?')} has been chosen. "
        f"A {quality} world (hab: {hab:.2f}) in {d.get('system_name', '?')}. "
        f"The ship descends for the last time.",
    ]
    return _random.choice(templates)


def _render_colony_established(event: GameEvent) -> str:
    d = event.data
    condition = d.get("condition", "moderate")
    colony = d.get("colony_name", "the colony")
    pop = d.get("population", 0)
    years = d.get("journey_years", "?")
    rejections = d.get("rejections", 0)
    notables_alive = d.get("notables_alive", 0)
    notables_total = d.get("notables_total", 0)
    details = d.get("condition_details", [])
    morale = d.get("morale", 0)

    lines = [f"⏸ COLONY FOUNDED: {colony}"]
    lines.append(f"")
    lines.append(f"After {years} years, {pop} colonists have a new home.")

    if rejections > 0:
        lines.append(f"{rejections} star system(s) were rejected before this one was chosen.")

    lines.append(f"")

    # Condition assessment
    condition_labels = {
        "strong": "The colony begins from a position of strength.",
        "moderate": "The colony has a reasonable start, though not without challenges.",
        "weakened": "The journey has taken its toll. The colony starts diminished.",
        "desperate": "Against all odds, they've made it — but barely. A desperate founding.",
    }
    lines.append(condition_labels.get(condition, ""))

    for detail in details:
        lines.append(f"  · {detail}")

    lines.append(f"")
    lines.append(f"Population: {pop} | Morale: {int(morale * 100)}%")
    lines.append(f"Notables surviving: {notables_alive}/{notables_total}")

    res = d.get("resources", {})
    if res:
        res_parts = [f"{k}: {v}" for k, v in res.items()]
        lines.append(f"Stores: {', '.join(res_parts)}")

    lines.append(f"")

    # Closing flavor
    closings = {
        "strong": [
            "The future looks bright for the people of " + colony + ".",
            "A new chapter begins — and this one starts well.",
        ],
        "moderate": [
            "It won't be easy, but " + colony + " has what it needs to survive.",
            "The hard part is over. The harder part begins.",
        ],
        "weakened": [
            "They've survived the journey. Now they must survive the world.",
            colony + " will need every ounce of resilience its people have left.",
        ],
        "desperate": [
            "They call it a colony. For now, it's a camp of survivors clinging to hope.",
            "The name " + colony + " is more prayer than declaration. But they're alive.",
        ],
    }
    lines.append(_random.choice(closings.get(condition, ["The colony is founded."])))

    return "\n".join(lines)


def _render_survey_begin(event: GameEvent) -> str:
    d = event.data
    rejections = d.get("rejections_so_far", 0)
    suffix = ""
    if rejections > 0:
        suffix = f" This is attempt #{rejections + 1}."
    templates = [
        f"⏸ Entering the {d.get('system_name', '?')} system. "
        f"{d.get('num_planets', '?')} planets to survey.{suffix}",
        f"⏸ Survey operations beginning in {d.get('system_name', '?')}. "
        f"Probes deploying toward {d.get('num_planets', '?')} worlds.{suffix}",
    ]
    return _random.choice(templates)


def _render_planet_report(event: GameEvent) -> str:
    d = event.data
    initial = d.get("initial_hab", 0)
    refined = d.get("refined_hab", 0)
    change = refined - initial
    change_str = f"+{change:.2f}" if change >= 0 else f"{change:.2f}"

    lines = [
        f"SURVEY REPORT: {d.get('planet_name', '?')} "
        f"({d.get('planet_type', '?')}, {d.get('size', '?')}R⊕) "
        f"[{d.get('planet_number', '?')}/{d.get('total_planets', '?')}]",
        f"  Atmosphere: {d.get('atmosphere', '?')}",
        f"  Water: {d.get('water', '?')}",
        f"  Minerals: {d.get('minerals', '?')}",
        f"  Hazards: {d.get('hazards', '?')}",
        f"  Seasons: {d.get('seasons', '?')}",
        f"  Habitability: {initial:.2f} → {refined:.2f} ({change_str})",
    ]
    return "\n".join(lines)


def _render_system_accepted(event: GameEvent) -> str:
    d = event.data
    rejections = d.get("rejections", 0)
    journey_note = ""
    if rejections > 0:
        journey_note = f" After {rejections} rejected system(s), this is the one."

    templates = [
        f"⏸ System accepted: {d.get('system_name', '?')}. "
        f"Best candidate: {d.get('best_planet', '?')} "
        f"(habitability {d.get('best_hab', 0):.2f}, threshold {d.get('threshold', 0):.2f}).{journey_note}",
        f"⏸ This is the one. {d.get('best_planet', '?')} in the {d.get('system_name', '?')} system "
        f"meets our requirements (hab: {d.get('best_hab', 0):.2f}).{journey_note}",
    ]
    return _random.choice(templates)


def _render_system_rejected(event: GameEvent) -> str:
    d = event.data
    remaining = d.get("max_rejections", 3) - d.get("rejections", 0)
    templates = [
        f"⏸ System rejected: {d.get('system_name', '?')}. "
        f"Best planet {d.get('best_planet', '?')} scored only {d.get('best_hab', 0):.2f} "
        f"(threshold: {d.get('threshold', 0):.2f}). "
        f"Resuming search. {remaining} rejection(s) remaining before forced acceptance.",
        f"⏸ Not good enough. {d.get('system_name', '?')} fails to meet standards "
        f"(best: {d.get('best_hab', 0):.2f}, need: {d.get('threshold', 0):.2f}). "
        f"The search continues. {remaining} chances left.",
    ]
    return _random.choice(templates)


# ── Custom renderer registry ────────────────────────────────────

_CUSTOM_RENDERERS: dict[str, Callable[[GameEvent], str]] = {
    "loadout.notable_introduction": _render_notable_introduction,
    "phase.transition": _render_phase_transition,
    "behavior.deliberation": _render_deliberation,
    "search.system_discovered": _render_system_discovered,
    "search.destination_selected": _render_destination_selected,
    "transit.yearly_summary": _render_transit_yearly_summary,
    "transit.arrival": _render_transit_arrival,
    "transit.phase_change": _render_transit_phase_change,
    "population.yearly_report": _render_population_yearly,
    "notable.death": _render_notable_death,
    "notable.succession": _render_notable_succession,
    "survey.begin": _render_survey_begin,
    "survey.planet_report": _render_planet_report,
    "survey.system_accepted": _render_system_accepted,
    "survey.system_rejected": _render_system_rejected,
    "founding.landing_site_selected": _render_landing_site_selected,
    "founding.colony_established": _render_colony_established,
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
