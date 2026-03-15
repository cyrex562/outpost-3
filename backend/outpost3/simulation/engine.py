"""Time engine — async tick loop with speed control and auto-pause."""

from __future__ import annotations

import asyncio
from typing import Callable, Awaitable

from . import GameEvent, GameTime, Severity

# Maps speed multiplier → seconds between ticks (real time)
# At 1x: 1 tick/sec. At max: as fast as possible (0 delay).
SPEED_DELAYS: dict[int, float] = {
    1: 1.0,
    5: 0.2,
    25: 0.04,
    100: 0.01,
    0: 0.0,  # max speed — 0 means no sleep, just yield
}

VALID_SPEEDS = [1, 5, 25, 100, 0]  # 0 = max


class TimeEngine:
    """Drives the simulation clock forward, emitting events each tick."""

    def __init__(self) -> None:
        self.day: int = 0  # total days elapsed
        self.speed: int = 1
        self.paused: bool = True
        self._running: bool = False
        self._task: asyncio.Task | None = None

        # Callbacks invoked each tick and on events
        self._tick_handlers: list[Callable[[GameTime], Awaitable[list[GameEvent]]]] = []
        self._event_listeners: list[Callable[[GameEvent], Awaitable[None]]] = []
        self._state_listeners: list[Callable[[], Awaitable[None]]] = []

    # ── registration ──────────────────────────────────────────────

    def on_tick(self, handler: Callable[[GameTime], Awaitable[list[GameEvent]]]) -> None:
        """Register an async handler called each tick. Should return events."""
        self._tick_handlers.append(handler)

    def on_event(self, listener: Callable[[GameEvent], Awaitable[None]]) -> None:
        """Register an async listener for every event emitted."""
        self._event_listeners.append(listener)

    def on_state_change(self, listener: Callable[[], Awaitable[None]]) -> None:
        """Register listener for pause/speed/time state changes."""
        self._state_listeners.append(listener)

    # ── control ───────────────────────────────────────────────────

    def set_speed(self, speed: int) -> None:
        if speed not in VALID_SPEEDS:
            return
        self.speed = speed
        asyncio.ensure_future(self._broadcast_state())

    def pause(self) -> None:
        self.paused = True
        asyncio.ensure_future(self._broadcast_state())

    def resume(self) -> None:
        self.paused = False
        asyncio.ensure_future(self._broadcast_state())

    def start(self) -> None:
        """Start the tick loop as a background task."""
        if self._running:
            return
        self._running = True
        self._task = asyncio.create_task(self._loop())

    def stop(self) -> None:
        self._running = False
        if self._task:
            self._task.cancel()
            self._task = None

    # ── state snapshot ────────────────────────────────────────────

    def state_dict(self) -> dict:
        gt = GameTime(self.day)
        return {
            "type": "state",
            "paused": self.paused,
            "speed": self.speed,
            "day": gt.day_of_year,
            "year": gt.year,
            "day_offset": self.day,
        }

    # ── internals ─────────────────────────────────────────────────

    async def _loop(self) -> None:
        while self._running:
            if self.paused:
                await asyncio.sleep(0.05)
                continue

            delay = SPEED_DELAYS.get(self.speed, 1.0)

            # Advance one day
            self.day += 1
            game_time = GameTime(self.day)

            # Collect events from all tick handlers
            events: list[GameEvent] = []
            for handler in self._tick_handlers:
                try:
                    result = await handler(game_time)
                    if result:
                        events.extend(result)
                except Exception as exc:
                    events.append(GameEvent(
                        event_type="engine.error",
                        severity=Severity.CRITICAL,
                        game_time=game_time,
                        data={"error": str(exc)},
                    ))

            # Broadcast events
            should_pause = False
            for event in events:
                for listener in self._event_listeners:
                    try:
                        await listener(event)
                    except Exception:
                        pass
                if event.auto_pause:
                    should_pause = True

            # Auto-pause after broadcasting the event that triggered it
            if should_pause:
                self.paused = True
                await self._broadcast_state()

            # Broadcast state (for tick counter updates)
            await self._broadcast_state()

            if delay > 0:
                await asyncio.sleep(delay)
            else:
                # max speed: yield to event loop so we don't starve it
                await asyncio.sleep(0)

    async def _broadcast_state(self) -> None:
        for listener in self._state_listeners:
            try:
                await listener()
            except Exception:
                pass
