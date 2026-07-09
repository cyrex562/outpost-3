# Client Event Interface & Renderer-Agnostic World Model

> Status: **Proposed** (design + tickets; no implementation yet).
> Author: architecture review, 2026-07-04.
> Supersedes the backend-only sketch in [`typed_events.md`](typed_events.md) (Python-era)
> and extends it to the full backend→client contract and a renderer-agnostic client model.
> Tickets: HR-792 … HR-796 (see `todo.md`).

## Motivation

The event/world-state interface between the Rust backend and the Vue frontend works, but
every seam is **convention with zero enforcement**. That convention has already produced a
recurring class of bug — an event is emitted but nothing consumes it, or an event is consumed
but never reaches the client — most recently **HR-791** (movement froze because a client-notice
event was mis-named with a `_requested` suffix and the controller filtered it out), and before
that the entire HR-771…790 event-wiring audit.

Separately, we want to reuse the same backend under a **different renderer** — a WebGL/canvas
tilemap and/or a desktop-focused game engine. Today that is impossible without a rewrite: the
event-dispatch logic, the client world state, and the Vue rendering are fused together in
`_websocketHandlers.ts` + Pinia, and the "world model" is fragmented into three incompatible
grid shapes, each hand-built for one Vue SVG component.

This document proposes a **layered interface with a generated contract** that (a) makes the
whole HR-791 class a compile/CI failure instead of a runtime surprise, and (b) makes the
client world model portable across renderers.

## Current state (2026-07-04)

Data flow (already the right *shape*, not yet a *contract*):

```
Rust subsystem ── emit GameEvent{event_type:String, data:JSON} ──► EventBus / broadcast
      │                                                                    │
      │ (*_requested events write SQLite via handlers, then are filtered)  │
      ▼                                                                    ▼
  SQLite world state                              wsmsg::event_to_frames (generic passthrough)
      ▲                                                                    │
      │ REST snapshot (GET /map, /character, /status_effects)              │ WS {type:"game_event"}
      └──────────── hydrate ◄───────────────────────────────────┐         ▼
                                                    _websocketHandlers.ts (501-line if/else, 31 branches)
                                                                 │ mutates Pinia stores
                                                                 ▼
                                       game / map / town / encounter stores ──► Vue SVG components
```

### What is already good (keep it)

- **Event taxonomy is clean.** `classify_event_kind` (`crates/harsh-core/src/events.rs`) already
  distinguishes three kinds:
  - **CommandIntent** — `event_type` ends with `_requested`; consumed by server-side handlers to
    write SQLite; **filtered out** of the client stream in
    `GMController::resolve_domain_events` (`controller.rs`).
  - **Presentation** — `gm.narrate`, `gm.suggestions`, `town.map`.
  - **DomainResult** — everything else; these are the ~30 client notices that update the world model.
- **Transport is already renderer-agnostic.** `event_to_frames` (`crates/harsh-web/src/wsmsg.rs`)
  forwards every non-narration event generically as `{type:"game_event", event:{…}}`. No per-type
  allowlist. This layer does not need to change.
- **Hydration is already snapshot + delta.** REST gives full snapshots
  (`GET /api/worlds/current/map`, `/api/character`, `/api/character/:id/status_effects`); the WS
  stream gives deltas on top.

### What is broken (all convention, no enforcement)

1. **Stringly-typed events.** ~50 `event_type` string literals inline at emit sites; no registry
   or enum. `event.data` is `JsonObject` on the wire and `Record<string, unknown>` on the client.
2. **Hand-mirrored types.** `frontend/src/types/api.ts` is hand-written; its header comment even
   says the shapes "mirror the harsh-core Rust structs." Per-event payloads are **not typed** —
   the client casts `data as Record<string,unknown>` and narrows field-by-field with inline `as`.
   A Rust field rename breaks the client silently.
3. **Dispatch is a 501-line, two-pass if/else with no `else` warning.** Unknown events either
   silently no-op (if in a hardcoded suppress list) or dump `[event_type]` into the chat log.
   Nothing signals an unhandled event.
4. **No coverage test.** Nothing asserts every emitted `DomainResult` has a client handler, or
   that field names align. HR-791 passed the entire suite.
5. **The client "world model" is fragmented and Vue-coupled.** Three incompatible grid shapes for
   the same concept — `map` (`Map<"q,r",CellData>`), `town` (`TownCell[]`), `encounter`
   (`PositionsCell[]`) — plus world state (cells, player pos, HP, combatants) tangled with pure UI
   state (chat messages, suggestions) inside `gameStore`. None of it is reusable by another renderer.

### Latent issues found during the review (ticket regardless of redesign)

- **`exploration.search_requested` / `take_requested` / `rest_requested`** carry *result* data
  (found items, etc.) but end in `_requested`, so they are filtered — the structured result never
  reaches the client and survives only as `gm.narrate` text. Same failure mode as HR-791, currently
  masked by narration. → **HR-793**.
- **`*Requested` struct naming is a footgun.** `ExplorationMoveRequested` now emits
  `"exploration.moved"`; that struct-name / event-name mismatch is exactly what caused HR-791. →
  fold into **HR-792**.
- **Dead payload structs**: `ActionMoveNotice`, `ExplorationEncounterNotice`,
  `ExplorationSearchCompletedNotice`, `ExplorationEnterCellNotice`, `InventoryItemTakenNotice`, and
  all the `gm.*` editor payloads are defined but never emitted. → fold into **HR-792**.

## Proposed architecture

Split the one fused thing (dispatch + world state + Vue rendering) into **three layers** joined by
a **generated contract**.

### Layer 1 — Contract (single source of truth, generated)

The Rust payload structs already *are* the schema; make them authoritative.

- Add a backend **event registry**: an enum (or const table) where each variant declares its
  `event_type` name, its `kind` (CommandIntent / Presentation / DomainResult), and its payload type.
  This replaces inline string literals and — crucially — makes `kind` **explicit data** instead of
  a `_requested` string-suffix guess, permanently removing the HR-791 footgun.
- **Generate the TS types from Rust** (`ts-rs` derive, or `schemars` → JSON Schema → TS; we already
  depend on `schemars` for the IR schema, so the pipeline exists). The client gets a real
  discriminated union:
  ```ts
  type ClientEvent =
    | { event_type: "exploration.moved"; data: ExplorationMoved }
    | { event_type: "combat.attack";     data: CombatAttack }
    | … ;
  ```
  Rename a Rust field and the frontend fails to compile.

### Layer 2 — Client world model + reducer registry (pure TS, no Vue)

A framework-free module holding the canonical client world:

- **One unified `Grid` model** (`{kind, width, height, cells: Map<Coord,Cell>, entities}`) serving
  world / town / encounter — replacing the three bespoke shapes.
- Entities as `{id, kind, coord, hp, maxHp, status, …}`; plus player, scene, chaos, etc.
- A **table-driven reducer registry** `Map<event_type, (model, payload) => ChangeSet>` — not a
  500-line if/else. Because the handled set is *enumerable*, we can CI-assert coverage; unknown
  events **warn loudly** in dev.
- `hydrate(snapshot)` formalizes the REST snapshot; events apply as deltas on top.
- The model emits **fine-grained change notifications** ("cell (q,r) changed", "entity X hp
  changed", "scene changed") — the seam renderers subscribe to.

This layer is the reusable core. (Prior art: the `typed_events.md` registry idea, here moved to the
client and generalized from validation-wrapper to full projection.)

### Layer 3 — View adapters (renderer-specific, pluggable)

- **Vue / Pinia (today):** Pinia becomes a *thin subscriber* to Layer 2; it keeps only genuinely-UI
  state (panel layout, chat log, modals, suggestions). Components render reactive projections.
- **WebGL / canvas (future):** a renderer subscribes to the *same* change notifications and updates
  GPU buffers / a scene graph. Because the grid is unified, **one tilemap renderer serves world,
  town, and encounter**.
- **Native / Rust desktop engine (future, e.g. Bevy):** the payload structs are *already shared*
  Rust, so this renderer can skip WS/serialization entirely and apply domain events straight into
  an in-memory ECS world in-process. The Layer-1 contract is the invariant that stays constant
  across JS and native renderers.

**Through-line:** `GameEvent` + typed payloads are the ABI; the Layer-2 world model is a pure
projection of that ABI; renderers are pluggable subscribers. Same backend, swap the view.

### Enforcement (this is what ends the bug class)

1. **Codegen** — a Rust field rename breaks the TS build.
2. **Coverage test** — every `DomainResult` in the registry has a Layer-2 reducer (CI gate). *This
   is the test that would have caught HR-791 and the whole audit class.*
3. **Dev-mode client warning** on any unhandled/unknown event.
4. **Round-trip fixture** — sample payloads serialize in Rust, deserialize against the generated TS
   types in a test.

## Migration plan (incremental — Vue keeps working throughout)

- **Phase 0 — cheap guardrails (HR-792).** Dev-mode `console.warn` on unhandled events; a
  backend↔frontend coverage test (even against a hand-listed set catches drift today); rename the
  misleading `*Requested` structs; delete the dead payload structs. Continuous with HR-771…791.
- **HR-793 — latent bug.** Decide search/take/rest: rename to notice events + add client reducers,
  or confirm narration-only is intended and document it.
- **Phase 1 — contract (HR-794).** Event-registry enum with explicit `kind`; TS payload codegen →
  generated `events.gen.ts` discriminated union; round-trip fixture.
- **Phase 2 — world model (HR-795).** Extract the renderer-agnostic `worldModel` + reducer registry;
  unify the three grids; Pinia becomes a subscriber. This is the step that actually unlocks reuse.
- **Phase 3 — proof (HR-796).** A WebGL tilemap for the map panel driven off the same model,
  proving the seam holds.

## Non-goals

- Replacing `GameEvent` as the transport envelope (it stays; it is already renderer-agnostic).
- Changing the WS frame shape or published event names for existing clients (Phase 0–2 preserve
  the wire format; codegen documents it, does not alter it).
- Building the WebGL or desktop renderer now — Phase 3 is a proof-of-seam, not a product renderer.
- Moving mechanics onto the client. The backend remains authoritative; the client model is a
  read-projection plus optimistic echoes.

## Open questions

- **Codegen tool:** `ts-rs` (derive on each struct, simplest) vs. `schemars` → JSON Schema → TS
  (reuses existing dep, one more build step). Decide in HR-794.
- **Reducer granularity vs. Pinia reactivity:** whether Layer 2 owns reactivity (e.g. a signals lib)
  or stays plain and lets each adapter wrap it (Pinia `reactive`, WebGL dirty-flags). Leaning plain
  core + adapter-owned reactivity.
- **In-process native path:** whether a future Rust renderer consumes `GameEvent`s over a channel or
  applies domain events pre-serialization. Deferred until a native renderer is real.
