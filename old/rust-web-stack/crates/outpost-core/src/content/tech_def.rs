/// Technology tree definition for research and progression
///
/// Placeholder implementation for Alpha phase. Will be expanded with:
/// - Research point accumulation system
/// - Multiple research categories (engineering, science, social)
/// - Tech node dependencies and branching paths
/// - Dynamic unlock conditions beyond prerequisites

use serde::{Deserialize, Serialize};

/// A technology node in the research tree
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TechDefinition {
    /// Unique identifier (kebab-case)
    pub id: String,
    
    /// Display name
    pub name: String,
    
    /// Detailed description of what this technology enables
    pub description: String,
    
    /// Research category for UI organization
    pub category: TechCategory,
    
    /// Research tier (1 = basic, higher = advanced)
    pub tier: u8,
    
    /// Research cost in research points
    pub research_cost: f64,
    
    /// Research time in game ticks
    pub research_time_ticks: u64,
    
    /// IDs of technologies that must be researched first
    #[serde(default)]
    pub prerequisites: Vec<String>,
    
    /// What this technology unlocks
    #[serde(default)]
    pub unlocks: TechUnlocks,
    
    /// Optional resource costs for research (in addition to research points)
    #[serde(default)]
    pub resource_costs: Vec<ResearchCost>,
}

/// Research category for organization and UI filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TechCategory {
    /// Engineering and construction technologies
    Engineering,
    
    /// Physics, chemistry, materials science
    PhysicalSciences,
    
    /// Biology, medicine, agriculture
    LifeSciences,
    
    /// Computing, automation, AI
    Computing,
    
    /// Social systems, governance, culture
    Social,
    
    /// Space exploration and propulsion
    Exploration,
    
    /// Military and defense (future)
    Military,
}

/// What a technology unlocks when researched
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TechUnlocks {
    /// Building type IDs that become available
    #[serde(default)]
    pub buildings: Vec<String>,
    
    /// Resource type IDs that become available (for advanced materials)
    #[serde(default)]
    pub resources: Vec<String>,
    
    /// Recipe IDs that become available (advanced production methods)
    #[serde(default)]
    pub recipes: Vec<String>,
    
    /// Passive bonuses (e.g., "mining_efficiency_20_percent")
    #[serde(default)]
    pub bonuses: Vec<String>,
    
    /// Event IDs that become possible (new narrative branches)
    #[serde(default)]
    pub events: Vec<String>,
}

/// Resource cost for research (some techs require material investment)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResearchCost {
    /// Resource ID
    pub resource_id: String,
    
    /// Amount consumed during research
    pub amount: f64,
}

impl TechDefinition {
    /// Validate that this tech definition is complete and consistent
    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty() {
            return Err("Tech ID cannot be empty".to_string());
        }
        
        if self.name.is_empty() {
            return Err(format!("Tech {} has empty name", self.id));
        }
        
        if self.research_cost <= 0.0 {
            return Err(format!("Tech {} has invalid research cost", self.id));
        }
        
        if self.research_time_ticks == 0 {
            return Err(format!("Tech {} has zero research time", self.id));
        }
        
        if self.tier == 0 {
            return Err(format!("Tech {} has invalid tier (must be >= 1)", self.id));
        }
        
        // Check for self-referential prerequisites
        if self.prerequisites.contains(&self.id) {
            return Err(format!("Tech {} lists itself as a prerequisite", self.id));
        }
        
        // Validate resource costs
        for cost in &self.resource_costs {
            if cost.resource_id.is_empty() {
                return Err(format!("Tech {} has resource cost with empty ID", self.id));
            }
            if cost.amount <= 0.0 {
                return Err(format!(
                    "Tech {} has invalid resource cost for {}",
                    self.id, cost.resource_id
                ));
            }
        }
        
        Ok(())
    }
    
    /// Check if this tech has any prerequisites
    pub fn has_prerequisites(&self) -> bool {
        !self.prerequisites.is_empty()
    }
    
    /// Check if this tech unlocks anything
    pub fn has_unlocks(&self) -> bool {
        !self.unlocks.buildings.is_empty()
            || !self.unlocks.resources.is_empty()
            || !self.unlocks.recipes.is_empty()
            || !self.unlocks.bonuses.is_empty()
            || !self.unlocks.events.is_empty()
    }
    
    /// Get the total number of unlocks
    pub fn unlock_count(&self) -> usize {
        self.unlocks.buildings.len()
            + self.unlocks.resources.len()
            + self.unlocks.recipes.len()
            + self.unlocks.bonuses.len()
            + self.unlocks.events.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tech_validation() {
        let valid_tech = TechDefinition {
            id: "advanced_mining".to_string(),
            name: "Advanced Mining Techniques".to_string(),
            description: "Improved extraction methods".to_string(),
            category: TechCategory::Engineering,
            tier: 2,
            research_cost: 1000.0,
            research_time_ticks: 500,
            prerequisites: vec!["basic_mining".to_string()],
            unlocks: TechUnlocks {
                buildings: vec!["advanced_mine".to_string()],
                bonuses: vec!["mining_efficiency_20_percent".to_string()],
                ..Default::default()
            },
            resource_costs: vec![],
        };

        assert!(valid_tech.validate().is_ok());
        assert!(valid_tech.has_prerequisites());
        assert!(valid_tech.has_unlocks());
        assert_eq!(valid_tech.unlock_count(), 2);
    }

    #[test]
    fn test_tech_validation_empty_id() {
        let tech = TechDefinition {
            id: "".to_string(),
            name: "Test".to_string(),
            description: "Test".to_string(),
            category: TechCategory::Engineering,
            tier: 1,
            research_cost: 100.0,
            research_time_ticks: 100,
            prerequisites: vec![],
            unlocks: TechUnlocks::default(),
            resource_costs: vec![],
        };

        assert!(tech.validate().is_err());
    }

    #[test]
    fn test_tech_validation_invalid_cost() {
        let tech = TechDefinition {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test".to_string(),
            category: TechCategory::Engineering,
            tier: 1,
            research_cost: 0.0,
            research_time_ticks: 100,
            prerequisites: vec![],
            unlocks: TechUnlocks::default(),
            resource_costs: vec![],
        };

        assert!(tech.validate().is_err());
    }

    #[test]
    fn test_tech_validation_self_prerequisite() {
        let tech = TechDefinition {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test".to_string(),
            category: TechCategory::Engineering,
            tier: 1,
            research_cost: 100.0,
            research_time_ticks: 100,
            prerequisites: vec!["test".to_string()],
            unlocks: TechUnlocks::default(),
            resource_costs: vec![],
        };

        assert!(tech.validate().is_err());
        assert!(tech
            .validate()
            .unwrap_err()
            .contains("lists itself as a prerequisite"));
    }

    #[test]
    fn test_tech_with_resource_costs() {
        let tech = TechDefinition {
            id: "prototype_reactor".to_string(),
            name: "Prototype Reactor".to_string(),
            description: "Experimental fusion reactor design".to_string(),
            category: TechCategory::PhysicalSciences,
            tier: 3,
            research_cost: 5000.0,
            research_time_ticks: 1000,
            prerequisites: vec!["fusion_basics".to_string()],
            unlocks: TechUnlocks {
                buildings: vec!["fusion_reactor".to_string()],
                ..Default::default()
            },
            resource_costs: vec![
                ResearchCost {
                    resource_id: "uranium_fuel_rod".to_string(),
                    amount: 10.0,
                },
                ResearchCost {
                    resource_id: "electronics".to_string(),
                    amount: 50.0,
                },
            ],
        };

        assert!(tech.validate().is_ok());
        assert_eq!(tech.resource_costs.len(), 2);
    }

    #[test]
    fn test_tech_serialization() {
        let tech = TechDefinition {
            id: "basic_automation".to_string(),
            name: "Basic Automation".to_string(),
            description: "Simple automated systems".to_string(),
            category: TechCategory::Computing,
            tier: 1,
            research_cost: 500.0,
            research_time_ticks: 200,
            prerequisites: vec![],
            unlocks: TechUnlocks {
                bonuses: vec!["production_efficiency_10_percent".to_string()],
                ..Default::default()
            },
            resource_costs: vec![],
        };

        let yaml = serde_yaml::to_string(&tech).unwrap();
        let deserialized: TechDefinition = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(tech, deserialized);
    }
}
