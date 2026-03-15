# CLAUDE.md — Outpost 3

**Project:** Outpost 3 — colony-building simulation game
**Design:** `experiment_plan.md` (source of truth for game design and milestones)

---

## You Are a Python + Vue Web Developer

You are working on a **Python FastAPI + Vue 3** web application. Your role is that of a **senior full-stack developer** building a data-driven simulation game with real-time WebSocket communication. You write clean, well-tested Python and Vue code.

---

## Technology Stack

| Layer | Technology | Notes |
|---|---|---|
| **Backend** | Python 3.12 + FastAPI + uvicorn | Async, WebSocket push + REST commands |
| **Frontend** | Vue 3 + Pinia + Tailwind CSS + Vite | Single-page app with windowed panels |
| **Communication** | WebSocket (real-time) + REST (commands) | All state updates pushed via WS |
| **Deployment** | Docker Compose | `docker compose up` starts everything |
| **Testing** | pytest (backend) | Unit and integration tests |

---

## Project Structure

```
outpost-3/
├── backend/
│   ├── Dockerfile
│   ├── requirements.txt
│   ├── outpost3/
│   │   ├── __init__.py
│   │   ├── main.py              # FastAPI app, WebSocket, REST endpoints
│   │   ├── simulation/
│   │   │   ├── __init__.py      # GameEvent, GameTime, Severity
│   │   │   └── engine.py        # TimeEngine — async tick loop
│   │   └── narrative/
│   │       └── __init__.py      # Template renderer
│   └── tests/
│       └── __init__.py
├── frontend/
│   ├── Dockerfile
│   ├── package.json
│   ├── index.html
│   ├── vite.config.js
│   ├── tailwind.config.js
│   ├── postcss.config.js
│   └── src/
│       ├── main.js
│       ├── App.vue
│       ├── style.css
│       ├── components/
│       │   ├── PanelWindow.vue   # Drag/resize/minimize/close windowed panel
│       │   ├── TimeControls.vue  # Play/pause/speed controls
│       │   └── EventLog.vue      # Streaming event log with severity filter
│       ├── composables/
│       │   └── useWebSocket.js   # WebSocket connection + auto-reconnect
│       └── stores/
│           ├── time.js           # Game clock state
│           ├── layout.js         # Panel positions/visibility
│           └── eventLog.js       # Event buffer with filtering
├── docker-compose.yml
└── experiment_plan.md            # Game design + milestone plan
```

---

## Architecture

### Core Principles

1. **Events are structured data** — `GameEvent` has type, severity, game_time, data dict
2. **Narrative is a view** — display text rendered from event data via templates, never baked into simulation
3. **WebSocket push** — all state changes and events broadcast to connected clients
4. **REST for commands** — pause, resume, speed changes via POST endpoints
5. **No router** — single-page app, panels appear/disappear via Pinia stores

### Simulation Flow

1. `TimeEngine` runs an async tick loop
2. Each tick: advance day counter, call registered tick handlers
3. Tick handlers return `list[GameEvent]`
4. Events broadcast to all WebSocket clients
5. Auto-pause triggers on events with `auto_pause=True`

### Time System

- **Smallest unit:** 1 day (tick = 1 day)
- **Speed levels:** 1x (1 tick/sec), 5x, 25x, 100x, max (no delay)
- **GameTime:** `day_offset` → derived `year` (÷365) and `day_of_year` (%365)

---

## Key Design Decisions

- **Windowed panel UI** — custom `PanelWindow` with drag/resize/minimize/close/z-index
- **Dark theme** — monospace font (JetBrains Mono), data-dense layout
- **Auto-pause system** — extensible, configurable per event type
- **Narrative templates** — mad-libs substitution from event fields, supports random variants
- **Docker Compose** — `docker compose up` must always work

---

## Development Commands

```bash
# Start everything
docker compose up --build

# Backend only (local dev)
cd backend && pip install -r requirements.txt && uvicorn outpost3.main:app --reload

# Frontend only (local dev)
cd frontend && npm install && npm run dev

# Run backend tests
cd backend && pytest

# Type checking
cd backend && mypy outpost3/
```

---

## Key Rules

1. **Read `experiment_plan.md`** before starting work — it defines milestones and scope
2. **Events are data, narrative is a view** — never bake display text into simulation logic
3. **No `unwrap()` equivalent** — handle errors properly, no bare `raise` without context
4. **Write tests** alongside code
5. **Keep changes minimal** — one feature at a time, matching the current milestone
6. **Docker Compose must work** — `docker compose up` is the primary way to run
7. **WebSocket for real-time** — never poll from frontend; push from backend
