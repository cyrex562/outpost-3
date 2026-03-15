"""FastAPI app — WebSocket push + REST commands for the time engine."""

from __future__ import annotations

import asyncio
import json
import random
from contextlib import asynccontextmanager

from fastapi import FastAPI, WebSocket, WebSocketDisconnect
from fastapi.middleware.cors import CORSMiddleware

from .simulation import GameEvent, GameTime, Severity
from .simulation.engine import TimeEngine, VALID_SPEEDS
from .narrative import render

# ── globals ───────────────────────────────────────────────────────

engine = TimeEngine()
connected_clients: set[WebSocket] = set()


# ── placeholder tick handler (Milestone 0) ────────────────────────

DAILY_FLAVOR: list[tuple[str, str, Severity]] = [
    ("daily.weather", "clear_skies", Severity.DEBUG),
    ("daily.weather", "dust_storm", Severity.INFO),
    ("daily.weather", "solar_flare_warning", Severity.NOTABLE),
    ("daily.survey", "mineral_deposit_found", Severity.INFO),
    ("daily.survey", "geological_survey_complete", Severity.INFO),
    ("daily.crew", "crew_morale_high", Severity.DEBUG),
    ("daily.crew", "minor_injury_reported", Severity.INFO),
    ("daily.crew", "equipment_malfunction", Severity.NOTABLE),
    ("daily.construction", "foundation_work_continues", Severity.DEBUG),
    ("daily.construction", "structural_milestone", Severity.INFO),
    ("daily.power", "solar_panel_output_nominal", Severity.DEBUG),
    ("daily.power", "power_grid_fluctuation", Severity.INFO),
    ("daily.life_support", "oxygen_levels_stable", Severity.DEBUG),
    ("daily.life_support", "water_recycler_maintenance", Severity.INFO),
]


async def placeholder_tick(game_time: GameTime) -> list[GameEvent]:
    """Emit varied events each day as a proof of concept.

    Generates year-start, month-start, random daily events, and
    auto-pause milestones.
    """
    events: list[GameEvent] = []

    # Year start
    if game_time.is_year_start:
        events.append(GameEvent(
            event_type="time.year_start",
            severity=Severity.NOTABLE,
            game_time=game_time,
            data={"year": game_time.year},
        ))

    # Month start
    if game_time.is_month_start and not game_time.is_year_start:
        events.append(GameEvent(
            event_type="time.month_start",
            severity=Severity.INFO,
            game_time=game_time,
            data={"month": game_time.month, "year": game_time.year},
        ))

    # Random daily events (~30% chance per day)
    if random.random() < 0.30:
        event_type, variant, severity = random.choice(DAILY_FLAVOR)
        events.append(GameEvent(
            event_type=event_type,
            severity=severity,
            game_time=game_time,
            data={"variant": variant, "year": game_time.year,
                  "month": game_time.month, "day": game_time.day_of_month},
        ))

    # Auto-pause every year (365 days) as proof of concept
    if game_time.is_year_start and game_time.year > 1:
        events.append(GameEvent(
            event_type="auto_pause.year_end",
            severity=Severity.MILESTONE,
            game_time=game_time,
            auto_pause=True,
            data={"year": game_time.year - 1},
        ))

    # Render narrative text onto each event
    for event in events:
        event.text = render(event)

    return events


async def placeholder_batch(start: GameTime, end: GameTime, num_days: int) -> list[GameEvent]:
    """Batch handler for high-speed simulation. Summarizes a range of days."""
    events: list[GameEvent] = []

    # Emit year-start events for any year boundaries in the range
    for d in range(start.day_offset, end.day_offset + 1):
        gt = GameTime(d)
        if gt.is_year_start:
            events.append(GameEvent(
                event_type="time.year_start",
                severity=Severity.NOTABLE,
                game_time=gt,
                data={"year": gt.year},
            ))
            # Auto-pause on year boundary
            if gt.year > 1:
                events.append(GameEvent(
                    event_type="auto_pause.year_end",
                    severity=Severity.MILESTONE,
                    game_time=gt,
                    auto_pause=True,
                    data={"year": gt.year - 1},
                ))

    # Emit month-start events for month boundaries
    for d in range(start.day_offset, end.day_offset + 1):
        gt = GameTime(d)
        if gt.is_month_start and not gt.is_year_start:
            events.append(GameEvent(
                event_type="time.month_start",
                severity=Severity.INFO,
                game_time=gt,
                data={"month": gt.month, "year": gt.year},
            ))

    # A few random events for flavor (scaled by range size)
    num_random = max(1, num_days // 30)
    for _ in range(num_random):
        rand_day = random.randint(start.day_offset, end.day_offset)
        gt = GameTime(rand_day)
        event_type, variant, severity = random.choice(DAILY_FLAVOR)
        events.append(GameEvent(
            event_type=event_type,
            severity=severity,
            game_time=gt,
            data={"variant": variant, "year": gt.year,
                  "month": gt.month, "day": gt.day_of_month},
        ))

    # Sort by day offset so events appear in chronological order
    events.sort(key=lambda e: e.game_time.day_offset)

    # Render narrative text
    for event in events:
        event.text = render(event)

    return events


# ── broadcast helpers ─────────────────────────────────────────────

async def broadcast_json(data: dict) -> None:
    """Send JSON to all connected WebSocket clients."""
    if not connected_clients:
        return
    raw = json.dumps(data)
    dead: list[WebSocket] = []
    for ws in connected_clients:
        try:
            await ws.send_text(raw)
        except Exception:
            dead.append(ws)
    for ws in dead:
        connected_clients.discard(ws)


async def on_event(event: GameEvent) -> None:
    await broadcast_json({"type": "event", **event.to_dict()})


async def on_state_change() -> None:
    await broadcast_json(engine.state_dict())


# ── app lifecycle ─────────────────────────────────────────────────

@asynccontextmanager
async def lifespan(app: FastAPI):
    engine.on_tick(placeholder_tick)
    engine.on_batch(placeholder_batch)
    engine.on_event(on_event)
    engine.on_state_change(on_state_change)
    engine.start()
    yield
    engine.stop()


# ── FastAPI app ───────────────────────────────────────────────────

app = FastAPI(title="Outpost 3", lifespan=lifespan)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)


# ── WebSocket ─────────────────────────────────────────────────────

@app.websocket("/ws")
async def websocket_endpoint(ws: WebSocket):
    await ws.accept()
    connected_clients.add(ws)
    try:
        await ws.send_text(json.dumps(engine.state_dict()))
        while True:
            await ws.receive_text()
    except WebSocketDisconnect:
        pass
    finally:
        connected_clients.discard(ws)


# ── REST commands ─────────────────────────────────────────────────

@app.post("/api/pause")
async def pause():
    engine.pause()
    return engine.state_dict()


@app.post("/api/resume")
async def resume():
    engine.resume()
    return engine.state_dict()


@app.post("/api/speed/{speed}")
async def set_speed(speed: int):
    if speed not in VALID_SPEEDS:
        return {"error": f"Invalid speed. Valid: {VALID_SPEEDS}"}
    engine.set_speed(speed)
    return engine.state_dict()


@app.get("/api/state")
async def get_state():
    return engine.state_dict()
