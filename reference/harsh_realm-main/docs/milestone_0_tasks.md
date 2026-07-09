# Milestone 0: Skeleton — Task Specification

> **Goal:** End-to-end data flow working. No game logic yet.
> **Estimated time:** 1 week (AI-assisted development)
> **Prerequisite:** Read CLAUDE.md and AGENTS.md before starting.

## Success Criteria

When this milestone is complete, the following must all work:

1. `pytest` passes with all tests green.
2. Run `uvicorn harsh_realm.main:app` — server starts on `127.0.0.1:8080`.
3. Open `http://localhost:5173` (Vue dev server) — see a chat interface.
4. Type "hello" in the input box, press enter — see "hello" echoed in the chat log (via WebSocket round-trip through the server).
5. Via REST API or CLI, create a new world called "Ashfall" — a `.db` file appears in `worlds/`.
6. Via REST API, list worlds — see "Ashfall" in the list.
7. Via REST API, switch to the "Ashfall" world — server confirms the switch.
8. Events typed in chat are logged to the `event_log` table in the active world's SQLite database.

---

## Task 0.1: Project Scaffolding

**What:** Set up the Python project structure, dependencies, and configuration.

**Deliverables:**
- `pyproject.toml` with project metadata and dependencies:
  - Runtime: `fastapi`, `uvicorn[standard]`, `aiosqlite`, `pyyaml`
  - Dev: `pytest`, `pytest-asyncio`, `black`, `ruff`, `httpx` (for test client)
- `config.yaml` with default configuration:
  ```yaml
  server:
    host: "127.0.0.1"
    port: 8080
  worlds:
    directory: "worlds"
  logging:
    level: "info"
  ```
- `src/harsh_realm/__init__.py` (empty or version string)
- `src/harsh_realm/config.py` — loads `config.yaml`, returns a frozen dataclass `AppConfig`
- `src/harsh_realm/exceptions.py` — base exception classes (`HarshRealmError`, `EntityNotFoundError`, `InvalidCommandError`, `WorldNotLoadedError`)
- `.gitignore` including `worlds/`, `__pycache__/`, `.venv/`, `node_modules/`
- Empty `worlds/` directory with a `.gitkeep`

**Tests:**
- `test_config.py`: Loading config from YAML produces correct `AppConfig`. Missing file uses defaults.

**Acceptance:** `pip install -e ".[dev]"` succeeds. `pytest tests/test_config.py` passes.

---

## Task 0.2: SQLite World Database

**What:** Implement the `WorldDatabase` class that manages SQLite world files.

**File:** `src/harsh_realm/db.py`

**Deliverables:**
- `WorldDatabase` class with async methods:
  - `create(filepath, name, settings=None)` — create a new `.db` file with schema, write world_meta (name, created_at, tick=0)
  - `open(filepath)` — open an existing `.db` file, verify schema
  - `close()` — close the connection
  - `get_meta(key)` → `str | None`
  - `set_meta(key, value)` 
  - `execute(sql, params)` — parameterized query execution
  - `fetch_one(sql, params)` → `Row | None`
  - `fetch_all(sql, params)` → `list[Row]`
  - `save_snapshot(name)` — copy the current `.db` file to `{stem}_{name}.db`
  - `list_worlds(directory)` — static method, returns list of `{file, name, last_modified}` for all `.db` files in directory

- Full schema creation (all tables from the architecture doc §5):
  - `world_meta`, `hexes`, `entities`, `factions`, `faction_assets`, `faction_relations`, `reputation`, `random_tables`, `event_log`, `gm_state`, `dungeons`, `practice_log`

**Tests:** `test_db.py`
- Create a world → file exists, schema tables exist
- Write and read `world_meta` key
- Open existing world → meta values preserved
- `list_worlds` finds created worlds
- `save_snapshot` creates a copy with correct name
- Cannot open a non-existent file (raises error)

**Acceptance:** All `test_db.py` tests pass.

---

## Task 0.3: Event Bus

**What:** Implement the in-process event bus for pub/sub event dispatch.

**File:** `src/harsh_realm/events.py`

**Deliverables:**
- `GameEvent` frozen dataclass:
  - `id: str` (auto-generated UUID)
  - `tick: int`
  - `event_type: str`
  - `data: dict`
  - `source: str = "system"`
  - `timestamp: str` (ISO format, auto-generated)
  
- `EventBus` class:
  - `subscribe(event_type: str, handler: Callable[[GameEvent], list[GameEvent] | None])` — register a handler for an event type
  - `subscribe_all(handler: Callable)` — wildcard subscription (receives all events)
  - `publish(event: GameEvent) -> list[GameEvent]` — dispatch event to matching handlers, collect any returned events, process cascades up to `max_cascade_depth` (default 10)
  - `clear()` — remove all subscriptions (useful for testing)
  
- Cascade behavior: if a handler returns new events, those are published in order. If cascade depth exceeds `max_cascade_depth`, log a warning and stop.

- `EventLogger` — a handler that writes events to the `event_log` table in the active world database. Subscribes to `"*"` (wildcard).

**Tests:** `test_events.py`
- Publish an event → subscribed handler receives it
- Wildcard handler receives all event types
- Handler returning a new event → cascade occurs
- Cascade depth limit is enforced
- Non-matching handler does not fire
- Handler exception is caught and logged (does not crash the bus)

**Acceptance:** All `test_events.py` tests pass.

---

## Task 0.4: FastAPI Server + REST Endpoints

**What:** Set up the FastAPI application with world management endpoints.

**Files:**
- `src/harsh_realm/main.py` — FastAPI app creation, startup/shutdown, route mounting
- `src/harsh_realm/api/routes.py` — REST endpoint handlers

**Deliverables:**
- FastAPI app with CORS enabled (allow all origins for dev)
- Health check: `GET /health` → `{"status": "healthy", "service": "harsh_realm"}`
- World management endpoints:
  - `GET /api/worlds` → list of available worlds `[{name, file, last_modified}]`
  - `POST /api/worlds` with `{"name": "Ashfall"}` → creates world, returns `{name, file}`
  - `POST /api/worlds/load` with `{"file": "ashfall.db"}` → loads world as active, returns `{name, file}`
  - `GET /api/worlds/current` → current active world info or 404 if none loaded
  - `POST /api/worlds/save` with `{"name": "my_save"}` → creates named snapshot
- App state holding:
  - `active_world: WorldDatabase | None`
  - `event_bus: EventBus`
  - `config: AppConfig`
- On world load: connect `EventLogger` to the event bus

**Tests:** `test_api.py` (use `httpx.AsyncClient` with FastAPI test client)
- `GET /health` returns 200
- `POST /api/worlds` creates a world file
- `GET /api/worlds` lists the created world
- `POST /api/worlds/load` loads the world
- `GET /api/worlds/current` returns loaded world info
- `POST /api/worlds/save` creates snapshot file
- Loading a non-existent world returns 404

**Acceptance:** All `test_api.py` tests pass. Server starts with `uvicorn harsh_realm.main:app`.

---

## Task 0.5: WebSocket Handler

**What:** WebSocket endpoint that receives player input and broadcasts events.

**Files:**
- `src/harsh_realm/api/websocket.py` — WebSocket handler + connection manager

**Deliverables:**
- `ConnectionManager` class:
  - `connect(websocket)` — accept and track connection
  - `disconnect(websocket)` — remove connection
  - `broadcast(message: dict)` — send JSON message to all connected clients
  
- WebSocket endpoint at `/ws`:
  - On connect: add to connection manager
  - On receive text message: parse as JSON `{"type": "command", "text": "..."}`
  - For now (no game logic): publish a `player.command` event with the text, then send back `{"type": "echo", "text": "..."}` as acknowledgment
  - On disconnect: remove from connection manager

- Wire the event bus wildcard to broadcast: when any event is published, forward it to all WebSocket clients as `{"type": "game_event", "event": {...}}`

**Tests:** `test_websocket.py`
- Connect to WebSocket → connection accepted
- Send a command → receive echo response
- Published event → received by connected client as `game_event`
- Disconnect → no errors, client removed

**Acceptance:** All `test_websocket.py` tests pass. Can connect from browser dev console and exchange messages.

---

## Task 0.6: Vue Frontend — Chat Interface

**What:** Minimal Vue 3 frontend with a chat log and text input, connected via WebSocket.

**Files:**
- `frontend/package.json` — dependencies: vue, typescript, vite, tailwind, pinia
- `frontend/vite.config.ts` — dev server on `:5173`, proxy `/api` and `/ws` to `:8080`
- `frontend/src/App.vue` — root component
- `frontend/src/components/ChatLog.vue` — scrollable message list
- `frontend/src/components/CommandInput.vue` — text input with enter-to-send
- `frontend/src/stores/connection.ts` — Pinia store managing WebSocket connection
- `frontend/src/stores/game.ts` — Pinia store for game state (message log for now)
- `frontend/src/composables/useWebSocket.ts` — WebSocket connection composable

**Deliverables:**
- On page load: connect to `ws://localhost:8080/ws`
- Display connection status (connected / disconnected / reconnecting)
- Text input at the bottom. On enter: send `{"type": "command", "text": "<input>"}` via WebSocket, clear input.
- Chat log: scrollable list of messages. Each message has a sender label ("You" for player input, "System" for echo/events) and text content.
- When a `game_event` or `echo` message arrives via WebSocket, append to the chat log.
- Auto-scroll to bottom on new messages.
- Minimal styling with Tailwind: dark background, monospace or clean font, clear visual distinction between player input and system output.

**Layout:**
```
┌──────────────────────────────────────────────────────┐
│  Harsh Realm                          [●] Connected  │
├──────────────────────────────────────────────────────┤
│                                                       │
│  [scrollable chat log area]                           │
│                                                       │
├──────────────────────────────────────────────────────┤
│ > [text input]                                        │
└──────────────────────────────────────────────────────┘
```

**Tests:** Manual verification (or Playwright E2E if time permits):
- Page loads without errors
- WebSocket connects (status shows "Connected")
- Type text, press enter → message appears in log as "You: ..."
- Echo response appears as "System: ..."

**Acceptance:** Frontend renders, WebSocket connection works, messages round-trip through the server.

---

## Task 0.7: Integration Verification

**What:** Verify the full stack works end-to-end.

**Deliverables:**
- A simple integration test or script that:
  1. Starts the server (or uses test client)
  2. Creates a world via `POST /api/worlds`
  3. Loads the world via `POST /api/worlds/load`
  4. Connects via WebSocket
  5. Sends a command message
  6. Verifies the echo response
  7. Verifies the event was logged to the `event_log` table in the SQLite database
  8. Creates a save snapshot
  9. Verifies the snapshot file exists

- Update `CLAUDE.md` with:
  - Current state: "Milestone 0 complete"
  - Any deviations from the spec or issues discovered

**Acceptance:** Integration test passes. All individual task tests pass. `pytest` runs clean.

---

## Dependency Graph

```
Task 0.1 (scaffolding)
  ↓
Task 0.2 (database)    Task 0.3 (event bus)
  ↓                      ↓
Task 0.4 (REST API) ←───┘
  ↓
Task 0.5 (WebSocket)
  ↓
Task 0.6 (Vue frontend)
  ↓
Task 0.7 (integration)
```

Tasks 0.2 and 0.3 can be done in parallel. Everything else is sequential.

---

## Notes for the Coding Agent

- Read AGENTS.md before writing any code. Follow all conventions.
- The database schema should include ALL tables from the architecture doc, even though most won't be used until later milestones. This avoids schema migrations later.
- The WebSocket echo behavior is temporary — it will be replaced by the GM Controller in Milestone 1.
- Keep the frontend minimal. No fancy animations, no complex layouts. A dark-themed chat window that works. Tailwind utility classes only.
- If you encounter a decision not covered by the spec, make the simplest choice and document it in a comment.
