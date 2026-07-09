//! Action economy, costs, and uses (R5-P5).
//!
//! The types are data on an action; the pure helper [`can_afford`] checks a cost
//! list against a resource snapshot. Per-round budget tracking and cooldown
//! ticking are stateful and live in the host; the engine supplies the types and
//! the affordability check.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// How an action fits the action economy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Economy {
    /// No action-budget cost.
    Free,
    /// The entity's main action for the round.
    #[default]
    Action,
    /// Spent from the reaction budget (default 1/round, overridable).
    Reaction,
}

/// A resource cost (PP, strain, slots, ammo, charges — all resources).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Cost {
    pub resource: String,
    pub amount: i64,
}

/// The scope over which a `uses` cap resets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UseScope {
    Round,
    Scene,
    Day,
}

/// A cap on how many times an action may be used within a scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Uses {
    pub scope: UseScope,
    pub n: u32,
}

/// An action's activation spec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Activation {
    #[serde(default)]
    pub economy: Economy,
    #[serde(default)]
    pub costs: Vec<Cost>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cooldown: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uses: Option<Uses>,
}

/// Whether the available resource pool snapshot covers every cost.
pub fn can_afford(costs: &[Cost], available: &BTreeMap<String, i64>) -> bool {
    costs
        .iter()
        .all(|c| available.get(&c.resource).copied().unwrap_or(0) >= c.amount)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affordability() {
        let mut have = BTreeMap::new();
        have.insert("spell_slots".to_string(), 2);
        have.insert("ammo".to_string(), 0);
        assert!(can_afford(
            &[Cost {
                resource: "spell_slots".into(),
                amount: 1
            }],
            &have
        ));
        // Not enough ammo (0 < 1).
        assert!(!can_afford(
            &[Cost {
                resource: "ammo".into(),
                amount: 1
            }],
            &have
        ));
        // Unknown resource counts as 0.
        assert!(!can_afford(
            &[Cost {
                resource: "strain".into(),
                amount: 1
            }],
            &have
        ));
        // Empty costs are always affordable.
        assert!(can_afford(&[], &have));
    }

    #[test]
    fn economy_defaults_to_action() {
        let a: Activation = serde_json::from_str("{}").unwrap();
        assert_eq!(a.economy, Economy::Action);
        assert!(a.costs.is_empty());
        assert!(a.cooldown.is_none());
    }
}
