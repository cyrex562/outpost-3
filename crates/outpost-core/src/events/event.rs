use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEvent {
    pub event_id: u64,
    pub timestamp: DateTime<Utc>,
    pub turn_number: u64,
    pub event_type: EventType,
}

impl GameEvent {
    pub fn new(event_id: u64, turn_number: u64, event_type: EventType) -> Self {
        Self {
            event_id,
            timestamp: Utc::now(),
            turn_number,
            event_type,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EventType {
    // Colony events
    ColonyFounded {
        colony_id: ColonyId,
        planet_id: PlanetId,
        name: String,
        starting_resources: Resources,
    },
    BuildingConstructionStarted {
        building_id: BuildingId,
        colony_id: ColonyId,
        building_type: BuildingType,
    },
    BuildingConstructionCompleted {
        building_id: BuildingId,
        colony_id: ColonyId,
    },
    BuildingStateChanged {
        building_id: BuildingId,
        colony_id: ColonyId,
        new_state: BuildingState,
    },
    ResourcesExtracted {
        colony_id: ColonyId,
        resource_type: ResourceType,
        amount: i64,
    },
    ResourcesConsumed {
        colony_id: ColonyId,
        resource_type: ResourceType,
        amount: i64,
    },
    ResourcesProduced {
        colony_id: ColonyId,
        resource_type: ResourceType,
        amount: i64,
    },
    PopulationChanged {
        colony_id: ColonyId,
        new_population: u64,
    },
    PowerGridUpdated {
        colony_id: ColonyId,
        generation: u32,
        consumption: u32,
    },
    PopulationGrew {
        colony_id: ColonyId,
        old_population: u64,
        new_population: u64,
    },
    LaborAllocated {
        colony_id: ColonyId,
        building_id: BuildingId,
        workers_allocated: u32,
    },
    LaborDeallocated {
        colony_id: ColonyId,
        building_id: BuildingId,
        workers_deallocated: u32,
    },
    BuildingDamaged {
        building_id: BuildingId,
        colony_id: ColonyId,
        damage_severity: u8,
    },
    BuildingRepaired {
        building_id: BuildingId,
        colony_id: ColonyId,
    },
    BuildingUpgraded {
        building_id: BuildingId,
        colony_id: ColonyId,
        new_level: u8,
    },
    BuildingRecipeChanged {
        building_id: BuildingId,
        colony_id: ColonyId,
        old_recipe_index: u32,
        new_recipe_index: u32,
    },
    
    // V5 Construction events (using SiteId instead of ColonyId)
    ConstructionQueued {
        site_id: SiteId,
        building_id: BuildingId,
        building_def_id: String,
        tick: u64,
    },
    ConstructionProgressed {
        site_id: SiteId,
        building_id: BuildingId,
        progress_ticks: u64,
        total_ticks: u64,
        tick: u64,
    },
    BuildingConstructed {
        site_id: SiteId,
        building_id: BuildingId,
        building_def_id: String,
        tick: u64,
    },
    ConstructionCancelled {
        site_id: SiteId,
        building_id: BuildingId,
        refunded_resources: std::collections::HashMap<String, f64>,
        tick: u64,
    },
    ConstructionPaused {
        site_id: SiteId,
        building_id: BuildingId,
        tick: u64,
    },
    ConstructionResumed {
        site_id: SiteId,
        building_id: BuildingId,
        tick: u64,
    },

    // V5 Production events (using SiteId and string resource IDs)
    RecipeStarted {
        site_id: SiteId,
        building_id: BuildingId,
        recipe_id: String,
        tick: u64,
    },
    RecipeProgressed {
        site_id: SiteId,
        building_id: BuildingId,
        recipe_id: String,
        progress_ticks: u64,
        total_ticks: u64,
        tick: u64,
    },
    RecipeCompleted {
        site_id: SiteId,
        building_id: BuildingId,
        recipe_id: String,
        tick: u64,
    },
    ProductionInputsConsumed {
        site_id: SiteId,
        building_id: BuildingId,
        recipe_id: String,
        inputs: std::collections::HashMap<String, f64>,
        tick: u64,
    },
    ProductionOutputsProduced {
        site_id: SiteId,
        building_id: BuildingId,
        recipe_id: String,
        outputs: std::collections::HashMap<String, f64>,
        tick: u64,
    },
    ProductionHalted {
        site_id: SiteId,
        building_id: BuildingId,
        recipe_id: String,
        reason: String,
        tick: u64,
    },
    DepositDepleted {
        site_id: SiteId,
        body_id: CelestialBodyId,
        deposit_id: String,
        resource_type: String,
        tick: u64,
    },

    // Wormhole events
    PlanetDiscovered {
        planet_id: PlanetId,
        planet: Planet,
    },
    GateConstructionStarted {
        gate_id: WormholeId,
        source_planet: PlanetId,
        destination_planet: PlanetId,
    },
    GateConstructionCompleted {
        gate_id: WormholeId,
    },
    GateActivated {
        gate_id: WormholeId,
    },
    GateDeactivated {
        gate_id: WormholeId,
    },

    // Simulation events
    TurnAdvanced {
        turn_number: u64,
    },

    // Market events
    MarketPriceChanged {
        resource: ResourceType,
        old_price: f64,
        new_price: f64,
    },

    // Trading events
    ResourceTraded {
        colony_id: ColonyId,
        resource_type: ResourceType,
        quantity: i64,
        price: f64,
        side: TradeSide,
    },

    // Economy events
    EconomySnapshot {
        colony_id: ColonyId,
        gdp: i64,
        income: i64,
        expenses: i64,
        net_worth: i64,
    },

    // Banking events
    LoanIssued {
        loan_id: LoanId,
        colony_id: ColonyId,
        principal: f64,
        interest_rate: f64,
        term_turns: u32,
    },
    LoanPaymentMade {
        loan_id: LoanId,
        colony_id: ColonyId,
        amount: f64,
        principal_paid: f64,
        interest_paid: f64,
        remaining_principal: f64,
    },
    LoanPaidOff {
        loan_id: LoanId,
        colony_id: ColonyId,
    },

    // V5 Power and Life Support events (use SiteId)
    PowerGridUpdatedV5 {
        site_id: SiteId,
        generation_mw: f64,
        consumption_mw: f64,
        has_brownout: bool,
        power_efficiency: f64,
        tick: u64,
    },
    LifeSupportUpdated {
        site_id: SiteId,
        oxygen_sat: f32,
        water_sat: f32,
        food_sat: f32,
        is_critical: bool,
        tick: u64,
    },
    /// Life support has dropped to warning level (overall score < 0.75 but not critical).
    LifeSupportWarning {
        site_id: SiteId,
        oxygen_sat: f32,
        water_sat: f32,
        food_sat: f32,
        tick: u64,
    },
    /// Life support is in critical state (any satisfaction < 0.25). Triggers auto-pause.
    LifeSupportCritical {
        site_id: SiteId,
        oxygen_sat: f32,
        water_sat: f32,
        food_sat: f32,
        tick: u64,
    },
    BuildingPowerToggled {
        site_id: SiteId,
        building_id: BuildingId,
        powered_on: bool,
        tick: u64,
    },

    // V5 Population events (using SiteId)
    PopulationGrewV5 {
        site_id: SiteId,
        old_population: u64,
        new_population: u64,
        tick: u64,
    },
    ColonistsDiedV5 {
        site_id: SiteId,
        deaths: u64,
        cause: String,
        tick: u64,
    },

    /// A gameplay event (from the event engine / events.yaml) has fired for a site.
    GameEventFired {
        site_id: SiteId,
        event_id: String,
        title: String,
        tick: u64,
        /// True if the event has choices the player must resolve.
        requires_choice: bool,
    },

    /// A player has resolved a pending event choice.
    GameEventChoiceResolved {
        site_id: SiteId,
        event_id: String,
        choice_id: String,
        tick: u64,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TradeSide {
    Buy,
    Sell,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_recipe_started_event_serialization() {
        let event = GameEvent::new(
            1,
            100,
            EventType::RecipeStarted {
                site_id: SiteId::new(),
                building_id: BuildingId::new(),
                recipe_id: "smelt_iron".to_string(),
                tick: 100,
            },
        );

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("RecipeStarted"));
        assert!(json.contains("smelt_iron"));

        let deserialized: GameEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event.event_id, deserialized.event_id);
        assert_eq!(event.turn_number, deserialized.turn_number);
    }

    #[test]
    fn test_recipe_progressed_event_serialization() {
        let event = GameEvent::new(
            2,
            101,
            EventType::RecipeProgressed {
                site_id: SiteId::new(),
                building_id: BuildingId::new(),
                recipe_id: "smelt_iron".to_string(),
                progress_ticks: 5,
                total_ticks: 10,
                tick: 101,
            },
        );

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("RecipeProgressed"));
        assert!(json.contains("\"progress_ticks\":5"));
        assert!(json.contains("\"total_ticks\":10"));

        let deserialized: GameEvent = serde_json::from_str(&json).unwrap();
        match deserialized.event_type {
            EventType::RecipeProgressed { progress_ticks, total_ticks, .. } => {
                assert_eq!(progress_ticks, 5);
                assert_eq!(total_ticks, 10);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_recipe_completed_event_serialization() {
        let event = GameEvent::new(
            3,
            110,
            EventType::RecipeCompleted {
                site_id: SiteId::new(),
                building_id: BuildingId::new(),
                recipe_id: "smelt_iron".to_string(),
                tick: 110,
            },
        );

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("RecipeCompleted"));

        let deserialized: GameEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event.event_id, deserialized.event_id);
    }

    #[test]
    fn test_production_inputs_consumed_event() {
        let mut inputs = HashMap::new();
        inputs.insert("iron_ore".to_string(), 15.0);
        inputs.insert("power".to_string(), 20.0);

        let event = GameEvent::new(
            4,
            100,
            EventType::ProductionInputsConsumed {
                site_id: SiteId::new(),
                building_id: BuildingId::new(),
                recipe_id: "smelt_iron".to_string(),
                inputs: inputs.clone(),
                tick: 100,
            },
        );

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("ProductionInputsConsumed"));
        assert!(json.contains("iron_ore"));
        assert!(json.contains("15.0"));

        let deserialized: GameEvent = serde_json::from_str(&json).unwrap();
        match deserialized.event_type {
            EventType::ProductionInputsConsumed { inputs: deserialized_inputs, .. } => {
                assert_eq!(deserialized_inputs.get("iron_ore"), Some(&15.0));
                assert_eq!(deserialized_inputs.get("power"), Some(&20.0));
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_production_outputs_produced_event() {
        let mut outputs = HashMap::new();
        outputs.insert("iron_ingots".to_string(), 10.0);

        let event = GameEvent::new(
            5,
            102,
            EventType::ProductionOutputsProduced {
                site_id: SiteId::new(),
                building_id: BuildingId::new(),
                recipe_id: "smelt_iron".to_string(),
                outputs: outputs.clone(),
                tick: 102,
            },
        );

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("ProductionOutputsProduced"));
        assert!(json.contains("iron_ingots"));
        assert!(json.contains("10.0"));

        let deserialized: GameEvent = serde_json::from_str(&json).unwrap();
        match deserialized.event_type {
            EventType::ProductionOutputsProduced { outputs: deserialized_outputs, .. } => {
                assert_eq!(deserialized_outputs.get("iron_ingots"), Some(&10.0));
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_production_halted_event() {
        let event = GameEvent::new(
            6,
            105,
            EventType::ProductionHalted {
                site_id: SiteId::new(),
                building_id: BuildingId::new(),
                recipe_id: "smelt_iron".to_string(),
                reason: "Insufficient iron_ore (need 15.0, have 5.0)".to_string(),
                tick: 105,
            },
        );

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("ProductionHalted"));
        assert!(json.contains("Insufficient"));

        let deserialized: GameEvent = serde_json::from_str(&json).unwrap();
        match deserialized.event_type {
            EventType::ProductionHalted { reason, .. } => {
                assert!(reason.contains("iron_ore"));
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_deposit_depleted_event() {
        let event = GameEvent::new(
            7,
            200,
            EventType::DepositDepleted {
                site_id: SiteId::new(),
                body_id: CelestialBodyId::new(),
                deposit_id: "deposit_123".to_string(),
                resource_type: "iron_ore".to_string(),
                tick: 200,
            },
        );

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("DepositDepleted"));
        assert!(json.contains("iron_ore"));

        let deserialized: GameEvent = serde_json::from_str(&json).unwrap();
        match deserialized.event_type {
            EventType::DepositDepleted { resource_type, .. } => {
                assert_eq!(resource_type, "iron_ore");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_production_events_roundtrip() {
        // Test all production event types can serialize and deserialize
        let events = vec![
            EventType::RecipeStarted {
                site_id: SiteId::new(),
                building_id: BuildingId::new(),
                recipe_id: "test_recipe".to_string(),
                tick: 1,
            },
            EventType::RecipeProgressed {
                site_id: SiteId::new(),
                building_id: BuildingId::new(),
                recipe_id: "test_recipe".to_string(),
                progress_ticks: 5,
                total_ticks: 10,
                tick: 2,
            },
            EventType::RecipeCompleted {
                site_id: SiteId::new(),
                building_id: BuildingId::new(),
                recipe_id: "test_recipe".to_string(),
                tick: 3,
            },
        ];

        for event_type in events {
            let event = GameEvent::new(1, 1, event_type);
            let json = serde_json::to_string(&event).unwrap();
            let deserialized: GameEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(event.event_id, deserialized.event_id);
        }
    }
}
