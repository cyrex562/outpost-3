use serde::{Deserialize, Serialize};
use std::fmt;

use crate::domain::{RouteId, SegmentId, StationId, TrainId, TrainRouteState};

/// Detailed position of a train on the rail network
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrainPosition {
    /// Train is at a station
    Station(StationId),
    /// Train is on a segment between stations
    OnSegment(SegmentId),
    /// Train location unknown (should not occur in normal operation)
    Unknown,
}

impl fmt::Display for TrainPosition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TrainPosition::Station(id) => write!(f, "Station({})", id.0),
            TrainPosition::OnSegment(id) => write!(f, "Segment({})", id.0),
            TrainPosition::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Train movement state - tracks train's progress and actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainMovement {
    pub train_id: TrainId,
    pub route_id: RouteId,
    pub route_state: TrainRouteState,
    pub position: TrainPosition,
    pub waypoint_index: usize,
    /// How many ticks elapsed on current segment (0 if at station)
    pub segment_progress_ticks: u64,
    /// Total ticks to traverse current segment
    pub segment_total_ticks: u64,
    /// Total ticks spent on this route
    pub total_ticks: u64,
    /// Cargo queued for loading at current station (resource_id, quantity)
    pub pending_cargo: Vec<(u32, u32)>,
}

impl TrainMovement {
    pub fn new(train_id: TrainId, route_id: RouteId, start_station: StationId) -> Self {
        Self {
            train_id,
            route_id,
            route_state: TrainRouteState::Queued,
            position: TrainPosition::Station(start_station),
            waypoint_index: 0,
            segment_progress_ticks: 0,
            segment_total_ticks: 0,
            total_ticks: 0,
            pending_cargo: Vec::new(),
        }
    }

    /// Dispatch train from station (transition Queued -> Active)
    pub fn dispatch_from_station(&mut self) {
        self.route_state = TrainRouteState::Active;
    }

    /// Enter a segment (Moving state)
    pub fn enter_segment(&mut self, segment: SegmentId, travel_ticks: u64) {
        self.position = TrainPosition::OnSegment(segment);
        self.segment_progress_ticks = 0;
        self.segment_total_ticks = travel_ticks;
        self.waypoint_index += 1;
    }

    /// Advance movement on segment
    pub fn advance_on_segment(&mut self, ticks: u64) -> bool {
        self.segment_progress_ticks = self.segment_progress_ticks.saturating_add(ticks);
        self.total_ticks = self.total_ticks.saturating_add(ticks);

        // Check if reached segment end
        self.segment_progress_ticks >= self.segment_total_ticks
    }

    /// Exit segment and arrive at station
    pub fn arrive_at_station(&mut self, station: StationId) {
        self.position = TrainPosition::Station(station);
        self.segment_progress_ticks = 0;
        self.segment_total_ticks = 0;
        self.waypoint_index += 1;
    }

    /// Start loading cargo at station
    pub fn start_loading(&mut self) {
        self.route_state = TrainRouteState::Loading;
    }

    /// Finish loading cargo
    pub fn finish_loading(&mut self) {
        self.route_state = TrainRouteState::Active;
        self.pending_cargo.clear();
    }

    /// Start unloading cargo
    pub fn start_unloading(&mut self) {
        self.route_state = TrainRouteState::Unloading;
    }

    /// Finish unloading cargo
    pub fn finish_unloading(&mut self) {
        self.route_state = TrainRouteState::Active;
        self.pending_cargo.clear();
    }

    /// Complete the route
    pub fn complete_route(&mut self) {
        self.route_state = TrainRouteState::Completed;
    }

    /// Cancel the route
    pub fn cancel_route(&mut self) {
        self.route_state = TrainRouteState::Cancelled;
    }

    /// Get progress on current segment (0.0-1.0)
    pub fn segment_progress(&self) -> f32 {
        if self.segment_total_ticks == 0 {
            0.0
        } else {
            (self.segment_progress_ticks as f32 / self.segment_total_ticks as f32).clamp(0.0, 1.0)
        }
    }

    /// Check if train has completed segment traversal
    pub fn segment_complete(&self) -> bool {
        self.segment_progress_ticks >= self.segment_total_ticks
    }

    /// Add cargo to loading queue
    pub fn queue_cargo(&mut self, resource_id: u32, quantity: u32) {
        self.pending_cargo.push((resource_id, quantity));
    }

    /// Get total pending cargo
    pub fn pending_cargo_amount(&self) -> u32 {
        self.pending_cargo.iter().map(|(_, q)| q).sum()
    }

    /// Clear pending cargo (dropped/spilled)
    pub fn drop_cargo(&mut self) {
        self.pending_cargo.clear();
    }
}

/// Movement state machine errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MovementError {
    /// Invalid state transition
    InvalidTransition,
    /// Train not at expected location
    WrongLocation,
    /// No cargo to unload
    NoCargo,
    /// Train not in correct state
    WrongState,
}

impl fmt::Display for MovementError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MovementError::InvalidTransition => write!(f, "Invalid state transition"),
            MovementError::WrongLocation => write!(f, "Train not at expected location"),
            MovementError::NoCargo => write!(f, "No cargo to unload"),
            MovementError::WrongState => write!(f, "Train not in correct state"),
        }
    }
}

/// Manages all train movements in a colony
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovementManager {
    trains: Vec<TrainMovement>,
}

impl MovementManager {
    pub fn new() -> Self {
        Self {
            trains: Vec::new(),
        }
    }

    /// Create and register a new train movement
    pub fn register_train(
        &mut self,
        train_id: TrainId,
        route_id: RouteId,
        start_station: StationId,
    ) -> &mut TrainMovement {
        let movement = TrainMovement::new(train_id, route_id, start_station);
        self.trains.push(movement);
        self.trains.last_mut().unwrap()
    }

    /// Get train movement by ID
    pub fn get_train(&self, train_id: TrainId) -> Option<&TrainMovement> {
        self.trains.iter().find(|t| t.train_id == train_id)
    }

    /// Get mutable train movement
    pub fn get_train_mut(&mut self, train_id: TrainId) -> Option<&mut TrainMovement> {
        self.trains.iter_mut().find(|t| t.train_id == train_id)
    }

    /// Get all trains on a route
    pub fn get_route_trains(&self, route_id: RouteId) -> Vec<&TrainMovement> {
        self.trains.iter().filter(|t| t.route_id == route_id).collect()
    }

    /// Get trains at a specific station
    pub fn get_trains_at_station(&self, station: StationId) -> Vec<&TrainMovement> {
        self.trains
            .iter()
            .filter(|t| t.position == TrainPosition::Station(station))
            .collect()
    }

    /// Get trains on a specific segment
    pub fn get_trains_on_segment(&self, segment: SegmentId) -> Vec<&TrainMovement> {
        self.trains
            .iter()
            .filter(|t| t.position == TrainPosition::OnSegment(segment))
            .collect()
    }

    /// Get trains in a specific state
    pub fn get_trains_in_state(&self, state: TrainRouteState) -> Vec<&TrainMovement> {
        self.trains.iter().filter(|t| t.route_state == state).collect()
    }

    /// Get total trains
    pub fn total_trains(&self) -> usize {
        self.trains.len()
    }

    /// Get total active trains
    pub fn total_active(&self) -> usize {
        self.trains
            .iter()
            .filter(|t| t.route_state == TrainRouteState::Active)
            .count()
    }

    /// Get total trains loading/unloading
    pub fn total_loading_unloading(&self) -> usize {
        self.trains
            .iter()
            .filter(|t| {
                t.route_state == TrainRouteState::Loading
                    || t.route_state == TrainRouteState::Unloading
            })
            .count()
    }

    /// Remove train from tracking (when route complete/cancelled)
    pub fn remove_train(&mut self, train_id: TrainId) -> Option<TrainMovement> {
        if let Some(pos) = self.trains.iter().position(|t| t.train_id == train_id) {
            Some(self.trains.remove(pos))
        } else {
            None
        }
    }
}

impl Default for MovementManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_train_creation() {
        let movement = TrainMovement::new(TrainId(1), RouteId(1), StationId(1));
        assert_eq!(movement.train_id, TrainId(1));
        assert_eq!(movement.route_id, RouteId(1));
        assert_eq!(movement.route_state, TrainRouteState::Queued);
        assert_eq!(movement.position, TrainPosition::Station(StationId(1)));
    }

    #[test]
    fn test_dispatch() {
        let mut movement = TrainMovement::new(TrainId(1), RouteId(1), StationId(1));
        movement.dispatch_from_station();
        assert_eq!(movement.route_state, TrainRouteState::Active);
    }

    #[test]
    fn test_segment_traversal() {
        let mut movement = TrainMovement::new(TrainId(1), RouteId(1), StationId(1));
        movement.dispatch_from_station();

        // Enter segment with 10 tick travel time
        movement.enter_segment(SegmentId(1), 10);
        assert_eq!(movement.position, TrainPosition::OnSegment(SegmentId(1)));
        assert_eq!(movement.segment_progress_ticks, 0);
        assert!(!movement.segment_complete());

        // Advance 5 ticks
        movement.advance_on_segment(5);
        assert_eq!(movement.segment_progress_ticks, 5);
        assert_eq!(movement.segment_progress(), 0.5);
        assert!(!movement.segment_complete());

        // Advance 5 more ticks (complete)
        let completed = movement.advance_on_segment(5);
        assert!(completed);
        assert_eq!(movement.total_ticks, 10);

        // Arrive at destination
        movement.arrive_at_station(StationId(2));
        assert_eq!(movement.position, TrainPosition::Station(StationId(2)));
    }

    #[test]
    fn test_loading_unloading() {
        let mut movement = TrainMovement::new(TrainId(1), RouteId(1), StationId(1));
        movement.start_loading();
        assert_eq!(movement.route_state, TrainRouteState::Loading);

        movement.finish_loading();
        assert_eq!(movement.route_state, TrainRouteState::Active);

        movement.start_unloading();
        assert_eq!(movement.route_state, TrainRouteState::Unloading);

        movement.finish_unloading();
        assert_eq!(movement.route_state, TrainRouteState::Active);
    }

    #[test]
    fn test_cargo_queueing() {
        let mut movement = TrainMovement::new(TrainId(1), RouteId(1), StationId(1));
        assert_eq!(movement.pending_cargo_amount(), 0);

        movement.queue_cargo(1, 50);
        assert_eq!(movement.pending_cargo_amount(), 50);

        movement.queue_cargo(2, 30);
        assert_eq!(movement.pending_cargo_amount(), 80);

        movement.drop_cargo();
        assert_eq!(movement.pending_cargo_amount(), 0);
    }

    #[test]
    fn test_route_completion() {
        let mut movement = TrainMovement::new(TrainId(1), RouteId(1), StationId(1));
        movement.complete_route();
        assert_eq!(movement.route_state, TrainRouteState::Completed);
    }

    #[test]
    fn test_route_cancellation() {
        let mut movement = TrainMovement::new(TrainId(1), RouteId(1), StationId(1));
        movement.cancel_route();
        assert_eq!(movement.route_state, TrainRouteState::Cancelled);
    }

    #[test]
    fn test_movement_manager() {
        let mut manager = MovementManager::new();
        manager.register_train(TrainId(1), RouteId(1), StationId(1));
        manager.register_train(TrainId(2), RouteId(1), StationId(1));

        assert_eq!(manager.total_trains(), 2);

        let train1 = manager.get_train(TrainId(1));
        assert!(train1.is_some());
        assert_eq!(train1.unwrap().train_id, TrainId(1));
    }

    #[test]
    fn test_movement_queries() {
        let mut manager = MovementManager::new();
        let mov1 = manager.register_train(TrainId(1), RouteId(1), StationId(1));
        mov1.dispatch_from_station();

        let mov2 = manager.register_train(TrainId(2), RouteId(1), StationId(2));
        mov2.dispatch_from_station();
        mov2.start_loading();

        assert_eq!(manager.get_route_trains(RouteId(1)).len(), 2);
        assert_eq!(manager.get_trains_at_station(StationId(1)).len(), 1); // Train 1 is still at Station 1
        assert_eq!(manager.get_trains_at_station(StationId(2)).len(), 1); // Train 2 is at Station 2
        assert_eq!(manager.total_active(), 1);
        assert_eq!(manager.total_loading_unloading(), 1);
    }

    #[test]
    fn test_train_removal() {
        let mut manager = MovementManager::new();
        manager.register_train(TrainId(1), RouteId(1), StationId(1));
        assert_eq!(manager.total_trains(), 1);

        let removed = manager.remove_train(TrainId(1));
        assert!(removed.is_some());
        assert_eq!(manager.total_trains(), 0);
    }

    #[test]
    fn test_segment_progress_calculation() {
        let mut movement = TrainMovement::new(TrainId(1), RouteId(1), StationId(1));
        movement.enter_segment(SegmentId(1), 10);

        assert_eq!(movement.segment_progress(), 0.0);
        movement.advance_on_segment(2);
        assert_eq!(movement.segment_progress(), 0.2);
        movement.advance_on_segment(3);
        assert_eq!(movement.segment_progress(), 0.5);
        movement.advance_on_segment(5);
        assert!(movement.segment_progress() >= 1.0);
    }
}
