use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PowerGrid {
    pub total_generation: u32,
    pub total_consumption: u32,
    pub storage_capacity: u32,
    pub stored_energy: u32,
}

impl PowerGrid {
    pub fn new() -> Self {
        Self {
            total_generation: 0,
            total_consumption: 0,
            storage_capacity: 0,
            stored_energy: 0,
        }
    }

    pub fn update_generation(&mut self, generation: u32) {
        self.total_generation = generation;
    }

    pub fn update_consumption(&mut self, consumption: u32) {
        self.total_consumption = consumption;
    }

    pub fn net_power(&self) -> i32 {
        self.total_generation as i32 - self.total_consumption as i32
    }

    pub fn has_brownout(&self) -> bool {
        self.net_power() < 0
    }

    pub fn power_deficit(&self) -> u32 {
        if self.has_brownout() {
            (-self.net_power()) as u32
        } else {
            0
        }
    }

    pub fn power_surplus(&self) -> u32 {
        if !self.has_brownout() {
            self.net_power() as u32
        } else {
            0
        }
    }

    pub fn utilization_percentage(&self) -> f32 {
        if self.total_generation == 0 {
            return 0.0;
        }
        (self.total_consumption as f32 / self.total_generation as f32) * 100.0
    }

    pub fn efficiency_rating(&self) -> &'static str {
        let util = self.utilization_percentage();
        match util {
            0.0..=30.0 => "Very Low",
            30.1..=50.0 => "Low",
            50.1..=75.0 => "Good",
            75.1..=90.0 => "Optimal",
            90.1..=99.9 => "High",
            _ => "Overloaded",
        }
    }

    pub fn can_support_additional_load(&self, additional_load: u32) -> bool {
        self.total_generation >= self.total_consumption + additional_load
    }

    pub fn add_storage_capacity(&mut self, capacity: u32) {
        self.storage_capacity += capacity;
    }

    pub fn store_energy(&mut self, amount: u32) -> u32 {
        let space_available = self.storage_capacity.saturating_sub(self.stored_energy);
        let to_store = amount.min(space_available);
        self.stored_energy += to_store;
        to_store
    }

    pub fn draw_from_storage(&mut self, amount: u32) -> u32 {
        let to_draw = amount.min(self.stored_energy);
        self.stored_energy -= to_draw;
        to_draw
    }
}

impl Default for PowerGrid {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_grid_creation() {
        let grid = PowerGrid::new();
        assert_eq!(grid.total_generation, 0);
        assert_eq!(grid.total_consumption, 0);
        assert!(!grid.has_brownout());
    }

    #[test]
    fn test_net_power() {
        let mut grid = PowerGrid::new();
        grid.update_generation(100);
        grid.update_consumption(75);
        assert_eq!(grid.net_power(), 25);
        assert!(!grid.has_brownout());
    }

    #[test]
    fn test_brownout() {
        let mut grid = PowerGrid::new();
        grid.update_generation(50);
        grid.update_consumption(75);
        assert!(grid.has_brownout());
        assert_eq!(grid.power_deficit(), 25);
    }

    #[test]
    fn test_utilization() {
        let mut grid = PowerGrid::new();
        grid.update_generation(100);
        grid.update_consumption(80);
        assert_eq!(grid.utilization_percentage(), 80.0);
        assert_eq!(grid.efficiency_rating(), "Optimal");
    }

    #[test]
    fn test_additional_load() {
        let mut grid = PowerGrid::new();
        grid.update_generation(100);
        grid.update_consumption(70);
        assert!(grid.can_support_additional_load(20));
        assert!(!grid.can_support_additional_load(40));
    }

    #[test]
    fn test_energy_storage() {
        let mut grid = PowerGrid::new();
        grid.add_storage_capacity(100);
        assert_eq!(grid.store_energy(60), 60);
        assert_eq!(grid.stored_energy, 60);
        assert_eq!(grid.draw_from_storage(30), 30);
        assert_eq!(grid.stored_energy, 30);
    }
}
