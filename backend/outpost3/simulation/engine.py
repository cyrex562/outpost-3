"""Time engine — async tick loop with 15-level speed control, batching, and ECS integration."""

from __future__ import annotations

import asyncio
import time
from typing import Callable, Awaitable

from . import GameEvent, GameTime, Severity
from .ecs import World
from .systems import System, Phase

# Speed config: 15 levels, each defines days per 2.5s cycle.
CYCLE_DELAY = 2.5  # seconds between cycles

SPEED_LEVELS: list[dict] = [
    {"level": 1,  "days": 1,     "label": "1d"},
    {"level": 2,  "days": 2,     "label": "2d"},
    {"level": 3,  "days": 5,     "label": "5d"},
    {"level": 4,  "days": 10,    "label": "10d"},
    {"level": 5,  "days": 25,    "label": "25d"},
    {"level": 6,  "days": 50,    "label": "50d"},
    {"level": 7,  "days": 100,   "label": "100d"},
    {"level": 8,  "days": 250,   "label": "250d"},
    {"level": 9,  "days": 500,   "label": "500d"},
    {"level": 10, "days": 1000,  "label": "1Kd"},
    {"level": 11, "days": 2500,  "label": "2.5Kd"},
    {"level": 12, "days": 5000,  "label": "5Kd"},
    {"level": 13, "days": 10000, "label": "10Kd"},
    {"level": 14, "days": 25000, "label": "25Kd"},
    {"level": 15, "days": 36500, "label": "MAX"},
]

# Lookup: level number → days per cycle
SPEED_CONFIG: dict[int, int] = {s["level"]: s["days"] for s in SPEED_LEVELS}

VALID_SPEEDS = list(SPEED_CONFIG.keys())

# Levels at or above this use batching (tick systems once per batch, not per day)
BATCH_THRESHOLD_LEVEL = 8

# Max state pushes per second at high speeds
MAX_STATE_PUSHES_PER_SEC = 30


class TimeEngine:
    """Drives the simulation clock forward using registered Systems."""

    def __init__(self) -> None:
        self.world: World = World()
        self.day: int = 0  # total days elapsed
        self.speed: int = 1  # speed level (1-15)
        self.paused: bool = True
        self._running: bool = False
        self._task: asyncio.Task | None = None

        # Registered ECS systems (sorted by order)
        self._systems: list[System] = []

        # Legacy callbacks (kept for backward compatibility / direct event listeners)
        self._tick_handlers: list[Callable[[GameTime], Awaitable[list[GameEvent]]]] = []
        self._batch_handlers: list[Callable[[GameTime, GameTime, int], Awaitable[list[GameEvent]]]] = []
        self._event_listeners: list[Callable[[GameEvent], Awaitable[None]]] = []
        self._state_listeners: list[Callable[[], Awaitable[None]]] = []

    # ── system registration ──────────────────────────────────────

    def register_system(self, system: System) -> None:
        """Register an ECS system. Systems are sorted by order."""
        self._systems.append(system)
        self._systems.sort(key=lambda s: s.order)

    # ── legacy callback registration ─────────────────────────────

    def on_tick(self, handler: Callable[[GameTime], Awaitable[list[GameEvent]]]) -> None:
        self._tick_handlers.append(handler)

    def on_batch(self, handler: Callable[[GameTime, GameTime, int], Awaitable[list[GameEvent]]]) -> None:
        self._batch_handlers.append(handler)

    def on_event(self, listener: Callable[[GameEvent], Awaitable[None]]) -> None:
        self._event_listeners.append(listener)

    def on_state_change(self, listener: Callable[[], Awaitable[None]]) -> None:
        self._state_listeners.append(listener)

    # ── control ───────────────────────────────────────────────────

    def set_speed(self, speed: int) -> None:
        if speed not in SPEED_CONFIG:
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

    # ── current phase ────────────────────────────────────────────

    def _get_current_phase(self) -> Phase | None:
        """Get current phase from the world's faction entity, if any."""
        # Look for any entity with a FactionPhase component
        # Import here to avoid circular imports; component types defined elsewhere
        from .systems import Phase
        for comp_type, store in self.world._components.items():
            if comp_type.__name__ == "FactionPhase":
                for eid, comp in store.items():
                    if hasattr(comp, "phase"):
                        try:
                            return Phase(comp.phase) if isinstance(comp.phase, str) else comp.phase
                        except (ValueError, KeyError):
                            pass
        return None

    # ── main loop ────────────────────────────────────────────────

    async def _loop(self) -> None:
        while self._running:
            if self.paused:
                await asyncio.sleep(0.05)
                continue

            days_per_cycle = SPEED_CONFIG.get(self.speed, 1)
            use_batching = self.speed >= BATCH_THRESHOLD_LEVEL

            if use_batching:
                should_pause = await self._run_batched_days(days_per_cycle)
            else:
                should_pause = await self._run_individual_days(days_per_cycle)

            if should_pause:
                self.paused = True

            await self._broadcast_state()
            await asyncio.sleep(CYCLE_DELAY)

    async def _run_individual_days(self, num_days: int) -> bool:
        """Simulate days one at a time, running all systems per day."""
        should_pause = False
        for _ in range(num_days):
            if self.paused or not self._running:
                break

            self.day += 1
            game_time = GameTime(self.day)

            events = await self._tick_one_day(game_time)
            pause_hit = await self._emit_events(events)
            if pause_hit:
                should_pause = True
                break

        return should_pause

    async def _run_batched_days(self, num_days: int) -> bool:
        """Batch ticks: run systems for each day but throttle state pushes."""
        should_pause = False
        start_day = self.day

        # Calculate push interval to stay under MAX_STATE_PUSHES_PER_SEC
        # At high speeds we process all days in one go, push state once at end
        all_events: list[GameEvent] = []

        for i in range(num_days):
            if self.paused or not self._running:
                break

            self.day += 1
            game_time = GameTime(self.day)

            events = await self._tick_one_day(game_time)
            all_events.extend(events)

            # Check for auto-pause
            for event in events:
                if event.auto_pause:
                    should_pause = True
                    break
            if should_pause:
                break

        # Emit all collected events
        for event in all_events:
            for listener in self._event_listeners:
                try:
                    await listener(event)
                except Exception:
                    pass

        # Also run legacy batch handlers if any
        if self._batch_handlers and not should_pause:
            start_time = GameTime(start_day + 1)
            end_time = GameTime(self.day)
            for handler in self._batch_handlers:
                try:
                    result = await handler(start_time, end_time, self.day - start_day)
                    if result:
                        pause_hit = await self._emit_events(result)
                        if pause_hit:
                            should_pause = True
                except Exception:
                    pass

        return should_pause

    async def _tick_one_day(self, game_time: GameTime) -> list[GameEvent]:
        """Run all systems and legacy tick handlers for a single day."""
        events: list[GameEvent] = []
        current_phase = self._get_current_phase()

        # Run ECS systems in order
        for system in self._systems:
            if system.should_run(current_phase):
                try:
                    result = system.tick(self.world, game_time)
                    if result:
                        events.extend(result)
                except Exception as exc:
                    events.append(GameEvent(
                        event_type="engine.error",
                        severity=Severity.CRITICAL,
                        game_time=game_time,
                        data={"error": f"{type(exc).__name__}: {exc}", "system": type(system).__name__},
                    ))

        # Run legacy tick handlers
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
