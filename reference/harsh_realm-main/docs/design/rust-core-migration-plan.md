# Rust Core Migration & Plain-Language Authoring — Design Plan

> Status: **Planning** (branch `feat/rust-core-migration`, off `main`).
> Companion task checklist lives in `todo.md` under "Rust Core Migration".
> This is a deliberate, multi-phase effort. It is developed on its own branch and
> PR'd / merged to `main` only when each phase is reviewed and accepted.

## 1. Vision

Move the **authoritative game engine into Rust** while keeping Python as a
**prototyping and plugin layer**, and grow a **vocabulary of declarative,
DSL-like components** flexible enough that a user can describe new rules and
objects in *almost plain language* and have the system populate the
corresponding logic and tables.

Three goals, in priority order:

1. **Rust core of record.** The condition language, the trigger/effect
   dispatcher, and the pure rules resolvers (dice, checks, saves, combat math,
   table/oracle rolls) become Rust, with behavioural parity against the current
   Python engine proven by tests.
2. **Maximum-flexibility component vocabulary.** Formalize the existing
   `dsl` / `triggers` / `effects` / `traits` / `tags` / `modifiers` /
   `status_effects` / `procedures` / `payloads` concepts into one **versioned
   intermediate representation (IR)** that is the single source of truth shared
   by the Rust engine, the Python models, and the authoring layer.
3. **Data-described content (a deterministic authoring grammar).** A YAML
   authoring surface compiles — deterministically, no LLM — to validated IR
   records, so monsters, items, feats, status effects, tables, and (the big one)
   **a system-agnostic action/skill/resolution vocabulary** are all authored as
   data. New mechanics are data until they name an engine primitive that doesn't
   exist; the compiler then names exactly which primitive to build.
   > **Superseded:** the original goal #3 was LLM-based natural-language
   > authoring. That was dropped in favour of a hand-designed grammar +
   > deterministic compiler (offline, version-controllable, exactly testable).
   > The full design is the four-doc R5 content-system set — see §11.

### Non-goals (explicitly out of scope, at least initially)

- Rewriting FastAPI / WebSocket / HTTP transport in Rust.
- Moving durable persistence (per-world SQLite) into Rust **now** — see §5.
- macOS desktop support (Windows + Linux only).
- Replacing Python wholesale. Python is a **kept, first-class** prototyping and
  plugin surface.

## 2. Guiding principles

- **Engine is pure; the world is written elsewhere.** `harsh-core` computes and
  returns **intents**; it performs no I/O and mutates no durable state. This is
  what makes it testable, embeddable (PyO3), and portable (Tauri).
- **One IR, three consumers.** The component schema is defined once and consumed
  by (a) the Rust engine, (b) Python (Pydantic), (c) the content-grammar compiler
  (validates emitted records against the exported JSON Schema). No hand-maintained
  parallel definitions that can drift.
- **Parity before replacement.** Every ported resolver ships with golden parity
  tests comparing Rust output to the current Python output for the same inputs,
  plus property tests for invariants. Nothing is "migrated" until parity holds.
- **Incremental and always-shippable.** At every step the app runs. Python keeps
  working; Rust takes over one well-scoped component at a time behind a flag.
- **Stable contracts, movable implementations.** The IR and the intent protocol
  are the durable contracts. Where the *writer* lives (Python now, Rust later)
  can change without touching the engine or the authoring layer.

## 3. Current state (what already exists in Python)

The "DSL-like vocabulary" is largely built; this migration formalizes and ports
it rather than inventing it.

| Package | Role | LoC (approx) |
|---|---|---|
| `dsl/` (`ast`, `parser`, `evaluator`, `context`) | Condition expression language: frozen-Pydantic AST, dotted path refs (`entity`/`event`/`world`/`target`/`self`/`local`), binary/unary ops, whitelisted funcs (`has_tag`, `has_trait`, `has_status`, `len`, `min`, `max`, `abs`) | ~720 |
| `triggers/` (`schema`, `effects`, `dispatcher`) | Declarative `{on: event, when: condition, do: [effects]}` subscriptions | ~160 |
| `effects` verbs | `apply_modifier`, `remove_modifier`, `change_resource`, `apply_status`, `remove_status`, `emit_event`, `roll_dice`, `run_procedure`, `log` | — |
| `traits/`, `tags/`, `modifiers/`, `status_effects/` | Data-driven entity component layer | ~850 |
| `procedures/` | Multi-step generators (`roll`/`compute`/`procedure`/`format`) with pack overrides | ~510 |
| `payloads/` | Typed event payloads (combat/world notices, requests, transport) | ~1050 |
| `engine/` resolvers | `dice` (✅ ported), `skill_checks`, `saves`, `advancement`, `damage`, combat, `tables`, `oracle`, … | — |

Already done on the desktop branch: `crates/harsh-core` (pure Rust) with the
dice engine ported and tested; the Tauri shell links it.

## 4. Target architecture

### 4.1 Crate / module topology

```
crates/harsh-core/         Pure Rust. No I/O. The engine of record.
  ir/                      The component vocabulary (IR) types + JSON Schema export.
  dsl/                     Condition AST + parser + evaluator + path resolver.
  dispatch/                Trigger matching + effect → intent lowering.
  resolvers/               dice (done), checks, saves, advancement, damage, tables, oracle RNG.
  intent.rs                The typed Intent enum returned to callers.

crates/harsh-core-py/      Thin PyO3 wrapper exposing harsh-core to Python (built via maturin).
src-tauri/                 Tauri desktop shell; links harsh-core directly for native IPC commands.

src/harsh_realm/ (Python)  FastAPI + WebSocket host; persistence (applies intents → SQLite);
                           prototyping sandbox; plugin host (subprocess/IPC); authoring compiler.
```

**Why `src-tauri/` stays at the repo root (not under `crates/`):** `crates/`
holds reusable *library* crates (`harsh-core`, `harsh-core-py`); `src-tauri` is
the desktop *application shell* (a binary + GUI), a different kind of artifact.
`src-tauri` is also the canonical, tooling-recognized directory name — the Tauri
CLI (`tauri dev/build/icon`) and `tauri.conf.json`'s root-relative paths
(`../frontend/dist`, `npm --prefix ../frontend`) assume root-level placement
adjacent to the frontend. The shell's GUI build is deliberately isolated from
the pure-core build/test loop: it has its own `Cargo.lock` (no shared
workspace), and CI (`.github/workflows/rust-core.yml`) builds only `crates/**`,
excluding the webkit2gtk-dependent shell. Moving it would churn the dep path,
config, build scripts, and docs for no functional gain. Decision: leave it.

**How Python calls Rust:** in-process via the `harsh-core-py` PyO3 extension —
*not* subprocess IPC. The engine path must be fast and synchronous. Subprocess /
IPC is reserved for **plugins** and **prototypes** (untrusted or rapidly-iterated
code that should not be linked into the core).

### 4.2 The intent boundary (the key seam)

`harsh-core` never writes. Evaluating a trigger produces a list of **intents** —
typed, serializable descriptions of *what should happen* (e.g.
`ChangeResource{entity, resource, delta}`, `ApplyStatus{entity, status, …}`,
`EmitEvent{type, payload}`). The host decides how to apply them.

```
event ──▶ harsh-core: match triggers, eval conditions, lower effects ──▶ [Intent]
                                                                            │
                          Python persistence layer applies intents ────────┘  (now)
                          Rust persistence layer applies intents  ──────────┘  (later, §5)
```

Decision (locked): **start with Python applying intents** (one writer, lowest
risk). The intent protocol is designed as the stable contract so the writer can
move into Rust later without changing the engine or authoring layer.

### 4.3 The IR as single source of truth

Define each component type **once** and generate the rest:

- **Define in Rust** (`harsh-core/ir`) with `serde` + `schemars` derive.
- **Export JSON Schema** from the Rust types (build step / CLI).
- **Python Pydantic models** are validated against — and ideally generated from —
  that JSON Schema, so Python and Rust cannot silently diverge.
- **The LLM authoring target** is the same JSON Schema (the model emits records
  that validate against it).

Component types to formalize into the IR (superset of what exists today):
`Entity`, `Component`/`Trait`, `Tag`, `Modifier`, `StatusEffect`, `Trigger`,
`Effect` (verb + params), `Condition` (expr), `Procedure` (steps), `Table`,
`Event`/`Payload`. Each is versioned; the IR carries a `schema_version`.

### 4.4 Content authoring pipeline (deterministic grammar — no LLM)

```
YAML content (authoring grammar)  ─▶  Rust compiler  ─▶  IR records  ─▶  stored (canonical)
content inspector (GUI) edits     ─┘     (validated against the exported JSON Schema)
```

- A hand-designed YAML grammar compiles, **deterministically**, to IR records.
  No LLM at compile time or run time. The YAML is an import surface; the IR is the
  source of truth and is discarded-from after a successful compile.
- The compiler is a pure function of `(documents + loaded pack catalogs)`; it
  collects all diagnostics, splits them into `SyntaxError` (malformed / broken
  internal reference) vs `UnimplementedPrimitive` (well-formed reference to an
  engine primitive that doesn't exist yet), and fails atomically — the
  unimplemented list is the precise punch-list of engine work to do.
- The grammar targets not just the existing rules records but a new, system-
  agnostic **action / skill / resolution vocabulary** (so all skill use and combat
  are data; AC, SDC/MDC, GURPS, Savage are configurations of one damage pipeline).
- Full design: the four-doc R5 content-system set (§11).

## 5. Coexistence & the eventual transition

| Concern | Now (R1–R5) | Later (R6+) |
|---|---|---|
| Condition eval, trigger dispatch, resolvers | **Rust** (`harsh-core`) | Rust |
| Effect application / durable writes | **Python** (applies intents → SQLite) | Rust (rusqlite) — staged, subsystem by subsystem |
| FastAPI / WebSocket / REST | Python | Python |
| Prototyping & plugins | Python (in-proc + subprocess/IPC) | Python |
| Content compiler (YAML→IR) | **Rust** (`harsh-core`), driven by a Python CLI | Rust |

The transition to Rust-owned writes is **deferred, not designed out**: because
the engine already speaks in intents, flipping a subsystem's writer from Python
to Rust is a localized change behind the same contract.

## 6. Testing & parity strategy

Per CLAUDE.md's four-layer rule, adapted across the language boundary:

- **Parity harness.** A shared corpus of `(input, expected)` cases. Run the
  Python engine and the Rust engine over identical inputs; assert byte-equal
  (or structurally-equal) results. This is the gate for declaring a resolver
  "migrated". Seed RNG deterministically on both sides.
- **Rust:** unit + property tests (`proptest`) in `harsh-core`; `cargo test` in CI.
- **Python:** existing pytest/hypothesis suites stay green; add tests for the
  PyO3 boundary and the intent-application layer.
- **Authoring:** schema-validation tests (every generated record validates),
  golden tests for representative NL→IR translations, and a "round-trip"
  property (record → file → record is identity).
- **Frontend/E2E:** Playwright for the authoring panel and upload/download.

## 7. Phased roadmap

Phases map 1:1 to the `todo.md` checklist. Each phase ends in a reviewable PR to
`main` (or an accumulation branch) and leaves the app shippable.

- **R0 — Foundations.** Branch, CI for Rust + maturin, the IR crate skeleton with
  JSON Schema export, the parity-harness scaffold, and the intent-protocol design
  doc. No behaviour change.
- **R1 — Engine core (FIRST SLICE).** Port the DSL: AST, parser, evaluator, path
  resolver → Rust, with a full parity corpus vs. Python. Port trigger matching +
  effect→intent lowering. Expose via `harsh-core-py`; Python can evaluate
  conditions through Rust behind a flag.
- **R2 — Pure resolvers.** `skill_checks`, `saves`, `advancement`, `damage` /
  combat math → Rust (dice already done), each with parity + property tests.
- **R3 — Component vocabulary online.** Traits, tags, modifiers, status-effect
  *evaluation* in Rust (writes still via intents). IR covers all component types.
- **R4 — Tables & oracle RNG.** Port the deterministic roll/compute paths
  (`tables`, oracle fate-chart math, procedure `roll`/`compute` steps).
- **R5 — Content system** (deterministic grammar → IR compiler + the action/
  resolution primitive engine). This grew into the largest phase; full design in
  §11. Sub-phases, dependency-ordered:
  - **R5-P — Resolution primitives (engine/code).** Phased interruptible resolver
    (`resolution.*` stage events), the `contest`/`apply` kinds + `reaction`
    trigger window, the `RollMechanic` registry (xwn_2d6 / d20_attack / d20_save),
    the damage pipeline (packet→wounding→mitigation→routing→pools), targeting,
    action-economy, and new effect verbs (`emit_damage`, `spawn_entity`,
    `reposition`, `run_action`, `set_event`).
  - **R5-A — Action/skill IR + XWN content.** `action` (L1 archetypes + L2
    concrete) and `skill` (L3 overlay) IR record types + schema; entity field
    changes (`actions`/`pools`/`defenses`; retire `attack_skill`); the XWN core
    pack: action archetypes + the 19 skills as overlays.
  - **R5-W — Make it live (A1–A4).** Wire the modifier pipeline into all five
    resolver points (targets now `action.<id>`); wire the declarative trigger
    evaluator into the live event loop; effect entity **roles** (`self`/`target`);
    event→payload catalog + `when:` field validation.
  - **R5-C — Grammar compiler (Rust).** YAML parse, record dispatch, form-2 verb
    registry, dice + condition validation, inline lifting, two-bucket diagnostics,
    schema validation. Agnostic to the action set (validates against catalogs).
  - **R5-CLI — Integration.** `hrctl content compile <dir>` gathers YAML +
    catalogs from packs, calls the Rust compiler via PyO3, reports diagnostics,
    stores IR.
- **R6 — Intent-writer hardening + rusqlite spike.** Formalize the intent
  application layer; spike Rust-owned persistence for one subsystem to validate
  the eventual transition. Decide go/no-go for moving the writer.
- **R7 — Plugin surface.** Define the subprocess/IPC plugin contract for
  Python-authored effects/procedures/compute hooks invoked by qualified name.

## 8. Risks & mitigations

- **Parity drift / subtle numeric differences.** → Parity harness is the gate;
  deterministic RNG; property tests for invariants.
- **IR divergence between Rust and Python.** → Generate both from one schema;
  CI check that the exported schema matches the committed one.
- **PyO3 / maturin build complexity in CI and packaging.** → Establish in R0 with
  the Docker build env already created for the Tauri shell; pin toolchains.
- **Content compiler emitting wrong-but-valid records.** → Deterministic compile;
  schema-validate every record; two-bucket diagnostics; `UnimplementedPrimitive`
  catches reference typos. (No LLM, so no plausible-hallucination class.)
- **Scope creep / big-bang temptation.** → Phase gates; each phase shippable;
  Python remains the fallback path until a component's parity is proven. R5 is
  large — its sub-phases (§7) are independently shippable.

## 9. Open questions (revisit as phases land)

- Generate Pydantic from JSON Schema, or validate hand-written models against it?
  (R0 will pick based on tooling friction.)
- Intent application: synchronous in the request path vs. queued through the
  existing EventBus? (R1/R6.)
- The R5 content-system residual open questions live in §11.

## 11. R5 content-system design set

R5's design is captured in four committed docs under `docs/design/`. Read them in
this order:

1. `2026-06-19-action-resolution-primitives-design.md` — **the foundation.** The
   L0–L3 abstraction stack; the three resolution primitives (`contest` / `reaction`
   / `apply`); the pluggable roll mechanic; the system-agnostic damage/health
   pipeline (AC = one configuration; SDC/MDC, GURPS, Savage map onto the same
   stages); phased interruptible resolution; the `action`/`skill` record shapes;
   and the primitive-vs-data line.
2. `2026-06-19-xwn-skill-action-decomposition.md` — the L1 action archetype
   catalog, seven worked examples, the 19 XWN skills re-expressed as L3 data
   overlays, and Judo as proof a new skill is pure data.
3. `2026-06-18-content-grammar-compiler-spec.md` — the YAML grammar → IR compiler
   (form-2 surface, verb registry, the two dice/condition grammars, inline
   lifting, the two-bucket diagnostics, strict-fail).
4. `2026-06-18-content-authoring-guide.md` — the human-facing "how to author"
   guide (the four shapes, the sugar dial, worked examples, reading compiler output).

### Locked decisions (this conversation)

- **B1** Reference classification: a missing **authored-content** record
  (trait/status/tag/table/trigger/procedure/action/skill) → `SyntaxError`; a
  missing **engine primitive** (event, verb, compute fn, resource, modifier
  target) → `UnimplementedPrimitive`.
- **B2** Dice fields stored as the validated **string** (engine parses at runtime).
- **C1** v1 authorable entity types = `creature` + `item`; `npc`/`character`
  deferred. (Their *models* still gain `pools`/`defenses`/`actions`.)
- **C2** Item-granted modifiers via `grants_traits: []` (reuse trait machinery).
- **C3** Compiler is pure Rust (`compile(docs, catalogs) -> {records|diagnostics}`)
  driven by a Python CLI.
- **A1** Wire modifiers into **all five** resolver points (skill check, save, AC,
  attack bonus, damage); the granular modifier target is `action.<id>` (skills are
  *sources*, not targets).
- **A2** Wire the declarative trigger evaluator into the live event loop (part of R5).
- **A3** Effects encode an entity **role** (`self`/`target`/literal).
- **A4** Build event→payload validation (catalog) so `when:` `event.<field>` paths
  are checkable.
- **Action model sign-offs** (primitives doc §9): `creature`/`character` gain
  `actions`/`pools`/`defenses`; retire `attack_skill`; fold `innate_attacks` into
  `actions`. The 19 XWN skills stay canonical but become data overlays.

### Gate decisions (resolved 2026-06-19)

- **Range model** → **wire square-grid distance from the start.** Range bands map
  to grid (Chebyshev) cell distances now, reusing the existing `SquareGrid`; no
  abstract-bands-deferred interim.
- **Degree banding** → **band everything.** Every contest (incl. ordinary checks)
  produces a `degree`; degrees confer bonuses/penalties.
- **Passive perception** → yes: a stored `perceive` defense value is the stealth
  `tn_source`; active perception is a `perceive` contest.
- **Reaction budget** → 1/round, overridable per reaction.
- **Unskilled penalty** → yes, a default untrained-penalty modifier applies when an
  entity lacks the named skill. (Value is config/data — default to the XWN
  convention of −1; confirm the exact number.)
- **Intent vocabulary split** → **confirmed:** `emit_damage`/`spawn_entity`/
  `reposition` are new **intents** (host applies post-resolution); `run_action`/
  `set_event` are **resolution control-flow** that mutate an in-flight resolution,
  not intents.
- **New condition functions** (still to add): `has_action(e,id)` and `has_skill(e,id)`
  → add to the DSL parser/evaluator + the R1 parity corpus.
