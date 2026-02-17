//! Property-based tests for resource deposits.
//!
//! Uses proptest to verify invariants hold for all inputs.

use outpost_core::domain::{ExtractionDifficulty, ResourceDeposit};
use proptest::prelude::*;

// Arbitrary generator for ExtractionDifficulty
fn arb_extraction_difficulty() -> impl Strategy<Value = ExtractionDifficulty> {
    prop_oneof![
        Just(ExtractionDifficulty::VeryEasy),
        Just(ExtractionDifficulty::Easy),
        Just(ExtractionDifficulty::Normal),
        Just(ExtractionDifficulty::Hard),
        Just(ExtractionDifficulty::VeryHard),
        Just(ExtractionDifficulty::Extreme),
    ]
}

// Arbitrary generator for ResourceDeposit
fn arb_deposit() -> impl Strategy<Value = ResourceDeposit> {
    (
        "[a-z_]{3,20}",           // resource_id
        1.0f64..1000000.0,        // quantity
        arb_extraction_difficulty(), // difficulty
        0.0f64..=1.0,             // accessibility
        0.0f64..=1.0,             // concentration
    )
        .prop_map(|(id, qty, diff, acc, conc)| {
            ResourceDeposit::new_detailed(id, qty, diff, acc, conc)
        })
}

proptest! {
    #[test]
    fn extraction_never_exceeds_remaining(
        mut deposit in arb_deposit(),
        extraction_amount in 0.0f64..2000000.0
    ) {
        let initial_remaining = deposit.remaining_quantity;
        let extracted = deposit.extract(extraction_amount);

        // Extracted amount must never exceed what was remaining
        prop_assert!(extracted <= initial_remaining);
        
        // Remaining must be non-negative
        prop_assert!(deposit.remaining_quantity >= 0.0);
        
        // Remaining + extracted must equal initial
        prop_assert!((deposit.remaining_quantity + extracted - initial_remaining).abs() < 0.001);
    }

    #[test]
    fn extraction_is_idempotent_when_depleted(
        mut deposit in arb_deposit()
    ) {
        // Deplete the deposit
        let total = deposit.remaining_quantity;
        deposit.extract(total * 2.0); // Extract more than available
        
        prop_assert!(deposit.is_depleted());
        prop_assert_eq!(deposit.remaining_quantity, 0.0);
        
        // Further extractions should return 0
        let further = deposit.extract(100.0);
        prop_assert_eq!(further, 0.0);
        prop_assert_eq!(deposit.remaining_quantity, 0.0);
    }

    #[test]
    fn depletion_rate_is_always_in_bounds(
        mut deposit in arb_deposit(),
        extraction_pct in 0.0f64..=1.5 // Can try to extract more than 100%
    ) {
        let to_extract = deposit.initial_quantity * extraction_pct;
        deposit.extract(to_extract);
        
        let depletion = deposit.depletion_rate();
        prop_assert!(depletion >= 0.0);
        prop_assert!(depletion <= 1.0);
    }

    #[test]
    fn remaining_percentage_is_inverse_of_depletion(
        mut deposit in arb_deposit(),
        extraction_amount in 0.0f64..1000000.0
    ) {
        deposit.extract(extraction_amount);
        
        let depletion = deposit.depletion_rate();
        let remaining_pct = deposit.remaining_percentage();
        
        // Should sum to 1.0 (within floating point tolerance)
        prop_assert!((depletion + remaining_pct - 1.0).abs() < 0.001);
    }

    #[test]
    fn multiple_extractions_are_consistent(
        mut deposit in arb_deposit(),
        amounts in prop::collection::vec(0.0f64..10000.0, 1..20)
    ) {
        let initial = deposit.remaining_quantity;
        let mut total_extracted = 0.0;
        
        for amount in amounts {
            let extracted = deposit.extract(amount);
            total_extracted += extracted;
        }
        
        // Total extracted cannot exceed initial (with floating point tolerance)
        prop_assert!(total_extracted <= initial + 0.001);
        
        // Remaining + extracted = initial (with tolerance for floating point errors)
        let difference = (deposit.remaining_quantity + total_extracted - initial).abs();
        prop_assert!(difference < 0.01, 
            "Difference {} exceeds tolerance. Remaining: {}, Extracted: {}, Initial: {}",
            difference, deposit.remaining_quantity, total_extracted, initial);
    }

    #[test]
    fn effective_extraction_rate_is_positive(
        deposit in arb_deposit(),
        base_rate in 0.0f64..1000.0
    ) {
        let effective = deposit.effective_extraction_rate(base_rate);
        prop_assert!(effective >= 0.0);
        
        // For positive base rate, effective should also be positive
        if base_rate > 0.0 {
            prop_assert!(effective > 0.0);
        }
    }

    #[test]
    fn validation_accepts_valid_deposits(
        deposit in arb_deposit()
    ) {
        // All generated deposits should be valid
        prop_assert!(deposit.validate().is_ok());
    }

    #[test]
    fn extraction_preserves_initial_quantity(
        mut deposit in arb_deposit(),
        extraction_amount in 0.0f64..1000000.0
    ) {
        let initial = deposit.initial_quantity;
        deposit.extract(extraction_amount);
        
        // Initial quantity should never change
        prop_assert_eq!(deposit.initial_quantity, initial);
    }

    #[test]
    fn zero_or_negative_extraction_does_nothing(
        mut deposit in arb_deposit(),
        negative_amount in -1000000.0f64..0.0
    ) {
        let before = deposit.remaining_quantity;
        let extracted = deposit.extract(negative_amount);
        
        prop_assert_eq!(extracted, 0.0);
        prop_assert_eq!(deposit.remaining_quantity, before);
    }

    #[test]
    fn accessibility_always_in_bounds(
        deposit in arb_deposit()
    ) {
        prop_assert!(deposit.accessibility >= 0.0);
        prop_assert!(deposit.accessibility <= 1.0);
    }

    #[test]
    fn concentration_always_in_bounds(
        deposit in arb_deposit()
    ) {
        prop_assert!(deposit.concentration >= 0.0);
        prop_assert!(deposit.concentration <= 1.0);
    }
}
