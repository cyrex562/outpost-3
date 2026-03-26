"""System base class for the ECS-Lite architecture.

Systems contain logic; components contain data. Each system implements
tick() for per-day simulation and/or batch() for high-speed summary.

Registration (M1 Session 2+):
    world = World()
    engine.on_tick(MySystem(world).tick)
    engine.on_batch(MySystem(world).batch)
"""

from __future__ import annotations

from ..simulation import GameEvent, GameTime
from .world import World


class System:
    """Base class for all simulation systems.

    Subclasses override tick() and/or batch(). Both return a list of
    GameEvents to be broadcast to clients.
    """

    def __init__(self, world: World) -> None:
        self.world = world

    async def tick(self, game_time: GameTime) -> list[GameEvent]:
        """Called once per simulated day at low speeds (< BATCH_THRESHOLD)."""
        return []

    async def batch(
        self,
        start_time: GameTime,
        end_time: GameTime,
        num_days: int,
    ) -> list[GameEvent]:
        """Called once per cycle at high speeds with a day range summary."""
        return []
