//! Resource content schema models.
//!
//! Ported from `src/harsh_realm/resources/schema.py`.
//!
//! These structs describe resource *type* definitions (HP, gold, etc.) loaded
//! from pack YAML content records.  Resource *instances* (per-entity rows) live
//! in `repositories::resources`.

use serde::{Deserialize, Serialize};

use crate::ir::ModifierCondition;

/// The starting current value for a new resource instance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResourceDefaultCurrent {
    /// Start at maximum capacity (the string sentinel `"max"`).
    Max(String),
    /// Start at a fixed integer amount.
    Amount(i32),
}

impl Default for ResourceDefaultCurrent {
    fn default() -> Self {
        Self::Max("max".to_string())
    }
}

impl ResourceDefaultCurrent {
    /// True iff this is the `"max"` sentinel.
    pub fn is_max(&self) -> bool {
        matches!(self, Self::Max(_))
    }
}

/// Passive regeneration rule for a resource.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceRegeneration {
    /// Amount to add per interval.
    pub rate: i32,
    /// Ticks between applications (must be ≥ 1).
    #[serde(default = "ResourceRegeneration::default_interval")]
    pub interval_ticks: i32,
    /// Condition that must hold for regen to apply.
    #[serde(default)]
    pub condition_predicate: ModifierCondition,
}

impl ResourceRegeneration {
    fn default_interval() -> i32 {
        1
    }
}

/// A resource type definition such as HP, gold, Bennies, or Effort.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resource {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Default maximum for new instances; `None` means unbounded.
    pub default_max: Option<i32>,
    /// Default starting current (`"max"` or a fixed integer).
    #[serde(default)]
    pub default_current: ResourceDefaultCurrent,
    /// Whether the current value is allowed to go below zero.
    #[serde(default)]
    pub can_go_negative: bool,
    pub regeneration: Option<ResourceRegeneration>,
    /// Event type emitted when current reaches 0.
    pub zero_event: Option<String>,
    /// Event type emitted when current reaches max.
    pub full_event: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Resource {
    /// Compute the initial `current` for a new instance.
    pub fn initial_current(&self, max_value: Option<i32>, current_override: Option<i32>) -> i32 {
        if let Some(c) = current_override {
            return c;
        }
        match &self.default_current {
            ResourceDefaultCurrent::Max(_) => max_value.unwrap_or(0),
            ResourceDefaultCurrent::Amount(v) => *v,
        }
    }

    /// Clamp `current` to the configured bounds.
    pub fn clamp_current(&self, current: i32, max: Option<i32>) -> i32 {
        let lower = if self.can_go_negative { current } else { current.max(0) };
        match max {
            Some(m) => lower.min(m),
            None => lower,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hp_resource() -> Resource {
        Resource {
            id: "xwn-core:resources.hp".into(),
            name: "HP".into(),
            description: String::new(),
            default_max: Some(10),
            default_current: ResourceDefaultCurrent::Max("max".into()),
            can_go_negative: false,
            regeneration: None,
            zero_event: Some("entity.died".into()),
            full_event: None,
            tags: vec!["health".into()],
        }
    }

    #[test]
    fn initial_current_max_sentinel() {
        let r = hp_resource();
        assert_eq!(r.initial_current(Some(10), None), 10);
    }

    #[test]
    fn initial_current_override_wins() {
        let r = hp_resource();
        assert_eq!(r.initial_current(Some(10), Some(3)), 3);
    }

    #[test]
    fn initial_current_fixed_amount() {
        let mut r = hp_resource();
        r.default_current = ResourceDefaultCurrent::Amount(5);
        assert_eq!(r.initial_current(Some(10), None), 5);
    }

    #[test]
    fn clamp_no_negative() {
        let r = hp_resource();
        assert_eq!(r.clamp_current(-5, Some(10)), 0);
        assert_eq!(r.clamp_current(15, Some(10)), 10);
        assert_eq!(r.clamp_current(7, Some(10)), 7);
    }

    #[test]
    fn clamp_can_go_negative() {
        let mut r = hp_resource();
        r.can_go_negative = true;
        assert_eq!(r.clamp_current(-5, Some(10)), -5);
        assert_eq!(r.clamp_current(15, Some(10)), 10);
    }

    #[test]
    fn clamp_unbounded_max() {
        let r = hp_resource();
        assert_eq!(r.clamp_current(9999, None), 9999);
    }

    #[test]
    fn regen_default_interval() {
        let regen = ResourceRegeneration {
            rate: 1,
            interval_ticks: ResourceRegeneration::default_interval(),
            condition_predicate: ModifierCondition::default(),
        };
        assert_eq!(regen.interval_ticks, 1);
    }
}
