"""System base class — systems run each tick and produce GameEvents.

Each system declares:
- order: execution priority (lower = runs first)
- active_phases: set of phases where this system runs (None = always)
"""

from __future__ import annotations

import enum
from abc import ABC, abstractmethod

from . import GameEvent, GameTime
from .ecs import World


class Phase(str, enum.Enum):
    """Colony ship arc phases."""
    LOADOUT = "loadout"
    SEARCH = "search"
    TRANSIT = "transit"
    SURVEY = "survey"
    FOUNDING = "founding"


class System(ABC):
    """Base class for all simulation systems."""

    order: int = 100  # execution priority, lower = earlier
    active_phases: set[Phase] | None = None  # None = always active

    @abstractmethod
    def tick(self, world: World, game_time: GameTime) -> list[GameEvent]:
        """Process one tick. Return events generated this tick."""
        ...

    def should_run(self, current_phase: Phase | None) -> bool:
        """Check if this system should run in the current phase."""
        if self.active_phases is None:
            return True
        if current_phase is None:
            return True
        return current_phase in self.active_phases
