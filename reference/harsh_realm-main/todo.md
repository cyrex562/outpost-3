# Harsh Realm TODO List

> ⚠️ **Active issue tracking moved to GitHub Issues (2026-07-04).** File new bugs/features
> in the Issues tab (or `gh issue create`); the agent reads/updates them live via `gh issue`
> — no commit/push needed. This file is now a **historical archive**; do not add new items here.
>
> Open items were migrated to issues (the `HR-###` prefix is kept in issue titles for
> continuity with commits/design docs; GitHub issue numbers are canonical going forward):
>
> | Item | Issue |
> |------|-------|
> | HR-786 searchable loot | [#99](https://github.com/cyrex562/harsh_realm/issues/99) |
> | HR-787 visual loot indicators | [#100](https://github.com/cyrex562/harsh_realm/issues/100) |
> | HR-788 faction events → frontend | [#101](https://github.com/cyrex562/harsh_realm/issues/101) |
> | HR-793 filtered search/take/rest results | [#102](https://github.com/cyrex562/harsh_realm/issues/102) |
> | HR-794 event contract + ts-rs codegen | [#103](https://github.com/cyrex562/harsh_realm/issues/103) |
> | HR-795 renderer-agnostic world model | [#104](https://github.com/cyrex562/harsh_realm/issues/104) |
> | HR-796 WebGL proof-of-seam | [#105](https://github.com/cyrex562/harsh_realm/issues/105) |
>
> (HR-792 was completed in PR #93; its checkbox below was stale.)

> Completed items are archived in [todo-archive.md](todo-archive.md) (HR-400 … HR-745).

## Event-wiring audit (2026-07-03)

Findings from a two-agent code review of event wiring (prompted by the movement bug
where `exploration.move_requested` was emitted but had no subscriber, so moving never
persisted position). Each item is fixed via the **fix → review → test → PR** loop in
AGENTS.md "Automated fix loop", one at a time, with the human gating each merge.

### Tier 1 — gameplay-breaking (player actions silently lost)

- [x] HR-771: `rest` never restores HP. Fixed: `handle_rest` now calls `HealingSystem::rest` (level+CON-mod HP for plain rest, max_hp for "until healed") and persists via `save_character(&healed, false, true, false)`; emits `character.hp_changed` for the frontend. Regression tests: `rest_restores_hp_and_persists` + `rest_until_healed_restores_to_full_hp` in exploration.rs.
- [x] HR-772: `take` never persists picked-up items. `handle_take` computes updated
  markers + inventory into `exploration.take_requested` but never writes them (no
  subscriber, no direct `CellRepository`/`EntityRepository` write). Item stays on the
  ground and never enters inventory. Fix: persist in `handle_take` + test.
  Done: handle_take now pushes the item to `character.equipment` + `save_character`
  and clears the marker via `CellRepository::save_cell_data`; regression test
  `take_persists_item_to_inventory_and_clears_marker` guards it.
- [x] HR-773: dungeon trap damage is silently lost. Fixed: `trigger_traps` now accepts
  a `&Character`, rolls `damage_expr` via `parse_damage_expr` + `DiceRoller`, builds a
  typed `CombatTakeDamageRequested {character_id, character_data, damage, source}` and
  emits it via `to_json_object`. Three regression tests guard the fix:
  `old_dungeon_trap_payload_fails_deserialization`, `trigger_traps_emits_valid_take_damage_payload`
  (dungeon.rs), and `hr773_old_dungeon_trap_payload_makes_handler_a_no_op` (event_handlers.rs).
- [x] HR-774: dungeon trap saving throws never resolve. Fixed: `trigger_traps` now
  resolves saves **inline** (same pattern as HR-773 rolling damage inline). When a trap
  has `save_type` + `avoid_diff`/`save_diff`, `saves::resolve_save(character, save_type,
  avoid_diff, 0, rng)` is called; `avoid_diff` is passed directly as `difficulty_modifier`
  so the effective target = `character.<save>_save + avoid_diff`. A `combat.save` event
  (payload: `CombatSaveNotice { character, save_type, roll, modifier, total, target,
  passed }`) is emitted for the frontend. Damage is **gated**: emitted only when there is
  no save or the save failed; a passed save fully negates trap damage. The orphan
  `action.save_requested` emission is removed (it had no consumer; HR-781 noted it — that
  is now superseded). Two regression tests guard the fix in dungeon.rs:
  `hr774_passed_save_negates_damage` (physical_save=1 → always passes → no damage) and
  `hr774_failed_save_triggers_damage` (physical_save=21 → always fails → damage emitted).
  Both tests fail without the fix (old code emits damage unconditionally + orphan
  `action.save_requested`, never a `combat.save`). HR-773 damage path and tests are
  unaffected (traps without saves still emit damage unconditionally).
- [x] HR-775: searched items never enter inventory. Fixed: extracted `grant_items` helper in `ExplorationScene`; `handle_search` now calls it for non-empty `discovered_items` and persists via `save_character(false,false,false)`. Three regression tests added: `grant_items_persists_discovered_items_to_inventory`, `grant_items_with_empty_slice_leaves_inventory_unchanged`, `grant_items_appends_multiple_items_to_inventory`.
- [x] HR-776: event-driven status effects never apply. Fixed: `resolve_domain_events` in `GMController` now intercepts `status.apply_requested` / `status.remove_requested` events and routes them through an on-demand `StatusEffectService` built from `runtime_content`, bypassing the lifetime-bound `register_status_effect_handlers`. Two regression tests added: `resolve_domain_events_applies_status_via_apply_requested` and `resolve_domain_events_removes_status_via_remove_requested`.

### Tier 2 — backend state not reflected in the UI

- [x] HR-777: `oracle.chaos_changed` is never emitted. Fixed: `run_scene_check` in `GMController` now captures `old_chaos` before the match block and emits `oracle.chaos_changed` (with `old_value`, `new_value`, `chaos_factor`) when the value changes. Regression test `run_scene_check_emits_chaos_changed_event` added.
- [x] HR-778: `social.disposition_change` payload mismatch. Backend enrichment: added
  `npc`, `old_mood`, `new_mood` to `SocialDispositionChangeNotice`; populated in
  `handle_disposition_update_requested` (request payload first, entity-table fallback).
  `SocialDispositionUpdateRequested` gains optional `npc_name` field; `social.rs` now
  carries `self.npc_name` in both emit sites. Mood labels use
  `gm::scenes::disposition_label` (social_support, Title Case) to match the social-scene
  narration casing. Frontend already read `npc`/`old_mood`/`new_mood` — no frontend
  change needed. Two regression tests added (pass-with-name, fallback-to-entity-lookup).
  UI render is a human-checklist item. Casing-drift follow-up: HR-785.
- [x] HR-779: inventory panel stale after combat. `inventory.item_given` /
  `inventory.item_lost` / `inventory.ammo_consumed` are neither handled nor suppressed by
  the frontend, so loot/ammo/gold changes don't refresh the panel until a scene change.
  Fix: added handler branch in `_websocketHandlers.ts` for all 3 event types — each calls
  `gameStore.loadCharacter()` (re-fetches equipment + gold) and adds a readable chat line
  ("You obtained X." / "You received N gp." / "You lost X." / "Ammo spent: type (N remaining).").
  Currency `item_given` detected via `item.type === "currency"` and shows gp amount.
  `ChatMessage.type` union extended with `"inventory"`. vue-tsc 0 errors.
  Existing e2e suite passes (no regressions). UI refresh-after-combat is a human-checklist item.
- [x] HR-780: character death has no UI treatment. `character.death` /
  `character.death_final` are unhandled + unsuppressed → they render as raw
  `[character.death]` chat lines. Fix: dedicated death UI + suppress the raw line; UI
  change → Playwright check.
  Styled death chat lines added for `character.death` ("You have fallen in battle!") and
  `character.death_final` ("{name} has died. Their story ends here."). `ChatMessage.type`
  union extended with `"death"`. ChatLog renders `death` messages with bold red text,
  thick red left border, and subtle red background — most prominent style in the log.
  Neither event reaches the raw `[evType]` catch-all. vue-tsc 0 errors.
  Full-screen death modal/overlay is a possible follow-up. UI render is a human-checklist item.

### Tier 3 — chat noise, dead code, robustness

- [x] HR-781: chat noise. `exploration.enter_hex` (every move) fell through to the catch-all
  and printed a `[exploration.enter_hex]` junk line. Fixed: added it to the suppressed list in
  `_websocketHandlers.ts` (position is applied via `exploration.move_requested`; enter_hex has no
  player-facing text). Re-scoped: `action.save_requested` was already retired by HR-774, so the
  only remaining junk-producer was `exploration.enter_hex`. Verified via cross-reference that no
  other emitted event is unhandled+unsuppressed (`time.tick`/`world.tick_advanced` are consumed
  internally by `run_time_tick` and never reach the frontend). UI render is a human-checklist item.
- [x] HR-782: dead frontend handlers for events the backend never emits. Re-scoped:
  `combat.save` is now LIVE (wired by HR-774), so it's kept. Removed the genuinely-dead
  handlers in `_websocketHandlers.ts`: `exploration.enter_cell` (redundant — only
  `exploration.move_requested` carries movement) and `faction.turn_completed` /
  `faction.reputation_changed` (the faction engine emits no game events). The faction
  message types + ChatLog styles are left in place for when faction events are wired
  (HR-788). vue-tsc clean; existing e2e green.
- [x] HR-783: systemic — all 14 domain handlers in `event_handlers.rs` were swallowing
  deserialize errors silently (`Err(_) => return vec![]`), which hid HR-773. Fix:
  `parse_payload` helper (eprintln! with event_type + target type name on failure) replaces
  all 14 silent sites; 3 regression tests added: `parse_payload_returns_some_for_valid`,
  `parse_payload_returns_none_and_does_not_panic_for_malformed`,
  `take_damage_handler_is_safe_no_op_on_malformed_event`.
- [x] HR-784: added `WorldDatabase::transaction` helper (BEGIN IMMEDIATE / COMMIT /
  ROLLBACK); wrapped `handle_take` (the only handler with genuine 2+ writes to different
  tables: `save_character` + `save_cell_data`). Sweepresult: all other multi-write sites
  are single-table or event-only — no additional wrapping needed. Tests:
  `transaction_commits_on_ok`, `transaction_rolls_back_on_err` (rollback regression).
  809 tests passing.

- [x] HR-785: consolidate the two duplicate disposition_label implementations. Analysis showed
  `npc_personality::disposition_label` (lowercase) had NO production callers — every consumer
  (event_handlers + all of social.rs) already uses `social_support::disposition_label` (Title
  Case, re-exported as `gm::scenes::disposition_label`). Removed the dead lowercase copy + its
  test, leaving one canonical implementation. No casing drift possible now.

## Loot & search (future features)

- [ ] HR-786: searchable loot sources — let the player search the ground, containers
  (crates/chests/corpses), and defeated NPCs' pockets/bodies for items, extending the
  existing `search` + death-marker/`take` systems into a richer looting flow (search a
  source → reveal its contents → take). Include a skill/luck element and empty-handed
  results. Perform a design iteration first, then implement.
- [ ] HR-787: visual loot indicators — render valuable/notable loot on the map + encounter
  grid (coin piles, gems, glint/sparkle markers) so worthwhile loot is spottable at a
  glance; scale the indicator by item value/rarity. Design iteration first.
- [x] HR-789: (follow-up from the HR-783 review) `status_effects/handlers.rs` had two more
  silent serde-deserialize swallows (`let Ok(..) else { return vec![] }` for
  `StatusApplyRequested` / `StatusRemoveRequested`). Fixed: added a local `parse_payload`
  helper (mirrors the HR-783 one, `eprintln!`s event type + target type on failure) and
  converted both `let Ok` sites to `let Some(..) = parse_payload::<T>(event)`. Test
  `parse_payload_returns_some_for_valid_and_none_for_malformed` added.
- [x] HR-790: buying gave nothing — `handle_buy` deducted gold but the purchased item was
  never added to inventory. Fixed: `handle_purchase_requested` now pushes `data.item` onto
  `character.equipment` before `save_character`. Regression test
  `hr790_purchase_adds_item_to_inventory` (fails without the push). (Selling was already fine —
  `handle_sell` persists the removal.)
- [x] HR-791: movement never updated the marker/LOCATION in the UI (found in Windows playtest).
  `handle_move` emitted the movement notice as `exploration.move_requested`, but
  `GMController::resolve_domain_events` drops every event ending in `_requested`, so it was
  filtered out and never published — the client only received `gm.narrate` (look text) +
  suppressed `enter_hex`. The backend DID persist the new position (earlier fix), so it was a
  pure notify-the-frontend gap. Fixed: renamed the emit to `exploration.moved` (a non-`_requested`
  notice that survives the filter) + the frontend listener. Controller regression test
  `move_publishes_exploration_moved_notice_to_frontend` asserts a move publishes the notice (fails
  with the old name). NOTE: no e2e asserts the marker/LOCATION actually moves — a robust
  movement-e2e is a coverage follow-up.
- [ ] HR-788: wire faction turn/reputation events to the frontend. The faction turn engine
  (`faction/`) runs on the weekly background tick but emits no game events, so there's no UI
  feedback when factions act or reputation shifts. Emit `faction.turn_completed` /
  `faction.reputation_changed` (published through the bus) and re-add the frontend handlers
  removed in HR-782 (the `faction_event`/`reputation` ChatLog styles are still present).

## Client event interface & renderer-agnostic world model

Design: [docs/design/2026-07-04-client-event-interface-and-world-model.md](docs/design/2026-07-04-client-event-interface-and-world-model.md).
Motivation: the HR-771…791 event-wiring bugs are all symptoms of a backend↔client interface
that is convention with zero enforcement (stringly-typed events, hand-mirrored `types/api.ts`,
a 501-line if/else dispatch with no unhandled-event warning, no coverage test). Separately, the
client "world model" is fragmented into three incompatible grid shapes and fused with Vue, so it
can't back a WebGL/canvas or desktop-engine renderer without a rewrite. Phased so Vue keeps
working throughout.

- [x] HR-792: (DONE — PR #93) Phase 0 — event-contract guardrails (cheap, high value; continuous with HR-771…791).
  (1) Dev-mode `console.warn` in `_websocketHandlers.ts` for any unhandled/unknown `event_type`
  (today they silently no-op or dump `[event_type]` into chat). (2) A backend↔frontend coverage
  test: enumerate the emitted `DomainResult` event types and assert the frontend has a handler for
  each — even against a hand-listed set this fails CI on drift (it would have caught HR-791).
  (3) Rename the misleading `*Requested` payload structs that emit non-`_requested` notices
  (e.g. `ExplorationMoveRequested` → `ExplorationMoved`) — that struct/event-name mismatch is the
  HR-791 footgun. (4) Delete dead payload structs never emitted (`ActionMoveNotice`,
  `ExplorationEncounterNotice`, `ExplorationSearchCompletedNotice`, `ExplorationEnterCellNotice`,
  `InventoryItemTakenNotice`, the unused `gm.*` editor payloads).
- [ ] HR-793: latent bug — `exploration.search_requested` / `take_requested` / `rest_requested`
  carry structured result data (found items, etc.) but end in `_requested`, so
  `resolve_domain_events` filters them and the client only ever sees the `gm.narrate` text. Same
  failure mode as HR-791, masked by narration. Decide: rename to notice events (`exploration.searched`
  / `.took` / `.rested`) + add client reducers so results are structured, OR confirm narration-only
  is intended and document it. If renamed, guard with the HR-792 coverage test.
- [ ] HR-794: Phase 1 — event contract as source of truth. Backend event registry (enum/const
  table: `event_type` name + explicit `kind` + payload type) replacing inline string literals, so
  `kind` is data, not a `_requested` suffix guess. Generate TS payload types from the Rust structs
  (`ts-rs` or `schemars`→JSON-Schema→TS; `schemars` already in-tree for the IR schema) into a
  generated `events.gen.ts` discriminated union — a Rust field rename then breaks the TS build. Add
  a serialize-in-Rust / deserialize-in-TS round-trip fixture. Open question: which codegen tool.
- [ ] HR-795: Phase 2 — renderer-agnostic client world model + reducer registry (the step that
  actually unlocks reuse). Extract a framework-free `worldModel` (no Vue) holding one unified `Grid`
  (`{kind, width, height, cells: Map<Coord,Cell>, entities}`) replacing the three bespoke
  map/town/encounter shapes, plus entities/player/scene/chaos. Table-driven reducer registry
  `Map<event_type, reduce>` replaces the 501-line if/else; `hydrate(snapshot)` formalizes the REST
  snapshot; model emits fine-grained change notifications. Pinia becomes a thin subscriber keeping
  only UI state (layout, chat, modals, suggestions).
- [ ] HR-796: Phase 3 — proof-of-seam. A WebGL/canvas tilemap for the map panel driven off the
  HR-795 world model (not the Pinia stores), demonstrating a second renderer reuses the same core
  with no changes to Layers 1–2. POC, not a product renderer.

- [x] HR-755: add an encounter window for battles/encounters that shows a top-down grid of the event with the positions of characters, monsters, and key terrain, that gets updated as the battle progresses. Perform a design iteration first, then implement.
  - Design iteration: docs/design/2026-06-30-hr-755-encounter-window.md. Chose the "real battle-grid substrate" approach — combat gains genuine per-combatant (q,r) + a captured terrain backdrop, emitted via a new `combat.positions` event; mechanics stay band-driven (positions mirror melee/near/ranged).
  - Backend: `Combatant.q/r` + `CombatState.terrain/features` (combat_runtime.rs); deterministic `assign_positions`/`positions_notice` on a fixed 9×9 grid (combat/positioning.rs); `CombatPositionsNotice` payload; emits `combat.positions` at combat start and on advance/withdraw; exploration threads the encounter cell's terrain/features into `create_combat`. 776 core + 18 web tests pass; clippy clean.
  - Frontend: `stores/encounter.ts`, `components/EncounterWindow.vue` (SVG grid cloned from TownMap/SquareMap, HP coloring from CombatPanel), WS routing for `combat.positions` (+ hp/defeat via existing combat events), `encounter` panel in layout + auto-show in GameView. vue-tsc clean; new e2e/encounter.spec.ts (4 tests) green.
  - Full `scripts/dev-test.sh` green: 120 Playwright e2e passed. Follow-up noted: `_websocketHandlers.ts` is now 435 lines (was already 419, over the 400-line rule) — split it in a separate cleanup.
  - Event-handling gap fixes (follow-up): (1) PC HP mirror now keys off the real `player_id` from `combat.player_hit` (was a hardcoded "player" that never matched) and the PC token renders an HP bar; (2) defeated enemies are kept as dimmed corpses consistently — `positions_notice` now includes them (`alive:false`) and `assign_positions` reserves corpse cells so live re-lays don't overlap; (3) reconnect resync — `get_initial_narration`'s new `Combat` arm re-announces the scene + replays `combat.start`/`combat.positions`, so a client dropping mid-fight recovers the encounter view. All via `scripts/dev-test.sh` green (120 e2e).

## IR interpreter roadmap (plan: [docs/design/2026-06-28-ir-interpreter-roadmap.md](docs/design/2026-06-28-ir-interpreter-roadmap.md))

Turns the IR machinery from a combat status engine into the game's general content
interpreter (authored YAML reacting to every event in every scene). See the plan
doc for current-state analysis, phase sequencing, and per-task acceptance criteria.

**Phase 1 — interpreter fires everywhere (do first; HR-756 → HR-757 → HR-758):**

- [x] HR-756: Extract a general `EntityResolver` trait (read entity view, apply resource delta) that combat and a new world/exploration resolver both implement; make `TriggerRuntime` generic over it. `CombatantResolver` becomes one impl. Combat path unchanged. (Plan §3 Phase 1)
  - Added scene-agnostic `EntitySnapshot` (entity_id + equipped_item_ids + intrinsic_trait_ids + scalar_fields) in `runtime_content/eval_context.rs`, built from combat via `EntitySnapshot::from_combatant` (carries the weapon-id-scheme bridging). Replaced `CombatantResolver` with `EntityResolver { snapshot(id) -> Option<EntitySnapshot>; apply_resource_delta(..) }`; `TriggerRuntime`/`EvalContextBuilder` now operate on snapshots and no longer depend on `Combatant`. `CombatScene`/`DemoCombatRunner` impl `EntityResolver`. Combat path unchanged (ir_triggers_combat, starter_pack_ir, vertical_slice, combat_scene tests green); added `non_combat_resolver_drives_the_loop` proving a non-`Combatant` resolver fires triggers on `exploration.enter_hex`. 726 lib tests pass (only the known autocrlf `committed_schema_matches_export` fails on Windows); clippy clean; harsh-web builds.
- [x] HR-757: Ungate `controller.rs::run_ir_triggers` so IR triggers fire for any published event in any scene (still guarded by `ir_triggers_enabled`); wire the active scene's resolver. Proof: a starter-pack `on: exploration.enter_hex` trigger fires on entering a tagged hex, pinned by an integration test. (Plan §3 Phase 1)
  - Dropped the `state == Combat` gate in `run_ir_triggers`; it now selects a resolver per scene — combat uses its in-memory `CombatScene`, every other scene uses a new `WorldEntityResolver` that snapshots the DB-backed player (`EntitySnapshot::from_character`, reusing the weapon-id bridging) and flushes `hp` deltas back via `save_character_state` (status/modifier intents persist through the services already). `TriggerRuntime::run`/`run_events` relaxed to `R: ?Sized` so a `&mut dyn EntityResolver` works. `ExplorationScene::handle_move` now emits `exploration.enter_hex` (self_id + terrain + features + coords) via a testable `enter_hex_event` helper. Starter pack ships the canonical non-combat example: a `ruin_dread` character trait → `dread` status `on: exploration.enter_hex when event.terrain == 'ruins'`. Tests: controller `enter_hex_fires_ir_trigger_in_exploration` (end-to-end: ruins applies dread, plains doesn't), `enter_hex_event_carries_self_id_and_terrain`, `from_character_snapshot_carries_traits_and_weapon`. 729 lib tests pass (only the known autocrlf schema test fails on Windows); integration tests green; clippy clean; harsh-web builds.
- [x] HR-758: World-clock `time.tick` heartbeat — emit a tick event on clock advance and evaluate `on: time.tick` triggers for all entities carrying them, so over-time effects (poison/buffs/wounds) tick on time in any scene. Replace the combat "ticks when the entity acts" hack and drive status expiry by tick count. (Plan §3 Phase 1)
  - Added `StatusEffectRepository::distinct_entity_ids` (the over-time carriers). New `GMController::run_time_tick` fans a `time.tick` event (self_id per entity) out over every status-bearing entity through the existing per-scene resolver, then `expire_due(tick)` removes elapsed statuses (ticks fire before expiry, so a status ticks on its final tick). Wired into `handle_input` for non-admin commands (the clock advances per turn). The acting-entity model is preserved — each entity is its own `self` for its tick. Starter pack: the `dread` status now carries an `on: time.tick` over-time trigger (gnaws 1 hp/tick) as the canonical clock-driven example, chaining from HR-757's `ruin_dread`. Tests: `time_tick_ticks_over_time_status_and_expires_it` (controller end-to-end: hp drops each tick in exploration via the WorldEntityResolver flush, status expires by tick count), `distinct_entity_ids_dedupes`. 731 lib tests pass (only the known autocrlf schema test); integration + clippy + harsh-web green. NOTE: outside combat the resolver snapshots only the player, so NPC-with-status over-time ticks await broader DB-entity snapshotting (and standalone/global triggers are HR-760). **Phase 1 of the IR interpreter roadmap is complete (HR-756→757→758).**

**Phase 2 — author-facing expressiveness (needs Phase 1):**

- [x] HR-759: Implement compute effects `roll_dice` / `run_procedure` in `dispatch::lower_effect`, feeding results to later effects via the eval context's `local` map (uses the procedure runner). Replaces the current explicit-error path. (Plan §3 Phase 2)
  - `dispatch` now threads an RNG and gives each fired trigger a per-trigger `local` compute scope: `roll_dice {dice, bind}` rolls (via `damage::parse_damage_expr`) and binds the integer total under `bind`; later effects reference it with `{ "ref": "name" }` (verbatim) or `{ "expr": "0 - local.name" }` (DSL-evaluated with `local` merged). Numeric params (`delta`, coords) resolve through `resolve_i64`/`resolve_value`. `TriggerRuntime` holds a `RefCell<SmallRng>` (entropy by default, `with_seed` for tests) so all existing `new()`/`run()` callers are untouched. `run_procedure` is an explicit, clearly-messaged deferral (wiring the procedure runner — ContentService + TableEngine + compute registry — into the trigger runtime is disproportionate here; tracked for later). Starter pack: `dread`'s over-time trigger now rolls `1d2` gnaw damage and spends it via `{expr}`, the canonical compute example. Tests: `roll_dice_binds_result_for_a_later_effect`, `ref_resolves_bound_value_directly`, `run_procedure_is_an_explicit_deferral`, `unbound_ref_errors`. 735 lib tests pass (only the known autocrlf schema test); integration + clippy + harsh-web green.
- [x] HR-760: Standalone & global trigger subscriptions — let authored `trigger` records and room/object/world-owned triggers fire by event type regardless of the acting entity, alongside entity-carried sources. Extend `index.rs` sourcing + the runtime context. (Plan §3 Phase 2)
  - The store already parsed `trigger` records into `standalone_triggers` (unused); added `RuntimeContentStore::standalone_triggers_on(event_type)` (sorted by id for determinism) and `triggers_for_event` now appends them as source #4, AFTER entity-carried statuses/items/intrinsic traits, so an entity's own reactions resolve before world reactions. Globals evaluate in the event's context (`self` = acting entity), so they fire on entity-bearing events (e.g. `exploration.enter_hex`) without being attached to that entity. Starter pack ships the canonical example: a `ruins_watch` global trigger logging "The ruins seem aware…" on entering any ruins hex. Tests: `standalone_globals_fire_last_and_sorted_by_id`, `standalone_globals_filter_by_event_type` (index), and controller `global_trigger_fires_without_being_carried` (a trait-less PC gets `dread` in a crypt via a global, none in plains). 738 lib tests pass (only the known autocrlf schema test); integration + clippy + harsh-web green. NOTE: globals fire on events that carry a `self`; entity-less pure-world events and room/object-owned `self` (the trigger's own entity as `self`) are a follow-up. **Phase 2 complete (HR-759 + HR-760).**

**Phase 3 — action model ("more detailed than Zork"; needs Phase 1):**

- [x] HR-761: Consume `actions` / `pools` / `defenses` instead of dropping them in the IR→`CreatureData` adapter — support multi-pool `EmitDamage`, named defenses, and authored creature/character actions so combat resolves through IR. (Plan §3 Phase 3)
  - **Damage-model core delivered.** Adapter now carries `pools`/`defenses`/`actions` onto `CreatureData` (only `actions` still warns — the turn loop doesn't perform them yet); `create_combat` copies pools/defenses onto enemy `Combatant`s (both gained the fields). `EntitySnapshot` now carries `pools` (derived `hp` pool + extra authored pools, live current) and `mitigations` (derived from named defenses: `dr`→DamageResistance, `armor`→ArmorRating). `IntentApplier::EmitDamage` no longer does naive `hp -= amount` — it queues a `(entity, packet)` on `ApplyOutcome.damage_packets`, and the trigger runtime routes each through `resolution::apply_damage` against the target's live pools/mitigations, applying per-pool deltas via the resolver (combat + demo + test resolvers now apply non-`hp` pool deltas to `Combatant.pools`). Default single-`hp` creatures behave exactly as before. Starter pack: the ash crawler gained `dr: 1` + a `carapace` SD pool (pinned by `starter_pack_ir`). Tests: `emit_damage_routes_through_pipeline_with_mitigation`, `emit_damage_md_packet_routes_to_md_pool`, `carries_pools_and_defenses_and_warns_only_for_actions`. 740 lib tests pass (only the known autocrlf schema test); integration + clippy + harsh-web green.
  - **Deferred (tracked as HR-765/HR-766):** authored `actions` driving the combat turn loop, and named-defense *avoidance* in the attack contest — both rewrite the turn/attack-contest layer rather than the damage model.

**Phase 4 — collapse the two content models (anti-rot; after HR-761):**

- [x] HR-762: Migrate the legacy `creatures:`/item-list catalog to IR format so there is one content model; provide a migration path/compat shim for existing worlds. (Plan §3 Phase 4)
  - **Compat shim delivered (creatures): IR is now a first-class live catalog source.** Added `CreatureRegistry::extend` (merge creatures, last-wins on id) and `RuntimeContentStore::creatures()`. `ExplorationScene::ensure_full_catalog(db)` (called before encounter selection) loads the legacy `creatures/*.yaml` catalog and folds the world's IR creatures (`ir_records` → `ir_creature_to_data`) into it, so IR-authored creatures (e.g. the starter `ash_crawler`) are now **encounterable** in live play and can override legacy entries by id. This makes IR the convergence target without touching every reader. Tests: `extend_merges_last_wins`, `ir_creatures_fold_into_the_live_catalog`. 753 lib tests pass (only the known autocrlf schema test); integration + clippy + harsh-web green.
  - **Remaining → HR-768:** an `ir_item_to_data` adapter + fold IR items into `ItemRegistry` the same way; convert the legacy `creatures/*.yaml` + `items/*.yaml` files to `ir/` records and retire the dual trees; do the same for status_effects/tables where they still live outside IR.
- [x] HR-768: Complete the legacy→IR catalog migration — add an `ir_item_to_data` adapter and fold IR items into `ItemRegistry` (mirror HR-762's creature shim); convert the legacy `creatures/*.yaml` + `items/*.yaml` (+ status_effects/tables) to `ir/` records and retire the dual content trees so there is a single IR catalog. (Continues HR-762.)
  - **Item bridge delivered (IR↔engine item parity).** `runtime_content::ir_item_to_data` converts an IR `Item` → engine `ItemData` (clamps i64→i32 with warnings; maps the consumable `effect` Food/Heal/SaveBonus incl. save-type parsing; notes `grants_traits` is served from the IR store, not the legacy model). `ItemRegistry::extend` (+ `RuntimeContentStore::items()`) folds IR items last-wins on id, mirroring HR-762's creature shim. Tests: adapter mapping/effect/save-bonus/unknown-save/clamp/grants-traits-note + `ItemRegistry::extend_merges_last_wins`. 760 lib tests pass (only the known autocrlf schema test); clippy + harsh-web green.
  - **Remaining (content-ops, deferred):** items aren't yet consumed by live combat (`CombatScene` is passed `None`; wiring a real `ItemRegistry` *enables* weapon/shock stats — an orthogonal, riskier change), so the item fold has no live consumer until that lands; and the legacy `creatures/*.yaml` + `items/*.yaml` (+ status_effects/tables) files still need converting to `ir/` records before the dual trees can be retired. The engine bridges (both adapters) are now in place to make that migration mechanical.

**Phase 3 follow-ups (split out of HR-761; need the turn/attack-contest layer, not just the damage model):**

- [x] HR-765: Authored actions drive the combat turn loop — make a creature/character perform its IR `actions` (named attacks/abilities) on its turn instead of the legacy single `damage`/`num_attacks`, resolving each action's effects (roll → emit_damage/apply_status) through the IR pipeline. (Carried by HR-761; the adapter already preserves `actions`.)
  - **Resolver delivered** (the reusable heart). `combat::action_resolver::resolve_action(action, actor_modifier, target, ctx, rng) -> ActionOutcome` generalizes the `vertical_slice` proof into engine code: it materializes the contest (mechanic from `roll_spec`, host-summed actor modifier, TN from `tn_source` — `defense` reads the target's named defense/legacy ac, `static`/`difficulty` value used directly; opposed/difficulty-key deferred with a clean error), rolls the die per mechanic, `resolve_contest` → `outcome_key`, selects the matching `outcome` branch, and lowers its effects to `Intent`s by reusing `dispatch` (so `roll_dice`/`{ref}`/`{expr}` and entity roles behave as in triggers). `apply` actions skip the contest. Tests cover hit→emit_damage, miss→no effects, static TN, named-defense TN, apply, and deferred-source errors. 750 lib + vertical_slice green; clippy + harsh-web clean.
  - **Remaining → HR-767:** wire the resolver into the live/demo turn loop (carry `actions` onto `Combatant`, pick + perform an action on a creature's turn instead of the legacy single attack, apply its intents through the IntentApplier + damage pipeline), plus action economy/targeting/multi-action selection.
- [x] HR-767: Wire `combat::action_resolver` into the turn loop — carry `actions` onto `Combatant`, have a creature perform an authored action on its turn (demo runner first, then live `CombatScene`) via `resolve_action`, applying its intents through the IntentApplier + damage pipeline; add action economy/targeting/multi-action selection. (Built on HR-765's resolver.)
  - Added `actions: Vec<String>` to `Combatant` (populated in `create_combat` from `CreatureData.actions`). Extracted the intent-apply path into `TriggerRuntime::route_applied` + exposed `TriggerRuntime::apply_intents` (resource deltas + `emit_damage` routed through the pipeline) so the turn loop spends an action's intents through the same path triggers use; `run_events` now reuses it. `DemoCombatRunner` turn now prefers a creature's first authored `action` (looked up in the store): builds the eval context, calls `resolve_action`, applies the intents, and records it — falling back to the legacy weapon attack when there's no action. Test: `creature_performs_authored_action_on_its_turn` (a brute performs `rend`, contest→success→emit_damage damages the PC through the IR pipeline). 751 lib tests pass (only the known autocrlf schema test); integration + clippy + harsh-web green.
  - **Live `CombatScene` wiring done.** `CombatScene` now holds an `Option<RuntimeContentStore>` (passed by the controller's `wire_combat_scene` from `runtime_content`). `run_enemy_turns` takes `db` (threaded through `dispatch_command`→`handle_attack`/`handle_use` + the surprise path); an enemy whose first `action` is in the store performs it via `perform_enemy_action` (take-store → build ctx → `resolve_action` → `TriggerRuntime::apply_intents` against the scene → restore store), mirroring the legacy player-damage consequences (`combat.player_hit` notice + last-stand + `character.hp` sync). Legacy weapon attack is byte-unchanged when there's no action. Test: `enemy_performs_authored_action_in_live_combat`. 765 lib tests pass (autocrlf-only failure); integration + clippy + harsh-web green.
  - **Damage models unified.** Extracted `apply_player_damage(attacker, damage) -> (events, paused)` — the single entry point for HP damage dealt to the player (Veteran's Luck offer + `combat.take_damage_requested` + apply + `combat.player_hit` + Last Stand + `character.hp` sync). The legacy weapon attack now routes its rolled damage through it, and `perform_enemy_action` splits the player's HP damage out of the action's intents (`split_player_damage` resolves `emit_damage` packets through the `apply_damage` pipeline against the player's live pools/mitigations), routes that total through the same handler, and applies the remaining intents (status/logs/non-player) via `apply_intents`. So action damage now offers Veteran's Luck, persists, notifies, and triggers Last Stand uniformly. Tests: `apply_player_damage_applies_and_notifies_for_non_warrior`, `…_offers_veteran_luck_for_warrior`, `…_triggers_last_stand_when_lethal`, plus the live `enemy_performs_authored_action_in_live_combat` now asserts HP loss ⇔ the unified take_damage event. 768 lib tests pass (autocrlf-only failure); integration + clippy + harsh-web green.
  - **Action economy / targeting / selection done.** `run_enemy_turns` now gives an action-bearing enemy a per-turn budget of `num_attacks` main-action activations (instead of one fixed action). Each slot re-reads the actor and calls `select_action` — the first of its `actions` that exists in the IR content and whose activation `costs` are affordable (`resolution::economy::can_afford` against a `resource_map` of the actor's pools + hp); the chosen action's `costs` are then spent via `apply_resource_delta`, so ammo/charges deplete and gate later slots in the same turn. `perform_enemy_action` resolves the target from the action's `targeting.shape` (`self` → the actor, else the player) and binds it as `target` in the eval context. Tests: `select_action_picks_first_affordable`, `enemy_action_economy_spends_budget_and_costs` (budget 2 + 1 ammo → one activation, cost spent), `enemy_self_targeted_action_spares_the_player`. 771 lib tests pass (autocrlf-only failure); integration + clippy + harsh-web green.
  - **Remaining (next slice):** cross-turn economy state — cooldowns + `uses` caps (`Activation.cooldown`/`uses`) and the reaction budget aren't tracked yet (needs per-combat state + tick); `prerequisites` beyond cost (range/condition) and grid range-band targeting (`in_range`) aren't enforced in abstract-band combat; non-HP player pools (shields) from actions still apply via `apply_intents` rather than the player-damage handler.
- [x] HR-766: Named-defense avoidance in the attack contest — use a creature/character's named `defenses` (ac/evasion/…) in the to-hit resolution (`resolvers::combat`) so authored defenses decide whether an attack lands, not just the legacy `ac` field.
  - Added `combat::resolvers::avoidance_defense(target, attacker_range_band)`: melee attacks contest the `ac` defense, ranged attacks contest `evasion` (falling back to `ac`), and with no named defenses the legacy `ac` field is used (so existing content is unchanged). `resolve_attack` now contests this value and reports it as `target_ac`. Starter pack: the ash crawler gained `evasion: 14` (harder to shoot than to stab); authoring guide §8.6/§8.8 updated to mark avoidance wired. Tests: `avoidance_prefers_named_defenses_by_range`, `ranged_attack_contests_evasion`. 744 lib tests pass (only the known autocrlf schema test); integration + clippy + harsh-web green.

**Cross-cutting:**

- [x] HR-763: Non-combat demo harnesses (NPC/social, dungeon, map) mirroring `DemoCombatRunner`, with endpoint + tests, so authored non-combat IR is exercisable in isolation. (Plan §3 Cross-cutting; can start after HR-757)
  - Built a general **event-driven** harness instead of per-subsystem runners (more useful: it exercises any non-combat IR — `exploration.enter_hex`, `time.tick`, global triggers — without coupling to social/dungeon/map). `runtime_content::DemoEventRunner` takes hand-specified `DemoEntity`s (traits/statuses/hp/pools/defenses) + a sequence of events, runs each through `TriggerRuntime` (advancing the clock + expiring statuses per step, mirroring the controller), and reports per-step narration/errors + final entity state (hp/pools/statuses). Endpoint `POST /api/demo/events` mirrors `/api/demo/combat` (compile YAML → ephemeral world → run → outcome). Tests: core `enter_ruins_then_ticks_apply_and_decay` + `global_fires_for_entity_without_the_carried_trait`; web `run_event_demo_applies_status_on_enter_hex` + compile-error + no-events guards. 742 lib + harsh-web tests pass (only the known autocrlf schema test); clippy clean both crates. NOTE: a Content Studio frontend panel for the non-combat demo is an optional follow-up (the combat panel exists; this ships the core + endpoint + tests).
- [x] HR-764: Trigger/effect authoring guide — extend [content-authoring-guide](docs/design/2026-06-18-content-authoring-guide.md) with the full trigger/effect/intent vocabulary, the event catalog, and worked examples per phase. (Plan §3 Cross-cutting; trails each phase)
  - Added §8 "The live IR runtime (as implemented)" documenting the **raw IR** (`kind`/`params`) the engine actually consumes today: the interpreter loop, trigger sourcing order (statuses → item grants → intrinsic traits → global), the full effect-verb table (incl. `roll_dice` compute + `{ref}`/`{expr}`, and the deferred/control-flow verbs), `entity_id` roles, the event catalog (`combat.attack` / `time.tick` / `exploration.enter_hex` with payloads), the damage model (pools/tier-routing/defenses/mitigation), worked examples cross-referenced to the test-pinned starter pack, and a "not wired yet" list (HR-765/766, run_procedure). Added a top-of-doc callout distinguishing §1–7 (aspirational form-2 sugar) from §8 (what runs now). Field names verified against `ir/components.rs`, `ir/effect.rs`, `resolution/damage.rs`. Docs-only.

## Code review follow-ups (PR #30 — admin/editor CRUD + GM tools)

- [x] HR-746: `FactionRepository::delete_faction` (harsh-core) does not delete `faction_assets`, `faction_asset_state`, or `faction_relations`, and `PRAGMA foreign_keys = OFF`. The new `POST /api/admin/factions/:id/assets` endpoint creates asset rows that orphan on faction delete (contradicts the editor's "delete faction and all its assets" intent). Add the cascade deletes.
  - `delete_faction` now also removes `faction_asset_state` (via subquery on the faction's assets), `faction_assets`, and `faction_relations` (faction_a/faction_b). Test `delete_faction_cascades_assets_state_and_relations` asserts no orphans remain.
- [x] HR-747: `clear_overrides`/`reset` in `crates/harsh-web/src/admin.rs` deletes rows `WHERE source IS NULL OR source='' OR source='world'`. Verify nothing legitimate (content-authored items/creatures, imported rows, any future pack-seeded rows) carries a NULL/empty `source` — otherwise reset-all wipes them too. Tighten to `source='world'` or stamp an explicit override marker.
  - Confirmed the override write paths stamp `source = 'world'` (admin.rs INSERT OR REPLACE), so tightened `clear_overrides` to `DELETE … WHERE source = 'world'`. Content/imported/pack rows (NULL/empty/pack-id source) are now preserved on reset.
- [x] HR-748: character PUT (`editor/characters.rs`) with an existing `character_state` only syncs known stat fields via `sync_character_state`; any other keys in `data` are silently dropped and `entities.data` is never updated for PCs. Decide intended behavior — persist the extras, or document/reject them.
  - Decision (per request): unrecognized keys no longer silently drop — they return a detailed, structured **422** instead of crashing or losing data. For a normalised character, `update_character` now validates `data` top-level keys against `KNOWN_CHARACTER_DATA_KEYS` (the fields GET assembles / PUT persists) *before any writes*; unknown keys yield `{ error: "UnrecognizedFields", message, unrecognized: [{ path: "data.<key>", fragment: <value snippet>, did_you_mean: [closest known keys] }], known_fields }`. Suggestions are nearest known keys by Levenshtein distance (≤ a small threshold). Plain (non-`character_state`) entities still persist their whole free-form `data` blob (no schema there to validate against). To add a new term, extend `KNOWN_CHARACTER_DATA_KEYS` (+ `sync_character_state`/`assemble_character_data`). Tests: known→no error, unknown→path+fragment+suggestions, no-suggestion threshold, levenshtein, fragment truncation. harsh-web 18 tests + clippy + build green; harsh-core unaffected.
- [x] HR-749 (altitude): `character_state`/`npc_state` assembly is reimplemented with raw SQL in the web layer (`editor/characters.rs::load_character_state`) instead of a harsh-core repository; orphaned `EditorEntityRecord`/`CharacterPreviewResult` models already exist in core. Consider an `EditorEntityRepository` so schema changes live in one place. (Assessed: a sizeable repository-extraction refactor; deferred to its own focused pass.)
  - Added `repositories::editor_entity::EditorEntityRepository` (harsh-core) owning the editor entity reads: `assemble_character_data` (the `character_state` + npc `personality` assembly, moved verbatim), `load` (entities row with `data` = assembled state or the stored blob → the orphaned `EditorEntityRecord` model), and `list` (type/alive-filtered summaries). `editor/characters.rs` `list_characters`/`get_character` now call the repo and no longer issue raw SQL (the `load_character_state` fn is gone); wire shapes preserved (list summaries omit `data`; GET serializes `EditorEntityRecord`). Tests: `load_assembles_character_state_into_data`, `load_falls_back_to_data_blob_without_character_state`, `list_filters_by_type_and_alive`. 764 lib tests pass (only the known autocrlf schema test); harsh-web tests + clippy + build green. (The PUT write-back raw SQL stays in the web layer — that's HR-750's `UPDATE…SET` builder.)
- [x] HR-750 (cleanup): extract the duplicated partial-`UPDATE … SET` SQL builder (`editor/characters.rs` `update_entity_columns`/`sync_character_state`, `editor/oracle.rs` `run_update`) into one shared helper. (Assessed: doable but the three sites bind params differently; moderate refactor risk — deferred from the correctness batch.)
  - Added `editor::apply_partial_update<S: AsRef<str>>(db, table, sets, params, key_col, key_val) -> Result<bool>` (generic over the SET fragments so all three call shapes — `&[&str]` and `&[String]` — reuse it; appends the key as the final param; returns `false` for an empty SET list). `update_entity_columns`, `sync_character_state`, and `run_update` now delegate to it (only the column/value collection is site-specific). Tests: `partial_update_applies_sets_and_keys_on_the_id`, `partial_update_is_a_noop_for_empty_sets`. harsh-web: 12 tests + clippy + build green; harsh-core unaffected.
- [x] HR-751 (cleanup): consolidate the two near-identical `respond()` helpers (`admin.rs` vs `editor/mod.rs`) into one shared fn.
  - Added `crate::response::respond_with(result, error_kind)`; both `respond` helpers delegate (keeping their `"AdminError"`/`"EditorError"` labels), and the now-unused imports were removed.
- [x] HR-752 (cleanup): de-duplicate the directory-walk logic (`editor/files.rs::walk_yaml` vs `editor/archive.rs::zip_tables`) and fold `yaml_table_status`'s double walk + per-file re-read into a single pass. (Assessed: the two walks build different outputs (Value list vs zip bytes); a shared walk helper is reasonable but moderate — deferred.)
  - Added `editor::walk_yaml_files(root) -> Vec<(PathBuf, String)>` (one recursive `*.yaml`/`*.yml` walker yielding absolute + forward-slash relative paths, sorted). `walk_yaml` (→ `[{path,size}]`), `zip_tables` (filters `tables/`, reads bytes), and `table_status` all delegate to it; `table_status` now walks once and reads each `tables/` file a single time (was: walk via `walk_yaml`, then re-derive + re-read). Tests: `walk_yaml_files_recurses_filters_and_sorts`. harsh-web: 13 tests + clippy + build green; harsh-core unaffected.
- [x] HR-753 (efficiency, low): `export_all`/`import_all` (`editor/transfer.rs`) run all ~22 tables sequentially inside one blocking `session.read` closure; on a large world this holds the actor thread (blocks gameplay) for the whole dump/load. Revisit if worlds grow. (Assessed: low priority, already documented in-code as acceptable until worlds grow — no change.)
  - Both now do **one short `session.read` per table** (looping in the async handler, assembling the result outside the lock) instead of one closure over all ~22 tables — so the actor thread is released between tables and gameplay isn't blocked for the whole dump/load. Trade-off noted in-code: export is now a per-table snapshot rather than one global snapshot (fine for an editor export); import was already best-effort with per-table error collection (no cross-table transaction lost). Behavior/output preserved. harsh-web 18 tests + clippy + build green.
- [x] HR-754 (conventions, low): silent failures — `editor/files.rs` `clone_world_file` uses `let _ = db.set_meta("name", …)` and `editor/gm.rs::entity_name` uses `.ok()`, both swallowing DB errors. Log on failure per AGENTS.md "no silent failures".
  - Both now log via `eprintln!` (the harsh-web non-fatal convention) on error before their best-effort fallback (`clone_world_file` also logs an open failure; `entity_name` logs the lookup error before falling back to the id).

## Playtest / build feedback

- [x] HR-769: Fix admin-panel e2e: route.request().json() is not a function (use postDataJSON)
  - e2e/admin-panel.spec.ts:248 ("Difficulty Targets — Save fires PUT with updated value") uses Playwright's non-existent Request.json(); should be route.request().postDataJSON(). Test currently always fails. Surfaced by scripts/dev-test.sh.
  - Fixed: replaced `void route.request().json().then((b) => putBodies.push(b))` with the synchronous `putBodies.push(route.request().postDataJSON())`. vue-tsc --noEmit clean; the test now passes (1 passed).
- [x] HR-770: Fix ui e2e: connection-status span not visible when no world loaded
  - e2e/ui.spec.ts:75 expects a span matching /Connecting|Disconnected/ visible before a world is loaded, but the element is not found (5s timeout). Either the status indicator UI regressed or the test is stale. Surfaced by scripts/dev-test.sh.
  - Root cause: stale test. App.vue opens the WebSocket at startup, so before a world loads the label reads "Connected" — deliberately excluded by the old regex (to avoid matching the "Connected to Harsh Realm." chat line). Fixed durably: added `data-testid="connection-status"` to the status-label span in GameView.vue and asserted on it via getByTestId + toHaveText(/Connecting|Connected|Disconnected|Reconnecting/). vue-tsc clean; test now passes (1 passed).
