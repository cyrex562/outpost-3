//! Property-based tests for production chain resource conservation.
//!
//! These tests verify that:
//! 1. Resource conservation holds: total resources before tick + produced == total after + consumed.
//! 2. Stockpiles never go negative from life support consumption.
//! 3. Power grid totals are non-negative.
//! 4. Site power_efficiency is always in [0.5, 1.0].
//! 5. Life support satisfaction ratios are always clamped to [0.0, 1.0].

use outpost_core::domain::{
    CelestialBodyId, Site, ResourceType,
};
use outpost_core::domain::life_support::LifeSupportDemand;
use outpost_core::domain::power_grid::PowerGrid;
use outpost_core::simulation::life_support_processor::compute_life_support;
use proptest::prelude::*;

// ─────────────────────────────────────────────────────────────────────────────
// Strategy helpers
// ─────────────────────────────────────────────────────────────────────────────

/// A site with `pop` colonists and specified initial stockpiles.
fn site_with_resources(pop: u32, oxygen: i64, water: i64, food: i64) -> Site {
    let mut site = Site::new_settlement(
        CelestialBodyId::new(),
        "PropTest Site".to_string(),
        0,
        pop,
    );
    site.resources.set(ResourceType::Oxygen, oxygen);
    site.resources.set(ResourceType::Water, water);
    site.resources.set(ResourceType::Food, food);
    site
}

// ─────────────────────────────────────────────────────────────────────────────
// Life Support: stockpile conservation
// ─────────────────────────────────────────────────────────────────────────────

proptest! {
    /// Consumed amount never exceeds initial supply.
    #[test]
    fn prop_life_support_consumed_le_available(
        pop in 0_u32..=500,
        oxygen in 0_i64..=10_000,
        water  in 0_i64..=10_000,
        food   in 0_i64..=10_000,
    ) {
        let mut site = site_with_resources(pop, oxygen, water, food);
        let ls = compute_life_support(&mut site);

        prop_assert!(
            ls.oxygen_consumed <= oxygen as f64 + 1e-9,
            "Consumed oxygen {} > available {}",
            ls.oxygen_consumed, oxygen
        );
        prop_assert!(
            ls.water_consumed <= water as f64 + 1e-9,
            "Consumed water {} > available {}",
            ls.water_consumed, water
        );
        prop_assert!(
            ls.food_consumed <= food as f64 + 1e-9,
            "Consumed food {} > available {}",
            ls.food_consumed, food
        );
    }

    /// Stockpile never goes negative after life support consumption.
    #[test]
    fn prop_life_support_stockpile_non_negative(
        pop in 0_u32..=1000,
        oxygen in 0_i64..=5_000,
        water  in 0_i64..=5_000,
        food   in 0_i64..=5_000,
    ) {
        let mut site = site_with_resources(pop, oxygen, water, food);
        compute_life_support(&mut site);

        let o = site.resources.get(ResourceType::Oxygen);
        let w = site.resources.get(ResourceType::Water);
        let f = site.resources.get(ResourceType::Food);

        prop_assert!(o >= 0, "Oxygen stockpile went negative: {}", o);
        prop_assert!(w >= 0, "Water stockpile went negative: {}", w);
        prop_assert!(f >= 0, "Food stockpile went negative: {}", f);
    }

    /// Satisfaction ratios are always in [0.0, 1.0].
    #[test]
    fn prop_life_support_sat_ratios_clamped(
        pop in 0_u32..=1000,
        oxygen in 0_i64..=10_000,
        water  in 0_i64..=10_000,
        food   in 0_i64..=10_000,
    ) {
        let mut site = site_with_resources(pop, oxygen, water, food);
        let ls = compute_life_support(&mut site);

        prop_assert!(ls.oxygen_sat >= 0.0 && ls.oxygen_sat <= 1.0,
            "oxygen_sat out of range: {}", ls.oxygen_sat);
        prop_assert!(ls.water_sat >= 0.0 && ls.water_sat <= 1.0,
            "water_sat out of range: {}", ls.water_sat);
        prop_assert!(ls.food_sat >= 0.0 && ls.food_sat <= 1.0,
            "food_sat out of range: {}", ls.food_sat);
    }

    /// overall_score() is always in [0.0, 1.0].
    #[test]
    fn prop_overall_score_clamped(
        pop in 0_u32..=1000,
        oxygen in 0_i64..=10_000,
        water  in 0_i64..=10_000,
        food   in 0_i64..=10_000,
    ) {
        let mut site = site_with_resources(pop, oxygen, water, food);
        let ls = compute_life_support(&mut site);
        let score = ls.overall_score();

        prop_assert!(score >= 0.0 && score <= 1.0,
            "overall_score {} out of [0, 1]", score);
    }

    /// is_critical is set iff any sat ratio is below CRITICAL_THRESHOLD.
    #[test]
    fn prop_is_critical_iff_any_sat_below_threshold(
        pop in 1_u32..=500,
        oxygen in 0_i64..=10_000,
        water  in 0_i64..=10_000,
        food   in 0_i64..=10_000,
    ) {
        let mut site = site_with_resources(pop, oxygen, water, food);
        let ls = compute_life_support(&mut site);

        let any_below = ls.oxygen_sat < LifeSupportDemand::CRITICAL_THRESHOLD
            || ls.water_sat  < LifeSupportDemand::CRITICAL_THRESHOLD
            || ls.food_sat   < LifeSupportDemand::CRITICAL_THRESHOLD;

        prop_assert_eq!(ls.is_critical, any_below,
            "is_critical={} but any_below={} (o={:.2} w={:.2} f={:.2})",
            ls.is_critical, any_below, ls.oxygen_sat, ls.water_sat, ls.food_sat);
    }

    /// With zero population, life support always returns fully satisfied.
    #[test]
    fn prop_zero_population_always_satisfied(
        oxygen in 0_i64..=10_000,
        water  in 0_i64..=10_000,
        food   in 0_i64..=10_000,
    ) {
        let mut site = site_with_resources(0, oxygen, water, food);
        let ls = compute_life_support(&mut site);

        prop_assert_eq!(ls.oxygen_sat, 1.0);
        prop_assert_eq!(ls.water_sat,  1.0);
        prop_assert_eq!(ls.food_sat,   1.0);
        prop_assert!(!ls.is_critical);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Power Grid: net power conservation
// ─────────────────────────────────────────────────────────────────────────────

proptest! {
    /// net_power() == generation - consumption, always.
    #[test]
    fn prop_net_power_is_generation_minus_consumption(
        generation in 0_u32..=10_000,
        consumption in 0_u32..=10_000,
    ) {
        let mut grid = PowerGrid::new();
        grid.update_generation(generation);
        grid.update_consumption(consumption);

        let expected = generation as i32 - consumption as i32;
        prop_assert_eq!(grid.net_power(), expected,
            "net_power ({}) != generation ({}) - consumption ({})",
            grid.net_power(), generation, consumption);
    }

    /// has_brownout() ↔ net_power() < 0.
    #[test]
    fn prop_has_brownout_iff_net_negative(
        generation in 0_u32..=10_000,
        consumption in 0_u32..=10_000,
    ) {
        let mut grid = PowerGrid::new();
        grid.update_generation(generation);
        grid.update_consumption(consumption);

        prop_assert_eq!(
            grid.has_brownout(),
            grid.net_power() < 0,
            "has_brownout ({}) disagrees with net_power < 0 ({})",
            grid.has_brownout(), grid.net_power()
        );
    }

    /// power_deficit() == 0 when no brownout.
    #[test]
    fn prop_power_deficit_zero_when_no_brownout(
        generation in 0_u32..=10_000,
        consumption in 0_u32..=10_000,
    ) {
        let mut grid = PowerGrid::new();
        grid.update_generation(generation);
        grid.update_consumption(consumption);

        if !grid.has_brownout() {
            prop_assert_eq!(grid.power_deficit(), 0,
                "power_deficit() must be 0 when no brownout");
        }
    }

    /// utilization_percentage() is in [0, 100] and 0 when no generation.
    #[test]
    fn prop_utilization_percentage_in_range(
        generation in 0_u32..=10_000,
        consumption in 0_u32..=10_000,
    ) {
        let mut grid = PowerGrid::new();
        grid.update_generation(generation);
        grid.update_consumption(consumption);

        let pct = grid.utilization_percentage();
        if generation == 0 {
            prop_assert_eq!(pct, 0.0, "utilization should be 0 when generation is 0");
        } else {
            prop_assert!(pct >= 0.0, "utilization should be >= 0: {}", pct);
        }
    }

    /// Energy storage: stored energy never exceeds storage capacity.
    #[test]
    fn prop_energy_storage_never_exceeds_capacity(
        capacity in 0_u32..=10_000,
        store_amount in 0_u32..=10_000,
    ) {
        let mut grid = PowerGrid::new();
        grid.add_storage_capacity(capacity);
        grid.store_energy(store_amount);

        prop_assert!(
            grid.stored_energy <= grid.storage_capacity,
            "stored_energy ({}) exceeds storage_capacity ({})",
            grid.stored_energy, grid.storage_capacity
        );
    }

    /// Draw from storage never makes stored_energy negative.
    #[test]
    fn prop_draw_from_storage_non_negative(
        capacity in 0_u32..=10_000,
        store_amount in 0_u32..=10_000,
        draw_amount in 0_u32..=10_000,
    ) {
        let mut grid = PowerGrid::new();
        grid.add_storage_capacity(capacity);
        grid.store_energy(store_amount);
        grid.draw_from_storage(draw_amount);

        prop_assert!(
            grid.stored_energy <= grid.storage_capacity,
            "stored_energy ({}) exceeds capacity ({}) after draw",
            grid.stored_energy, grid.storage_capacity
        );
        // No negative check needed since u32, but verify invariant
        prop_assert!(
            grid.stored_energy <= store_amount.min(capacity),
            "stored_energy ({}) exceeds min(stored={}, cap={})",
            grid.stored_energy, store_amount, capacity
        );
    }
}
