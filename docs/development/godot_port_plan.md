## Godot Port Plan

**Goals**
- Provide a Godot 4 project scaffold in C# that mirrors the current Bevy simulation loop.
- Preserve data structures for solar-system simulation and turn-based updates.
- Keep the port lightweight for rapid prototyping while leaving room for future Rust interop.

**High-Level Architecture**
- Main.tscn root scene with an autoload GameRoot.cs script that bootstraps services.
- Services
  - GameStateService: wraps global simulation state and tick/update methods.
  - SolarSystemService: loads CSV data, computes orbital mechanics, tracks positions.
  - SimulationService: maintains turn counter and higher-level economic/process layers (future expansion).
- Data
  - Plain C# record structs/classes that correspond to Rust structs: CelestialBody, OrbitalParameters, OrbitalState, etc.
  - Serialization via System.Text.Json with custom converters for DateTime parsing.
- UI
  - Minimal scene for date/turn display. Full UI systems will be implemented later with Godot's Control nodes.

**Mapping from Bevy Modules**
- esources::game_state::GameState -> GameStateService singleton (autoload).
- esources::solar_system::SolarSystemManager -> SolarSystemService C# class. Loading remains CSV-based using Godot.FileAccess.
- components::orbital_mechanics -> Static helper module OrbitalMath plus DTOs for current position (update method per turn).
- systems::simulation::process_turn -> SimulationService.AdvanceTurn (increment counter, call SolarSystemService.UpdatePositions).
- systems::ui -> replaced with C# UI scripts leveraging Godot scene tree.

**Key Adaptations**
- Replace Bevy ECS resources with service singletons managed via autoloads.
- Use double precision floats in C# to match Rust 64 values.
- Represent NaiveDate with UTC DateTime.
- Convert Bevy event loop into Godot's _Process or manual turn advancement to avoid real-time drift initially.
- Logging will use GD.Print for now.

**Open Questions**
- Keep CSV in es://data for parity vs migrating to JSON.
- Long-term Rust interop via GDNative is out of scope for the first pass.
