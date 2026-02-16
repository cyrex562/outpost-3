use thiserror::Error;

use super::Command;
use crate::domain::*;
use crate::events::EventType;

#[derive(Error, Debug)]
pub enum RailError {
    #[error("Insufficient resources")]
    InsufficientResources,

    #[error("Invalid rail construction: {0}")]
    InvalidConstruction(String),

    #[error("Segment {segment_id:?} not found")]
    SegmentNotFound { segment_id: SegmentId },

    #[error("Invalid upgrade: {0}")]
    InvalidUpgrade(String),

    #[error("Colony {colony_id:?} not found")]
    ColonyNotFound { colony_id: ColonyId },

    #[error("Building {building_id:?} not found")]
    BuildingNotFound { building_id: BuildingId },

    #[error("Station already exists at building {building_id:?}")]
    StationAlreadyExists { building_id: BuildingId },

    #[error("No rail segments adjacent to building")]
    NoAdjacentRail,
}

/// Command to build a new rail segment
pub struct BuildRail {
    pub colony_id: ColonyId,
    pub segment_id: SegmentId,
    pub from_tile: HexCoord,
    pub to_tile: HexCoord,
    pub rail_type: RailType,
    pub available_resources: Resources,
}

impl Command for BuildRail {
    type Error = RailError;

    fn validate(&self) -> Result<(), Self::Error> {
        // Validate tiles are adjacent
        if !self.from_tile.is_adjacent(&self.to_tile) {
            return Err(RailError::InvalidConstruction(
                "Tiles must be adjacent to build rail".to_string(),
            ));
        }

        // Calculate cost and check resources
        let distance = self.from_tile.distance(&self.to_tile) as f32;
        let cost = calculate_construction_cost(self.rail_type, distance);

        if !self.available_resources.can_afford(&cost) {
            return Err(RailError::InsufficientResources);
        }

        Ok(())
    }

    fn execute(&self) -> Result<Vec<EventType>, Self::Error> {
        self.validate()?;

        let distance = self.from_tile.distance(&self.to_tile) as f32;
        let cost = calculate_construction_cost(self.rail_type, distance);

        Ok(vec![
            EventType::RailConstructionStarted {
                segment_id: self.segment_id,
                colony_id: self.colony_id,
                from: self.from_tile,
                to: self.to_tile,
                rail_type: self.rail_type,
            },
            EventType::ResourcesConsumed {
                colony_id: self.colony_id,
                resource_type: ResourceType::Credits,
                amount: cost.get(ResourceType::Credits),
            },
            EventType::ResourcesConsumed {
                colony_id: self.colony_id,
                resource_type: ResourceType::Steel,
                amount: cost.get(ResourceType::Steel),
            },
        ])
    }
}

/// Command to upgrade an existing rail segment
pub struct UpgradeRail {
    pub colony_id: ColonyId,
    pub segment_id: SegmentId,
    pub current_rail_type: RailType,
    pub current_track_count: u8,
    pub upgrade_type: UpgradeType,
    pub available_resources: Resources,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpgradeType {
    /// Upgrade rail type (e.g., Standard -> Advanced)
    UpgradeType(RailType),
    /// Add a parallel track
    AddTrack,
}

impl Command for UpgradeRail {
    type Error = RailError;

    fn validate(&self) -> Result<(), Self::Error> {
        match self.upgrade_type {
            UpgradeType::UpgradeType(new_type) => {
                // Create temporary segment to check upgrade validity
                let temp_segment = RailSegment {
                    id: self.segment_id,
                    from_tile: HexCoord::new(0, 0),
                    to_tile: HexCoord::new(1, 0),
                    rail_type: self.current_rail_type,
                    track_count: self.current_track_count,
                };

                if !temp_segment.can_upgrade(new_type) {
                    return Err(RailError::InvalidUpgrade(format!(
                        "Cannot upgrade {:?} to {:?}",
                        self.current_rail_type, new_type
                    )));
                }

                // Check resources for type upgrade
                let cost = calculate_construction_cost(new_type, 1.0);
                if !self.available_resources.can_afford(&cost) {
                    return Err(RailError::InsufficientResources);
                }
            }
            UpgradeType::AddTrack => {
                if self.current_track_count >= 8 {
                    return Err(RailError::InvalidUpgrade(
                        "Maximum track count (8) already reached".to_string(),
                    ));
                }

                // Check resources for track addition (50% of base cost)
                let mut cost = calculate_construction_cost(self.current_rail_type, 1.0);
                cost.set(
                    ResourceType::Credits,
                    cost.get(ResourceType::Credits) / 2,
                );
                cost.set(ResourceType::Steel, cost.get(ResourceType::Steel) / 2);

                if !self.available_resources.can_afford(&cost) {
                    return Err(RailError::InsufficientResources);
                }
            }
        }

        Ok(())
    }

    fn execute(&self) -> Result<Vec<EventType>, Self::Error> {
        self.validate()?;

        let mut events = Vec::new();

        match self.upgrade_type {
            UpgradeType::UpgradeType(new_type) => {
                let cost = calculate_construction_cost(new_type, 1.0);

                events.push(EventType::RailUpgraded {
                    segment_id: self.segment_id,
                    colony_id: self.colony_id,
                    new_type,
                    new_count: self.current_track_count,
                });

                events.push(EventType::ResourcesConsumed {
                    colony_id: self.colony_id,
                    resource_type: ResourceType::Credits,
                    amount: cost.get(ResourceType::Credits),
                });

                events.push(EventType::ResourcesConsumed {
                    colony_id: self.colony_id,
                    resource_type: ResourceType::Steel,
                    amount: cost.get(ResourceType::Steel),
                });
            }
            UpgradeType::AddTrack => {
                let mut cost = calculate_construction_cost(self.current_rail_type, 1.0);
                cost.set(
                    ResourceType::Credits,
                    cost.get(ResourceType::Credits) / 2,
                );
                cost.set(ResourceType::Steel, cost.get(ResourceType::Steel) / 2);

                let new_count = self.current_track_count + 1;

                events.push(EventType::RailUpgraded {
                    segment_id: self.segment_id,
                    colony_id: self.colony_id,
                    new_type: self.current_rail_type,
                    new_count,
                });

                events.push(EventType::ResourcesConsumed {
                    colony_id: self.colony_id,
                    resource_type: ResourceType::Credits,
                    amount: cost.get(ResourceType::Credits),
                });

                events.push(EventType::ResourcesConsumed {
                    colony_id: self.colony_id,
                    resource_type: ResourceType::Steel,
                    amount: cost.get(ResourceType::Steel),
                });
            }
        }

        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_rail_validation() {
        let mut resources = Resources::new();
        resources.set(ResourceType::Credits, 1000);
        resources.set(ResourceType::Steel, 50);

        let cmd = BuildRail {
            colony_id: ColonyId(1),
            segment_id: SegmentId(1),
            from_tile: HexCoord::new(0, 0),
            to_tile: HexCoord::new(1, 0),
            rail_type: RailType::Standard,
            available_resources: resources,
        };

        assert!(cmd.validate().is_ok());
    }

    #[test]
    fn test_build_rail_non_adjacent_fails() {
        let mut resources = Resources::new();
        resources.set(ResourceType::Credits, 1000);
        resources.set(ResourceType::Steel, 50);

        let cmd = BuildRail {
            colony_id: ColonyId(1),
            segment_id: SegmentId(1),
            from_tile: HexCoord::new(0, 0),
            to_tile: HexCoord::new(2, 0),
            rail_type: RailType::Standard,
            available_resources: resources,
        };

        assert!(cmd.validate().is_err());
    }

    #[test]
    fn test_build_rail_insufficient_resources() {
        let resources = Resources::new(); // Empty resources

        let cmd = BuildRail {
            colony_id: ColonyId(1),
            segment_id: SegmentId(1),
            from_tile: HexCoord::new(0, 0),
            to_tile: HexCoord::new(1, 0),
            rail_type: RailType::Standard,
            available_resources: resources,
        };

        assert!(matches!(
            cmd.validate(),
            Err(RailError::InsufficientResources)
        ));
    }

    #[test]
    fn test_build_rail_execute_generates_events() {
        let mut resources = Resources::new();
        resources.set(ResourceType::Credits, 1000);
        resources.set(ResourceType::Steel, 50);

        let cmd = BuildRail {
            colony_id: ColonyId(1),
            segment_id: SegmentId(1),
            from_tile: HexCoord::new(0, 0),
            to_tile: HexCoord::new(1, 0),
            rail_type: RailType::Standard,
            available_resources: resources,
        };

        let events = cmd.execute().unwrap();
        assert_eq!(events.len(), 3); // Construction started + 2 resource consumed

        assert!(matches!(
            events[0],
            EventType::RailConstructionStarted { .. }
        ));
    }

    #[test]
    fn test_upgrade_rail_type() {
        let mut resources = Resources::new();
        resources.set(ResourceType::Credits, 1000);
        resources.set(ResourceType::Steel, 50);

        let cmd = UpgradeRail {
            colony_id: ColonyId(1),
            segment_id: SegmentId(1),
            current_rail_type: RailType::Standard,
            current_track_count: 1,
            upgrade_type: UpgradeType::UpgradeType(RailType::Advanced),
            available_resources: resources,
        };

        assert!(cmd.validate().is_ok());
        let events = cmd.execute().unwrap();
        assert!(events.len() >= 1);
    }

    #[test]
    fn test_upgrade_rail_add_track() {
        let mut resources = Resources::new();
        resources.set(ResourceType::Credits, 1000);
        resources.set(ResourceType::Steel, 50);

        let cmd = UpgradeRail {
            colony_id: ColonyId(1),
            segment_id: SegmentId(1),
            current_rail_type: RailType::Standard,
            current_track_count: 1,
            upgrade_type: UpgradeType::AddTrack,
            available_resources: resources,
        };

        assert!(cmd.validate().is_ok());
        let events = cmd.execute().unwrap();
        assert!(events.len() >= 1);

        if let EventType::RailUpgraded { new_count, .. } = events[0] {
            assert_eq!(new_count, 2);
        } else {
            panic!("Expected RailUpgraded event");
        }
    }

    #[test]
    fn test_upgrade_rail_max_tracks() {
        let mut resources = Resources::new();
        resources.set(ResourceType::Credits, 1000);
        resources.set(ResourceType::Steel, 50);

        let cmd = UpgradeRail {
            colony_id: ColonyId(1),
            segment_id: SegmentId(1),
            current_rail_type: RailType::Standard,
            current_track_count: 8,
            upgrade_type: UpgradeType::AddTrack,
            available_resources: resources,
        };

        assert!(cmd.validate().is_err());
    }

    #[test]
    fn test_upgrade_rail_invalid_type() {
        let mut resources = Resources::new();
        resources.set(ResourceType::Credits, 1000);
        resources.set(ResourceType::Steel, 50);

        let cmd = UpgradeRail {
            colony_id: ColonyId(1),
            segment_id: SegmentId(1),
            current_rail_type: RailType::Advanced,
            current_track_count: 1,
            upgrade_type: UpgradeType::UpgradeType(RailType::Standard), // Downgrade not allowed
            available_resources: resources,
        };

        assert!(cmd.validate().is_err());
    }
}

/// Command to build a station at a building
pub struct BuildStation {
    pub colony_id: ColonyId,
    pub station_id: StationId,
    pub building_id: BuildingId,
    pub rail_sides: Vec<Direction>,
    pub connected_segments: Vec<SegmentId>,
    pub available_resources: Resources,
}

impl Command for BuildStation {
    type Error = RailError;

    fn validate(&self) -> Result<(), Self::Error> {
        // Check if building exists (would need access to game state in real impl)
        // For now, validate that we have rail connections
        if self.connected_segments.is_empty() {
            return Err(RailError::NoAdjacentRail);
        }

        // Check resources for station construction
        let mut cost = Resources::new();
        cost.set(ResourceType::Credits, 2000); // Base station cost
        cost.set(ResourceType::Steel, 30);
        cost.set(ResourceType::Electronics, 10);

        if !self.available_resources.can_afford(&cost) {
            return Err(RailError::InsufficientResources);
        }

        Ok(())
    }

    fn execute(&self) -> Result<Vec<EventType>, Self::Error> {
        self.validate()?;

        let mut cost = Resources::new();
        cost.set(ResourceType::Credits, 2000);
        cost.set(ResourceType::Steel, 30);
        cost.set(ResourceType::Electronics, 10);

        Ok(vec![
            EventType::StationBuilt {
                station_id: self.station_id,
                colony_id: self.colony_id,
                building_id: self.building_id,
                connected_segments: self.connected_segments.clone(),
            },
            EventType::ResourcesConsumed {
                colony_id: self.colony_id,
                resource_type: ResourceType::Credits,
                amount: cost.get(ResourceType::Credits),
            },
            EventType::ResourcesConsumed {
                colony_id: self.colony_id,
                resource_type: ResourceType::Steel,
                amount: cost.get(ResourceType::Steel),
            },
            EventType::ResourcesConsumed {
                colony_id: self.colony_id,
                resource_type: ResourceType::Electronics,
                amount: cost.get(ResourceType::Electronics),
            },
        ])
    }
}

#[cfg(test)]
mod station_tests {
    use super::*;

    #[test]
    fn test_build_station_valid() {
        let mut resources = Resources::new();
        resources.set(ResourceType::Credits, 5000);
        resources.set(ResourceType::Steel, 100);
        resources.set(ResourceType::Electronics, 50);

        let cmd = BuildStation {
            colony_id: ColonyId(1),
            station_id: StationId(1),
            building_id: BuildingId(10),
            rail_sides: vec![Direction::North],
            connected_segments: vec![SegmentId(5)],
            available_resources: resources,
        };

        assert!(cmd.validate().is_ok());
        let events = cmd.execute().unwrap();
        assert!(events.len() >= 1);

        assert!(matches!(events[0], EventType::StationBuilt { .. }));
    }

    #[test]
    fn test_build_station_no_rail() {
        let mut resources = Resources::new();
        resources.set(ResourceType::Credits, 5000);
        resources.set(ResourceType::Steel, 100);
        resources.set(ResourceType::Electronics, 50);

        let cmd = BuildStation {
            colony_id: ColonyId(1),
            station_id: StationId(1),
            building_id: BuildingId(10),
            rail_sides: vec![Direction::North],
            connected_segments: vec![], // No rail connections
            available_resources: resources,
        };

        assert!(matches!(cmd.validate(), Err(RailError::NoAdjacentRail)));
    }

    #[test]
    fn test_build_station_insufficient_resources() {
        let resources = Resources::new(); // Empty

        let cmd = BuildStation {
            colony_id: ColonyId(1),
            station_id: StationId(1),
            building_id: BuildingId(10),
            rail_sides: vec![Direction::North],
            connected_segments: vec![SegmentId(5)],
            available_resources: resources,
        };

        assert!(matches!(
            cmd.validate(),
            Err(RailError::InsufficientResources)
        ));
    }
}

/// Command to purchase an engine
pub struct PurchaseEngine {
    pub colony_id: ColonyId,
    pub engine_id: EngineId,
    pub engine_type: crate::domain::train::EngineType,
    pub available_resources: Resources,
}

impl Command for PurchaseEngine {
    type Error = RailError;

    fn validate(&self) -> Result<(), Self::Error> {
        let mut cost = Resources::new();
        cost.set(ResourceType::Credits, self.engine_type.cost_credits);

        if !self.available_resources.can_afford(&cost) {
            return Err(RailError::InsufficientResources);
        }

        Ok(())
    }

    fn execute(&self) -> Result<Vec<EventType>, Self::Error> {
        self.validate()?;

        Ok(vec![
            EventType::TrainPurchased {
                train_id: TrainId(self.engine_id.0),
                colony_id: self.colony_id,
                train_type: TrainType::Freight,
                cost: self.engine_type.cost_credits,
            },
            EventType::ResourcesConsumed {
                colony_id: self.colony_id,
                resource_type: ResourceType::Credits,
                amount: self.engine_type.cost_credits,
            },
        ])
    }
}

/// Command to purchase a freight car
pub struct PurchaseCar {
    pub colony_id: ColonyId,
    pub car_id: CarId,
    pub car_type: crate::domain::train::CarType,
    pub available_resources: Resources,
}

impl Command for PurchaseCar {
    type Error = RailError;

    fn validate(&self) -> Result<(), Self::Error> {
        let mut cost = Resources::new();
        cost.set(ResourceType::Credits, self.car_type.cost_credits);

        if !self.available_resources.can_afford(&cost) {
            return Err(RailError::InsufficientResources);
        }

        Ok(())
    }

    fn execute(&self) -> Result<Vec<EventType>, Self::Error> {
        self.validate()?;

        Ok(vec![EventType::ResourcesConsumed {
            colony_id: self.colony_id,
            resource_type: ResourceType::Credits,
            amount: self.car_type.cost_credits,
        }])
    }
}

/// Command to assemble a consist from purchased engine and cars
pub struct AssembleConsist {
    pub colony_id: ColonyId,
    pub consist_id: ConsistId,
    pub engine_id: EngineId,
    pub car_ids: Vec<CarId>,
}

impl Command for AssembleConsist {
    type Error = RailError;

    fn validate(&self) -> Result<(), Self::Error> {
        if self.car_ids.len() > crate::domain::train::Consist::MAX_CARS {
            return Err(RailError::InvalidConstruction(format!(
                "Cannot assemble consist with more than {} cars",
                crate::domain::train::Consist::MAX_CARS
            )));
        }

        Ok(())
    }

    fn execute(&self) -> Result<Vec<EventType>, Self::Error> {
        self.validate()?;

        Ok(vec![EventType::TrainPurchased {
            train_id: TrainId(self.consist_id.0),
            colony_id: self.colony_id,
            train_type: TrainType::Freight,
            cost: 0, // Already paid for engine and cars separately
        }])
    }
}

#[cfg(test)]
mod train_command_tests {
    use super::*;
    use crate::domain::train::{CarType, EngineType};

    #[test]
    fn test_purchase_engine() {
        let mut resources = Resources::new();
        resources.set(ResourceType::Credits, 20000);

        let cmd = PurchaseEngine {
            colony_id: ColonyId(1),
            engine_id: EngineId(1),
            engine_type: EngineType::diesel(),  // Cost: 10,000
            available_resources: resources,
        };

        assert!(cmd.validate().is_ok());
        let events = cmd.execute().unwrap();
        assert!(events.len() >= 1);
    }

    #[test]
    fn test_purchase_engine_insufficient_funds() {
        let resources = Resources::new(); // Empty

        let cmd = PurchaseEngine {
            colony_id: ColonyId(1),
            engine_id: EngineId(1),
            engine_type: EngineType::diesel(),
            available_resources: resources,
        };

        assert!(matches!(
            cmd.validate(),
            Err(RailError::InsufficientResources)
        ));
    }

    #[test]
    fn test_purchase_car() {
        let mut resources = Resources::new();
        resources.set(ResourceType::Credits, 5000);

        let cmd = PurchaseCar {
            colony_id: ColonyId(1),
            car_id: CarId(1),
            car_type: CarType::flatbed(),
            available_resources: resources,
        };

        assert!(cmd.validate().is_ok());
        let events = cmd.execute().unwrap();
        assert!(events.len() >= 1);
    }

    #[test]
    fn test_assemble_consist_valid() {
        let cmd = AssembleConsist {
            colony_id: ColonyId(1),
            consist_id: ConsistId(1),
            engine_id: EngineId(1),
            car_ids: vec![CarId(1), CarId(2), CarId(3)],
        };

        assert!(cmd.validate().is_ok());
        let events = cmd.execute().unwrap();
        assert!(events.len() >= 1);
    }

    #[test]
    fn test_assemble_consist_too_many_cars() {
        let mut car_ids = Vec::new();
        for i in 0..20 {
            // More than MAX_CARS
            car_ids.push(CarId(i));
        }

        let cmd = AssembleConsist {
            colony_id: ColonyId(1),
            consist_id: ConsistId(1),
            engine_id: EngineId(1),
            car_ids,
        };

        assert!(cmd.validate().is_err());
    }
}
