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

### State Management (ECS-Lite)
- Entities are integer IDs, components are dataclasses, systems are classes with `tick()` methods
- World container provides typed queries: `world.get_components(Resources, ShipConstruction)`
- No archetype tables or bitmasking — clean separation that scales toward full ECS later
- Systems registered with engine, executed in order each tick

### Simulation
- **Smallest time unit:** 1 day
- **Speed levels:** 15 levels from 1 day/2.5s to 36,500 days/2.5s (~100 years/2.5s). Tick batching at high speeds to prevent browser flooding.
- **Auto-pause system:** Extensible — configurable per event type. Initially auto-pauses on all major milestones.
- **Decision-making:** Hybrid behavior tree + weighted utility scoring. Crew evaluates conditions, scores options, Named Notables advocate based on personality traits. Full deliberation records exposed to player.
- **Travel model:** Sub-light fusion rockets (< 0.7c), generation ship for M1, cryosleep variant planned for M2+. Time dilation ignored for M1.
- **Failure model:** No game-over states. Ship limps on if critically damaged; colony starts weakened.

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
**Goal:** Prove "watching a simulation feels like something." The player observes
(no commands) as a generation ship departs for a distant star, navigates challenges
during transit, surveys the destination system, and founds a colony — or rejects
the system and searches again.

**Travel model:** Sub-light fusion rockets at relativistic velocities (< 0.7c).
Generation ship — crew lives full lives during transit. Time dilation ignored for M1.

**Narrative arc:**
1. **Loadout Phase** — Ship is built. Auto-generate random starting supplies, crew
   composition (500–2000 colonists), and 5–8 named Notable individuals with roles
   and personality traits. Immediate transition to Search.
2. **Search Phase** — Discover 3–5 candidate star systems with procedurally generated
   planets. Evaluate habitability, select best candidate.
3. **Transit Phase** — Generation ship travels 20–50 years. Resource consumption, ship
   degradation, population births/deaths/aging. Random events (abstracted with time/
   resource costs). Crew behavior system produces deliberation records with full
   transparency (trigger, options evaluated, advocates, chosen action).
4. **Survey Phase** — Ship enters system, surveys planets. Evaluates habitability.
   Can REJECT system → loops back to Search (capped at 3 rejections).
5. **Founding Phase** — Landing, colony established → auto-pause. Colony starting
   conditions reflect journey quality (weakened if ship limped in).

**Architecture:** ECS-Lite (World container, dataclass components, System classes
with typed queries). 15 speed levels (1 day/2.5s → 36,500 days/2.5s) with tick
batching. Behavior system: hybrid behavior tree + weighted utility scoring producing
deliberation records. Named Notable individuals with traits biasing their advocacy.

**Frontend panels:** Time Controls (15-level speed slider), Event Log (narrative +
deliberations), Ship Status (resources + ship systems + notable roster), Resource
Graph (SVG sparklines), Star Map (discovered systems + ship position).

**See:** `docs/milestone_1_plan.md` for full session breakdown (6 sessions),
architecture details, tuning targets, and open questions.

**Estimated effort:** 5–6 sessions

---

## Milestone 2: Player Directives
**Goal:** Add player commands that influence the simulation during the colony ship arc.

Scope TBD based on Milestone 1 learnings — likely:
- Configure starting loadout (crew size, resource allocation, equipment priorities)
- Select destination star system from candidates
- Override crew decisions during transit (accept/reject deliberation outcomes)
- Accept/reject star systems during Survey (instead of auto-decision)
- Cryosleep ship variant as alternative to generation ship

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
