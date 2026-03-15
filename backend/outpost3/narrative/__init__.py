"""Narrative template renderer — turns structured events into readable text.

For Milestone 0 this is minimal. Milestone 1 will add ~30 templates for the
colony ship arc.
"""

from __future__ import annotations

from ..simulation import GameEvent


# Template registry: event_type → format string
# Templates can reference any key in event.data plus {game_time}
_TEMPLATES: dict[str, str | list[str]] = {
    "time.year_start": "— Year {year} begins —",
    "time.month_start": "Month {month} of Year {year}.",
    "engine.error": "⚠ Simulation error: {error}",
    "auto_pause.placeholder": "⏸ Simulation paused at {game_time}.",
}


def register_template(event_type: str, template: str | list[str]) -> None:
    """Register a narrative template for an event type.

    If a list is provided, one is chosen at random when rendering.
    """
    _TEMPLATES[event_type] = template


def render(event: GameEvent) -> str | None:
    """Render an event into narrative text using templates.

    Returns None if no template is registered for this event type.
    """
    template = _TEMPLATES.get(event.event_type)
    if template is None:
        return None

    if isinstance(template, list):
        import random
        template = random.choice(template)

    # Build substitution context from event data + game_time fields
    ctx: dict[str, object] = {
        "game_time": str(event.game_time),
        "year": event.game_time.year,
        "day": event.game_time.day_of_year,
        "day_offset": event.game_time.day_offset,
        **event.data,
    }

    try:
        return template.format_map(ctx)
    except (KeyError, ValueError):
        # Fallback: just return the raw template
        return template
