use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Trigger condition for a gameplay event
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventTrigger {
    /// Trigger when a specific building is constructed
    BuildingConstructed { building_id: String },
    
    /// Trigger when population reaches a threshold
    PopulationReached { threshold: u32 },
    
    /// Trigger when a resource stockpile reaches a level
    ResourceThreshold { resource_id: String, amount: f64, above: bool },
    
    /// Trigger randomly with a probability per tick
    Random { probability_per_tick: f64 },
    
    /// Trigger after a certain number of ticks
    TimeBased { ticks: u64 },
    
    /// Trigger when morale falls below a threshold
    MoraleThreshold { threshold: f64 },
}

/// Outcome of choosing an event choice
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventOutcome {
    /// Modify resources
    ResourceChange { resource_id: String, amount: f64 },
    
    /// Modify morale
    MoraleChange { amount: f64 },
    
    /// Modify population
    PopulationChange { amount: i32 },
    
    /// Unlock a building type
    UnlockBuilding { building_id: String },
    
    /// Trigger another event
    TriggerEvent { event_id: String },
    
    /// Custom narrative text
    NarrativeText { text: String },
}

/// A choice the player can make in response to an event
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventChoice {
    pub id: String,
    pub text: String,
    pub description: String,
    pub outcomes: Vec<EventOutcome>,
    pub cost: HashMap<String, f64>, // Resource costs to make this choice
}

/// Complete definition of a gameplay event loaded from YAML
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventDefinition {
    /// Unique identifier
    pub id: String,
    
    /// Display title
    pub title: String,
    
    /// Event description/narrative text
    pub description: String,
    
    /// Trigger conditions
    pub triggers: Vec<EventTrigger>,
    
    /// Choices the player can make
    pub choices: Vec<EventChoice>,
    
    /// Can this event repeat
    pub repeatable: bool,
    
    /// Cooldown in ticks before event can trigger again
    pub cooldown_ticks: Option<u64>,
    
    /// Weight for random selection (higher = more likely)
    pub weight: f64,
}

impl EventDefinition {
    /// Validate that this event definition is complete and consistent
    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty() {
            return Err("Event ID cannot be empty".to_string());
        }
        
        if self.title.is_empty() {
            return Err(format!("Event {} has empty title", self.id));
        }
        
        if self.triggers.is_empty() {
            return Err(format!("Event {} has no triggers", self.id));
        }
        
        if self.choices.is_empty() {
            return Err(format!("Event {} has no choices", self.id));
        }
        
        for choice in &self.choices {
            if choice.id.is_empty() {
                return Err(format!("Event {} has choice with empty ID", self.id));
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_definition_validation() {
        let valid_event = EventDefinition {
            id: "first_colony".to_string(),
            title: "A New Beginning".to_string(),
            description: "The first colonists arrive at the landing site.".to_string(),
            triggers: vec![
                EventTrigger::PopulationReached { threshold: 1 }
            ],
            choices: vec![
                EventChoice {
                    id: "celebrate".to_string(),
                    text: "Celebrate the arrival".to_string(),
                    description: "Boost morale with a celebration".to_string(),
                    outcomes: vec![
                        EventOutcome::MoraleChange { amount: 10.0 }
                    ],
                    cost: HashMap::new(),
                }
            ],
            repeatable: false,
            cooldown_ticks: None,
            weight: 1.0,
        };

        assert!(valid_event.validate().is_ok());
    }
}
