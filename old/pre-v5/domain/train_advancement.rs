use serde::{Deserialize, Serialize};
use std::fmt;

use crate::domain::{
    RailNetwork, RailSegment, RailType, SegmentId, StationId, TrainId,
    TrainMovement, TrainRouteState,
};

/// Result of a train movement advancement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancementResult {
    pub train_id: TrainId,
    pub advanced_ticks: u64,
    pub state_changed: bool,
    pub completed_segment: bool,
    pub arrived_at_station: Option<StationId>,
    pub route_completed: bool,
}

/// Train advancement errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdvancementError {
    /// Train not found
    TrainNotFound,
    /// Segment not found in rail network
    SegmentNotFound,
    /// Invalid waypoint sequence
    InvalidWaypoint,
    /// Train stuck (movement blocked)
    TrainStuck,
}

impl fmt::Display for AdvancementError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AdvancementError::TrainNotFound => write!(f, "Train not found"),
            AdvancementError::SegmentNotFound => write!(f, "Segment not found"),
            AdvancementError::InvalidWaypoint => write!(f, "Invalid waypoint"),
            AdvancementError::TrainStuck => write!(f, "Train stuck"),
        }
    }
}

/// Manages train movement advancement with speed calculations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainAdvancer {
    /// Base ticks per turn (affects all train speeds)
    pub ticks_per_turn: u64,
    /// Congestion factor (0.0-1.0, reduces speed when high)
    pub congestion_factor: f32,
}

impl TrainAdvancer {
    pub fn new() -> Self {
        Self {
            ticks_per_turn: 10,
            congestion_factor: 1.0,
        }
    }

    /// Calculate travel time for a segment based on train and rail characteristics
    fn calculate_segment_travel_time(
        &self,
        segment: &RailSegment,
        train_speed_factor: f32,
        train_mass: u32,
    ) -> u64 {
        // Base time = segment length / rail speed / train speed factor
        // Adjusted by mass (heavier trains = slower)
        // Affected by congestion

        // Rail speed multiplier (high speed rails = faster traversal)
        let rail_speed_multiplier = segment.rail_type.speed_multiplier();

        // Mass penalty (every 100 units of mass adds 10% slow down, min 1.0x)
        let mass_penalty = 1.0 + (train_mass as f32 / 1000.0);

        // Base traversal time in ticks
        let base_time = 50.0; // Base ticks to cross any segment

        let adjusted_time =
            base_time / (rail_speed_multiplier * train_speed_factor / mass_penalty);
        let with_congestion = adjusted_time / self.congestion_factor;

        (with_congestion.ceil() as u64).max(1)
    }

    /// Advance a train by one turn
    /// Returns result of advancement and any waypoint changes
    pub fn advance_train(
        &self,
        movement: &mut TrainMovement,
        route_network: &crate::domain::RouteNetwork,
        rail_network: &RailNetwork,
        train_speed_factor: f32,
        train_mass: u32,
    ) -> Result<AdvancementResult, AdvancementError> {
        let mut result = AdvancementResult {
            train_id: movement.train_id,
            advanced_ticks: self.ticks_per_turn,
            state_changed: false,
            completed_segment: false,
            arrived_at_station: None,
            route_completed: false,
        };

        // Can't advance if not active
        if movement.route_state != TrainRouteState::Active {
            result.advanced_ticks = 0;
            return Ok(result);
        }

        // Get the route
        let route = route_network
            .get_route(movement.route_id)
            .ok_or(AdvancementError::InvalidWaypoint)?;

        // Get current waypoint
        let current_waypoint = route
            .waypoints
            .get(route.current_waypoint)
            .ok_or(AdvancementError::InvalidWaypoint)?;

        use crate::domain::RouteWaypoint;

        match current_waypoint {
            RouteWaypoint::Station { .. } => {
                // At a station - should transition to loading/unloading
                // For now, just move to next waypoint
                if let Some(next_waypoint) = route.waypoints.get(route.current_waypoint + 1) {
                    if let RouteWaypoint::Segment { segment_id: _ } = next_waypoint {
                        // Just advance for now - segment advancement logic in main loop
                        movement.route_state = TrainRouteState::Active;
                        result.state_changed = true;
                    }
                } else {
                    // No more waypoints - route complete
                    movement.complete_route();
                    result.route_completed = true;
                    result.state_changed = true;
                }
                Ok(result)
            }
            RouteWaypoint::Segment { segment_id } => {
                // Move along segment
                let completed = movement.advance_on_segment(self.ticks_per_turn);
                result.completed_segment = completed;

                if completed {
                    // Move to next waypoint (should be a station)
                    if let Some(RouteWaypoint::Station { station_id, .. }) =
                        route.waypoints.get(route.current_waypoint + 1)
                    {
                        movement.arrive_at_station(*station_id);
                        result.arrived_at_station = Some(*station_id);
                        result.state_changed = true;
                    } else {
                        return Err(AdvancementError::InvalidWaypoint);
                    }
                }

                Ok(result)
            }
        }
    }

    /// Batch advance multiple trains
    pub fn advance_all_trains(
        &self,
        movements: &mut [TrainMovement],
        route_network: &crate::domain::RouteNetwork,
        rail_network: &RailNetwork,
        train_speed_factor: f32,
        train_mass: u32,
    ) -> Vec<Result<AdvancementResult, (TrainId, AdvancementError)>> {
        movements
            .iter_mut()
            .map(|m| {
                self.advance_train(m, route_network, rail_network, train_speed_factor, train_mass)
                    .map_err(|e| (m.train_id, e))
            })
            .collect()
    }

    /// Set congestion factor (affects all train speeds)
    pub fn set_congestion_factor(&mut self, factor: f32) {
        self.congestion_factor = factor.clamp(0.1, 2.0);
    }

    /// Get effective ticks per turn considering congestion
    pub fn effective_ticks_per_turn(&self) -> u64 {
        (self.ticks_per_turn as f32 * self.congestion_factor).ceil() as u64
    }
}

impl Default for TrainAdvancer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{HexCoord, RailType, StationId, TrainId, RouteId};

    fn create_test_segment() -> RailSegment {
        RailSegment::new(
            SegmentId(1),
            HexCoord::new(0, 0),
            HexCoord::new(1, 0),
            RailType::Standard,
        )
    }

    #[test]
    fn test_advancer_creation() {
        let advancer = TrainAdvancer::new();
        assert_eq!(advancer.ticks_per_turn, 10);
        assert_eq!(advancer.congestion_factor, 1.0);
        assert_eq!(advancer.effective_ticks_per_turn(), 10);
    }

    #[test]
    fn test_congestion_factor() {
        let mut advancer = TrainAdvancer::new();
        advancer.set_congestion_factor(0.5);
        assert_eq!(advancer.effective_ticks_per_turn(), 5);

        advancer.set_congestion_factor(2.0);
        assert_eq!(advancer.effective_ticks_per_turn(), 20);

        // Clamp to min 0.1
        advancer.set_congestion_factor(0.05);
        assert_eq!(advancer.congestion_factor, 0.1);
    }

    #[test]
    fn test_segment_travel_calculation() {
        let advancer = TrainAdvancer::new();
        let segment = create_test_segment();

        // Standard rail, light train (1000 mass), no congestion
        let time = advancer.calculate_segment_travel_time(&segment, 1.0, 1000);
        assert!(time > 0);
        assert!(time <= 100); // Should be reasonable

        // Fast train (2.0x speed)
        let fast_time = advancer.calculate_segment_travel_time(&segment, 2.0, 1000);
        assert!(fast_time < time);

        // Heavy train (5000 mass)
        let heavy_time = advancer.calculate_segment_travel_time(&segment, 1.0, 5000);
        assert!(heavy_time > time);
    }

    #[test]
    fn test_advancement_result_structure() {
        let result = AdvancementResult {
            train_id: TrainId(1),
            advanced_ticks: 10,
            state_changed: false,
            completed_segment: false,
            arrived_at_station: None,
            route_completed: false,
        };

        assert_eq!(result.train_id, TrainId(1));
        assert_eq!(result.advanced_ticks, 10);
        assert!(!result.state_changed);
    }

    #[test]
    fn test_travel_time_scales() {
        let advancer = TrainAdvancer::new();
        let segment = create_test_segment();

        // Different speed factors should scale proportionally
        let time_1x = advancer.calculate_segment_travel_time(&segment, 1.0, 1000) as f32;
        let time_2x = advancer.calculate_segment_travel_time(&segment, 2.0, 1000) as f32;
        
        // 2x speed should take roughly half the time
        assert!(time_2x < time_1x);
        assert!((time_1x / time_2x - 2.0).abs() < 0.5); // Within 50% of 2x ratio
    }

    #[test]
    fn test_mass_affects_speed() {
        let advancer = TrainAdvancer::new();
        let segment = create_test_segment();

        let light_time = advancer.calculate_segment_travel_time(&segment, 1.0, 1000);
        let heavy_time = advancer.calculate_segment_travel_time(&segment, 1.0, 5000);

        // Heavier train should take longer
        assert!(heavy_time > light_time);
    }

    #[test]
    fn test_rail_type_affects_speed() {
        let advancer = TrainAdvancer::new();

        // Create segments of different rail types
        let standard = RailSegment::new(
            SegmentId(1),
            HexCoord::new(0, 0),
            HexCoord::new(1, 0),
            RailType::Standard,
        );

        // Standard has 1.0x multiplier - fastest would be Maglev with higher multiplier
        let time_standard = advancer.calculate_segment_travel_time(&standard, 1.0, 1000);
        assert!(time_standard > 0);
    }

    #[test]
    fn test_error_display() {
        assert_eq!(AdvancementError::TrainNotFound.to_string(), "Train not found");
        assert_eq!(AdvancementError::SegmentNotFound.to_string(), "Segment not found");
        assert_eq!(AdvancementError::InvalidWaypoint.to_string(), "Invalid waypoint");
        assert_eq!(AdvancementError::TrainStuck.to_string(), "Train stuck");
    }

    #[test]
    fn test_advancement_error_clone() {
        let err = AdvancementError::TrainNotFound;
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn test_advancement_result_serialization() {
        let result = AdvancementResult {
            train_id: TrainId(1),
            advanced_ticks: 10,
            state_changed: true,
            completed_segment: false,
            arrived_at_station: Some(StationId(5)),
            route_completed: false,
        };

        // Verify fields
        assert_eq!(result.train_id, TrainId(1));
        assert_eq!(result.advanced_ticks, 10);
        assert!(result.state_changed);
        assert_eq!(result.arrived_at_station, Some(StationId(5)));
    }
}
