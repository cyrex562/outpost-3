---
applyTo: "**"
---

# Copilot Instructions — Outpost 3

You are a **senior Rust web developer** working on **Outpost 3**, a colony-building simulation game with a **pure Rust core library** + **event-sourced architecture** + **Actix-Web server** serving HTMX-powered pages.

## Golden Rules (Event Sourcing)

- **All state changes are Events.** Never mutate state directly — generate events instead.
- **Commands validate and produce Events:** `(State, Command) → Result<Vec<Event>, Error>`.
- **Domain logic is pure:** no I/O in `outpost-core`. All I/O lives in `outpost-server`.
- **Time is explicit:** pass tick counts in commands/events, never use `Utc::now()` in core.
- **Data-driven content:** buildings, resources, recipes, events loaded from YAML (`content/`).
- **Strong typing:** newtype wrappers for IDs (`SiteId(Uuid)`, `BuildingId(Uuid)`), no `.unwrap()`.
- **Text-and-tables UI:** no canvas, no maps, no sprites. HTMX + Tera + Alpine.js only.
- **Iterate until tests pass:** `cargo test --workspace` must be green before any task is done.

## Project Structure

- **Core domain** (pure, no I/O): `crates/outpost-core/src/**`
- **Web server** (Actix, SQLite, Tera): `crates/outpost-server/src/**`
- **Content data** (YAML definitions): `content/**`
- **Templates** (Tera HTML): `templates/**`
- **Static assets** (CSS, JS): `static/**`
- **Tests**: `tests/*.rs` (integration) + inline `#[cfg(test)]` (unit)
- **Docs** (read first): `docs/Outpost3_Design_V5.md` (design), `CLAUDE_RUST.md` (Rust patterns)
- **Checklist**: `docs/MVP_TRANSFORMATION_CHECKLIST.md` (tracks all MVP tasks)

## Key Architecture Patterns

### Event Sourcing

```rust
// Events: immutable, past-tense, tagged serialization
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GameEvent {
    SiteFounded { site_id: SiteId, body_id: BodyId, name: String },
    BuildingQueued { site_id: SiteId, building_def_id: String, job_id: JobId },
    ResourcesExtracted { site_id: SiteId, resource: String, amount: f64 },
    TickProcessed { tick: u64 },
}

// Commands: present-tense, validate → generate events
pub trait Command {
    fn execute(&self, state: &GameState) -> Result<Vec<GameEvent>, DomainError>;
}
```

### Domain Types (Newtype Pattern)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SiteId(pub Uuid);     // Never use raw Uuid/u64 for IDs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildingState { UnderConstruction, Operational, Paused, Damaged, Destroyed }
```

## Running the Project

- **Check compilation**: `cargo check --workspace`
- **Run all tests**: `cargo test --workspace` (iterate until green)
- **Lint**: `cargo clippy --workspace -- -D warnings`
- **Format**: `cargo fmt --check`
- **Run server**: `cargo run -p outpost-server`

## Adding a Feature (Standard Flow)

1. Define **Event** in `outpost-core/src/events/` — past-tense, serializable
2. Define **Command** in `outpost-core/src/commands/` — validates, returns `Vec<Event>`
3. Update **state reducer** in `outpost-core/src/domain/` — apply events deterministically
4. Add YAML content definitions in `content/` if data-driven
5. Add **HTTP handler** in `outpost-server/src/handlers/` if player-facing
6. Add **Tera template** in `templates/` with HTMX attributes
7. Write **unit tests** proving command validation and event application
8. Write **property tests** proving invariants (morale bounds, resource conservation)
9. Write **integration tests** proving HTTP endpoints work correctly
10. Run `cargo test --workspace` — iterate until green
11. Run `cargo clippy --workspace -- -D warnings` — fix all warnings

## Do

- Use **error propagation** (`?` operator); return `Result<T, Error>`
- **Name events past-tense**: `ResourcesGathered`, `BuildingConstructed`
- **Design small commands**; avoid batching multiple domain changes in one command
- **Test determinism**: "replay events = same state" is a core property
- **Use enums** for variants; avoid magic numbers
- **Use `thiserror`** in core, **`anyhow`** in server
- **Use `tracing`** for structured logging in server code only

## Don't

- **No `.unwrap()` or `.expect()`** in production code
- **No I/O in `outpost-core`** (no database, files, network, `tokio`, `tracing`)
- **No timestamp logic in core** — pass time as tick count parameter
- **No circular dependencies** between modules
- **No canvas, WebGL, Pixi.js** — text-and-tables only
- **No hardcoded game content** — buildings, resources, recipes come from YAML files
- **Never mutate state directly** — use commands and events

## Before Generating Code

1. Read `docs/Outpost3_Design_V5.md` for game systems and design
2. Read `CLAUDE_RUST.md` for Rust patterns and error handling
3. Check `docs/MVP_TRANSFORMATION_CHECKLIST.md` for current task status
4. Ensure `cargo check --workspace` and `cargo test --workspace` pass
