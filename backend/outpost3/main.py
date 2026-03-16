"""FastAPI app — WebSocket push + REST commands for the time engine."""

from __future__ import annotations

import asyncio
import json
import random
from contextlib import asynccontextmanager

from fastapi import FastAPI, WebSocket, WebSocketDisconnect
from fastapi.middleware.cors import CORSMiddleware

from .simulation import GameEvent, GameTime, Severity
from .simulation.engine import TimeEngine, VALID_SPEEDS, SPEED_LEVELS
from .simulation.systems import Phase
from .simulation.components import FactionPhase
from .simulation.generation.loadout import LoadoutSystem
from .simulation.generation.search import SearchSystem
from .simulation.transit import TransitSystem
from .simulation.population import PopulationSystem
from .simulation.maintenance import ShipMaintenanceSystem
from .simulation.behavior import BehaviorSystem
from .narrative import render

# ── globals ───────────────────────────────────────────────────────

engine = TimeEngine()
connected_clients: set[WebSocket] = set()


# ── narrative rendering tick handler ──────────────────────────────

async def render_narrative(game_time: GameTime) -> list[GameEvent]:
    """No-op tick handler placeholder. Narrative rendering happens via on_event."""
    return []


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
    # Render narrative text if not already set
    if event.text is None:
        event.text = render(event)
    await broadcast_json({"type": "event", **event.to_dict()})


async def on_state_change() -> None:
    await broadcast_json(engine.state_dict())


# ── world setup ──────────────────────────────────────────────────

def setup_world(engine: TimeEngine) -> None:
    """Initialize the world with a faction entity and register all systems."""
    # Create the faction entity
    faction_id = engine.world.create_entity()
    engine.world.add_component(faction_id, FactionPhase(phase=Phase.LOADOUT))

    # Register systems in execution order
    engine.register_system(LoadoutSystem())       # order=1
    engine.register_system(SearchSystem())        # order=10
    engine.register_system(TransitSystem())       # order=20
    engine.register_system(PopulationSystem())    # order=30
    engine.register_system(ShipMaintenanceSystem())  # order=40
    engine.register_system(BehaviorSystem())      # order=60


# ── app lifecycle ─────────────────────────────────────────────────

@asynccontextmanager
async def lifespan(app: FastAPI):
    setup_world(engine)
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
        return {"error": f"Invalid speed level. Valid: {VALID_SPEEDS}"}
    engine.set_speed(speed)
    return engine.state_dict()


@app.get("/api/state")
async def get_state():
    return engine.state_dict()


@app.get("/api/speeds")
async def get_speeds():
    """Return the list of available speed levels with labels."""
    return {"speeds": SPEED_LEVELS}
