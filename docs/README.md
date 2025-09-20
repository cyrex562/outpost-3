# Harsh Realm Documentation

## Overview
Harsh Realm is a long-term simulation-driven 4X project that now spans two codebases:
- **Rust/Bevy sandbox** (src/): the original ECS-based simulation prototype.
- **Godot 4 C# prototype** (godot/): a rapid-prototyping environment focused on UI iteration and gameplay experiments.

Use this documentation hub to keep both tracks aligned while the simulation depth grows.

## Documentation Map

### Simulation & Mechanics
- [Game Mechanics Overview](game_mechanics.md)
- [Detailed Mechanics Notes](game_mechanics/current_mechanics.md)
- [Faction Structures](faction_structures.md)

### Technical Design
- [Component Reference](components/README.md)
- [Development Plan (Rust)](development/development_plan.md)
- [Godot Port Notes](development/godot_port_plan.md)

### Project Direction
- [Roadmap](roadmap.md) – staged milestones for both engines.
- [Todo / Backlog](todo.md) – actionable, short-term tasks.

## Getting Started

### Rust / Bevy build
`ash
cargo run
`
The Bevy project remains the simulation reference. Keep it compiling even while primary feature work shifts to Godot.

### Godot 4 / C# prototype
1. Open godot/project.godot with Godot 4.2+.
2. Ensure the .NET SDK and Godot C# support are installed (the .csproj targets .NET 6).
3. Run the scene es://scenes/Main.tscn to watch the turn-by-turn orbital updates print to the console.

## Collaboration Notes
- Simulation logic should live in pure C# (or Rust) services so it can be shared across UI layers later.
- When adding new mechanics, update both the [roadmap](roadmap.md) and the [todo list](todo.md) so priorities stay visible.
- Record major decisions or engine-specific quirks in docs/development/ alongside the existing plans.
