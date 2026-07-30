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

**Commodities vs colony resources (issue #304).** A **commodity** is cargo: it has weight and a trade value, and haulers can move it between colonies. A **colony resource** — `power`, `housing`, `research` — is produced and consumed in place and can never be shipped. These are now separate concepts in the kernel rather than one table with a flag: `ResourceDef`/`ResourceKind` in `content/base/resources.yaml`, held in `Colony.resources` (`ColonyResourcePool`) rather than `Colony.pool` (`ColonyPool`). Recipes and needs name either kind freely; `ColonyStores` is the single dispatch point that routes an id to the store that owns it, keyed off `ContentRegistry::is_resource`, and the loader rejects an id declared as both.

The separation is *structural*, and that's the point. The trade pipeline is only ever handed the commodity pool, so a resource is not reachable from the code that would ship it — there is no flag for a future caller to forget to check. Before this, `CommodityDef::tradeable` was authored on all 45 commodities and **read by nothing**: the auto-trade pass built its list from every id present in any colony pool, so `power`, `housing`, and `research` all flowed over trade routes despite being authored `tradeable: false`. That flag is now enforced too, as a second mechanism for commodities that are genuine cargo but authored unshippable (`oxygen` today).

**Colony resources do not persist across sols.** `ColonyResourcePool::clear()` runs between the needs step and the production step of each colony sol, which is load-bearing in both directions: needs draw on what production banked *last* sol (so clearing earlier would starve the colony instantly), and production runs *after* the clear (so once the sol finishes the pool still holds this sol's figures, which is what the colony screen reports). Power not drawn this sol is lost; buffering is a later feature to build on the existing `power_cell` commodity rather than on the pool. This also fixed two unbounded-accumulation bugs that had been invisible: as ordinary commodities, `power` netted a positive surplus every sol and banked it forever, and `housing` — a capacity check that consumes nothing — gained a whole habitat's worth every sol, so the housing need became trivially satisfied after a handful of turns.

**Power is colony-local for now.** It does not cross colony boundaries. Transmission over the existing road/rail/pipeline infrastructure edges is plausible mid-game work and is filed as a follow-up; authoring power as content data rather than hard-coding it keeps that option open. **Water stays a tradeable commodity** — it has weight, it's extracted from specific bodies, and shipping it to a dry colony is exactly the logistics problem the trade network exists for.

**The sol is the only cadence (issue #332).** There is no strategic-month pipeline any more. `TurnProcessor::advance` advances one sol and runs every system; `state.month` is a calendar label derived from `sol / sols_per_month` that nothing branches on. Four things used to be gated behind a 30-sol month and now run every sol:

- **Research progress.** Balance-neutral: `tech::apply_research_turn_scaled` drains the *whole* pool toward the current project whenever it runs, and the pool is fed per sol, so the same total RP is applied — just in finer increments instead of sitting idle for up to 29 sols.
- **Shipment delivery.** `CargoShipment.turns_remaining` is denominated in sols, converted from the route's distance-derived `travel_time_months` at dispatch (`system::SOLS_PER_MONTH`), so a voyage takes the same elapsed time and reports progress every sol rather than jumping in 30-sol steps.
- **Orbital construction.** Same treatment: `OrbitalConstructionProject` counts down in sols, converted from the blueprint's authored build months at creation. `tick()` also gained a guard so an already-complete project can't complete twice.
- **Inter-colony trade.** The one deliberate behaviour change: rebalancing every sol instead of every 30. The 30-sol gap was the sole reason a need reserve sized in *sols of consumption* had to exist at all (see above), so this is the cadence fix the reserve was standing in for.

`Command::AdvanceStrategicMonth` is removed. It only ever incremented the month counter without running the pipeline, so it would have skipped trade, research, and shipment delivery while desyncing the `sol % sols_per_month` alignment — dead surface (no host called it) that would have become a live footgun. `Event::StrategicMonthAdvanced` is kept and now fires on the sol that crosses a month boundary, since a calendar rollover is still worth announcing.

**Trade takes time — surpluses travel as convoys (issue #332).** Auto-trade is no longer a teleport. `run_trade_flow` decides what to send exactly as before (surplus comparison, route `throughput_cap`, manual overrides), but it **withdraws** the cargo from the sender's pool at dispatch and hands back a `TradeConvoy`; the receiver is credited only when the convoy's `sols_remaining` reaches zero. While travelling, the goods live in `TradeNetwork.convoys` and in neither pool — which is why the turn processor must store what the pass returns, and why a snapshot has to round-trip the manifest.

Two things fall out of that and are load-bearing:

- **The dispatch pass must see cargo already in flight** (`TradeNetwork::inbound_in_flight`). A receiver's stock cannot move until the first convoy lands, so a pass blind to the pipeline re-ships a full share every sol and overshoots by roughly `transit_sols`×. In-pass dispatches count too, or two routes feeding one colony each ship a full share against the same stale reading.
- **Arrivals are processed before dispatch, within the sol.** Otherwise a one-sol route is decremented to zero and delivered in the sol it was sent — the teleport this replaced. Landing first also lets a colony forward on what it just received.

**Convoys are deliberately not `CargoShipment`.** That type is interplanetary bulk freight: it consumes a `Hauler` from a finite fleet, is keyed by body rather than colony, and is dispatched by hand. A standing trade arrangement has to keep working with no free hauler, and has to work between two colonies on the *same* body, where a body-to-body travel time is undefined. Route transit is derived from body separation but **read as sols, not months** — a trade convoy is a fast automatic short-haul transfer; month-scale freight is what `CargoShipment` is for. Reading it as months would put every inter-body route on a 30-sol pipeline and reintroduce the long rebalance gap #332 set out to remove.

**`TRADE_RESERVE_SOLS` came down from 10 sols to 3.** Its original job — bridge the 30-sol gap to the next rebalance — is gone. All it still does is cover the round trip if a colony exports and then needs the stock back, which is a few sols of consumption, comfortably above the default one-sol convoy transit. Still a balance dial; expect the harness to retune it.

A long route slows imports but does not break them: cargo simply spends more sols in flight, and a regression test pins that nothing goes negative or is created across an 80-sol run on a 10-sol route. `max_sols` on a fast-forward is clamped (`MAX_FAST_FORWARD_SOLS`) because it arrives as untrusted host input and an unbounded request would run the turn pipeline that many times inside one uninterruptible command.

**Fast-forward is reachable (issue #332).** `GameEngine::advance_until_interrupted` and `collect_turn_interrupts` were implemented and well tested long before anything could call them; `Command::FastForward { max_sols, threshold }` is the drive-interface wrapper that finally exposes them. It emits the per-sol events for every sol it ran, then one `Event::FastForwardEnded` reporting how far it got and why it stopped; the accumulated below-threshold interrupts are read separately from `Query::InterruptDigest`. Both hosts carry the command (tier as a slug, unknown values rejected rather than defaulted) and a digest read path.

**Pause/play/speed is a UI timer, not kernel change.** The kernel keeps discrete sols — `apply(Command)` and snapshot-per-turn are load-bearing (rules 3 and 4) — so "continuous time" is the footer issuing one `fast_forward` per timer tick. Speed only changes how often the timer fires, which is what keeps a fast-forwarded game byte-identical to a stepped one. A halt stops the timer and opens the digest panel: `Urgent` is documented as "halts fast-forward and hands control back", and the clock rolling on past a crisis would make that a lie.

**Only surpluses of survival commodities are tradeable.** Several commodities are cargo *and* consumed by colony needs — `water`, `oxygen`, `food_ration`. They stay fully tradeable, but the auto-trade pass now offers only the stock a colony holds **above** its own need reserve (`TRADE_RESERVE_SOLS` sols of per-capita demand, computed per colony from `NeedsConfig` and population). Everything at or below the reserve is invisible to trade, so a colony can be a net water exporter and still never ship away what its colonists are about to drink.

This closed a real self-starvation bug rather than adding a nicety: `trade.rs`'s module doc had always *described* shipping "surplus (pool amount above the configured target)", but no target existed anywhere in the code — the flow pass simply equalised raw stock between route endpoints, transferring half the difference. A colony holding exactly enough water for its population would send half of it to a neighbour with none. Direction and transfer volume are now both computed from surpluses, so a colony *below* its reserve reads as maximally needy and pulls imports, which is the behaviour the doc had promised all along.

**Two scarce resources force specialization: build slots and labor.** A colony cannot build every chain — finite, tech-gated build capacity (the "underground capacity" concept, expressed as a slot limit rather than literal digging) plus finite labor means the player must choose a focus. Specialization is therefore a *consequence* of constraints, never a rule imposed on the player.

**You specialize to grow, not to survive.** Survival basics (food, water, oxygen, power, housing) are cheap and few-slot, so a colony can always be self-sufficient in essentials. The advanced/growth chains are what compete for slots and labor. Interdependence is **encouraged, not enforced** — a colony can limp along solo on basics, but specializing and trading is how you grow and get rich.

**Decision drivers (primary vs modifier):**
- *Primary (what the player actively decides):* **environmental fit** (a site's resources/hazards give it a starting hand) and **specialization** (its role in the network).
- *Modifier (shapes the menu, not a separate decision):* **risk vs throughput** and **tech-gating**.

**The decision is continuous, but at the directive level.** The player re-tunes colonies as conditions drift — but by adjusting a colony's role/priorities, not micromanaging construction queues. Manual per-colony control is always available; strong automation is the encouraged default; the game surfaces colonies needing attention (management-by-exception).

**Dependency:** Continuous re-tuning is only engaging if the world keeps drifting. The simulation must generate reasons to re-tune — resource depletion, tech unlocks shifting the optimal build, population growth changing labor, trade shifts, and events.

**Starter buildings deploy instantly at founding, "lander"-style (playtest feedback round 2).** Buildings picked in the Found Colony wizard's loadout step are no longer queued through the normal multi-turn `build_queue` — they land operational the moment the colony is founded, via a dedicated one-shot `DeployStarterKit` command that places the whole batch directly into `colony.buildings`. This fixes two playtest pain points: (1) selecting a full starter roster could exceed the 5-slot budget and surface a confusing "build slot queue" error with no clear explanation, and (2) waiting multiple sols for a colony's very first buildings to complete felt wrong for what should be an immediate landing event. The batch is validated atomically (tech gates, total slot cost) before anything is placed, so a rejected request never partially deploys, and the one-shot guard (`Colony.starter_kit_deployed`) prevents it from becoming a standing free/instant alternative to `QueueConstruction` later in the game. Regular construction after founding is unaffected — it still goes through `build_queue`/`construction_turns` as before.

*Deferred follow-up:* consolidating the starter roster into fewer, more capable multi-function buildings (e.g. a combined colony HQ, or a power/atmosphere/water utility building) was raised in the same playtest round but is explicitly out of scope here — it requires reworking the single-recipe-per-building-type architecture (`Colony.active_recipes` is keyed by `building_type`, colony-wide) to support true simultaneous multi-output buildings, and is tracked as a separate, larger future effort.

**Multi-function buildings: engine mechanism landed (`RecipeDef.concurrent`).** The architecture rework the paragraph above deferred is now in place: a `RecipeDef` can be authored with `concurrent: true`, meaning it always runs every turn alongside every other `concurrent` recipe for the same `building`, regardless of `Colony.active_recipes`'s pick-one selection — a building type can mix at most one pick-one recipe set (the ordinary, player-switchable kind) with any number of always-on `concurrent` recipes. All of a building instance's simultaneously-running recipes (pick-one + every concurrent one) share **one** combined scale factor each turn — their inputs, maintenance draws, and deposit-gated outputs pool into a single demand computation (`colony/production.rs::compute_effective_input_ratio`/`compute_deposit_ratio`, extended to accept the concurrent set alongside the existing pick-one recipe), and their power draws sum into the same power-grid demand entry (`compute_power_grid_scaled`). This keeps the "one building = one operational state this turn" model intact — a multi-function building throttles all its outputs together under a shared power/labor/input constraint, rather than one function silently continuing at full output while another starves. `BuildingProductionResult` gained an additive `concurrent_recipe_ids: Vec<String>` field (`recipe_id` — the pick-one field — is untouched, so existing single-recipe-building UI/consumers are unaffected). See `colony/production.rs`'s module doc comment for the full mechanism and its test suite (`concurrent_recipes_all_run_simultaneously_with_no_active_recipe_selection` and neighbors) for worked examples, including a building with *only* concurrent recipes (no pick-one alternatives at all — the "colony HQ" shape) and one mixing both kinds. **Consolidated starter building landed: `colony_hq`.** `content/base/buildings.yaml`/`recipes.yaml` now author a real `colony_hq` starter building (`category: services`, no `tech_prerequisite`, so it appears in the Found Colony wizard's starter roster automatically — no frontend changes needed, the wizard already lists every non-tech-gated building generically) with four `concurrent: true` recipes (`hq_generate_power`, `hq_pump_water`, `hq_scrub_oxygen`, `hq_conduct_research`) whose power/water/oxygen output rates exactly match `solar_array_mk1` + `water_well` + `life_support_module` combined — the same power/water/oxygen coverage in **1 build slot instead of 3**. The three standalone buildings remain available (this is an *additional* option, not a replacement) for players who'd rather spread hazard risk across separate structures than have one hazard event potentially cost all three functions at once. Proven two ways: `outpost_web::ws::tests::colony_hq_runs_all_concurrent_recipes_from_real_pack` exercises the real `production.rs` code path (all four recipes actually run, sharing one scale) against the loaded base content pack, and `content/checks/colony_hq_efficiency/` is a new balance-harness bundle confirming the same water/oxygen/power net-rate margins as `content/checks/bootstrap_colony/`'s standalone trio (the harness represents this faithfully since #272 — see the authoring guide below). `hydroponic_bay` (food) and `basic_habitat` (housing) are intentionally left out of `colony_hq` — folding those in too is possible future work, not done here.

**Colony HQ seeds the research economy (issue #310).** `colony_hq`'s fourth concurrent recipe, `hq_conduct_research`, outputs **1 `research`/sol** with no inputs — administrative/survey overhead paid for by the HQ's existing power draw and worker slots, and still throttled by the building's shared scale like its other three outputs. This closes a genuine bootstrap deadlock rather than adding flavor: the only two buildings that output `research` are `research_lab` (gated behind `basic_construction`, itself 100 RP) and `physics_lab` (deeper still), so before this a fresh colony had **no route to its first tech through ordinary production at all** — the sole alternative source was a lucky survey-expedition anomaly (`anomalies.yaml`'s `research_bonus` outcomes). The rate is deliberately one fifth of `research_lab`'s `conduct_research` (5 RP/sol): `basic_construction` lands in ~100 sols on HQ alone versus ~17 with one lab, so labs stay clearly worth building. It takes no `water`, unlike `conduct_research`, so it can't compete with `hq_pump_water` for the HQ's own output. Research is **per-HQ and therefore per-colony** — a second colony doubles the trickle, which is intended (founding a colony is expensive) and remains a single content scalar to retune if it proves too generous. Unemployment-style gameplay consequences aren't attached: the trickle is flat, not scaled by population or stability. Both balance bundles (`content/checks/bootstrap_colony/`, `content/checks/colony_hq_efficiency/`) now assert the +1/sol net so a future content edit can't silently zero the only pre-lab research source.

### Authoring a multi-function building (issue #272)

The mechanism above is general — `RecipeDef.inputs`/`outputs` have always been `Vec`, so any recipe already supports 0-N inputs and 0-N outputs, and `concurrent: true` is what lets several run at once. This is the recipe for using it without tripping over the sharp edges.

**Pick a shape.** A building type may have *at most one* pick-one recipe set (the ordinary, player-switchable kind selected via `Colony.active_recipes`) plus *any number* of always-on `concurrent` recipes. Three shapes are supported and each reads differently in the UI:

| Shape | Authoring | Player sees |
|---|---|---|
| Single-function | one recipe, `concurrent` absent | its inputs and outputs |
| Switchable | 2+ recipes, none `concurrent` | a recipe picker |
| Multi-function | 1+ recipes with `concurrent: true` | an "always-on" badge, an "N recipes" badge, and the merged I/O line |
| Both | pick-one alternatives **plus** concurrent ones | a picker *and* the always-on flows |

A building whose every recipe is `concurrent` correctly has **no** pick-one recipe — `colony_hq` is that shape, and `BuildingDetailData.recipe` is `None` for it while `concurrent_recipes` is populated.

**Production lines: several chains in one building (issue #272).** `RecipeDef.line` partitions a building's recipes. Recipes sharing a line are **alternatives** — one runs. Different lines run **simultaneously and throttle independently**, each scaling from its own inputs, so a starved smelting line doesn't stop the machine shop beside it.

This was not possible before, for a structural reason worth recording: `Colony.active_recipes` is keyed by building type, so a building had room for exactly *one* selection — a second choice silently overwrote the first. The only way to get several chains was to mark all but one `concurrent: true`, i.e. always-on with no player choice. `colony_hq`'s shape was dictated by that limit, not chosen.

Three properties make the change cheap and safe:

- **No save migration.** The default line keys its selection on the bare `building_type` — exactly how selections were keyed before lines existed — so pre-#272 saves resolve unchanged. Named lines use a composite key separated by ASCII unit separator, which cannot occur in a content id.
- **No new command field.** A recipe knows its own line, so `Command::SetActiveRecipe` derives it: selecting a smelting recipe sets the smelting line and leaves machining alone. No host or frontend change was needed. Selecting an always-on recipe is rejected — a line of one has nothing to choose.
- **`concurrent: true` is now a special case of a line**, not a parallel mechanism: it makes a line of one, which always runs. The old rule, restated in the new vocabulary.

`fabrication_complex` is the first content to use it. It was always described as a combination foundry *and* machine shop but could only run one at a time; it now runs both, on independent lines. That is a deliberate throughput increase — one instance nets +2.5 `structural_metal`/sol **and** +1 `components`/sol, where before a player picked either +4 metal or +1 components — pinned by `content/checks/fabrication_lines/`.

**Power, labour and upkeep are still shared.** Lines throttle independently on their *inputs*, but the constraints that are genuinely building- or colony-wide still apply across all of them: power and labour ratios are computed colony-wide, and maintenance is pooled into every line's demand while being *charged* only once, at the busiest line's scale. Charging upkeep per line would silently multiply it by the line count.

**Don't produce the same commodity from two always-on recipes** unless you mean to. Their outputs are *summed*, so two `concurrent` recipes each yielding 5 power give 10, silently. That is occasionally what you want (two distinct generation processes), so it's a warning rather than an error: `ContentRegistry::lint()` reports it, and the balance harness prints it before the numbers on every `check` run. The same applies when an always-on recipe overlaps a *selectable* one — that doubles conditionally, depending on which recipe the player picked, which is harder to spot.

**Read the total footprint rather than summing recipes by hand.** `colony::building_io_summary(building_type, active_recipes, registry)` returns the merged per-cycle inputs and outputs of everything the building actually runs, following the player's recipe selection. This is what the buildings list displays, and what a content author should check against their intent. A commodity that is both consumed and produced stays in both lists rather than being netted — throughput and net change are different questions.

**Balance-harness bundles express this directly.** A check bundle lists the building once; its `concurrent` recipes are picked up from the pack automatically (listing them by hand would only be a way to get them wrong). What the harness cannot express, by construction, is *shared-scale throttling* — `BalanceCalculator` is a steady-state net-rate accumulator with no scale factor for any building, concurrent or not. This is also why the old "N co-located instances" workaround gave correct numbers: the calculator reads recipe flows and nothing else, so `power_delta`, `worker_slots`, `slot_cost`, and `maintenance` were never double-counted. Shortfall behaviour belongs in engine tests instead.

### The consolidation trade-off, measured (issue #272 gap 5)

Gap 5 asks whether one slot for three functions is *well-balanced* against keeping the standalone buildings separate. That was previously unanswerable with the tools to hand: `BalanceCalculator` reads only recipe flows, so it could confirm a chain closes but said nothing about what the configuration *costs* — and cost is the entire question. `BalanceReport` now carries a `ColonyFootprint` (build slots, worker slots, summed `power_delta`, longest build, total construction cost), printed by `harness check` as a `COLONY FOOTPRINT` block. Two bundles measure the two sides: `content/checks/colony_hq_efficiency/` (one `colony_hq`) and `content/checks/standalone_trio/` (the four standalone buildings covering the same output set).

**Against the trio it claims to replace, `colony_hq` is strictly dominant.** `solar_array_mk1` + `water_well` + `life_support_module` cost 3 slots, 37 `structural_metal`, 2 worker slots, and net 13 power. `colony_hq` costs **1 slot** and *exactly* the same 37 metal, 2 workers, and 13 power, for the same power/water/oxygen output — then adds a research trickle the trio has no equivalent for, and needs no tech while all three standalones are gated (`basic_power`, `resource_extraction`, `basic_power`). DESIGN.md's own narrative described the rates "exactly matching the sum" as a neutral starting point; the footprint shows that matching the sum *while saving two slots and skipping the tech gates* is not neutral at all.

**At four buildings it becomes a real trade, not dominance.** Matching `colony_hq`'s full output set needs `research_lab` as well — 4 slots, 67 metal, 5 workers, net 5 power. In exchange the standalones deliver **5 RP/sol against the HQ's 1** (the trickle is deliberately one fifth of a real lab, issue #310) at the cost of 1/sol less water, since `conduct_research` drinks water and `hq_conduct_research` pointedly does not.

So the honest summary: the standalones' only advantages are research throughput and hazard-risk isolation (one event can't cost all functions at once), and the second of those is not modelled anywhere. **What still needs playtest data** is whether slot pressure and hazard frequency are severe enough for those two advantages to matter. That is a much sharper question than "is it balanced", and it is the one to take into a play session — but the numbers above say the current tuning favours consolidation more heavily than the design notes implied, so *if* play confirms slots are cheap and hazards rare, `colony_hq` wants a cost increase rather than the standalones wanting a buff. No scalars have been changed here: this is measurement, and the retune is a balance decision for a human with play experience.

**Landing-kit redesign: full starter roster replaced (playtest feedback round 2).** The old 8-building starter set (colony_hq + 7 standalone singles) is retired from the wizard's default loadout in favor of 8 new/kept buildings, every one self-powered (`power_delta <= 0` — no starter building depends on a shared grid, so losing one to a hazard doesn't brown out the rest): `colony_hq` (power/water/oxygen physical plant + a research trickle, see below), `ice_miner` (bulk water), `excavation_rig` (deposit-gated `structural_ore`, `VEIN_COMMODITIES`-scaled per #232), `fabrication_complex` (foundry+machine-shop, pick-one between smelting ore→metal and machining metal→components), `air_miner` (oxygen + a `carbon` byproduct), `chem_plant` (synthesizes `chemicals` from `air_miner`'s carbon + water — deliberately not on the tech-gated hydrocarbons/plastics chain, to keep the landing kit self-sufficient without a 7th dependency), `greenhouse_dome` (food, self-powered — mirrors `hydroponic_bay`'s function, kept separate from `colony_hq` per the existing "intentionally left out" decision above), and `habitat_pod` (housing, self-powered — mirrors `basic_habitat`'s function, same reasoning). The old 7 standalone buildings (`water_well`, `hydroponic_bay`, `smelter`, `research_lab`, `basic_habitat`, `solar_array_mk1`, `life_support_module`) aren't removed — they're moved behind Tier 1 tech (`basic_construction`/`basic_power`/`resource_extraction`, all zero-prerequisite) as cheap early-game second sources, per `content/base/tech.yaml`'s `unlocks.buildings` lists. `content/checks/bootstrap_colony/` is rewritten to prove the new minimal viable landing kit (`colony_hq` + `greenhouse_dome` — just 2 of the 8 available starter slots) still closes food/water/oxygen/power at population 100, using just 2 of the 8 available starter buildings (well within the colony's 5 default build slots).

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

**Content — a solid start, not the final scale (issue #236).** `content/base/tech.yaml` originally had ~48 entries across six categories (`engineering`, `physical_sciences`, `life_sciences`, `computing`, and two added by #236: `materials_science` — unifying the refining/fabrication chains that used to be bundled two-deep under `advanced_materials`/`automation` — and `astronautics`, giving the #235 survey-expedition system and the #234 orbital system real tech hooks), spanning tiers 1–6 with genuine DAG convergence/fan-out points (not a single-file chain). This is sized against SMAC (~90 techs) and Stellaris (~400 techs) as reference points but was explicitly a first wave.

**Wave 2, delivered (issue #249).** The tree now has **81 entries spanning tiers 1–7**, matching SMAC's ~90-tech scale — not an arbitrary round number, but the reference point #236 itself set. Every existing tier-3/4/5 category gained real branch points (not just capstone padding): 5 new tier-3 techs, 7 tier-4, 7 tier-5, 8 tier-6, plus a genuinely new **tier 7 of ascension/secret-project-style capstones** (`zero_point_energy`, `dyson_swarm_engineering`, `warp_theory`, `post_scarcity_economics`, `genesis_engineering`, `transcendent_computing`) — each converging two prerequisites, several across two different categories, into forward-looking `unlock_capability`/`bonus` gates that mirror the `megastructure_engineering`/`terraforming_theory` precedent from #236 (a real unlock for future mechanics, not a dead-end flavor node). `zero_point_energy` (the tree's sole single-prerequisite tier-7 tech) is itself a prerequisite for three of the other five — same-tier convergence, an established pattern already present in the pre-#249 tree (e.g. tier-2 `urban_habitation`/`advanced_materials`), not strictly tier-6-only. All new content uses `effects:` entries the engine already resolves for real (`Bonus` — feeding #248's production wiring; `UnlockCapability`; `SurveyModifierBonus`/`ReduceTransitTime`/`ExtendOutpostRange` for the astronautics branch) rather than `unlocks.buildings`/`resources`, which would have required also authoring new buildings/commodities/`tech_prerequisite` entries across other content files — a separate, larger scope this issue deliberately didn't take on (see `TechUnlocks.bonuses`/`events`' recorded decision above: new content uses real `effects:`, not decorative sugar, throughout).
  - **Two design considerations from the issue explicitly left unresolved, not silently dropped.** A Stellaris-style *tier-gate* mechanic (N techs researched in a category before the next tier unlocks) and a Stellaris-style *repeatable/infinite* late-game tech were both raised as options in #249's own "proposal, not settled" framing. Neither was implemented: both are real engine-mechanic additions (a new gating rule in `tech::apply_research_turn_scaled`, and a new `TechDef` repeatability flag respectively), not content-authoring — genuinely different scope from "author more tech.yaml entries," and the pacing test below shows the DAG-only approach is still not trivially beelinable at this scale, so neither is yet a demonstrated *need*. Revisit if real playtesting shows the finite-tree-plus-scarce-research-rate model (`docs/DESIGN.md`'s stated design) stops being sufficient on its own.
  - **Pacing re-verified against the harness, not just claimed.** `real_tech_yaml_parses_and_forms_a_wide_multi_tier_dag`'s thresholds were raised alongside the content (≥75 techs, max tier ≥7, ≥10 convergence points, ≥5 fan-out points — all real counts from the actual authored tree, not adjusted to just barely pass) and `real_tech_yaml_is_not_trivially_beelinable_at_baseline_research_rate` continues to pass (a larger tree can only raise total research cost, never lower it, so this bound strengthens automatically). All 11 `content/checks/*` balance-harness bundles re-run and pass unchanged — expected, since none of their short check sequences research a tech mid-run, so wave 2's new `Bonus` capstones have no bundle to interact with either way.

**#235 integration, delivered.** Two new `TechEffect` variants close the loop #236 was written to close: `SurveyModifierBonus { full_reveal_bonus, partial_reveal_bonus }` (accumulated into `GameState.tech_survey_modifiers`, combined with a mission's own mid-mission-choice modifiers when `resolve_survey` runs) and `ReduceTransitTime { fraction }` (accumulated multiplicatively into `GameState.propulsion_transit_scalar`, applied to a new survey expedition's transit leg at launch). Both are purely additive/default-neutral (`1.0` scalar, `0.0` bonuses pre-research) — no existing behavior changes until a player researches the relevant `astronautics` tech, so this required no retroactive gating of anything already available.

**Two gaps found and explicitly flagged, not silently left (both filed as follow-ups):**
- **#247 — tech-gate enforcement, delivered for `QueueConstruction`.** `TechEffect::UnlockBuilding`/`UnlockCommodity`/`UnlockCapability` populate tracking state; `Command::QueueConstruction` now checks the requested `building_type`'s `BuildingDef.tech_prerequisite` (when the registry defines it) against `TechState.researched`, rejecting with `EngineError::TechLocked` — the exact same check #241 already added for `Command::QueueOutpostConstruction`, now shared by both (the error variant's doc comment was broadened accordingly rather than adding a near-duplicate variant). Audited the blast radius before writing the check, not after: every existing `lib.rs` test that calls `QueueConstruction` runs with `registry: None` (the gate is inert without a loaded registry, mirroring `tech::unlocked_buildings`'s own None-prerequisite-is-open convention), and `outpost_harness`'s `simulate.rs` never sets a registry either — so this landed with **zero** existing test/harness breakage, confirmed by a full `cargo test --workspace` + all 11 `content/checks/*` bundles passing unchanged.
  - **Orbital/outpost/expedition commands — decision recorded, not deferred by omission.** Outpost construction (`QueueOutpostConstruction`) already got this exact treatment in #241. Orbital blueprints (`OrbitalStationBlueprint`, `content/types.rs`) and expedition content types (`Expedition`, `AnomalyDef`, `expedition.rs`) have **no `tech_prerequisite`-equivalent field today** — extending tech-gating to `BeginOrbitalConstruction`/`BuildOrbitalStation`/`DeployConstellation`/`LaunchSurveyExpedition` would require a new content-schema field on each (not a drop-in copy of the `BuildingDef` pattern this issue used), which is a real, separate scope of work rather than a same-shape extension. Left for a dedicated follow-up rather than bundled here to keep this PR's blast radius matched to its actual mechanism (a straight port of an already-shipped pattern), not silently expanded to cover unrelated content types with no existing gating hook.
- **#248 — `Bonus` effects wired into real production output, delivered.** `colony::process_production_scaled` now takes `modifier_accumulator`/`difficulty_scalar` parameters and resolves a third, independent per-output multiplicative factor via `modifier::resolve` (the module's own single-authoritative-formula function, previously called nowhere outside `modifier.rs`'s own tests), applied at the exact same Pass-B deposit call site `category_modifiers` (#184) already uses. A new `tech_bonus_category_key(YieldCategory) -> &str` function is the sole reconciliation point between the tech tree's free-form bonus-category vocabulary (`power_generation`, `research_output`, `food_production`, `production_efficiency` — the four categories that map onto a real production-output concept) and the structural `YieldCategory` enum `production.rs` already classifies every output by; the remaining tech.yaml bonus categories (`construction_speed`, `labor_efficiency`, `colonist_health`, `trade_throughput`, `orbital_construction_speed`) belong to other subsystems entirely (construction queue, labor pipeline, population/needs, trade routes, orbital construction) and are out of scope for *this* file — wiring them into their own subsystems is separate follow-up work, not silently promised here. Proven with real numbers, not just accumulator-populated assertions: `tech_bonus_scales_matching_category_output`/`tech_bonus_does_not_apply_to_a_different_category` (`colony/production.rs`) assert an exact multiplied pool deposit (1.0 base × 1.25 for a +25% bonus) using the same category-isolation test shape `category_modifier_stacks_multiplicatively_on_matching_category` already established for body modifiers, chained with the pre-existing `bonus_accumulates_in_modifier_accumulator` (`turn/mod.rs`) proving a completed tech's `Bonus` effect reaches the accumulator through the real `TurnProcessor::apply_tech_effects` path — together these cover the full tech-completes → accumulator-populated → production-scaled chain. Balance harness (`content/checks/*`) re-verified: all 11 bundles pass unchanged (their short check sequences never complete a `Bonus` tech mid-run, so this is confirmed inert for them, not merely assumed).
  - **`TechUnlocks.bonuses`/`events` fate, decided: permanently decorative.** These YAML sugar fields (e.g. `"construction_speed_10_percent"`) are **not** migrated to real `TechEffect` entries — their slug format is human-readable flavor text, not a stable machine-parseable convention, and every bonus that needs a genuine numeric effect already has a real `effects: [{type: bonus, ...}]` block available (the mechanism this issue just wired up). Content authors use `effects:` for anything that must actually change a number; `bonuses`/`events` stay pure flavor text. A follow-up content pass (alongside #249) can migrate the handful of pre-#236 techs still relying only on the sugar fields to explicit `effects:` blocks, but that's a content-authoring task, not an engine change.

**#250 — frontend tech-tree UI expanded to the 81-entry tree, delivered.** `TechTreeView.vue` was already a real SVG-DAG graph (not a placeholder) with researched/available/locked coloring and single-project research — issue #250 extended it rather than rebuilding it: category and tier dropdown filters (the graph was designed for ~12 nodes, not 81), a research-queue panel showing the active project's live progress plus queued follow-on techs in order, a `Queue` action (alongside the existing `Research` action) that calls the already-real `EnqueueResearch` core command, a `Cancel` action for `CancelResearch`, and an effects list in the detail panel rendering each `TechEffect` variant in plain language. A new `queued` state (previously lumped into `in_progress` alongside the truly-active project, making them visually indistinguishable) is now surfaced server-side and given its own color/legend entry.

  - **Command-wiring checkerboard closed.** Auditing before implementing turned up that `research_tech`/`enqueue_research`/`cancel_research` were each missing from exactly one of the three layers (frontend `Command` union, `outpost_tauri::ClientCommand`, `outpost_web::ClientCommand`) despite the underlying core `Command::EnqueueResearch`/`CancelResearch`/`ResearchTech` and `TechState.research_queue: VecDeque<TechId>` all being fully implemented already (queueing was never a missing core mechanic, only missing plumbing). Added the three missing links: `enqueue_research`/`cancel_research` to the frontend `Command` type, `ResearchTech` to `outpost_web`'s `ClientCommand`/`ws.rs`, and `CancelResearch` to `outpost_tauri`'s `ClientCommand`/match arm.
  - **`outpost_web` gained a `get_tech_tree` REST route.** Mirroring the `get_colonize_targets`/`list_outposts` pattern (`query_routes.rs`), `GET /api/tech-tree` computes the same researched/`in_progress`/`queued`/available/locked state `outpost_tauri::commands::get_tech_tree` already computed for desktop mode, now also including each tech's `effects: Vec<TechEffect>` (both wire types gained this field) so the frontend can render them without a separate lookup.
  - **A real, previously-undiscovered browser-mode bug, found and fixed while writing the e2e spec.** `outpost_web`'s `handle_new_game` never loaded `content/base/tech.yaml` into `GameState.tech_registry` at all — every tech-dependent core command (`ResearchTech`, `EnqueueResearch`, the new `get_tech_tree` route) would fail with `"no tech registry loaded"` in browser mode, a gap that predates this issue (desktop/Tauri mode loads it via `load_embedded_tech`, but browser mode had no equivalent). Not a consequence of anything #250 added — it surfaced only because this was the first browser-mode feature to actually exercise the tech system end-to-end. Fixed by loading `content/base/tech.yaml` from disk in `handle_new_game`, mirroring the Tauri embedded-pack load but reading from the on-disk content directory instead; non-fatal on missing/unparsable file (a base pack without a tech tree still boots), matching the `if let Ok(...)` tolerance the desktop path already uses.
  - **Tests.** `frontend/src/views/__tests__/TechTreeView.test.ts` (new) covers category/tier filtering, the queue panel's active + queued display, `Research`/`Cancel` command dispatch, and effects rendering. `frontend/e2e/tech-tree-live.spec.ts` (new) drives a live `outpost_web` backend: new game → Tech Tree → filter by category → select an available node → start research → queue panel appears. `outpost_web`'s `wsmsg.rs`/`query_routes.rs`/`ws.rs` each gained matching Rust-side tests (`client_command_research_tech_deserialises`, `tech_tree_404_before_content_loaded`, `base_tech_yaml_loads_into_a_non_empty_registry`).

**Pacing — verified, not just claimed.** `outpost_core::tech::tests::real_tech_yaml_is_not_trivially_beelinable_at_baseline_research_rate` computes the real tree's total research cost against a baseline single-`research_lab` output rate (5/sol × 30 sols/month = 150/month) and asserts clearing the whole tree takes well over 300 strategic months — tech-ordering remains a real prioritization decision, not a beeline. A second test (`real_tech_yaml_parses_and_forms_a_wide_multi_tier_dag`) asserts the DAG shape itself (≥40 techs, all 6 categories present, ≥5 tiers, at least one convergence point and one fan-out point) so the shape claims above are enforced by CI, not just prose.

---

## 8. The Strategic Layer (multi-zoom)

### 8.1 Planet zoom
A **hex map** of the planet showing terrain, biome, and resource deposits. Colonies are nodes on hexes. Infrastructure (roads, pipelines, power lines) are connections between nodes, with cost and throughput based on distance and terrain crossed. Trade flows **automatically once a route exists, with manual priority overrides**. Roughly equal weight on *where to expand* (site selection, prospecting) and *how to optimize what exists* (infrastructure, balancing).

**Hex color layering (issue #316, resolved).** `PlanetHexMap.vue`'s fill color is a layered pipeline, in order: **terrain** (`outpost_core::map::Terrain` — the physical landform: Plains/Hills/Mountains/Wetlands/Ocean/Volcanic) sets the base ground color; a **water/ice overlay** blends on top, driven by `HexCell::water_coverage: f32` (`[0.0, 1.0]` — high for Ocean, moderate flat value for Wetlands, zero elsewhere) and tinted white/ice on a `Frozen`/`Extreme`-band cell or blue/liquid otherwise; a **vegetation overlay** blends on top of that, driven by `HexCell::vegetation_density: f32` (`[0.0, 1.0]`, derived from biome and tempered by how harsh the cell's temperature band is — a nominally-lush biome reads less green on a harsh-temperature cell); then the existing elevation-shading and temperature-tint layers apply unchanged. Both new per-cell fields are real gradients, not category lookups, and both default to `0.0` via `#[serde(default)]` so old save files/snapshots deserialize cleanly. Whether a body supports vegetation at all is a body-level property, `PlanetarySubtype::has_vegetation()` (true for `Unclassified`/`EarthLike`/`Ocean`, false for every barren/molten/icy/giant archetype) — `vegetation_density` is forced to `0.0` on every cell of a non-vegetated body regardless of biome roll. This replaced the earlier biome-only-derived vegetation overlay (itself a resolved follow-up from the original terrain-plus-biome pass) — deriving vegetation from a real per-cell density field, and giving water/ice its own overlay instead of being folded into terrain's base color, was the natural follow-up flagged when the biome-derived approximation shipped. `PlanetHexMap.vue` still falls back to the old biome/terrain-derived approximation when a hex is missing the new fields (older fixtures/snapshots), so the fallback path never disappears — it's just no longer the primary signal.

**Hex scale and map projection (issue #340) — decided, not yet implemented.** A hex is **~100 km across**. This is the scale that makes both halves of the design above work at once: a site is large enough to hold a substantial number of buildings, while a colony still occupies a small dot on the planet, so regional variety in terrain and climate is real and specialization emerges from geography rather than being asserted. The two ends were weighed explicitly. *Civ-scale hexes* (200–500 km) would make a hex a region rather than a site — big enough that a colony simply owns whatever deposits fall inside it, which dissolves the reach problem without new systems but leaves the hex map as a place-names layer carrying almost no gameplay. *Very fine hexes* (5–25 km) put hundreds of thousands to millions of hexes on a planet: simulation-only, never player-facing. 100 km sits at the coarse end of the *Emperor of the Fading Suns* band — the map is a real board, with adjacency, terrain, and routing that matter, while the count stays renderable.

The surface map is a **rectangle that wraps horizontally** (east edge joins west; poles are the top and bottom rows), and its hex count **derives from body radius** rather than being authored per body. Hex count is therefore a consequence of physical size, not a tuning knob: at 100 km, a Mars-sized body (145M km²) is roughly 17,000 hexes, a Ceres-sized body a few hundred, a super-Earth on the order of 60,000. That ~200× spread across bodies is the load-bearing constraint on this design — the same mechanics and the same UI have to hold at both ends, which is why nothing in the reach model may assume a particular map size. A player realistically interacts with a few dozen hexes over a campaign; the rest is terrain they fly over.

**The wrapping-rectangle topology is implemented (issue #315); the 100 km-real-scale hex count derived from a body's physical radius (issue #340) is not yet.** `PlanetMap::generate(seed, width, height)` builds a `width`-column × `height`-row rectangle: `q` wraps east-west (column `width - 1` is adjacent to column `0`), `r` is a hard-bounded pole axis with no vertical wrap (`r = 0` and `r = height - 1` are the poles), and `HexCoord::wrapped_distance`/`PlanetMap::wrap_coord` give every distance, adjacency, site-scoring, and infrastructure-pathing query the wrap-aware behaviour the old hex-of-radius-N disc never needed. `width`/`height` still come from `BodySize::map_dimensions()` — a size-class lookup table (§8.3AC), not yet a derivation from a body's actual radius in km — so the ~200×-across-bodies scale spread this section describes is still future work, layered on top of the topology change #315 already shipped.

**Deposit rendering, corrected + upgraded.** Deposits now render as a small colored box with a two-letter code (e.g. `SO` for `structural_ore`, `FI` for `fissile_ore`) rather than a plain colored dot — the code/color lookup table (`DEPOSIT_STYLE`) also fixed a real latent bug: the previous table's keys (`iron`, `rare_metals`, `water_ice`, ...) didn't match any of `outpost_core::map::VEIN_COMMODITIES`'s actual 9 entries (`structural_ore`/`conductive_ore`/`precious_ore`/`refractory_ore`/`semiconductor_ore`/`fissile_ore`/`silicates`/`hydrocarbons`/`biomass`), so every deposit had silently been falling back to a generic grey dot. Codes and colors are hand-picked client-side (no backend `short_code` field) with a first-two-letters-uppercased fallback for anything not in the table, so new commodities never render blank.

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

### 8.3AA Orbital-slot archetype variety + generation sliders (playtest feedback round 3)

Playtest feedback: the innermost inner-planet slot ended up "always most habitable" in practice, and the generator produced little of the Mercury/Venus/Mars/Jupiter/Neptune-style variety a player would expect across a system's orbital slots. Root cause wasn't a hard-coded "innermost wins" rule — `force_habitable` (`system_gen.rs`) picks whichever slot lands *nearest* the habitable-zone center, not literally index 0 — but the original distance curve (tight `[0.3, 0.5)` AU start, `1.5-1.9×` growth per slot) concentrated realistic habitability odds into just the first one or two slots, and gravity/atmosphere were rolled almost independent of *which* slot a body occupied.

Two changes, both in `system_gen.rs`:

- **The innermost slot (index 0) is now explicitly biased toward a Mercury-like archetype** — forced non-Temperate temperature, no breathable-atmosphere roll, small gravity (`0.2-0.5g`), high radiation — rather than left to the same distance-only roll every other slot gets. Gas giants split Jupiter-like (warmer, `1.8-3.2g`) vs. Neptune-like (colder, `1.0-1.8g`) based on their already-rolled temperature band, reusing the same Frozen/Cold split `subtype_for` already made between `PlanetarySubtype::GasGiant` and `IceGiant`.
- **The distance curve was widened** (closer start `0.25 + [0, 0.15)` AU, gentler `1.35-1.75×` growth, favorable-atmosphere band widened from `±0.35` to `±0.45` AU) so more than the first couple of slots have a realistic shot at landing near the habitable zone.

**New: `SystemGenParams` struct** (`system_gen.rs`) lifts the previously-hardcoded `HABITABLE_ZONE_CENTER_AU`/`MIN_INNER_PLANETS`/`MAX_INNER_PLANETS` constants into a parameters struct threaded through `generate_system`, `SystemCommand::GenerateSystem`, and `ClientCommand::NewGame` (all new fields optional with defaults matching the pre-slider constants, for backward compatibility). The New Game screen (`NewGameView.vue`, browser mode) now exposes: an independent **Star System Seed** (the backend already supported rerolling the system independently of the planet map since issue #199 — the UI simply never sent it before this), a **Habitable Zone Center** slider, an **Inner Planets** count slider, and a **Resource Abundance** slider (overriding the difficulty-derived default when moved). `outpost_tauri`'s `bootstrap` command accepts the same optional parameters for the desktop-shell New Game flow, and `MainMenuView.vue` (the Tauri desktop New Game panel — the primary UI host per this doc's Technology Stack table) now mirrors the same seed field + three sliders, wired through `tauriBridge.ts`'s `bootstrap()` helper's `genParams` argument. As with the browser-mode wizard, entering a value here and pressing Start in browser mode (where `MainMenuView`'s "New Game" button just routes to `/new-game`) doesn't use these fields — pre-existing behavior, not a regression from this change.

Widened by issue #318 (§8.3AF) to also cover gas-giant/asteroid-belt/cometary-belt/moon counts, following the same `SystemGenParams` slider pattern established here.

### 8.3AB Quantitative planet archetypes (issue #313)

`PlanetarySubtype` (`system.rs`) previously biased hex-map generation only *qualitatively* — a subtype nudged which commodities spawned as deposits (`subtype_commodity_multiplier`), but terrain/elevation generation itself was subtype-blind, so an "Ocean" world and an "EarthLike" one produced statistically identical land/water splits. Issue #313 makes the archetype roster's land/water ratio and elevation profile a real, quantitatively-hit property of the map:

- **`PlanetarySubtype::target_land_fraction()`** gives every inner-planet-compatible archetype (Mercury-/Venus-/Mars-/Pluto-like, EarthLike, Ocean, the new **Mountain**, and generic Rocky) an explicit target land fraction. `PlanetMap::generate_for_body_and_subtype` (`map.rs`) computes elevation for the whole map first, then picks the elevation *quantile* below which cells become ocean so the map's actual water coverage lands on that target — a genuine ocean world now reliably comes out under 25% land, not "mostly land with an occasional lake" from the old fixed low-probability roll. Returns `None` for `GasGiant`/`IceGiant`, which keeps the pre-#313 fixed-probability behaviour — see the open question below.
- **`PlanetarySubtype::elevation_bias()`** additively shifts every cell's raw elevation before the quantile is computed, so "all high elevation" archetypes (Mountain, +0.35) report genuinely high `HexCell::elevation`, not just a higher chance of rolling a mountain terrain tag.
- **New `PlanetarySubtype::Mountain` variant** (inner-planet only): land fraction 0.95 (issue's own "less than 10% water"), strong positive elevation bias, ore-favouring deposit multiplier (exposed rock, nothing buried under lowland sediment). `subtype_for` (`system_gen.rs`) rolls it as a 25% alternative to `RockyBarrenCold` when a cold inner planet has a thin (not vacuum) atmosphere.
- **`Unclassified` and `EarthLike` share one target (0.55)** — preserving issue #196's existing "Unclassified never biases anything differently than EarthLike" contract, which the new land/water axis would otherwise have silently broken.
- Frontend: `SystemMapView.vue` shows the archetype's named label ("Mercury-like", "Ocean world", ...) instead of the raw snake_case wire tag.

**Deliberately deferred, matching the issue's own open questions:**
- **Gas/ice giant surface-mapping.** `BodyKind::has_surface()` is still `true` for `GasGiant`, and `body_surface_preview` still generates a full hex map (oceans, mountains, ore deposits) for them, unchanged from before this issue — a giant genuinely has no solid surface for a land/water split to mean anything, and building a distinct atmospheric-band representation (new terrain kind, biome rules, suitability, frontend rendering) is a separate feature, not a quantitative-target tweak.
- **Habitability reading the archetype directly.** `Body::habitability()` is unchanged and still doesn't read `subtype` at all (its own doc comment predates #313 and says so explicitly) — archetypes now read distinctly through *hex-level* `suitability()`, which already factors in terrain/elevation/temperature and therefore differs structurally once land/water/elevation differ, without adding a new subtype term.
- **Which archetypes are colonisable**, and **tunable archetype frequency at new-game time** — both open questions in the issue itself, neither blocking the quantitative-generation core this issue asked for.

### 8.3AC Planet size drives hex-map radius (issue #314)

Hex maps used to come from a flat `radius` the caller passed in, unrelated to anything about the body — `Command::SeedPlanet`'s caller-supplied `radius` for the home/founding map, and a flat `BODY_SURFACE_PREVIEW_RADIUS = 8` (issue #289) for every other body's scouting preview, regardless of that body's kind or rolled attributes.

- **New `BodySize` enum** (`system.rs`): `Tiny`/`Small`/`Medium`/`Large`, mapping to hex radius `4`/`6`/`9`/`12` via `BodySize::hex_radius()`. A closed enum with a lookup table, mirroring `PlanetarySubtype`'s own shape (issue #313) — easier to author/balance than a continuous radius, and `Large`'s radius is deliberately the same ceiling `map.rs`'s existing `subtype_aware_generation_stays_within_performance_budget` test already exercises (radius 12, <100ms), so this issue doesn't push the performance envelope, just makes the existing ceiling a real outcome instead of a hardcoded default.
- **Derived from gravity, not an independent roll** (`system_gen.rs::size_for`): a body's `size` is computed from its already-rolled `gravity_g` right after `generate_body` finalizes it (including the innermost-slot Mercury-like override), so the two attributes stay consistent by construction — a heavier world is a bigger one. Moons are unconditionally `Tiny` regardless of gravity, matching the issue's own "moons get proportionally small maps" wording. Gas giants need no special case: their gravity floor (1.0) already clears the shared `Large` threshold (0.9), so giants come out big for free.
- **`Command::SeedPlanet`'s `radius` field is now a fallback, not the source of truth.** The handler resolves the designated home body first; if that body exists in a generated system, `body.size.hex_radius()` wins and the caller's `radius` is ignored entirely. The caller value only applies when there's no real system generated yet (most engine tests, the balance harness) — this is what let the change land without touching the ~25 existing `SeedPlanet` test call sites that pass small radii purely for test speed, none of which assert on the resulting radius/cell-count in a way this affects.
- **`body_surface_preview`** (`lib.rs`) now uses `body.size.hex_radius()` directly instead of the flat constant, which is removed.

**Deliberately out of scope**, per the issue's own framing: size affecting anything besides tile count (absolute deposit counts, colony footprint, travel time) is left for a future issue if it turns out to matter for balance.

**Superseded by issue #315.** `BodySize::hex_radius()` (a single radius, for the old hex-of-radius-N disc topology) is gone, replaced by `BodySize::map_dimensions() -> (width, height)`. The size-class table and its derivation-from-gravity are otherwise unchanged — only the shape each class resolves to changed, from a radius to a `width`/`height` pair sized to roughly the same per-class cell-count order of magnitude. See §8.3AD.

### 8.3AD Surface map wraps east-west, rectangular topology (issue #315)

The hex-of-radius-N disc (§8.3AC) never wrapped and had no real geographic poles — "near a pole" was a fuzzy distance-from-the-map's-coordinate-origin heuristic, not a place on the map. This issue replaced it with the rectangular, east-west-wrapping topology §8.1 already described as the target design, ahead of an "Approved" decision comment on the issue resolving its own stated crux (does the map shape change?) in favor of a breaking change rather than trying to make a hex-of-radius-N region wrap cleanly.

- **`PlanetMap`** now stores `width`/`height` (column/row counts) instead of `radius`. Cells are generated and stored canonically with `q` in `[0, width)` and `r` in `[0, height)`; `r = 0` and `r = height - 1` are the poles, with no vertical wrap.
- **`HexCoord::wrapped_distance(self, other, width)`** — a new method alongside the existing (unwrapped) `distance` — returns the minimum hex distance over the direct offset and the two wrap-shifted copies of `other` (∓ one map-width in `q`). Everything that measures proximity or enforces minimum separation across the map (`site_score`, `top_landing_sites`) uses this instead of plain `distance`.
- **`PlanetMap::wrap_coord`** canonicalises an arbitrary (possibly out-of-range) `HexCoord` back into `[0, width)` in `q` — needed because pathing and proximity scans naturally produce raw offsets outside the stored range near the seam.
- **Infrastructure pathing is wrap-aware.** `edge_cost` gained a `width` parameter and a `shortest_wrap_target` helper: it costs and interpolates a path against whichever of {direct, wrapped-west, wrapped-east} copy of the destination is actually nearest, so a colony pair straddling the seam is priced (and would draw) via the short way round, not the long way across the map interior.
- **Elevation generation is seam-seamless.** `compute_elevation`'s `q`-dependent sinusoidal terms use an integer spatial frequency scaled to the map's `width` (cycles-per-circumference, not an arbitrary constant), so column `width - 1` and column `0` — genuinely adjacent through the wrap — don't show a visible terrain cliff at the seam.
- **Latitude is now literal, not a seed-random rotated line.** The old model picked a random "equator" line through the hex disc's origin per seed; `cell_latitude_abs` now just measures a cell's row distance from the middle row. This retired an unconditional "near-origin cells are always Plains" special case that made sense for the old coordinate-distance heuristic but, measured against a real small map's row count, was forcing an implausibly large fraction of the map (both pole rows) to a fixed terrain regardless of the archetype's water/elevation target — poles are cold (via the existing latitude → temperature shift) but their terrain now follows the same elevation/water-threshold bucketing as everywhere else.
- **`snapshot::SCHEMA_VERSION` bumped 11 → 12.** An old save's serialized `PlanetMap`/`Command::SeedPlanet`/`Event::PlanetSeeded` shapes no longer deserialize under the new field names, so the existing schema-version hard-error (`SnapshotError::SchemaVersionMismatch`) is what rejects an incompatible old save with a clear message, per the issue's own stated preference over silently reshaping a player's colony placements.
- **Both hosts' wire types** (`PlanetMapWire` in `outpost_web`/`outpost_tauri`) carry `width`/`height` in place of `radius`; the frontend's `PlanetMap` TS type matches.
- **`PlanetHexMap.vue` renders seam-crossing infrastructure edges as two mirrored line segments** (drawn one map-width apart in pixel space) rather than a single straight line — whichever half falls inside the current viewBox is what's visible, giving a continuous wrap-around look without any special SVG clipping logic.

**Deliberately out of scope**, matching the issue's own "remaining detail, not blocking" framing: continuous/infinite horizontal pan vs. a clamped view with the seam visible was left as a future frontend polish item, not resolved here.

### 8.3AE Finite-deposit mode: opt-in richness depletion (issue #317)

§8.3A's coverage guarantee says every founding site has *something* to mine; it says nothing about whether mining it ever runs out. Historically it didn't — richness only ever affected *rate* (§8.3A's 50%-floor formula), never depleted. Issue #317 raised two open questions, both resolved by decision comment on the issue before implementation:

1. **Should a commodity input with no local deposit/trade route ever have an off-map "core worlds" import stopgap, or stay hard-blocked?** Resolved: hard-blocked — and this was already the shipped behavior (`colony::TRACE_DEPOSIT_RATIO`, §7's deposit-gating section), just not previously called out as a deliberate answer to this question. No code changed for this half.
2. **Should surface deposits deplete with extraction, or stay effectively infinite?** Resolved: both — infinite by default, finite as an opt-in difficulty knob. This is the actual new work.

**The toggle.** `GameState.deposit_depletion_enabled: bool` (default `false`, matching every prior world's behavior). `DifficultyPreset::Hard`/`Brutal` turn it on automatically via `Command::SetDifficulty`; `Sandbox`/`Easy`/`Normal` leave it off. `Command::SetCustomDifficulty` carries an explicit `deposit_depletion_enabled` field for the custom-difficulty panel's atomic apply (mirroring how `hazards_enabled`/`maintenance_enabled` are threaded through the same command). A standalone `Command::SetDepositDepletionEnabled { enabled }` also exists for toggling it independent of a full difficulty re-apply, mirroring `SetHazardsEnabled`/`SetMaintenanceEnabled`.

**The mechanism reuses `Deposit.richness` itself as the remaining-quantity signal**, rather than adding a second field to track depletion separately. `map::DEPOSIT_DEPLETION_UNITS_PER_RICHNESS: f32 = 500.0` is the conversion constant: a deposit at richness `1.0` holds 500 extractable units before it's gone. `PlanetMap::deplete_deposit(coord, commodity_id, amount_extracted)` converts the extracted amount to a richness delta and subtracts it; when richness hits zero or below, **the deposit entry is removed outright** rather than left as a lingering zero-richness ghost — so a fully-depleted deposit reads identically to a hex that never had one, and falls through to the exact same `TRACE_DEPOSIT_RATIO` trickle-floor path §7's deposit-gating section already implements. No new fallback logic was needed in `production.rs` for this.

**Wired into the turn pipeline** as a new step ("Step 3a2: Deposit depletion") right after the per-colony production loop and before outpost production: for each colony with a hex site, its `last_production_by_building` results are read back, filtered to recipe outputs in `VEIN_COMMODITIES` (the same curated raw-material list §8.3A's guarantees cover), and each is depleted by `output.quantity × line.scale` — the identical formula the pool-withdrawal pass already uses, so depletion tracks exactly what was actually produced that sol, not a theoretical maximum. The whole pass is a no-op unless `deposit_depletion_enabled` is true, so infinite-mode worlds pay zero extra cost.

**Deliberately scoped to hex-level `Deposit` only.** `system::BodyDeposit.abundance` — the system-wide flavor stat covering bodies without a colonized hex (used by outposts and gating for non-hex-sited colonies) — is left alone. It was never consulted for the 50%-floor gating math the same way hex `Deposit`s are, and extending depletion to it would require deciding how an outpost's off-site extraction should drain a shared system-wide number, a genuinely separate design question this issue didn't raise. A future issue can widen scope if outposts' effectively-infinite `BodyDeposit` abundance turns out to need the same treatment.

**Founding-site question left open, and deliberately not addressed by #317.** A related but distinct question — whether the founding-site guarantee itself should widen (e.g. showing planet-wide commodity absence in the map UI before founding) — was raised during scoping and explicitly deferred: it's better framed as a site-selection/proximity question (which #340's surface-expedition range mechanic already partially addresses) than as part of this depletion mechanic, so no map-UI change shipped here.

### 8.3AF System-composition New Game options: gas giants, asteroid belts, cometary belt, moons (issue #318)

§8.3AA gave the player New Game sliders for habitable-zone center, inner-planet count, and resource abundance — everything else in a generated system (gas giants, asteroid belts, the cometary belt, moon counts) stayed a hardcoded roll: gas giants `0..=2` (so a system could roll zero), an asteroid belt present with 75% probability (0 or 1, never more), a cometary belt present with 60% probability. Issue #318 widens the same slider pattern to cover these, resolved by decision comment before implementation:

- **Min/max range, not an exact count**, mirroring `min_inner_planets`/`max_inner_planets` — `SystemGenParams` gains `min_gas_giants`/`max_gas_giants` (default `1..=4`, reversing the old possible-zero roll), `min_asteroid_belts`/`max_asteroid_belts` (default `1..=3`), `min_cometary_belts`/`max_cometary_belts` (default `0..=1` — a **presence range, not a belt count**: a system has at most one Kuiper analogue regardless of how high `max_cometary_belts` is set), `min_giant_moons`/`max_giant_moons` (default `2..=20`, the prior hardcoded `MIN_GIANT_MOONS`/`MAX_GIANT_MOONS`), and `max_rocky_moons` (default `3`, the prior hardcoded `MAX_ROCKY_MOONS` — no paired minimum since `0` rocky moons was already valid).
- **Multiple asteroid belts sit inside the existing inner-system distance walk**, placed sequentially before any gas giant, each at a strictly increasing radius from the last — not interleaved with an outer Kuiper-analogue ring after the giants (a Solar-system-like inner-belt-plus-outer-belt layout was considered and explicitly rejected as a bigger, unnecessary change to the generator for this issue's scope).
- **Moon counts are in scope**, not deferred to a follow-up — `min_giant_moons`/`max_giant_moons`/`max_rocky_moons` get the same slider treatment in this same issue.
- **Slider surface is Tauri-only**, matching where §8.3AA's sliders already live (`MainMenuView.vue`) — `outpost_web`'s browser-mode `/new-game` view is not widened; its `handle_new_game` always resolves the six new fields to `SystemGenParams::default()`.

**UI treatment differs by field.** Gas-giant and asteroid-belt counts use a single slider sent as `min == max` (mirroring how `innerPlanetCount` already works for inner planets) rather than exposing two slider handles for what's framed to the player as picking one number. The cometary belt is a checkbox (`0` or `1`), not a slider, since it's a presence toggle. Giant moons genuinely wants two independent slider values — `giantMoonsMin`/`giantMoonsMax` — because each giant *in the same system* still rolls its own moon count independently within that spread; collapsing it to one exact value would remove the per-giant variety the generator has always produced. `SystemCommand::GenerateSystem` and `outpost_tauri::bootstrap` both grew nine new fields/parameters (all `#[serde(default = "...")]`/`Option<u32>` respectively, backward compatible with no `SCHEMA_VERSION` bump), and `#[allow(clippy::too_many_arguments)]` was added to `bootstrap` — a flat-scalar-argument Tauri IPC command that was already over clippy's default threshold before this issue.

### 8.3A Founding-site resource guarantee (issue #232)

Procedural system generation (§8.3, issue #199) guarantees a habitable founding body but says nothing about whether that body — or the system around it — actually has the raw materials to survive and progress. Issue #232 closes that gap with two guarantees, both **coverage guarantees, not depletion/economy guarantees** (deposits carry no finite budget by default; extraction doesn't drain them unless finite-deposit mode is on — see §8.3AE):

- **Founding site**: `PlanetMap` generation (`map.rs`) force-places at least one real hex `Deposit` of every commodity in the curated `VEIN_COMMODITIES` list if normal vein placement didn't produce one, mirroring Factorio's "starting-area" override pattern (a guarantee pass layered on top of, not replacing, ordinary probabilistic placement).
- **System-wide**: every generated `Body` gets a lightweight `BodyDeposit` tag (`system_gen.rs::distribute_system_resources`) — DSP-style archetype-biased placement (asteroid belts favor ores, gas giants favor hydrocarbons, moons skew fissile) with the same verify-and-patch guarantee on top, so every curated commodity exists *somewhere* in the system even before a colony can reach it.

**`VEIN_COMMODITIES`** (`map.rs`) is the single curated raw-material list both guarantees share — deliberately small (9 entries: `structural_ore`, `conductive_ore`, `precious_ore`, `refractory_ore`, `semiconductor_ore`, `fissile_ore`, `silicates`, `hydrocarbons`, `biomass`), mirroring `content/checks/bootstrap_colony`'s curated-starter-loadout philosophy rather than requiring every raw material in the content pack on one world. New raw-material commodity families should be evaluated against this list — added if a colony's tech-progression path genuinely needs them guaranteed-reachable, left out if they're meant to be a genuine system-wide scarcity/exploration incentive instead.

Deposit *generosity* (not coverage) scales with difficulty via `ModifiableQuantity::DepositAbundance`.

**Deposit gating — live as of issue #239.** Extraction recipes with an output in `VEIN_COMMODITIES` now actually check for a matching deposit at the colony's site/body, via a new `deposit_richness: Option<HashMap<commodity_id, richness>>` parameter threaded through `colony::process_production_scaled` (`compute_deposit_ratio` in `production.rs`). Three distinct cases, each load-bearing:

- **No spatial placement at all** (`None`) — a colony with neither a planet-map hex (never `FoundColonyAtSite`d) nor a `home_body_id` — e.g. every colony created via the bare `Command::FoundColony` test/fixture path used throughout the existing test suite. Gating is inert; this is the pre-#239 behavior, fully preserved (zero test breakage from this change).
- **Placed with a matching deposit** (`Some(map)`, entry present) — the recipe scales by richness: `ratio = 0.5 + richness × 0.5`, so any guaranteed deposit (#232's coverage guarantee) produces at least half output even at minimum richness, while richness above the guaranteed floor genuinely matters. This is the sizing method in place of a one-shot number: **coverage guarantees a non-zero floor, not a fixed abundance target** — a founding colony is never hard-locked out of a curated raw material, only slowed relative to a richer deposit.
- **Placed with no matching deposit** (`Some(map)`, entry absent/empty) — hard zero (`ratio = 0.0`), reported as a new `ShortfallReason::DepositShort`. A real site or body that genuinely has nothing to mine is meaningfully different from "no placement data to check" and must gate, not silently succeed.

`GameEngine::colony_deposit_richness`/`body_deposit_richness` (`lib.rs`) resolve the map from hex `Deposit`s (colonies with a planet-map site) and `BodyDeposit`s (colonies/outposts linked to a system body), taking the max richness/abundance per commodity across both sources.

**Sizing/tuning scope actually delivered vs. deferred further:** the 50%-floor formula above *is* the "how much abundance is enough" answer for the coverage-guarantee case — derived from the design constraint that #232's guarantee must never fully starve a colony, not from an arbitrary number. What's still deferred (no interactive playtesting available to generate real data yet): whether the exact `DepositAbundance` difficulty-grade-table multipliers (Sandbox 3.0×, Easy 1.5×, Normal 1.0×, Hard 0.7×, Brutal 0.5×) translate into a *felt* difficulty curve once gating is live — left unchanged pending real playtest data, tracked as follow-up tuning work rather than guessed at here.

**Test coverage note:** `content/checks/*` (the balance-harness bundle format) has no concept of hex/body spatial placement in its `colony.yaml` schema — every harness scenario builds a bare, unplaced colony, so deposit gating is inert for all of them by design (confirmed: every existing bundle still passes unchanged). The "founding site with guaranteed deposits reaches a meaningful milestone" proof this issue's Definition of Done calls for is instead delivered as engine tests (`lib.rs::founding_site_with_guaranteed_deposit_sustains_gated_extraction` / `founding_site_without_matching_deposit_blocks_gated_extraction`) that seed a real `PlanetMap`, locate the specific hex #232's guarantee placed a deposit on, and found a colony there via `FoundColonyAtSite` — closer to the real mechanism than the harness format can currently express.

### 8.3B Outposts (issue #233)

An **outpost** is a lightweight, single-purpose off-world presence that extends a colony's reach — the mechanism by which a colony reaches resources or performs work "located elsewhere" in the system (§232's motivating need), rather than relocating or founding a second full colony everywhere a needed material happens to be. Always owned by a parent colony (`parent_colony_id`) and anchored to a specific system body (`body_id`); it does not exist independently.

**Architecture decision** (resolved after auditing `orbital::OrbitalStation` and the megaproject mechanic, per this issue's own "avoid a fourth parallel thing that produces stuff on a body" caution):

- `orbital::OrbitalStation` was ruled out as a base to generalize — it's purely structural bookkeeping (slot cost, a turn-countdown construction project) with no production/upkeep loop of its own, and is scoped system-wide rather than to a body.
- A bare `PlacedBuilding` wrapper was ruled out — `PlacedBuilding` has no resource pool or construction-queue context of its own; it's meaningless detached from a container that owns a `ColonyPool`.
- The chosen shape: a genuinely new `outpost::Outpost` struct, stored in its own `GameState.outposts: Vec<Outpost>` — **not** a reuse of `Colony`/`state.colonies`, since `Colony` and `PopulationPool` are 1:1-indexed everywhere in the codebase (`FoundColony` always creates both together) and an entity with no population would break that invariant.
- Despite being a new struct, `Outpost` shares the *same internals* a `Colony` already uses rather than duplicating the production pipeline: `ColonyPool`, `PlacedBuilding`, `ConstructionQueue`/`ConstructionProject`, and `colony::process_production_scaled` are all reused unchanged, because none of them are actually coupled to `Colony`/population — `process_production_scaled` takes a raw `labor: f32` and a pool, not a `Colony` reference. An outpost supplies a fixed skeleton-crew `labor` constant (`outpost::OUTPOST_BASE_LABOR`) in place of a `PopulationPool`-derived value, and a neutral `1.0` habitability modifier (outposts aren't founded on habitability grounds).

**Upkeep shortfalls reduce output, they don't destroy the outpost** — this falls out for free from reusing `process_production_scaled` unchanged: a power/maintenance shortfall already scales a building's output down via the same mechanism a colony building uses, with no new failure-state machinery needed.

**Megaproject support** reuses `system::SystemCommand::ContributeToMegaproject` as-is — that command already accepts raw `resources`/`research` with no colony/source reference, so `Command::ContributeOutpostToMegaproject` is a thin withdraw-from-outpost-pool-then-forward wrapper, not a new contribution pathway.

**Deliberately out of scope for #233** (tracked as sub-issues of #233): tech/bonus-gated building availability and the max-range constraint (#241, delivered — see §8.3C below); promotion of an established outpost into a full colony/city (#242, delivered — see §8.3D below); frontend UI (#243).

---

### 8.3C Outpost gating — tech/bonus and max range (issue #241)

#233 deliberately shipped `EstablishOutpost` and `QueueOutpostConstruction` with no gating at all. This issue splits the two kinds of gating the original design called for onto the two commands where each actually belongs, rather than gating both the same way:

- **Range gate on `EstablishOutpost`.** A colony can only establish an outpost on a body within `outpost::max_outpost_range_au(propulsion_level, outpost_range_bonus_au)` AU of its own `home_body_id` — `BASE_OUTPOST_RANGE_AU` (3.0 AU) scaled by the system's `propulsion_level`, plus an additive tech-driven bonus. Exceeding it returns `EngineError::OutpostOutOfRange`. This reverses #233's "establishment is never gated" note — the issue's own DoD explicitly asked for range enforcement on `EstablishOutpost` itself, not on construction, so that note is superseded here rather than left standing.
  - **Grandfathering**: the check only runs when the *parent colony* has a `home_body_id` (i.e. was founded via `FoundColonyAtSite`/spatially placed). A colony founded via the bare `Command::FoundColony` (no spatial placement — the shape used throughout most of the test suite and by any caller that hasn't opted into the planet-map/system-map flow) has no "distance from home" to measure, so the gate is inert for it rather than defaulting to a spurious zero distance that would make every body simultaneously in- and out-of-range. This mirrors #239's `Option`-typed grandfathering pattern for deposit gating.
- **Tech gate on `QueueOutpostConstruction`.** Rather than adding a new curated `outpost_eligible` flag to `BuildingDef` (the scope item 1 alternative), this reuses the existing `BuildingDef.tech_prerequisite` field and the same convention `tech::unlocked_buildings` already applies to colonies: a building with no prerequisite is always available, one with an unresearched prerequisite is rejected with `EngineError::TechLocked`. The check only runs when the requested `building_type` resolves to a real `BuildingDef` in the loaded registry — an unregistered building-type string used by an older ad-hoc test is left to fail downstream unchanged, not newly rejected here. (Enforcing this same check on the *colony*-side `QueueConstruction` is issue #247's separate, broader concern — not duplicated here.)
- **`TechEffect::ExtendOutpostRange { bonus_au }`** — new tech effect, applied by `TurnProcessor::apply_tech_effects` into `GameState.outpost_range_bonus_au` (additive across every researched tech carrying it, same accumulation convention as `TechEffect::Bonus`). `propulsion_engineering` (tier 3, astronautics — already reduces survey transit time) is the first tech authored with this effect, at `bonus_au: 5.0`, giving the mechanism a real, non-synthetic consumer.

**Sizing rationale for `BASE_OUTPOST_RANGE_AU = 3.0`**: chosen so a colony with `propulsion_level` 1 (the game's starting value) can reach a same-inner-system asteroid belt or an adjacent planet (bodies are typically authored 1-a-few AU apart in `content/base/systems.yaml`) but not casually reach an outer-system body without investing in propulsion tech first — a real, if initially rough, constraint rather than a number wide enough to never actually bind. Left open for harness-driven tuning once outposts get real playtesting, matching #239's "derive a principled floor, tune the exact number later" approach.

---

### 8.3D Outpost promotion to a full colony (issue #242)

`Command::PromoteOutpostToColony { outpost_id, name, starting_population }` converts an established `Outpost` into a full, independent `Colony` — the alternative to `DecommissionOutpost` when the outpost's investment should be kept and grown rather than discarded (as `DecommissionOutpost`'s own doc comment has flagged since #233).

**Design calls made** (per this issue's own "needs a design call, not an assumption" framing for both open questions):

- **Promotion prerequisites: unconditional.** Any established outpost can be promoted at any time — no minimum stockpile, building, or tech unlock is required. This matches #233's own precedent for `EstablishOutpost` ("any colony can establish an outpost on any known body at any time") and avoids inventing a balance threshold with no playtesting data to derive it from, the same reasoning #239 used to defer `DepositAbundance` grade-table tuning. A future issue can add a real prerequisite once actual play reveals promotion is too easy/cheap as a strategy.
- **Post-promotion relationship: fully independent.** The resulting `Colony` has no retained link to its former parent colony or to the fact it was ever an outpost — `Outpost.parent_colony_id` is simply dropped. `Colony` has no "parent colony"/"child outpost" concept anywhere else in the data model to hang such a link on, and no consumer (UI, production, directives) currently needs one; adding a speculative link field with no reader would violate the "don't design for hypothetical future requirements" principle. If a real need for tracking colony lineage emerges later, it can be added then.

**Mechanics**: `colonies`/`populations` are two separate, index-parallel `Vec`s (`GameState.colonies[i]` and `GameState.populations[i]` are always the same entity) maintained *only* via `GameState::add_colony`, which pushes to both together — this is a load-bearing convention with no runtime assertion behind it, so promotion (like `FoundColony`/`FoundColonyAtSite` before it) goes through `add_colony` rather than pushing to `state.colonies` directly. `Colony`'s fields are a proper superset of `Outpost`'s: `pool`, `buildings`, `build_queue`, `category_modifiers`, `active_recipes`, and `last_production` carry over unchanged (identical types on both structs); `body_id` becomes `home_body_id` (wrapped in `Some`); `slot_capacity` carries over from the outpost but is floored at `colony::BASE_SLOT_CAPACITY` so a promoted colony isn't stuck below the normal colony minimum just because outposts start smaller (`outpost::OUTPOST_BASE_SLOT_CAPACITY` is 2 vs. the colony base of 5); `habitability_modifier` is recomputed fresh from the body's current state (mitigations included) rather than carried over from the outpost's establishment-time cache, since a colony's habitability is meant to reflect the body's *live* state and tech mitigations may have been researched since the outpost was established. A fresh `PopulationPool` is spun up from `starting_population` exactly as `FoundColony` does, since an outpost has no population of its own to carry over. Per-colony side-maps (`stability_trackers`, `population_trackers`, `interrupt_configs`) need no explicit initialization — they're all lazily populated on first use, the same as for any newly founded colony.

**Balance-harness coverage note**: `outpost_harness`'s `ColonyConfig`/`simulate.rs` always founds colonies via the bare `Command::FoundColony` (no hex/body placement field exists in that schema — confirmed by inspection, same limitation #239 hit), so a colony as instantiated by the harness always has `home_body_id: None` and the range gate is structurally inert for it. Substituted with engine-level tests instead (`establish_outpost_within_range_succeeds_for_placed_colony`, `establish_outpost_beyond_range_fails_for_placed_colony`, `establish_outpost_range_extending_tech_permits_a_previously_out_of_range_body`, `queue_outpost_construction_fails_when_tech_not_researched`, `queue_outpost_construction_succeeds_once_tech_researched`, plus a focused `extend_outpost_range_tech_effect_accumulates_additively` unit test for the tech-effect wiring itself) rather than silently declaring the harness tier inapplicable without explanation.

---

### 8.3E Outposts frontend UI (issue #243)

`frontend/src/views/OutpostsView.vue` — a colony-scoped list/management view (routed at `/outposts`, linked from the header nav) covering the full outpost lifecycle: establish (a body-picker filtered by #241's range gate, showing `in_range`/distance per candidate), view (pool stockpile, buildings, slot usage), queue a construction project, decommission, and promote to a full colony (#242). Deliberately **not** included in this pass: per-building recipe selection (`SetOutpostActiveRecipe`) and megaproject contribution (`ContributeOutpostToMegaproject`) — both have real engine commands already, but neither is needed to satisfy "visible and manageable," and adding their UI now would be scope creep ahead of any real playtesting signal that they're needed; can be added as a follow-up once outposts get actual play.

**Wire-layer additions** (`outpost_tauri::commands`, `outpost_web::{wsmsg, ws, query_routes}`): the four outpost/promotion `Command` variants (`EstablishOutpost`, `DecommissionOutpost`, `QueueOutpostConstruction`, `PromoteOutpostToColony`) and their `Event` counterparts are mirrored into both wire layers' `ClientCommand`/`ServerEvent` shims, following the exact translation pattern every other command already uses (parse UUID strings, forward to `Command`/`Event`, `Unknown`/`Ignored` fallback for untyped events). Two new bespoke read-only endpoints (following `get_colonize_targets`'s established "read `engine.state` directly, no core `Query` variant" pattern, documented at `query_routes.rs`'s module doc): `list_outposts` (mirrors `ListColonies`'s "return everything, client filters by parent" shape) and `get_outpost_targets(colony_id)` (computes `outpost::max_outpost_range_au` against the given colony's `home_body_id`, annotating every system body with `distance_from_home_au`/`in_range` so the placement flow can filter/explain out-of-range bodies before the player even attempts `EstablishOutpost`, rather than only discovering it on rejection).

**`outpost_tauri` verification caveat**: consistent with `outpost_tauri`'s pre-existing, documented WebKit2GTK/gdk system-library gap (`CLAUDE.md`), `cargo check -p outpost_tauri` cannot run in this environment (confirmed: `pkg-config` fails to locate `gdk-3.0`). The `outpost_tauri::commands` changes were written by exact structural mirroring of the parallel, independently-compiled-and-tested `outpost_web` wire layer (same field names, same UUID-parsing pattern, same `ServerEvent`/`ClientCommand` shape) rather than left unverified — but true compiler verification of that crate specifically is deferred to an environment where it's available (CI, or a local dev machine), per the standing policy.

**A real, pre-existing bug fixed as part of this work** (found via the new `outpost-live.spec.ts` e2e spec failing intermittently when run alongside `found-colony-live.spec.ts`, both hitting `outpost_web`'s shared single-engine backend): `useGameSocket()` was called independently from both `App.vue` (root, app-lifetime) and `NewGameView.vue` (a routed view, mounted only for the `/new-game` screen), and — being a plain composable with no shared state — each call opened its *own* independent `WebSocket` connection. Two live connections to the same session both received every broadcasted event (double-applying some to the world store) and, worse, `NewGameView`'s `onUnmounted` teardown called `disconnect()` on *its own* socket when the user navigated away, which unconditionally clobbered the shared `connectionStatus` to `'disconnected'` even though `App.vue`'s own connection was still alive. Fixed by making the connection module-level singleton state (one real `WebSocket`, shared `send`/`disconnect` across every caller) instead of per-call state. A second, related race was fixed in `worldStore.ts`: `new_game_snapshot` handling now authoritatively sets `gameStore.selectedColonyId` from that direct, request-scoped response, rather than leaving colony selection to each view's ad-hoc "pick `colonies[0]` if unset" fallback (`ColonyView.vue`, `OutpostsView.vue`) — that fallback could latch onto whichever colony arrived first over the shared connection, which is not necessarily the one *this* session's own `new_game` call just founded. Both bugs were latent since `outpost_web`'s browser-mode bootstrap existed (#220) — they only became *visible* once a second live e2e spec (`outpost-live.spec.ts`) made the resulting cross-session id mismatch surface as a real, reproducible test failure (`"colony not found"`) instead of a silent, cosmetically-harmless UI flicker no prior spec asserted against.

**Test coverage**: `frontend/e2e/outpost-live.spec.ts` drives the full lifecycle against a live `outpost_web` backend — new game, navigate to Outposts, establish on the first in-range body, assert it appears in the list, promote it, assert it's removed from the list (now an independent colony). Also required setting Playwright's `workers: 1` (`playwright.config.ts`) — with `fullyParallel: true` and no worker cap, two live-backend spec files running concurrently each drove their own `new_game` reset against the *same* shared engine, racing. Serializing spec execution costs a little wall-clock time on this still-small suite but removes an entire class of shared-backend flakiness for any future live spec, on top of the two bugs above already making it correct regardless of ordering.

---

### 8.3F Settlement tiers: expedition / outpost / colony (issue #340)

Three distinct kinds of presence, in increasing permanence and capability. This is the canonical hierarchy; the tiers are a deliberate progression, not three names for one mechanic.

| | **Expedition** | **Outpost** | **Colony** |
|---|---|---|---|
| What it is | a temporary camp | a satellite facility tied to a parent colony | a full-fledged city |
| Function | exactly **one** — extract a resource, or one other single job | limited, but may hold buildings of **more than one type** | unrestricted |
| Built on site | nothing | buildings | buildings |
| Population | a crew, no `PopulationPool` | skeleton crew (`outpost::OUTPOST_BASE_LABOR`) | a real `PopulationPool`, 1:1 with the `Colony` |
| Ends by | being recalled | being decommissioned (or promoted, §8.3D) | — |

The distinction earns its keep economically: an expedition is how you exploit something opportunistically or early, an outpost is how you commit to a location for a narrow purpose, a colony is a place people live. Nothing about an expedition is constructed, so nothing is lost by recalling one; an outpost represents real invested construction.

**Why this needed writing down.** "A remote thing on another hex that feeds a parent colony continuously" describes an expedition *and* an outpost, so without the table above the two would have drifted into duplicating each other — exactly the "fourth parallel thing that produces stuff on a body" failure §8.3B was written to avoid. The separating rules are: **an expedition builds nothing and does exactly one job; an outpost builds buildings and may do several.**

---

## 9. Expeditions & Exploration

A **schematic system node map** (planets, moons, asteroid belt as nodes; travel time by distance and propulsion tech). Exploration is **textured, not abstracted**: journeys and surveys trigger events, encounters, and mid-mission decisions that affect the outcome. Surveys reveal candidate colony sites and resource locations (full / partial / failed reveals).

This layer repurposes the existing event/decision infrastructure from colony-disasters to expedition-encounters.

### 9A. Body-scouting survey expeditions (issue #235)

Implements the schematic-survey model above: `Command::LaunchSurveyExpedition` targets any `system::BodyId` (not a planet-map hex) with one of `expedition::ExpeditionType`'s four tiers (`FastFlybyProbe` → `OrbitalSurvey` → `Lander` → `MannedExpedition`, unmanned to crewed, cheap-and-thin to expensive-and-thorough). This is distinct from the older `Command::LaunchFieldExpedition` (planet-hex only, deterministic-arithmetic discovery rolls, M8/#103) — both remain live; `LaunchSurveyExpedition` is the system-scale counterpart.

**Probe/satellite naming overlap with #234 — resolved.** #234 separately introduced body-scoped `orbital::SatelliteConstellation` and floated "probes" as a satellite concept. Settled as: **#235 owns "probe" as body-scouting** — `ExpeditionType::FastFlybyProbe`/`OrbitalSurvey` are unmanned survey missions that resolve to a `SurveyOutcome` via `resolve_survey`, model risk/cost/data-quality tradeoffs, and (per #234's own design doc) a probe is "simply a `SatelliteConstellation` ... with `body_id` set to a non-home body" only in the sense of *standing coverage* (comms/sensor/defense layers), never a one-shot scouting mission. **#234 keeps "satellite" strictly to standing orbital coverage.** The two systems don't overlap in practice: a `SatelliteConstellation` is a persistent asset that contributes `CoverageFootprint` every turn; an `ExpeditionType::FastFlybyProbe` is a one-shot mission with a lifecycle (`InTransit` → `Surveying` → `Completed`) that resolves once and is done. No shared entity was built, matching #235's own "no distinct probe type; reuse existing coverage math and command" framing for the *satellite* half specifically.

**Transit → survey → resolution.** Each `ExpeditionState` in the new `GameState.expedition_registry` counts down a fixed `SURVEY_TRANSIT_TURNS` transit leg, then an `ExpeditionType`-specific survey leg (`base_duration_turns`), advancing one sol at a time in `Command::AdvanceColonySol`'s Step 4f — deterministically, via `expedition::deterministic_roll(expedition_id, sol ^ salt)` rather than an external RNG stream (keeps replays and saved-state resumption reproducible, matching the older field-expedition system's precedent of deterministic-arithmetic rolls over injected randomness).

**Anomalies fire through the interrupt system, as designed.** Each `Surveying`-phase expedition is checked once per sol against every loaded `expedition::AnomalyDef` (content-pack data, `content/base/anomalies.yaml`); a trigger halts the survey countdown, builds a `MidMissionEvent` (Investigate / Ignore), and moves the expedition to `ExpeditionPhase::AwaitingDecision`. The event is surfaced via the *existing* interrupt collection (`InterruptSource::EventFired`, tier taken from the event itself) rather than a new interrupt variant — the module doc's original intent ("mid-mission interrupts that reuse the interrupt + predicate system") is honored literally, not just in spirit. `Command::ResolveMissionDecision` resolves the pending choice; investigating rolls a weighted `AnomalyOutcome` (`expedition::resolve_anomaly_outcome`) and applies its `research_bonus` into `SystemResearchPool`, `resource_reward` into the origin colony's pool, and `unlocks_tech` (if any) via the same `TurnProcessor::apply_tech_effects` path normal research completion uses — not a shortcut that skips tech-effect application.

**Survey completion reveals state, it doesn't gate anything.** `resolve_survey`'s outcome (`FullReveal`/`PartialReveal`/`Failed`) sets `Body.surveyed = true` (and `Body.candidate_site_name` on a full reveal) — new, UI-facing world state, not a new mechanical gate. Deposits themselves (`Body.deposits`, #232) were already always-true world state with no fog-of-war; #235 deliberately does **not** retrofit deposit visibility gating onto that model — `surveyed` only marks that an expedition *looked*, matching #232's explicit "coverage guarantee, not a gate" precedent rather than inventing a new one here. **Superseded by §9C (#344):** the game now has fog of war and deposits are hidden until found. `Body.surveyed` survives as the *system-scale* marker described here, but the "always-true world state" claim no longer holds — surface deposits are gated. See §9C.

---

### 9B. Surface expeditions — reaching off-colony deposits (issue #340)

The problem this solves: `#232` gave bodies real per-hex deposits and mining recipes read deposit richness, but a colony can only work the hex it occupies, so a deposit one hex away is as good as absent. A colony not founded on a vein was permanently cut off from that ore chain, which loaded the founding-site decision far past its intended weight and left the whole map inert.

**The mechanic.** A colony *builds* an expedition (see §8.3F for what an expedition is), and the player picks a target hex within its range. Once deployed it **returns resources every sol from the deposit it sits on, continuously, until recalled** — not a one-off haul. Its **cost is entirely up front, in resources**: there is no per-sol crew upkeep, so a deployed expedition needs no ongoing accounting beyond its yield tick, and recall returns the expedition rather than the outlay.

**Range is a radius derived from terrain and technology**, not a flat constant — terrain difficulty on the path plus the colony's unlocked tech together set how far it can reach.

**Contention is per resource, not per hex.** Two expeditions may target the same hex provided they extract *different* resources. Since an expedition performs exactly one function, one expedition means one resource, so this needs no special-casing.

**Terrain's effect on yield lives in the deposit, not the extraction path.** Terrain does influence how much a deposit gives up, but that influence is baked into the deposit's own properties at generation time rather than applied as a second multiplier when extracting. This keeps the per-sol extraction arithmetic identical to a colony's — the existing deposit-richness path in `colony::process_production_scaled` is reused unchanged — and puts terrain in exactly one place. It follows §8.3's precedent, where `#184`'s body modifiers already fold into yields rather than being applied at the point of use.

**Failure resolves from a content-authored effect table** — loss of colonists, loss of resources, and so on — reusing the `AnomalyDef`/`AnomalyOutcome` mechanism §9A already established (`content/`-authored, weighted, resolved in `expedition.rs`) rather than hardcoding failure effects in the kernel.

**Why not vehicles, and why not a deposit floor.** Two alternatives were weighed and rejected. *Free-roaming mining vehicles* — a colony builds vehicles that drive to a hex, extract, and return — give more logistics texture but constitute a **third** parallel logistics layer alongside trade convoys (§7/#332) and infrastructure edges (§8.1/#26), for gameplay the expedition model already delivers; the reach, the cost, and the risk are all expressible without per-vehicle state, dispatch scheduling, or movement rendering. *A low-richness surface-deposit floor on every hex* would make mining always work with a one-line generation change, but drains the meaning out of the deposit map entirely, which is the opposite of what the 100 km hex scale (§8.1) was chosen to achieve.

**Deposit visibility is fog-of-war — see §9C.** §9B needs deposits hidden until found, for the targeting decision to carry any weight at all. That contradicted `#235`/`#232`, which had asserted deposits are always-visible world state. **Resolved in favour of fog-of-war**, and the earlier precedent is deliberately retracted rather than worked around; see §9C for the model and for what it supersedes.

---

### 9C. Fog of war (issue #344)

**Decided: the game has fog of war.** Resource deposits, and eventually other world facts, are hidden until the player finds them. This is a deliberate reversal of the earlier position, and it **supersedes** two prior decisions:

- `#232` established `Body.deposits` as a "coverage guarantee, not a gate" — always-true world state with no visibility model.
- `#235` (§9A) explicitly declined to retrofit visibility gating, keeping `Body.surveyed` as a marker that "an expedition *looked*" rather than a gate on anything.

Both were reasonable when the map had nothing to *do* with deposits. Once §9B makes reaching a deposit a real decision, always-visible deposits collapse that decision into arithmetic — the player reads the map and takes the best hex. **The reversal is the point, not a concession:** hiding deposits is what gives survey and exploration a reason to exist, and turns the 100 km hex map (§8.1) from a backdrop into something worth investigating.

**Revealing is progressive, and technology makes it cheaper over time.** Early game, discovery is manual and local: expeditions and surveys extend the known area outward one target at a time. Later, technology buys breadth — survey satellites, reconnaissance flights, deep-scan instruments — so a mature player is no longer squinting hex by hex. This gives the tech tree a category of unlock whose value is *information* rather than throughput or capacity, which nothing in §7A currently occupies.

**The starting state: a well-known home planet, an unknown system.** The colony ship is assumed to have done survey work on approach, so at founding the player gets **detailed information about the immediate area** (deposits, terrain, anomalies in the founding neighbourhood) plus a **general map of the whole home planet** — its terrain and broad geography, but not what every distant hex contains. Every *other* body in the system starts near-blank: little to nothing about surfaces, deposits, or anomalies until the player sends probes or expeditions.

This asymmetry is what makes the opening legible without making it solved. The player can orient themselves and plan locally from turn one, and the rest of the planet is a map with blanks on it rather than a black void — but the system beyond is genuinely unexplored, and §9A's survey expeditions are how that changes.

**This settles the boolean-vs-graded question: fog is layered, not a single flag.** A general planet map that shows terrain while hiding deposits cannot be expressed by one `revealed: bool` per hex — at founding, a distant home-planet hex is simultaneously *known* (terrain) and *unknown* (contents). So per-hex visibility needs at least two independent layers:

- **Geography** — terrain, biome, elevation. Revealed planet-wide at founding for the home body; requires survey for other bodies.
- **Contents** — deposits, anomalies, and anything else worth travelling to. Revealed only locally at founding, and thereafter only by going and looking (or by a technology that looks remotely).

Whether *contents* is itself further graded (a satellite detects "something is there" while only a ground survey establishes richness) is still open, but the two-layer split above is settled.

**Two scales of fog, kept distinct.** The system scale (which bodies exist, and what they hold) and the surface scale (which hexes on a body hold what) are different questions with different instruments, and conflating them would make one mechanic answer both. `Body.surveyed` (§9A) remains the system-scale marker; per-hex deposit visibility on the `PlanetMap` is the new surface-scale one. A body-scale survey should not reveal every hex on the surface, and a surface expedition should not reveal other bodies.

**Existing machinery this should reuse rather than duplicate:**
- `#234`'s `SatelliteConstellation` already computes a body-scoped `CoverageFootprint` every turn. That is the natural implementation of "survey satellites" — a coverage layer that reveals hexes — rather than a new orbital thing. Worth checking whether the reveal falls out of coverage math directly.
- `#235`'s survey-expedition lifecycle and `AnomalyDef` outcome tables already model "send a thing, get information back, with variance." A reconnaissance flight is a short-range sibling, not a new subsystem.
- `#184`'s modifier plumbing already carries tech effects into world values; an information-granting `TechEffect` variant is a new *kind* of effect but not a new mechanism.

**Open, and deliberately not decided here:** whether the *contents* layer is itself graded (a remote sensor detects "something is there" while only a ground survey establishes richness); whether reveals are persistent once made or can decay; and whether terrain or atmosphere impedes remote sensing. Persistent-once-seen is almost certainly right — decay creates busywork unless it's tied to something meaningful.

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
10. ~~**Hex scale & off-colony resource reach**~~ — **RESOLVED, see §8.1, §8.3F, §9B** (100 km hexes, wrapping rectangle sized from body radius, expedition/outpost/colony tiers, continuous-yield recallable surface expeditions). Remaining TBD: the **range/yield numbers** themselves (harness). The deposit-visibility contradiction that previously blocked this is resolved — see item 11.
11. ~~**Deposit visibility / fog of war**~~ — **RESOLVED, see §9C** (fog of war exists; deposits hidden until found; per-hex visibility layered into *geography* and *contents*; home planet starts with planet-wide geography plus local contents, the rest of the system near-blank; technology buys breadth over time; system-scale and surface-scale fog kept distinct). Supersedes #232's "coverage guarantee, not a gate" and §9A/#235's decision not to gate visibility. Remaining TBD: whether the *contents* layer is further graded, whether reveals decay, and whether terrain impedes remote sensing.

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
