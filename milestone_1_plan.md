# Milestone 1: Colony Ship Arc (Observe Only)

## Goal
Prove "watching a simulation feels like something." The player observes (no
commands) as a generation ship departs for a distant star, navigates challenges
during transit, surveys the destination system, and founds a colony — or
rejects the system and searches again.

**Done when:** You click Play, watch the full arc unfold across narrative log +
dashboard panels, and it auto-pauses at colony founding. The experience feels
like a coherent story with moments of tension, not a spreadsheet incrementing.
Running 3+ times produces meaningfully different stories.

**Estimated effort:** 5–6 sessions

---

## Architecture Decisions

### ECS-Lite
Entities are integer IDs. Components are dataclasses stored in the World as
`dict[int, ComponentType]`. Systems are classes with `tick(world, game_time) →
list[GameEvent]`. The World provides typed queries:
`world.get_components(Resources, ShipConstruction)` returns matching entities.

No archetype tables, no bitmasking. Clean separation that grows toward a real
ECS if needed. World has `to_dict()`/`from_dict()` from day 1 for future
save/load.

**Key types for M1:**

Entities:
- Faction (1) — owns the ship, tracks phase
- ColonyShip (1) — resources, position, velocity, crew
- Notable (5–8) — named individuals with roles and traits
- StarSystem (3–5) — discovered candidate systems
- Planet (varies) — belong to star systems

Components:
- FactionPhase (LOADOUT → SEARCH → TRANSIT → SURVEY → FOUNDING)
- Resources (materials, fuel, food, water, spare_parts)
- Population (total, by_role breakdown, births_yr, deaths_yr, morale)
- ShipSystems (hull_integrity, engines, life_support, sensors — each 0–1)
- OrbitalPosition (for in-system movement during Survey)
- Velocity (current_c, target_c — fraction of lightspeed)
- Notable (name, role, age, traits, alive)
- StarSystem (name, position_ly, distance_ly, habitability_score)
- Planet (type, size, atmosphere, water, minerals, habitability)
- SurveyProgress (per-planet survey completion)
- Loadout (starting configuration record — immutable after departure)

Systems (execution order):
1. LoadoutSystem — generates starting configuration
2. SearchSystem — discovers star systems, selects destination
3. TransitSystem — moves ship, consumes resources, ages population
4. PopulationSystem — births, deaths, aging, role assignments
5. ShipMaintenanceSystem — degradation, breakdowns, repairs
6. BehaviorSystem — evaluates conditions, produces deliberation records, picks actions
7. SurveySystem — in-system planet survey
8. MilestoneSystem — detects phase transitions, founding condition
9. ReportingSystem — periodic summary events
10. NarrativeSystem — observes changes, emits story events

### 15 Speed Levels
Defined as days per 2.5 seconds of real time:

| Level | Days/2.5s | Delay (ms) | Use case                          |
|-------|-----------|------------|-----------------------------------|
|  1    |     1     |   2500     | Watching events in detail         |
|  2    |     2     |   1250     | Slow real-time                    |
|  3    |     5     |    500     | Daily events readable             |
|  4    |    10     |    250     | Fast daily                        |
|  5    |    25     |    100     | ~monthly pace                     |
|  6    |    50     |     50     | Bi-monthly                        |
|  7    |   100     |     25     | ~quarterly                        |
|  8    |   250     |     10     | Yearly review (batch ticks)       |
|  9    |   500     |      5     | Multi-year skip (batch)           |
| 10    |  1000     |    2.5     | Decade skip (batch)               |
| 11    |  2500     |      1     | Fast forward (batch)              |
| 12    |  5000     |    0.5     | Very fast forward (batch)         |
| 13    | 10000     |   0.25     | Ultra fast (batch)                |
| 14    | 25000     |    0.1     | Near-max (batch)                  |
| 15    | 36500     |   0.07     | Max ~100 years/2.5s (batch)       |

Levels 1–7: 1 tick per loop iteration with sleep.
Levels 8–15: batch N ticks per iteration, one WebSocket push per batch.
State updates capped at ~30 pushes/sec regardless of tick rate.
Events always pushed immediately (they're infrequent by design).

### Travel Model
Fusion rockets at relativistic velocities (< 0.7c). Ship accelerates to
cruise velocity, coasts, decelerates on approach.

- Alpha Centauri (~4 ly) at 0.5c ≈ 9 years
- Typical target (10–25 ly) at 0.5c ≈ 20–50 years
- Time dilation ignored for M1 (one clock)
- Fuel consumption tied to acceleration/deceleration phases

Generation ship: crew lives full lives during transit. Births, deaths, aging,
generational turnover. Future milestone adds cryosleep ship variant.

### Phase System
Phases are a component on the Faction entity. Systems check current phase to
decide if they run. Transitions happen when a system detects completion.

```
LOADOUT → SEARCH → TRANSIT → SURVEY ──→ FOUNDING (done)
                      ↑          │
                      └──────────┘  (reject system)
```

### Behavior System (Crew Decision-Making)
Hybrid behavior tree + weighted utility scoring. The tree defines the
structure of decisions; leaf nodes score options by weighted factors.

**Architecture:**
- `BehaviorSystem` runs each tick (or periodically — not every day)
- Checks conditions against thresholds (food < 30%, hull < 50%, etc.)
- When a condition triggers, generates a `Deliberation`:
  - Trigger: what condition caused evaluation
  - Options: list of possible actions with scores and tradeoffs
  - Advocates: which Notable argued for which option (based on traits)
  - Chosen: the selected action + reason
  - Rejected: other options + why they lost
- Deliberation is attached to the GameEvent as structured data
- NarrativeSystem renders it at configurable detail levels

**Example deliberation:**
```
Trigger: Food stores at 28% (critical threshold)
Options evaluated:
  1. Reduce rations to 80% [score: 0.72]
     + Extends food 40 days | − Morale impact −15
     Advocated by: Dr. Okafor (crew welfare)
  2. Divert to asteroid field [score: 0.31]
     + Potential food synthesis | − 2.3 years off course, fuel cost
     Advocated by: Chief Engineer Vasquez (resourceful)
  3. Cull hydroponics bay C [score: 0.18]
     + Immediate food boost | − Permanent production loss
     Advocated by: none
Decision: Reduce rations to 80% (highest utility)
```

**Notable traits that bias advocacy:**
- cautious / bold — risk tolerance
- crew_welfare / mission_focus — priority axis
- resourceful / conservative — novel solutions vs. proven methods
- optimistic / pragmatic — assessment of uncertain outcomes

### Named Notable Individuals
5–8 generated per playthrough with:
- Name (procedural — cultural variety)
- Role: Captain, Chief Engineer, Chief Medical Officer, Head of Agriculture,
  Chief Scientist, Security Chief, Navigator, Quartermaster
- Age (25–55 at departure)
- Traits (2–3 from the trait list)
- Personality summary (derived from traits, used in narrative)

Notables age during transit. When one dies (old age, accident, illness),
a successor is promoted — itself a narrative event. Character events during
transit: disagreements, breakthroughs, relationships, leadership challenges.

### Loadout Generation (Preparation Phase Replacement)
The ship is already built. The LOADOUT phase auto-generates starting
conditions with randomness:

- **Crew size:** 500–2000 colonists
- **Resources:** materials, fuel, food, water, spare_parts — each randomized
  within viable ranges (enough to complete transit with margin, but not
  comfortably)
- **Ship systems:** hull, engines, life_support, sensors — starting condition
  0.85–1.0 (new but not perfect)
- **Equipment/modules:** hydroponics capacity, manufacturing capability,
  medical bay level, sensor range
- **Notable individuals:** 5–8 generated with roles and traits

Loadout is recorded as an immutable component for later reference.
Phase immediately transitions to SEARCH after generation.

### Display Separation
- **Event Log:** Story beats, deliberations, milestones, character events.
  NOT every tick, NOT resource deltas.
- **Ship Status Panel:** Live dashboard — resources table with current values
  and rates, ship systems health bars, current phase, crew count, notable
  individuals roster.
- **Resource Graph:** Line charts of resource levels over time. Historical
  data in ring buffer.
- **Star Map:** Discovered systems, ship position, travel path.
- **Periodic Summaries:** ReportingSystem emits summaries into event log at
  intervals that adapt to phase (monthly during Survey, yearly during Transit).

Values freeze during pause. Rates show "PAUSED." No projections.

### Narrative Design
**Event density targets by phase:**
- LOADOUT: 5–10 events (crew manifest, loadout summary, notable introductions)
- SEARCH: 2–3 per game-year (system discoveries, evaluations, destination selected)
- TRANSIT: 1–2 per game-year baseline + triggered events (decisions, character
  events, periodic summaries). Long quiet stretches are OK — the dashboard
  shows the ship is alive.
- SURVEY: 3–5 per game-year (planet surveys, evaluations, reject/accept)
- FOUNDING: 5–8 events (approach, landing, first structures, colony named)

**Narrative rendering:** Full transparency by default. Deliberations rendered
with trigger, options evaluated, advocates, chosen action + reasoning.

---

## Session Breakdown

### Session 1: ECS-Lite + Speed System + Engine Refactor
**Goal:** Replace the placeholder architecture with real infrastructure.

**Backend deliverables:**
- [ ] `simulation/ecs.py` — World class:
  - Entity ID generator (incrementing int)
  - `create_entity() → int`
  - `add_component(entity_id, component)` — stores in `dict[type, dict[int, Component]]`
  - `get_component(entity_id, ComponentType) → Component | None`
  - `get_components(entity_id, *types) → tuple | None`
  - `entities_with(*ComponentTypes) → list[int]`
  - `remove_entity(entity_id)`
  - `to_dict() → dict` / `from_dict(data) → World`
- [ ] `simulation/systems.py` — System base class:
  - `order: int` (execution priority)
  - `tick(world, game_time) → list[GameEvent]`
  - `active_phases: set[Phase] | None` (None = always active)
- [ ] 15-level speed system in engine.py:
  - Replace SPEED_DELAYS with SPEED_LEVELS list of `{level, days_per_2_5s, label}`
  - Tick batching: levels 8+ process multiple ticks per loop iteration
  - `_loop()` refactored: calculate batch_size from speed level, process N
    ticks, emit events, push one state update per batch
  - WebSocket throttle: state pushes capped at ~30/sec
- [ ] Refactor main.py:
  - Engine owns a World instance
  - Engine iterates registered Systems in order each tick
  - Systems receive World + GameTime, return events
  - Replace placeholder_tick with a minimal test system
- [ ] REST endpoints updated for speed levels 1–15
- [ ] Narrative renderer unchanged (still works)

**Frontend deliverables:**
- [ ] TimeControls.vue: replace 5 buttons with slider (1–15) + speed label
- [ ] time.js store: update for new speed level values

**Done when:** `docker compose up`, Play works, slider moves through 15 speeds,
batching keeps browser responsive at level 15, test system emits events.

---

### Session 2: Loadout + Notable Individuals + Behavior System Foundation
**Goal:** Ship gets a crew and supplies. Named characters exist. The behavior
system framework is in place.

**Backend deliverables:**
- [ ] Component dataclasses: Resources, Population, ShipSystems, FactionPhase,
      Loadout, Notable (name, role, age, traits, alive)
- [ ] `simulation/generation/loadout.py` — LoadoutSystem:
  - Generates random crew size (500–2000)
  - Generates random resource levels within viable ranges
  - Generates ship system health (0.85–1.0)
  - Generates 5–8 Notable individuals with procedural names, roles, traits
  - Records immutable Loadout component
  - Emits loadout summary events + notable introduction events
  - Transitions to SEARCH phase
- [ ] `simulation/generation/names.py` — procedural name generator
  - First/last name pools with cultural variety
  - Ship name generator (used later)
  - Star system name generator (catalog numbers + proper names)
- [ ] `simulation/behavior.py` — BehaviorSystem foundation:
  - Condition evaluators: check component values against thresholds
  - Deliberation dataclass: trigger, options (list of {action, score,
    pros, cons, advocate}), chosen, rejected
  - Option scoring: weighted utility function
  - Notable advocacy: trait → preference mapping
  - For Session 2: implement 2–3 simple conditions (food low, hull damage,
    morale low) with 2–3 options each as proof of concept
  - Deliberation attached to GameEvent.data for narrative rendering
- [ ] Narrative templates: loadout summary, crew manifest, notable introductions,
      first deliberation events (~15 templates)
- [ ] Update NarrativeSystem to render Deliberation records with full
      transparency (trigger, options, advocates, decision)

**Done when:** Press Play → loadout generates → you can read about the ship's
crew and supplies in the event log → named characters appear with roles/traits
→ behavior system triggers at least one deliberation during a test scenario.

---

### Session 3: Search Phase + Transit Phase
**Goal:** Ship finds a destination and travels there. The generation ship
simulation produces human drama during the journey.

**Backend deliverables:**
- [ ] Components: StarSystem, Planet, SearchProgress, Velocity
- [ ] `SearchSystem`:
  - Periodic probability of discovering a new star system
  - Procedural generation: name, distance (5–25 ly), 2–8 planets
  - Planet generation: type (rocky/gas/ice), size, atmosphere, water, minerals
  - Habitability scoring from planet mix
  - After 3–5 discoveries, selects best candidate → TRANSIT
- [ ] `TransitSystem`:
  - Acceleration phase (weeks), cruise phase (years), deceleration (weeks)
  - Resource consumption: fuel (accel/decel), food+water (continuous)
  - Ship system degradation over time (slow, stochastic)
  - Distance tracking + ETA calculation
  - Arrival detection → SURVEY
- [ ] `PopulationSystem`:
  - Birth rate (affected by morale, food availability)
  - Death rate (age, accidents, medical capability)
  - Aging: Notable individuals age, die, get replaced
  - Role rebalancing as population changes
- [ ] `ShipMaintenanceSystem`:
  - Gradual degradation of ship systems
  - Random breakdowns (probability per system per year)
  - Repair: consumes spare_parts, takes time
  - Critical failures trigger BehaviorSystem deliberations
- [ ] Transit random events (abstracted as narrative events with costs):
  - Asteroid field: minor hull damage or avoidance maneuver (fuel cost)
  - Equipment failure: specific system degrades sharply
  - Cosmic phenomenon: flavor event (nebula, rogue planet spotted)
  - Resource discovery: divert to asteroid (abstracted: +N days, +resources)
  - Social events: crew conflict, celebration, cultural shift
- [ ] Behavior system expansions: 5–8 more conditions/options for transit
  scenarios (resource depletion, breakdowns, crew conflicts)
- [ ] Narrative templates: ~15 for Search + ~15 for Transit

**Tuning targets:**
- Search: 1–3 game-years
- Transit: 20–50 game-years depending on distance
- Transit narrative events: 1–3/year + behavior deliberations as triggered
- Notable character events: 1–2 per decade (deaths, successions, conflicts)

**Done when:** Full SEARCH → TRANSIT arc plays out. Named characters age and
die during transit. Behavior system triggers deliberations on resource/system
problems. Ship arrives at destination. Multiple runs show meaningful variation.

---

### Session 4: Ship Status Panel + Resource Graph + Star Map
**Goal:** Frontend catches up. The player can see what's happening without
reading every log entry.

**Frontend deliverables:**
- [ ] New WebSocket message type: `"snapshot"` — periodic world state push
  - Current phase, resources (values + rates), ship systems health,
    population breakdown, notable roster, position, ETA
  - Backend: snapshot serialization from World, throttled at 2–10/sec
- [ ] New Pinia store: `gameState.js`
  - Receives snapshots, exposes reactive state
  - Ring buffer for historical data (resource levels over time)
- [ ] `ShipStatus.vue` panel:
  - Current phase badge
  - Resources table: name, amount, rate/day, status indicator (ok/low/critical)
  - Ship systems: health bars for hull, engines, life_support, sensors
  - Population: total, by role, morale indicator
  - Notable roster: name, role, age, traits (compact list)
- [ ] `ResourceGraph.vue` panel:
  - SVG sparkline charts (hand-rolled, no library)
  - One line per resource, toggleable
  - Time axis shows game years
  - Scales appropriately as values change
- [ ] `StarMap.vue` panel:
  - 2D SVG scatter plot
  - Home system at center
  - Discovered systems as dots with name + habitability score
  - Selected destination highlighted
  - Ship position during transit (interpolated between home and destination)
  - Travel line from home to destination
- [ ] Layout store: default positions for all 5 panels (Time Controls, Event
      Log, Ship Status, Resource Graph, Star Map)

**Done when:** All 5 panels visible and updating. Dashboard shows live resource
values during simulation. Graphs draw meaningful curves over a full transit.
Star Map shows the journey. Everything freezes cleanly on pause.

---

### Session 5: Survey Phase + Reject Loop
**Goal:** Ship arrives, explores the system, evaluates planets, and either
founds a colony or rejects and searches again.

**Backend deliverables:**
- [ ] Components: SurveyProgress (per-planet), OrbitalPosition
- [ ] `SurveySystem`:
  - Ship enters system → begins surveying planets in order
  - In-system transit between planets (days to weeks)
  - Per-planet survey duration (weeks to months)
  - Survey reveals detailed data: atmosphere composition, water quality,
    mineral deposits, hazards, seasonal patterns
  - Refined habitability score post-survey (may differ from Search estimate)
  - Decision point: best planet habitability > threshold?
    - Yes → select landing site → FOUNDING
    - No → reject system → back to SEARCH (new search from current position)
- [ ] Reject loop handling:
  - Ship must travel to next candidate (new transit)
  - Cap at 3 rejections, then force-accept best available
  - Each rejection is a narrative event with deliberation record
- [ ] Behavior system: survey-phase deliberations
  - "Is this planet good enough?" with trait-based advocacy
  - Risk tolerance affects acceptance threshold
  - Desperation increases with each rejection (resources depleting)
- [ ] Narrative templates: ~10 for Survey (planet reports, deliberations,
      rejection events, "this is the one" moments)

**Tuning targets:**
- Survey per system: 0.5–2 game-years
- In-system transit: days to weeks per planet
- 60–70% chance first system is accepted (most runs don't loop)
- Reject loop adds 25–55 years per iteration (new search + transit + survey)

**Done when:** Survey phase works end-to-end. Rejection triggers new search
cycle. Star Map updates with new systems. Multiple runs show both accept-first
and reject-then-accept paths. Deliberation records explain why systems were
accepted or rejected.

---

### Session 6: Founding + Narrative Polish + End-to-End Tuning
**Goal:** Complete the arc. Polish. Playtest until it feels good.

**Backend deliverables:**
- [ ] `FoundingSystem`:
  - Landing site selection (best surveyed planet)
  - Descent sequence events
  - First structures established
  - Colony named (procedural or from Notable suggestion)
  - FOUNDING milestone → auto-pause
  - Colony starting conditions reflect journey:
    - Resources remaining from transit
    - Population (grown or shrunk)
    - Ship system health (salvageable equipment)
    - Morale state
    - If "limped in": weakened start clearly narrated
- [ ] Narrative templates: ~5 for Founding
- [ ] **Narrative polish pass:**
  - Review all ~60 templates
  - Add 2–3 variant lists per event type for replayability
  - Ensure consistent tone across phases
  - Verify deliberation rendering reads well at all detail levels
  - Fill narrative gaps (abrupt transitions, missing context)
- [ ] **Pacing pass (5+ full playthroughs):**
  - Phase durations: any too long/short?
  - Event density: too sparse? Flooding?
  - Transit feel: is the "long quiet journey" bearable with summaries?
  - Behavior system: are deliberations interesting or repetitive?
  - Notable characters: do you care when one dies?
  - Speed levels: which speed feels right for each phase?
  - Auto-pause triggers: add more beyond founding?
- [ ] **Edge cases:**
  - Resources fully depleted mid-transit → ship limps on, colony severely
    weakened (not game over)
  - All Notables die → new ones promoted from aggregate crew
  - 3 rejections → force-accept with "desperate founding" narrative
  - Population drops below viable threshold → emergency measures narrative

**Frontend deliverables:**
- [ ] Event log: fix severity filter to threshold logic ("notable+" = notable,
      critical, milestone)
- [ ] Panel layout polish: sensible defaults, nothing overlapping
- [ ] Any UX issues from playtesting

**Done when:** Run the full arc 3+ times and each one:
1. Feels like a distinct story (meaningful variation between runs)
2. Has at least 2–3 "oh, interesting" moments
3. Has at least 1 deliberation you want to read carefully
4. Takes 3–8 minutes at a comfortable speed
5. Named characters feel like people, not labels
6. Star Map shows a meaningful journey
7. Founding feels earned — especially if the journey was rough

---

## Design Questions to Answer During M1

1. **Is the loadout meaningful?** Do different starting conditions produce
   noticeably different journeys, or does everything converge?

2. **Are deliberations interesting?** Do you read them, or skip past? Is full
   transparency the right default, or is it too verbose?

3. **Do Notable characters matter?** When Captain Vasquez dies in Year 31, do
   you feel something? Or is it just a name in a log entry?

4. **Is transit too long?** 20–50 years is a lot of simulation. Do periodic
   summaries + occasional events carry it, or does it drag?

5. **Does the reject loop add replayability or frustration?** When the first
   system is rejected, is it "ooh, what happens next" or "ugh, more waiting"?

6. **What's the first thing you want to click?** This directly informs M2.

7. **Which speed level feels right for each phase?** This informs whether we
   need phase-aware auto-speed suggestions.

---

## Decisions Log

| Decision | Choice | Rationale |
|----------|--------|-----------|
| State management | ECS-Lite with World container | Scales toward real ECS, typed queries keep it clean |
| Speed levels | 15 (1 day/2.5s → 36500 days/2.5s) | Matches Josh's original design, logarithmic range |
| Travel model | Sub-light fusion, < 0.7c | Grounds the setting, enables generation ship drama |
| Ship variant | Generation ship (M1), cryosleep later | Generation ship = richer transit narrative |
| Preparation phase | Auto-generated loadout (ship is built) | Focuses M1 on the interesting parts |
| Population model | Named notables (5–8) + aggregate crew | Human faces on decisions without simulating 1000 individuals |
| Decision transparency | Full (trigger, options, scores, advocates) | Makes observe-only compelling |
| Mid-transit gathering | Abstracted events (time + resource cost) | Avoids sub-simulation complexity for M1 |
| Total failure | Ship limps on, colony weakened | Every run reaches a conclusion |
| Time dilation | Ignored for M1 | One clock is enough complexity |
| System rejection | Allowed, capped at 3 | Adds replayability without infinite loops |
| M1 scope | Through survey + founding (expanded) | Richer arc worth 5–6 sessions |
