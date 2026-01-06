---
applyTo: "**"
---

# Copilot Workspace Instructions

You are an **assistant coder** for **Outpost 3: Wormhole Empire**, a Rust-based web simulation game with a **core Rust library** + **event-sourced architecture** + **multi-platform clients** (Bevy desktop/web, legacy Actix server).

## Golden Rules (Event Sourcing)

- **All state changes are Events**. Never mutate state directly—generate events instead.
- **Commands validate and produce Events**; reducers replay events deterministically.
- **Domain logic is pure**: `(State, Command) -> (State', Events[])`.
- **Time is explicit**: pass ticks/params in commands, never use `Utc::now()` in core.
- **Projections are caches**: read models can be rebuilt anytime; they're not source of truth.
- **Strong typing**: use newtype patterns for IDs/Resources; avoid `.unwrap()` and `.expect()`.

## Project Structure

- **Core domain** (platform-agnostic): `crates/outpost-core/src/**` – entities, events, commands, logic
- **Legacy server** (Actix + SQLite): `src/**` with features enabled via `cargo run --features server`
- **Bevy client** (desktop/wasm): `crates/outpost-client/src/**`
- **Tests**: `tests/*.rs` (integration) + `crates/*/tests/**` (unit)
- **Docs** (read first): `docs/*.md` and `DESIGN.md` (game design), `CLAUDE_RUST.md` (Rust patterns)

## Key Architecture Patterns

### Event Sourcing

```rust
// Events: immutable, past-tense, tagged serialization
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EventType {
    ColonyFounded { colony_id: ColonyId, starting_resources: Resources },
    ResourcesGathered { colony_id: ColonyId, amount: u64 },
}

// Commands: present-tense, validate → generate events
pub trait Command {
    type Event;
    fn execute(&self, state: &State) -> Result<Vec<Self::Event>, Error>;
}
```

### Domain Types (Newtype Pattern)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ColonyId(pub u64);  // Never use raw u64 for IDs

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Credits(pub i64);   // Strongly-typed amounts
```

## Critical Workflows

### Running the Project

- **Bevy client (desktop)**: `cargo run -p outpost-client` (default `desktop` feature)
- **Bevy client (WASM)**: `cargo run -p outpost-client --target wasm32-unknown-unknown --no-default-features --features wasm`
- **Legacy server**: `cargo run --features server` (runs Actix on `http://127.0.0.1:8080`)
- **Tests**: `cargo test` (all workspaces); `cargo test -p outpost-core` (core only)

### Adding a Feature (Standard Flow)

1. **Define Event** in `crates/outpost-core/src/events/` – past-tense, versioned
2. **Define Command** in `crates/outpost-core/src/commands/` – validates, returns `Vec<Event>`
3. **Update State Reducer** in `crates/outpost-core/src/domain/` – replay events deterministically
4. **Write tests** proving command → event → state replay is deterministic
5. **Update projection** (read model) if UI needs this data differently
6. **Run `cargo test`** before commit

### File Organization by Concern

| Layer           | Location                       | Purpose                                                       |
| --------------- | ------------------------------ | ------------------------------------------------------------- |
| **Domain**      | `crates/outpost-core/src/`     | Pure logic (no I/O), events, commands, entities               |
| **Clients**     | `crates/outpost-client/src/`   | Bevy ECS systems, rendering, input handling                   |
| **Tests**       | `tests/` and `crates/*/tests/` | Integration and unit tests; use `proptest` for property tests |
| **Persistence** | `src/db/` (legacy)             | SQLite schema, migrations, event store                        |

## Conventions & Anti-Patterns

### ✅ Do

- Use **error propagation** (`?` operator); return `Result<T, Error>`
- **Name events past-tense**: `ResourcesGathered`, `BuildingConstructed`, not `GatherResources`
- **Design small commands**; avoid batching multiple domain changes in one command
- **Test determinism**: "replay events = same state" is a core property
- **Use enums** for variants; avoid magic numbers

### ❌ Don't

- **No `.unwrap()` or `.expect()`** in domain/production code
- **No I/O in domain logic** (no database queries, file reads, network calls in `commands/` or reducers)
- **No timestamp logic in core** – pass time as parameter (tick count)
- **No circular dependencies** between modules
- **Never mutate projections from commands** – they're read-only caches

## Cross-Platform Considerations

- **Core logic** (`outpost-core`) is platform-agnostic; all tests must pass everywhere
- **Bevy client** uses ECS for rendering and input; keep UI logic separate from domain
- **Legacy server** is single-threaded Actix; SQLite for persistence
- **WASM** uses IndexedDB; test both paths if modifying persistence

## Before Generating Code

- Read [DESIGN.md](DESIGN.md) for game systems (economy, trains, wormholes)
- Check [CLAUDE_RUST.md](CLAUDE_RUST.md) for Rust patterns and error handling
- Verify workspace structure: `cargo metadata --format-version 1 | jq '.workspace_members'`
- Ensure `cargo check` and `cargo test` pass locally
