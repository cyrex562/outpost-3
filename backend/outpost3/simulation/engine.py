"""Time engine — async tick loop with speed control, batching, and auto-pause."""

from __future__ import annotations

import asyncio
import time
from typing import Callable, Awaitable

from . import GameEvent, GameTime, Severity

# Speed config: days per cycle and cycle delay.
# Each speed level = how many game days pass per ~2.5s real-time cycle.
CYCLE_DELAY = 2.5  # seconds between cycles

SPEED_CONFIG: dict[int, int] = {
    # speed_id: days_per_cycle
    1:     1,       # 1 day / 2.5s — 1 year ≈ 15 min
    2:     2,       # 2 days / 2.5s — 1 year ≈ 7.5 min
    7:     7,       # 1 week / 2.5s — 1 year ≈ 2 min
    10:    10,      # 10 days / 2.5s — 1 year ≈ 1.5 min
    30:    30,      # 1 month / 2.5s — 1 year ≈ 30s
    90:    90,      # 3 months / 2.5s — 1 year ≈ 10s
    180:   180,     # 6 months / 2.5s — 1 year ≈ 5s
    365:   365,     # 1 year / 2.5s
    730:   730,     # 2 years / 2.5s
    1825:  1825,    # 5 years / 2.5s
    3650:  3650,    # 10 years / 2.5s
    9125:  9125,    # 25 years / 2.5s
    18250: 18250,   # 50 years / 2.5s
    36500: 36500,   # 100 years / 2.5s
}

VALID_SPEEDS = list(SPEED_CONFIG.keys())

# At or above this many days/cycle, use batch handlers instead of per-day ticks.
BATCH_THRESHOLD = 10


class TimeEngine:
    """Drives the simulation clock forward, emitting events each tick."""

    def __init__(self) -> None:
        self.day: int = 0  # total days elapsed
        self.speed: int = 1
        self.paused: bool = True
        self._running: bool = False
        self._task: asyncio.Task | None = None

        # Callbacks
        self._tick_handlers: list[Callable[[GameTime], Awaitable[list[GameEvent]]]] = []
        self._batch_handlers: list[Callable[[GameTime, GameTime, int], Awaitable[list[GameEvent]]]] = []
        self._event_listeners: list[Callable[[GameEvent], Awaitable[None]]] = []
        self._state_listeners: list[Callable[[], Awaitable[None]]] = []

    # ── registration ──────────────────────────────────────────────

    def on_tick(self, handler: Callable[[GameTime], Awaitable[list[GameEvent]]]) -> None:
        """Register a per-day tick handler. Called for each day at low speeds."""
        self._tick_handlers.append(handler)

    def on_batch(self, handler: Callable[[GameTime, GameTime, int], Awaitable[list[GameEvent]]]) -> None:
        """Register a batch handler called at high speeds.

        Args: (start_time, end_time, num_days) — handler should summarize
        the range rather than simulating each day individually.
        """
        self._batch_handlers.append(handler)

    def on_event(self, listener: Callable[[GameEvent], Awaitable[None]]) -> None:
        self._event_listeners.append(listener)

    def on_state_change(self, listener: Callable[[], Awaitable[None]]) -> None:
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
            "month": gt.month,
            "day_of_month": gt.day_of_month,
            "year": gt.year,
            "day_offset": self.day,
        }

    # ── internals ─────────────────────────────────────────────────

    async def _loop(self) -> None:
        while self._running:
            if self.paused:
                await asyncio.sleep(0.05)
                continue

            days_per_cycle = SPEED_CONFIG.get(self.speed, 1)

            if days_per_cycle < BATCH_THRESHOLD:
                # Low speed: simulate each day individually
                should_pause = await self._run_individual_days(days_per_cycle)
            else:
                # High speed: batch the days
                should_pause = await self._run_batched_days(days_per_cycle)

            if should_pause:
                self.paused = True

            # Always broadcast final state after a cycle
            await self._broadcast_state()

            await asyncio.sleep(CYCLE_DELAY)

    async def _run_individual_days(self, num_days: int) -> bool:
        """Simulate days one at a time, calling tick handlers for each."""
        should_pause = False
        for _ in range(num_days):
            if self.paused or not self._running:
                break

            self.day += 1
            game_time = GameTime(self.day)

            events = await self._collect_tick_events(game_time)
            pause_hit = await self._emit_events(events)
            if pause_hit:
                should_pause = True
                break

        return should_pause

    async def _run_batched_days(self, num_days: int) -> bool:
        """Advance multiple days at once, using batch handlers for summaries."""
        start_day = self.day + 1
        end_day = self.day + num_days

        start_time = GameTime(start_day)
        end_time = GameTime(end_day)

        # Collect events from batch handlers
        events: list[GameEvent] = []
        for handler in self._batch_handlers:
            try:
                result = await handler(start_time, end_time, num_days)
                if result:
                    events.extend(result)
            except Exception as exc:
                events.append(GameEvent(
                    event_type="engine.error",
                    severity=Severity.CRITICAL,
                    game_time=end_time,
                    data={"error": str(exc)},
                ))

        # If no batch handlers registered, fall back to checking key days only
        if not self._batch_handlers:
            events = await self._collect_key_day_events(start_day, end_day)

        # Advance the clock
        self.day = end_day

        # Check for auto-pause in events
        should_pause = False
        pause_events: list[GameEvent] = []
        other_events: list[GameEvent] = []
        for event in events:
            if event.auto_pause:
                should_pause = True
                pause_events.append(event)
            else:
                other_events.append(event)

        # Emit non-pause events, then pause events last
        await self._emit_events(other_events)
        if pause_events:
            await self._emit_events(pause_events)

        return should_pause

    async def _collect_key_day_events(self, start_day: int, end_day: int) -> list[GameEvent]:
        """At high speeds without batch handlers, check only milestone days."""
        events: list[GameEvent] = []
        for d in range(start_day, end_day + 1):
            gt = GameTime(d)
            # Only process days that are month/year boundaries
            if gt.day_of_year == 1 or gt.day_of_year % 30 == 1:
                for handler in self._tick_handlers:
                    try:
                        result = await handler(gt)
                        if result:
                            events.extend(result)
                    except Exception as exc:
                        events.append(GameEvent(
                            event_type="engine.error",
                            severity=Severity.CRITICAL,
                            game_time=gt,
                            data={"error": str(exc)},
                        ))
        return events

    async def _collect_tick_events(self, game_time: GameTime) -> list[GameEvent]:
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
        return events

    async def _emit_events(self, events: list[GameEvent]) -> bool:
        """Broadcast events to listeners. Returns True if any event triggers auto-pause."""
        should_pause = False
        for event in events:
            for listener in self._event_listeners:
                try:
                    await listener(event)
                except Exception:
                    pass
            if event.auto_pause:
                should_pause = True
        return should_pause

    async def _broadcast_state(self) -> None:
        for listener in self._state_listeners:
            try:
                await listener()
            except Exception:
                pass
