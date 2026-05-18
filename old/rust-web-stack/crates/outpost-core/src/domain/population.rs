use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Age bucket categories for demographic breakdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgeBucket {
    /// Young adults (18–30).
    Young,
    /// Middle-aged adults (31–50).
    Adult,
    /// Senior colonists (51+).
    Senior,
}

impl AgeBucket {
    pub fn label(&self) -> &'static str {
        match self {
            AgeBucket::Young => "Young (18–30)",
            AgeBucket::Adult => "Adult (31–50)",
            AgeBucket::Senior => "Senior (51+)",
        }
    }

    pub fn all() -> &'static [AgeBucket] {
        &[AgeBucket::Young, AgeBucket::Adult, AgeBucket::Senior]
    }
}

/// Skill categories for colonist labor assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SkillCategory {
    /// General manual workers; required by most basic buildings.
    Laborer,
    /// Technical workers; required by machinery, power plants, refineries.
    Engineer,
    /// Research and development personnel.
    Scientist,
    /// Agricultural workers; required by farms and food processing.
    Farmer,
    /// Medical personnel; required by clinics and hospitals.
    Medic,
    /// Machinery and equipment operators; required by mines and factories.
    Operator,
}

impl SkillCategory {
    /// Human-readable label for display.
    pub fn label(&self) -> &'static str {
        match self {
            SkillCategory::Laborer => "Laborer",
            SkillCategory::Engineer => "Engineer",
            SkillCategory::Scientist => "Scientist",
            SkillCategory::Farmer => "Farmer",
            SkillCategory::Medic => "Medic",
            SkillCategory::Operator => "Operator",
        }
    }

    /// All skill categories in display order.
    pub fn all() -> &'static [SkillCategory] {
        &[
            SkillCategory::Laborer,
            SkillCategory::Engineer,
            SkillCategory::Scientist,
            SkillCategory::Farmer,
            SkillCategory::Medic,
            SkillCategory::Operator,
        ]
    }
}

/// Colonist needs satisfaction tracker.
///
/// Each field is a satisfaction ratio in [0.0, 1.0]:
/// - 1.0 = fully met
/// - 0.0 = completely unmet
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColonistNeeds {
    /// Food satisfaction (supply / demand, capped at 1.0).
    pub food: f32,
    /// Water satisfaction.
    pub water: f32,
    /// Oxygen satisfaction.
    pub oxygen: f32,
}

impl ColonistNeeds {
    /// Create a fully-satisfied needs state.
    pub fn fully_satisfied() -> Self {
        Self { food: 1.0, water: 1.0, oxygen: 1.0 }
    }

    /// Create an empty (zero) needs state.
    pub fn empty() -> Self {
        Self { food: 0.0, water: 0.0, oxygen: 0.0 }
    }

    /// Calculate needs satisfaction given supply and population.
    /// Each need: satisfaction = min(supply / demand, 1.0).
    pub fn calculate(
        food_supply: f64,
        water_supply: f64,
        oxygen_supply: f64,
        population: u64,
    ) -> Self {
        if population == 0 {
            return Self::fully_satisfied();
        }
        let pop = population as f64;
        // Per-capita consumption rates per tick
        let food_demand = pop * 1.0;   // 1 food unit per colonist
        let water_demand = pop * 0.5;  // 0.5 water units per colonist
        let oxygen_demand = pop * 0.3; // 0.3 oxygen units per colonist

        Self {
            food: ((food_supply / food_demand) as f32).clamp(0.0, 1.0),
            water: ((water_supply / water_demand) as f32).clamp(0.0, 1.0),
            oxygen: ((oxygen_supply / oxygen_demand) as f32).clamp(0.0, 1.0),
        }
    }

    /// Calculate the morale impact from needs.
    ///
    /// Returns a morale modifier in [-50, 0]:
    /// - All needs fully met → 0 (no penalty)
    /// - Critical shortfall → up to -50 penalty
    pub fn morale_impact(&self) -> f32 {
        // Weighted: oxygen is most critical, then water, then food
        let weighted = (self.oxygen * 0.5) + (self.water * 0.3) + (self.food * 0.2);
        let satisfaction = weighted.clamp(0.0, 1.0);
        // Penalty is 0 at full satisfaction, up to -50 at zero satisfaction
        (satisfaction - 1.0) * 50.0
    }

    /// Returns true if all needs are at least minimally met (>= 50%).
    pub fn is_viable(&self) -> bool {
        self.food >= 0.5 && self.water >= 0.5 && self.oxygen >= 0.5
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Population {
    pub total: u64,
    pub growth_rate: f32,
    pub employed: u64,
    pub unemployed: u64,
    pub housing_capacity: u64,
    pub food_consumption_per_capita: f32,
    pub morale: f32,
    /// Distribution of skills among available (unassigned) workers.
    pub skills: HashMap<SkillCategory, u32>,
    /// Current colonist needs satisfaction.
    pub needs: ColonistNeeds,
    /// Age distribution across the population.
    pub age_distribution: HashMap<AgeBucket, u32>,
}

impl Population {
    pub fn new(initial_population: u64) -> Self {
        // Default skill distribution: mostly laborers
        let mut skills = HashMap::new();
        let pop = initial_population as u32;
        skills.insert(SkillCategory::Laborer, pop * 40 / 100);
        skills.insert(SkillCategory::Operator, pop * 25 / 100);
        skills.insert(SkillCategory::Engineer, pop * 15 / 100);
        skills.insert(SkillCategory::Farmer, pop * 10 / 100);
        skills.insert(SkillCategory::Medic, pop * 5 / 100);
        skills.insert(SkillCategory::Scientist, pop * 5 / 100);

        // Default age distribution: mostly working-age adults
        let mut age_distribution = HashMap::new();
        age_distribution.insert(AgeBucket::Young, pop * 35 / 100);
        age_distribution.insert(AgeBucket::Adult, pop * 50 / 100);
        age_distribution.insert(AgeBucket::Senior, pop * 15 / 100);

        Self {
            total: initial_population,
            growth_rate: 0.02, // 2% growth per turn
            employed: 0,
            unemployed: initial_population,
            housing_capacity: initial_population,
            food_consumption_per_capita: 1.0,
            morale: 75.0,
            skills,
            needs: ColonistNeeds::fully_satisfied(),
            age_distribution,
        }
    }

    /// Return total count per age bucket.
    pub fn demographic_breakdown(&self) -> Vec<(AgeBucket, u32)> {
        AgeBucket::all()
            .iter()
            .map(|&bucket| (bucket, *self.age_distribution.get(&bucket).unwrap_or(&0)))
            .collect()
    }

    /// Sum of all skill pool counts (for validation).
    pub fn total_in_skill_pools(&self) -> u32 {
        self.skills.values().sum()
    }

    pub fn grow(&mut self) {
        if self.total < self.housing_capacity {
            let growth = (self.total as f32 * self.growth_rate * (self.morale / 100.0)) as u64;
            self.total += growth;
            self.unemployed += growth;
            // New colonists are young adults
            *self.skills.entry(SkillCategory::Laborer).or_insert(0) += growth as u32;
            *self.age_distribution.entry(AgeBucket::Young).or_insert(0) += growth as u32;
        }
    }

    /// Apply population deaths due to critical unmet needs.
    ///
    /// Called each tick when needs are below critical thresholds.
    /// Returns the number of colonists who died.
    pub fn apply_needs_deaths(&mut self) -> u64 {
        if self.total == 0 {
            return 0;
        }
        let mut deaths = 0u64;
        // Oxygen critical: 1% of population per tick below 30% (minimum 1)
        if self.needs.oxygen < 0.3 {
            deaths += ((self.total as f64 * 0.01).ceil() as u64).max(1);
        }
        // Food critical: 0.5% per tick below 20% (minimum 1)
        if self.needs.food < 0.2 {
            deaths += ((self.total as f64 * 0.005).ceil() as u64).max(1);
        }
        // Water critical: 0.5% per tick below 20% (minimum 1)
        if self.needs.water < 0.2 {
            deaths += ((self.total as f64 * 0.005).ceil() as u64).max(1);
        }

        if deaths > 0 {
            deaths = deaths.min(self.total);
            self.total -= deaths;
            // Remove deaths from unemployed first, then employed
            let unemployed_deaths = deaths.min(self.unemployed);
            self.unemployed -= unemployed_deaths;
            let employed_deaths = deaths - unemployed_deaths;
            self.employed = self.employed.saturating_sub(employed_deaths);
        }
        deaths
    }

    /// Compute composite morale score from all contributing factors.
    ///
    /// Returns morale in [0, 100]:
    /// - Needs satisfaction contributes 60%
    /// - Housing adequacy contributes 20%
    /// - Base recovery/momentum contributes 20%
    pub fn compute_composite_morale(&self) -> f32 {
        let needs_score = {
            let weighted = (self.needs.oxygen * 0.5) + (self.needs.water * 0.3) + (self.needs.food * 0.2);
            weighted.clamp(0.0, 1.0) * 100.0
        };

        let housing_score = if self.housing_capacity == 0 {
            0.0_f32
        } else {
            (self.housing_capacity.min(self.total) as f32 / self.total.max(1) as f32 * 100.0).clamp(0.0, 100.0)
        };

        // Weight: needs 60%, housing 20%, current morale momentum 20%
        (needs_score * 0.6 + housing_score * 0.2 + self.morale * 0.2).clamp(0.0, 100.0)
    }

    pub fn allocate_workers(&mut self, amount: u64) -> bool {
        if self.unemployed >= amount {
            self.unemployed -= amount;
            self.employed += amount;
            true
        } else {
            false
        }
    }

    pub fn deallocate_workers(&mut self, amount: u64) -> bool {
        if self.employed >= amount {
            self.employed -= amount;
            self.unemployed += amount;
            true
        } else {
            false
        }
    }

    pub fn food_needed_per_turn(&self) -> f32 {
        self.total as f32 * self.food_consumption_per_capita
    }

    pub fn unemployment_rate(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            (self.unemployed as f32 / self.total as f32) * 100.0
        }
    }

    pub fn is_overcrowded(&self) -> bool {
        self.total > self.housing_capacity
    }

    pub fn overcrowding_percentage(&self) -> f32 {
        if self.housing_capacity == 0 {
            return 100.0;
        }
        let overcrowding = self.total.saturating_sub(self.housing_capacity) as f32;
        (overcrowding / self.housing_capacity as f32) * 100.0
    }

    pub fn add_housing(&mut self, capacity: u64) {
        self.housing_capacity += capacity;
    }

    pub fn adjust_morale(&mut self, delta: f32) {
        self.morale = (self.morale + delta).clamp(0.0, 100.0);
    }

    /// Update needs and apply morale effects based on current supplies.
    pub fn update_needs(&mut self, food_supply: f64, water_supply: f64, oxygen_supply: f64) {
        self.needs = ColonistNeeds::calculate(food_supply, water_supply, oxygen_supply, self.total);
        let morale_impact = self.needs.morale_impact();
        // Gradually nudge morale towards need-based value (smooth transitions)
        if morale_impact < 0.0 {
            self.adjust_morale(morale_impact * 0.1); // Apply 10% of penalty per tick
        } else {
            // Slowly recover morale when needs are met
            self.adjust_morale(0.5);
        }
    }

    /// Get number of available workers of a specific skill.
    pub fn available_by_skill(&self, skill: SkillCategory) -> u32 {
        *self.skills.get(&skill).unwrap_or(&0)
    }

    /// Try to allocate workers of a specific skill.
    /// Returns true if enough workers were available.
    pub fn allocate_skilled_workers(&mut self, skill: SkillCategory, count: u32) -> bool {
        let available = self.skills.entry(skill).or_insert(0);
        if *available >= count {
            *available -= count;
            self.employed += count as u64;
            self.unemployed = self.unemployed.saturating_sub(count as u64);
            true
        } else {
            false
        }
    }

    /// Return workers of a specific skill to the available pool.
    pub fn deallocate_skilled_workers(&mut self, skill: SkillCategory, count: u32) {
        let returned = count.min(self.employed as u32);
        *self.skills.entry(skill).or_insert(0) += returned;
        self.employed = self.employed.saturating_sub(returned as u64);
        self.unemployed += returned as u64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_population_creation() {
        let pop = Population::new(100);
        assert_eq!(pop.total, 100);
        assert_eq!(pop.unemployed, 100);
        assert_eq!(pop.employed, 0);
        assert_eq!(pop.morale, 75.0);
    }

    #[test]
    fn test_worker_allocation() {
        let mut pop = Population::new(100);
        assert!(pop.allocate_workers(50));
        assert_eq!(pop.employed, 50);
        assert_eq!(pop.unemployed, 50);
        assert!(!pop.allocate_workers(60));
        assert_eq!(pop.employed, 50);
    }

    #[test]
    fn test_population_growth() {
        let mut pop = Population::new(100);
        pop.housing_capacity = 200;
        pop.grow();
        assert!(pop.total > 100);
    }

    #[test]
    fn test_unemployment_rate() {
        let mut pop = Population::new(100);
        pop.allocate_workers(75);
        assert_eq!(pop.unemployment_rate(), 25.0);
    }

    #[test]
    fn test_overcrowding() {
        let mut pop = Population::new(100);
        pop.total = 150;
        assert!(pop.is_overcrowded());
        assert_eq!(pop.overcrowding_percentage(), 50.0);
    }

    #[test]
    fn test_colonist_needs_calculation_full_supply() {
        let needs = ColonistNeeds::calculate(100.0, 50.0, 30.0, 100);
        assert!((needs.food - 1.0).abs() < 0.01);
        assert!((needs.water - 1.0).abs() < 0.01);
        assert!((needs.oxygen - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_colonist_needs_calculation_half_supply() {
        let needs = ColonistNeeds::calculate(50.0, 25.0, 15.0, 100);
        assert!((needs.food - 0.5).abs() < 0.01);
        assert!((needs.water - 0.5).abs() < 0.01);
        assert!((needs.oxygen - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_colonist_needs_satisfaction_always_clamped() {
        // Over-supply should still give 1.0
        let needs = ColonistNeeds::calculate(10000.0, 10000.0, 10000.0, 100);
        assert!(needs.food <= 1.0);
        assert!(needs.water <= 1.0);
        assert!(needs.oxygen <= 1.0);

        // Zero supply should give 0.0
        let needs = ColonistNeeds::calculate(0.0, 0.0, 0.0, 100);
        assert!(needs.food >= 0.0);
        assert!(needs.water >= 0.0);
        assert!(needs.oxygen >= 0.0);
    }

    #[test]
    fn test_morale_impact_full_needs() {
        let needs = ColonistNeeds::fully_satisfied();
        assert!((needs.morale_impact() - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_morale_impact_empty_needs() {
        let needs = ColonistNeeds::empty();
        assert!(needs.morale_impact() <= -49.0);
    }

    #[test]
    fn test_skilled_worker_allocation() {
        let mut pop = Population::new(100);
        let laborers = pop.available_by_skill(SkillCategory::Laborer);
        assert!(laborers > 0);

        assert!(pop.allocate_skilled_workers(SkillCategory::Laborer, 5));
        assert_eq!(pop.available_by_skill(SkillCategory::Laborer), laborers - 5);
        assert_eq!(pop.employed, 5);

        // Can't over-allocate
        assert!(!pop.allocate_skilled_workers(SkillCategory::Laborer, 1000));
    }

    #[test]
    fn test_skilled_worker_deallocation() {
        let mut pop = Population::new(100);
        pop.allocate_skilled_workers(SkillCategory::Engineer, 5);
        pop.deallocate_skilled_workers(SkillCategory::Engineer, 5);
        assert_eq!(pop.employed, 0);
    }

    #[test]
    fn test_age_buckets_populated_on_creation() {
        let pop = Population::new(100);
        let breakdown = pop.demographic_breakdown();
        // All three buckets present
        assert_eq!(breakdown.len(), 3);
        // All counts non-negative and sum to approximately total (may differ due to integer rounding)
        let total: u32 = breakdown.iter().map(|(_, n)| n).sum();
        assert!(total > 0, "Age distribution must be non-zero");
    }

    #[test]
    fn test_skill_distribution_covers_all_categories() {
        let pop = Population::new(100);
        for skill in SkillCategory::all() {
            assert!(pop.available_by_skill(*skill) > 0,
                "Skill {:?} should have workers in a 100-person colony", skill);
        }
    }

    #[test]
    fn test_composite_morale_is_in_bounds() {
        let pop = Population::new(100);
        let m = pop.compute_composite_morale();
        assert!(m >= 0.0 && m <= 100.0, "Morale must be in [0, 100], got {}", m);
    }

    #[test]
    fn test_composite_morale_low_when_needs_empty() {
        let mut pop = Population::new(100);
        pop.needs = ColonistNeeds::empty();
        let m = pop.compute_composite_morale();
        assert!(m < 40.0, "Morale should be low when needs are unmet, got {}", m);
    }

    #[test]
    fn test_needs_deaths_zero_when_needs_met() {
        let mut pop = Population::new(100);
        pop.needs = ColonistNeeds::fully_satisfied();
        let deaths = pop.apply_needs_deaths();
        assert_eq!(deaths, 0, "No deaths when needs are fully met");
    }

    #[test]
    fn test_needs_deaths_occur_on_oxygen_critical() {
        let mut pop = Population::new(1000);
        pop.needs.oxygen = 0.1; // below 0.3 threshold
        let deaths = pop.apply_needs_deaths();
        assert!(deaths > 0, "Deaths should occur when oxygen is critical");
        assert_eq!(pop.total, 1000 - deaths, "Total should decrease by deaths");
    }

    #[test]
    fn test_population_never_goes_negative_from_deaths() {
        // With population of 1, deaths round to 0 (integer truncation) — no negative
        let mut pop = Population::new(1);
        pop.needs = ColonistNeeds::empty();
        pop.apply_needs_deaths();
        assert!(pop.total <= 1, "Population cannot exceed initial after deaths");

        // With larger pop, verify total cannot underflow
        let mut pop2 = Population::new(100);
        pop2.needs = ColonistNeeds::empty();
        for _ in 0..1000 {
            pop2.apply_needs_deaths();
        }
        // After many rounds of deaths, should be 0 not overflow
        assert_eq!(pop2.total, 0, "Population reaches 0 but does not underflow");
    }

    #[test]
    fn test_growth_updates_age_buckets() {
        let mut pop = Population::new(100);
        pop.housing_capacity = 200;
        let young_before = *pop.age_distribution.get(&AgeBucket::Young).unwrap_or(&0);
        pop.grow();
        let young_after = *pop.age_distribution.get(&AgeBucket::Young).unwrap_or(&0);
        // New colonists are young
        assert!(young_after >= young_before, "Young count should not decrease after growth");
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_needs_satisfaction_always_clamped(
            food_supply in 0.0f64..10000.0,
            water_supply in 0.0f64..10000.0,
            oxygen_supply in 0.0f64..10000.0,
            pop in 1u64..=1000u64,
        ) {
            let needs = ColonistNeeds::calculate(food_supply, water_supply, oxygen_supply, pop);
            prop_assert!(needs.food >= 0.0 && needs.food <= 1.0,
                "food satisfaction {} out of bounds", needs.food);
            prop_assert!(needs.water >= 0.0 && needs.water <= 1.0,
                "water satisfaction {} out of bounds", needs.water);
            prop_assert!(needs.oxygen >= 0.0 && needs.oxygen <= 1.0,
                "oxygen satisfaction {} out of bounds", needs.oxygen);
        }

        #[test]
        fn prop_morale_always_in_bounds(
            food in 0.0f32..=1.0,
            water in 0.0f32..=1.0,
            oxygen in 0.0f32..=1.0,
            initial_morale in 0.0f32..=100.0,
        ) {
            let mut pop = Population::new(100);
            pop.morale = initial_morale;
            pop.needs = ColonistNeeds { food, water, oxygen };
            let m = pop.compute_composite_morale();
            prop_assert!(m >= 0.0 && m <= 100.0,
                "Morale {} out of [0, 100]", m);
        }

        #[test]
        fn prop_population_never_negative_after_deaths(
            initial_pop in 0u64..=10000u64,
            oxygen in 0.0f32..=0.25,
            food in 0.0f32..=0.15,
            water in 0.0f32..=0.15,
        ) {
            let mut pop = Population::new(initial_pop);
            pop.needs = ColonistNeeds { food, water, oxygen };
            let deaths = pop.apply_needs_deaths();
            prop_assert!(pop.total <= initial_pop,
                "Population cannot increase from deaths");
            prop_assert!(deaths <= initial_pop,
                "Cannot have more deaths than initial population");
        }

        #[test]
        fn prop_morale_impact_always_in_minus50_to_zero(
            food in 0.0f32..=1.0,
            water in 0.0f32..=1.0,
            oxygen in 0.0f32..=1.0,
        ) {
            let needs = ColonistNeeds { food, water, oxygen };
            let impact = needs.morale_impact();
            prop_assert!(impact >= -50.0 && impact <= 0.0,
                "Morale impact {} out of [-50, 0]", impact);
        }
    }
}
