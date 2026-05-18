//! Life support status tracking for a site.
//!
//! Records per-tick satisfaction of oxygen, water, and food
//! demands for the site population.

use serde::{Deserialize, Serialize};

/// Current life support satisfaction at a site (recalculated each tick).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LifeSupportStatus {
    /// Oxygen satisfaction ratio: 0.0 (none) → 1.0 (fully met).
    pub oxygen_sat: f32,
    /// Water satisfaction ratio: 0.0 → 1.0.
    pub water_sat: f32,
    /// Food satisfaction ratio: 0.0 → 1.0.
    pub food_sat: f32,
    /// True when any satisfaction factor is below the critical threshold (0.25).
    pub is_critical: bool,
    /// Oxygen consumed from stockpile this tick (units).
    pub oxygen_consumed: f64,
    /// Water consumed from stockpile this tick (units).
    pub water_consumed: f64,
    /// Food consumed from stockpile this tick (units).
    pub food_consumed: f64,
}

impl LifeSupportStatus {
    /// Fully satisfied status (default when no population).
    pub fn satisfied() -> Self {
        Self {
            oxygen_sat: 1.0,
            water_sat: 1.0,
            food_sat: 1.0,
            is_critical: false,
            oxygen_consumed: 0.0,
            water_consumed: 0.0,
            food_consumed: 0.0,
        }
    }

    /// Compute the combined life support score (weighted, 0.0–1.0).
    /// Oxygen is most critical (50%), then water (30%), then food (20%).
    pub fn overall_score(&self) -> f32 {
        (self.oxygen_sat * 0.5) + (self.water_sat * 0.3) + (self.food_sat * 0.2)
    }

    /// Status label for UI display.
    pub fn status_label(&self) -> &'static str {
        if self.is_critical {
            "Critical"
        } else if self.overall_score() < 0.75 {
            "Warning"
        } else {
            "Good"
        }
    }
}

impl Default for LifeSupportStatus {
    fn default() -> Self {
        Self::satisfied()
    }
}

/// Per-tick demand constants for life support.
pub struct LifeSupportDemand;

impl LifeSupportDemand {
    /// Oxygen units required per colonist per tick.
    pub const OXYGEN_PER_COLONIST: f64 = 0.3;
    /// Water units required per colonist per tick.
    pub const WATER_PER_COLONIST: f64 = 0.2;
    /// Food units required per colonist per tick.
    pub const FOOD_PER_COLONIST: f64 = 0.15;
    /// Below this satisfaction ratio the system is "critical".
    pub const CRITICAL_THRESHOLD: f32 = 0.25;

    /// Compute oxygen demand for a population count.
    pub fn oxygen(population: u64) -> f64 {
        population as f64 * Self::OXYGEN_PER_COLONIST
    }

    /// Compute water demand for a population count.
    pub fn water(population: u64) -> f64 {
        population as f64 * Self::WATER_PER_COLONIST
    }

    /// Compute food demand for a population count.
    pub fn food(population: u64) -> f64 {
        population as f64 * Self::FOOD_PER_COLONIST
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_satisfied_default() {
        let ls = LifeSupportStatus::satisfied();
        assert_eq!(ls.oxygen_sat, 1.0);
        assert!(!ls.is_critical);
        assert!((ls.overall_score() - 1.0).abs() < 0.001);
        assert_eq!(ls.status_label(), "Good");
    }

    #[test]
    fn test_critical_when_oxygen_zero() {
        let ls = LifeSupportStatus {
            oxygen_sat: 0.0,
            water_sat: 1.0,
            food_sat: 1.0,
            is_critical: true,
            oxygen_consumed: 0.0,
            water_consumed: 0.0,
            food_consumed: 0.0,
        };
        assert!(ls.is_critical);
        assert_eq!(ls.status_label(), "Critical");
    }

    #[test]
    fn test_overall_score_weighted() {
        let ls = LifeSupportStatus {
            oxygen_sat: 0.5,
            water_sat: 1.0,
            food_sat: 1.0,
            is_critical: false,
            oxygen_consumed: 0.0,
            water_consumed: 0.0,
            food_consumed: 0.0,
        };
        // 0.5*0.5 + 1.0*0.3 + 1.0*0.2 = 0.25 + 0.3 + 0.2 = 0.75
        assert!((ls.overall_score() - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_demand_calculations() {
        assert!((LifeSupportDemand::oxygen(100) - 30.0).abs() < 0.001);
        assert!((LifeSupportDemand::water(100)  - 20.0).abs() < 0.001);
        assert!((LifeSupportDemand::food(100)   - 15.0).abs() < 0.001);
    }
}
