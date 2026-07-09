# Reference Architecture: Harsh Realm

This directory contains a copy of the **Harsh Realm** project — a Rust + Vue single-player MUD with an expert-system GM.

Harsh Realm uses the same architectural patterns that Outpost 3 is borrowing:

- Pure Rust headless core library (`crates/harsh-core/`)
- Axum web host (`crates/harsh-web/`)
- Vue 3 + Pinia frontend (`frontend/`)
- Content packs (`content/`)
- Typed server→client event contract
- Renderer-agnostic world model + reducer/projection

## How to Use

**Read for structural patterns. Do NOT copy game logic.** Harsh Realm is a different game domain (MUD/RPG with combat, dungeon generation, GM narration). What to borrow:

| Borrow | Don't borrow |
|---|---|
| Crate layout and module organization | SQLite-as-live-state for the turn loop |
| Pure headless core pattern | Per-mutation write-through repositories |
| Typed event contract (server→client) | RPG-specific subsystems (combat, dungeon gen) |
| Content pack structure | GM narration engine |
| Grade/difficulty table pattern | Any domain-specific logic |

## Source

The `harsh_realm-main/` directory was provided as a reference archive. It is checked in as read-only reference material and should not be modified.
