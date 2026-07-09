# HR-19 — Quest & Plotline Service

> Status: design approved 2026-07-08 (accept-command + one-PR chosen). Issue: #19.
> A durable `QuestService` subsystem: quests as pack content, per-world quest
> instances in SQLite, service + events + REST + a `quests` command + a sidebar
> badge. Infrastructure only (1–2 example quests).

## Architecture mapping (Python issue → Rust)

The issue references the removed Python backend; this maps it onto Rust
(`crates/harsh-core` engine + `crates/harsh-web` host + Vue frontend), following
the **`status_effects/` durable-subsystem template** (6 modules).

### Subsystem: `crates/harsh-core/src/quest/`

- **`schema.rs` — `Quest` content record:**
  `{ id, name, description, objectives: [QuestObjective{key, description, target?}],
  reward_gold, reward_xp, reward_items: [String], prerequisites: [id],
  expiry_tick?, faction_id? }`.
- **`catalog.rs` — `QuestCatalog`:** loads all quests from
  `content/xwn-core/content/quests/*.yaml` via `default_content_dir()` +
  `serde_yaml` (the loot-table/creature idiom). Exposes `get(id)` / `all()`.
  **Why not the pack `ContentService` / `RuntimeContentStore`:** the GM
  controller/scenes hold no `PackRegistry`, and quests are plain content (not
  IR-compiled `ComponentRecord`s like statuses). Direct content-dir loading
  matches how loot tables and creatures already load, and keeps the service
  decoupled + unit-testable via a `QuestContent` trait.
- **`models.rs` — `ActiveQuest`:**
  `{ id: i64, entity_id, quest_id, status, accepted_tick, completed_tick?,
  progress: JsonObject }`.
- **`repository.rs` — `QuestRepository<'a>`** over `&WorldDatabase`:
  `accept`/`insert`, `find_active(entity, quest_id)`, `list_active`,
  `list_completed`, `list_all`, `set_status`, `set_progress`.
- **`service.rs` — `QuestService<'a, Q: QuestContent, K: WorldClock>`:**
  `accept(entity_id, quest_id)` (validates the quest exists + prerequisites all
  completed + not already active), `update_progress(entity_id, quest_id,
  objective_key, value)` (→ `in_progress`), `complete(entity_id, quest_id)`,
  `fail(entity_id, quest_id)`, `list_active(entity_id)`.
- **`handlers.rs`:** `handle_*_requested(service, event)` returning the result
  notice event — mirrors `status_effects/handlers.rs`.

### States

`accepted → in_progress → completed | failed`. `list_active` returns
`accepted` + `in_progress`; terminal states never appear there. The status
column is always one of these four (invariant-tested).

### Table (`db_schema.rs`)

```sql
CREATE TABLE IF NOT EXISTS quest_instances (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_id      TEXT NOT NULL,
    quest_id       TEXT NOT NULL,
    status         TEXT NOT NULL DEFAULT 'accepted',
    accepted_tick  INTEGER NOT NULL,
    completed_tick INTEGER,
    progress_json  TEXT NOT NULL DEFAULT '{}'
);
CREATE INDEX IF NOT EXISTS idx_quest_instances_entity ON quest_instances(entity_id);
```
Add `"quest_instances"` to `REQUIRED_TABLES` (bump the array length; a validation
test enforces the DDL matches).

### Events (client-facing)

`quest.accepted` / `quest.progress_updated` / `quest.completed` / `quest.failed`,
plus the cross-subsystem request events `quest.accept_requested` /
`quest.progress_requested` / `quest.complete_requested` / `quest.fail_requested`
(Rule 2). Notice structs (ts-rs `TS` derive) in `payloads/`; add the four result
events to `CLIENT_FACING_EVENT_TYPES` + the `ClientEvent` union + the export list;
`cargo xtask gen-types`; worldModel reducers + the coverage gate.

**Live wiring:** the request events are resolved in `GMController::
resolve_domain_events` by building an on-demand `QuestService`
(`QuestCatalog::load_default()` + `QuestRepository::new(db)` + the controller
clock) and calling the handlers — the same on-demand pattern HR-776 uses for
`status.*_requested` (the lifetime-bound `register_*_handlers` can't be wired at
construction). The `quests accept/abandon <id>` command emits the request events.

### REST + command

- `GET /api/character/:entity_id/quests` (`routes.rs` + `gameplay.rs` +
  `worldsvc.rs::quests_json`) — active + completed instances, mirroring the
  `status_effects` route.
- **`quests` command** (`parser.rs` verb alias + `exploration.rs` handler):
  `quests` lists active/completed with objectives + progress; `quests accept
  <id>` emits `quest.accept_requested`; `quests abandon <id>` emits
  `quest.fail_requested`.

### Frontend

- `types/events.gen.ts` (regenerated) + worldModel reducers for the four events
  updating `model.quests { active, completed }`; projection → a `questCount` on
  the game store; `StatusSidebar.vue` shows an **active-quest badge**.
- coverage spec `ALL_CLIENT_EVENT_TYPES` gains the four events.

### Content (example)

`content/xwn-core/content/quests/starter.yaml` — 1–2 example quests with
objectives, rewards, and one with a prerequisite, to drive tests.

## Testing

- **Rust unit:** every service method + transition (accept → in_progress →
  completed/failed), event emission (handlers return the right notices),
  prerequisite gating, duplicate-accept rejection, REST/`quests_json` shape.
- **Invariant tests:** status is always in `{accepted, in_progress, completed,
  failed}`; a completed/failed quest never appears in `list_active`;
  `list_active` and `list_completed` never overlap.
- **vitest:** the four reducers update `model.quests`; the sidebar badge.
- Gates: cargo core/web, clippy, ts-rs drift + coverage, vue-tsc, vitest, and a
  targeted exploration Playwright pass (the `quests` command path).

## Delivery

One PR (cohesive subsystem, like `status_effects`), closing #19. Implemented in
order: subsystem core (schema/catalog/models/repo/service) → events + handlers +
controller wiring → REST + command → frontend → tests.

## Out of scope (per issue)

Quest editor UI; procedural generation; prerequisite chains beyond simple id
lists; Adventure-Crafter integration; reward *processing* (the `quest.completed`
payload carries the reward; a granting handler is a later step); auto-progress
from gameplay events (objectives are advanced via `update_progress` / the future
faction+NPC hooks, tested directly here).
