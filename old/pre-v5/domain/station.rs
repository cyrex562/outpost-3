use serde::{Deserialize, Serialize};
use super::{BuildingId, SegmentId, ResourceType};
use crate::domain::rail::HexCoord;
use std::collections::HashMap;

/// Station identifier
pub use super::train::StationId;

/// Direction for rail connection sides
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    North,
    NorthEast,
    SouthEast,
    South,
    SouthWest,
    NorthWest,
}

/// Cargo request at a station
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CargoRequest {
    pub id: CargoRequestId,
    pub resource: ResourceType,
    pub quantity: u32,
    pub priority: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CargoRequestId(pub u64);

/// Station for boarding/unboarding cargo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Station {
    pub id: StationId,
    pub building_id: BuildingId,
    pub connected_segments: Vec<SegmentId>,
    pub capacity: u32,
    pub pending_cargo: Vec<CargoRequest>,
}

impl Station {
    pub fn new(
        id: StationId,
        building_id: BuildingId,
        connected_segments: Vec<SegmentId>,
    ) -> Self {
        Self {
            id,
            building_id,
            connected_segments,
            capacity: 500, // Default capacity
            pending_cargo: Vec::new(),
        }
    }

    /// Get available capacity
    pub fn available_capacity(&self) -> u32 {
        let used: u32 = self.pending_cargo.iter().map(|c| c.quantity).sum();
        self.capacity.saturating_sub(used)
    }

    /// Check if cargo can be added
    pub fn can_add_cargo(&self, quantity: u32) -> bool {
        self.available_capacity() >= quantity
    }

    /// Add cargo request
    pub fn add_cargo(&mut self, cargo: CargoRequest) -> Result<(), String> {
        if !self.can_add_cargo(cargo.quantity) {
            return Err("Insufficient capacity".to_string());
        }
        self.pending_cargo.push(cargo);
        Ok(())
    }

    /// Remove cargo request
    pub fn remove_cargo(&mut self, cargo_id: CargoRequestId) -> Option<CargoRequest> {
        if let Some(pos) = self
            .pending_cargo
            .iter()
            .position(|c| c.id == cargo_id)
        {
            Some(self.pending_cargo.remove(pos))
        } else {
            None
        }
    }

    /// Get highest priority cargo
    pub fn get_highest_priority_cargo(&self) -> Option<&CargoRequest> {
        self.pending_cargo.iter().max_by_key(|c| c.priority)
    }
}

/// Station module for a building
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StationModule {
    pub station_id: StationId,
    pub connected_rail_sides: Vec<Direction>,
}

impl StationModule {
    pub fn new(station_id: StationId, connected_rail_sides: Vec<Direction>) -> Self {
        Self {
            station_id,
            connected_rail_sides,
        }
    }

    /// Check if station connects to rail on a specific side
    pub fn connects_on_side(&self, direction: Direction) -> bool {
        self.connected_rail_sides.contains(&direction)
    }
}

/// Check if a station can transfer to another (connectivity check)
pub fn can_transfer_between(
    from: &Station,
    to: &Station,
    network: &crate::domain::rail::RailNetwork,
) -> bool {
    // For now, just check if both stations have connected segments
    // In full implementation, would check path exists in network
    !from.connected_segments.is_empty() && !to.connected_segments.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_station_creation() {
        let station = Station::new(StationId(1), BuildingId(10), vec![SegmentId(5)]);
        assert_eq!(station.id, StationId(1));
        assert_eq!(station.building_id, BuildingId(10));
        assert_eq!(station.capacity, 500);
        assert_eq!(station.available_capacity(), 500);
    }

    #[test]
    fn test_station_add_cargo() {
        let mut station = Station::new(StationId(1), BuildingId(10), vec![SegmentId(5)]);

        let cargo = CargoRequest {
            id: CargoRequestId(1),
            resource: ResourceType::Steel,
            quantity: 100,
            priority: 5,
        };

        assert!(station.add_cargo(cargo).is_ok());
        assert_eq!(station.available_capacity(), 400);
        assert_eq!(station.pending_cargo.len(), 1);
    }

    #[test]
    fn test_station_capacity_exceeded() {
        let mut station = Station::new(StationId(1), BuildingId(10), vec![SegmentId(5)]);

        let cargo = CargoRequest {
            id: CargoRequestId(1),
            resource: ResourceType::Steel,
            quantity: 600, // Exceeds capacity
            priority: 5,
        };

        assert!(station.add_cargo(cargo).is_err());
    }

    #[test]
    fn test_station_remove_cargo() {
        let mut station = Station::new(StationId(1), BuildingId(10), vec![SegmentId(5)]);

        let cargo = CargoRequest {
            id: CargoRequestId(1),
            resource: ResourceType::Steel,
            quantity: 100,
            priority: 5,
        };

        station.add_cargo(cargo).unwrap();
        let removed = station.remove_cargo(CargoRequestId(1));
        assert!(removed.is_some());
        assert_eq!(station.pending_cargo.len(), 0);
        assert_eq!(station.available_capacity(), 500);
    }

    #[test]
    fn test_station_priority_cargo() {
        let mut station = Station::new(StationId(1), BuildingId(10), vec![SegmentId(5)]);

        station
            .add_cargo(CargoRequest {
                id: CargoRequestId(1),
                resource: ResourceType::Steel,
                quantity: 50,
                priority: 3,
            })
            .unwrap();

        station
            .add_cargo(CargoRequest {
                id: CargoRequestId(2),
                resource: ResourceType::Food,
                quantity: 30,
                priority: 8,
            })
            .unwrap();

        let highest = station.get_highest_priority_cargo().unwrap();
        assert_eq!(highest.id, CargoRequestId(2));
        assert_eq!(highest.priority, 8);
    }

    #[test]
    fn test_station_module_creation() {
        let module = StationModule::new(
            StationId(1),
            vec![Direction::North, Direction::NorthEast],
        );

        assert!(module.connects_on_side(Direction::North));
        assert!(!module.connects_on_side(Direction::South));
    }

    #[test]
    fn test_station_module_all_directions() {
        let all_directions = vec![
            Direction::North,
            Direction::NorthEast,
            Direction::SouthEast,
            Direction::South,
            Direction::SouthWest,
            Direction::NorthWest,
        ];

        let module = StationModule::new(StationId(1), all_directions.clone());

        for direction in all_directions {
            assert!(module.connects_on_side(direction));
        }
    }
}
