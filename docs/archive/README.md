# Component Documentation

This directory contains detailed documentation for each major component of the Harsh Realm game engine.

## Core Components

### Simulation
- [Simulation System](simulation.md) - Core simulation mechanics and turn processing
- [Event System](events.md) - Event handling and processing

### Game State
- [Game State](game_state.md) - Main game state management
- [State Management](state_management.md) - State handling and persistence

### World Systems
- [Universe](universe.md) - Universe generation and management
- [Maps](maps.md) - Strategic map systems
- [Celestial Bodies](celestial_bodies.md) - Planet and space object management

### Gameplay Systems
- [Factions](factions.md) - Faction management and relationships
- [Population](population.md) - Population dynamics and management
- [Resources](resources.md) - Resource systems and economy
- [Buildings](buildings.md) - Building and infrastructure systems
- [Units](units.md) - Unit management and control

### Technical Systems
- [Procedural Generation](procedural_generation.md) - World and content generation
- [Production](production.md) - Production chains and manufacturing

## Component Relationships
Each component is designed to be modular but interconnected. The documentation for each component includes:
- Purpose and responsibilities
- Public API
- Dependencies
- Interaction with other components
- Implementation details
- Future improvements

## Development Guidelines
When working with components:
1. Maintain clear interfaces between components
2. Document all public APIs
3. Keep components focused on their core responsibilities
4. Use events for cross-component communication
5. Follow the established architectural patterns 