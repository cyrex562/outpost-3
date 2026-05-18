use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use super::{PlanetId, Resources, Population, PowerGrid};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ColonyId(pub u64);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Colony {
    pub id: ColonyId,
    pub planet_id: PlanetId,
    pub name: String,
    pub founded_at: DateTime<Utc>,
    pub resources: Resources,
    pub population: Population,
    pub power_grid: PowerGrid,
    pub pollution_level: f32,
}

impl Colony {
    pub fn new(id: ColonyId, planet_id: PlanetId, name: String) -> Self {
        Self {
            id,
            planet_id,
            name,
            founded_at: Utc::now(),
            resources: Resources::starting_resources(),
            population: Population::new(100),
            power_grid: PowerGrid::new(),
            pollution_level: 0.0,
        }
    }

    pub fn add_pollution(&mut self, amount: f32) {
        self.pollution_level += amount;
    }

    pub fn morale(&self) -> f32 {
        self.population.morale
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_colony_creation() {
        let colony = Colony::new(ColonyId(1), PlanetId(1), "New Hope".to_string());
        assert_eq!(colony.name, "New Hope");
        assert_eq!(colony.population.total, 100);
        assert_eq!(colony.morale(), 75.0);
    }

    #[test]
    fn test_morale_clamping() {
        let mut colony = Colony::new(ColonyId(1), PlanetId(1), "Test".to_string());
        colony.population.adjust_morale(-100.0);
        assert_eq!(colony.morale(), 0.0);

        colony.population.adjust_morale(200.0);
        assert_eq!(colony.morale(), 100.0);
    }
}
