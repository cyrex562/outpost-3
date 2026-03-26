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

    # ── Search Phase ──────────────────────────────────────────────
    "search.candidate_found": [
        "Long-range survey confirms {name} ({star_type}), {distance_ly} light-years out. {planet_count} worlds detected — best candidate {best_planet} at {best_habitability:.0f}% habitability.",
        "{name}: {star_type} star, {distance_ly} ly distant. {planet_count} planetary bodies. Peak habitability {best_habitability:.0f}%.",
        "Navigation logs {name} ({star_type}) at {distance_ly} light-years. Sensor sweep shows {planet_count} worlds; {best_planet} registers {best_habitability:.0f}% habitability.",
    ],
    "search.destination_selected": [
        "⏸ Navigation committee selects {name} as primary destination. Estimated transit: {transit_years} years. {candidates_rejected} alternative system(s) set aside.",
        "⏸ Course plotted to {name} ({star_type}), {distance_ly} light-years distant. Transit time: {transit_years} years. Best habitability: {best_habitability:.0f}%.",
        "⏸ Destination confirmed: {name}. A {transit_years}-year voyage lies ahead. {candidates_rejected} candidate(s) were considered and rejected.",
    ],

    # ── Transit Phase ─────────────────────────────────────────────
    "transit.year_summary": [
        "Transit year {transit_year}: {population:,} colonists aboard. Food reserves for {food_years:.1f} more years. Destination {years_remaining:.1f} years out. Hull at {hull_integrity:.0f}%.",
        "Year {transit_year} of transit. Pop: {population:,} (+{births} births, -{deaths} deaths). {years_remaining:.1f} years remaining. Food: {food_years:.1f} yr.",
    ],
    "transit.batch_summary": [
        "Transit update — {population:,} colonists, {years_remaining:.1f} years to destination. Food reserves: {food_years:.1f} years. Hull: {hull_integrity:.0f}%.",
    ],
    "transit.food_critical": [
        "⏸ Food reserves critical — {food_years:.2f} years remaining for {population:,} colonists. Rationing protocols initiated.",
    ],
    "transit.medicine_low": [
        "Medical stores running low — {medicine} units remain for {population:,} colonists.",
    ],
    "transit.hull_warning": [
        "⏸ Hull integrity at {hull_integrity:.0f}% — structural engineers report accelerating microcrack propagation. Emergency repair teams deployed.",
    ],
    "transit.arrival": [
        "⏸ {destination} system acquired on long-range sensors. After {transit_years} years of transit, {population:,} colonists approach their new home. Hull integrity: {hull_integrity:.0f}%.",
        "⏸ Transit complete. {destination} fills the forward viewports. {population:,} souls survived the {transit_years}-year journey. Survey operations begin.",
    ],
    "transit.equipment_failure": [
        "Equipment failure in engineering section — {parts_cost} spare parts consumed in repairs.",
        "Malfunction reported in drive systems. Repair crews respond; {parts_cost} components replaced.",
        "Critical relay failure in habitat module 4. Resolved in {parts_cost} parts. No casualties.",
    ],
    "transit.medical_emergency": [
        "Medical emergency aboard — {medicine_cost} treatment units expended. Patient stabilised.",
        "Outbreak of respiratory illness in crew quarters. Medical team responds, {medicine_cost} medicine units consumed.",
        "Emergency surgery required. {medicine_cost} units of medical stores used. Prognosis good.",
    ],
    "transit.hull_microcrack": [
        "Micrometeorite impact detected — {parts_cost} parts used to seal minor hull breach.",
        "Hull sensor reports new stress fracture, sector 7. Repair team dispatched; {parts_cost} components used.",
        "Routine hull inspection finds hairline crack in outer hull panel. Patched with {parts_cost} parts.",
    ],
    "transit.resource_leak": [
        "Pipe rupture in storage bay — {food_cost} food units and {water_cost} water units lost before containment.",
        "Storage tank seal failure. {food_cost} food and {water_cost} water units vented before repairs completed.",
    ],
    "transit.crew_conflict": [
        "Dispute among colonists in habitat ring B — social council intervenes. Situation resolved.",
        "Tension in crew quarters over work rotation schedules. Leadership meets; compromise reached.",
        "Altercation in the mess hall. Security called; minor injuries, no lasting harm.",
    ],
    "transit.scientific_observation": [
        "Astronomy team records unusual pulsar timing anomaly — logged for post-arrival analysis.",
        "Telescope array captures close-approach of rogue planetoid. Samples impossible — logged.",
        "Cosmic ray flux measurement exceeds background model. Shielding adjusted; no crew exposure.",
    ],
    "transit.morale_high": [
        "Morale survey across crew quarters returns positive results. The journey continues in good spirit.",
        "Children born during transit hold first organised school assembly — a heartening milestone.",
        "Crew celebrates a decade of transit with a communal gathering in the central atrium.",
    ],
    "transit.system_anomaly": [
        "Navigation computer flags an unexpected gravitational perturbation — course adjusted, {parts_cost} components recalibrated.",
        "Sensor array detects uncharted debris field — trajectory modified, minor {parts_cost}-unit equipment cost.",
    ],

    # ── Survey Phase ──────────────────────────────────────────────
    "survey.started": [
        "Entering {system_name}. Survey operations begin — {planet_count} world(s) to assess.",
        "{system_name} system acquired. Sensor arrays active; {planet_count} planetary body/bodies detected.",
        "Long-range transit complete. Survey teams mobilised across {planet_count} world(s) in the {system_name} system.",
    ],
    "survey.planet_scanned": [
        "World {scan_number}/{total_planets}: {planet_name} — {planet_type}. Habitability {habitability:.0f}%. {notes_text}",
        "Scan complete on {planet_name} ({planet_type}). Habitability index {habitability:.0f}%. {notes_text}",
    ],
    "survey.batch_scan": [
        "Survey team completes assessment of {planets_scanned} world(s) in {system_name}. Peak habitability: {best_habitability:.0f}%.",
    ],
    "survey.rejected": [
        "⏸ {system_name} assessed at {habitability:.0f}% peak habitability — below threshold. Crew votes to continue the search. ({rejections_remaining} rejection(s) remaining.)",
        "⏸ Navigation committee rejects {system_name} ({habitability:.0f}% habitability). A new search begins. {rejections_remaining} attempt(s) remain before forced settlement.",
    ],

    # ── Deliberation events ───────────────────────────────────────
    # These render a short summary; the full record is in event.data for the UI.
    "deliberation.complete": [
        "Crew deliberation: {trigger_label}. After debate, the committee selects: {chosen_label}. {effect_summary}",
        "Officers convene on {trigger_label}. Decision reached: {chosen_label}. {effect_summary}",
        "Ship council addresses {trigger_label}. Chosen course: {chosen_label}. {effect_summary}",
    ],

    # ── Founding Phase ────────────────────────────────────────────
    "colony.founded": [
        "⏸ Colony established on {planet_name}, {system_name} system. {population:,} colonists make landfall. Habitability: {habitability:.0f}%. Colony status: {quality}.",
        "⏸ {population:,} colonists land on {planet_name}. Year {founded_year}. Hull integrity at touchdown: {hull_integrity:.0f}%. Colony assessment: {quality}.",
        "⏸ After {founded_year} years, {population:,} souls found a new home on {planet_name}. The {quality} colony takes its first steps under an alien sky.",
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
