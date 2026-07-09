# Harsh Realm — Active Tasks

This is the current list of outstanding tasks and features for the Harsh Realm project.
For historical context and completed architectural migrations, see [docs/archive/todo_superseded_2026-05-30.md](docs/archive/todo_superseded_2026-05-30.md).

> **Active initiative:** the Rust Core Migration (branch `feat/rust-core-migration`).
> Design rationale lives in [docs/design/rust-core-migration-plan.md](docs/design/rust-core-migration-plan.md).
> See the "Rust Core Migration" section at the bottom of this file for the task list.

## Architecture & Persistence

- [x] **SQLModel Evaluation:**
  - [x] Evaluate adopting SQLModel for schema/row definitions or broader use vs. current Pydantic + Repository pattern.
- [x] **Town & NPC Migration (ARCH-08):**
  - [x] Move disposition changes, healer interactions, town entry/leave state, and NPC state persistence behind event-driven adapters.
- [x] **YAML API Refactoring:**
  - [x] Replace generic `object` returns in `GET /export/{table}` in `api/editor/yaml_files.py` with typed union/envelope models.
  - [x] Replace raw `dict[str, ImportAllTableResult]` return in `api/editor/yaml_files.py` with an explicit `ImportAllResult` wrapper.

## Runtime Simulation

> **2026-06-09 — ECS removed.** The exploratory ECS substrate
> (`src/harsh_realm/ecs/`) was torn out in favour of the rules-based pattern
> (focused entities + domain subsystems coordinated by events, plus the
> trigger/effect DSL), per `docs/superpowers/specs/2026-04-22-rules-architecture-design.md`.
> ECS — or a similar data-oriented pattern — may be revisited only if a specific
> large-scale system shows real performance problems. Where ECS work had been
> done, the behaviour now lives in plain scene logic / subsystems:
>
> - Hazard/trap triggering: inlined in `DungeonScene._trigger_traps` (emits the
>   same `combat.take_damage_requested` / `action.save_requested` events).
> - Combat HP: `Combatant` models are mutated directly (`combat_support`).
> - `CellData.features`: backed by the persisted `features_raw` column.
> - Loot/`death_markers`: read/written directly on `CellData.data`.
> - Status-effect expiry: owned by the durable `status_effects/` subsystem.

## Character Creation & Progression

- [x] **CC-03 Skill Point Modal:**
  - [x] Implement backend support for selecting skills and increasing proficiency during character creation. (Bulk skill assignment command added)

## Exploration & World

- [x] **EXP-01 Map Legend:** Toggleable overlay explaining terrain and markers.
- [x] **EXP-02 Time of Day:** Track elapsed time during travel. (1 tick = 1 hour travel, shown in status)
- [x] **EXP-03 Weather:** Random weather events.
- [x] **EXP-04 Foraging:** Gather food/materials from the wilderness. (Survive/WIS check, awards rations)
- [x] **EXP-05 Landmarks:** Define gameplay purpose and make interactive. (Examine support, specific types added)
- [x] **EXP-06 Lairs:** Define purpose (dungeon entry, encounter trigger). (Specific lair types added, enterable)
- [x] **EXP-07 Ruins Expansion:** Add searchable content, encounters, or loot. (Ruin discoveries added)

## Data & Tables

- [x] **DAT-01 Random Tables:** Support conventional roll values (e.g., d100 ranges).
- [x] **DAT-02 External Integration:** Map external book tables to the game engine. (Fate Chart added)
- [x] **DAT-03 Tag System:** Create mapping table/YAML defining tag logic.
- [x] **DAT-04 Unified Abilities:** Define attack types (bite, claws) in a dedicated effects table.
- [x] **DAT-05 Logic Correction:** Move innate abilities (bite, claws) from item tables to creature abilities.

## Desktop Application

- [x] **DSK-01 Standalone Executable:** Package the app as a desktop application.
  - [x] Serve frontend via FastAPI static files.
  - [x] Wrap application in PyWebView native window.
  - [x] Automate build process with PyInstaller.
- [x] **DSK-02 PyWebView Polish (robustness + cross-platform build):**
  - [x] Dynamic free-port selection (no hardcoded 8080) with graceful uvicorn
        shutdown on window close and a native error dialog on startup failure.
  - [x] Portable `HarshRealm.spec` (no machine-specific absolute paths; optional
        icon resolution) and a spec-driven `scripts/build_desktop.py`.
- [x] **DSK-03 Tauri desktop shell (scaffold):** Rust/Tauri shell that supervises
      the Python backend on a free port and loads the origin-relative frontend
      from it, with native IPC commands so engine logic can move to Rust over time.
  - [x] `crates/harsh-core` pure-Rust core (no GUI deps, unit-tested). First
        ported component: the XWN dice engine.
  - [x] `src-tauri/` Tauri v2 project (backend supervisor, IPC commands, window
        creation, remote-URL capability) + `useNativeCore` frontend bridge.
  - [x] Reproducible Docker build env (`docker/tauri-build.Dockerfile` +
        `scripts/build_tauri.sh`); shell compiles + links against Tauri 2.11.3 /
        webkit2gtk with no host changes. Icons generated via `tauri icon`.
  - [ ] Run the windowed app on real hardware (CI/dev box) and wire a packaged
        Python backend sidecar for distribution. See `src-tauri/README.md`.

## Bug Fixes

- [x] **FIX-01 Broken admin-tab templates:** Five admin tab components
      (`SkillMappingsTab`, `DifficultyTargetsTab`, `DispositionOutcomesTab`,
      `EncounterWeightsTab`, `FactionAssetStatsTab`) had an orphan `</div>` that
      broke `vite build` (and thus both desktop bundles). `vue-tsc` did not catch
      it; the production Vue compiler does. Removed the stray tags.

## Future / Nice-to-Have

- [ ] **NICE-01:** Add more creatures.
- [ ] **NICE-02:** Random weather system.
- [ ] **NICE-03:** Bard/druid class (animal communication).
- [ ] **NICE-04:** Expanded equipment kits.

## Rust Core Migration

> Goal: make Rust (`crates/harsh-core`) the authoritative game engine, keep
> Python as a prototyping + plugin layer, formalize the component vocabulary into
> one versioned IR, and add plain-language authoring (NL → validated IR records).
> Full design: [docs/design/rust-core-migration-plan.md](docs/design/rust-core-migration-plan.md).
> Engine is pure and returns **intents**; Python applies durable writes (for now).
> Each phase ships independently and keeps the app working.

### Phase R0 — Foundations ✅

- [x] **R0-01 Branch + CI:** `feat/rust-core-migration` established; CI workflow
      `.github/workflows/rust-core.yml` runs fmt + clippy (`-D warnings`) + tests
      + the schema drift gate, plus a parity job that builds the PyO3 wrapper via
      maturin and runs the harness.
- [x] **R0-02 IR crate skeleton:** `crates/harsh-core/src/ir/` with `serde` +
      `schemars` types: Condition AST (`Expr`), `Effect`/`Trigger`, `Modifier`,
      `Trait`, `Tag`, `StatusEffect`, `Procedure`, `Table`, and the
      `ComponentRecord` envelope. Carries `SCHEMA_VERSION`. Field names/defaults
      mirror the Python schemas. (Entity/Event payloads deferred to their phases.)
- [x] **R0-03 JSON Schema export:** `export-schema` binary writes
      `crates/harsh-core/schema/harsh-ir.schema.json`; the
      `committed_schema_matches_export` test is the CI drift gate.
- [x] **R0-04 PyO3 wrapper crate:** `crates/harsh-core-py` (maturin); Python
      `import harsh_core` exposes `attr_modifier`, `ir_schema_json`,
      `validate_record`, `roll_seeded`, `SCHEMA_VERSION`. Verified round-trip.
- [x] **R0-05 Parity harness:** `tests/parity/` — exact `attr_modifier` parity
      across the full range, and Rust-vs-Pydantic structural validation agreement
      for IR records. Skips cleanly when the extension isn't built. (39 checks.)
- [x] **R0-06 Intent protocol:** `crates/harsh-core/src/intent.rs` — typed
      `Intent` enum (internally tagged) covering the effect verbs; documented as
      the stable contract (Python applies now, Rust later). No behaviour change.

> R0 verified: 31 Rust tests + 39 parity checks green; both crates fmt/clippy
> clean. Next: **R1** (port the DSL parser/evaluator + trigger dispatch to Rust
> against the parity harness).

### Phase R1 — Engine core (first slice) ✅

- [x] **R1-01 Port DSL AST + parser:** `crates/harsh-core/src/dsl/parser.rs` —
      tokenizer + recursive-descent parser producing `ir::Expr`. Reproduces the
      Python tokenizer's leftmost-first quirks so ASTs match byte-for-byte.
- [x] **R1-02 Port evaluator + path resolver:** `dsl/eval.rs` — pure evaluator
      over a caller-supplied `EvalContext` (host materializes entity state; engine
      does no I/O). All roots, whitelisted funcs, Python-matching numeric/`in`/
      comparison semantics.
- [x] **R1-03 Condition parity gate:** `tests/parity/test_dsl_parser_parity.py`
      — Python-vs-Rust over a hand corpus (valid + invalid) **and** a Hypothesis
      property test; "both reject" counts as agreement. Rust is the reference impl.
- [x] **R1-04 Trigger matching:** `dispatch.rs` `dispatch()` selects triggers by
      `on` + `when` (evaluated), fail-closed on errors. Selection only, no writes.
- [x] **R1-05 Effect → intent lowering:** `dispatch.rs` `lower_effect()` maps the
      state-change verbs (`change_resource`, `apply_status`, `remove_status`,
      `apply_modifier`, `remove_modifier`, `emit_event`, `log`) to typed `Intent`s;
      `roll_dice`/`run_procedure` error explicitly (need the procedure runner — R4).
- [x] **R1-06 Wire through PyO3 behind a flag:** `harsh_core.parse_condition_json`
      exposed; `src/harsh_realm/dsl/rust_backend.py` routes parsing through Rust
      when `HARSH_REALM_RULES_BACKEND=rust` (default `python`), with fallback.
      Tested in `tests/parity/test_rust_backend_flag.py`.

> R1 verified: 52 Rust tests + 95 parity checks green; both crates fmt/clippy
> clean. Note: cross-language *evaluator* parity (vs Python's async, service-
> backed evaluator) is deferred to R2/R3 when entity materialization lands — the
> Rust evaluator has full unit coverage; the parser is the proven cross-language
> gate. Next: **R2** (pure resolvers — skill checks, saves, advancement, damage).

### Phase R2 — Pure resolvers ✅

All resolvers live in `crates/harsh-core/src/resolvers/`. Dice rolls stay with
the host, so each is deterministic and exactly parity-testable.

- [x] **R2-01 Skill checks → Rust** (`skill_check.rs`): `classify_margin`,
      `resolve_skill_check`, `deceive_failure_delta`. Exact `classify_margin`
      parity + invariant grid test.
- [x] **R2-02 Saving throws → Rust** (`saves.rs`): `resolve_save`,
      `save_stat_key`. Invariant grid test (pass iff total ≥ target).
- [x] **R2-03 Advancement / XP / level → Rust** (`advancement.rs`):
      `attack_bonus_for_class`, `xp_for_level`/`DEFAULT_XP_TABLE`,
      `hit_die_for_class`, level-up helpers. Exact parity vs Python pure fns.
- [x] **R2-04 Damage + combat math → Rust** (`combat.rs`): `parse_damage_expr`,
      `resolve_hit`, `damage_total`, `resolve_shock`. Exact parity for
      `parse_damage_expr` (valid + invalid) and `resolve_shock`; hit/damage
      invariant grids.
- [x] **R2-05 Flip resolvers behind the flag:** `rust_backend.py` wrappers
      (`classify_margin`, `attack_bonus_for_class`, `parse_damage_expr`) route to
      Rust when `HARSH_REALM_RULES_BACKEND=rust` (default python). Tested both ways.

> R2 verified: 71 Rust tests + 195 parity checks green; both crates fmt/clippy
> clean; full Python suite collects (1661). Exact cross-language parity holds for
> the importable pure Python fns; RNG-driven entry points use Rust invariant grids
> (cross-language RNG isn't byte-equal). Next: **R3** (component vocabulary online
> — traits/tags/modifiers/status evaluation in Rust, returning intents).

### Phase R3 — Component vocabulary online ✅

In `crates/harsh-core/src/components/`. Evaluation only; durable writes stay with
the host.

- [x] **R3-01 Traits + tags evaluation in Rust** (`tags.rs`):
      `resolve_effective_tags` (static ∪ trait-provided ∪ status-provided union,
      matching the Python service — no `implies` processing). Trait/tag/status
      *queries* used by conditions are handled by the R1 evaluator + this union.
- [x] **R3-02 Modifiers pipeline in Rust** (`modifiers.rs`): `evaluate_condition`,
      `collect`, `resolve_final_value` (replace→additive→multiplicative→max/min,
      float order preserved). Exact parity vs `_resolve_final_value` and
      `evaluate_condition`.
- [x] **R3-03 Status-effect evaluation in Rust** (`status.rs`): `expiry_tick`,
      `extended_expiry`, `is_due`, `expirations` → `RemoveStatus` intents. Exact
      parity vs `_extended_expiry`.
- [x] **R3-04 IR coverage:** extended `Procedure` (description/output/tags) +
      added `ProcedureOutput`; real status-effect & procedure pack YAML validate
      and round-trip through Rust `validate_record`
      (`tests/parity/test_ir_pack_roundtrip.py`). Schema regenerated.
- [x] **Evaluator parity (R1 deferral closed):** `harsh_core.evaluate_json`
      exposes the pure evaluator; `tests/parity/test_dsl_evaluator_parity.py`
      compares Python vs Rust over the service-free subset (corpus + Hypothesis,
      agreeing on values *and* errors). Entity-backed paths / `has_*` await
      materialized entity state (later phase).

> R3 verified: 82 Rust tests + 275 parity checks green; both crates fmt/clippy
> clean; full Python suite collects (1741). Next: **R4** (tables & oracle RNG —
> table roll paths, fate-chart math, procedure roll/compute steps).

### Phase R4 — Tables & oracle RNG ✅

- [x] **R4-01 Table engine roll paths → Rust** (`tables.rs`): `select_by_range`
      (ranged/d100), `weighted_select` (cumulative + `bisect_right`, matching
      `random.choices`), `max_range`. RNG stays in host; selection-given-a-roll is
      exact. Parity: `select_by_range` vs `_roll_on_table`; `weighted_select` vs
      stdlib bisect. (Subtable refs are recursion in the host's `roll_on`, which
      drives the Rust selection.)
- [x] **R4-02 Oracle fate-chart / scene-check math → Rust** (`oracle.rs`):
      `classify_fate`, `classify_scene`, `roll_on_ranged_table`. Parity runs the
      real `FateChecker`/`SceneChecker` with a seeded RNG and feeds the actual roll
      to the Rust classifier across every likelihood × chaos × seeds.
- [x] **R4-03 Procedure roll-step selection → Rust:** roll steps now select via
      the Rust table functions. `compute` steps (Python plugin registry) and
      `format` steps (templates) stay Python by design (see R7); the runner's
      orchestration stays Python. Flag-gated wrappers in `rust_backend.py`
      (`classify_fate`, `classify_scene`, `select_by_range`), tested both ways.

> R4 verified: 90 Rust tests + 291 parity checks green; both crates fmt/clippy
> clean; full Python suite collects (1757). Next: **R5** (plain-language authoring
> — NL→IR compiler via Claude API, schema-validated, in-app panel + upload/download).

### Phase R5 — Content system (deterministic grammar → IR + action/resolution primitives)

> **Reframed** from the original LLM authoring idea to a hand-designed YAML grammar
> + deterministic Rust compiler, **plus** a system-agnostic action/skill/resolution
> primitive engine (the big new piece). Full design = four committed docs in
> `docs/design/` (see migration-plan §11) and the decisions/open-questions there.
> R5 is now the largest phase; sub-phases below are dependency-ordered.
> **Gate:** resolve the §11 residual open questions before starting R5-P.

#### R5-P — Resolution primitives (engine/code)

- [x] **R5-P1 Phased resolver (pure-core building blocks):** `Stage` sequence +
      event-type contract (`resolution/stage.rs`); `ContestRequest`/`resolve_contest`,
      reaction application (`SetTn`/`Cancel`), and `outcome_key` (`resolution/contest.rs`).
      The host's event-loop *orchestration* of these stages is R5-W2.
- [~] **R5-P2 Resolution kinds:** ✅ `ResolutionResult` margin currency
      (success/raw_margin/degree/crit/fumble) landed (`resolution/result.rs`).
      Pending: `contest` (parameterized) + `apply` + `reaction`-on-`reaction_window`
      orchestration (waits on R5-P1).
- [x] **R5-P3 RollMechanic registry:** `xwn_2d6`, `xwn_d20_attack`, `xwn_d20_save`
      (`resolution/mechanic.rs`) — the R2 resolvers re-expressed to one result
      shape, with degree banding; in-crate parity vs R2 (transitive Python parity).
      GURPS/Savage designed-for, not built.
- [x] **R5-P4 Damage/health pipeline** (`resolution/damage.rs`): `DamagePacket`
      → wounding (per-type multipliers) → mitigation (subtractive DR, threshold
      armor) → tier routing (MD bypasses SD) → typed `Pool`s (clamp/`can_go_negative`/
      depletion). Tested for XWN HP, GURPS wounding+DR, Savage threshold, Rifts
      MD-bypass — AC is one config. (Absorbing-pool soak deferred.)
- [x] **R5-P5 Targeting + grid range + action-economy:** `targeting.rs`
      (`TargetShape`, `Targeting`, **Chebyshev grid distance** + range-band
      membership using the SquareGrid metric — grid wired from the start per the
      decision); `economy.rs` (`Economy`/`Cost`/`Uses`/`Activation` + `can_afford`).
      Per-round budget tracking is host-side runtime state.

> **R5-P complete** (resolution primitives engine): 121 Rust tests, clippy clean.
> P1 stages/contest/reactions · P2 margin currency · P3 mechanics (parity vs R2) ·
> P4 damage pipeline (AC=one config) · P5 targeting/grid/economy · P6 new intents +
> DSL `has_action`/`has_skill`. Next: **R5-A** (action/skill IR + XWN pack) and the
> `unarmed_strike` + `parry` vertical slice.
- [x] **R5-P6 New effect verbs / intents:** `Intent::{EmitDamage, SpawnEntity,
      Reposition}` + `lower_effect` lowering; `run_action`/`set_event` rejected as
      resolution control-flow (not intents, per decision). DSL gains `has_action`/
      `has_skill` (`EntityView` now carries `actions`/`skills`). 116 Rust + 291
      parity green.

#### R5-A — Action/skill IR + XWN content

- [x] **R5-A1 IR record types:** `ir/action.rs` — `Action` (L1 archetype + L2 via
      `base:`; `kind`/`resolution`/`tn_source`/`roll_spec`/`outcome`/`targeting`/
      `activation`/`prerequisites`/`modifiers`) and `Skill` (L3 overlay: `governs`/
      `grants`/`triggers`). Added to `ComponentRecord` (schema → 0.2.0, regenerated);
      reuses the runtime `Targeting`/`Activation`/`RollMechanic`/`Roller`. Validates
      through Rust + PyO3.
- [x] **Vertical slice** (`tests/vertical_slice.rs`): `unarmed_strike` + `parry`
      driven end-to-end through R5-P (contest → outcome → emit_damage → damage
      pipeline → pool delta; parry `SetTn` turns a hit into a miss). Proves R5-P +
      R5-A compose; stands in for the host event loop (R5-W).
- [x] **R5-A2 Entity IR types** (`ir/entity.rs`): `Creature` + `Item` mirror the
      Python models (defaults preserved) and join `ComponentRecord` (schema 0.2.0).
      Creature gains `actions`/`pools`/`defenses`, retires `attack_skill`, folds
      `innate_attacks` into `actions`; Item gains `grants_traits` (C2). `npc`/
      `character` deferred (C1). Validate through Rust + PyO3.
- [ ] **R5-A3 XWN core pack:** the action-archetype set + the 19 skills as L3
      overlays (Judo as a pure-data smoke test). **Rides on R5-C** — authored in the
      grammar, not hand-written as raw IR.

#### R5-W — Make it live (A1–A4)

- [~] **R5-W1 Modifier consumption (A1):**
      - ✅ Enabling layer: the five resolvers accept `modifier_total`/`attack_modifier`/
        `damage_modifier` (skill_checks.resolve, saves.resolve_save,
        combat/resolvers.py AttackResolver+DamageResolver), folded into the total
        (default 0 = no behaviour change).
      - ✅ The modifier system is now LIVE: `ModifierService` is instantiated for the
        first time and exposed as `GMController.modifier_resolver` (a
        `modifiers/resolver.py` `ModifierResolver` that gathers tags/traits and
        aggregates via the Rust pipeline). Confirmed in real init ("Modifier resolver
        ready"). Tested: `tests/engine/test_modifier_consumption.py`.
      - ⬜ **Remaining (call-site consumption):** thread `modifier_resolver` into the
        service-free scenes so each resolver call passes a real total — `combat.ac`
        (bake on the Combatant at `create_combat`), `combat.attack`/`combat.damage`
        (combat scene), `skill.<id>` (social), `save.<type>`. Needs the resolver
        plumbed into the scene layer; mid-combat dynamic re-resolution is a sub-gap.
        Also: a modifier-target registry / `action.<id>` stage-scoping.
- [x] **R5-W2 Declarative trigger wiring (A2):** ✅ machinery — PyO3
      `dispatch_triggers(triggers, event, context)` (EvalContext/EntityView now
      serde-(de)serializable); `triggers/runner.py` (`gather_triggers` →
      `evaluate_triggers` → `apply_intents` via an `IntentSink`, sink-decoupled +
      tested in isolation, `tests/triggers/test_runner.py`).
      - ✅ `triggers/sinks.py` `ServiceIntentSink` (change_resource→ResourceService,
        apply/remove_status→StatusEffectService, emit_event→callback, log→logger).
      - ✅ `triggers/materialize.py` (`entity_view`/`build_context` pure builders +
        `materialize_entity` service-backed gatherer; resources → short-named fields
        so `entity.hp` resolves). Composed end-to-end in `test_wiring.py`.
      - ✅ `triggers/runtime.py` `TriggerRuntime.fire()` orchestrator + `as_id`
        coercion (services return `Trait` objects, not id strings).
      - ✅ `combat.attack` payload carries `attacker_id`/`target_id`;
        `triggers/handler.py` `CombatTriggerHandler` fires the attacker's on-hit
        triggers; `GMController._wire_trigger_runner` assembles the services
        (trait/tag/resource + content getter + service sink) and registers the
        handler — confirmed wired in real world init (logs "wired to combat.attack").
      - ⬜ Follow-ups: defender reactive triggers (need a distinct event); route
        `emit_damage`/`spawn`/`reposition`/modifier intents in the sink; R5-A3
        content with actual on-hit triggers to exercise it live.
- [x] **R5-W3 Effect entity roles (A3):** `dispatch.rs` `entity_id()` resolves the
      role tokens `self` (→ ctx.self_id) and `target` (→ ctx.target_id, fail-closed
      if absent); any other value is a literal id; absent defaults to `self`. The
      compiled IR carries the role token; dispatch resolves it at lowering time.
- [~] **R5-W4 Event-payload catalog (A4):** ✅ compiler validation —
      `Catalogs.event_fields` (event_type → allowed payload fields); `references.rs`
      walks each trigger's `when:` AST, extracts `event.<field>` reads, and flags any
      the event doesn't carry (syntax error). Opt-in per event. Tested in Rust +
      via `compile_content` (`test_event_field_validation_via_catalog`).
      ⬜ Remaining: populate `event_fields` from the live event/payload registry
      (CombatAttackNotice etc.) and pass it through the CLI — same pending wiring as
      the other catalogs.

#### R5-C — Grammar compiler (Rust) + CLI

- [x] **R5-C1 Compiler core** (`crates/harsh-core/src/compiler/`): form-2 verb
      registry (`verbs.rs`), record dispatch + `do:`-list lowering + structural
      validation + two-bucket diagnostics with atomic fail (`compile.rs`), semantic
      validation of dice + `when:` conditions (`validate.rs`), and cross-reference
      resolution against opt-in `Catalogs` (`references.rs` — events/resources →
      unimplemented; statuses/actions → syntax, per B1). Host parses YAML→JSON (no
      YAML dep in harsh-core). **Deferred:** inline-record lifting (§4) — sugar;
      reference-by-ID is the primary form, addable later without grammar changes.
- [x] **R5-C2 CLI + bridge:** PyO3 `harsh_core.compile_content(docs, catalogs?)` →
      CompileReport; `hrctl content compile <dir> [--out]` (`harsh_realm/content/`:
      canonical `ContentLoader` that doesn't booleanize `on/off/yes/no`, file
      walking, two-bucket report, exit-non-zero on diagnostics). Smoke-tested
      end-to-end. Pending: wiring real pack catalogs into the CLI, storing IR into
      a world/pack, optional `--draft`.

### Phase R5-R — Rust resolver liveness + remaining conversions (Tiers 1–3)

> Audit (2026-06-20): the dividing line is Rust owns deterministic computation
> (returns intents), Python owns I/O + orchestration + (for now) persistence.
> What remains to convert, by tier:

**Tier 2 — wire the EXISTING Rust resolvers into live play (highest leverage).**
The Rust resolvers exist and are parity-tested, but the live scenes still call the
Python math; two implementations coexist. The Rust resolvers are deterministic
*given the roll*, so Python keeps rolling the dice (RNG/save-compat preserved) and
delegates the arithmetic to Rust. Fold the R5-W1 modifier totals into the inputs.

- [x] **R5-R1 Expose the full resolvers via PyO3:** `resolve_save`,
      `resolve_skill_check`, `resolve_hit`, `damage_total` — return the `*Outcome`
      structs as JSON (`harsh-core-py/src/lib.rs`).
- [x] **R5-R2 Wire `engine/saves.py`** → `harsh_core.resolve_save` (Python rolls the
      d20; modifier_total folds into `bonus`). 22 save tests green.
- [x] **R5-R3 Wire `engine/skill_checks.py`** → `harsh_core.resolve_skill_check`
      (Python rolls 2d6; modifier_total folds into attr_mod; disposition mapping
      stays Python). 57 skill/social tests green.
- [x] **R5-R4 Wire `engine/combat/resolvers.py`** → `harsh_core.resolve_hit` +
      `harsh_core.damage_total` (Python rolls; modifiers fold into attr_mod).
      146 combat tests green.
- [x] **R5-R5 Retire the Python math** (saves/skill/combat): the three resolver
      bodies are now thin adapters — gather inputs, roll, call Rust, build the
      result; no duplicate arithmetic remains. Parity tests stay as regression
      guards. (Advancement still uses the per-helper PyO3 fns; audit later for any
      leftover inline math.) Full suite: 1800 passed (the lone failure,
      test_combat_victory_flow, is pre-existing unseeded-RNG flakiness — passes 5/5
      in isolation; the RNG sequence is unchanged by the wiring.)

**Tier 1 — pure logic still in Python, port to Rust (self-contained).**
- [~] **R5-R6 Faction turn AI → Rust:** the priority-based decision
      (`faction_ai.py`) is now `resolvers/faction_ai.rs` `choose_action` — pure,
      with Python-matching tie-breaks (min/max first-wins). Python gathers state
      (own + alliance-filtered enemy assets + asset-stat catalog w/ parsed specials),
      calls `harsh_core.choose_faction_action`, rebuilds the typed choice. 49 faction
      tests green. **Remaining:** the action *execution* (`faction_turn.py`
      `_action_*`) stays host — it's DB writes + RNG attack rolls + trivial deltas;
      only port its math to Rust if a parity need arises.
- [~] **R5-R7 `engine/enemy_ai.py`** combat AI decisions → Rust. **DEFERRED:** the
      current `EnemyAI.choose_action` is a placeholder (always "attack the player"),
      no real decision logic to port. Revisit when behaviours differentiate.
- [x] **R5-R8 `engine/character_recalc.py`** → Rust (`resolvers/character.rs`
      `recalculate`: attr mods, max HP, AC from equipment + heavy-armor rule, attack
      bonus, saves). PyO3 `recalculate_character(json)`; the Python class is now a
      thin marshaler. 128 recalc/character/editor/property tests green; clippy clean.
- [x] **R5-R9 Mythic adjacencies — break down, do NOT port to Rust.** Per the
      directive (no system-specific rules/components in the engine): porting the
      Mythic adventure-crafter into Rust would bake a system-specific component into
      the generic engine — wrong. `threads.py` is generic DB CRUD (host); the
      adventure-crafter is host orchestration (DB + random selection over Mythic
      *content* tables). The deterministic Mythic math (fate chart, scene-check,
      chaos) is already generic Rust (`oracle.rs`, R4). **Action taken:** broke the
      one hardcoded system-specific rule — the scene narration template — out of
      Python into content (`narration_template` on the AC themes document /
      `ac_themes.yaml`); the crafter now formats from data. 140 tests green.
      **Optional full breakdown (deferred):** express the whole scene-gen as a
      data-driven `ac_scene` procedure (roll/format primitives) — bigger (RNG-path
      change + ProcedureRunner wiring); not needed for correctness.
- [x] **R5-R10 Small combat/exploration math** → Rust (`resolvers/scene.rs`):
      `resolve_first_aid` (healing.py; probe-then-resolve so the heal die only rolls
      on success), `resolve_awareness` (awareness.py; the 3-way surprise
      classification), `resolve_flee_check` (flee.py). 91 heal/awareness/flee/combat
      tests green. **Not ported:** `encounters.py`/`loot.py` are RNG + table
      selection — the selection primitive is already Rust (`weighted_select`);
      rewiring their inline weighting to it is optional cleanup, not a math port.

**Tier 3 — procedural generators (convert with the RNG caveat).**
- [x] **R5-R11 Generators — assessed; DO NOT port (directive + RNG caveat).**
      `generators/` (~1700 lines) are RNG-driven host *orchestration*: they (a) use
      generic primitives that are ALREADY in Rust — weighted selection
      (`weighted_select`), grid Chebyshev distance/neighbors (`targeting.rs` + grid
      model) — and (b) carry system-specific *content* (terrain weights, room types,
      settlement data). Porting would change generated worlds (cross-language RNG ≠
      byte-equal) AND risk baking system-specific generation rules into the generic
      engine. So they stay host. **Directive follow-up (separate, lower priority):**
      extract inline system-specific content baked in code into content tables.
      - [x] **Class progression → content** (commit 1a336fc): classes.yaml
        `attack_bonus_by_level`/`skill_points_per_level`; `class_progression.py`
        loader; advancement + recalc read content; Rust class formulas demoted to
        parity refs (`tests/test_class_progression.py` verifies content==formula).
      - [x] **Option-list content → tables** via a shared loader
        (`generators/content_tables.py` `load_table_results`): dungeon room types
        + dungeon trap types (`dungeon_gen.py`) and settlement sizes
        (`settlement_sizes.yaml`, replacing `_SETTLEMENT_SIZES`). Terrain weights +
        settlement/ruin/landmark names were ALREADY content. Same order → generation
        unchanged.
      - [ ] **Optional remaining** (generation *flavor/tuning*, lower value, more
        embedded): dungeon entrance/room description strings, the hardcoded loot
        item, trap stats (notice/avoid/damage) + spawn probabilities. These are
        generation defaults more than option lists; extract into a dungeon config if
        desired.

**Stays Python by design (not conversion targets):** FastAPI/WS/REST, GM controller
+ scenes (orchestration), pack loading/registry, DB + Pydantic models, the
service/repo persistence layers (until R6), bot, admin, command parser.

### Phase R6 — Intent-writer hardening + rusqlite spike

- [ ] **R6-01 Formalize the intent application layer** in Python (single place
      that turns intents into SQLite writes via repositories).
- [ ] **R6-02 rusqlite spike:** prototype Rust-owned writes for one subsystem;
      measure and validate the eventual transition. Produce a go/no-go.

### Phase R7 — Plugin surface

- [ ] **R7-01 Plugin contract:** 
- [ ] **R7-02 Sandbox + lifecycle:** process management, timeouts, error
      surfacing; a reference example plugin + docs.