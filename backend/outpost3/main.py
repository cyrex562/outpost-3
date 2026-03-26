"""FastAPI app — WebSocket push + REST commands for the time engine."""

from __future__ import annotations

import json
from contextlib import asynccontextmanager

from fastapi import FastAPI, HTTPException, WebSocket, WebSocketDisconnect
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel

from .simulation import GameEvent, GameTime, Severity
from .simulation.engine import TimeEngine, VALID_SPEEDS
from .simulation.world import World
from .simulation.components import ShipHull
from .simulation.loadout import run_loadout, world_state_dict
from .simulation.search import confirm_destination, run_search
from .simulation.transit import TransitSystem
from .simulation.survey import SurveySystem, confirm_survey_decision
from .simulation.deliberation import confirm_deliberation
from .narrative import render

# ── globals ───────────────────────────────────────────────────────

engine = TimeEngine()
world = World()
connected_clients: set[WebSocket] = set()


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
    """Broadcast engine state + world state after every tick cycle."""
    await broadcast_json(engine.state_dict())
    await broadcast_json(world_state_dict(world))


# ── app lifecycle ─────────────────────────────────────────────────

@asynccontextmanager
async def lifespan(app: FastAPI):
    # Phase 1: Loadout — populate world with ship, crew, notables
    _loadout_result, loadout_events = run_loadout(world)
    for event in loadout_events:
        event.text = render(event)

    # Phase 2: Search — discover star systems, select destination
    _search_result, search_events = run_search(world)
    for event in search_events:
        event.text = render(event)

    # All startup events replayed for late-joining clients
    startup_events = loadout_events + search_events
    app.state.startup_events = startup_events

    # Register simulation systems — order matters: transit runs before survey
    transit_system = TransitSystem(world)
    survey_system = SurveySystem(world)
    engine.on_tick(transit_system.tick)
    engine.on_tick(survey_system.tick)
    engine.on_batch(transit_system.batch)
    engine.on_batch(survey_system.batch)
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
        # Send time + world state on connect
        await ws.send_text(json.dumps(engine.state_dict()))
        await ws.send_text(json.dumps(world_state_dict(world)))
        # Replay startup events so late-joining clients see full history
        for event in app.state.startup_events:
            await ws.send_text(json.dumps({"type": "event", **event.to_dict()}))
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


@app.get("/api/world")
async def get_world():
    return world_state_dict(world)


# ── Player command endpoints ───────────────────────────────────────

class SelectDestinationRequest(BaseModel):
    name: str


class SurveyDecisionRequest(BaseModel):
    decision: str   # "accept" | "reject"


class DeliberationDecisionRequest(BaseModel):
    chosen: str   # option key (e.g. "strict_rations", "emergency_repairs", etc.)


async def _broadcast_events(events: list[GameEvent]) -> None:
    """Render narrative + broadcast a list of events to all clients."""
    for event in events:
        event.text = render(event)
        await on_event(event)
    await on_state_change()


@app.post("/api/command/select_destination")
async def cmd_select_destination(req: SelectDestinationRequest):
    """Player chooses a destination from the candidate systems (search phase)."""
    try:
        _, events = confirm_destination(world, req.name)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc))

    await _broadcast_events(events)
    # Add to startup replay so late-joining clients see the decision
    app.state.startup_events.extend(events)
    return {"ok": True, "destination": req.name}


@app.post("/api/command/survey_decision")
async def cmd_survey_decision(req: SurveyDecisionRequest):
    """Player accepts or rejects the surveyed system (survey phase)."""
    if req.decision not in ("accept", "reject"):
        raise HTTPException(status_code=400, detail="decision must be 'accept' or 'reject'")

    game_time = GameTime(engine.day)
    try:
        events = confirm_survey_decision(world, req.decision, game_time)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc))

    await _broadcast_events(events)
    app.state.startup_events.extend(events)
    return {"ok": True, "decision": req.decision}


@app.post("/api/command/deliberation_decision")
async def cmd_deliberation_decision(req: DeliberationDecisionRequest):
    """Player approves or overrides the crew's deliberation recommendation."""
    game_time = GameTime(engine.day)
    ship_pair = world.query_one(ShipHull)
    ship_eid = ship_pair[0] if ship_pair else -1
    try:
        events = confirm_deliberation(world, ship_eid, req.chosen, game_time)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc))

    await _broadcast_events(events)
    app.state.startup_events.extend(events)
    return {"ok": True, "chosen": req.chosen}
