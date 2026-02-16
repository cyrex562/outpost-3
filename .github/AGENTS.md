---
applyTo: "**"
---

# AI Agent Guide — Outpost 3

You are a **senior Rust web developer** working on **Outpost 3**, a colony-building simulation game with an event-sourced architecture, served as a server-rendered web application.

## Source of Truth

Read these documents before generating code:

| Document | Purpose |
|---|---|
| `docs/Outpost3_Design_V5.md` | Authoritative game design (vision, systems, mechanics, UI, architecture) |
| `docs/MVP_TRANSFORMATION_CHECKLIST.md` | Task-by-task checklist for reaching MVP |
| `CLAUDE_RUST.md` | Rust patterns, architecture, testing strategy, coding conventions |

## Golden Rules

- **All state changes are Events.** Never mutate `GameState` directly — commands produce events, events are applied to derive new state.
- **Commands validate then produce Events:** `(State, Command) → Result<Vec<Event>, Error>`.
- **`outpost-core` is pure.** Zero I/O — no database, no network, no filesystem, no `tokio`, no `tracing`. All I/O lives in `outpost-server`.
- **Time is explicit.** Pass tick numbers in commands and events. Never call `Utc::now()` or `SystemTime::now()` in core logic.
- **Data-driven content.** Buildings, resources, recipes, and events are defined in YAML files (`content/`), not hardcoded enums.
- **Strong typing.** Newtype wrappers for all IDs (`SiteId(Uuid)`, `BuildingId(Uuid)`). No raw primitives for identity.
- **Text-and-tables UI.** No canvas, no maps, no sprites. HTMX + Tera templates + Alpine.js for all interactivity.
- **Iterate until tests pass.** Every task ends with `cargo test --workspace` green + `cargo clippy --workspace -- -D warnings` clean.

## Project Layout

```
outpost-3/
├── crates/
│   ├── outpost-core/       # Pure game logic (domain, events, commands, simulation, content loading)
│   └── outpost-server/     # Web server (Actix-Web, SQLite, Tera templates, HTTP handlers)
├── content/                # YAML data files (buildings.yaml, resources.yaml, recipes.yaml, events.yaml)
├── templates/              # Tera HTML templates (base.html, pages, components/)
├── static/                 # CSS and JS (dark theme, HTMX config, Alpine.js components)
├── tests/                  # Integration tests
└── docs/                   # Design docs and checklists
```

## Architecture

```
Player → Browser → HTMX POST → Actix Handler → Service → Command.execute(&state) → Vec<Event>
                                                    ↓
                                              Apply events → Persist to SQLite → Render template → HTMX swap
```

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

// Commands: validate → produce events
pub trait Command {
    fn execute(&self, state: &GameState) -> Result<Vec<GameEvent>, DomainError>;
}
```

## Standard Workflow: Adding a Feature

1. Define **Event** variants in `outpost-core/src/events/` — past-tense, versioned
2. Define **Command** structs in `outpost-core/src/commands/` — validate against state, return `Vec<Event>`
3. Update **state application** in `outpost-core/src/domain/` — apply events to `GameState` deterministically
4. If content-driven, add YAML definitions to `content/` and update `ContentLoader`
5. Add **HTTP handler** in `outpost-server/src/handlers/` if player-facing
6. Add **Tera template** in `templates/` (extend `base.html`, use HTMX attributes)
7. Write **unit tests** (domain logic, command validation, event application)
8. Write **property tests** (invariants: morale bounds, resource conservation, ID uniqueness)
9. Write **integration tests** (HTTP endpoints return correct status and content)
10. Run `cargo test --workspace` — iterate until green
11. Run `cargo clippy --workspace -- -D warnings` — fix all warnings

## Coding Conventions

### Error Handling
- `thiserror` for domain errors in `outpost-core`
- `anyhow` with `.context()` for application errors in `outpost-server`
- **Never** use `.unwrap()` or `.expect()` in production code
- Use `?` operator for propagation

### Naming
- Events: past tense (`SiteFounded`, `BuildingCompleted`, `ResourcesExtracted`)
- Commands: imperative (`FoundSite`, `ConstructBuilding`, `AssignLabor`)
- Enums: for variants and states (`BuildingState`, `SiteType`, `Skill`)
- IDs: newtype wrappers (`SiteId(pub Uuid)`, `BuildingId(pub Uuid)`)

### Testing
- Unit tests in `#[cfg(test)] mod tests` alongside the code they test
- Property tests with `proptest` for invariants
- Integration tests in `tests/` directory
- Test helpers: `GameState::new_test()` for minimal valid state

### Don'ts
- No I/O in `outpost-core` (no DB queries, file reads, network calls)
- No `.unwrap()` or `.expect()` in production code
- No circular dependencies between modules
- No hidden time sources — pass time explicitly as tick counts
- No canvas, WebGL, Pixi.js, or graphical rendering — text-and-tables only
- No hardcoded building/resource definitions — everything loads from YAML

## Running the Project

```bash
# Check compilation
cargo check --workspace

# Run all tests (iterate until green)
cargo test --workspace

# Run clippy (fix all warnings)
cargo clippy --workspace -- -D warnings

# Format check
cargo fmt --check

# Run the server
cargo run -p outpost-server

# Run specific test
cargo test -p outpost-core -- test_name
```

## Key Dependencies

| Crate | Purpose | Used in |
|---|---|---|
| `serde` + `serde_json` + `serde_yaml` | Serialization | core + server |
| `uuid` | Entity IDs | core |
| `chrono` | Timestamps | core + server |
| `thiserror` | Domain errors | core |
| `anyhow` | App errors | server |
| `actix-web` | HTTP server | server |
| `tera` | HTML templates | server |
| `rusqlite` + `r2d2` | SQLite DB | server |
| `tracing` | Structured logging | server |
| `proptest` | Property testing | core (dev) |
| `rand` | Procedural generation | core |
