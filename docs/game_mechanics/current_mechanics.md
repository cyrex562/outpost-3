# Current Game Mechanics

## Core Systems

### Turn-Based Simulation
- Implemented basic turn processing system
- Each turn represents a discrete time unit
- Turn processing order is defined but needs implementation
- Logging system in place for turn tracking

### Structure Types
1. Settlements
   - Fixed locations with permanent population
   - Multiple building types (Mine, Refinery, Factory, Laboratory)
   - Population management
   - Resource storage and production
   - Building queues (planned)

2. Installations
   - Specialized facilities with minimal/no permanent population
   - Types: Mine, Refinery, Factory, Military, Research
   - Crew-based operation
   - Resource production focused

3. Spacecraft
   - Mobile units with various capabilities
   - Module-based design (Mine, Refinery, Factory, Laboratory, Military, Administrative)
   - Cargo capacity
   - Population support (optional)
   - Fleet formation support
   - Location tracking system

### Resource System
- Basic resource types defined:
  - Extracted: Ice, Minerals, Gases, Hydrocarbons, Organics
  - Refined: Water, Air, Metal, NonMetal, Energy, Food, BioMatter
  - Future: Waste
- Resource storage implemented
- Production chains planned

### Location System
- Surface locations (hex-based)
- Orbital locations
- Deep space locations
- Docking system

### Faction System
- Basic faction structure
- Faction AI framework (placeholder)
- Faction relationships (planned)

## Implementation Status

### Implemented
- Basic turn processing
- Structure type definitions
- Resource type definitions
- Location system
- Basic faction framework
- Logging system

### In Progress
- Resource production chains
- Population mechanics
- Building systems
- Faction interactions

### Planned
- Combat system
- Research and technology
- Advanced diplomacy
- Trade networks
- Cultural systems

## Technical Notes
- Using Rust with serde for serialization
- UUID-based entity identification
- Hex-based map system
- Event-driven architecture (planned) 