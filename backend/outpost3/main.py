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
from .simulation.components import (
    FactionPhase, Resources, Population, ShipSystems, Notable,
    Loadout, StarSystem, Planet, SearchProgress, TransitState,
)
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


def build_snapshot() -> dict:
    """Build a world snapshot with resources, population, ship systems, etc."""
    world = engine.world
    gt = GameTime(engine.day)

    snapshot: dict = {
        "type": "snapshot",
        "day_offset": engine.day,
        "year": gt.year,
        "month": gt.month,
        "phase": None,
        "resources": None,
        "population": None,
        "ship_systems": None,
        "notables": [],
        "star_systems": [],
        "transit": None,
        "search": None,
        "ship_name": None,
    }

    # Faction data
    faction_entities = world.entities_with(FactionPhase)
    if faction_entities:
        fid = faction_entities[0]
        fp = world.get_component(fid, FactionPhase)
        if fp:
            snapshot["phase"] = fp.phase.value

        res = world.get_component(fid, Resources)
        if res:
            snapshot["resources"] = res.to_summary()

        pop = world.get_component(fid, Population)
        if pop:
            snapshot["population"] = {
                "total": pop.total,
                "scientists": pop.scientists,
                "engineers": pop.engineers,
                "medical": pop.medical,
                "military": pop.military,
                "civilians": pop.civilians,
                "morale": round(pop.morale, 2),
            }

        ss = world.get_component(fid, ShipSystems)
        if ss:
            snapshot["ship_systems"] = ss.to_summary()

        transit = world.get_component(fid, TransitState)
        if transit:
            dest_system = world.get_component(transit.destination_id, StarSystem)
            snapshot["transit"] = {
                "destination": dest_system.name if dest_system else "Unknown",
                "distance_ly": transit.distance_ly,
                "distance_traveled_ly": round(transit.distance_traveled_ly, 2),
                "transit_phase": transit.transit_phase,
                "eta_days": transit.eta_days,
                "departure_day": transit.departure_day,
                "pct_complete": round(
                    transit.distance_traveled_ly / max(transit.distance_ly, 0.01) * 100, 1
                ),
            }

        search = world.get_component(fid, SearchProgress)
        if search:
            snapshot["search"] = {
                "systems_discovered": search.systems_discovered,
                "target_discoveries": search.target_discoveries,
            }

        loadout = world.get_component(fid, Loadout)
        if loadout:
            snapshot["ship_name"] = loadout.ship_name

    # Notables
    for eid, notable in world.all_components_of_type(Notable).items():
        snapshot["notables"].append({
            "name": notable.name,
            "role": notable.role,
            "age": int(notable.age),
            "traits": notable.traits,
            "alive": notable.alive,
        })

    # Star systems
    for eid, star in world.all_components_of_type(StarSystem).items():
        snapshot["star_systems"].append({
            "name": star.name,
            "distance_ly": star.distance_ly,
            "habitability_score": star.habitability_score,
            "num_planets": star.num_planets,
            "selected": star.selected,
        })

    return snapshot


# Periodic snapshot broadcast
_snapshot_task: asyncio.Task | None = None


async def _snapshot_loop() -> None:
    """Broadcast world snapshots at regular intervals."""
    while True:
        if connected_clients:
            try:
                snap = build_snapshot()
                await broadcast_json(snap)
            except Exception:
                pass
        await asyncio.sleep(2.0)  # 0.5 snapshots/sec


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
    global _snapshot_task
    setup_world(engine)
    engine.on_event(on_event)
    engine.on_state_change(on_state_change)
    engine.start()
    _snapshot_task = asyncio.create_task(_snapshot_loop())
    yield
    engine.stop()
    if _snapshot_task:
        _snapshot_task.cancel()


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


@app.get("/api/snapshot")
async def get_snapshot():
    return build_snapshot()


@app.get("/api/speeds")
async def get_speeds():
    """Return the list of available speed levels with labels."""
    return {"speeds": SPEED_LEVELS}
