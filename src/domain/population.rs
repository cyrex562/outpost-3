use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Population {
    pub total: u64,
    pub growth_rate: f32,
    pub employed: u64,
    pub unemployed: u64,
    pub housing_capacity: u64,
    pub food_consumption_per_capita: f32,
    pub morale: f32,
}

impl Population {
    pub fn new(initial_population: u64) -> Self {
        Self {
            total: initial_population,
            growth_rate: 0.02, // 2% growth per turn
            employed: 0,
            unemployed: initial_population,
            housing_capacity: initial_population,
            food_consumption_per_capita: 1.0,
            morale: 75.0,
        }
    }

    pub fn grow(&mut self) {
        if self.total < self.housing_capacity {
            let growth = (self.total as f32 * self.growth_rate * (self.morale / 100.0)) as u64;
            self.total += growth;
            self.unemployed += growth;
        }
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
        pop.housing_capacity = 200; // Ensure there's room to grow
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
}
