/// Property tests for content definitions
/// Verifies round-trip serialization/deserialization invariants

use outpost_core::content::*;
use proptest::prelude::*;

// ============================================================================
// Strategy generators for arbitrary content definitions
// ============================================================================

fn arb_building_category() -> impl Strategy<Value = BuildingCategory> {
    prop_oneof![
        Just(BuildingCategory::PowerGeneration),
        Just(BuildingCategory::Mining),
        Just(BuildingCategory::Refining),
        Just(BuildingCategory::Manufacturing),
        Just(BuildingCategory::Farming),
        Just(BuildingCategory::Storage),
        Just(BuildingCategory::Housing),
        Just(BuildingCategory::Services),
        Just(BuildingCategory::Entertainment),
        Just(BuildingCategory::Education),
        Just(BuildingCategory::Healthcare),
        Just(BuildingCategory::Infrastructure),
    ]
}

fn arb_construction_cost() -> impl Strategy<Value = Vec<outpost_core::content::building_def::ConstructionCost>> {
    prop::collection::vec(
        ("[a-z_]{3,15}", 1.0f64..10000.0).prop_map(|(resource_id, amount)| {
            outpost_core::content::building_def::ConstructionCost {
                resource_id,
                amount,
            }
        }),
        0..5,
    )
}

prop_compose! {
    fn arb_building_definition()
        (id in "[a-z_]{3,20}",
         name in "[A-Z][a-zA-Z ]{5,40}",
         description in "[A-Za-z .,]{10,100}",
         category in arb_building_category(),
         construction_costs in arb_construction_cost(),
         construction_time_ticks in 1u64..1000,
         power_mw in proptest::option::of(1.0f64..100.0),
         power_output_mw in proptest::option::of(1.0f64..100.0),
         worker_capacity in 0u32..100,
         storage_capacity in proptest::option::of(100.0f64..100000.0),
         housing_capacity in proptest::option::of(0u32..1000),
         morale_bonus in -50.0f64..50.0,
         upgradeable in any::<bool>(),
         upgrade_to in proptest::option::of("[a-z_]{3,20}"))
    -> BuildingDefinition
    {
        BuildingDefinition {
            id,
            name,
            description,
            category,
            construction_costs,
            construction_time_ticks,
            power: power_mw.map(|mw| outpost_core::content::building_def::PowerRequirement {
                base_mw: -mw.abs(), // Always negative for consumption
                per_level_mw: 0.0,
            }),
            power_output_mw,
            worker_capacity,
            storage_capacity,
            housing_capacity,
            recipes: vec![], // Simplified for property testing
            maintenance_cost: std::collections::HashMap::new(),
            upgradeable,
            upgrade_to,
            morale_bonus,
        }
    }
}


fn arb_resource_category() -> impl Strategy<Value = ResourceCategory> {
    prop_oneof![
        // Tier 0: Raw Materials
        Just(ResourceCategory::MetallicOre),
        Just(ResourceCategory::NonMetallicMineral),
        Just(ResourceCategory::Hydrocarbon),
        Just(ResourceCategory::AtmosphericGas),
        Just(ResourceCategory::Hydrospheric),
        Just(ResourceCategory::Agricultural),
        Just(ResourceCategory::Forestry),
        Just(ResourceCategory::Aquaculture),
        Just(ResourceCategory::Biological),
        // Tier 1: Processed Materials
        Just(ResourceCategory::Metal),
        Just(ResourceCategory::NonMetallicMaterial),
        Just(ResourceCategory::Chemical),
        Just(ResourceCategory::BiologicalProduct),
        Just(ResourceCategory::EnergyStorage),
        Just(ResourceCategory::LifeSupport),
        // Tier 2: Components
        Just(ResourceCategory::Component),
        Just(ResourceCategory::Electronics),
        Just(ResourceCategory::MechanicalParts),
        // Tier 3: Finished Goods
        Just(ResourceCategory::ConsumerGoods),
        Just(ResourceCategory::IndustrialEquipment),
        Just(ResourceCategory::Vehicle),
        // Special
        Just(ResourceCategory::Currency),
        Just(ResourceCategory::Data),
    ]
}

fn arb_resource_phase() -> impl Strategy<Value = ResourcePhase> {
    prop_oneof![
        Just(ResourcePhase::Solid),
        Just(ResourcePhase::Liquid),
        Just(ResourcePhase::Gas),
        Just(ResourcePhase::Plasma),
        Just(ResourcePhase::Virtual),
    ]
}

fn arb_storage_requirements(phase: ResourcePhase) -> impl Strategy<Value = outpost_core::content::resource_def::StorageRequirements> {
    let is_virtual = phase == ResourcePhase::Virtual;
    let phase_copy = phase.clone();
    (
        proptest::option::of(100.0f64..500.0), // min_temperature_k
        proptest::option::of(200.0f64..1000.0), // max_temperature_k
        proptest::option::of(0.1f64..100.0),   // min_pressure_atm
        proptest::option::of(1.0f64..1000.0),  // max_pressure_atm
        any::<bool>(),                         // hazardous
        proptest::option::of("[A-Za-z .,]{5,50}"), // special_handling
    )
        .prop_map(
            move |(
                min_temperature_k,
                max_temperature_k,
                min_pressure_atm,
                max_pressure_atm,
                hazardous,
                special_handling,
            )| {
                outpost_core::content::resource_def::StorageRequirements {
                    phase: phase_copy.clone(),
                    min_temperature_k: if is_virtual { None } else { min_temperature_k },
                    max_temperature_k: if is_virtual { None } else { max_temperature_k },
                    min_pressure_atm: if is_virtual { None } else { min_pressure_atm },
                    max_pressure_atm: if is_virtual { None } else { max_pressure_atm },
                    hazardous,
                    special_handling,
                }
            },
        )
}

fn arb_resource_definition() -> impl Strategy<Value = ResourceDefinition> {
    arb_resource_phase()
        .prop_flat_map(|phase| {
            let is_virtual = phase == ResourcePhase::Virtual;
            let phase_for_storage = phase.clone();
            let density_strategy: BoxedStrategy<f64> = if is_virtual {
                Just(0.0).boxed()
            } else {
                (0.1f64..20000.0).boxed()
            };
            (
                Just(phase),
                "[a-z_]{3,20}",                // id
                "[A-Z][a-zA-Z ]{3,30}",        // name
                "[A-Za-z .,]{10,100}",         // description
                arb_resource_category(),        // category
                arb_storage_requirements(phase_for_storage), // storage
                density_strategy,              // density (0.0 for virtual)
                0.0f64..10000.0,               // base_value
                any::<bool>(),                 // tradeable
                any::<bool>(),                 // extractable
                any::<bool>(),                 // consumable
                1u32..10000,                   // stack_size
            )
        })
        .prop_map(
            |(
                _phase,
                id,
                name,
                description,
                category,
                storage,
                density_kg_per_m3,
                base_value,
                tradeable,
                extractable,
                consumable,
                stack_size,
            )| {
                ResourceDefinition {
                    id,
                    name,
                    description,
                    category,
                    storage,
                    density_kg_per_m3,
                    base_value,
                    tradeable,
                    extractable,
                    consumable,
                    stack_size,
                }
            },
        )
}

fn arb_event_trigger() -> impl Strategy<Value = EventTrigger> {
    prop_oneof![
        "[a-z_]{3,20}".prop_map(|building_id| EventTrigger::BuildingConstructed { building_id }),
        (1u32..10000).prop_map(|threshold| EventTrigger::PopulationReached { threshold }),
        ("[a-z_]{3,20}", 1.0f64..10000.0, any::<bool>())
            .prop_map(|(resource_id, amount, above)| EventTrigger::ResourceThreshold { resource_id, amount, above }),
        (0.0f64..1.0).prop_map(|probability_per_tick| EventTrigger::Random { probability_per_tick }),
        (1u64..100000).prop_map(|ticks| EventTrigger::TimeBased { ticks }),
        (0.1f64..10.0).prop_map(|threshold| EventTrigger::MoraleThreshold { threshold }),
    ]
}

fn arb_event_outcome() -> impl Strategy<Value = EventOutcome> {
    prop_oneof![
        ("[a-z_]{3,20}", -1000.0f64..1000.0)
            .prop_map(|(resource_id, amount)| EventOutcome::ResourceChange { resource_id, amount }),
        (-10.0f64..10.0).prop_map(|amount| EventOutcome::MoraleChange { amount }),
        (-100i32..100).prop_map(|amount| EventOutcome::PopulationChange { amount }),
        "[a-z_]{3,20}".prop_map(|building_id| EventOutcome::UnlockBuilding { building_id }),
        "[a-z_]{3,20}".prop_map(|event_id| EventOutcome::TriggerEvent { event_id }),
        "[A-Za-z .,]{10,100}".prop_map(|text| EventOutcome::NarrativeText { text }),
    ]
}

fn arb_event_choice() -> impl Strategy<Value = outpost_core::content::event_def::EventChoice> {
    (
        "[a-z_]{3,20}",            // id
        "[A-Z][a-zA-Z ]{5,40}",    // text
        "[A-Za-z .,]{10,100}",     // description
        prop::collection::vec(arb_event_outcome(), 0..5), // outcomes
    )
        .prop_map(|(id, text, description, outcomes)| {
            outpost_core::content::event_def::EventChoice {
                id,
                text,
                description,
                outcomes,
                cost: std::collections::HashMap::new(), // Simplified
            }
        })
}

fn arb_event_definition() -> impl Strategy<Value = EventDefinition> {
    (
        "[a-z_]{3,20}",            // id
        "[A-Z][a-zA-Z ]{5,40}",    // title
        "[A-Za-z .,]{10,200}",     // description
        prop::collection::vec(arb_event_trigger(), 1..5), // triggers
        prop::collection::vec(arb_event_choice(), 1..5),  // choices
        any::<bool>(),             // repeatable
        proptest::option::of(100u64..10000), // cooldown_ticks
        0.1f64..10.0,              // weight
    )
        .prop_map(
            |(
                id,
                title,
                description,
                triggers,
                choices,
                repeatable,
                cooldown_ticks,
                weight,
            )| {
                EventDefinition {
                    id,
                    title,
                    description,
                    triggers,
                    choices,
                    repeatable,
                    cooldown_ticks,
                    weight,
                }
            },
        )
}

fn arb_tech_category() -> impl Strategy<Value = TechCategory> {
    prop_oneof![
        Just(TechCategory::Engineering),
        Just(TechCategory::PhysicalSciences),
        Just(TechCategory::LifeSciences),
        Just(TechCategory::Computing),
        Just(TechCategory::Social),
        Just(TechCategory::Exploration),
        Just(TechCategory::Military),
    ]
}

fn arb_tech_unlocks() -> impl Strategy<Value = outpost_core::content::tech_def::TechUnlocks> {
    (
        prop::collection::vec("[a-z_]{3,20}", 0..5), // buildings
        prop::collection::vec("[a-z_]{3,20}", 0..5), // resources
        prop::collection::vec("[a-z_]{3,20}", 0..5), // recipes
        prop::collection::vec("[a-z_]{3,30}", 0..5), // bonuses
        prop::collection::vec("[a-z_]{3,20}", 0..5), // events
    )
        .prop_map(|(buildings, resources, recipes, bonuses, events)| {
            outpost_core::content::tech_def::TechUnlocks {
                buildings,
                resources,
                recipes,
                bonuses,
                events,
            }
        })
}

fn arb_research_cost() -> impl Strategy<Value = Vec<outpost_core::content::tech_def::ResearchCost>> {
    prop::collection::vec(
        ("[a-z_]{3,20}", 1.0f64..1000.0).prop_map(|(resource_id, amount)| {
            outpost_core::content::tech_def::ResearchCost {
                resource_id,
                amount,
            }
        }),
        0..5,
    )
}

fn arb_tech_definition() -> impl Strategy<Value = TechDefinition> {
    (
        "[a-z_]{3,20}",            // id
        "[A-Z][a-zA-Z ]{5,40}",    // name
        "[A-Za-z .,]{10,100}",     // description
        arb_tech_category(),        // category
        1u8..10,                   // tier
        1.0f64..100000.0,          // research_cost
        1u64..10000,               // research_time_ticks
        prop::collection::vec("[a-z_]{3,20}", 0..5), // prerequisites
        arb_tech_unlocks(),         // unlocks
        arb_research_cost(),        // resource_costs
    )
        .prop_map(
            |(
                id,
                name,
                description,
                category,
                tier,
                research_cost,
                research_time_ticks,
                prerequisites,
                unlocks,
                resource_costs,
            )| {
                TechDefinition {
                    id,
                    name,
                    description,
                    category,
                    tier,
                    research_cost,
                    research_time_ticks,
                    prerequisites,
                    unlocks,
                    resource_costs,
                }
            },
        )
}

// ============================================================================
// Property tests
// ============================================================================

proptest! {
    #[test]
    fn building_roundtrip_serde_json(building in arb_building_definition()) {
        // First round-trip
        let json1 = serde_json::to_string(&building).unwrap();
        let deserialized1: BuildingDefinition = serde_json::from_str(&json1).unwrap();
        
        // Second round-trip to verify stability (handles float precision)
        let json2 = serde_json::to_string(&deserialized1).unwrap();
        let deserialized2: BuildingDefinition = serde_json::from_str(&json2).unwrap();
        
        assert_eq!(deserialized1, deserialized2);
    }

    #[test]
    fn building_roundtrip_serde_yaml(building in arb_building_definition()) {
        let yaml = serde_yaml::to_string(&building).unwrap();
        let deserialized: BuildingDefinition = serde_yaml::from_str(&yaml).unwrap();
        let yaml2 = serde_yaml::to_string(&deserialized).unwrap();
        let deserialized2: BuildingDefinition = serde_yaml::from_str(&yaml2).unwrap();
        assert_eq!(deserialized, deserialized2);
    }

    #[test]
    fn resource_roundtrip_serde_json(resource in arb_resource_definition()) {
        let json1 = serde_json::to_string(&resource).unwrap();
        let deserialized1: ResourceDefinition = serde_json::from_str(&json1).unwrap();
        let json2 = serde_json::to_string(&deserialized1).unwrap();
        let deserialized2: ResourceDefinition = serde_json::from_str(&json2).unwrap();
        assert_eq!(deserialized1, deserialized2);
    }

    #[test]
    fn resource_roundtrip_serde_yaml(resource in arb_resource_definition()) {
        let yaml = serde_yaml::to_string(&resource).unwrap();
        let deserialized: ResourceDefinition = serde_yaml::from_str(&yaml).unwrap();
        let yaml2 = serde_yaml::to_string(&deserialized).unwrap();
        let deserialized2: ResourceDefinition = serde_yaml::from_str(&yaml2).unwrap();
        assert_eq!(deserialized, deserialized2);
    }

    #[test]
    fn event_roundtrip_serde_json(event in arb_event_definition()) {
        let json1 = serde_json::to_string(&event).unwrap();
        let deserialized1: EventDefinition = serde_json::from_str(&json1).unwrap();
        let json2 = serde_json::to_string(&deserialized1).unwrap();
        let deserialized2: EventDefinition = serde_json::from_str(&json2).unwrap();
        assert_eq!(deserialized1, deserialized2);
    }

    #[test]
    fn event_roundtrip_serde_yaml(event in arb_event_definition()) {
        let yaml = serde_yaml::to_string(&event).unwrap();
        let deserialized: EventDefinition = serde_yaml::from_str(&yaml).unwrap();
        let yaml2 = serde_yaml::to_string(&deserialized).unwrap();
        let deserialized2: EventDefinition = serde_yaml::from_str(&yaml2).unwrap();
        assert_eq!(deserialized, deserialized2);
    }

    #[test]
    fn tech_roundtrip_serde_json(tech in arb_tech_definition()) {
        let json1 = serde_json::to_string(&tech).unwrap();
        let deserialized1: TechDefinition = serde_json::from_str(&json1).unwrap();
        let json2 = serde_json::to_string(&deserialized1).unwrap();
        let deserialized2: TechDefinition = serde_json::from_str(&json2).unwrap();
        assert_eq!(deserialized1, deserialized2);
    }

    #[test]
    fn tech_roundtrip_serde_yaml(tech in arb_tech_definition()) {
        let yaml = serde_yaml::to_string(&tech).unwrap();
        let deserialized: TechDefinition = serde_yaml::from_str(&yaml).unwrap();
        let yaml2 = serde_yaml::to_string(&deserialized).unwrap();
        let deserialized2: TechDefinition = serde_yaml::from_str(&yaml2).unwrap();
        assert_eq!(deserialized, deserialized2);
    }
}
