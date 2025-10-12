---
applyTo: "**"
---

# Copilot Workspace Instructions

You are an **assistant coder** for a Godot 4 + C# (net8) game that uses a **pure, engine-agnostic Core** + **event-sourced architecture**.

## Golden Rules
- **Never** mutate game state from UI code. Only emit **Commands**.
- All reducers/systems are **pure**: `(State, Command|Tick) -> (State', Events[])`.
- All changes are **Events**. State is derived by replaying events (+ snapshots).
- **Deterministic time** only (ticks/params in commands). No `DateTime.UtcNow` in Core.
- Use **C# records**, avoid `null` in domain (options/guards).
- **Projections** (read models) can be rebuilt anytime; treat them as caches.

## Project Layout (authoritative)
- Core domain & reducers: `src/Game.Core/**`
- App/single-writer + stores + projections engine: `src/Game.App/**`
- Persistence adapters (JSONL/SQLite): `src/Game.Persistence/**`
- Tests: `src/Game.Tests/**`
- Godot UI adapter (no business logic): `GodotProject/scripts/**`
- Content packs (tech tree, buildings, events): `content/**`

## Source of Truth (read these before generating code)
- Concept & phases: `docs/01_game_concept_rev2.md`
- Screens & flows: `docs/02_screens.md`
- Mechanics & systems: `docs/03_game_mechanics.md`
- Entities & relationships: `docs/04_Entities.md`
- Architecture & structure: `docs/05_architecture.md`
- Implementation plan & stories: `docs/06_implementation_plan_1.md`
- Roadmap: `docs/07_roadmap.md`

## Coding Style
- Target `net8.0`, `LangVersion` 12; treat nullable warnings as errors.
- Use **small value types** for IDs/Amounts; prefer **pure static calculators** for rules.
- Tests: xUnit (primary) + GdUnit4 (for Godot-specific features).
- Follow AAA pattern: Arrange, Act, Assert.

## What to generate when asked
- **If feature touches state**: add Command + Event + Reducer change + Tests + Projection delta.
- **If UI behavior**: Presenter that subscribes to a Projection and emits Commands via CommandBus.
- **If persistence**: append-only `EventStore` changes with replayable semantics.
- **If new test**: Use xUnit for pure domain logic; GdUnit4 for Godot features.

## PR Expectations (keep changesets small)
1) Add/extend Commands, Events, Reducers.
2) Write tests that prove determinism and replay = state (use xUnit).
3) Update Projections and (if applicable) ViewModels.
4) Keep Godot scripts thin (presenters only).
5) Run `dotnet test` before committing.

## Don’ts
- No direct state edits in UI or projections.
- No circular dependencies between systems.
- No hidden time sources; pass time explicitly.

