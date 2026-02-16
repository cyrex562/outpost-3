use serde::{Deserialize, Serialize};
use std::fmt;

use crate::domain::{ResourceType, StationId};

/// Cargo request identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CargoRequestId(pub u64);

/// Status of a cargo request
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CargoRequestStatus {
    /// Request created, waiting for assignment
    Pending,
    /// Assigned to a train route
    Assigned,
    /// Currently being loaded onto train
    Loading,
    /// In transit on train
    InTransit,
    /// Currently being unloaded at destination
    Unloading,
    /// Successfully delivered
    Completed,
    /// Request was cancelled
    Cancelled,
}

/// Priority levels for cargo requests (0-10, higher = more urgent)
pub const PRIORITY_MIN: u8 = 0;
pub const PRIORITY_MAX: u8 = 10;
pub const PRIORITY_DEFAULT: u8 = 5;
pub const PRIORITY_CRITICAL: u8 = 10;
pub const PRIORITY_LOW: u8 = 2;

/// Full cargo request with source and destination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CargoRequest {
    pub id: CargoRequestId,
    pub resource: ResourceType,
    pub quantity: u32,
    pub source_station: StationId,
    pub destination_station: StationId,
    pub priority: u8,
    pub status: CargoRequestStatus,
    pub created_tick: u64,
    pub assigned_tick: Option<u64>,
    pub completed_tick: Option<u64>,
}

impl CargoRequest {
    /// Create a new cargo request
    pub fn new(
        id: CargoRequestId,
        resource: ResourceType,
        quantity: u32,
        source: StationId,
        destination: StationId,
        priority: u8,
        created_tick: u64,
    ) -> Self {
        Self {
            id,
            resource,
            quantity,
            source_station: source,
            destination_station: destination,
            priority: priority.clamp(PRIORITY_MIN, PRIORITY_MAX),
            status: CargoRequestStatus::Pending,
            created_tick,
            assigned_tick: None,
            completed_tick: None,
        }
    }

    /// Assign the request to a route
    pub fn assign(&mut self, tick: u64) {
        self.status = CargoRequestStatus::Assigned;
        self.assigned_tick = Some(tick);
    }

    /// Mark as loading
    pub fn start_loading(&mut self) {
        self.status = CargoRequestStatus::Loading;
    }

    /// Mark as in transit
    pub fn start_transit(&mut self) {
        self.status = CargoRequestStatus::InTransit;
    }

    /// Mark as unloading
    pub fn start_unloading(&mut self) {
        self.status = CargoRequestStatus::Unloading;
    }

    /// Mark as completed
    pub fn complete(&mut self, tick: u64) {
        self.status = CargoRequestStatus::Completed;
        self.completed_tick = Some(tick);
    }

    /// Cancel the request
    pub fn cancel(&mut self) {
        self.status = CargoRequestStatus::Cancelled;
    }

    /// Check if request is active (not completed/cancelled)
    pub fn is_active(&self) -> bool {
        !matches!(
            self.status,
            CargoRequestStatus::Completed | CargoRequestStatus::Cancelled
        )
    }

    /// Get total time spent (if completed)
    pub fn total_time(&self) -> Option<u64> {
        self.completed_tick.map(|end| end - self.created_tick)
    }

    /// Get time to assignment
    pub fn time_to_assignment(&self) -> Option<u64> {
        self.assigned_tick
            .map(|assigned| assigned - self.created_tick)
    }
}

/// Error types for cargo request operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CargoRequestError {
    /// Request not found
    NotFound,
    /// Invalid state transition
    InvalidStateTransition,
    /// Insufficient capacity at station
    InsufficientCapacity,
    /// Source and destination are the same
    SameSourceDestination,
    /// Invalid priority value
    InvalidPriority,
    /// Request already exists
    DuplicateRequest,
}

impl fmt::Display for CargoRequestError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CargoRequestError::NotFound => write!(f, "Cargo request not found"),
            CargoRequestError::InvalidStateTransition => write!(f, "Invalid state transition"),
            CargoRequestError::InsufficientCapacity => write!(f, "Insufficient capacity"),
            CargoRequestError::SameSourceDestination => {
                write!(f, "Source and destination are the same")
            }
            CargoRequestError::InvalidPriority => write!(f, "Invalid priority"),
            CargoRequestError::DuplicateRequest => write!(f, "Duplicate request"),
        }
    }
}

/// Manages all cargo requests in a colony
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CargoRequestManager {
    requests: Vec<CargoRequest>,
    next_id: u64,
}

impl CargoRequestManager {
    pub fn new() -> Self {
        Self {
            requests: Vec::new(),
            next_id: 1,
        }
    }

    /// Create a new cargo request
    pub fn create_request(
        &mut self,
        resource: ResourceType,
        quantity: u32,
        source: StationId,
        destination: StationId,
        priority: u8,
        current_tick: u64,
    ) -> Result<CargoRequestId, CargoRequestError> {
        // Validate source != destination
        if source == destination {
            return Err(CargoRequestError::SameSourceDestination);
        }

        let id = CargoRequestId(self.next_id);
        self.next_id += 1;

        let request = CargoRequest::new(
            id,
            resource,
            quantity,
            source,
            destination,
            priority,
            current_tick,
        );

        self.requests.push(request);
        Ok(id)
    }

    /// Get a request by ID
    pub fn get_request(&self, id: CargoRequestId) -> Option<&CargoRequest> {
        self.requests.iter().find(|r| r.id == id)
    }

    /// Get a mutable request by ID
    pub fn get_request_mut(&mut self, id: CargoRequestId) -> Option<&mut CargoRequest> {
        self.requests.iter_mut().find(|r| r.id == id)
    }

    /// Get all pending requests for a station (as source)
    pub fn get_pending_at_source(&self, station_id: StationId) -> Vec<&CargoRequest> {
        self.requests
            .iter()
            .filter(|r| r.source_station == station_id && r.status == CargoRequestStatus::Pending)
            .collect()
    }

    /// Get all requests by status
    pub fn get_by_status(&self, status: CargoRequestStatus) -> Vec<&CargoRequest> {
        self.requests
            .iter()
            .filter(|r| r.status == status)
            .collect()
    }

    /// Get requests by priority (sorted high to low)
    pub fn get_by_priority(&self, min_priority: u8) -> Vec<&CargoRequest> {
        let mut requests: Vec<&CargoRequest> = self
            .requests
            .iter()
            .filter(|r| r.priority >= min_priority && r.is_active())
            .collect();
        requests.sort_by(|a, b| b.priority.cmp(&a.priority));
        requests
    }

    /// Assign a request
    pub fn assign_request(
        &mut self,
        id: CargoRequestId,
        tick: u64,
    ) -> Result<(), CargoRequestError> {
        let request = self
            .get_request_mut(id)
            .ok_or(CargoRequestError::NotFound)?;

        if request.status != CargoRequestStatus::Pending {
            return Err(CargoRequestError::InvalidStateTransition);
        }

        request.assign(tick);
        Ok(())
    }

    /// Update request status
    pub fn update_status(
        &mut self,
        id: CargoRequestId,
        new_status: CargoRequestStatus,
    ) -> Result<(), CargoRequestError> {
        let request = self
            .get_request_mut(id)
            .ok_or(CargoRequestError::NotFound)?;

        request.status = new_status;
        Ok(())
    }

    /// Complete a request
    pub fn complete_request(
        &mut self,
        id: CargoRequestId,
        tick: u64,
    ) -> Result<(), CargoRequestError> {
        let request = self
            .get_request_mut(id)
            .ok_or(CargoRequestError::NotFound)?;

        request.complete(tick);
        Ok(())
    }

    /// Cancel a request
    pub fn cancel_request(&mut self, id: CargoRequestId) -> Result<(), CargoRequestError> {
        let request = self
            .get_request_mut(id)
            .ok_or(CargoRequestError::NotFound)?;

        request.cancel();
        Ok(())
    }

    /// Get all active requests
    pub fn get_active_requests(&self) -> Vec<&CargoRequest> {
        self.requests.iter().filter(|r| r.is_active()).collect()
    }

    /// Get total active request count
    pub fn active_count(&self) -> usize {
        self.requests.iter().filter(|r| r.is_active()).count()
    }

    /// Get completed request count
    pub fn completed_count(&self) -> usize {
        self.requests
            .iter()
            .filter(|r| r.status == CargoRequestStatus::Completed)
            .count()
    }

    /// Remove completed and cancelled requests older than a threshold
    pub fn prune_old_requests(&mut self, before_tick: u64) {
        self.requests.retain(|r| {
            r.is_active()
                || r.completed_tick
                    .map(|tick| tick >= before_tick)
                    .unwrap_or(true)
        });
    }
}

impl Default for CargoRequestManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cargo_request_creation() {
        let request = CargoRequest::new(
            CargoRequestId(1),
            ResourceType::Steel,
            100,
            StationId(1),
            StationId(2),
            5,
            0,
        );

        assert_eq!(request.id, CargoRequestId(1));
        assert_eq!(request.resource, ResourceType::Steel);
        assert_eq!(request.quantity, 100);
        assert_eq!(request.source_station, StationId(1));
        assert_eq!(request.destination_station, StationId(2));
        assert_eq!(request.priority, 5);
        assert_eq!(request.status, CargoRequestStatus::Pending);
        assert!(request.is_active());
    }

    #[test]
    fn test_priority_clamping() {
        let request = CargoRequest::new(
            CargoRequestId(1),
            ResourceType::Steel,
            100,
            StationId(1),
            StationId(2),
            15, // Above max
            0,
        );

        assert_eq!(request.priority, PRIORITY_MAX);

        let request2 = CargoRequest::new(
            CargoRequestId(2),
            ResourceType::Steel,
            100,
            StationId(1),
            StationId(2),
            255, // Way above max
            0,
        );

        assert_eq!(request2.priority, PRIORITY_MAX);
    }

    #[test]
    fn test_request_lifecycle() {
        let mut request = CargoRequest::new(
            CargoRequestId(1),
            ResourceType::Steel,
            100,
            StationId(1),
            StationId(2),
            5,
            0,
        );

        // Assign
        request.assign(10);
        assert_eq!(request.status, CargoRequestStatus::Assigned);
        assert_eq!(request.assigned_tick, Some(10));
        assert!(request.is_active());

        // Loading
        request.start_loading();
        assert_eq!(request.status, CargoRequestStatus::Loading);

        // Transit
        request.start_transit();
        assert_eq!(request.status, CargoRequestStatus::InTransit);

        // Unloading
        request.start_unloading();
        assert_eq!(request.status, CargoRequestStatus::Unloading);

        // Complete
        request.complete(100);
        assert_eq!(request.status, CargoRequestStatus::Completed);
        assert_eq!(request.completed_tick, Some(100));
        assert!(!request.is_active());
        assert_eq!(request.total_time(), Some(100));
        assert_eq!(request.time_to_assignment(), Some(10));
    }

    #[test]
    fn test_request_cancellation() {
        let mut request = CargoRequest::new(
            CargoRequestId(1),
            ResourceType::Steel,
            100,
            StationId(1),
            StationId(2),
            5,
            0,
        );

        request.cancel();
        assert_eq!(request.status, CargoRequestStatus::Cancelled);
        assert!(!request.is_active());
    }

    #[test]
    fn test_manager_creation() {
        let manager = CargoRequestManager::new();
        assert_eq!(manager.active_count(), 0);
        assert_eq!(manager.completed_count(), 0);
    }

    #[test]
    fn test_manager_create_request() {
        let mut manager = CargoRequestManager::new();

        let id = manager
            .create_request(ResourceType::Steel, 100, StationId(1), StationId(2), 5, 0)
            .unwrap();

        assert_eq!(id, CargoRequestId(1));
        assert_eq!(manager.active_count(), 1);

        let request = manager.get_request(id).unwrap();
        assert_eq!(request.resource, ResourceType::Steel);
        assert_eq!(request.quantity, 100);
    }

    #[test]
    fn test_manager_same_source_destination() {
        let mut manager = CargoRequestManager::new();

        let result =
            manager.create_request(ResourceType::Steel, 100, StationId(1), StationId(1), 5, 0);

        assert_eq!(result, Err(CargoRequestError::SameSourceDestination));
    }

    #[test]
    fn test_manager_assign_request() {
        let mut manager = CargoRequestManager::new();

        let id = manager
            .create_request(ResourceType::Steel, 100, StationId(1), StationId(2), 5, 0)
            .unwrap();

        manager.assign_request(id, 10).unwrap();

        let request = manager.get_request(id).unwrap();
        assert_eq!(request.status, CargoRequestStatus::Assigned);
        assert_eq!(request.assigned_tick, Some(10));
    }

    #[test]
    fn test_manager_invalid_state_transition() {
        let mut manager = CargoRequestManager::new();

        let id = manager
            .create_request(ResourceType::Steel, 100, StationId(1), StationId(2), 5, 0)
            .unwrap();

        manager.assign_request(id, 10).unwrap();

        // Try to assign again
        let result = manager.assign_request(id, 20);
        assert_eq!(result, Err(CargoRequestError::InvalidStateTransition));
    }

    #[test]
    fn test_manager_get_by_status() {
        let mut manager = CargoRequestManager::new();

        let id1 = manager
            .create_request(ResourceType::Steel, 100, StationId(1), StationId(2), 5, 0)
            .unwrap();
        let id2 = manager
            .create_request(ResourceType::Food, 50, StationId(2), StationId(3), 3, 0)
            .unwrap();

        manager.assign_request(id1, 10).unwrap();

        let pending = manager.get_by_status(CargoRequestStatus::Pending);
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, id2);

        let assigned = manager.get_by_status(CargoRequestStatus::Assigned);
        assert_eq!(assigned.len(), 1);
        assert_eq!(assigned[0].id, id1);
    }

    #[test]
    fn test_manager_get_by_priority() {
        let mut manager = CargoRequestManager::new();

        manager
            .create_request(ResourceType::Steel, 100, StationId(1), StationId(2), 3, 0)
            .unwrap();
        manager
            .create_request(ResourceType::Food, 50, StationId(2), StationId(3), 8, 0)
            .unwrap();
        manager
            .create_request(ResourceType::Water, 75, StationId(1), StationId(3), 5, 0)
            .unwrap();

        let high_priority = manager.get_by_priority(5);
        assert_eq!(high_priority.len(), 2);
        assert_eq!(high_priority[0].priority, 8); // Sorted high to low
        assert_eq!(high_priority[1].priority, 5);
    }

    #[test]
    fn test_manager_get_pending_at_source() {
        let mut manager = CargoRequestManager::new();

        manager
            .create_request(ResourceType::Steel, 100, StationId(1), StationId(2), 5, 0)
            .unwrap();
        manager
            .create_request(ResourceType::Food, 50, StationId(1), StationId(3), 3, 0)
            .unwrap();
        manager
            .create_request(ResourceType::Water, 75, StationId(2), StationId(3), 5, 0)
            .unwrap();

        let pending = manager.get_pending_at_source(StationId(1));
        assert_eq!(pending.len(), 2);
    }

    #[test]
    fn test_manager_complete_request() {
        let mut manager = CargoRequestManager::new();

        let id = manager
            .create_request(ResourceType::Steel, 100, StationId(1), StationId(2), 5, 0)
            .unwrap();

        manager.complete_request(id, 100).unwrap();

        let request = manager.get_request(id).unwrap();
        assert_eq!(request.status, CargoRequestStatus::Completed);
        assert_eq!(request.completed_tick, Some(100));
        assert_eq!(manager.completed_count(), 1);
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn test_manager_cancel_request() {
        let mut manager = CargoRequestManager::new();

        let id = manager
            .create_request(ResourceType::Steel, 100, StationId(1), StationId(2), 5, 0)
            .unwrap();

        manager.cancel_request(id).unwrap();

        let request = manager.get_request(id).unwrap();
        assert_eq!(request.status, CargoRequestStatus::Cancelled);
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn test_manager_prune_old_requests() {
        let mut manager = CargoRequestManager::new();

        let id1 = manager
            .create_request(ResourceType::Steel, 100, StationId(1), StationId(2), 5, 0)
            .unwrap();
        let id2 = manager
            .create_request(ResourceType::Food, 50, StationId(2), StationId(3), 3, 0)
            .unwrap();

        manager.complete_request(id1, 50).unwrap();
        manager.complete_request(id2, 150).unwrap();

        // Prune requests completed before tick 100
        manager.prune_old_requests(100);

        // id1 (completed at 50) should be pruned
        assert!(manager.get_request(id1).is_none());
        // id2 (completed at 150) should remain
        assert!(manager.get_request(id2).is_some());
    }

    #[test]
    fn test_error_display() {
        assert_eq!(
            CargoRequestError::NotFound.to_string(),
            "Cargo request not found"
        );
        assert_eq!(
            CargoRequestError::InvalidStateTransition.to_string(),
            "Invalid state transition"
        );
        assert_eq!(
            CargoRequestError::SameSourceDestination.to_string(),
            "Source and destination are the same"
        );
    }
}
