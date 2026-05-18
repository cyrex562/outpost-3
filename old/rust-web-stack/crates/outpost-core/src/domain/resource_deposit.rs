//! ResourceDeposit — extractable resource deposits on celestial bodies.
//!
//! Represents a finite deposit of a specific resource that can be mined/extracted
//! over time. Deposits have total quantity, extraction difficulty, and deplete
//! as resources are removed.

use serde::{Deserialize, Serialize};

/// Difficulty of extracting resources from this deposit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ExtractionDifficulty {
    /// Easy to extract, surface deposits, high concentration.
    VeryEasy,
    /// Straightforward extraction with basic equipment.
    Easy,
    /// Standard difficulty, typical mining operations.
    Normal,
    /// Challenging, requires advanced equipment or techniques.
    Hard,
    /// Very difficult, deep deposits or low concentration.
    VeryHard,
    /// Extremely difficult, exotic conditions or trace amounts.
    Extreme,
}

impl ExtractionDifficulty {
    /// Returns extraction efficiency multiplier (1.0 = normal, lower = harder).
    /// This affects how much of the resource is successfully extracted vs. lost.
    pub fn efficiency_multiplier(&self) -> f64 {
        match self {
            ExtractionDifficulty::VeryEasy => 1.3,
            ExtractionDifficulty::Easy => 1.15,
            ExtractionDifficulty::Normal => 1.0,
            ExtractionDifficulty::Hard => 0.85,
            ExtractionDifficulty::VeryHard => 0.7,
            ExtractionDifficulty::Extreme => 0.5,
        }
    }

    /// Returns time multiplier for extraction (1.0 = normal, higher = slower).
    pub fn time_multiplier(&self) -> f64 {
        match self {
            ExtractionDifficulty::VeryEasy => 0.7,
            ExtractionDifficulty::Easy => 0.85,
            ExtractionDifficulty::Normal => 1.0,
            ExtractionDifficulty::Hard => 1.25,
            ExtractionDifficulty::VeryHard => 1.5,
            ExtractionDifficulty::Extreme => 2.0,
        }
    }
}

/// A deposit of extractable resources on a celestial body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceDeposit {
    /// Type of resource (references resource definition ID).
    pub resource_id: String,
    
    /// Total initial quantity in metric tons.
    pub initial_quantity: f64,
    
    /// Current remaining quantity in metric tons.
    pub remaining_quantity: f64,
    
    /// Difficulty of extracting from this deposit.
    pub difficulty: ExtractionDifficulty,
    
    /// Accessibility rating (0.0 = inaccessible, 1.0 = surface/easy access).
    /// Affects whether deposit can be exploited without special tech.
    pub accessibility: f64,
    
    /// Concentration/purity (0.0-1.0, higher = more resource per unit extracted).
    pub concentration: f64,
}

impl ResourceDeposit {
    /// Create a new resource deposit.
    pub fn new(
        resource_id: String,
        quantity: f64,
        difficulty: ExtractionDifficulty,
    ) -> Self {
        Self {
            resource_id,
            initial_quantity: quantity,
            remaining_quantity: quantity,
            difficulty,
            accessibility: 1.0,
            concentration: 0.5,
        }
    }

    /// Create deposit with all parameters.
    pub fn new_detailed(
        resource_id: String,
        quantity: f64,
        difficulty: ExtractionDifficulty,
        accessibility: f64,
        concentration: f64,
    ) -> Self {
        Self {
            resource_id,
            initial_quantity: quantity,
            remaining_quantity: quantity,
            difficulty,
            accessibility: accessibility.clamp(0.0, 1.0),
            concentration: concentration.clamp(0.0, 1.0),
        }
    }

    /// Check if deposit is depleted.
    pub fn is_depleted(&self) -> bool {
        self.remaining_quantity <= 0.0
    }

    /// Calculate depletion percentage (0.0 = full, 1.0 = empty).
    pub fn depletion_rate(&self) -> f64 {
        if self.initial_quantity <= 0.0 {
            return 1.0;
        }
        1.0 - (self.remaining_quantity / self.initial_quantity)
    }

    /// Calculate remaining percentage (0.0 = empty, 1.0 = full).
    pub fn remaining_percentage(&self) -> f64 {
        if self.initial_quantity <= 0.0 {
            return 0.0;
        }
        (self.remaining_quantity / self.initial_quantity).clamp(0.0, 1.0)
    }

    /// Extract resources from this deposit.
    /// 
    /// Returns the actual amount extracted (may be less than requested).
    /// Updates remaining_quantity.
    pub fn extract(&mut self, amount: f64) -> f64 {
        if amount <= 0.0 {
            return 0.0;
        }

        let actual_extracted = amount.min(self.remaining_quantity);
        self.remaining_quantity -= actual_extracted;
        
        // Ensure we don't go negative due to floating point errors
        if self.remaining_quantity < 0.001 {
            self.remaining_quantity = 0.0;
        }

        actual_extracted
    }

    /// Calculate effective extraction rate considering difficulty and concentration.
    /// 
    /// Base rate is multiplied by efficiency and concentration.
    pub fn effective_extraction_rate(&self, base_rate: f64) -> f64 {
        base_rate * self.difficulty.efficiency_multiplier() * self.concentration
    }

    /// Validate deposit parameters.
    pub fn validate(&self) -> Result<(), String> {
        if self.resource_id.is_empty() {
            return Err("Resource ID cannot be empty".to_string());
        }

        if self.initial_quantity < 0.0 {
            return Err("Initial quantity cannot be negative".to_string());
        }

        if self.remaining_quantity < 0.0 {
            return Err("Remaining quantity cannot be negative".to_string());
        }

        if self.remaining_quantity > self.initial_quantity {
            return Err("Remaining quantity cannot exceed initial quantity".to_string());
        }

        if !(0.0..=1.0).contains(&self.accessibility) {
            return Err("Accessibility must be between 0.0 and 1.0".to_string());
        }

        if !(0.0..=1.0).contains(&self.concentration) {
            return Err("Concentration must be between 0.0 and 1.0".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deposit_creation() {
        let deposit = ResourceDeposit::new(
            "iron_ore".to_string(),
            1000.0,
            ExtractionDifficulty::Normal,
        );

        assert_eq!(deposit.resource_id, "iron_ore");
        assert_eq!(deposit.initial_quantity, 1000.0);
        assert_eq!(deposit.remaining_quantity, 1000.0);
        assert!(!deposit.is_depleted());
        assert_eq!(deposit.depletion_rate(), 0.0);
        assert_eq!(deposit.remaining_percentage(), 1.0);
    }

    #[test]
    fn test_deposit_extraction() {
        let mut deposit = ResourceDeposit::new(
            "copper_ore".to_string(),
            500.0,
            ExtractionDifficulty::Easy,
        );

        // Extract 100 tons
        let extracted = deposit.extract(100.0);
        assert_eq!(extracted, 100.0);
        assert_eq!(deposit.remaining_quantity, 400.0);
        assert_eq!(deposit.remaining_percentage(), 0.8);

        // Extract more than remaining
        let extracted = deposit.extract(500.0);
        assert_eq!(extracted, 400.0); // Only what's left
        assert_eq!(deposit.remaining_quantity, 0.0);
        assert!(deposit.is_depleted());
    }

    #[test]
    fn test_extraction_never_exceeds_remaining() {
        let mut deposit = ResourceDeposit::new(
            "uranium_ore".to_string(),
            50.0,
            ExtractionDifficulty::Hard,
        );

        // Try to extract more than available
        let extracted = deposit.extract(100.0);
        assert!(extracted <= 50.0);
        assert_eq!(extracted, 50.0);
        assert!(deposit.is_depleted());
    }

    #[test]
    fn test_extraction_difficulty_multipliers() {
        assert!(ExtractionDifficulty::VeryEasy.efficiency_multiplier() > 1.0);
        assert!(ExtractionDifficulty::Extreme.efficiency_multiplier() < 1.0);
        
        assert!(ExtractionDifficulty::VeryEasy.time_multiplier() < 1.0);
        assert!(ExtractionDifficulty::Extreme.time_multiplier() > 1.0);
    }

    #[test]
    fn test_effective_extraction_rate() {
        let deposit = ResourceDeposit::new_detailed(
            "iron_ore".to_string(),
            1000.0,
            ExtractionDifficulty::Normal,
            1.0,
            0.8,
        );

        let base_rate = 10.0;
        let effective_rate = deposit.effective_extraction_rate(base_rate);
        
        // Normal difficulty = 1.0x, concentration = 0.8x
        assert!((effective_rate - 8.0).abs() < 0.001);
    }

    #[test]
    fn test_deposit_validation() {
        let valid = ResourceDeposit::new(
            "ice".to_string(),
            100.0,
            ExtractionDifficulty::VeryEasy,
        );
        assert!(valid.validate().is_ok());

        let mut invalid = valid.clone();
        invalid.resource_id = String::new();
        assert!(invalid.validate().is_err());

        let mut invalid = valid.clone();
        invalid.remaining_quantity = -10.0;
        assert!(invalid.validate().is_err());

        let mut invalid = valid.clone();
        invalid.remaining_quantity = 200.0; // More than initial
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_depletion_tracking() {
        let mut deposit = ResourceDeposit::new(
            "silicon_ore".to_string(),
            1000.0,
            ExtractionDifficulty::Normal,
        );

        assert_eq!(deposit.depletion_rate(), 0.0);

        deposit.extract(250.0);
        assert!((deposit.depletion_rate() - 0.25).abs() < 0.001);

        deposit.extract(500.0);
        assert!((deposit.depletion_rate() - 0.75).abs() < 0.001);

        deposit.extract(250.0);
        assert!((deposit.depletion_rate() - 1.0).abs() < 0.001);
        assert!(deposit.is_depleted());
    }

    #[test]
    fn test_zero_extraction() {
        let mut deposit = ResourceDeposit::new(
            "regolith".to_string(),
            100.0,
            ExtractionDifficulty::Normal,
        );

        let extracted = deposit.extract(0.0);
        assert_eq!(extracted, 0.0);
        assert_eq!(deposit.remaining_quantity, 100.0);

        let extracted = deposit.extract(-10.0); // Negative should do nothing
        assert_eq!(extracted, 0.0);
        assert_eq!(deposit.remaining_quantity, 100.0);
    }
}
