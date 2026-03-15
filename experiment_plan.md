# Outpost 3 — Experiment Plan

## Vision
A colony simulation game where a human faction sets out to colonize a distant star system. The core experience is **emergent narrative from simulation** — watching (and eventually directing) interconnected systems produce stories worth reading.

## Architecture
- **Backend:** Python + FastAPI + uvicorn (rapid prototyping; hot-path can migrate to Rust later)
- **Frontend:** Vue 3 + Pinia + Tailwind CSS + Vite
- **Communication:** WebSocket (real-time event push) + REST (commands, state queries)
- **Deployment:** Docker Compose (backend + frontend services)
- **UI Pattern:** Windowed panels (drag/resize/minimize/z-index), no router — single-page app

## Key Design Decisions

### Narrative Engine
- **Events are structured data** (type, severity, entities, quantities, timestamp)
- **Narrative is a view** rendered from event data via templates
- Templates use mad-libs style substitution from event fields
- Architecture supports swapping template renderer for LLM renderer later
- No coupling between simulation logic and display text

### Simulation
- **Smallest time unit:** 1 day
- **Speed levels:** 1x, 5x, 25x, 100x, max
- **Auto-pause system:** Extensible — configurable per event type. Initially auto-pauses on all major milestones.
- **Decision-making:** Rules generate candidate actions, weighted random selects from them
- **Target:** Colony ship arc (build → search → travel → found) plays out in ~2-5 minutes at max speed (~50 game-years, ~18,000 ticks)

### Frontend
- Fresh codebase, borrowing panel/layout/websocket patterns from harsh_realm
- No router — all state in Pinia stores, panels appear/disappear
- Custom PanelWindow with drag/resize/minimize/close/z-index management

---

## Milestone 0: Scaffolding + Time Engine
**Goal:** Prove the full stack works end-to-end.

**Deliverables:**
- Docker Compose starts backend + frontend
- Vue app with windowed panels (Time Controls + Event Log)
- Time engine ticks forward, emitting day/year over WebSocket
- Play/pause/speed controls (5 levels)
- Event log panel shows tick events streaming in
- Auto-pause fires (placeholder: pauses every 365 days as proof of concept)

**Done when:** Browser shows panels, clock ticks, speed controls work, pause/resume works, events stream into the log.

**Estimated effort:** 1 session

---

## Milestone 1: Colony Ship Arc (Observe Only)
**Goal:** Prove "watching a simulation feels like something." The player observes (no commands yet) as a faction builds a colony ship, searches for a destination star, travels there, and founds a colony.

**Narrative arc:**
1. **Preparation Phase** — Faction gathers resources, builds ship components, recruits colonists
2. **Search Phase** — Probes/telescopes survey candidate star systems, evaluate habitability
3. **Transit Phase** — Ship departs, journey events (resource consumption, random encounters, morale)
4. **Founding Phase** — Arrival, landing site selection, colony established → **auto-pause, milestone complete**

**Simulation systems needed:**
- Resource accumulation (simplified: materials, fuel, food, population)
- Ship construction progress (% complete, component milestones)
- Star system generation (candidates with habitability scores)
- Transit simulation (distance → time, resource burn rate, random events)
- Milestone detection + auto-pause

**Narrative systems needed:**
- Event types: resource_milestone, construction_progress, system_discovered, departure, transit_event, arrival, colony_founded
- Template renderer with ~20-30 canned templates
- Event severity levels (info, notable, critical) for log filtering

**Done when:** You can click Play, watch the entire arc unfold in the event log with readable narrative, and it auto-pauses at colony founding. The arc feels like a coherent story, not a list of numbers changing.

**Design questions to answer during this milestone:**
- How much randomness makes the arc replayable vs. feeling arbitrary?
- Is the pacing right? (Are any phases boring? Too fast?)
- What's the first thing you want to click/command when watching?

**Estimated effort:** 3-4 sessions

---

## Milestone 2: Player Directives
**Goal:** Add player commands that influence the simulation during the colony ship arc.

Scope TBD based on Milestone 1 learnings — likely:
- Choose what to prioritize (ship construction vs. resource gathering)
- Select destination star system from candidates
- Manage transit resource allocation

---

## Milestone 3: Colony Management
**Goal:** Post-founding simulation — buildings, population needs, resource chains.

---

## Milestone 4+: Depth
- Multiple colonies / expansion
- Automation policies
- Trade / inter-colony logistics
- Wormhole gates
- Victory conditions

---

## Development Principles
1. **Playtest after every iteration** — if it's not interesting to watch, don't add more systems
2. **Events are data, narrative is a view** — never bake display text into simulation logic
3. **Observe first, interact later** — simulation must generate interesting behavior autonomously before adding player controls
4. **Time-boxed sessions** — each session has a clear deliverable
5. **Docker Compose for reproducibility** — `docker compose up` always works
