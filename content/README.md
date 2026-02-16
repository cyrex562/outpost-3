# Outpost 3 — Content Definition Reference

This directory contains data-driven content definitions for **Outpost 3**. All buildings, resources, gameplay events, and technologies are defined in YAML files and loaded at runtime.

**Modding Support:** You can create your own YAML files or modify existing ones to add custom content. The game will validate all definitions on load.

---

## Directory Structure

```
content/
├── buildings/        # Building type definitions
├── resources/        # Resource type definitions
├── events/           # Narrative event definitions
├── tech/             # Technology tree nodes (Alpha phase)
└── README.md         # This file
```

---

## Buildings (`buildings/*.yaml`)

Buildings are constructed at sites and perform functions like power generation, resource extraction, production, housing, and storage.

### Schema

```yaml
- id: unique_building_id          # Required: kebab-case unique identifier
  name: Display Name               # Required: human-readable name
  description: Detailed description # Required: what this building does
  category: building_category      # Required: see categories below
  construction_cost:               # Required: resources needed to build
    - resource_id: steel
      amount: 100.0
  construction_time_ticks: 100     # Required: build duration in game ticks
  
  # Optional: Power rating (negative = consumes, positive = generates)
  power_requirement:
    rating_mw: -5.0                # Megawatts (negative for consumption)
  
  # Optional: Worker capacity
  worker_capacity: 10              # Max workers this building employs
  
  # Optional: Housing capacity
  housing_capacity: 20             # Max colonists this building houses
  
  # Optional: Storage capacity
  storage_capacity: 10000.0        # Storage volume in cubic meters
  
  # Optional: Production recipe (for factories, extractors, farms)
  recipe:
    inputs:                        # Resources consumed per cycle
      - resource_id: iron_ore
        amount_per_tick: 1.0
    outputs:                       # Resources produced per cycle
      - resource_id: steel
        amount_per_tick: 0.5
    cycle_time_ticks: 10           # How often the recipe runs
  
  # Optional: Morale bonus/penalty
  morale_bonus: 5                  # Integer bonus to colony morale
  
  # Optional: Upgrade path
  upgraded_to: advanced_building_id # ID of building this upgrades to
```

### Building Categories

- **power_generation**: Solar arrays, reactors, generators
- **resource_extraction**: Mines, water extractors, ice harvesters
- **manufacturing**: Smelters, electronics factories, refineries
- **life_support**: O2 generators, water recycling, climate control
- **agriculture**: Hydroponics, greenhouses, algae farms
- **housing**: Habitats, dormitories, luxury apartments
- **storage**: Warehouses, silos, propellant tanks
- **research**: Laboratories, observatories, data centers
- **transport**: Spaceports, landing pads, cargo hubs
- **defense**: Turrets, shields, bunkers (future)
- **governance**: Administration, security, medical
- **entertainment**: Recreation centers, parks, theaters
- **utility**: Generic multipurpose buildings

### Power Plant Example

```yaml
- id: solar_array_mk1
  name: Solar Array Mk I
  description: Photovoltaic panels that generate electricity from sunlight.
  category: power_generation
  construction_cost:
    - resource_id: steel
      amount: 50.0
    - resource_id: electronics
      amount: 20.0
  construction_time_ticks: 50
  power_requirement:
    rating_mw: 10.0                # Generates 10 MW
  worker_capacity: 2
  upgraded_to: solar_array_mk2
```

### Extractor Example

```yaml
- id: iron_mine
  name: Iron Mine
  description: Automated mining facility that extracts iron ore from deposits.
  category: resource_extraction
  construction_cost:
    - resource_id: steel
      amount: 100.0
  construction_time_ticks: 80
  power_requirement:
    rating_mw: -3.0                # Consumes 3 MW
  worker_capacity: 5
  recipe:
    inputs: []                     # Extractors have no inputs
    outputs:
      - resource_id: iron_ore
        amount_per_tick: 2.0
    cycle_time_ticks: 10
```

---

## Resources (`resources/*.yaml`)

Resources represent materials, commodities, life support elements, and abstract values like currency.

### Schema

```yaml
- id: unique_resource_id           # Required: kebab-case unique identifier
  name: Display Name                # Required: human-readable name
  description: Detailed description # Required: what this resource is
  category: resource_category       # Required: see categories below
  
  storage:                          # Required: physical storage requirements
    phase: solid                    # Required: solid|liquid|gas|plasma|virtual
    min_temperature_k: 273.0        # Optional: minimum storage temperature (Kelvin)
    max_temperature_k: 373.0        # Optional: maximum storage temperature (Kelvin)
    min_pressure_atm: 0.2           # Optional: minimum pressure (atmospheres)
    max_pressure_atm: 100.0         # Optional: maximum pressure (atmospheres)
    hazardous: false                # Optional: requires special handling
    special_handling: "Handle with care" # Optional: free-text notes
  
  density_kg_per_m3: 7850.0         # Required: kg per cubic meter (0 for virtual)
  base_value: 50.0                  # Required: economic value per unit
  tradeable: true                   # Required: can be bought/sold
  extractable: false                # Required: can be mined/harvested
  consumable: false                 # Required: consumed by colonists
  stack_size: 0                     # Required: max stack (0 = unlimited)
```

### Resource Categories (4 Tiers)

**Tier 0: Raw Materials**

- `metallic_ore`: Iron ore, copper ore, aluminum ore
- `hydrospheric`: Water, ice, brine
- `atmospheric_gas`: Oxygen, nitrogen, CO2, argon
- `organic_raw`: Biomass, wood, cellulose

**Tier 1: Processed Materials**

- `metal`: Steel, aluminum, titanium
- `chemical`: Acids, polymers, nutrients
- `fuel`: Hydrogen, methane, propellant
- `component`: Electronics, machinery, tools

**Tier 2: Advanced Products**

- `manufactured_good`: Consumer goods, equipment, vehicles
- `pharmaceutical`: Medicine, vaccines, supplements
- `biological_product`: Food, textiles, bio-plastics

**Tier 3: Special & Virtual**

- `data`: Research data, software, intelligence
- `currency`: Credits, bonds, IOUs
- `luxury`: Art, jewelry, rare collectibles
- `waste`: Scrap, slag, garbage

### Physical Resource Example

```yaml
- id: oxygen
  name: Oxygen
  description: O2 gas - essential for colonist respiration and life support systems.
  category: atmospheric_gas
  storage:
    phase: gas
    min_pressure_atm: 0.2
    max_pressure_atm: 100.0
    hazardous: false
    special_handling: "Oxidizer - keep away from flammables"
  density_kg_per_m3: 1.429
  base_value: 5.0
  tradeable: true
  extractable: true
  consumable: true
  stack_size: 0
```

### Virtual Resource Example

```yaml
- id: credits
  name: Credits
  description: Abstract currency representing economic value.
  category: currency
  storage:
    phase: virtual
    hazardous: false
  density_kg_per_m3: 0.0            # Virtual resources have zero density
  base_value: 1.0
  tradeable: true
  extractable: false
  consumable: false
  stack_size: 0
```

---

## Events (`events/*.yaml`)

Gameplay events are narrative moments that present player choices and trigger consequences.

### Schema

```yaml
- id: unique_event_id              # Required: kebab-case unique identifier
  title: Event Title                # Required: short headline
  description: Longer narrative text # Required: event story/context
  
  triggers:                         # Required: when this event can fire
    - trigger_type: building_constructed # See trigger types below
      building_id: basic_habitat
    - trigger_type: population_reached
      threshold: 100
  
  choices:                          # Required: player-selectable options
    - choice_id: accept_proposal
      label: "Accept the proposal"
      description: "Detailed description of this choice"
      cost:                         # Optional: cost to select this choice
        - resource_id: credits
          amount: 1000.0
      outcomes:                     # Required: what happens
        - outcome_type: morale_change
          amount: 10
        - outcome_type: resource_change
          resource_id: food
          amount: 50.0
  
  repeatable: false                 # Optional: can fire multiple times
  cooldown_ticks: 1000              # Optional: minimum ticks between repeats
  weight: 1.0                       # Optional: relative probability (for random triggers)
```

### Trigger Types

- `building_constructed`: Fires when specific building is built
  - Required field: `building_id`
- `population_reached`: Fires when population hits threshold
  - Required field: `threshold` (integer)
- `resource_threshold`: Fires when resource amount crosses threshold
  - Required fields: `resource_id`, `threshold` (float), `direction` ("above"|"below")
- `random`: Fires randomly with probability
  - Required field: `probability` (0.0-1.0)
- `time_based`: Fires after tick count
  - Required field: `tick` (integer)
- `morale_threshold`: Fires when morale crosses threshold
  - Required fields: `threshold` (integer), `direction` ("above"|"below")

### Outcome Types

- `morale_change`: Adjust colony morale by `amount` (integer)
- `resource_change`: Adjust resource stockpile by `amount` (float), specify `resource_id`
- `tech_unlock`: Unlock technology node, specify `tech_id`
- `building_unlock`: Enable building type, specify `building_id`
- `text_log`: Add log message, specify `message` (string)
- `spawn_event`: Trigger another event, specify `event_id`

### Example Event

```yaml
- id: morale_crisis
  title: Morale Crisis
  description: Colonists are on the verge of mutiny. Morale has dropped to critical levels.
  triggers:
    - trigger_type: morale_threshold
      threshold: 20
      direction: below
  choices:
    - choice_id: emergency_rations
      label: "Distribute Emergency Rations"
      description: "Use stored food to boost morale immediately."
      cost:
        - resource_id: food
          amount: 100.0
      outcomes:
        - outcome_type: morale_change
          amount: 20
    - choice_id: ignore_complaints
      label: "Ignore Complaints"
      description: "They'll get over it. Right?"
      outcomes:
        - outcome_type: morale_change
          amount: -10
  repeatable: true
  cooldown_ticks: 500
```

---

## Validation Rules

The `ContentLoader` validates all definitions on load. Common validation errors:

- **Empty IDs or names:** All definitions must have non-empty `id` and `name`/`title` fields.
- **Duplicate IDs:** Each ID must be unique within its category (buildings, resources, events).
- **Invalid density:** Physical resources must have positive density; virtual resources can have 0.0.
- **Negative values:** `base_value`, `construction_time_ticks`, etc. cannot be negative.
- **Missing required fields:** All fields marked "Required" in schemas above must be present.
- **Power plant validation:** Buildings with category `power_generation` must have positive `power_requirement.rating_mw`.
- **Recipe validation:** If a building has a recipe, it must have non-empty `outputs` and valid `cycle_time_ticks`.

---

## Loading Content (Developer Reference)

Content is loaded at server startup. The server reads YAML files and passes them as strings to `outpost-core`'s `ContentLoader`:

```rust
use outpost_core::content::{ContentLoader, BuildingDefinition};
use std::fs;

let mut loader = ContentLoader::new();

// Read YAML file
let yaml_content = fs::read_to_string("content/buildings/basic_buildings.yaml")?;

// Load and validate
loader.load_buildings(&yaml_content)?;

// Access loaded content
if let Some(building) = loader.get_building("iron_mine") {
    println!("Building: {}", building.name);
}
```

**Architecture Note:** `outpost-core` is I/O-free. The `ContentLoader` accepts YAML strings, not file paths. All file I/O happens in `outpost-server`.

---

## Modding Tips

1. **Start with examples:** Copy an existing YAML file and modify it.
2. **Use unique IDs:** Always use descriptive, unique IDs in kebab-case.
3. **Test incrementally:** Add one definition at a time and reload to catch validation errors.
4. **Check logs:** Validation errors are logged with specific messages about what's wrong.
5. **Balance carefully:** Adjust `construction_cost`, `power_requirement`, and `recipe` values to maintain game balance.
6. **Document your content:** Use `description` fields to explain mechanics clearly.

---

## Technologies (`tech/*.yaml`)

**(Placeholder for Alpha phase)** — Research tree nodes that unlock buildings, resources, bonuses, and new gameplay systems.

### Schema

```yaml
- id: unique_tech_id                # Required: kebab-case unique identifier
  name: Display Name                 # Required: human-readable name
  description: Detailed description  # Required: what this tech enables
  category: tech_category            # Required: see categories below
  tier: 2                            # Required: research tier (1 = basic, higher = advanced)
  research_cost: 1000.0              # Required: research points needed
  research_time_ticks: 500           # Required: research duration in game ticks
  
  prerequisites:                     # Optional: IDs of techs that must be researched first
    - prerequisite_tech_id
  
  unlocks:                           # Optional: what this tech unlocks
    buildings:                       # Building IDs that become available
      - advanced_building_id
    resources:                       # Resource IDs that become available
      - advanced_resource_id
    recipes:                         # Recipe IDs that become available
      - advanced_recipe_id
    bonuses:                         # Passive bonuses (string descriptors)
      - mining_efficiency_20_percent
    events:                          # Event IDs that become possible
      - new_event_id
  
  resource_costs:                    # Optional: material costs for research
    - resource_id: electronics
      amount: 50.0
```

### Tech Categories

- **engineering**: Construction, mining, manufacturing technologies
- **physical_sciences**: Physics, chemistry, materials science
- **life_sciences**: Biology, medicine, agriculture, genetics
- **computing**: Automation, AI, data processing
- **social**: Governance, culture, diplomacy
- **exploration**: Space travel, propulsion, sensors
- **military**: Defense, weapons, security (future)

### Tier 1 Example (No Prerequisites)

```yaml
- id: basic_construction
  name: Basic Construction
  description: Foundational construction techniques for simple buildings.
  category: engineering
  tier: 1
  research_cost: 100.0
  research_time_ticks: 50
  prerequisites: []
  unlocks:
    buildings:
      - basic_habitat
      - warehouse
    bonuses:
      - construction_speed_10_percent
```

### Advanced Tech Example (With Prerequisites and Costs)

```yaml
- id: fusion_basics
  name: Fusion Fundamentals
  description: Theoretical foundation for fusion power generation.
  category: physical_sciences
  tier: 3
  research_cost: 2000.0
  research_time_ticks: 800
  prerequisites:
    - improved_solar
    - advanced_materials
  unlocks:
    buildings:
      - fusion_reactor_prototype
  resource_costs:
    - resource_id: research_data
      amount: 100.0
```

---

## Future Additions (Alpha Phase)

- **Character templates** (`characters/*.yaml`): Procedural colonist personalities, traits, skills
- **Mission definitions** (`missions/*.yaml`): Multi-objective quest chains
- **Disaster definitions** (`disasters/*.yaml`): Catastrophic events with escalation mechanics

---

**Last Updated:** 2026-02-16  
**Outpost 3 Version:** v0.2.0 (MVP Phase 0 — Data-Driven Content System + Tech Tree Placeholder)
