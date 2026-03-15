"""FastAPI app — WebSocket push + REST commands for the time engine."""

from __future__ import annotations

import asyncio
import json
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

async def placeholder_tick(game_time: GameTime) -> list[GameEvent]:
    """Emit year-start events and an auto-pause every 10 years as proof of concept."""
    events: list[GameEvent] = []

    if game_time.day_of_year == 1:
        events.append(GameEvent(
            event_type="time.year_start",
            severity=Severity.INFO,
            game_time=game_time,
            data={"year": game_time.year},
        ))

    # Auto-pause proof of concept: pause every 10 years
    if game_time.day_of_year == 1 and game_time.year % 10 == 0:
        events.append(GameEvent(
            event_type="auto_pause.placeholder",
            severity=Severity.MILESTONE,
            game_time=game_time,
            auto_pause=True,
        ))

    # Render narrative text onto each event
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
    # Send current state immediately on connect
    try:
        await ws.send_text(json.dumps(engine.state_dict()))
        while True:
            # Keep connection alive; we don't expect client messages yet
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
