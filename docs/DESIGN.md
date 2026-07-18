# Outpost 3 — Design Document

**Working title:** Outpost 3 (spiritual successor to the original *Outpost*)
**Status:** Core design locked; pre-implementation. This document supersedes all prior design iterations (Bevy+Rust, Rust/Actix/HTMX Phases 0–6, Python/FastAPI+Vue, and the Godot+C# reboot).
**Scope of this doc:** Captures decided design and architecture. Items marked **[TBD]** are deliberately deferred, not overlooked.

---

## 1. Vision

A turn-based grand-strategy game about colonizing an entire **star system**, starting from a single colony. Depth comes from **breadth and interconnection** — many structures, commodities, production chains, and decisions spanning many sites — rather than from spatial simulation (no dig-sim, no building-placement puzzle). The player grows a system-wide industrial civilization whose capstone achievement is launching an **interstellar expedition**.

The founding motivation: colony games almost always stop at one site, or a few sites on one planet. Outpost 3's distinguishing scope is the **whole star system** — multiple worlds, orbital infrastructure, inter-body logistics, and system-scale megaprojects.

---

## 2. Design Pillars

1. **One loop, applied fractally.** The same core loop — *specialize, connect, build* — operates at every scope, from a single colony up to the whole system. Learn it once, apply it at bigger scope. This is what makes a multi-scope game learnable instead of four games bolted together.
2. **Breadth over spatial simulation.** Richness lives in the content graph (structures, goods, chains, decisions), not in belts, routing, or placement. Logistics is abstract *within* a colony and explicit *between* colonies/worlds.
3. **Automation-first, management-by-exception.** The player can hand-direct anything, but good automation via directives is the encouraged, rewarded path. The game surfaces which colonies need attention; the rest run themselves.
4. **Tunable difficulty (Factorio ethos).** A clean economic-survival core, with optional layers (adversary, existential clock) that scale the game from peaceful sandbox to brutal campaign.
5. **Core-loop-first discipline.** Prior attempts stalled by prioritizing exploration/travel/UI before the colony economy was solid. Colony economy and the turn model come first; everything else layers on a proven core.
6. **Crisp scope boundaries.** With nested scopes, the player must always know *which zoom solves which problem*. Non-overlapping responsibilities per scope is a hard discipline, not a nicety.

---

## 3. Core Loop

At every scope the player does three things:

- **Specialize** a node — give it a role (a colony's economic focus; a world's system-role).
- **Connect** flows — move goods (and people) between nodes.
- **Build** projects — structures at colony scope, infrastructure at planetary scope, megaprojects at system scope.

The verbs stay constant across scopes. Only the abstraction and the stakes change.

---

## 4. Game Structure: Scopes & Cadences

Two turn cadences, one of which is multi-zoom.

| Cadence | Scope | Speed | What the player does |
|---|---|---|---|
| **Colony turn** | A single colony's internal economy | Fast (1 sol) | Set/adjust a colony's role and directives |
| **Strategic turn** | Everything above the colony | Slow (~1 month / 30 sols, configurable) | Expansion, logistics, projects, exploration |

The **strategic layer is a single multi-zoom view**, not several separate layers:

- **Planet zoom** — colonies on a planet + the infrastructure connecting them.
- **Orbital zoom** — stations around a body.
- **System zoom** — all bodies, inter-body logistics, and megaprojects.

This revives the original "two zoom layers" instinct: colony ↔ strategic, with strategic now zooming all the way out to the full system.

---

## 4A. Pacing, Rhythm & Arc

**Core rhythm — strategic/interrupt-primary.** The player's primary time-advance is the strategic turn, or better, **"advance until something needs me"** (the *wait N turns unless interrupted* mechanism from the original vision — this is the core time control, not a nice-to-have). Colonies auto-run their sols on directives underneath; the player drops into a colony only when it is flagged (management-by-exception). The colony-sol clock is the simulation's resolution — mostly *watched as it resolves*, not clicked through. Hand-advancing colonies sol-by-sol is explicitly **not** the primary interaction.

**Consequence:** the quality and cadence of *interrupts* is the moment-to-moment game. The exception/notification system is a **first-class system**, not UI polish (see §16).

**Playthrough arc — three *unequal* phases:**
- **Early (short hump, a few hours)** — single-colony survival + first expansion. Tight, hands-on, learning the loop; environmental pressure most acute. Onboarding, not the bulk of playtime.
- **Mid (the long body — most of the ~40h)** — planetary network + orbital. Specialization/trade come online; the player shifts from hands-on to directing; management-by-exception carries the load.
- **Late (climax)** — system-scale + the megaproject race toward the interstellar expedition (under the existential clock if enabled).

**Target length:** long / Factorio-like, **~40h+ at default difficulty**. Tunable difficulty and the optional clock stretch (sandbox) or compress (brutal) around this default.

**What ~40h+ commits us to.** Achievable for this genre (4X/grand-strategy routinely hit it), but the engagement engine is **not** Factorio's throughput optimization — logistics is abstracted away. The long-game engine here is **scope-escalation (colony → planet → system) + tech/commodity breadth + emergent crises + the megaproject goal.** That works, but it is **content-hungry** and depends on:
1. **Deep content** — a large tech web, rich commodity graph, and many event/crisis types, so the world keeps generating fresh decisions across 40h. Sharply raises the stakes on the harness (§13) as the content-authoring/validation engine.
2. **High-quality interrupt cadence** — "wait until interrupted" is the primary interaction for 40h, so interrupt *rate and meaningfulness* must stay engaging throughout.

**Research-rate anchor:** tech-tree completion should stretch to **late in the ~40h arc** at default difficulty, so tech prioritization stays meaningful the whole way (tuned via the harness).

---

## 5. The Colony Layer

A **management screen, not a spatial grid.** The old grid + isometric placement mechanic is dropped. A colony is a set of buildings + stats + a build queue, plus a **non-interactive flavor image** that reflects its size/type/state for visual identity (no placement, no renderer burden).

**Commodities are colony-pooled.** Buildings draw from and add to the colony's shared stockpile. There are no belts or intra-colony routing. The interesting question is "does this colony stay balanced," not "did I route the ore." Logistics is abstract within a colony, explicit between colonies.

**Two scarce resources force specialization: build slots and labor.** A colony cannot build every chain — finite, tech-gated build capacity (the "underground capacity" concept, expressed as a slot limit rather than literal digging) plus finite labor means the player must choose a focus. Specialization is therefore a *consequence* of constraints, never a rule imposed on the player.

**You specialize to grow, not to survive.** Survival basics (food, water, oxygen, power, housing) are cheap and few-slot, so a colony can always be self-sufficient in essentials. The advanced/growth chains are what compete for slots and labor. Interdependence is **encouraged, not enforced** — a colony can limp along solo on basics, but specializing and trading is how you grow and get rich.

**Decision drivers (primary vs modifier):**
- *Primary (what the player actively decides):* **environmental fit** (a site's resources/hazards give it a starting hand) and **specialization** (its role in the network).
- *Modifier (shapes the menu, not a separate decision):* **risk vs throughput** and **tech-gating**.

**The decision is continuous, but at the directive level.** The player re-tunes colonies as conditions drift — but by adjusting a colony's role/priorities, not micromanaging construction queues. Manual per-colony control is always available; strong automation is the encouraged default; the game surfaces colonies needing attention (management-by-exception).

**Dependency:** Continuous re-tuning is only engaging if the world keeps drifting. The simulation must generate reasons to re-tune — resource depletion, tech unlocks shifting the optimal build, population growth changing labor, trade shifts, and events.

---

## 6. Population

**Aggregate pool per colony:** a population count, a single stability/contentment scalar, and aggregate needs (food, water, housing, etc.) met in bulk. No per-colonist simulation. Meet needs → population grows, labor is available. Neglect them → stability falls, growth stalls or reverses. **Labor derives from population × stability.**

**Population is a fluid resource.** People flow/ship between colonies along infrastructure, mirroring the commodity-flow model. This makes a colony in crisis a *decision*: evacuate (ship people out), reinforce, or triage.

**Sources:** immigration waves (colonist ships — a strategic lever and event source, dominant early) plus local growth (dominant later).

**Deliberately double-edged (asset *and* liability).** Every colonist is both labor you need and a need you must satisfy / a life you can lose. This creates a moving optimal growth rate: too fast outpaces buffers and stability crashes; too safe leaves you labor-starved and unable to build megaprojects. A colony's falling stability is a primary management-by-exception trigger.

**Note:** The "responsibility to protect" half of this tension only bites because of the threat model (§10). The two systems are coupled by design.

---

## 6A. Migration & Population Flow

Detail on how population moves. **Governing principle: migration must have friction, or geography stops mattering** — frictionless labor teleportation would collapse the environmental-fit / specialization design.

**Model — mirrors commodities (§8.1): auto base-flow + directed override.**
- **Auto pull-flow:** people drift toward attractive colonies (labor demand + available housing + good stability) and away from failing ones.
- **Player gates/incentives:** mark a colony open/closed to emigration, incentivize retention or attraction — so auto-flow is *steerable*, not loss-of-agency (prevents a struggling colony death-spiralling as people flee it).
- **Directed override:** force an evacuation or labor-rush when needed.

**Frictions:**
- **Time/distance** — moves take turns proportional to distance over infrastructure. You **cannot instantly rescue a crashing colony**; evacuation must be planned. Couples tightly to predictive early-warning interrupts (§12A): the "crash in ~N turns" warning is what buys time to start moving people.
- **Transport capacity** — consumes shipping/hauler throughput (same managed resource as inter-body logistics). Intra-planet is cheaper/faster; inter-body is slow and costly.
- **Willingness** — voluntary moves are smooth; **forced moves (evacuation, conscription) cost stability**. People aren't cargo.

**Immigration interaction:** external immigration waves land at port/gateway colonies (spaceport or orbital logistics), then distribute internally via migration — giving logistics colonies a population-gateway role.

**Cascading crises (feature + risk).** Evacuation **displaces, not erases** the problem: refugees strain receivers (overcrowding → stability hit). With forced-move cost and time friction, this enables emergent failure cascades (colony fails → evacuate → receiver overcrowds → second colony wobbles → …).
- *Feature:* a strong emergent-narrative engine — the tense, story-generating crises that fill a 40h game.
- *Risk:* without recovery levers, a cascade can death-spiral the network unrecoverably and feel unfair. The player's levers (predictive warnings, directed evacuation, open/close gates, emergency housing, external relief) must be sufficient to *fight* a cascade. **Difficulty tunes severity** (sandbox: gentle/recoverable; brutal: lethal). Balance via the harness.

---

## 7. Commodities & Production

**Depth target:** X4 / Captain of Industry-level production chains (extract → refine → manufacture → assemble, with traded intermediate goods and facilities whose input demands must be met) — **but without their logistics simulation** (no belts, trucks, or per-building routing). Depth of chains, not depth of transport.

- **Within a colony:** pooled stockpile, abstract logistics.
- **Between colonies/worlds:** explicit shipping over the strategic map.
- **Tiers:** survival basics (cheap, soloable) vs advanced/growth chains (compete for slots + labor).
- **Food:** several distinct foodstuffs rather than many crops. Resolved via harness experimentation (issue #211): the starter `hydroponic_bay` yields `food_ration` (the sole needs-tracked survival staple, weight unchanged) plus a `biomass` byproduct, which feeds two tech-gated (`hydroponics`) distinct foodstuffs — `protein_rations` and `produce_rations` — as separate tradeable end-items. A diet-variety stability bonus was considered and explicitly deferred pending playtesting signal; see `content/base/commodities.yaml`'s `biomass` entry.

**The commodity graph is discovered, not designed on paper.** The real early deliverable is not the final list of goods — it's the machinery to author and stress-test commodity graphs quickly (see §13).

---

## 7A. Technology & Research

**Role:** Tech-gating is a *modifier* driver (§5) — it shapes the menu of available options, not a primary per-colony decision.

**Effects — unlocks-first.** Most techs are **binary unlocks**: they make a new building, chain, good, station type, orbit type, megaproject, or expedition capability *available*. Numeric bonuses (efficiency, +build-slots, +capacity) are a deliberate **minority**, reserved for explicit "efficiency" techs. This keeps tech legible and sidesteps multiplier-stacking for most of the tree.

**Difficulty-stacking discipline** (resolves the risk flagged in the original code review):

```
effective = base × (1 + Σ tech_bonuses_in_category) × difficulty_scalar
```

- Tech numeric bonuses are **additive within a category** (three +20% = +60%, not ×1.728) — predictable, reason-able.
- Difficulty is a **single outermost multiplicative scalar per quantity, applied last** (from the grade tables) — so a difficulty setting means the same thing regardless of tech level.
- Because tech is unlocks-first, most techs never enter this formula at all.

**Research economy — research as a commodity.** Lab buildings produce *research* as an output (consuming labor + inputs), pooled **system-wide**. Science is therefore a **colony specialization**, part of the specialize/connect/build loop rather than a bolted-on subsystem. Tech pace scales with the economy (bigger industrial base → faster research → more world-drift).

**Structure — a data-authored DAG.** A tech is `{ id, prerequisites[], cost, effects[] }`; the tree's shape emerges from authored data. **Web / eventually-everything:** order matters, but there are no permanent mutually-exclusive branches. Tech choice is therefore **prioritization under scarcity** — teching toward the unlock your current situation needs — not permanent commitment. Fits management-by-exception (a bottleneck surfaces → tech toward its solution).

**Scope:** system-wide empire tech, fed by pooled research (not per-colony).

**Content — a solid start, not the final scale (issue #236).** `content/base/tech.yaml` now has ~48 entries across six categories (`engineering`, `physical_sciences`, `life_sciences`, `computing`, and two added by #236: `materials_science` — unifying the refining/fabrication chains that used to be bundled two-deep under `advanced_materials`/`automation` — and `astronautics`, giving the #235 survey-expedition system and the #234 orbital system real tech hooks), spanning tiers 1–6 with genuine DAG convergence/fan-out points (not a single-file chain). This is sized against SMAC (~90 techs) and Stellaris (~400 techs) as reference points but is explicitly a first wave — #249 tracks the next wave(s) toward that scale.

**#235 integration, delivered.** Two new `TechEffect` variants close the loop #236 was written to close: `SurveyModifierBonus { full_reveal_bonus, partial_reveal_bonus }` (accumulated into `GameState.tech_survey_modifiers`, combined with a mission's own mid-mission-choice modifiers when `resolve_survey` runs) and `ReduceTransitTime { fraction }` (accumulated multiplicatively into `GameState.propulsion_transit_scalar`, applied to a new survey expedition's transit leg at launch). Both are purely additive/default-neutral (`1.0` scalar, `0.0` bonuses pre-research) — no existing behavior changes until a player researches the relevant `astronautics` tech, so this required no retroactive gating of anything already available.

**Two gaps found and explicitly flagged, not silently left (both filed as follow-ups):**
- **#247 — tech-gate enforcement.** `TechEffect::UnlockBuilding`/`UnlockCommodity`/`UnlockCapability` populate tracking state, but `Command::QueueConstruction` (and the orbital/outpost/expedition commands) don't actually check `BuildingDef.tech_prerequisite` against `TechState.researched` before accepting a command — this predates #236 (the original 12-entry tree had the same gap) and #236 didn't newly introduce or worsen it, but a much larger tree makes the gap more consequential. Not fixed in #236 itself: real enforcement has meaningful blast radius (existing tests/harness bundles/frontend flows may rely on the current unenforced behavior) and deserves its own review.
- **#248 — `Bonus` effects are tracked but numerically inert.** `TechEffect::Bonus` accumulates into `GameState.modifier_accumulator`, but nothing in `colony/production.rs` reads it back to scale actual output. #236 kept `Bonus`-type techs to "a deliberate minority" per the unlocks-first discipline below specifically because of this — they're honest placeholders for a real numeric effect, not silently broken.

**Pacing — verified, not just claimed.** `outpost_core::tech::tests::real_tech_yaml_is_not_trivially_beelinable_at_baseline_research_rate` computes the real tree's total research cost against a baseline single-`research_lab` output rate (5/sol × 30 sols/month = 150/month) and asserts clearing the whole tree takes well over 300 strategic months — tech-ordering remains a real prioritization decision, not a beeline. A second test (`real_tech_yaml_parses_and_forms_a_wide_multi_tier_dag`) asserts the DAG shape itself (≥40 techs, all 6 categories present, ≥5 tiers, at least one convergence point and one fan-out point) so the shape claims above are enforced by CI, not just prose.

---

## 8. The Strategic Layer (multi-zoom)

### 8.1 Planet zoom
A **hex map** of the planet showing terrain, biome, and resource deposits. Colonies are nodes on hexes. Infrastructure (roads, pipelines, power lines) are connections between nodes, with cost and throughput based on distance and terrain crossed. Trade flows **automatically once a route exists, with manual priority overrides**. Roughly equal weight on *where to expand* (site selection, prospecting) and *how to optimize what exists* (infrastructure, balancing).

### 8.2 Orbital zoom
Orbital infrastructure is represented as a **schematic coverage map** (altitude/footprint indicated, not physically accurate), not a spatial placement puzzle.

- **Station types (specialization):** Habitat (population capacity, no planetary surface needed), Industrial (vacuum/zero-g production strictly better for certain recipes), Logistics (dock/hub connecting surface shipments to system traffic).
- **Orbit types (discrete tradeoffs):** Low Orbit (cheap surface access, more exposure), Geostationary (fixed over one colony, dedicated link), Lagrange (system-wide vantage, not tied to one body).
- **Coverage layers (per satellite type, toggleable):** Comms, Sensor, Defense — each its own constellation and footprint. Layered map overlays, not a single blended blob.

### 8.2A Satellite/station split + body scoping (issue #234)

Two design decisions settle open questions from the original §8.2 model:

**Station scaling — variable slot count per type, not a module-slot rework.** `StationType`'s three roles (Habitat/Industrial/Logistics) stay as fixed enum variants — the incremental path recommended by the issue itself — but each now scales across a type-specific `slot_range()` (Habitat 1-4, Industrial 2-6, Logistics 1-3, centred on the pre-#234 fixed costs of 2/3/2 so existing content stays valid) instead of every station of a type costing exactly one fixed value. `OrbitalStation::new` is now fallible, validating the caller's chosen `slot_cost` against the type's range (`OrbitalError::InvalidSlotCost`). `StationType::construction_turns_for` computes a size-scaled build-time (implemented and unit-tested) but is **not yet wired into any command**: `Command::BuildOrbitalStation` builds immediately regardless of chosen size (pre-#234 behavior, unchanged), and blueprint-driven `Command::BeginOrbitalConstruction` always builds at the type's default size using the blueprint's fixed `build_months`. Wiring size-scaled build time into a live command is left for a future pass once there's a concrete need for slower large-station construction. A full generic module-slot subsystem (N independently-typed slots per station) was explicitly not built — revisit only if the fixed-type-plus-range model turns out too coarse in play.

**Body scoping — `Option<BodyId>` on both `OrbitalStation` and `SatelliteConstellation`.** `None` is reserved for `OrbitType::Lagrange` (a genuinely system-wide asset, matching the pre-#234 behavior for that band and Lagrange's real-world "vantage point between bodies" framing); `Low`/`Geostationary` entities carry `Some(body_id)`. `OrbitalRegistry::slots_used`/`slots_available` are now scoped by `(orbit_type, body_id)` — two bodies' Low orbits are independent 12-slot pools, not one shared system-wide pool. Coverage gained the body-aware `OrbitalRegistry::combined_coverage_for_body` alongside the pre-existing (now explicitly system-wide-only) `combined_coverage`.

**Probes are not a new entity.** A probe is simply a `SatelliteConstellation` (typically `count: 1`) with `body_id` set to a body other than the founding colony's home body — it reuses the exact same coverage math, `DeployConstellation` command, and `OrbitalRegistry` storage as a standing coverage constellation. This was chosen over a distinct lightweight probe type since it reuses more code and probes don't (yet) need mechanics a constellation can't already express (one-way deployment, lifespan limits, and discovery events are all deferred until a concrete need for them shows up in play).

**Deliberately out of scope for #234**: whether orbital stations and #233's surface `Outpost` should share a common "facility" abstraction — kept separate. They already run on genuinely different mechanics (slot-cost/registry accounting vs. body-anchored resource pool + buildings), and #233's `Outpost` had just landed with no second consumer to justify a shared trait yet; revisit if a real shared need emerges (e.g. during #242's promotion-to-colony work).

### 8.3 System zoom
The distinct jobs the system scope owns that no lower scope does:

- **World-scale specialization** — which body plays which role (e.g., inner planet = industry, belt = raw extraction, gas giant = volatiles/fuel).
- **Inter-body logistics** — shipping goods between bodies, where **shipping/hauler capacity itself becomes a managed resource**.
- **Megaprojects** — pooled from the entire system. This is where victory lives (the interstellar expedition; plus wormhole gate, terraforming engine, system-scale power, etc.).

### 8.3A Founding-site resource guarantee (issue #232)

Procedural system generation (§8.3, issue #199) guarantees a habitable founding body but says nothing about whether that body — or the system around it — actually has the raw materials to survive and progress. Issue #232 closes that gap with two guarantees, both **coverage guarantees, not depletion/economy guarantees** (deposits carry no finite budget; extraction doesn't currently drain them):

- **Founding site**: `PlanetMap` generation (`map.rs`) force-places at least one real hex `Deposit` of every commodity in the curated `VEIN_COMMODITIES` list if normal vein placement didn't produce one, mirroring Factorio's "starting-area" override pattern (a guarantee pass layered on top of, not replacing, ordinary probabilistic placement).
- **System-wide**: every generated `Body` gets a lightweight `BodyDeposit` tag (`system_gen.rs::distribute_system_resources`) — DSP-style archetype-biased placement (asteroid belts favor ores, gas giants favor hydrocarbons, moons skew fissile) with the same verify-and-patch guarantee on top, so every curated commodity exists *somewhere* in the system even before a colony can reach it.

**`VEIN_COMMODITIES`** (`map.rs`) is the single curated raw-material list both guarantees share — deliberately small (9 entries: `structural_ore`, `conductive_ore`, `precious_ore`, `refractory_ore`, `semiconductor_ore`, `fissile_ore`, `silicates`, `hydrocarbons`, `biomass`), mirroring `content/checks/bootstrap_colony`'s curated-starter-loadout philosophy rather than requiring every raw material in the content pack on one world. New raw-material commodity families should be evaluated against this list — added if a colony's tech-progression path genuinely needs them guaranteed-reachable, left out if they're meant to be a genuine system-wide scarcity/exploration incentive instead.

Deposit *generosity* (not coverage) scales with difficulty via `ModifiableQuantity::DepositAbundance`.

**Explicitly deferred** (tracked as a follow-up issue): deposits don't yet gate mining/production at all — every extraction recipe currently runs unconditionally regardless of local deposits. The guarantees above are forward-looking world-state, not yet a live gameplay constraint. Real sizing of "how much abundance is actually enough" needs that gating to exist first, then playtesting data (a proposed workflow: a tester uses `Command::DebugGrantColonyResources` to simulate having enough of a resource, and the resulting playthrough is analyzed to back out real requirements) — not a one-shot calculation.

### 8.3B Outposts (issue #233)

An **outpost** is a lightweight, single-purpose off-world presence that extends a colony's reach — the mechanism by which a colony reaches resources or performs work "located elsewhere" in the system (§232's motivating need), rather than relocating or founding a second full colony everywhere a needed material happens to be. Always owned by a parent colony (`parent_colony_id`) and anchored to a specific system body (`body_id`); it does not exist independently.

**Architecture decision** (resolved after auditing `orbital::OrbitalStation` and the megaproject mechanic, per this issue's own "avoid a fourth parallel thing that produces stuff on a body" caution):

- `orbital::OrbitalStation` was ruled out as a base to generalize — it's purely structural bookkeeping (slot cost, a turn-countdown construction project) with no production/upkeep loop of its own, and is scoped system-wide rather than to a body.
- A bare `PlacedBuilding` wrapper was ruled out — `PlacedBuilding` has no resource pool or construction-queue context of its own; it's meaningless detached from a container that owns a `ColonyPool`.
- The chosen shape: a genuinely new `outpost::Outpost` struct, stored in its own `GameState.outposts: Vec<Outpost>` — **not** a reuse of `Colony`/`state.colonies`, since `Colony` and `PopulationPool` are 1:1-indexed everywhere in the codebase (`FoundColony` always creates both together) and an entity with no population would break that invariant.
- Despite being a new struct, `Outpost` shares the *same internals* a `Colony` already uses rather than duplicating the production pipeline: `ColonyPool`, `PlacedBuilding`, `ConstructionQueue`/`ConstructionProject`, and `colony::process_production_scaled` are all reused unchanged, because none of them are actually coupled to `Colony`/population — `process_production_scaled` takes a raw `labor: f32` and a pool, not a `Colony` reference. An outpost supplies a fixed skeleton-crew `labor` constant (`outpost::OUTPOST_BASE_LABOR`) in place of a `PopulationPool`-derived value, and a neutral `1.0` habitability modifier (outposts aren't founded on habitability grounds).

**Upkeep shortfalls reduce output, they don't destroy the outpost** — this falls out for free from reusing `process_production_scaled` unchanged: a power/maintenance shortfall already scales a building's output down via the same mechanism a colony building uses, with no new failure-state machinery needed.

**Establishment is never gated** (tech, range, or otherwise) — any colony can `EstablishOutpost` on any known body at any time. What can actually be *built* at an established outpost is gated by tech/bonuses/max-range-from-parent-colony — deliberately split into a follow-up (#241) since it's a distinct, content-authoring-heavy concern that doesn't need to block the base mechanism.

**Megaproject support** reuses `system::SystemCommand::ContributeToMegaproject` as-is — that command already accepts raw `resources`/`research` with no colony/source reference, so `Command::ContributeOutpostToMegaproject` is a thin withdraw-from-outpost-pool-then-forward wrapper, not a new contribution pathway.

**Deliberately out of scope for #233** (tracked as sub-issues of #233): tech/bonus-gated building availability and the max-range constraint (#241); promotion of an established outpost into a full colony/city (#242); frontend UI (#243).

---

## 9. Expeditions & Exploration

A **schematic system node map** (planets, moons, asteroid belt as nodes; travel time by distance and propulsion tech). Exploration is **textured, not abstracted**: journeys and surveys trigger events, encounters, and mid-mission decisions that affect the outcome. Surveys reveal candidate colony sites and resource locations (full / partial / failed reveals).

This layer repurposes the existing event/decision infrastructure from colony-disasters to expedition-encounters.

### 9A. Body-scouting survey expeditions (issue #235)

Implements the schematic-survey model above: `Command::LaunchSurveyExpedition` targets any `system::BodyId` (not a planet-map hex) with one of `expedition::ExpeditionType`'s four tiers (`FastFlybyProbe` → `OrbitalSurvey` → `Lander` → `MannedExpedition`, unmanned to crewed, cheap-and-thin to expensive-and-thorough). This is distinct from the older `Command::LaunchFieldExpedition` (planet-hex only, deterministic-arithmetic discovery rolls, M8/#103) — both remain live; `LaunchSurveyExpedition` is the system-scale counterpart.

**Probe/satellite naming overlap with #234 — resolved.** #234 separately introduced body-scoped `orbital::SatelliteConstellation` and floated "probes" as a satellite concept. Settled as: **#235 owns "probe" as body-scouting** — `ExpeditionType::FastFlybyProbe`/`OrbitalSurvey` are unmanned survey missions that resolve to a `SurveyOutcome` via `resolve_survey`, model risk/cost/data-quality tradeoffs, and (per #234's own design doc) a probe is "simply a `SatelliteConstellation` ... with `body_id` set to a non-home body" only in the sense of *standing coverage* (comms/sensor/defense layers), never a one-shot scouting mission. **#234 keeps "satellite" strictly to standing orbital coverage.** The two systems don't overlap in practice: a `SatelliteConstellation` is a persistent asset that contributes `CoverageFootprint` every turn; an `ExpeditionType::FastFlybyProbe` is a one-shot mission with a lifecycle (`InTransit` → `Surveying` → `Completed`) that resolves once and is done. No shared entity was built, matching #235's own "no distinct probe type; reuse existing coverage math and command" framing for the *satellite* half specifically.

**Transit → survey → resolution.** Each `ExpeditionState` in the new `GameState.expedition_registry` counts down a fixed `SURVEY_TRANSIT_TURNS` transit leg, then an `ExpeditionType`-specific survey leg (`base_duration_turns`), advancing one sol at a time in `Command::AdvanceColonySol`'s Step 4f — deterministically, via `expedition::deterministic_roll(expedition_id, sol ^ salt)` rather than an external RNG stream (keeps replays and saved-state resumption reproducible, matching the older field-expedition system's precedent of deterministic-arithmetic rolls over injected randomness).

**Anomalies fire through the interrupt system, as designed.** Each `Surveying`-phase expedition is checked once per sol against every loaded `expedition::AnomalyDef` (content-pack data, `content/base/anomalies.yaml`); a trigger halts the survey countdown, builds a `MidMissionEvent` (Investigate / Ignore), and moves the expedition to `ExpeditionPhase::AwaitingDecision`. The event is surfaced via the *existing* interrupt collection (`InterruptSource::EventFired`, tier taken from the event itself) rather than a new interrupt variant — the module doc's original intent ("mid-mission interrupts that reuse the interrupt + predicate system") is honored literally, not just in spirit. `Command::ResolveMissionDecision` resolves the pending choice; investigating rolls a weighted `AnomalyOutcome` (`expedition::resolve_anomaly_outcome`) and applies its `research_bonus` into `SystemResearchPool`, `resource_reward` into the origin colony's pool, and `unlocks_tech` (if any) via the same `TurnProcessor::apply_tech_effects` path normal research completion uses — not a shortcut that skips tech-effect application.

**Survey completion reveals state, it doesn't gate anything.** `resolve_survey`'s outcome (`FullReveal`/`PartialReveal`/`Failed`) sets `Body.surveyed = true` (and `Body.candidate_site_name` on a full reveal) — new, UI-facing world state, not a new mechanical gate. Deposits themselves (`Body.deposits`, #232) were already always-true world state with no fog-of-war; #235 deliberately does **not** retrofit deposit visibility gating onto that model — `surveyed` only marks that an expedition *looked*, matching #232's explicit "coverage guarantee, not a gate" precedent rather than inventing a new one here.

---

## 10. Threat & Difficulty Model

A clean core with optional layers, spanning the "sandbox → brutal" spectrum.

- **Environmental hazards — primary, always on.** Dust storms, quakes, meteor strikes, equipment failure, disease, radiation. Fits the Outpost DNA, feeds the event system, and supplies the loss-events that make population a genuine responsibility.
- **Existential clock — optional (campaign on / sandbox off).** A system-wide menace (destabilizing star, approaching cataclysm, failing Earth support) that gives the interstellar expedition urgency and purpose.
- **Adversary (raiders / rival factions) — optional difficulty toggle.** Present for players who want competition; **never the core**, and explicitly not a 4X-military focus (consistent with the "economy over military/diplomacy" intent).

**Defense satellites** defend primarily against environmental hazards (meteors, debris, storms), and against raiders only if the adversary toggle is enabled.

---

## 10A. Existential Clock (detail)

An **optional** pressure layer (campaign on / sandbox off). **Discipline: the base game must be complete and compelling without it** — the clock is a modifier; nothing in the core loop may depend on it.

**Menace as authored data — no single canon.** One clock mechanism reads a **menace definition**; multiple menace types are authored equally (e.g., *failing Earth support* = immigration/relief rates decline across phases; *destabilizing star / cataclysm* = escalating hazards + environmental shifts). Replayability via content, consistent with content-as-data.

**Schema requirement:** menaces are heterogeneous (economic squeeze vs physical hazard), so the phase schema needs a **flexible effect vocabulary** — rate changes, hazard injections, environmental-parameter shifts, cost multipliers, etc. A menace ≈ `{ id, phases: [{ trigger_time, telegraph, effects[] }], final_semantics }`.
- *Architectural opportunity:* this effect vocabulary overlaps with difficulty grade-tables (§7A) and tech effects — a shared **effect/modifier descriptor** could serve tech bonuses, difficulty scalars, and menace-phase effects (applied at their respective layers). Consider a common descriptor shape; don't force identical application.

**Escalating + phased (telegraphed).** The clock advances through *known* stages, each removing a crutch or adding a hazard (immigration slows → relief stops → Earth goes dark). Phases map onto the mid→late arc. **Phase warnings reuse the interrupt system (§12A)** — same machinery, bigger scale — giving the player agency to prepare for a signposted escalation, not gamble against a hidden number.

**Fixed clock; prepare, don't stop it.** The clock is external and unstoppable; the player mitigates *effects* (builds resilience, races to self-sufficiency and the expedition), not the clock. (Optional late-game "buy time" valve — deferred.)

**Degrading failure, not scripted game-over.** At the end, the menace makes survival *untenable*; collapse is emergent (withdrawn support / escalating hazards overwhelm the network), not an instant loss screen. Difficulty tunes final-phase lethality.

**Victory relationship:** launch the expedition before the clock overwhelms you = win; clock off (sandbox) = the expedition is a pressure-free achievement.

---

## 11. Victory & Endgame

- **Capstone victory:** launch the **interstellar expedition** (a system-scale megaproject).
- **Optional alternate conditions:** economic / population / scientific goals for players who want a different objective.
- **Sandbox continue:** play on after victory (Factorio-style "you won but keep going").

---

## 12. Automation & Directives  **[Approach TBD]**

Deferred deliberately — the automation approach (game "AI", scripts, or a DSL) should be chosen after the mechanics exist and the pain is felt, not designed up front. What is **locked** now:

- **Hard architectural rule:** the simulation core must be **fully drivable programmatically**, with no assumption that a human issues directives each turn. This keeps AI/script/DSL as *later layers on the same interface*, not rewrites.
- The directive system must be **expressive enough to run a colony between interventions**, must **surface exceptions** (what needs attention), and must **always allow manual override**.

**Direction now indicated (not fully committed):** the interrupt system's design (§12A) — deep player-authored condition rules + convert-interrupt-to-directive — points the *player-facing* automation toward a **rules/conditions model** (scripts/DSL family) rather than opaque game-AI. Implementation still deferred; the default (out-of-box) automation could still be simple heuristics under a rules-based customization layer.

---

## 12A. Exception & Interrupt System

The **primary player-facing system** for a 40h+ interrupt-driven game (§4A), and the *visible half of automation* (§12): automation runs colonies per directives; interrupts fire when a colony can't cope or when the player asked to be told. Design the **contract** now; the automation *implementation* stays deferred.

**Tiers (tier governs fast-forward halting):**
- **Blocking** — halts time; requires a decision to continue (branching/critical events). Rare.
- **Urgent** — stops the fast-forward and hands back control; act or dismiss-and-continue (a crisis beginning).
- **Notable** — logged and shown in the digest; does *not* stop fast-forward (completions, minor events).
- **Ambient** — background; visible only in the event log.

**"Wait N turns unless interrupted" (the core loop):** advance up to N turns; the sim checks each turn for interrupts ≥ the player's threshold tier; the first one halts the advance at that turn and returns control + a digest of accumulated Notable items; a clean run returns the digest at N. The player sets the threshold.

**Predictive, not reactive.** Crisis interrupts fire on **trajectory** ("stability declining — crash in ~5 turns"), not on the event ("colony lost"). Without this, fast-forward is unsafe and players stop trusting it.
- **Cost ceiling:** predictive warnings default to cheap **trend-extrapolation** (track rates of change, linearly project to threshold-crossing), *not* full forward-simulation, so fast-forward stays fast across 10+ colonies. Reserve any full lookahead for rare high-stakes checks. "Accept the sim cost" means a *bounded* cost.

**Return-from-fast-forward digest.** A single triage summary ("what happened while you waited / what needs you"), not a popup storm. This *is* the session event log with filtering/search from the UI backlog. Alerts are **actionable from the notification** (jump-to-colony, decide-in-place).

**Deep configurability, defaulted.** Per-colony / per-source / per-condition rules for what interrupts the player, plus **convert-an-interrupt-into-a-directive** on the spot ("always auto-handle this"). Must ship with **strong defaults + progressive disclosure** — a single global threshold works out of the box; depth is opt-in — or the depth becomes a barrier.

**Architectural note — shared condition vocabulary.** Deep interrupt rules and directives share one substrate: a **condition/predicate language** (e.g., `colony.stability < 20`, `stockpile.food declining AND eta < 5 turns`). "Stop me when X" and "auto-handle when X" are the same predicate pointed at different actions. Build the predicate system **once**; both systems consume it. This is the technical heart of both features.

---

## 13. Tooling: Commodity-Balance Harness (build early)

A **headless tool** (runs the pure core, no UI) that takes a proposed colony/network configuration and answers: does the chain *close* (sustainable)? where's the *bottleneck*? is it *trivial* (boring) or *impossible* (broken)?

- **Start static** (steady-state rate math) — cheap, high value. Grow toward dynamic later; avoid overlapping prematurely with the full AutoSim survival test.
- This *is* the commodity-graph experimentation tool — without it, "experiment with commodities" means slow hand-play.
- It **doubles as the runner** for the planned GitHub-issue prototyping loop: "generate a prototype and test a playability idea" mostly means "author content, run the sim headless, check the numbers."

---

## 14. Architecture

**Stack:** Rust pure-library core + Vue 3 / TypeScript frontend. Chosen because this is no longer a game-engine game (C#'s only real advantage was Godot/Unity), and a mature, aligned Rust+Vue architecture already exists to borrow from.

**State model:** **in-memory turn model that snapshots to SQLite between turns.** The turn processor computes a whole turn in memory, then persists a snapshot. SQLite is save/checkpoint, **not** the live per-mutation store — a deliberate divergence from the reference architecture's SQLite-as-truth, because a recompute-heavy turn sim has very different write patterns than an RPG.

**Hard rules:**
- Core is a **pure library**: no I/O/framework deps, runs headless (tests, bots, CLI, harness).
- Core is **fully programmatically drivable** (see §12).
- **Content as data** (authored packs → loaded), so chains are balanced as content, not code.
- **Two-cadence turn model** (colony-sol, strategic-month).

**Two shared substrates (build once, reused widely) — identified during design:**
- **Condition/predicate language** — powers both interrupts ("stop me when X", §12A) and directives ("auto-handle when X", §12). One predicate evaluator; two consumers. This is also what indicates the automation direction is rules-based.
- **Effect/modifier descriptor** — a common data shape for tech effects (§7A), difficulty grade-tables (§7A), and menace-phase effects (§10A). All "apply a modifier to a sim quantity." Share the descriptor shape; apply at each system's own layer (e.g., difficulty stays the outermost scalar). Do not over-unify their *application* — only their *representation*.

**Patterns borrowed from the reference architecture (Harsh Realm), selectively:**

| Port cleanly (structural) | Do **not** port |
|---|---|
| Pure headless core | SQLite-as-live-state for the hot turn loop |
| Typed server→client event contract (kills client/server drift) | Per-mutation write-through repositories in the turn loop |
| Renderer-agnostic world model + reducer/projection (Vue now, WebGL/native later) | RPG-specific domain subsystems (combat resolver, dungeon gen, GM narration) |
| Content packs | — |
| Grade/difficulty tables (→ tunable difficulty) | — |
| Subsystem template (makes adding a system fast) | — |
| Event-sourcing **at the turn/decision boundary** (what the client must see) | Event-sourcing every internal pipeline step |

**Reference repo:** a copy of Harsh Realm is checked into the Outpost 3 repo for reference; Outpost 3 is a fresh, separate project that borrows patterns.

**Frontend:** Vue for deep, interactive menus/panels/maps (the game is primarily map-and-menu). The renderer-agnostic model keeps the door open for a WebGL/canvas or native client later without a rewrite.

---

## 15. What This Supersedes

- The prior **C# simulation core (163 passing tests)** becomes a **behavioral spec** for the Rust rebuild — the systems (resource store, turn processing, power, needs, construction) are understood and tested; they are re-expressed in Rust, not ported. The tests are reference, not waste.
- The **isometric colony renderer and grid-placement machinery are dropped** (no spatial colony layer).
- The **non-Rust-collaborator constraint no longer applies**, which is part of why the C#→Rust move is now sound rather than reflexive.
- All prior stacks are archived.

---

## 16. Key Risks & Discipline

- **Boundary ambiguity between zooms** *(highest design risk).* With nested scopes, overlapping responsibilities will confuse the player. Every zoom needs a crisp, non-overlapping job. Guard this actively.
- **Content sprawl in commodities.** Every new good potentially touches every chain. Keep the graph as data and lean on the harness; do not hand-expand blindly.
- **Automation must be strong.** "Continuous re-tuning" + "10+ colonies" only works if directives + management-by-exception genuinely reduce load. If automation is weak, the game becomes a spreadsheet job.
- **The world must drift.** Depletion, tech, growth, and events must keep invalidating old setups, or re-tuning becomes optional fiddling.
- **Sustaining a 40h+ game** *(major, from the length target).* Throughput-optimization is **not** the engine (logistics is abstracted). The long-game engine is scope-escalation + tech/commodity breadth + emergent crises + the megaproject goal. Proven in the genre, but content-hungry: under-content it and the player runs out of meaningful decisions well before 40h. Makes the harness (content authoring/validation) mission-critical, not optional.
- **Interrupt cadence carries the long game.** "Wait until interrupted" as the primary 40h interaction makes the exception/notification system first-class. Tune interrupt rate and meaningfulness continuously; too few is boredom, too many defeats the automation.
- **Restart discipline.** This project has reset many times. This redesign is the first where scope (system-wide) and core loop (fractal specialize/connect/build) reinforce each other. Resist further resets; iterate on this spine.

---

## 17. Open Questions / TBD

1. **Automation approach** — AI vs scripts vs DSL (chosen after mechanics exist).
2. **Commodity graph specifics** — the actual goods, tiers, recipes, and the foodstuff set (discovered via the harness).
3. **Building/structure roster** — the concrete list per scope.
4. ~~**Tech tree system**~~ — **RESOLVED, see §7A** (unlocks-first, additive-within-category + difficulty-outermost, research-as-commodity, web/eventually-everything DAG). Tree *contents*: a ~48-entry, 6-category, 6-tier solid-start DAG authored in #236 (pacing-verified against the harness); further scale tracked in #249. Two related gaps found and filed, not silently left: #247 (tech-gate enforcement at the command layer) and #248 (`Bonus`-effect production impact).
5. ~~**Game length & pacing**~~ — **RESOLVED, see §4A** (strategic/interrupt-primary rhythm, three-phase unequal arc, ~40h+ default). Remaining TBD: the research-rate and interrupt-cadence *tuning* (harness work).
6. **Colony flavor-image approach** — static vs state-reflecting; art pipeline (deferred; placeholder-first).
7. **Balance numbers** — all scalars, to be tuned against the harness.
8. ~~**Migration mechanics detail**~~ — **RESOLVED, see §6A** (hybrid auto+directed, time/capacity/willingness friction, cascading evacuations tuned by difficulty). Remaining TBD: friction *numbers* (harness).
9. ~~**Existential-clock specifics**~~ — **RESOLVED, see §10A** (menace-as-data, escalating/phased/telegraphed, degrading failure, optional). Remaining TBD: authoring the actual menace definitions and phase timings (content + harness).

---

## 18. Suggested Build Sequence (for task breakdown)

Order reflects "core-loop-first" and "harness early":

1. **Pure Rust core skeleton** — turn model (in-memory), content-pack loading, programmatic drive interface, headless test setup.
2. **Colony economy** — pooled commodities, slots + labor constraints, basic production chains, population aggregate pool.
3. **Commodity-balance harness (static)** — before committing to a large commodity graph.
4. **Condition/predicate substrate + directive & interrupt layer (minimal)** — one predicate system (§12A) powering both "auto-handle when X" (directives) and "stop me when X" (interrupts); colonies run between interventions; basic interrupt tiers + a global threshold; "wait N turns unless interrupted"; manual override.
5. **Strategic layer — planet zoom** — hex map, colonies as nodes, infrastructure, auto-trade + override.
6. **Vue frontend spine** — renderer-agnostic model + projection; colony screen + planet map.
7. **Population dynamics + fluid migration**, then **events/threats (environmental)**, then **predictive early-warning + return-from-fast-forward digest (event log)** — the interrupt system's trajectory-warnings and triage screen, once there are crises to warn about.
8. **Orbital zoom**, then **system zoom (world-specialization, inter-body logistics, megaprojects)**.
9. **Expeditions** (textured events).
10. **Difficulty toggles** (adversary, existential clock), victory conditions, sandbox continue.
11. **CI** — do not defer to "before Phase 6"; add once the core exists to prevent regression accumulation.

*(This sequence is a starting point for GitHub-issue breakdown, not a locked roadmap.)*
