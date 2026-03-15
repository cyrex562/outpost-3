"""Event system — events are structured data, narrative is a view."""

from __future__ import annotations

import enum
from dataclasses import dataclass, field
from typing import Any


class Severity(str, enum.Enum):
    DEBUG = "debug"
    INFO = "info"
    NOTABLE = "notable"
    CRITICAL = "critical"
    MILESTONE = "milestone"


@dataclass(frozen=True)
class GameTime:
    """Game time measured in days since start. Year/day derived."""

    day_offset: int = 0  # total days since simulation start

    @property
    def year(self) -> int:
        return self.day_offset // 365 + 1

    @property
    def day_of_year(self) -> int:
        return self.day_offset % 365 + 1

    def to_dict(self) -> dict[str, int]:
        return {
            "day_offset": self.day_offset,
            "year": self.year,
            "day": self.day_of_year,
        }

    def __str__(self) -> str:
        return f"Year {self.year}, Day {self.day_of_year}"


@dataclass
class GameEvent:
    """A single simulation event. Events are data — display text is rendered
    separately by the narrative layer."""

    event_type: str
    severity: Severity
    game_time: GameTime
    data: dict[str, Any] = field(default_factory=dict)
    # If true, the auto-pause system should trigger on this event
    auto_pause: bool = False
    # Rendered narrative text (filled in by the narrative layer)
    text: str | None = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "event_type": self.event_type,
            "severity": self.severity.value,
            "game_time": self.game_time.to_dict(),
            "data": self.data,
            "auto_pause": self.auto_pause,
            "text": self.text,
        }
