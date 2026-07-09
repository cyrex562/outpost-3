# Outpost 3 — Build Issues

Derived from **DESIGN.md §18**. Standalone-prompt style: each issue is written so an AI coding agent (or you) can pick it up with the repo's DESIGN.md available for reference.

**Conventions**
- **Labels:** `phase-N-<name>` · `type-<setup|feature|tooling|ui|ci>` · `fidelity-<high|medium|coarse>`
- **Fidelity** reflects how soon it'll be built. Phases 1–4b are `high` (detailed, small, ready to execute). Phases 5–11 are `medium`/`coarse` (epic-ish; decompose further when you reach them — decomposing them precisely now would be speculative work you'd rewrite).
- **Dependencies** are listed as *Depends on* / *Blocks*.
- Every issue cites the DESIGN.md sections that specify it. The agent should read those before implementing.

> **Gap flagged during decomposition:** §18 does not list the **tech/research system** as an explicit step, though it's specified in §7A. It is inserted below as **Phase 4b** (and research-as-commodity production appears in Phase 2). Update §18 to include it.

---

# Phase 1 — Pure Rust core skeleton  `fidelity-high`

### [P1] Scaffold Rust workspace with pure-library sim core
**Labels:** `phase-1-core` `type-setup` `fidelity-high`
**Depends on:** none · **Blocks:** all Phase 1
**Context:** The sim core is a *pure Rust library* — no I/O or framework deps — so it runs headless in tests, a CLI, the balance harness, and behind a Vue frontend. This creates the empty scaffold. No game logic yet.
**Task:**
- Cargo workspace; core lib crate (`outpost_core`) as pure `lib` (no bin, no async runtime, no I/O crates).
- Standard `cargo test` setup (no CI yet — later issue).
- Module stubs reflecting DESIGN.md systems (`turn/`, `colony/`, `content/`, `population/`).
- `rustfmt` + `clippy` config.
**Done when:** `cargo build` + `cargo test` pass on an empty test; core crate has zero I/O/framework deps (verify Cargo.toml); stubs compile.
**References:** §14, §18 step 1. **Constraints:** No game logic; no SQLite; no web/framework deps in core.

### [P1] In-memory turn model + two-cadence turn loop
**Labels:** `phase-1-core` `type-feature` `fidelity-high`
**Depends on:** scaffold · **Blocks:** colony economy
**Context:** The game runs an in-memory turn model computed wholly in memory each turn (SQLite is snapshot-only, separate issue). Two cadences: fast colony-sol, slow strategic-month (~30 sols, configurable).
**Task:**
- Define the top-level game state struct (in-memory, owns all live state).
- Implement a turn processor that advances one colony-sol; strategic-month as an aggregate of N sols.
- Deterministic ordering; separate any RNG stream from math (RNG injected, seedable).
**Done when:** advancing a turn mutates state deterministically for a fixed seed; a test advances M sols and 1 strategic month and asserts stable results; no I/O.
**References:** §4, §4A, §14.

### [P1] Content-pack loading pipeline (content-as-data)
**Labels:** `phase-1-core` `type-feature` `fidelity-high`
**Depends on:** scaffold · **Blocks:** colony economy, harness
**Context:** All authored content (commodities, recipes, buildings, techs, menaces) is data, not code. The kernel holds zero authored records.
**Task:**
- Define a pack format (e.g. YAML/JSON) and a loader that builds in-memory content tables.
- Support multiple packs merged deterministically; validation with clear errors.
- Start with a tiny sample pack (2–3 commodities, 1–2 recipes) to prove the path.
**Done when:** loader ingests the sample pack; malformed content yields a clear error; content is queryable in-memory; no authored data hardcoded in the kernel.
**References:** §7, §14.

### [P1] Programmatic drive interface (fully drivable core)
**Labels:** `phase-1-core` `type-feature` `fidelity-high`
**Depends on:** turn model · **Blocks:** directives, harness, frontend
**Context:** **Hard rule:** the core must be fully drivable programmatically, with no assumption a human issues actions each turn. This is what makes AI/scripts/DSL later *layers*, not rewrites, and enables the harness and bots.
**Task:**
- Define a command/action API that any driver (test, CLI, harness, bot, future UI) uses to inspect state and issue actions.
- No action requires human presence; all are callable in code.
**Done when:** a test drives a full turn purely through the API (no direct struct mutation); the API exposes read + act without UI assumptions.
**References:** §12 (hard rule), §14.

### [P1] SQLite snapshot / restore between turns
**Labels:** `phase-1-core` `type-feature` `fidelity-high`
**Depends on:** turn model · **Blocks:** frontend persistence
**Context:** SQLite is save/checkpoint, **not** the live per-mutation store. Compute the turn in memory, then persist a snapshot between turns. A 40h game is played across many sessions, so resume must be clean.
**Task:**
- Serialize/deserialize full game state to/from SQLite as a between-turns snapshot.
- Round-trip fidelity; versioned schema for forward migration.
**Done when:** save→load→continue reproduces identical subsequent turns for a fixed seed; snapshot happens between turns, not per-mutation.
**References:** §14. **Note:** address save-version compatibility now (a gap in the original review).

---

# Phase 2 — Colony economy  `fidelity-high`

### [P2] Commodity + recipe data model (colony-pooled)
**Labels:** `phase-2-colony` `type-feature` `fidelity-high`
**Depends on:** content loading · **Blocks:** production step, harness
**Context:** Commodities are colony-pooled: buildings draw from / add to a shared stockpile. No belts or intra-colony routing. Depth of chains (X4/CoI), not depth of transport.
**Task:** Model commodities, a colony stockpile (pool), and recipes (inputs→outputs per turn, incl. power). Load them from content.
**Done when:** a colony holds a pool; recipes are represented as data; a unit test applies a recipe to a pool and gets correct deltas.
**References:** §7.

### [P2] Building/structure model + finite tech-gated build slots
**Labels:** `phase-2-colony` `type-feature` `fidelity-high`
**Depends on:** commodity model · **Blocks:** production, specialization
**Context:** Two scarce resources force specialization: **build slots** (finite, tech-gated capacity — the "underground capacity" concept as a slot limit, not literal digging) and labor. A colony can't build every chain.
**Task:** Model buildings (a recipe + labor need + slot cost), a colony's slot capacity, and build/queue logic respecting the slot limit.
**Done when:** a colony rejects builds beyond slot capacity; buildings occupy slots; queue processes within limits.
**References:** §5.

### [P2] Production step in the turn pipeline
**Labels:** `phase-2-colony` `type-feature` `fidelity-high`
**Depends on:** buildings, recipes · **Blocks:** stability, harness
**Context:** Each turn, buildings consume inputs from and produce outputs to the colony pool, bounded by available inputs, power, and labor.
**Task:** Implement the per-turn production resolution (input-limited, power-limited, labor-limited); handle shortfalls gracefully (partial/paused production).
**Done when:** a colony with a small chain sustains output when fed and stalls predictably when starved; deterministic.
**References:** §5, §7.

### [P2] Population aggregate pool + labor
**Labels:** `phase-2-colony` `type-feature` `fidelity-high`
**Depends on:** turn model · **Blocks:** stability, migration
**Context:** Population per colony is an aggregate pool: count + a single stability scalar + bulk needs. **Labor = population × stability.** No per-colonist sim.
**Task:** Model the pool; derive available labor; wire labor as an input constraint to production.
**Done when:** labor availability scales with population×stability and limits production; unit-tested.
**References:** §6.

### [P2] Needs resolution + stability dynamics
**Labels:** `phase-2-colony` `type-feature` `fidelity-high`
**Depends on:** population, production · **Blocks:** events, interrupts
**Context:** Meet bulk needs (food/water/housing/…) → population grows, stability holds. Neglect them → stability falls, growth stalls/reverses. Survival basics are cheap/soloable; advanced chains compete for slots+labor.
**Task:** Implement per-turn needs check → stability delta → growth/decline; tag commodities as basic vs advanced tier.
**Done when:** a well-supplied colony grows; a starved one loses stability then population; tiers behave (basics cheap, advanced costly).
**References:** §5, §6.

### [P2] Research as a commodity (science specialization)
**Labels:** `phase-2-colony` `type-feature` `fidelity-high`
**Depends on:** commodity model, production · **Blocks:** tech system (4b)
**Context:** Research is produced by lab buildings (consuming labor + inputs) and pooled system-wide — making science a colony specialization rather than a separate subsystem.
**Task:** Add a `research` commodity + lab building recipe; accumulate research into a system-wide pool.
**Done when:** a science colony produces research each turn into a shared pool; tested.
**References:** §7A.

---

# Phase 3 — Commodity-balance harness (static)  `fidelity-high`

### [P3] Static flow-balance calculator
**Labels:** `phase-3-harness` `type-tooling` `fidelity-high`
**Depends on:** commodity model, production · **Blocks:** harness CLI
**Context:** A headless tool that takes a proposed colony/network config and answers: does the chain **close** (sustainable)? where's the **bottleneck**? is it **trivial** (boring) or **impossible** (broken)? Steady-state rate math (not full dynamic sim yet).
**Task:** Given buildings + imports, compute steady-state input/output rates; detect closure, bottleneck input, trivial/impossible configs.
**Done when:** for sample chains it correctly reports closed/bottleneck/trivial/impossible; pure function over content + config.
**References:** §13.

### [P3] Harness CLI + report output
**Labels:** `phase-3-harness` `type-tooling` `fidelity-high`
**Depends on:** flow-balance calculator
**Context:** The harness is how commodity graphs are experimented on (they're discovered, not designed on paper).
**Task:** A CLI that loads a content pack + a config, runs the calculator, prints a readable report (and machine-readable output for automation).
**Done when:** `harness run <pack> <config>` prints a balance report; exit codes reflect closed/broken.
**References:** §13.

### [P3] Prototyping-loop runner hook
**Labels:** `phase-3-harness` `type-tooling` `fidelity-high`
**Depends on:** harness CLI
**Context:** The harness doubles as the runner for the GitHub-issue prototyping loop: "test a playability idea" ≈ "author content, run headless, check numbers."
**Task:** A thin entrypoint that takes a content/config bundle, runs the harness (and later, sim scenarios), and emits pass/fail + metrics suitable for CI or an agent loop.
**Done when:** a bundle can be run non-interactively and yields structured results; documented for the loop.
**References:** §13.

---

# Phase 4 — Condition/predicate substrate + directives & interrupts (minimal)  `fidelity-high`

### [P4] Condition/predicate language + evaluator (shared substrate)
**Labels:** `phase-4-control` `type-feature` `fidelity-high`
**Depends on:** colony economy · **Blocks:** directives, interrupts
**Context:** **One shared substrate** powers both interrupts ("stop me when X") and directives ("auto-handle when X"). Build once; two consumers. Predicates over sim quantities, e.g. `colony.stability < 20`, `stockpile.food declining AND eta < 5 turns`.
**Task:** Define a predicate representation (data + evaluator) over readable sim quantities; support comparisons, trends (rate/eta), and boolean composition.
**Done when:** predicates evaluate against live state; unit-tested incl. a trend/eta predicate.
**References:** §12A, §14 (shared substrates).

### [P4] Directive system (auto-handle) + manual override
**Labels:** `phase-4-control` `type-feature` `fidelity-high`
**Depends on:** predicate substrate · **Blocks:** management-by-exception
**Context:** Colonies run between interventions on directives (predicate → action). Manual override always available. Automation is encouraged; the player intervenes by exception.
**Task:** Directive = predicate + action; evaluate directives each turn; apply actions via the drive API; allow manual override of any colony.
**Done when:** a colony runs unattended per a directive across many turns; manual override supersedes; tested.
**References:** §5, §12.

### [P4] Interrupt tiers + threshold + "wait N turns unless interrupted"
**Labels:** `phase-4-control` `type-feature` `fidelity-high`
**Depends on:** predicate substrate · **Blocks:** frontend digest, events
**Context:** Four tiers — Blocking / Urgent / Notable / Ambient — where tier governs fast-forward halting. Core loop: advance up to N turns, stop at the first interrupt ≥ the player's threshold, return control + a digest of accumulated Notable items.
**Task:** Model interrupts with tiers; implement the advance-until-interrupted loop with a configurable threshold; accumulate a digest.
**Done when:** advancing halts on the first ≥-threshold interrupt and returns the digest; a clean run returns the digest at N; tested.
**References:** §4A, §12A.

---

# Phase 4b — Tech & Research system  `fidelity-high`  *(fills §18 gap)*

### [P4b] Tech DAG + unlock application
**Labels:** `phase-4b-tech` `type-feature` `fidelity-high`
**Depends on:** research-as-commodity, content loading · **Blocks:** strategic content gating
**Context:** Tech is **unlocks-first**: most techs are binary gates opening new buildings/chains/capabilities. Web/eventually-everything DAG (order matters, no permanent exclusions). System-wide, fed by pooled research.
**Task:** Model techs as `{id, prerequisites[], cost, effects[]}`; spend pooled research to complete techs; apply unlock effects (make content available).
**Done when:** research completes a tech; its unlocks become buildable; prerequisites enforced; DAG loaded from content.
**References:** §7A.

### [P4b] Effect/modifier descriptor + additive-within-category / difficulty-outermost
**Labels:** `phase-4b-tech` `type-feature` `fidelity-high`
**Depends on:** tech DAG · **Blocks:** difficulty (Phase 10), menace (Phase 10)
**Context:** Numeric bonuses are a **minority**. Stacking discipline: `effective = base × (1 + Σ tech_bonuses_in_category) × difficulty_scalar`. The **effect/modifier descriptor** is a shared shape reused by tech, difficulty, and menace effects (applied at each layer; difficulty stays outermost).
**Task:** Define the effect descriptor; implement additive-within-category accumulation; apply a single outermost difficulty scalar per quantity.
**Done when:** stacked bonuses sum within category; difficulty applies last; a test verifies the formula and ordering.
**References:** §7A, §14 (shared substrates).

---

# Phase 5 — Strategic layer: planet zoom  `fidelity-medium`

### [P5] Hex map + colonies-as-nodes + infrastructure
**Labels:** `phase-5-planet` `type-feature` `fidelity-medium`
**Depends on:** colony economy
**Context:** A planet hex map (terrain/biome/deposits); colonies are nodes; infrastructure connects them with cost/throughput by distance + terrain.
**Task:** Hex map data model; colony placement on hexes; infrastructure edges with cost/throughput. *(Decompose further when reached.)*
**Done when:** colonies exist on a hex map connected by infrastructure with distance/terrain-based cost.
**References:** §8.1.

### [P5] Inter-colony trade (auto base flow + manual override) + expansion
**Labels:** `phase-5-planet` `type-feature` `fidelity-medium`
**Depends on:** infrastructure
**Context:** Goods flow automatically once a route exists, with manual priority overrides. Equal weight on expansion (site selection) and optimization.
**Task:** Auto trade flow over routes; manual priority override; site-selection/founding flow. *(Decompose when reached.)*
**Done when:** surplus/deficit balances across connected colonies automatically; overrides work; new colonies can be founded on surveyed sites.
**References:** §8.1.

---

# Phase 6 — Vue frontend spine  `fidelity-medium`

### [P6] Renderer-agnostic world model + typed event contract + projection
**Labels:** `phase-6-ui` `type-ui` `fidelity-medium`
**Depends on:** drive interface
**Context:** A renderer-agnostic model with a typed server→client event contract and a projection/reducer, so Vue is one renderer and WebGL/native can come later without a rewrite.
**Task:** Define the typed event contract (codegen if feasible); world model + projection; Vue app shell consuming it. *(Decompose when reached.)*
**Done when:** the Vue shell reflects core state via the projection; contract is typed end-to-end.
**References:** §14.

### [P6] Colony screen + planet map + interrupt digest UI
**Labels:** `phase-6-ui` `type-ui` `fidelity-medium`
**Depends on:** world model; Phase 5; Phase 4
**Context:** Colony = management screen (list/panels + non-interactive flavor image, no grid). Planet = hex map. Interrupt digest = the return-from-fast-forward triage screen (the session event log w/ filtering).
**Task:** Colony management screen; planet hex map view; interrupt digest/event-log UI with filtering; actionable-from-alert (jump-to-colony). *(Decompose when reached.)*
**Done when:** player can run a colony, view the planet, advance-until-interrupted, and triage from the digest.
**References:** §5, §8.1, §12A.

---

# Phase 7 — Population dynamics, migration, events/threats, predictive warnings  `fidelity-medium`

### [P7] Population growth + immigration waves
**Labels:** `phase-7-pop-events` `type-feature` `fidelity-medium`
**Depends on:** population pool
**Context:** Sources: immigration waves (dominant early, land at port/gateway colonies) + local growth (dominant later).
**Task:** Growth model; immigration waves as events landing at gateway colonies. *(Decompose when reached.)*
**Done when:** population grows locally and via immigration arriving at gateways.
**References:** §6, §6A.

### [P7] Migration (hybrid) with time/capacity/willingness friction
**Labels:** `phase-7-pop-events` `type-feature` `fidelity-medium`
**Depends on:** population, infrastructure
**Context:** Hybrid: auto pull-flow toward opportunity + directed override; player gates/incentives. Frictions: time/distance, transport capacity, and stability cost on forced moves. Evacuation displaces the problem (refugees strain receivers).
**Task:** Auto pull-flow; directed transfers; frictions; open/close gates; receiver-strain on evacuation. *(Decompose when reached.)*
**Done when:** people flow toward opportunity; forced moves cost stability; evacuations take time and can strain receivers; difficulty tunes severity.
**References:** §6A.

### [P7] Environmental hazard/event system
**Labels:** `phase-7-pop-events` `type-feature` `fidelity-medium`
**Depends on:** turn model, content loading
**Context:** Environmental hazards are the always-on primary tension (storms, quakes, meteors, failure, disease, radiation). Data-driven events feeding interrupts.
**Task:** Event/hazard definitions as content; scheduler; effects; hooks to the interrupt system. *(Decompose when reached.)*
**Done when:** hazards fire as authored, affect colonies, and raise interrupts.
**References:** §10.

### [P7] Predictive early-warning + return-from-fast-forward digest
**Labels:** `phase-7-pop-events` `type-feature` `fidelity-medium`
**Depends on:** interrupts, needs/stability
**Context:** Crisis interrupts fire on **trajectory** ("crash in ~5 turns"), not on the event. **Cost ceiling:** default to cheap trend-extrapolation, not full lookahead, so fast-forward stays fast across 10+ colonies.
**Task:** Trend/eta extrapolation on key metrics → predictive Urgent interrupts; polished return digest. *(Decompose when reached.)*
**Done when:** declining metrics raise early warnings with an eta before the failure; fast-forward remains performant.
**References:** §12A, §6A.

---

# Phase 8 — Orbital + system zoom  `fidelity-coarse`

### [P8] Orbital layer (epic)
**Labels:** `phase-8-orbital-system` `type-feature` `fidelity-coarse`
**Context:** Station types (Habitat/Industrial/Logistics) in discrete orbit types (Low/Geo/Lagrange); per-satellite-type coverage layers (comms/sensor/defense) as toggleable map overlays. List/panel + schematic coverage map, not a placement puzzle.
**Scope (decompose later):** station model, orbit-type tradeoffs, coverage computation, coverage-layer UI.
**References:** §8.2.

### [P8] System zoom (epic)
**Labels:** `phase-8-orbital-system` `type-feature` `fidelity-coarse`
**Context:** World-scale specialization; inter-body logistics with hauler/shipping capacity as a managed resource; megaprojects pooled from the whole system (victory lives here).
**Scope (decompose later):** system node model, inter-body shipping + capacity, megaproject framework.
**References:** §8.3.

---

# Phase 9 — Expeditions  `fidelity-coarse`

### [P9] Expeditions (epic)
**Labels:** `phase-9-expeditions` `type-feature` `fidelity-coarse`
**Context:** Schematic system node map; textured exploration with events/encounters/mid-mission decisions; surveys reveal sites (full/partial/failed). Reuses the event/decision infrastructure.
**Scope (decompose later):** node map + travel time, survey/reveal, expedition event/decision content.
**References:** §9.

---

# Phase 10 — Difficulty toggles, victory, sandbox  `fidelity-coarse`

### [P10] Difficulty grade-tables (epic)
**Labels:** `phase-10-endgame` `type-feature` `fidelity-coarse`
**Depends on:** effect/modifier descriptor (4b)
**Context:** Tunable difficulty via grade-tables; difficulty is the outermost scalar per quantity. Reuses the shared effect descriptor.
**Scope (decompose later):** grade-table content + application points.
**References:** §7A, §10, §14.

### [P10] Existential clock (menace-as-data) (epic)
**Labels:** `phase-10-endgame` `type-feature` `fidelity-coarse`
**Context:** Optional, campaign-on/sandbox-off. Menace = `{id, phases:[{trigger, telegraph, effects[]}], final_semantics}`; escalating/phased/telegraphed (reuses interrupts); degrading (emergent collapse, not scripted game-over); multiple menaces as content. **Base game must stand alone without it.**
**Scope (decompose later):** menace schema, phase engine, telegraph→interrupt wiring, ≥1 authored menace.
**References:** §10A.

### [P10] Adversary toggle + victory + sandbox continue (epic)
**Labels:** `phase-10-endgame` `type-feature` `fidelity-coarse`
**Context:** Optional raider adversary (defense sats gain a job); victory = interstellar-expedition capstone + optional alt conditions (economic/pop/scientific); sandbox continue after victory.
**Scope (decompose later):** adversary events + defense, victory checks, post-win sandbox.
**References:** §10, §11.

---

# Phase 11 — CI  `fidelity-coarse`

### [P11] CI pipeline (build + test + lint)
**Labels:** `phase-11-ci` `type-ci` `fidelity-coarse`
**Depends on:** scaffold (add as soon as the core exists)
**Context:** Do **not** defer CI to "before Phase 6" — add it once the core exists to prevent regression accumulation during the rebuild (a gap flagged in the original review). Can be pulled earlier than its number suggests.
**Task:** CI on push/PR: `cargo build`, `cargo test`, `clippy`, `fmt --check`; later, run the harness prototyping bundle.
**Done when:** green pipeline on push; failing tests/lints block; harness hook stubbed for later.
**References:** §16, §18 step 11.

---

## Suggested first milestone
Issues through **Phase 3** get you to *a headless colony economy + a harness that can run and validate a prototype* — the point where your GitHub-issue prototyping loop becomes real. Prioritize reaching that before Phase 4+.
