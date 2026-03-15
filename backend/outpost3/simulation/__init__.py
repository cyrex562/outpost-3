"""Event system — events are structured data, narrative is a view."""

from __future__ import annotations

import enum
from dataclasses import dataclass, field
from typing import Any

DAYS_PER_YEAR = 365
DAYS_PER_MONTH = 30  # ~12 months per year (last month is 35 days)
MONTHS_PER_YEAR = 12


class Severity(str, enum.Enum):
    DEBUG = "debug"
    INFO = "info"
    NOTABLE = "notable"
    CRITICAL = "critical"
    MILESTONE = "milestone"


@dataclass(frozen=True)
class GameTime:
    """Game time measured in days since start. Year/day/month derived."""

    day_offset: int = 0  # total days since simulation start

    @property
    def year(self) -> int:
        return self.day_offset // DAYS_PER_YEAR + 1

    @property
    def day_of_year(self) -> int:
        return self.day_offset % DAYS_PER_YEAR + 1

    @property
    def month(self) -> int:
        """Month of the year (1-12)."""
        m = (self.day_of_year - 1) // DAYS_PER_MONTH + 1
        return min(m, MONTHS_PER_YEAR)

    @property
    def day_of_month(self) -> int:
        return (self.day_of_year - 1) % DAYS_PER_MONTH + 1

    @property
    def is_year_start(self) -> bool:
        return self.day_of_year == 1

    @property
    def is_month_start(self) -> bool:
        return self.day_of_month == 1

    def to_dict(self) -> dict[str, int]:
        return {
            "day_offset": self.day_offset,
            "year": self.year,
            "day": self.day_of_year,
            "month": self.month,
            "day_of_month": self.day_of_month,
        }

    def __str__(self) -> str:
        return f"Year {self.year}, Month {self.month}, Day {self.day_of_month}"


@dataclass
class GameEvent:
    """A single simulation event. Events are data — display text is rendered
    separately by the narrative layer."""

    event_type: str
    severity: Severity
    game_time: GameTime
    data: dict[str, Any] = field(default_factory=dict)
    auto_pause: bool = False
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
