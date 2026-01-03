# Milestone 7 Detailed Implementation Plan

**Status**: Planning
**Last Updated**: 2026-01-03
**Purpose**: Break down Milestone 7 broad features into specific, LLM-executable tasks with comprehensive logging, events, property-based tests, and image verification.

---

## Overview

This document provides detailed, actionable task breakdowns for all Milestone 7 features. Each feature includes:
- **Specific implementation tasks** that can be completed by an LLM
- **Event definitions** for the event sourcing system
- **Logging requirements** using the tracing crate
- **Property-based tests** using proptest
- **Image verification** where applicable using visual regression testing

---

## Table of Contents

1. [Scene System (Start Menu + Game Play)](#1-scene-system-start-menu--game-play)
2. [Resources, Materials, and Goods Expansion](#2-resources-materials-and-goods-expansion)
3. [Banking and Financial Markets](#3-banking-and-financial-markets)
4. [Production Chains Expansion](#4-production-chains-expansion)
5. [Satellite Launch System](#5-satellite-launch-system)
6. [Planet Gateway Exploration](#6-planet-gateway-exploration)
7. [Train Mechanics Expansion](#7-train-mechanics-expansion)
8. [Galaxy Map System](#8-galaxy-map-system)
9. [High-Level Economy System](#9-high-level-economy-system)
10. [Population Migration Mechanics](#10-population-migration-mechanics)
11. [Population Buildings](#11-population-buildings)
12. [Underground Excavation](#12-underground-excavation)
13. [Terraforming System](#13-terraforming-system)

---

## 1. Scene System (Start Menu + Game Play)

### Overview
Implement a multi-scene architecture in the Bevy client to separate the start menu from gameplay. This enables proper game initialization, save management, and settings configuration before entering the game.

### Architecture

**New Bevy States:**
```rust
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum AppState {
    #[default]
    StartMenu,
    GamePlay,
    Settings,
}
```

### Tasks

#### Task 1.1: Define AppState and Scene Transition System
**File**: `crates/outpost-client/src/scenes/mod.rs` (new)

**Objective**: Create the scene state management infrastructure.

**Implementation**:
- Define `AppState` enum with variants: `StartMenu`, `GamePlay`, `Settings`
- Implement scene transition system that cleans up entities from previous scene
- Add `SceneEntity` marker component for automatic cleanup

**Events**:
```rust
SceneTransitioned { from: AppState, to: AppState, timestamp: f64 }
```

**Logging**:
```rust
info!("Transitioning from {:?} to {:?}", from_state, to_state);
debug!("Cleaned up {} entities from previous scene", entity_count);
```

**Property Tests**:
```rust
proptest! {
    #[test]
    fn scene_transitions_are_reversible(states in vec(arbitrary_app_state(), 2..10)) {
        // Verify that transitioning through states doesn't corrupt game state
    }

    #[test]
    fn scene_cleanup_removes_all_entities(entity_count in 1..1000usize) {
        // Verify all scene entities are cleaned up on transition
    }
}
```

**Image Verification**:
- Capture screenshot after each scene transition
- Verify no visual artifacts from previous scene
- Use egui rect logging (F10) to verify UI elements are positioned correctly

**Acceptance Criteria**:
- [ ] AppState enum defined with at least 3 states
- [ ] Scene transition system removes all `SceneEntity` components
- [ ] SceneTransitioned events logged and emitted
- [ ] Property tests verify state transition integrity
- [ ] Visual regression tests pass for all transitions

---

#### Task 1.2: Implement Start Menu Scene
**File**: `crates/outpost-client/src/scenes/start_menu.rs` (new)

**Objective**: Create a functional start menu with buttons for New Game, Load Game, Settings, About, and Exit.

**Implementation**:
- Create `setup_start_menu` system that runs on entering `AppState::StartMenu`
- Render centered menu using `bevy_egui` with custom styling
- Implement button handlers for each menu option
- Add background (starfield or planet render)
- Display game version and build info

**UI Layout** (using egui):
```rust
egui::CentralPanel::default().show(contexts.ctx_mut(), |ui| {
    ui.vertical_centered(|ui| {
        ui.heading("OUTPOST 3");
        ui.add_space(20.0);

        if ui.button("New Game").clicked() {
            // Transition to GamePlay with new GameState
        }
        if ui.button("Load Game").clicked() {
            // Open save file selector
        }
        if ui.button("Settings").clicked() {
            // Transition to Settings
        }
        if ui.button("About").clicked() {
            // Show about modal
        }
        if ui.button("Exit").clicked() {
            // Send AppExit event
        }
    });
});
```

**Events**:
```rust
MenuButtonClicked { button: MenuButton, timestamp: f64 }
NewGameStarted { timestamp: f64 }
SaveGameLoaded { save_name: String, timestamp: f64 }
```

**Logging**:
```rust
info!("Start menu initialized");
info!("User clicked: {:?}", button_name);
info!("Starting new game");
info!("Loading save: {}", save_name);
```

**Property Tests**:
```rust
proptest! {
    #[test]
    fn menu_buttons_always_trigger_valid_states(button in arbitrary_menu_button()) {
        // Verify each button leads to a valid AppState
    }
}
```

**Image Verification**:
- Capture reference screenshot of start menu layout
- Verify button positions using egui rect logging
- Test on 1920x1080 and verify centering
- Compare screenshots for visual regressions

**Acceptance Criteria**:
- [ ] Start menu renders centered UI
- [ ] All 5 buttons are functional
- [ ] MenuButtonClicked events logged for each button
- [ ] Visual regression test captures menu layout
- [ ] UI rect positions logged and verified

---

#### Task 1.3: Implement Save File Selection UI
**File**: `crates/outpost-client/src/scenes/save_selector.rs` (new)

**Objective**: Create a modal/panel for browsing and loading save files.

**Implementation**:
- List all available save files from storage backend
- Display save metadata: colony name, turn number, last saved timestamp
- Allow filtering/sorting by date or turn
- Preview save file details before loading
- Delete save file option with confirmation

**Storage Integration**:
```rust
pub trait Storage {
    fn list_saves(&self) -> Result<Vec<SaveMetadata>, StorageError>;
    fn load_save(&self, save_id: &str) -> Result<GameState, StorageError>;
    fn delete_save(&self, save_id: &str) -> Result<(), StorageError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveMetadata {
    pub save_id: String,
    pub colony_name: String,
    pub turn_number: u64,
    pub last_saved: String, // ISO 8601 timestamp
    pub version: String,
}
```

**Events**:
```rust
SaveFileSelected { save_id: String, timestamp: f64 }
SaveFileDeleted { save_id: String, timestamp: f64 }
SaveLoadFailed { save_id: String, error: String, timestamp: f64 }
```

**Logging**:
```rust
info!("Listing save files");
debug!("Found {} save files", save_count);
info!("User selected save: {}", save_id);
warn!("Failed to load save {}: {}", save_id, error);
```

**Property Tests**:
```rust
proptest! {
    #[test]
    fn save_metadata_roundtrip(metadata in arbitrary_save_metadata()) {
        // Verify metadata serialization is lossless
    }

    #[test]
    fn list_saves_returns_sorted_by_timestamp(saves in vec(arbitrary_save_metadata(), 1..20)) {
        // Verify saves are returned in descending timestamp order
    }
}
```

**Image Verification**:
- Capture save selector UI with mock save files
- Verify table/list layout and scrolling
- Test on both desktop and WASM

**Acceptance Criteria**:
- [ ] Save files listed with metadata
- [ ] User can select and load a save
- [ ] Delete functionality with confirmation
- [ ] Events logged for all operations
- [ ] Property tests verify metadata handling
- [ ] UI layout verified via screenshots

---

#### Task 1.4: Implement Settings Scene
**File**: `crates/outpost-client/src/scenes/settings.rs` (new)

**Objective**: Create a settings UI for configuring game and client options.

**Implementation**:
- Audio settings (volume, mute)
- Graphics settings (resolution, fullscreen, vsync)
- Gameplay settings (autosave interval, turn speed)
- Keybinding configuration
- Save settings to persistent storage

**Settings Structure**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSettings {
    pub audio: AudioSettings,
    pub graphics: GraphicsSettings,
    pub gameplay: GameplaySettings,
    pub keybindings: KeyBindings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSettings {
    pub master_volume: f32,  // 0.0 - 1.0
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub muted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphicsSettings {
    pub resolution: (u32, u32),
    pub fullscreen: bool,
    pub vsync: bool,
    pub ui_scale: f32,  // 0.5 - 2.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameplaySettings {
    pub autosave_interval: u32,  // turns
    pub show_tooltips: bool,
    pub show_notifications: bool,
}
```

**Events**:
```rust
SettingsChanged { setting_name: String, old_value: String, new_value: String, timestamp: f64 }
SettingsSaved { timestamp: f64 }
SettingsReset { timestamp: f64 }
```

**Logging**:
```rust
info!("Settings scene opened");
info!("Changed {}: {} -> {}", setting_name, old_value, new_value);
info!("Settings saved to persistent storage");
info!("Settings reset to defaults");
```

**Property Tests**:
```rust
proptest! {
    #[test]
    fn settings_roundtrip_preserves_values(settings in arbitrary_game_settings()) {
        // Verify settings serialization is lossless
    }

    #[test]
    fn volume_always_clamped(volume in any::<f32>()) {
        let clamped = clamp_volume(volume);
        prop_assert!(clamped >= 0.0 && clamped <= 1.0);
    }

    #[test]
    fn resolution_always_valid(width in 640u32..7680, height in 480u32..4320) {
        // Verify resolution is within reasonable bounds
    }
}
```

**Image Verification**:
- Capture settings UI layout
- Verify sliders, checkboxes, and dropdowns render correctly
- Test UI scaling at different values

**Acceptance Criteria**:
- [ ] Settings UI displays all options
- [ ] Changes persist to storage
- [ ] Events logged for all setting changes
- [ ] Property tests verify value constraints
- [ ] Visual regression tests capture UI

---

#### Task 1.5: Implement Scene Transitions with Fade Effect
**File**: `crates/outpost-client/src/scenes/transitions.rs` (new)

**Objective**: Add smooth visual transitions between scenes.

**Implementation**:
- Fade-out current scene over 0.3 seconds
- Clean up entities during fade
- Fade-in new scene over 0.3 seconds
- Optional loading screen for slow transitions

**Transition System**:
```rust
#[derive(Component)]
pub struct FadeOverlay {
    pub alpha: f32,
    pub direction: FadeDirection,  // In or Out
    pub duration: f32,
    pub elapsed: f32,
}

fn update_fade_overlay(
    time: Res<Time>,
    mut query: Query<(&mut FadeOverlay, &mut Sprite)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    // Animate fade overlay
    // Trigger state transition when fade-out complete
}
```

**Events**:
```rust
FadeStarted { direction: FadeDirection, timestamp: f64 }
FadeCompleted { direction: FadeDirection, timestamp: f64 }
```

**Logging**:
```rust
debug!("Starting fade {:?}, duration: {}s", direction, duration);
debug!("Fade completed");
```

**Property Tests**:
```rust
proptest! {
    #[test]
    fn fade_alpha_always_clamped(delta_time in 0.0f32..1.0) {
        // Verify alpha stays in 0.0-1.0 range
    }
}
```

**Image Verification**:
- Capture frames during fade transition
- Verify smooth alpha interpolation
- Test on both desktop and WASM

**Acceptance Criteria**:
- [ ] Fade overlay system implemented
- [ ] Transitions smoothly between scenes
- [ ] Events logged at start/end of fade
- [ ] Property tests verify alpha clamping
- [ ] Visual test captures transition frames

---

### Scene System Summary

**Total Tasks**: 5
**Estimated Complexity**: Medium
**Dependencies**: None (foundational feature)

**Testing Coverage**:
- 10+ property-based tests
- Visual regression tests for all scenes
- UI rect verification using F10 overlay

**Events Introduced**: 10+
**Logging Points**: 15+

---

## 2. Resources, Materials, and Goods Expansion

### Overview
Expand the existing 29 resource types to cover a comprehensive economy with minerals, elements, metals, alloys, food, chemicals, energy, and specialized goods. Current implementation has basic resources; this expands to 100+ types organized in a realistic resource tree.

### Current State Analysis
- **Existing**: 29 resource types in `domain/resource.rs`
- **Categories**: Currency, Raw Materials (Basic/Advanced), Processed Goods (Basic/Advanced), Specialized
- **System**: HashMap-based storage with get/set/add/subtract/consume methods

### New Resource Categories

#### 2.1 Minerals and Elements (Task Group)

**File**: `crates/outpost-core/src/domain/resource.rs` (extend)

**Objective**: Add realistic mineral and elemental resources for mining and extraction.

##### Task 2.1.1: Define Mineral Resources
**Implementation**:
```rust
// Add to ResourceType enum
pub enum ResourceType {
    // ... existing variants ...

    // Minerals (ores and raw extracts)
    IronOre,        // Existing
    CopperOre,      // Existing
    GoldOre,
    SilverOre,
    LeadOre,
    ZincOre,
    TinOre,
    NickelOre,
    CobaltOre,
    ManganeseOre,
    ChromiumOre,
    TungstenOre,
    MolybdenumOre,
    Bauxite,        // Aluminum ore
    Cassiterite,    // Tin ore
    Galena,         // Lead ore
    Sphalerite,     // Zinc ore

    // Rare earth elements
    Neodymium,
    Europium,
    Terbium,
    Dysprosium,
    Yttrium,
    Scandium,

    // Industrial minerals
    Quartz,
    Limestone,
    Gypsum,
    Salt,
    Sulfur,
    Phosphate,
    Potash,
}
```

**Resource Properties**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceProperties {
    pub resource_type: ResourceType,
    pub category: ResourceCategory,
    pub base_value: f64,        // Credits per unit
    pub density: f32,            // kg per unit
    pub rarity: Rarity,
    pub extraction_difficulty: u8,  // 1-10
    pub storage_requirements: StorageType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    VeryRare,
    Legendary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageType {
    Bulk,           // Silos, warehouses
    Tank,           // Liquids, gases
    Specialized,    // Temperature controlled
    Vault,          // High value, small volume
}
```

**Events**:
```rust
ResourceTypeAdded { resource_type: ResourceType, timestamp: f64 }
ResourcePropertiesUpdated { resource_type: ResourceType, property: String, timestamp: f64 }
```

**Logging**:
```rust
info!("Added {} new mineral resource types", count);
debug!("Resource properties for {:?}: value={}, rarity={:?}", resource_type, base_value, rarity);
```

**Property Tests**:
```rust
proptest! {
    #[test]
    fn all_resources_have_positive_value(resource in arbitrary_resource_type()) {
        let props = ResourceProperties::for_type(resource);
        prop_assert!(props.base_value > 0.0);
    }

    #[test]
    fn rarer_resources_have_higher_value(
        common in arbitrary_common_resource(),
        rare in arbitrary_rare_resource()
    ) {
        prop_assert!(rare.base_value > common.base_value);
    }

    #[test]
    fn extraction_difficulty_correlates_with_rarity(resource in arbitrary_resource_type()) {
        let props = ResourceProperties::for_type(resource);
        match props.rarity {
            Rarity::Common => prop_assert!(props.extraction_difficulty <= 3),
            Rarity::Legendary => prop_assert!(props.extraction_difficulty >= 7),
            _ => {}
        }
    }
}
```

**Image Verification**:
- Resource icon atlas visualization
- Resource tree graph showing relationships
- UI verification of resource tooltips

**Acceptance Criteria**:
- [ ] 30+ mineral resource types added
- [ ] ResourceProperties implemented for all types
- [ ] Rarity and extraction difficulty assigned
- [ ] Property tests verify value/rarity correlations
- [ ] Events logged when resources added

---

##### Task 2.1.2: Define Elemental Resources
**Implementation**:
```rust
pub enum ResourceType {
    // ... existing ...

    // Pure elements (post-processing)
    Iron,
    Copper,
    Gold,
    Silver,
    Aluminum,
    Titanium,      // Existing
    Platinum,      // Existing
    Lead,
    Zinc,
    Tin,
    Nickel,
    Cobalt,
    Manganese,
    Chromium,
    Tungsten,
    Molybdenum,

    // Non-metals
    Carbon,
    Silicon,       // Existing
    Oxygen,
    Nitrogen,
    Hydrogen,
    Chlorine,
    Fluorine,

    // Noble gases
    Helium,
    Neon,
    Argon,
    Xenon,
}
```

**Processing Chains**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionRecipe {
    pub input: ResourceType,     // e.g., Bauxite
    pub output: ResourceType,    // e.g., Aluminum
    pub yield_ratio: f32,        // e.g., 0.4 (40% yield)
    pub energy_cost: i64,
    pub processing_time: u32,    // turns
    pub required_building: BuildingType,
}

// Example: Bauxite -> Aluminum
const ALUMINUM_EXTRACTION: ExtractionRecipe = ExtractionRecipe {
    input: ResourceType::Bauxite,
    output: ResourceType::Aluminum,
    yield_ratio: 0.4,
    energy_cost: 500,
    processing_time: 2,
    required_building: BuildingType::Refinery,
};
```

**Events**:
```rust
ResourceExtracted { ore: ResourceType, element: ResourceType, amount: i64, yield: f32, timestamp: f64 }
ExtractionFailed { ore: ResourceType, reason: String, timestamp: f64 }
```

**Logging**:
```rust
info!("Extracting {} from {} (yield: {}%)", element, ore, yield_ratio * 100.0);
debug!("Extraction produced {} units of {}", output_amount, element);
warn!("Extraction failed: {}", reason);
```

**Property Tests**:
```rust
proptest! {
    #[test]
    fn extraction_yield_always_produces_less_output(
        input_amount in 1i64..10000,
        yield_ratio in 0.1f32..1.0
    ) {
        let output = (input_amount as f32 * yield_ratio) as i64;
        prop_assert!(output <= input_amount);
    }

    #[test]
    fn energy_cost_scales_with_amount(
        amount in 1i64..1000,
        recipe in arbitrary_extraction_recipe()
    ) {
        let total_cost = recipe.energy_cost * amount;
        prop_assert!(total_cost >= recipe.energy_cost);
    }
}
```

**Acceptance Criteria**:
- [ ] 20+ elemental resource types added
- [ ] ExtractionRecipe struct defined
- [ ] 10+ ore-to-element recipes created
- [ ] Property tests verify extraction economics
- [ ] Events logged for all extractions

---

#### 2.2 Metals and Alloys (Task Group)

##### Task 2.2.1: Define Metal Alloys
**File**: `crates/outpost-core/src/domain/resource.rs`

**Implementation**:
```rust
pub enum ResourceType {
    // ... existing ...

    // Alloys
    Steel,         // Existing (Iron + Carbon)
    StainlessSteel,  // Steel + Chromium + Nickel
    CarbonSteel,
    Bronze,        // Copper + Tin
    Brass,         // Copper + Zinc
    TitaniumAlloy,
    AluminumAlloy,
    Superalloy,    // Nickel-based high-temp alloy

    // Specialized alloys
    Nitinol,       // Nickel-Titanium shape memory alloy
    Inconel,       // High-performance superalloy
    Stellite,      // Cobalt-chromium alloy
}
```

**Alloy Recipes**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlloyRecipe {
    pub output: ResourceType,
    pub inputs: Vec<(ResourceType, i64)>,  // (resource, amount)
    pub output_amount: i64,
    pub energy_cost: i64,
    pub processing_time: u32,
    pub required_building: BuildingType,
    pub required_temperature: u32,  // Kelvin
}

// Example: Steel production
const STEEL_RECIPE: AlloyRecipe = AlloyRecipe {
    output: ResourceType::Steel,
    inputs: vec![
        (ResourceType::Iron, 100),
        (ResourceType::Carbon, 5),
        (ResourceType::Oxygen, 10),
    ],
    output_amount: 100,
    energy_cost: 1000,
    processing_time: 3,
    required_building: BuildingType::Refinery,
    required_temperature: 1873,  // ~1600°C
};
```

**Events**:
```rust
AlloyProduced { alloy: ResourceType, amount: i64, inputs_consumed: Vec<(ResourceType, i64)>, timestamp: f64 }
AlloyProductionFailed { alloy: ResourceType, reason: String, timestamp: f64 }
```

**Logging**:
```rust
info!("Producing {} units of {:?}", amount, alloy);
debug!("Alloy recipe requires: {:?}", inputs);
info!("Successfully produced {} {:?}", amount, alloy);
warn!("Alloy production failed: {}", reason);
```

**Property Tests**:
```rust
proptest! {
    #[test]
    fn alloy_production_conserves_mass(recipe in arbitrary_alloy_recipe()) {
        let input_mass: f32 = recipe.inputs.iter()
            .map(|(rt, amt)| ResourceProperties::for_type(*rt).density * (*amt as f32))
            .sum();
        let output_mass = ResourceProperties::for_type(recipe.output).density
            * (recipe.output_amount as f32);

        // Allow 10% loss due to slag/waste
        prop_assert!((output_mass / input_mass) >= 0.9 && (output_mass / input_mass) <= 1.0);
    }

    #[test]
    fn complex_alloys_require_more_energy(
        simple in arbitrary_simple_alloy(),
        complex in arbitrary_complex_alloy()
    ) {
        prop_assert!(complex.energy_cost > simple.energy_cost);
    }
}
```

**Acceptance Criteria**:
- [ ] 10+ alloy types defined
- [ ] AlloyRecipe struct with multi-input support
- [ ] 15+ alloy recipes created
- [ ] Mass conservation property tests pass
- [ ] Events logged for production/failures

---

#### 2.3 Food and Drink (Task Group)

##### Task 2.3.1: Define Food Resources
**File**: `crates/outpost-core/src/domain/resource.rs`

**Implementation**:
```rust
pub enum ResourceType {
    // ... existing ...

    // Raw food
    Food,          // Existing (generic)
    Grain,
    Vegetables,
    Fruit,
    Meat,
    Fish,
    Eggs,
    Dairy,
    Legumes,
    Nuts,

    // Processed food
    Flour,
    Bread,
    PreservedFood,
    FrozenFood,
    MealKits,
    NutrientPaste,  // Space food!

    // Beverages
    Water,         // Existing
    Juice,
    Milk,
    Alcohol,
    Coffee,
    Tea,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoodProperties {
    pub resource_type: ResourceType,
    pub calories_per_unit: u32,
    pub nutrition_value: f32,  // 0.0 - 1.0
    pub perishability: Perishability,
    pub shelf_life: u32,  // turns before spoilage
    pub morale_bonus: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Perishability {
    NonPerishable,
    Long,      // > 100 turns
    Medium,    // 20-100 turns
    Short,     // < 20 turns
}
```

**Food Spoilage System**:
```rust
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct FoodStorage {
    pub stock: HashMap<ResourceType, Vec<FoodBatch>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoodBatch {
    pub amount: i64,
    pub produced_turn: u64,
    pub expires_turn: u64,
}

fn process_food_spoilage(
    state: &mut GameState,
    storage: &mut FoodStorage,
) -> Vec<EventType> {
    let mut events = vec![];

    for (resource_type, batches) in &mut storage.stock {
        batches.retain(|batch| {
            if batch.expires_turn <= state.turn {
                events.push(EventType::FoodSpoiled {
                    resource_type: *resource_type,
                    amount: batch.amount,
                    turn: state.turn,
                });
                false
            } else {
                true
            }
        });
    }

    events
}
```

**Events**:
```rust
FoodProduced { food_type: ResourceType, amount: i64, expires_turn: u64, timestamp: f64 }
FoodConsumed { food_type: ResourceType, amount: i64, population_fed: i64, timestamp: f64 }
FoodSpoiled { food_type: ResourceType, amount: i64, timestamp: f64 }
FoodShortage { required: i64, available: i64, timestamp: f64 }
```

**Logging**:
```rust
info!("Produced {} units of {:?}, expires on turn {}", amount, food_type, expires_turn);
debug!("Fed {} population with {} {:?}", population, amount, food_type);
warn!("{} units of {:?} spoiled", amount, food_type);
error!("Food shortage! Required: {}, Available: {}", required, available);
```

**Property Tests**:
```rust
proptest! {
    #[test]
    fn food_spoilage_always_happens_after_shelf_life(
        food_type in arbitrary_perishable_food(),
        turns_elapsed in 1u64..1000
    ) {
        let props = FoodProperties::for_type(food_type);
        let spoiled = turns_elapsed > (props.shelf_life as u64);
        // Verify spoilage logic
    }

    #[test]
    fn population_food_consumption_is_proportional(
        population in 1i64..100000,
        food_type in arbitrary_food_type()
    ) {
        let props = FoodProperties::for_type(food_type);
        let required = calculate_food_requirement(population, props.calories_per_unit);
        prop_assert!(required >= population);  // At least 1 unit per capita minimum
    }

    #[test]
    fn preserved_food_lasts_longer(
        fresh in arbitrary_fresh_food(),
        preserved in arbitrary_preserved_food()
    ) {
        prop_assert!(preserved.shelf_life > fresh.shelf_life);
    }
}
```

**Image Verification**:
- Food storage UI showing batch expiration dates
- Spoilage warning indicators
- Nutrition status panel

**Acceptance Criteria**:
- [ ] 20+ food resource types defined
- [ ] FoodProperties and spoilage system implemented
- [ ] Food batches track expiration
- [ ] Property tests verify spoilage mechanics
- [ ] Events logged for production/consumption/spoilage

---

#### 2.4 Chemicals (Task Group)

##### Task 2.4.1: Define Chemical Resources
**File**: `crates/outpost-core/src/domain/resource.rs`

**Implementation**:
```rust
pub enum ResourceType {
    // ... existing ...

    // Basic chemicals
    Chemicals,     // Existing (generic)
    Acid,
    Base,
    Solvent,
    Catalyst,

    // Industrial chemicals
    SulfuricAcid,
    HydrochloricAcid,
    Ammonia,
    SodiumHydroxide,
    CalciumCarbonate,

    // Petrochemicals
    Ethylene,
    Propylene,
    Benzene,
    Methanol,
    Ethanol,

    // Polymers and plastics
    Plastics,      // Existing (generic)
    Polyethylene,
    Polypropylene,
    PVC,
    Polyester,
    Nylon,
    Epoxy,

    // Specialty chemicals
    Fertilizer,
    Pesticide,
    Lubricant,
    Coolant,
    Explosives,
    Propellant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChemicalProperties {
    pub resource_type: ResourceType,
    pub chemical_formula: String,
    pub hazard_level: HazardLevel,
    pub storage_requirements: ChemicalStorageType,
    pub reactions: Vec<ChemicalReaction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HazardLevel {
    Safe,
    Corrosive,
    Flammable,
    Toxic,
    Explosive,
    Radioactive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChemicalStorageType {
    StandardTank,
    PressurizedTank,
    CryogenicTank,
    ShieldedContainer,
    InertAtmosphere,
}
```

**Chemical Reactions**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChemicalReaction {
    pub reactants: Vec<(ResourceType, i64)>,
    pub products: Vec<(ResourceType, i64)>,
    pub catalyst: Option<ResourceType>,
    pub energy_delta: i64,  // Negative = exothermic, Positive = endothermic
    pub reaction_rate: f32,  // units per turn
    pub temperature_range: (u32, u32),  // (min, max) Kelvin
    pub pressure_required: Option<f32>,  // atmospheres
}

// Example: Ammonia synthesis (Haber process)
const AMMONIA_SYNTHESIS: ChemicalReaction = ChemicalReaction {
    reactants: vec![
        (ResourceType::Nitrogen, 1),
        (ResourceType::Hydrogen, 3),
    ],
    products: vec![
        (ResourceType::Ammonia, 2),
    ],
    catalyst: Some(ResourceType::Iron),
    energy_delta: -92,  // kJ/mol (exothermic)
    reaction_rate: 10.0,
    temperature_range: (673, 773),  // 400-500°C
    pressure_required: Some(200.0),  // 200 atm
};
```

**Events**:
```rust
ChemicalReactionStarted { reaction_id: String, inputs: Vec<(ResourceType, i64)>, timestamp: f64 }
ChemicalReactionCompleted { reaction_id: String, outputs: Vec<(ResourceType, i64)>, timestamp: f64 }
ChemicalReactionFailed { reaction_id: String, reason: String, timestamp: f64 }
HazardousSpill { chemical: ResourceType, amount: i64, location: Hex, timestamp: f64 }
```

**Logging**:
```rust
info!("Starting chemical reaction: {}", reaction_id);
debug!("Reaction conditions: temp={}K, pressure={}atm", temp, pressure);
info!("Reaction produced: {:?}", outputs);
warn!("Reaction failed: {}", reason);
error!("HAZARDOUS SPILL: {} units of {:?} at {:?}", amount, chemical, location);
```

**Property Tests**:
```rust
proptest! {
    #[test]
    fn chemical_reactions_conserve_atoms(reaction in arbitrary_chemical_reaction()) {
        // Verify atom count is same on both sides (simplified)
        let reactant_count: i64 = reaction.reactants.iter().map(|(_, n)| n).sum();
        let product_count: i64 = reaction.products.iter().map(|(_, n)| n).sum();
        // In real chemistry this would check specific atoms, here we check total
    }

    #[test]
    fn exothermic_reactions_generate_energy(reaction in arbitrary_exothermic_reaction()) {
        prop_assert!(reaction.energy_delta < 0);
    }

    #[test]
    fn hazardous_chemicals_require_special_storage(
        chemical in arbitrary_hazardous_chemical()
    ) {
        let props = ChemicalProperties::for_type(chemical);
        prop_assert!(props.storage_requirements != ChemicalStorageType::StandardTank);
    }
}
```

**Acceptance Criteria**:
- [ ] 30+ chemical resource types defined
- [ ] ChemicalReaction system implemented
- [ ] Hazard levels assigned to all chemicals
- [ ] Property tests verify reaction conservation
- [ ] Events logged for reactions and spills

---

#### 2.5 Energy and Power Resources (Task Group)

##### Task 2.5.1: Define Energy Resources
**File**: `crates/outpost-core/src/domain/resource.rs`

**Implementation**:
```rust
pub enum ResourceType {
    // ... existing ...

    // Energy carriers
    Energy,        // Existing (generic electrical energy)
    Fuel,          // Existing (generic)
    Coal,          // Existing
    Oil,           // Existing
    Gas,           // Existing
    Uranium,       // Existing

    // Refined fuels
    Gasoline,
    Diesel,
    JetFuel,
    RocketFuel,
    NuclearFuel,

    // Alternative energy
    Hydrogen,      // Also chemical, but energy carrier
    Biofuel,
    SyntheticFuel,

    // Energy storage
    Battery,       // Stored electrical energy
    Capacitor,
    FuelCell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyProperties {
    pub resource_type: ResourceType,
    pub energy_density: f64,  // MJ per unit
    pub power_generation_rate: i64,  // Energy per turn when burned/used
    pub carbon_emissions: f32,  // CO2 per unit
    pub fuel_type: FuelType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FuelType {
    Solid,
    Liquid,
    Gas,
    Nuclear,
    Electrical,
}
```

**Power Generation Integration**:
```rust
// Extend PowerGrid to support multiple fuel types
impl PowerGrid {
    pub fn generate_power_from_fuel(
        &mut self,
        fuel_type: ResourceType,
        amount: i64,
        resources: &mut Resources,
    ) -> Result<i64, PowerGridError> {
        let props = EnergyProperties::for_type(fuel_type);

        if resources.can_afford(&[(fuel_type, amount)]) {
            resources.consume(&[(fuel_type, amount)])?;
            let energy_generated = amount * props.power_generation_rate;
            self.generation += energy_generated;

            // Track emissions
            self.carbon_emissions += props.carbon_emissions * (amount as f32);

            Ok(energy_generated)
        } else {
            Err(PowerGridError::InsufficientFuel)
        }
    }
}
```

**Events**:
```rust
PowerGenerated { fuel_type: ResourceType, fuel_consumed: i64, energy_produced: i64, timestamp: f64 }
CarbonEmitted { source: String, amount: f32, timestamp: f64 }
EnergyStorageCharged { storage_type: ResourceType, amount: i64, timestamp: f64 }
EnergyStorageDischarged { storage_type: ResourceType, amount: i64, timestamp: f64 }
```

**Logging**:
```rust
info!("Generated {} energy from {} units of {:?}", energy, amount, fuel_type);
debug!("Power generation efficiency: {}%", efficiency);
info!("Emitted {} CO2", carbon);
debug!("Energy storage charged: {} / {} capacity", current, max);
```

**Property Tests**:
```rust
proptest! {
    #[test]
    fn higher_energy_density_produces_more_power(
        low_density_fuel in arbitrary_low_density_fuel(),
        high_density_fuel in arbitrary_high_density_fuel()
    ) {
        let low_props = EnergyProperties::for_type(low_density_fuel);
        let high_props = EnergyProperties::for_type(high_density_fuel);
        prop_assert!(high_props.energy_density > low_props.energy_density);
    }

    #[test]
    fn clean_energy_has_zero_emissions(clean_fuel in arbitrary_clean_fuel()) {
        let props = EnergyProperties::for_type(clean_fuel);
        prop_assert_eq!(props.carbon_emissions, 0.0);
    }

    #[test]
    fn energy_storage_charge_discharge_roundtrip(
        storage_type in arbitrary_energy_storage(),
        amount in 1i64..10000
    ) {
        // Verify charging then discharging returns ~same energy (with efficiency loss)
    }
}
```

**Acceptance Criteria**:
- [ ] 15+ energy resource types defined
- [ ] EnergyProperties with energy density tracking
- [ ] PowerGrid supports multiple fuel types
- [ ] Carbon emissions tracked
- [ ] Property tests verify energy economics
- [ ] Events logged for generation/emissions

---

#### 2.6 Other Goods (Task Group)

##### Task 2.6.1: Define Manufactured Goods
**File**: `crates/outpost-core/src/domain/resource.rs`

**Implementation**:
```rust
pub enum ResourceType {
    // ... existing ...

    // Consumer goods
    ConsumerGoods,  // Existing (generic)
    Clothing,
    Furniture,
    Appliances,
    PersonalElectronics,
    Toys,
    Books,
    ArtSupplies,

    // Luxury goods
    Luxuries,       // Existing (generic)
    Jewelry,
    FineArt,
    GourmetFood,
    LuxuryClothing,

    // Construction materials
    Concrete,       // Existing
    Timber,         // Existing
    Bricks,
    Glass,
    Insulation,
    Wiring,
    Pipes,
    Roofing,

    // Tools and equipment
    HandTools,
    PowerTools,
    MiningEquipment,
    FarmingEquipment,
    ConstructionEquipment,

    // Medical supplies
    Medicine,       // Existing
    Bandages,
    Antibiotics,
    Vaccines,
    Surgical supplies,
    MedicalEquipment,

    // Scientific equipment
    Research,       // Existing (generic)
    LabEquipment,
    Microscopes,
    Spectrometers,
    Computers,
    Servers,
    Sensors,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoodsProperties {
    pub resource_type: ResourceType,
    pub category: GoodsCategory,
    pub quality_tiers: Vec<QualityTier>,
    pub durability: u32,  // turns before degradation
    pub maintenance_cost: i64,  // per turn
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoodsCategory {
    Consumer,
    Luxury,
    Construction,
    Industrial,
    Medical,
    Scientific,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityTier {
    pub tier_name: String,  // e.g., "Basic", "Standard", "Premium"
    pub value_multiplier: f32,
    pub production_cost_multiplier: f32,
    pub morale_bonus: f32,
}
```

**Quality System**:
```rust
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct GoodsInstance {
    pub goods_type: ResourceType,
    pub quality: QualityTier,
    pub condition: f32,  // 0.0 - 1.0
    pub age: u32,  // turns since production
}

fn degrade_goods(instance: &mut GoodsInstance, props: &GoodsProperties) {
    instance.age += 1;

    if instance.age > props.durability {
        let degradation_rate = 0.01;  // 1% per turn past durability
        instance.condition -= degradation_rate;
        instance.condition = instance.condition.max(0.0);
    }
}
```

**Events**:
```rust
GoodsProduced { goods_type: ResourceType, quality: String, amount: i64, timestamp: f64 }
GoodsDegraded { goods_type: ResourceType, amount: i64, new_condition: f32, timestamp: f64 }
GoodsScrapped { goods_type: ResourceType, amount: i64, salvage: Vec<(ResourceType, i64)>, timestamp: f64 }
```

**Logging**:
```rust
info!("Produced {} units of {} quality {:?}", amount, quality, goods_type);
debug!("Goods condition: {:.1}%", condition * 100.0);
info!("Scrapped {} {:?}, salvaged: {:?}", amount, goods_type, salvage);
```

**Property Tests**:
```rust
proptest! {
    #[test]
    fn goods_condition_never_negative(
        initial_condition in 0.0f32..1.0,
        degradation in 0.0f32..0.5
    ) {
        let final_condition = (initial_condition - degradation).max(0.0);
        prop_assert!(final_condition >= 0.0);
    }

    #[test]
    fn higher_quality_costs_more_to_produce(
        goods_type in arbitrary_goods_type(),
        tier_index in 0usize..3
    ) {
        let props = GoodsProperties::for_type(goods_type);
        if tier_index > 0 {
            prop_assert!(
                props.quality_tiers[tier_index].production_cost_multiplier >
                props.quality_tiers[tier_index - 1].production_cost_multiplier
            );
        }
    }

    #[test]
    fn luxury_goods_provide_morale_bonus(luxury in arbitrary_luxury_goods()) {
        let props = GoodsProperties::for_type(luxury);
        prop_assert!(props.quality_tiers.iter().any(|tier| tier.morale_bonus > 0.0));
    }
}
```

**Acceptance Criteria**:
- [ ] 40+ manufactured goods types defined
- [ ] Quality tier system implemented
- [ ] Goods degradation mechanics working
- [ ] Property tests verify quality economics
- [ ] Events logged for production/degradation

---

### Resources Expansion Summary

**Total Resource Types**: 100+ (up from 29)
**New Systems**:
- Extraction recipes (ores → elements)
- Alloy recipes (multi-input)
- Food spoilage with batching
- Chemical reactions
- Energy density and emissions
- Goods quality tiers

**Testing Coverage**:
- 40+ property-based tests
- Mass/energy conservation tests
- Economic balance tests
- Spoilage/degradation tests

**Events Introduced**: 30+
**Logging Points**: 50+

---

## 3. Banking and Financial Markets

### Overview
Implement a comprehensive financial system including banking, loans, investments, currency exchange, bonds, stocks, and inter-colony trading. This enables economic gameplay beyond simple resource production.

### Architecture

**Core Financial Entities**:
```rust
// File: crates/outpost-core/src/domain/finance.rs (new)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bank {
    pub bank_id: String,
    pub name: String,
    pub reserves: i64,  // Credits
    pub loans_outstanding: Vec<Loan>,
    pub interest_rate: f32,  // Annual percentage
    pub credit_rating: CreditRating,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Loan {
    pub loan_id: String,
    pub borrower_id: String,  // Colony ID
    pub principal: i64,
    pub interest_rate: f32,
    pub term: u32,  // turns
    pub remaining_turns: u32,
    pub payment_per_turn: i64,
    pub collateral: Vec<(ResourceType, i64)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CreditRating {
    AAA,
    AA,
    A,
    BBB,
    BB,
    B,
    CCC,
    Default,
}
```

### Tasks

#### Task 3.1: Implement Banking System
**File**: `crates/outpost-core/src/domain/finance.rs` (new)

**Objective**: Create a banking entity that can issue loans and accept deposits.

**Implementation**:
```rust
impl Bank {
    pub fn new(name: String, initial_reserves: i64) -> Self {
        Bank {
            bank_id: Uuid::new_v4().to_string(),
            name,
            reserves: initial_reserves,
            loans_outstanding: vec![],
            interest_rate: 0.05,  // 5% default
            credit_rating: CreditRating::AAA,
        }
    }

    pub fn issue_loan(
        &mut self,
        borrower_id: String,
        amount: i64,
        term: u32,
        collateral: Vec<(ResourceType, i64)>,
    ) -> Result<Loan, BankingError> {
        if amount > self.reserves {
            return Err(BankingError::InsufficientReserves);
        }

        let payment_per_turn = calculate_loan_payment(amount, self.interest_rate, term);

        let loan = Loan {
            loan_id: Uuid::new_v4().to_string(),
            borrower_id,
            principal: amount,
            interest_rate: self.interest_rate,
            term,
            remaining_turns: term,
            payment_per_turn,
            collateral,
        };

        self.reserves -= amount;
        self.loans_outstanding.push(loan.clone());

        Ok(loan)
    }

    pub fn process_loan_payment(
        &mut self,
        loan_id: &str,
        payment: i64,
    ) -> Result<LoanPaymentResult, BankingError> {
        let loan = self.loans_outstanding.iter_mut()
            .find(|l| l.loan_id == loan_id)
            .ok_or(BankingError::LoanNotFound)?;

        if payment < loan.payment_per_turn {
            return Err(BankingError::InsufficientPayment);
        }

        loan.remaining_turns -= 1;
        self.reserves += payment;

        if loan.remaining_turns == 0 {
            let loan_id_copy = loan.loan_id.clone();
            self.loans_outstanding.retain(|l| l.loan_id != loan_id_copy);
            Ok(LoanPaymentResult::LoanPaidOff)
        } else {
            Ok(LoanPaymentResult::PaymentAccepted)
        }
    }

    pub fn default_on_loan(&mut self, loan_id: &str) -> Result<Vec<(ResourceType, i64)>, BankingError> {
        let loan_index = self.loans_outstanding.iter()
            .position(|l| l.loan_id == loan_id)
            .ok_or(BankingError::LoanNotFound)?;

        let loan = self.loans_outstanding.remove(loan_index);
        let seized_collateral = loan.collateral.clone();

        // Reduce bank reserves by outstanding principal
        self.reserves -= loan.principal;

        // Downgrade credit rating
        self.credit_rating = match self.credit_rating {
            CreditRating::AAA => CreditRating::AA,
            CreditRating::AA => CreditRating::A,
            CreditRating::A => CreditRating::BBB,
            CreditRating::BBB => CreditRating::BB,
            CreditRating::BB => CreditRating::B,
            CreditRating::B => CreditRating::CCC,
            CreditRating::CCC | CreditRating::Default => CreditRating::Default,
        };

        Ok(seized_collateral)
    }
}

fn calculate_loan_payment(principal: i64, annual_rate: f32, terms: u32) -> i64 {
    // Convert annual rate to per-turn rate (assuming ~50 turns per year)
    let per_turn_rate = annual_rate / 50.0;

    if per_turn_rate == 0.0 {
        return principal / (terms as i64);
    }

    // Standard loan payment formula: P * (r * (1 + r)^n) / ((1 + r)^n - 1)
    let r = per_turn_rate as f64;
    let n = terms as f64;
    let payment = (principal as f64) * (r * (1.0 + r).powf(n)) / ((1.0 + r).powf(n) - 1.0);

    payment.ceil() as i64
}
```

**Events**:
```rust
LoanIssued { loan_id: String, borrower_id: String, amount: i64, term: u32, interest_rate: f32, timestamp: f64 }
LoanPaymentReceived { loan_id: String, payment: i64, remaining_balance: i64, timestamp: f64 }
LoanPaidOff { loan_id: String, total_paid: i64, timestamp: f64 }
LoanDefaulted { loan_id: String, borrower_id: String, collateral_seized: Vec<(ResourceType, i64)>, timestamp: f64 }
CreditRatingChanged { entity_id: String, old_rating: CreditRating, new_rating: CreditRating, timestamp: f64 }
```

**Logging**:
```rust
info!("Bank {} issued loan: {} credits for {} turns at {}% APR", bank_name, amount, term, interest_rate * 100.0);
debug!("Loan payment: {} credits, {} turns remaining", payment, remaining_turns);
info!("Loan {} paid off", loan_id);
warn!("Loan {} defaulted by {}, seizing collateral", loan_id, borrower_id);
info!("Credit rating changed: {:?} -> {:?}", old_rating, new_rating);
```

**Property Tests**:
```rust
proptest! {
    #[test]
    fn loan_payments_eventually_pay_off_principal(
        principal in 1000i64..1000000,
        annual_rate in 0.01f32..0.20,
        term in 10u32..100
    ) {
        let payment = calculate_loan_payment(principal, annual_rate, term);
        let total_paid = payment * (term as i64);
        prop_assert!(total_paid >= principal);
    }

    #[test]
    fn higher_interest_rates_mean_higher_payments(
        principal in 10000i64..100000,
        low_rate in 0.01f32..0.05,
        high_rate in 0.10f32..0.20,
        term in 20u32..50
    ) {
        let low_payment = calculate_loan_payment(principal, low_rate, term);
        let high_payment = calculate_loan_payment(principal, high_rate, term);
        prop_assert!(high_payment > low_payment);
    }

    #[test]
    fn bank_reserves_never_negative(
        initial_reserves in 10000i64..1000000,
        loan_amounts in vec(1000i64..10000, 1..10)
    ) {
        let mut bank = Bank::new("TestBank".to_string(), initial_reserves);
        for amount in loan_amounts {
            let _ = bank.issue_loan("borrower".to_string(), amount, 10, vec![]);
        }
        prop_assert!(bank.reserves >= 0);
    }

    #[test]
    fn credit_rating_degrades_monotonically_on_defaults(rating in arbitrary_credit_rating()) {
        // Verify that defaulting never improves credit rating
    }
}
```

**Acceptance Criteria**:
- [ ] Bank struct with reserves and loan tracking
- [ ] Loan issuance with collateral support
- [ ] Loan payment processing
- [ ] Default handling with credit rating degradation
- [ ] Property tests verify loan mathematics
- [ ] Events logged for all banking operations

---

#### Task 3.2: Implement Bond Market
**File**: `crates/outpost-core/src/domain/finance.rs`

**Objective**: Create a bond market where colonies can issue bonds to raise capital and investors can buy/sell bonds.

**Implementation**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bond {
    pub bond_id: String,
    pub issuer_id: String,  // Colony ID
    pub face_value: i64,
    pub coupon_rate: f32,  // Annual interest rate
    pub maturity: u64,  // Turn when bond matures
    pub current_price: i64,
    pub credit_rating: CreditRating,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondMarket {
    pub bonds: HashMap<String, Bond>,
    pub order_book: Vec<BondOrder>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondOrder {
    pub order_id: String,
    pub bond_id: String,
    pub order_type: OrderType,
    pub quantity: i64,
    pub price: i64,
    pub trader_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    Buy,
    Sell,
}

impl BondMarket {
    pub fn issue_bond(
        &mut self,
        issuer_id: String,
        face_value: i64,
        coupon_rate: f32,
        maturity_turns: u32,
        current_turn: u64,
        credit_rating: CreditRating,
    ) -> Bond {
        let bond = Bond {
            bond_id: Uuid::new_v4().to_string(),
            issuer_id,
            face_value,
            coupon_rate,
            maturity: current_turn + (maturity_turns as u64),
            current_price: face_value,  // Initial price = face value
            credit_rating,
        };

        self.bonds.insert(bond.bond_id.clone(), bond.clone());
        bond
    }

    pub fn place_order(
        &mut self,
        bond_id: String,
        order_type: OrderType,
        quantity: i64,
        price: i64,
        trader_id: String,
    ) -> Result<String, MarketError> {
        if !self.bonds.contains_key(&bond_id) {
            return Err(MarketError::BondNotFound);
        }

        let order = BondOrder {
            order_id: Uuid::new_v4().to_string(),
            bond_id,
            order_type,
            quantity,
            price,
            trader_id,
        };

        let order_id = order.order_id.clone();
        self.order_book.push(order);

        Ok(order_id)
    }

    pub fn match_orders(&mut self) -> Vec<Trade> {
        let mut trades = vec![];

        // Simple matching: find buy and sell orders for same bond at compatible prices
        for i in 0..self.order_book.len() {
            for j in (i + 1)..self.order_book.len() {
                let (order1, order2) = (&self.order_book[i], &self.order_book[j]);

                if order1.bond_id == order2.bond_id
                    && order1.order_type != order2.order_type
                {
                    let (buy_order, sell_order) = match order1.order_type {
                        OrderType::Buy => (order1, order2),
                        OrderType::Sell => (order2, order1),
                    };

                    if buy_order.price >= sell_order.price {
                        // Match!
                        let quantity = buy_order.quantity.min(sell_order.quantity);
                        let price = (buy_order.price + sell_order.price) / 2;  // Mid-price

                        trades.push(Trade {
                            trade_id: Uuid::new_v4().to_string(),
                            bond_id: buy_order.bond_id.clone(),
                            buyer_id: buy_order.trader_id.clone(),
                            seller_id: sell_order.trader_id.clone(),
                            quantity,
                            price,
                        });

                        // Update bond price based on trade
                        if let Some(bond) = self.bonds.get_mut(&buy_order.bond_id) {
                            bond.current_price = price;
                        }
                    }
                }
            }
        }

        trades
    }

    pub fn pay_coupon(&mut self, bond_id: &str, current_turn: u64) -> Result<i64, MarketError> {
        let bond = self.bonds.get(bond_id)
            .ok_or(MarketError::BondNotFound)?;

        // Annual coupon payment (assuming 50 turns per year)
        let annual_payment = (bond.face_value as f64 * bond.coupon_rate as f64) as i64;
        let per_turn_payment = annual_payment / 50;

        Ok(per_turn_payment)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub trade_id: String,
    pub bond_id: String,
    pub buyer_id: String,
    pub seller_id: String,
    pub quantity: i64,
    pub price: i64,
}
```

**Events**:
```rust
BondIssued { bond_id: String, issuer_id: String, face_value: i64, coupon_rate: f32, maturity: u64, timestamp: f64 }
BondOrderPlaced { order_id: String, bond_id: String, order_type: OrderType, quantity: i64, price: i64, timestamp: f64 }
BondTradeExecuted { trade_id: String, bond_id: String, buyer_id: String, seller_id: String, quantity: i64, price: i64, timestamp: f64 }
CouponPaid { bond_id: String, amount: i64, holder_id: String, timestamp: f64 }
BondMatured { bond_id: String, face_value: i64, holder_id: String, timestamp: f64 }
```

**Logging**:
```rust
info!("Bond issued: {} at {} coupon, matures turn {}", bond_id, coupon_rate, maturity);
debug!("Bond order placed: {:?} {} units at {} credits", order_type, quantity, price);
info!("Bond trade executed: {} units at {}, buyer={}, seller={}", quantity, price, buyer_id, seller_id);
info!("Coupon payment: {} credits for bond {}", amount, bond_id);
info!("Bond {} matured, paying {} to holder", bond_id, face_value);
```

**Property Tests**:
```rust
proptest! {
    #[test]
    fn bond_price_converges_to_face_value_at_maturity(
        face_value in 1000i64..100000,
        coupon_rate in 0.01f32..0.10,
        turns_to_maturity in 1u32..10
    ) {
        // As maturity approaches, price should approach face value
    }

    #[test]
    fn higher_credit_rating_bonds_have_lower_yields(
        aaa_bond in arbitrary_aaa_bond(),
        ccc_bond in arbitrary_ccc_bond()
    ) {
        // Lower-rated bonds should offer higher yields to compensate for risk
    }

    #[test]
    fn buy_sell_orders_always_match_at_valid_price(
        buy_price in 1000i64..10000,
        sell_price in 1000i64..10000
    ) {
        if buy_price >= sell_price {
            let trade_price = (buy_price + sell_price) / 2;
            prop_assert!(trade_price >= sell_price && trade_price <= buy_price);
        }
    }
}
```

**Acceptance Criteria**:
- [ ] Bond issuance system
- [ ] Order book for bond trading
- [ ] Order matching algorithm
- [ ] Coupon payment mechanics
- [ ] Property tests verify bond pricing
- [ ] Events logged for all market activity

---

#### Task 3.3: Implement Stock Market
**File**: `crates/outpost-core/src/domain/finance.rs`

**Objective**: Create a stock market where colonies can list shares and trade equity.

**Implementation**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stock {
    pub symbol: String,  // e.g., "MARS-ALPHA"
    pub company_id: String,  // Colony ID
    pub shares_outstanding: i64,
    pub current_price: i64,
    pub dividend_per_share: i64,
    pub market_cap: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockMarket {
    pub stocks: HashMap<String, Stock>,
    pub order_book: HashMap<String, Vec<StockOrder>>,  // symbol -> orders
    pub trade_history: Vec<StockTrade>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockOrder {
    pub order_id: String,
    pub symbol: String,
    pub order_type: OrderType,
    pub quantity: i64,
    pub limit_price: Option<i64>,  // None = market order
    pub trader_id: String,
    pub placed_turn: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockTrade {
    pub trade_id: String,
    pub symbol: String,
    pub buyer_id: String,
    pub seller_id: String,
    pub quantity: i64,
    pub price: i64,
    pub timestamp: u64,
}

impl StockMarket {
    pub fn list_stock(
        &mut self,
        symbol: String,
        company_id: String,
        shares: i64,
        initial_price: i64,
    ) -> Stock {
        let stock = Stock {
            symbol: symbol.clone(),
            company_id,
            shares_outstanding: shares,
            current_price: initial_price,
            dividend_per_share: 0,
            market_cap: shares * initial_price,
        };

        self.stocks.insert(symbol.clone(), stock.clone());
        self.order_book.insert(symbol, vec![]);

        stock
    }

    pub fn place_order(
        &mut self,
        symbol: String,
        order_type: OrderType,
        quantity: i64,
        limit_price: Option<i64>,
        trader_id: String,
        current_turn: u64,
    ) -> Result<String, MarketError> {
        if !self.stocks.contains_key(&symbol) {
            return Err(MarketError::StockNotFound);
        }

        let order = StockOrder {
            order_id: Uuid::new_v4().to_string(),
            symbol: symbol.clone(),
            order_type,
            quantity,
            limit_price,
            trader_id,
            placed_turn: current_turn,
        };

        let order_id = order.order_id.clone();

        self.order_book.entry(symbol)
            .or_insert_with(Vec::new)
            .push(order);

        Ok(order_id)
    }

    pub fn execute_trades(&mut self, current_turn: u64) -> Vec<StockTrade> {
        let mut trades = vec![];

        for (symbol, orders) in &mut self.order_book {
            // Separate buy and sell orders
            let mut buy_orders: Vec<_> = orders.iter()
                .filter(|o| o.order_type == OrderType::Buy)
                .cloned()
                .collect();
            let mut sell_orders: Vec<_> = orders.iter()
                .filter(|o| o.order_type == OrderType::Sell)
                .cloned()
                .collect();

            // Sort: buy orders by price descending, sell orders by price ascending
            buy_orders.sort_by(|a, b| {
                let a_price = a.limit_price.unwrap_or(i64::MAX);
                let b_price = b.limit_price.unwrap_or(i64::MAX);
                b_price.cmp(&a_price)
            });
            sell_orders.sort_by(|a, b| {
                let a_price = a.limit_price.unwrap_or(0);
                let b_price = b.limit_price.unwrap_or(0);
                a_price.cmp(&b_price)
            });

            // Match orders
            while let (Some(buy), Some(sell)) = (buy_orders.first(), sell_orders.first()) {
                let buy_price = buy.limit_price.unwrap_or(i64::MAX);
                let sell_price = sell.limit_price.unwrap_or(0);

                if buy_price >= sell_price {
                    let quantity = buy.quantity.min(sell.quantity);
                    let price = (buy_price + sell_price) / 2;

                    let trade = StockTrade {
                        trade_id: Uuid::new_v4().to_string(),
                        symbol: symbol.clone(),
                        buyer_id: buy.trader_id.clone(),
                        seller_id: sell.trader_id.clone(),
                        quantity,
                        price,
                        timestamp: current_turn,
                    };

                    trades.push(trade.clone());
                    self.trade_history.push(trade);

                    // Update stock price
                    if let Some(stock) = self.stocks.get_mut(symbol) {
                        stock.current_price = price;
                        stock.market_cap = stock.shares_outstanding * price;
                    }

                    // Remove or update orders
                    if buy.quantity == quantity {
                        buy_orders.remove(0);
                    } else {
                        buy_orders[0].quantity -= quantity;
                    }

                    if sell.quantity == quantity {
                        sell_orders.remove(0);
                    } else {
                        sell_orders[0].quantity -= quantity;
                    }
                } else {
                    break;  // No more matches possible
                }
            }
        }

        trades
    }

    pub fn pay_dividends(&mut self, symbol: &str) -> Result<Vec<(String, i64)>, MarketError> {
        let stock = self.stocks.get(symbol)
            .ok_or(MarketError::StockNotFound)?;

        // Calculate dividend payments to all shareholders
        // (simplified: assumes we track shareholders)
        let total_dividend = stock.dividend_per_share * stock.shares_outstanding;

        // In full implementation, this would distribute to all shareholders
        Ok(vec![(stock.company_id.clone(), total_dividend)])
    }
}
```

**Events**:
```rust
StockListed { symbol: String, company_id: String, shares: i64, initial_price: i64, timestamp: f64 }
StockOrderPlaced { order_id: String, symbol: String, order_type: OrderType, quantity: i64, limit_price: Option<i64>, timestamp: f64 }
StockTradeExecuted { trade_id: String, symbol: String, buyer_id: String, seller_id: String, quantity: i64, price: i64, timestamp: f64 }
StockPriceUpdated { symbol: String, old_price: i64, new_price: i64, market_cap: i64, timestamp: f64 }
DividendPaid { symbol: String, per_share: i64, total: i64, timestamp: f64 }
```

**Logging**:
```rust
info!("Stock listed: {} ({}) - {} shares at {} credits", symbol, company_id, shares, initial_price);
debug!("Stock order: {:?} {} {} @ {:?}", order_type, quantity, symbol, limit_price);
info!("Stock trade: {} {} @ {} ({} -> {})", quantity, symbol, price, seller_id, buyer_id);
info!("Stock price updated: {} {} -> {} (market cap: {})", symbol, old_price, new_price, market_cap);
info!("Dividend paid: {} credits per share (total: {})", per_share, total);
```

**Property Tests**:
```rust
proptest! {
    #[test]
    fn market_orders_always_execute_if_counterparty_exists(
        buy_quantity in 1i64..1000,
        sell_quantity in 1i64..1000,
        price in 100i64..10000
    ) {
        // Market orders should always match if there's a counterparty
    }

    #[test]
    fn stock_price_never_negative(trades in vec(arbitrary_stock_trade(), 0..100)) {
        // After any sequence of trades, price should be positive
    }

    #[test]
    fn market_cap_equals_price_times_shares(
        shares in 1000i64..1000000,
        price in 1i64..10000
    ) {
        let market_cap = shares * price;
        prop_assert!(market_cap >= shares && market_cap >= price);
    }

    #[test]
    fn dividend_distribution_equals_total_payment(
        shares in 1000i64..1000000,
        dividend_per_share in 1i64..100
    ) {
        let total = shares * dividend_per_share;
        prop_assert_eq!(total, shares * dividend_per_share);
    }
}
```

**Acceptance Criteria**:
- [ ] Stock listing system
- [ ] Limit and market order support
- [ ] Order matching with price-time priority
- [ ] Stock price updates based on trades
- [ ] Dividend distribution mechanics
- [ ] Property tests verify market integrity
- [ ] Events logged for all stock operations

---

### Banking and Financial Markets Summary

**Total Tasks**: 3
**Estimated Complexity**: High
**Dependencies**: Resources system (Credits currency)

**New Systems**:
- Banking with loans and credit ratings
- Bond market with coupon payments
- Stock market with dividends

**Testing Coverage**:
- 15+ property-based tests
- Financial mathematics validation
- Market integrity tests

**Events Introduced**: 15+
**Logging Points**: 25+

---

*[Document continues with remaining sections 4-13...]*

---

## 4. Production Chains Expansion

### Overview
Expand the existing 12 production recipes to include complex multi-stage production chains with alternative production methods, byproducts, and production efficiency mechanics.

### Current State
- **Existing**: 12 basic recipes in `domain/production_chain.rs`
- **Examples**: IronOre→Steel, OilToFuel
- **System**: Single-input, single-output primarily

### Architecture Enhancement

**Multi-Stage Production**:
```rust
// File: crates/outpost-core/src/domain/production_chain.rs (extend)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionChain {
    pub chain_id: String,
    pub name: String,
    pub stages: Vec<ProductionStage>,
    pub total_time: u32,
    pub efficiency_rating: f32,  // 0.0 - 1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionStage {
    pub stage_number: u32,
    pub recipe: Recipe,
    pub required_building: BuildingType,
    pub alternative_methods: Vec<AlternativeMethod>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlternativeMethod {
    pub method_name: String,
    pub recipe: Recipe,
    pub efficiency_modifier: f32,
    pub required_tech: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub inputs: Vec<(ResourceType, i64)>,
    pub outputs: Vec<(ResourceType, i64)>,
    pub byproducts: Vec<(ResourceType, i64, f32)>,  // (resource, amount, probability)
    pub energy_cost: i64,
    pub processing_time: u32,
}
```

### Tasks

#### Task 4.1: Implement Steel Production Chain
**File**: `crates/outpost-core/src/domain/production_chain.rs`

**Objective**: Create a comprehensive steel production chain with multiple methods.

**Implementation**:
```rust
// Method 1: Basic Oxygen Furnace (BOF)
const STEEL_PRODUCTION_BOF: ProductionChain = ProductionChain {
    chain_id: "steel_bof".to_string(),
    name: "Steel Production (Basic Oxygen Furnace)".to_string(),
    stages: vec![
        // Stage 1: Iron ore processing
        ProductionStage {
            stage_number: 1,
            recipe: Recipe {
                inputs: vec![(ResourceType::IronOre, 100)],
                outputs: vec![(ResourceType::Iron, 70)],
                byproducts: vec![
                    (ResourceType::Slag, 20, 1.0),
                    (ResourceType::Dust, 5, 0.8),
                ],
                energy_cost: 500,
                processing_time: 2,
            },
            required_building: BuildingType::Refinery,
            alternative_methods: vec![],
        },
        // Stage 2: Steelmaking
        ProductionStage {
            stage_number: 2,
            recipe: Recipe {
                inputs: vec![
                    (ResourceType::Iron, 100),
                    (ResourceType::Carbon, 2),
                    (ResourceType::Oxygen, 10),
                    (ResourceType::Limestone, 5),
                ],
                outputs: vec![(ResourceType::Steel, 95)],
                byproducts: vec![
                    (ResourceType::CarbonDioxide, 15, 1.0),
                    (ResourceType::Slag, 8, 1.0),
                ],
                energy_cost: 1000,
                processing_time: 3,
            },
            required_building: BuildingType::Refinery,
            alternative_methods: vec![
                // Electric Arc Furnace (EAF) method
                AlternativeMethod {
                    method_name: "Electric Arc Furnace".to_string(),
                    recipe: Recipe {
                        inputs: vec![
                            (ResourceType::IronScrap, 100),
                            (ResourceType::Carbon, 1),
                        ],
                        outputs: vec![(ResourceType::Steel, 98)],
                        byproducts: vec![(ResourceType::Slag, 5, 1.0)],
                        energy_cost: 800,  // More energy but cleaner
                        processing_time: 2,
                    },
                    efficiency_modifier: 1.1,
                    required_tech: Some("electric_arc_furnace".to_string()),
                },
            ],
        },
    ],
    total_time: 5,
    efficiency_rating: 0.95,
};

// Method 2: Direct Reduced Iron (DRI) + EAF
const STEEL_PRODUCTION_DRI: ProductionChain = ProductionChain {
    chain_id: "steel_dri".to_string(),
    name: "Steel Production (Direct Reduction)".to_string(),
    stages: vec![
        // Stage 1: Direct reduction
        ProductionStage {
            stage_number: 1,
            recipe: Recipe {
                inputs: vec![
                    (ResourceType::IronOre, 100),
                    (ResourceType::NaturalGas, 50),
                ],
                outputs: vec![(ResourceType::DirectReducedIron, 90)],
                byproducts: vec![(ResourceType::CarbonDioxide, 10, 1.0)],
                energy_cost: 300,
                processing_time: 3,
            },
            required_building: BuildingType::Refinery,
            alternative_methods: vec![],
        },
        // Stage 2: EAF melting
        ProductionStage {
            stage_number: 2,
            recipe: Recipe {
                inputs: vec![(ResourceType::DirectReducedIron, 100)],
                outputs: vec![(ResourceType::Steel, 98)],
                byproducts: vec![(ResourceType::Slag, 3, 1.0)],
                energy_cost: 700,
                processing_time: 2,
            },
            required_building: BuildingType::Refinery,
            alternative_methods: vec![],
        },
    ],
    total_time: 5,
    efficiency_rating: 0.98,  // Higher efficiency, lower emissions
};
```

**Events**:
```rust
ProductionChainStarted { chain_id: String, colony_id: String, timestamp: f64 }
ProductionStageCompleted { chain_id: String, stage: u32, outputs: Vec<(ResourceType, i64)>, timestamp: f64 }
ByproductGenerated { chain_id: String, byproduct: ResourceType, amount: i64, timestamp: f64 }
ProductionChainCompleted { chain_id: String, total_time: u32, efficiency: f32, timestamp: f64 }
ProductionMethodSwitched { chain_id: String, old_method: String, new_method: String, timestamp: f64 }
```

**Logging**:
```rust
info!("Started production chain: {}", chain_name);
debug!("Stage {} completed: {:?} outputs", stage, outputs);
info!("Byproduct generated: {} units of {:?}", amount, byproduct);
info!("Production chain completed in {} turns at {}% efficiency", time, efficiency * 100.0);
info!("Switched production method: {} -> {}", old_method, new_method);
```

**Property Tests**:
```rust
proptest! {
    #[test]
    fn production_chains_preserve_mass_approximately(chain in arbitrary_production_chain()) {
        // Sum of input masses ~= sum of output masses (allowing for waste)
        let input_mass: f32 = chain.stages.iter()
            .flat_map(|s| &s.recipe.inputs)
            .map(|(rt, amt)| ResourceProperties::for_type(*rt).density * (*amt as f32))
            .sum();

        let output_mass: f32 = chain.stages.iter()
            .flat_map(|s| &s.recipe.outputs)
            .map(|(rt, amt)| ResourceProperties::for_type(*rt).density * (*amt as f32))
            .sum();

        prop_assert!((output_mass / input_mass) >= 0.5 && (output_mass / input_mass) <= 1.0);
    }

    #[test]
    fn alternative_methods_have_tradeoffs(
        base_method in arbitrary_production_stage(),
        alt_method in arbitrary_alternative_method()
    ) {
        // Alternative should be better in some way (efficiency, time, or cost)
        if alt_method.efficiency_modifier > 1.0 {
            // Higher efficiency might mean higher energy cost
            prop_assert!(alt_method.recipe.energy_cost >= base_method.recipe.energy_cost * 0.9);
        }
    }

    #[test]
    fn byproduct_probability_always_valid(recipe in arbitrary_recipe()) {
        for (_, _, prob) in &recipe.byproducts {
            prop_assert!(*prob >= 0.0 && *prob <= 1.0);
        }
    }
}
```

**Image Verification**:
- Production chain visualization UI (flow diagram)
- Building pipeline status display
- Resource flow animation

**Acceptance Criteria**:
- [ ] Steel production chain with 2+ methods
- [ ] Multi-stage production with byproducts
- [ ] Alternative method switching
- [ ] Property tests verify mass conservation
- [ ] Events logged for all stages

---

#### Task 4.2: Implement Electronics Production Chain
**File**: `crates/outpost-core/src/domain/production_chain.rs`

**Objective**: Create a complex electronics manufacturing chain.

**Implementation**:
```rust
const ELECTRONICS_PRODUCTION: ProductionChain = ProductionChain {
    chain_id: "electronics".to_string(),
    name: "Electronics Manufacturing".to_string(),
    stages: vec![
        // Stage 1: Silicon wafer production
        ProductionStage {
            stage_number: 1,
            recipe: Recipe {
                inputs: vec![
                    (ResourceType::Silicon, 10),
                    (ResourceType::Energy, 500),
                ],
                outputs: vec![(ResourceType::SiliconWafer, 8)],
                byproducts: vec![(ResourceType::SiliconDust, 1, 0.9)],
                energy_cost: 500,
                processing_time: 4,
            },
            required_building: BuildingType::Factory,
            alternative_methods: vec![],
        },
        // Stage 2: Chip fabrication
        ProductionStage {
            stage_number: 2,
            recipe: Recipe {
                inputs: vec![
                    (ResourceType::SiliconWafer, 10),
                    (ResourceType::Chemicals, 50),
                    (ResourceType::RareEarthElements, 2),
                ],
                outputs: vec![(ResourceType::Microchips, 100)],
                byproducts: vec![
                    (ResourceType::ChemicalWaste, 30, 1.0),
                    (ResourceType::DefectiveChips, 5, 0.3),
                ],
                energy_cost: 800,
                processing_time: 5,
            },
            required_building: BuildingType::Factory,
            alternative_methods: vec![],
        },
        // Stage 3: PCB assembly
        ProductionStage {
            stage_number: 3,
            recipe: Recipe {
                inputs: vec![
                    (ResourceType::Microchips, 50),
                    (ResourceType::Copper, 20),
                    (ResourceType::Plastics, 10),
                    (ResourceType::Solder, 5),
                ],
                outputs: vec![(ResourceType::Electronics, 10)],
                byproducts: vec![(ResourceType::ElectronicScrap, 2, 0.2)],
                energy_cost: 300,
                processing_time: 3,
            },
            required_building: BuildingType::Factory,
            alternative_methods: vec![],
        },
    ],
    total_time: 12,
    efficiency_rating: 0.85,
};
```

**Acceptance Criteria**:
- [ ] 3-stage electronics production chain
- [ ] Defect/waste byproduct mechanics
- [ ] High energy and chemical requirements
- [ ] Events logged at each stage

---

#### Task 4.3: Implement Food Processing Chains
**File**: `crates/outpost-core/src/domain/production_chain.rs`

**Objective**: Create various food processing chains from raw ingredients to meals.

**Implementation**:
```rust
// Example: Bread production
const BREAD_PRODUCTION: ProductionChain = ProductionChain {
    chain_id: "bread".to_string(),
    name: "Bread Baking".to_string(),
    stages: vec![
        // Stage 1: Milling
        ProductionStage {
            stage_number: 1,
            recipe: Recipe {
                inputs: vec![(ResourceType::Grain, 100)],
                outputs: vec![(ResourceType::Flour, 90)],
                byproducts: vec![(ResourceType::Bran, 8, 1.0)],
                energy_cost: 50,
                processing_time: 1,
            },
            required_building: BuildingType::Farm,
            alternative_methods: vec![],
        },
        // Stage 2: Baking
        ProductionStage {
            stage_number: 2,
            recipe: Recipe {
                inputs: vec![
                    (ResourceType::Flour, 100),
                    (ResourceType::Water, 50),
                    (ResourceType::Yeast, 2),
                    (ResourceType::Salt, 3),
                ],
                outputs: vec![(ResourceType::Bread, 120)],
                byproducts: vec![(ResourceType::CarbonDioxide, 5, 1.0)],
                energy_cost: 200,
                processing_time: 2,
            },
            required_building: BuildingType::Farm,
            alternative_methods: vec![],
        },
    ],
    total_time: 3,
    efficiency_rating: 0.9,
};

// Example: Preserved food
const FOOD_PRESERVATION: ProductionChain = ProductionChain {
    chain_id: "food_preservation".to_string(),
    name: "Food Preservation".to_string(),
    stages: vec![
        ProductionStage {
            stage_number: 1,
            recipe: Recipe {
                inputs: vec![
                    (ResourceType::Food, 100),
                    (ResourceType::Salt, 10),
                    (ResourceType::Energy, 100),
                ],
                outputs: vec![(ResourceType::PreservedFood, 95)],
                byproducts: vec![(ResourceType::Brine, 10, 1.0)],
                energy_cost: 100,
                processing_time: 2,
            },
            required_building: BuildingType::Farm,
            alternative_methods: vec![
                // Freezing method
                AlternativeMethod {
                    method_name: "Flash Freezing".to_string(),
                    recipe: Recipe {
                        inputs: vec![
                            (ResourceType::Food, 100),
                            (ResourceType::Energy, 200),
                        ],
                        outputs: vec![(ResourceType::FrozenFood, 99)],
                        byproducts: vec![],
                        energy_cost: 200,
                        processing_time: 1,
                    },
                    efficiency_modifier: 1.05,
                    required_tech: Some("cryogenics".to_string()),
                },
            ],
        },
    ],
    total_time: 2,
    efficiency_rating: 0.95,
};
```

**Acceptance Criteria**:
- [ ] 5+ food processing chains
- [ ] Preservation methods with extended shelf life
- [ ] Byproduct utilization (bran, etc.)
- [ ] Alternative preservation methods

---

### Production Chains Summary

**Total Tasks**: 3 main categories (Steel, Electronics, Food)
**Total Production Chains**: 15+
**Estimated Complexity**: Medium-High

**New Features**:
- Multi-stage production
- Alternative production methods
- Byproduct generation with probability
- Production efficiency tracking

**Testing Coverage**:
- 10+ property-based tests
- Mass conservation validation
- Efficiency/cost tradeoff tests

**Events Introduced**: 5+
**Logging Points**: 10+

---

## 5. Satellite Launch System

### Overview
Implement a complete satellite and space launch infrastructure, including launch pad construction, satellite/rocket manufacturing, launch planning, orbital mechanics (simplified), and satellite operations.

### Architecture

**Core Entities**:
```rust
// File: crates/outpost-core/src/domain/satellite.rs (new)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Satellite {
    pub satellite_id: String,
    pub name: String,
    pub satellite_type: SatelliteType,
    pub mass: f32,  // kg
    pub orbit: Orbit,
    pub status: SatelliteStatus,
    pub power_available: i64,
    pub fuel_remaining: f32,
    pub health: f32,  // 0.0 - 1.0
    pub deployed_turn: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SatelliteType {
    Communications,
    Observatory,
    Weather,
    Navigation,
    ScienceProbe,
    ResourceScanner,
    Military,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Orbit {
    pub altitude: f32,  // km
    pub inclination: f32,  // degrees
    pub orbit_type: OrbitType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrbitType {
    LowOrbit,      // < 2000 km
    MediumOrbit,   // 2000 - 35786 km
    Geostationary, // 35786 km
    HighOrbit,     // > 35786 km
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SatelliteStatus {
    InProduction,
    ReadyForLaunch,
    InTransit,
    Operational,
    Degraded,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchVehicle {
    pub vehicle_id: String,
    pub name: String,
    pub vehicle_class: LaunchClass,
    pub payload_capacity: f32,  // kg
    pub reliability: f32,  // 0.0 - 1.0
    pub fuel_type: ResourceType,
    pub fuel_required: i64,
    pub cost: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LaunchClass {
    Light,      // < 2000 kg to LEO
    Medium,     // 2000 - 20000 kg
    Heavy,      // 20000 - 50000 kg
    SuperHeavy, // > 50000 kg
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchPad {
    pub pad_id: String,
    pub colony_id: String,
    pub pad_status: PadStatus,
    pub supported_vehicle_classes: Vec<LaunchClass>,
    pub turnaround_time: u32,  // turns
    pub last_launch_turn: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PadStatus {
    Idle,
    PreparingLaunch,
    Launching,
    Maintenance,
    Damaged,
}
```

### Tasks

#### Task 5.1: Implement Launch Infrastructure Buildings
**File**: `crates/outpost-core/src/domain/building.rs` (extend)

**Objective**: Add building types for satellite and launch vehicle production.

**Implementation**:
```rust
// Add to BuildingType enum
pub enum BuildingType {
    // ... existing ...

    // Launch infrastructure
    LaunchPad,
    VehicleAssemblyBuilding,
    SatelliteIntegrationFacility,
    MissionControl,
    TrackingStation,
    FuelDepot,
}

impl Building {
    pub fn launch_infrastructure_cost(building_type: BuildingType) -> Vec<(ResourceType, i64)> {
        match building_type {
            BuildingType::LaunchPad => vec![
                (ResourceType::Concrete, 5000),
                (ResourceType::Steel, 2000),
                (ResourceType::Electronics, 500),
                (ResourceType::Credits, 1000000),
            ],
            BuildingType::VehicleAssemblyBuilding => vec![
                (ResourceType::Steel, 3000),
                (ResourceType::Concrete, 2000),
                (ResourceType::Machinery, 1000),
                (ResourceType::Credits, 500000),
            ],
            BuildingType::SatelliteIntegrationFacility => vec![
                (ResourceType::Steel, 1000),
                (ResourceType::Electronics, 800),
                (ResourceType::AdvancedComponents, 200),
                (ResourceType::Credits, 750000),
            ],
            BuildingType::MissionControl => vec![
                (ResourceType::Concrete, 500),
                (ResourceType::Computers, 200),
                (ResourceType::Servers, 100),
                (ResourceType::Credits, 300000),
            ],
            BuildingType::TrackingStation => vec![
                (ResourceType::Steel, 200),
                (ResourceType::Electronics, 300),
                (ResourceType::Sensors, 50),
                (ResourceType::Credits, 200000),
            ],
            BuildingType::FuelDepot => vec![
                (ResourceType::Steel, 1500),
                (ResourceType::Concrete, 1000),
                (ResourceType::PressurizedTanks, 100),
                (ResourceType::Credits, 400000),
            ],
            _ => vec![],
        }
    }
}
```

**Events**:
```rust
LaunchInfrastructureBuilt { building_type: BuildingType, colony_id: String, timestamp: f64 }
LaunchPadStatusChanged { pad_id: String, old_status: PadStatus, new_status: PadStatus, timestamp: f64 }
```

**Logging**:
```rust
info!("Built {} for satellite launch operations", building_type);
info!("Launch pad {} status: {:?} -> {:?}", pad_id, old_status, new_status);
```

**Acceptance Criteria**:
- [ ] 6 launch infrastructure building types
- [ ] High construction costs (realistic)
- [ ] Events logged for construction
- [ ] Property tests verify cost scaling

---

#### Task 5.2: Implement Satellite Manufacturing
**File**: `crates/outpost-core/src/domain/satellite.rs`

**Objective**: Create production chains for different satellite types.

**Implementation**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SatelliteBlueprint {
    pub satellite_type: SatelliteType,
    pub base_mass: f32,
    pub components_required: Vec<(ComponentType, i64)>,
    pub production_time: u32,
    pub production_cost: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComponentType {
    SolarPanels,
    Battery,
    Antenna,
    Transponder,
    NavigationSensors,
    Camera,
    Telescope,
    Spectrometer,
    Thruster,
    FuelTank,
    Computer,
    RadiationShielding,
}

impl SatelliteBlueprint {
    pub fn for_type(satellite_type: SatelliteType) -> Self {
        match satellite_type {
            SatelliteType::Communications => SatelliteBlueprint {
                satellite_type,
                base_mass: 500.0,
                components_required: vec![
                    (ComponentType::SolarPanels, 4),
                    (ComponentType::Battery, 2),
                    (ComponentType::Antenna, 3),
                    (ComponentType::Transponder, 2),
                    (ComponentType::Computer, 1),
                    (ComponentType::Thruster, 2),
                ],
                production_time: 10,
                production_cost: 100000,
            },
            SatelliteType::Observatory => SatelliteBlueprint {
                satellite_type,
                base_mass: 1200.0,
                components_required: vec![
                    (ComponentType::SolarPanels, 6),
                    (ComponentType::Battery, 3),
                    (ComponentType::Telescope, 1),
                    (ComponentType::Spectrometer, 2),
                    (ComponentType::Camera, 3),
                    (ComponentType::Computer, 2),
                    (ComponentType::Thruster, 4),
                ],
                production_time: 15,
                production_cost: 250000,
            },
            SatelliteType::ResourceScanner => SatelliteBlueprint {
                satellite_type,
                base_mass: 800.0,
                components_required: vec![
                    (ComponentType::SolarPanels, 5),
                    (ComponentType::Battery, 2),
                    (ComponentType::Spectrometer, 3),
                    (ComponentType::Camera, 2),
                    (ComponentType::Computer, 2),
                    (ComponentType::Thruster, 3),
                ],
                production_time: 12,
                production_cost: 180000,
            },
            // ... other types
        }
    }
}

pub fn manufacture_satellite(
    blueprint: &SatelliteBlueprint,
    components: &mut HashMap<ComponentType, i64>,
) -> Result<Satellite, ManufacturingError> {
    // Check all components available
    for (component_type, required) in &blueprint.components_required {
        let available = components.get(component_type).unwrap_or(&0);
        if available < required {
            return Err(ManufacturingError::InsufficientComponents {
                component: *component_type,
                required: *required,
                available: *available,
            });
        }
    }

    // Consume components
    for (component_type, required) in &blueprint.components_required {
        *components.get_mut(component_type).unwrap() -= required;
    }

    // Create satellite
    Ok(Satellite {
        satellite_id: Uuid::new_v4().to_string(),
        name: format!("{:?} Satellite", blueprint.satellite_type),
        satellite_type: blueprint.satellite_type,
        mass: blueprint.base_mass,
        orbit: Orbit {
            altitude: 0.0,  // Not yet launched
            inclination: 0.0,
            orbit_type: OrbitType::LowOrbit,
        },
        status: SatelliteStatus::ReadyForLaunch,
        power_available: 1000,  // watts
        fuel_remaining: 100.0,  // kg
        health: 1.0,
        deployed_turn: 0,
    })
}
```

**Events**:
```rust
SatelliteProductionStarted { satellite_type: SatelliteType, estimated_completion: u64, timestamp: f64 }
SatelliteManufactured { satellite_id: String, satellite_type: SatelliteType, mass: f32, timestamp: f64 }
ComponentProduced { component_type: ComponentType, quantity: i64, timestamp: f64 }
```

**Logging**:
```rust
info!("Started satellite production: {:?}, ETA: {} turns", satellite_type, eta);
info!("Manufactured satellite: {} ({:?}), mass: {} kg", satellite_id, satellite_type, mass);
debug!("Produced {} units of {:?}", quantity, component_type);
```

**Property Tests**:
```rust
proptest! {
    #[test]
    fn satellite_mass_proportional_to_components(
        satellite_type in arbitrary_satellite_type()
    ) {
        let blueprint = SatelliteBlueprint::for_type(satellite_type);
        let component_count: i64 = blueprint.components_required.iter()
            .map(|(_, qty)| qty)
            .sum();
        // More components generally means more mass
        prop_assert!(blueprint.base_mass > (component_count as f32 * 10.0));
    }

    #[test]
    fn manufacturing_consumes_exact_components(
        blueprint in arbitrary_satellite_blueprint(),
        initial_components in arbitrary_component_inventory()
    ) {
        // Verify manufacturing consumes exactly the required components
    }
}
```

**Acceptance Criteria**:
- [ ] 7 satellite type blueprints
- [ ] 12 component types defined
- [ ] Component-based manufacturing system
- [ ] Property tests verify manufacturing logic
- [ ] Events logged for production

---

#### Task 5.3: Implement Launch Vehicle System
**File**: `crates/outpost-core/src/domain/satellite.rs`

**Objective**: Create launch vehicle definitions and manufacturing.

**Implementation**:
```rust
impl LaunchVehicle {
    pub fn light_launcher() -> Self {
        LaunchVehicle {
            vehicle_id: Uuid::new_v4().to_string(),
            name: "Pegasus-L".to_string(),
            vehicle_class: LaunchClass::Light,
            payload_capacity: 1800.0,
            reliability: 0.92,
            fuel_type: ResourceType::RocketFuel,
            fuel_required: 5000,
            cost: 50000,
        }
    }

    pub fn medium_launcher() -> Self {
        LaunchVehicle {
            vehicle_id: Uuid::new_v4().to_string(),
            name: "Atlas-M".to_string(),
            vehicle_class: LaunchClass::Medium,
            payload_capacity: 15000.0,
            reliability: 0.95,
            fuel_type: ResourceType::RocketFuel,
            fuel_required: 25000,
            cost: 150000,
        }
    }

    pub fn heavy_launcher() -> Self {
        LaunchVehicle {
            vehicle_id: Uuid::new_v4().to_string(),
            name: "Titan-H".to_string(),
            vehicle_class: LaunchClass::Heavy,
            payload_capacity: 45000.0,
            reliability: 0.97,
            fuel_type: ResourceType::RocketFuel,
            fuel_required: 100000,
            cost: 400000,
        }
    }

    pub fn can_launch(&self, payload_mass: f32) -> bool {
        payload_mass <= self.payload_capacity
    }

    pub fn calculate_launch_probability(&self, payload_mass: f32, weather_factor: f32) -> f32 {
        let capacity_factor = 1.0 - (payload_mass / self.payload_capacity);
        let base_reliability = self.reliability;

        (base_reliability + capacity_factor * 0.05) * weather_factor
    }
}
```

**Events**:
```rust
LaunchVehicleManufactured { vehicle_id: String, vehicle_class: LaunchClass, timestamp: f64 }
LaunchVehicleFueled { vehicle_id: String, fuel_amount: i64, timestamp: f64 }
```

**Logging**:
```rust
info!("Manufactured launch vehicle: {} ({}), payload capacity: {} kg", name, vehicle_class, payload_capacity);
info!("Launch vehicle {} fueled with {} units of {:?}", vehicle_id, fuel_amount, fuel_type);
```

**Acceptance Criteria**:
- [ ] 4 launch vehicle classes
- [ ] Payload capacity limits
- [ ] Reliability calculations
- [ ] Events logged for vehicle operations

---

#### Task 5.4: Implement Launch Planning and Execution
**File**: `crates/outpost-core/src/domain/satellite.rs`

**Objective**: Create launch mission planning and execution system.

**Implementation**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchMission {
    pub mission_id: String,
    pub satellite_id: String,
    pub vehicle_id: String,
    pub pad_id: String,
    pub target_orbit: Orbit,
    pub launch_turn: u64,
    pub mission_status: MissionStatus,
    pub success_probability: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissionStatus {
    Planning,
    Preparation,
    ReadyForLaunch,
    Launching,
    InFlight,
    Success,
    Failure,
    Aborted,
}

pub fn plan_launch(
    satellite: &Satellite,
    vehicle: &LaunchVehicle,
    pad: &LaunchPad,
    target_orbit: Orbit,
    current_turn: u64,
) -> Result<LaunchMission, LaunchError> {
    // Verify vehicle can carry satellite
    if !vehicle.can_launch(satellite.mass) {
        return Err(LaunchError::PayloadTooHeavy {
            payload: satellite.mass,
            capacity: vehicle.payload_capacity,
        });
    }

    // Verify pad can support vehicle
    if !pad.supported_vehicle_classes.contains(&vehicle.vehicle_class) {
        return Err(LaunchError::IncompatiblePad);
    }

    // Verify pad is available
    if pad.pad_status != PadStatus::Idle {
        return Err(LaunchError::PadNotAvailable);
    }

    // Calculate launch window (simplified)
    let preparation_time = match vehicle.vehicle_class {
        LaunchClass::Light => 2,
        LaunchClass::Medium => 4,
        LaunchClass::Heavy => 6,
        LaunchClass::SuperHeavy => 10,
    };

    let launch_turn = current_turn + preparation_time;

    // Calculate success probability
    let success_probability = vehicle.calculate_launch_probability(
        satellite.mass,
        1.0,  // Weather factor (simplified)
    );

    Ok(LaunchMission {
        mission_id: Uuid::new_v4().to_string(),
        satellite_id: satellite.satellite_id.clone(),
        vehicle_id: vehicle.vehicle_id.clone(),
        pad_id: pad.pad_id.clone(),
        target_orbit,
        launch_turn,
        mission_status: MissionStatus::Planning,
        success_probability,
    })
}

pub fn execute_launch(
    mission: &mut LaunchMission,
    satellite: &mut Satellite,
    rng: &mut impl rand::Rng,
) -> Result<LaunchOutcome, LaunchError> {
    if mission.mission_status != MissionStatus::ReadyForLaunch {
        return Err(LaunchError::NotReady);
    }

    mission.mission_status = MissionStatus::Launching;

    // Roll for success
    let roll: f32 = rng.gen();

    if roll < mission.success_probability {
        // Success!
        satellite.orbit = mission.target_orbit.clone();
        satellite.status = SatelliteStatus::Operational;
        mission.mission_status = MissionStatus::Success;

        Ok(LaunchOutcome::Success {
            satellite_id: satellite.satellite_id.clone(),
            orbit: mission.target_orbit.clone(),
        })
    } else {
        // Failure
        satellite.status = SatelliteStatus::Failed;
        mission.mission_status = MissionStatus::Failure;

        // Determine failure type
        let failure_roll: f32 = rng.gen();
        let failure_type = if failure_roll < 0.4 {
            LaunchFailureType::ExplosionOnPad
        } else if failure_roll < 0.7 {
            LaunchFailureType::ExplosionInFlight
        } else {
            LaunchFailureType::WrongOrbit
        };

        Ok(LaunchOutcome::Failure {
            satellite_id: satellite.satellite_id.clone(),
            failure_type,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LaunchOutcome {
    Success {
        satellite_id: String,
        orbit: Orbit,
    },
    Failure {
        satellite_id: String,
        failure_type: LaunchFailureType,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LaunchFailureType {
    ExplosionOnPad,
    ExplosionInFlight,
    WrongOrbit,
    CommunicationLoss,
}
```

**Events**:
```rust
LaunchMissionPlanned { mission_id: String, satellite_id: String, vehicle_id: String, launch_turn: u64, timestamp: f64 }
LaunchSequenceStarted { mission_id: String, success_probability: f32, timestamp: f64 }
LaunchSuccessful { mission_id: String, satellite_id: String, orbit: Orbit, timestamp: f64 }
LaunchFailed { mission_id: String, satellite_id: String, failure_type: LaunchFailureType, timestamp: f64 }
```

**Logging**:
```rust
info!("Launch mission planned: {}, scheduled for turn {}", mission_id, launch_turn);
info!("Launch sequence initiated: {} ({}% success probability)", mission_id, success_prob * 100.0);
info!("LAUNCH SUCCESS! Satellite {} in {:?} orbit", satellite_id, orbit.orbit_type);
error!("LAUNCH FAILURE! Satellite {}: {:?}", satellite_id, failure_type);
```

**Property Tests**:
```rust
proptest! {
    #[test]
    fn success_probability_always_valid(
        payload_mass in 100.0f32..50000.0,
        capacity in 1000.0f32..100000.0,
        reliability in 0.8f32..0.99
    ) {
        let vehicle = LaunchVehicle {
            payload_capacity: capacity,
            reliability,
            ..Default::default()
        };
        let prob = vehicle.calculate_launch_probability(payload_mass, 1.0);
        prop_assert!(prob >= 0.0 && prob <= 1.0);
    }

    #[test]
    fn lighter_payloads_increase_success_probability(
        capacity in 10000.0f32..50000.0,
        reliability in 0.9f32..0.95
    ) {
        let vehicle = LaunchVehicle {
            payload_capacity: capacity,
            reliability,
            ..Default::default()
        };
        let light_prob = vehicle.calculate_launch_probability(capacity * 0.5, 1.0);
        let heavy_prob = vehicle.calculate_launch_probability(capacity * 0.95, 1.0);
        prop_assert!(light_prob >= heavy_prob);
    }
}
```

**Image Verification**:
- Launch mission UI showing countdown
- Success/failure animation
- Orbital diagram showing satellite position

**Acceptance Criteria**:
- [ ] Launch mission planning system
- [ ] Probabilistic launch outcomes
- [ ] Multiple failure types
- [ ] Property tests verify probability calculations
- [ ] Events logged for all launch phases

---

### Satellite Launch Summary

**Total Tasks**: 4
**Estimated Complexity**: High
**Dependencies**: Resources, Buildings, Production Chains

**New Systems**:
- Launch infrastructure buildings
- Satellite manufacturing with components
- Launch vehicle system
- Mission planning and execution
- Orbital mechanics (simplified)

**Testing Coverage**:
- 10+ property-based tests
- Probability validation
- Component consumption tests

**Events Introduced**: 15+
**Logging Points**: 20+

---

## Conclusion and Summary

This detailed plan provides comprehensive, LLM-executable task breakdowns for the first 5 major features of Milestone 7. Each feature includes:

✅ **Specific implementation tasks** with file paths and code examples
✅ **Event definitions** for complete event sourcing
✅ **Logging requirements** using tracing crate
✅ **Property-based tests** with proptest
✅ **Image verification** strategies where applicable
✅ **Clear acceptance criteria** for each task

### Current Progress
- ✅ Scene System: 5 tasks defined
- ✅ Resources Expansion: 6 task groups defined (100+ new resources)
- ✅ Banking/Finance: 3 systems defined (loans, bonds, stocks)
- ✅ Production Chains: 3 categories defined (steel, electronics, food)
- ✅ Satellite Launch: 4 tasks defined (full launch infrastructure)

### Remaining Features (8 more to define)
6. Planet Gateway Exploration
7. Train Mechanics Expansion
8. Galaxy Map System
9. High-Level Economy System
10. Population Migration Mechanics
11. Population Buildings
12. Underground Excavation
13. Terraforming System

---

**Next Steps**: Continue this document with sections 6-13 following the same detailed pattern, or begin implementation of the defined tasks in priority order.
