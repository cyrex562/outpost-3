# CLAUDE.md — Outpost 3 AI Assistant Guide

**For:** AI coding assistants (Claude, Copilot, Gemini, Cursor, etc.)
**Project:** Outpost 3 — turn-based isometric colony survival and grand strategy game
**Engine:** Godot 4 + C#
**Design doc:** `docs/Outpost3_Design_V5.md` — authoritative game design
**Task list:** `docs/TODO.md` — master task list, current priorities

---

## You Are a Godot 4 / C# Game Developer

You write idiomatic C# targeting .NET 8. You keep simulation logic in pure C# classes with zero Godot dependencies. You iterate until **all NUnit tests pass** before considering any task complete.

---

## Technology Stack

| Layer | Technology | Notes |
|---|---|---|
| **Engine** | Godot 4.3 + C# | `.NET 8`, `Nullable enable` |
| **Core Logic** | Pure C# (`godot/src/Core/`) | No Godot namespace — plain classes, no nodes |
| **Rendering** | Godot nodes (`godot/src/Rendering/`) | `Node2D`, `_Draw()`, signals |
| **UI** | Godot nodes (`godot/src/UI/`) | Control nodes, panels, labels |
| **Content** | YAML / JSON / CSV files (`content/`) | Loaded at runtime; buildings, resources, events, tech |
| **Testing** | NUnit 4 (`tests/OutpostCore.Tests/`) | Pure C# tests, no Godot runtime needed |
| **Serialization** | `System.Text.Json` | Save/load, content loading |

---

## Project Structure

```
outpost-3/
├── CLAUDE.md                       # This file
├── docs/
│   ├── Outpost3_Design_V5.md       # Game design — source of truth
│   └── TODO.md                     # Master task list
├── content/                        # Data files loaded at runtime
│   ├── buildings/                  # Building definitions (YAML)
│   ├── resources/                  # Resource definitions (YAML)
│   ├── events/                     # Event definitions (YAML)
│   └── tech/                       # Tech tree (YAML)
├── data/                           # Reference data (solar system, etc.)
├── godot/                          # Godot 4 project root
│   ├── project.godot
│   ├── OutpostGame.csproj
│   ├── src/
│   │   ├── Core/                   # Pure C# — ZERO Godot dependencies
│   │   │   ├── Colony/             # Building, resource, population, power, labor
│   │   │   ├── Simulation/         # Turn processor, session, save/load, difficulty
│   │   │   └── World/              # Terrain generation, site definitions, biomes
│   │   ├── Rendering/              # Godot rendering layer (ColonyGridView, etc.)
│   │   ├── UI/                     # Godot UI nodes
│   │   └── Game/                   # Scene controllers, autoloads
│   ├── scenes/                     # Godot .tscn scene files
│   └── tests/                      # Godot-side tests (GDUnit4 — TBD)
└── tests/
    └── OutpostCore.Tests/          # NUnit tests for Core logic (163 tests, all passing)
        ├── OutpostCore.Tests.csproj
        ├── Phase0Tests.cs
        ├── Phase1Tests.cs
        ...
        └── Phase8Tests.cs
```

---

## Critical Rule: `godot/src/Core/` Has Zero Godot Dependencies

Files under `godot/src/Core/` must **never** reference:
- `Godot` namespace (`using Godot;`)
- `Node`, `Resource`, `GodotObject`, or any Godot type
- `GD.Print`, `GD.Randomize`, or any Godot global
- `Vector2`, `Vector2I`, `Color` — use `GridPosition`, `GridSize` plain structs instead

All Godot-specific code belongs in `Rendering/`, `UI/`, or `Game/`. Core ↔ Godot boundary is crossed only at the rendering/UI layer.

---

## Running Tests

```powershell
# Run all NUnit tests (the primary test suite)
dotnet test tests/OutpostCore.Tests/OutpostCore.Tests.csproj

# Run with verbose output
dotnet test tests/OutpostCore.Tests/OutpostCore.Tests.csproj --logger "console;verbosity=normal"
```

All 163 tests must stay green. Never submit work with failing tests.

---

## What Is Currently Implemented

### Core simulation (all tested, headless)
- `ColonyState` — grid, resources, population, labor, power, event log
- `ColonyGrid` — multi-size building placement, occupancy validation
- `ResourceStore` — add/consume/cap, snapshot/restore
- `LaborPool` — allocation, skill-based efficiency (1.0 / 0.8 / 0.65 modifiers)
- `PowerGrid` — producers/consumers, brownout, essential priority
- `PopulationGroup` — needs satisfaction, health/morale deltas, deaths, growth
- `ColonyTurnProcessor` — construction, production, consumption, population needs, events, power
- `ColonySession` — high-level API (QueueConstruction, EndTurn, AutoAssignLabor, CreateSave, ApplySave)
- `DifficultySettings` — 5 presets (Sandbox→Brutal): resource multipliers, consumption multipliers, event intervals
- `DifficultyPreset` — wired into session init, turn processor, random events
- `SkillType` — 6 skills (Laborer, Engineer, Scientist, Farmer, Medic, Operator)
- `TerrainGenerator` — biome-aware two-pass terrain generation
- `RandomEventProcessor` — strategic events (dust storms, equipment failure, supply drops, arrivals)
- Save/load — full JSON round-trip via `ColonySaveData`

### Registries (JSON-driven via ContentLoader)
- `BuildingRegistry` — 13 buildings with PrimarySkill, recipes, power, labor
  loaded from `EmbeddedContent.BuildingsJson` (mirror at `content/buildings.json`)
- `ResourceRegistry` — 26 resources across Raw/Refined/Advanced/Virtual tiers
  loaded from `EmbeddedContent.ResourcesJson` (mirror at `content/resources.json`)
- Phase 3.2 task: swap embedded JSON for runtime file loading.

### Godot rendering (Phase 1)
- `ColonyGridView` — isometric grid rendering, terrain colors per biome, building overlays
- Camera system, building placement ghost, basic UI stubs

---

## Content Data Files

Buildings and resources flow through `ContentLoader` in `godot/src/Core/Content/`.
There are two sources, in priority order at game runtime:

1. **`user://content/<name>.json`** — per-user mod overrides (Godot writes user-data
   here; modders drop files in to replace any individual file).
2. **`res://content/<name>.json`** — bundled with the game (the canonical
   shipping files at `godot/content/buildings.json` + `godot/content/resources.json`).
3. **`EmbeddedContent.cs` constants** — compiled-in fallback that keeps tests and
   any environment without file access working.

`ContentBootstrap.EnsureLoaded()` is called from every scene's `_Ready`; it reads the
files via `Godot.FileAccess` and hands the strings to `BuildingRegistry.LoadFrom` /
`ResourceRegistry.LoadFrom`. If the file is missing or malformed the embedded defaults
remain in effect (validation runs before the registry is mutated).

To add or modify a building/resource:

1. Edit `godot/content/<file>.json` for the runtime change.
2. Mirror the change in `godot/src/Core/Content/EmbeddedContent.cs` so tests and the
   embedded fallback stay in sync.
3. Add a test in `ContentLoaderTests.cs` if it's a new ID.
4. Run `dotnet test` — registries reload the JSON on every static init.

JSON schema rules:
- String IDs (snake_case), unique within their file.
- Enum fields (`category`, `tier`, `primarySkill`) take the C# enum name as a string.
- Optional fields can be omitted — `ContentLoader` applies sensible defaults.
- Duplicate IDs, invalid enums, and missing required fields throw
  `ContentLoader.ContentValidationException` at load.

---

## Coding Rules

1. **Read `docs/Outpost3_Design_V5.md`** before starting new features
2. **Check `docs/TODO.md`** to pick up the right next task
3. **No Godot namespaces in `Core/`** — enforce this strictly
4. **No `null!` or `!` null-forgiving** without a comment explaining why it's safe
5. **Write tests alongside code** — every new Core class gets NUnit coverage
6. **Iterate until tests pass** — `dotnet test` must be green before stopping
7. **No premature abstraction** — implement what the current task needs, nothing more
8. **No comments that describe what the code does** — only comments explaining non-obvious *why*
9. **Content comes from data files, not hardcoded values** — new buildings/resources go in `content/`
10. **Keep changes minimal and focused** — one feature at a time

---

## Common Tasks

### Adding a new building
1. Add YAML entry to `content/buildings/` (or temporarily to `BuildingRegistry.cs` while loader is not yet built)
2. Assign `PrimarySkill`, power values, labor, recipe, construction cost and turns
3. Add NUnit test asserting the definition loads and key fields are correct

### Adding a new resource
1. Add YAML entry to `content/resources/` (or `ResourceRegistry.cs`)
2. Specify tier, category, base weight
3. Update any buildings that produce or consume it

### Adding a Core system
1. Create class in appropriate `Core/` subdirectory
2. No Godot references — pure C#
3. Expose via `ColonyState` if needed
4. Wire into `ColonyTurnProcessor.ProcessTurn()` if turn-driven
5. Add to `ColonySaveData` if state must persist
6. Write NUnit tests

### Adding a Godot scene/UI
1. Create scene in `godot/scenes/`
2. Create C# script in `godot/src/UI/` or `godot/src/Rendering/`
3. Access Core state only through `ColonySession` — never reach into Core internals from UI
4. Emit/handle Godot signals at the scene boundary

---

## Git Workflow

- Branch from `main` for each feature
- Commit message style: `Phase N: brief description` or `Fix: brief description`
- Tests must pass before merging
- The worktree at `.claude/worktrees/` is used by Claude Code for isolated work
